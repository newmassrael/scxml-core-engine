// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Synchronization

/// What a hazard domain retires: an object that carries the link the domain chains it by and a hook that
/// runs once the domain has decided nothing names it any more.
///
/// A retired object is reached through raw pointers (the hazard slots and the domain's list), which hold
/// no reference. The ownership the unlinking thread held on it is therefore handed to the domain with the
/// object (`HazardDomain.retire(adopting:)`), and the domain gives it up after the hook ran.
class Retirable: @unchecked Sendable {
    /// The next retired object. Written only by the domain.
    fileprivate let retiredNext = Atomic<UnsafeMutableRawPointer?>(nil)

    init() {}

    /// Runs once, on the thread whose scan found no hazard naming the object, before the domain gives up
    /// the ownership it was handed. The object is alive, and nothing but a hazard that no longer names it
    /// could reach it.
    func willReclaim() {}
}

/// The reclamation domain of a `segmented` queue (SCE Protocol-Synthesis RFC section synth-5-P,
/// *Reclamation domain*): hazard pointers, sized from the number of participants and built by the caller.
///
/// A segment can still be read by one participant after another has taken it out of the queue, so it cannot
/// be given up at that moment. Each participant publishes, in a slot of its own, the one segment it is about
/// to touch (`Hazard.protect`); the thread that unlinks a segment hands it to the domain
/// (`retire(adopting:)`); and the domain gives it up only when no slot names it. A participant that stalls
/// holds back the segments its slot names and nothing else, which is why hazard pointers were chosen over
/// epochs: the memory a stalled participant keeps unreclaimed is bounded by the participants.
///
/// Swift has no collector and the objects here are reference counted, so a raw pointer read from a shared
/// location is not yet a reference: between the load and the retain the object may end. A hazard closes that
/// window, and nothing else is asked of it. The scheme is otherwise the one the Rust runtime has
/// (`queue/hazard.rs`): a fixed array of slots, and an intrusive list of the retired objects (`Retirable`).
///
/// **Progress.** `protect` and `retire` are lock-free: a retry means another participant moved the pointer
/// or pushed a node. The scan that gives nodes up is run by one thread at a time; a thread that finds a scan
/// running does not wait for it, it returns and leaves the nodes on the list, so no operation of a queue
/// waits for another participant on account of the domain.
///
/// **Ordering.** A hazard is published with a sequentially consistent store and re-validated with a
/// sequentially consistent load of the pointer it guards, and the scan reads every hazard with a
/// sequentially consistent load after the node was unlinked. Either the scan sees the hazard and keeps the
/// node, or the participant's validation sees the pointer already moved and does not use it. That holds when
/// the location a participant validates against is written, with a sequentially consistent operation, by
/// whoever unlinks the node before the node is retired; the queue using the domain has to arrange that
/// (`Lscq.swift`, *Where a hazard is validated*).
///
/// One domain per queue is the intended use. A domain several queues share holds the nodes of all of them;
/// what a node does when it is finally given up is its own (`Retirable.willReclaim`). A queue that ends
/// flushes the domain, so none of its segments outlives it on the list.
///
/// **Sendable.** The class is `@unchecked Sendable`: after the initializer it mutates nothing but atomics
/// and the scan's scratch array, which only the thread that holds `scanning` touches.
public final class HazardDomain: @unchecked Sendable {
    /// How many participants the domain serves: the most handles, both sides of every queue it serves
    /// together, alive at the same time.
    public let slots: Int

    /// Atomic words from one hazard slot to the next: one 64-byte line each, so that a participant publishing
    /// its hazard does not invalidate the line its neighbour's sits on.
    private static let hazardStride = 8

    /// The object each participant is currently touching, or nil.
    private let hazards: UnsafeMutablePointer<Atomic<UnsafeMutableRawPointer?>>

    /// Which slots a participant holds.
    private let taken: UnsafeMutablePointer<Atomic<Bool>>

    /// The hazards the scan in progress read. Used by the thread that holds `scanning`, and by it alone.
    private let named: UnsafeMutablePointer<UnsafeMutableRawPointer?>

    /// Retired objects not yet given up.
    private let retired = Atomic<UnsafeMutableRawPointer?>(nil)

    /// How many are on `retired`; a scan runs when it reaches twice the slots, so the unreclaimed memory is
    /// bounded by the participants.
    private let pending = Atomic<Int>(0)

    /// A scan is running.
    private let scanning = Atomic<Bool>(false)

    private let scanAt: Int

    /// An empty domain of `slots` hazard slots. A domain with none stops the program: it is a defect of the
    /// caller, and nothing could use it.
    public init(slots: Int) {
        precondition(slots >= 1, "a domain needs at least one slot")
        self.slots = slots
        scanAt = 2 * slots
        let cells = UnsafeMutablePointer<Atomic<UnsafeMutableRawPointer?>>.allocate(
            capacity: slots * Self.hazardStride)
        for position in 0..<(slots * Self.hazardStride) {
            (cells + position).initialize(to: Atomic<UnsafeMutableRawPointer?>(nil))
        }
        hazards = cells
        let flags = UnsafeMutablePointer<Atomic<Bool>>.allocate(capacity: slots)
        for position in 0..<slots {
            (flags + position).initialize(to: Atomic<Bool>(false))
        }
        taken = flags
        let scratch = UnsafeMutablePointer<UnsafeMutableRawPointer?>.allocate(capacity: slots)
        scratch.initialize(repeating: nil, count: slots)
        named = scratch
    }

    deinit {
        // A handle holds its domain, so none is alive and no hazard names anything: what is still on the
        // list is given up, which a queue that shares this domain with others cannot rely on its own end
        // to do.
        flush()
        hazards.deinitialize(count: slots * Self.hazardStride)
        hazards.deallocate()
        taken.deinitialize(count: slots)
        taken.deallocate()
        named.deinitialize(count: slots)
        named.deallocate()
    }

    /// How many retired objects are waiting to be given up. For tests and for a target that budgets: it is
    /// bounded by the slots.
    public var waiting: Int {
        pending.load(ordering: .acquiring)
    }

    /// Gives up what can be given up now: the retired objects no hazard names, unless a scan is already
    /// running, which is then left to finish. A participant that would rather not hold memory back until the
    /// next retirement calls it; the tests call it to reach the reclaiming path, which a few retirements
    /// never would (the scan runs when twice the slots are waiting). Lock-free: it never waits for another
    /// scan.
    public func collect() {
        scan()
    }

    /// Gives up every retired object no hazard names, waiting for a scan that is running instead of leaving
    /// the objects to it. A queue that ends calls it: no participant of that queue is alive, so none of its
    /// segments is named, and every one of them is given up before the queue's allocator can go. Blocking,
    /// which the end of a queue may be.
    public func flush() {
        while scanning.exchange(true, ordering: .acquiring) {}
        scanHeld()
    }

    /// A slot for one participant, or nil when all are taken. Dropping the handle gives the slot back.
    func acquire() -> Hazard? {
        for index in 0..<slots {
            if (taken + index).pointee.compareExchange(
                expected: false, desired: true, ordering: .acquiringAndReleasing
            ).exchanged {
                return Hazard(domain: self, index: index)
            }
        }
        return nil
    }

    /// Hands `node`, already unlinked from every structure a participant can reach it through, to the
    /// domain, together with the one ownership the caller holds on it. The node's `willReclaim` runs and
    /// the ownership is given up once no hazard names it, now or at a later retirement.
    ///
    /// The node is retired once. The caller holds no hazard on it, or the scan this may start would keep it
    /// for the caller.
    func retire(adopting node: Unmanaged<Retirable>) {
        let pointer = node.toOpaque()
        // Counted before it is published, so a scan that gives it up at once never subtracts what was not
        // yet added.
        let waiting = pending.wrappingAdd(1, ordering: .acquiringAndReleasing).newValue
        push(first: pointer, last: pointer)
        if waiting >= scanAt {
            scan()
        }
    }

    fileprivate func cell(_ index: Int) -> UnsafeMutablePointer<Atomic<UnsafeMutableRawPointer?>> {
        hazards + index * Self.hazardStride
    }

    fileprivate func give(_ index: Int) {
        (taken + index).pointee.store(false, ordering: .releasing)
    }

    /// The object a pointer on the list names, as a reference that lasts the caller's use of it. The list
    /// owns every node on it, so the node is alive.
    private static func node(_ pointer: UnsafeMutableRawPointer) -> Retirable {
        Unmanaged<Retirable>.fromOpaque(pointer).takeUnretainedValue()
    }

    /// Pushes the chain `first ... last` (linked through `retiredNext`) onto the list.
    private func push(first: UnsafeMutableRawPointer, last: UnsafeMutableRawPointer) {
        let tail = Self.node(last)
        var head = retired.load(ordering: .relaxed)
        while true {
            // `last` belongs to a chain this thread owns until the exchange below publishes it.
            tail.retiredNext.store(head, ordering: .relaxed)
            let (exchanged, original) = retired.compareExchange(
                expected: head, desired: first, successOrdering: .releasing, failureOrdering: .relaxed)
            if exchanged {
                return
            }
            head = original
        }
    }

    /// Gives up every retired object no hazard names. A thread that finds a scan already running returns:
    /// the objects wait for the next one.
    private func scan() {
        if scanning.exchange(true, ordering: .acquiring) {
            return
        }
        scanHeld()
    }

    /// The scan, run by the thread that holds `scanning`; it gives the flag back.
    private func scanHeld() {
        var list = retired.exchange(nil, ordering: .acquiring)
        // The objects on `list` were unlinked before they were retired, so a participant that has not
        // published a hazard for one by now will find the pointer moved when it validates (the type's
        // documentation).
        for index in 0..<slots {
            named[index] = cell(index).pointee.load(ordering: .sequentiallyConsistent)
        }
        var kept: UnsafeMutableRawPointer?
        var keptLast: UnsafeMutableRawPointer?
        var givenUp = 0
        while let pointer = list {
            let node = Self.node(pointer)
            list = node.retiredNext.load(ordering: .relaxed)
            if isNamed(pointer) {
                // It leaves this local chain only by `push` below.
                node.retiredNext.store(kept, ordering: .relaxed)
                if kept == nil {
                    keptLast = pointer
                }
                kept = pointer
            } else {
                // No hazard names it and nothing else reaches it: it goes back to what gave it, and the
                // ownership the list held ends the object's life once nothing else holds it.
                node.willReclaim()
                Unmanaged<Retirable>.fromOpaque(pointer).release()
                givenUp += 1
            }
        }
        if let kept, let keptLast {
            push(first: kept, last: keptLast)
        }
        if givenUp > 0 {
            pending.wrappingAdd(-givenUp, ordering: .acquiringAndReleasing)
        }
        scanning.store(false, ordering: .releasing)
    }

    private func isNamed(_ pointer: UnsafeMutableRawPointer) -> Bool {
        for index in 0..<slots where named[index] == pointer {
            return true
        }
        return false
    }
}

/// A participant's slot in a domain. It holds the slot until it ends, and publishes what its owner is about
/// to touch.
final class Hazard: @unchecked Sendable {
    private let domain: HazardDomain
    private let index: Int

    fileprivate init(domain: HazardDomain, index: Int) {
        self.domain = domain
        self.index = index
    }

    deinit {
        clear()
        domain.give(index)
    }

    /// Reads `source` and protects what it points to: publishes it, then checks that `source` still holds
    /// it, and tries again if not. The object returned is not given up while the hazard names it, unless it
    /// is nil.
    func protect(_ source: borrowing Atomic<UnsafeMutableRawPointer?>) -> UnsafeMutableRawPointer? {
        let slot = domain.cell(index)
        var pointer = source.load(ordering: .acquiring)
        while true {
            // Both are sequentially consistent, so in the one order they share the publication comes before
            // the check, and no fence is needed between.
            slot.pointee.store(pointer, ordering: .sequentiallyConsistent)
            let again = source.load(ordering: .sequentiallyConsistent)
            if again == pointer {
                return pointer
            }
            pointer = again
        }
    }

    /// Stops protecting what the hazard names. Called when the participant is done with it, so it does not
    /// hold the segment back longer than it uses it.
    func clear() {
        domain.cell(index).pointee.store(nil, ordering: .releasing)
    }
}
