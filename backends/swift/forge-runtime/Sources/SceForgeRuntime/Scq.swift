// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Synchronization

// The ring's entries are `Atomic<Int>` words used as 64 bits: the cycle, the IsSafe bit and the index are
// packed into one, and the wrap bound below is a count of 2^62. On a processor whose `Int` is narrower the
// words would not hold them, and on one without a lock-free 64-bit compare-and-swap the atomics would
// run under a lock and the row would give `blocking` where the document asked for more. So the file
// compiles only for the processors that have both, and says why on the others (SCE Protocol-Synthesis RFC
// section synth-5-P, Counter width).
#if !(arch(x86_64) || arch(arm64) || arch(riscv64) || arch(powerpc64le) || arch(s390x))
#error(
    "the SCQ queue needs a 64-bit Int and lock-free 64-bit atomics, which this processor is not known to have (SCE Protocol-Synthesis RFC section synth-5-P, Counter width)"
)
#endif

/// How many operations a participant may be delayed between reading an entry and acting on it before it
/// could be misled: 2^(w-2) for a word of w bits, independent of the capacity. An entry's cycle is
/// compared with a ticket's modulo 2^c, where c = w - 1 - log2(2R), so a delay is safe until the entry's
/// cycle has moved by half that range, 2^(c-1); each cycle takes 2R operations; the product is 2^(w-2).
/// At 10^9 operations a second 2^62 is 146 years (SCE Protocol-Synthesis RFC section synth-5-P, *Counter
/// width*).
public let scqWrapBoundOps: Int = 1 << 62

/// The largest ring that leaves an entry enough bits for its cycle.
let scqMaxRingSlots: Int = 1 << 30

/// How many entries one 64-byte cache line holds, as a power of two. Consecutive tickets are spread over
/// lines by rotating the ticket's bits by this much; a ring smaller than a line does not rotate.
private let lineShift: Int = 3

/// How often a dequeue that found an entry still empty looks again before it makes the entry unusable for
/// the enqueue that owns it. Bounded, so the operation stays lock-free. Past the first look again, it looks
/// only while an enqueue holds the entry's ticket (the tail is past it): with none, no look can find the
/// entry filled, and an empty ring would spend the whole count on every dequeue.
///
/// It is patience and not part of the correctness argument: any count, none included, only changes how
/// soon the entry is given up. A ring takes its count as a parameter, so that a test can pass a small one;
/// every other ring uses this.
let scqSpins: Int = 10_000

/// Whether `x` is before `y` in the order that wraps: the signed difference. Tickets and cycles are
/// compared this way throughout. The subtraction wraps, as it must: the words are used as unsigned.
private func before(_ x: Int, _ y: Int) -> Bool { x &- y < 0 }

/// Whether `x` is `y` or before it, in the same order.
private func atOrBefore(_ x: Int, _ y: Int) -> Bool { x &- y <= 0 }

/// The value a dequeue reports for "nothing": an index is never negative.
private let none: Int = -1

/// The tail's top bit, set once a ring is closed: an enqueue that takes a ticket from a closed tail gets
/// the bit back and puts nothing on the ring. This is the finalize bit of Nikolaev's LSCQ. The tickets are
/// the bits below it, and a 64-bit counter never reaches it (`scqWrapBoundOps`).
private let fin: Int = Int.min

/// Every bit of a word but the closing bit.
private let ticketBits: Int = Int.max

/// One SCQ ring of R slots (R a power of two) and 2R entries.
///
/// An entry is cycle | IsSafe | index: the index in the low log2(2R) bits, the IsSafe bit above it, the
/// cycle in the rest. An entry whose index bits are all ones holds nothing. The words are `Int`s used as
/// unsigned: nothing in the algorithm orders them but `before` and `atOrBefore`, which are the signed
/// difference, and every shift that must not extend a sign is a logical one.
///
/// Every access is sequentially consistent, which is stronger than the algorithm requires and the ordering
/// the Kotlin runtime has on the JVM; the algorithm is correct under it (SCE Protocol-Synthesis RFC section
/// synth-5-P, *Backends*).
///
/// The class is `@unchecked Sendable`: after the initializer it mutates nothing but the atomics, and every
/// access to one is an atomic operation. The pointer is never reallocated and never escapes the class.
final class ScqRing: @unchecked Sendable {
    private let spins: Int

    /// The next ticket a dequeue takes.
    private let head = Atomic<Int>(0)

    /// How many failed dequeues remain before the ring may be called empty; negative means it is empty.
    private let threshold = Atomic<Int>(0)

    /// The next ticket an enqueue takes.
    private let tail = Atomic<Int>(0)

    /// The number of entries, 2R; also the value of the IsSafe bit.
    private let entryCount: Int

    /// The bits an index occupies.
    private let indexMask: Int

    /// The index bits and the IsSafe bit: everything below the cycle.
    private let lowMask: Int

    /// log2(2R).
    private let entryOrder: Int

    /// 3R - 1: that many failed dequeues are enough to show that an empty-looking ring is empty.
    private let thresholdAfterEnqueue: Int

    private let entries: UnsafeMutablePointer<Atomic<Int>>

    init(slots: Int, filled: Int, spins: Int) {
        let count = 2 * slots
        let mask = count - 1
        let order = count.trailingZeroBitCount
        let storage = UnsafeMutablePointer<Atomic<Int>>.allocate(capacity: count)
        for position in 0..<count {
            (storage + position).initialize(
                to: Atomic<Int>(Self.initial(position, filled, count, mask, order)))
        }
        self.spins = spins
        entryCount = count
        indexMask = mask
        lowMask = 2 * count - 1
        entryOrder = order
        thresholdAfterEnqueue = 3 * slots - 1
        entries = storage
        threshold.store(filled == 0 ? -1 : thresholdAfterEnqueue, ordering: .sequentiallyConsistent)
        tail.store(filled, ordering: .sequentiallyConsistent)
    }

    deinit {
        entries.deinitialize(count: entryCount)
        entries.deallocate()
    }

    /// The entry a ticket names: the ticket rotated so that consecutive tickets land on different cache
    /// lines.
    private func mapTicket(_ ticket: Int) -> Int {
        if entryOrder >= lineShift {
            return ((ticket & indexMask) >> (entryOrder - lineShift)) | ((ticket << lineShift) & indexMask)
        }
        return ticket & indexMask
    }

    /// The ticket `mapTicket` sends to this entry.
    private static func unmap(_ position: Int, _ indexMask: Int, _ entryOrder: Int) -> Int {
        if entryOrder >= lineShift {
            return ((position & indexMask) >> lineShift) | ((position << (entryOrder - lineShift)) & indexMask)
        }
        return position & indexMask
    }

    /// What the entry at `position` holds in a ring built with `filled` indices: the first filled tickets'
    /// entries hold index ticket in cycle 0, safe; the rest hold nothing, in the cycle before it.
    private static func initial(
        _ position: Int, _ filled: Int, _ entryCount: Int, _ indexMask: Int, _ entryOrder: Int
    ) -> Int {
        let ticket = unmap(position, indexMask, entryOrder)
        return ticket < filled ? entryCount + ticket : -1
    }

    private func load(_ cell: UnsafeMutablePointer<Atomic<Int>>) -> Int {
        cell.pointee.load(ordering: .sequentiallyConsistent)
    }

    private func compareAndSet(_ cell: UnsafeMutablePointer<Atomic<Int>>, from old: Int, to new: Int) -> Bool {
        cell.pointee.compareExchange(expected: old, desired: new, ordering: .sequentiallyConsistent).exchanged
    }

    /// Closes the ring: an enqueue that takes its ticket after this one's fetch-or puts nothing on it. An
    /// enqueue that already holds a ticket may still complete; the dequeues that follow either find its
    /// entry or make it unusable, which sends that enqueue to the next ticket and so to a refusal.
    func close() {
        tail.bitwiseOr(fin, ordering: .sequentiallyConsistent)
    }

    /// Puts `index` on the ring and returns true, or returns false, putting nothing, when the ring is
    /// closed. An open ring never refuses: at most R indices circulate through it, and an enqueue that
    /// finds its entry unusable takes the next ticket.
    func enqueue(_ index: Int) -> Bool {
        let stored = index ^ indexMask
        while true {
            // Looking first keeps a closed ring's tail from counting attempts that cannot succeed; the
            // ticket's own bit is what decides.
            if tail.load(ordering: .sequentiallyConsistent) & fin != 0 {
                return false
            }
            let ticket = tail.wrappingAdd(1, ordering: .sequentiallyConsistent).oldValue
            if ticket & fin != 0 {
                return false
            }
            let ticketCycle = (ticket << 1) | lowMask
            let slot = entries + mapTicket(ticket)
            var entry = load(slot)
            // The entry is ours to fill when it is empty, from an earlier cycle, and either safe or not
            // yet passed by the head.
            while true {
                let entryCycle = entry | lowMask
                let usable =
                    before(entryCycle, ticketCycle)
                    && (entry == entryCycle
                        || (entry == (entryCycle ^ entryCount)
                            && atOrBefore(head.load(ordering: .sequentiallyConsistent), ticket)))
                if !usable {
                    break
                }
                if compareAndSet(slot, from: entry, to: ticketCycle ^ stored) {
                    if threshold.load(ordering: .sequentiallyConsistent) != thresholdAfterEnqueue {
                        threshold.store(thresholdAfterEnqueue, ordering: .sequentiallyConsistent)
                    }
                    return true
                }
                // The entry changed; judge what it is now.
                entry = load(slot)
            }
        }
    }

    /// Moves tail back to head after dequeues overshot an empty ring, so that the next enqueue does not
    /// start a cycle ahead of what is read. `observedTail` is the word as read, closing bit included, and
    /// the word written keeps the bit: a ring that was closed stays closed.
    private func catchUp(_ observedTail: Int, _ observedHead: Int) {
        var tailSeen = observedTail
        var headSeen = observedHead
        while !tail.compareExchange(
            expected: tailSeen, desired: headSeen | (tailSeen & fin), ordering: .sequentiallyConsistent
        ).exchanged {
            headSeen = head.load(ordering: .sequentiallyConsistent)
            tailSeen = tail.load(ordering: .sequentiallyConsistent)
            if !before(tailSeen & ticketBits, headSeen) {
                break
            }
        }
    }

    /// Reads the entry a dequeue's ticket names until it is settled: the index when the entry was this
    /// ticket's (the entry is left empty in the same cycle), `none` when the ticket found nothing and the
    /// caller goes on to the empty check.
    private func look(_ slot: UnsafeMutablePointer<Atomic<Int>>, _ ticket: Int) -> Int {
        let ticketCycle = (ticket << 1) | lowMask
        var looked = 0
        // Whether an enqueue holds this ticket, asked once, before the second look again: the tail only
        // moves on, so an enqueue that holds it then is the only one a look could wait for.
        var held: Bool?
        while true {
            var entry = load(slot)
            inner: while true {
                let entryCycle = entry | lowMask
                if entryCycle == ticketCycle {
                    // The entry is this ticket's: take its index and leave the entry empty in the same
                    // cycle.
                    slot.pointee.bitwiseOr(indexMask, ordering: .sequentiallyConsistent)
                    return entry & indexMask
                }
                let replacement: Int
                if (entry | entryCount) != entryCycle {
                    // It holds an index of another cycle: mark it unsafe, so no enqueue of this cycle's
                    // ticket fills the entry behind the head. Already unsafe: nothing to do.
                    replacement = entry & ~entryCount
                    if entry == replacement {
                        return none
                    }
                } else {
                    // It is empty. An enqueue holding this cycle's ticket may be about to fill it; look a
                    // few times, then move the entry to this cycle so that enqueue fails.
                    looked += 1
                    if looked <= spins {
                        if held == nil && looked > 1 {
                            held = before(ticket, tail.load(ordering: .sequentiallyConsistent) & ticketBits)
                        }
                        if looked == 1 || held == true {
                            break inner  // look again from a fresh read
                        }
                    }
                    replacement = ticketCycle ^ (~entry & entryCount)
                }
                if !before(entryCycle, ticketCycle) {
                    return none
                }
                if compareAndSet(slot, from: entry, to: replacement) {
                    return none
                }
                entry = load(slot)
            }
        }
    }

    /// Takes the oldest index off the ring, or returns `none` (-1) when it is empty. The empty answer may
    /// come from the threshold alone, which does not look at a ticket an enqueue holds and has not yet
    /// filled. With `byThreshold` false the threshold neither ends the dequeue early nor is spent by it:
    /// the head walks to the tail, which a closed ring no longer moves, so an enqueue still holding a
    /// ticket either finds its entry taken from it (and takes the next ticket, which is refused) or has
    /// already filled it, and then the walk returns its index.
    func dequeue(byThreshold: Bool = true) -> Int {
        if byThreshold && threshold.load(ordering: .sequentiallyConsistent) < 0 {
            return none
        }
        while true {
            let ticket = head.wrappingAdd(1, ordering: .sequentiallyConsistent).oldValue
            let index = look(entries + mapTicket(ticket), ticket)
            if index != none {
                return index
            }

            let tailSeen = tail.load(ordering: .sequentiallyConsistent)
            if atOrBefore(tailSeen & ticketBits, ticket &+ 1) {
                catchUp(tailSeen, ticket &+ 1)
                if byThreshold {
                    threshold.wrappingAdd(-1, ordering: .sequentiallyConsistent)
                }
                return none
            }
            // The new value is below zero exactly when the value before was not above it.
            if byThreshold && threshold.wrappingAdd(-1, ordering: .sequentiallyConsistent).newValue < 0 {
                return none
            }
        }
    }
}

/// The value a ring reports for "no index".
let scqNone: Int = none

/// A bounded queue of at most `capacity` elements for any number of producers and any number of consumers,
/// lock-free. Exactly `capacity` fit when nothing else is running.
///
/// The elements live in an array of capacity slots. Two rings of indices make it a data queue: the free
/// ring holds the indices of the slots nobody is using and starts with all of them; the allocated ring
/// holds the indices of the slots that hold an element, in the order they were filled, and starts empty. A
/// push takes an index from the free ring (none: the queue is full), writes its element into that slot and
/// puts the index on the allocated ring; a pop does the reverse. The rings carry only indices.
///
/// The ring has `ringSlots` slots, a power of two at least `capacity`, and at least the most producers or
/// consumers that will work the queue at once. The algorithm's empty test is justified for at most
/// ringSlots enqueuers and at most ringSlots dequeuers working one ring at once (Nikolaev 2019, section
/// 5.1); beyond that a completed push can leave the threshold at -1 with its element in the ring and every
/// pop reports the queue empty until another push completes. So a queue hands out at most `ringSlots`
/// producer handles and at most `ringSlots` consumer handles at a time.
///
/// A push may be refused while others hold slots: a slot's index goes back to the free ring only after the
/// pop that took the element has read it, and a push holds the index it took until it has published its
/// element. Waiting for the operation that holds the slot would not be lock-free, since that participant
/// may be stopped. The shortfall is at most one slot per other participant.
///
/// **Sendable.** The class is `@unchecked Sendable`: after the initializer it mutates only the two rings
/// and the places, which are atomics, and the slots, each of which belongs to exactly one participant at a
/// time (the one that holds its index). The pointer is never reallocated and never escapes the class.
///
/// An initializer given a capacity below one, a `ringSlots` that is not a power of two, is below
/// `capacity`, or is more than a ring can number, stops the program: it is a defect of the caller and not
/// an input.
public final class Scq<Element: Sendable>: @unchecked Sendable {
    /// The number of elements the queue holds when full.
    public let capacity: Int

    /// The slots of each ring.
    public let ringSlots: Int

    /// The indices of the slots that hold an element, oldest first.
    private let allocated: ScqRing

    /// The indices of the slots nobody is using.
    private let free: ScqRing

    fileprivate let producers = Atomic<Int>(0)
    fileprivate let consumers = Atomic<Int>(0)

    private let slots: UnsafeMutablePointer<Element?>

    /// A queue whose rings spin `scqSpins` times on an entry that is still empty.
    public convenience init(capacity: Int, ringSlots: Int) {
        self.init(capacity: capacity, ringSlots: ringSlots, spins: scqSpins)
    }

    init(capacity: Int, ringSlots: Int, spins: Int) {
        Self.checkShape(capacity: capacity, ringSlots: ringSlots)
        self.capacity = capacity
        self.ringSlots = ringSlots
        allocated = ScqRing(slots: ringSlots, filled: 0, spins: spins)
        free = ScqRing(slots: ringSlots, filled: capacity, spins: spins)
        let storage = UnsafeMutablePointer<Element?>.allocate(capacity: capacity)
        storage.initialize(repeating: nil, count: capacity)
        slots = storage
    }

    deinit {
        slots.deinitialize(count: capacity)
        slots.deallocate()
    }

    /// Refuses a shape no ring can have, by stopping the program. A list of rings checks it before it asks
    /// its allocator for a segment, so a bad shape is not reported as an allocator's refusal.
    static func checkShape(capacity: Int, ringSlots: Int) {
        precondition(capacity >= 1, "a queue's capacity must be at least one element")
        precondition(
            ringSlots >= 1 && ringSlots & (ringSlots - 1) == 0, "the ring size must be a power of two")
        precondition(ringSlots >= capacity, "the ring must have at least as many slots as the capacity")
        precondition(
            ringSlots <= scqMaxRingSlots, "a ring this large leaves its entries too few bits for the cycle")
    }

    /// Takes `value` and returns true, or returns false and takes nothing: when the queue has no free slot
    /// it is closed, and when it is closed already nothing more is ever pushed into it, which is what lets
    /// a list of rings put every later element behind this ring's. The queue's places are not consulted:
    /// the list of rings that owns this ring keeps its own. Lock-free.
    func pushOrClose(_ value: Element) -> Bool {
        let index = free.dequeue()
        if index == scqNone {
            allocated.close()
            return false
        }
        slots[index] = value
        if allocated.enqueue(index) {
            return true
        }
        // The ring was closed under the push: the index is still ours, and goes back to the free ring,
        // which is never closed.
        slots[index] = nil
        _ = free.enqueue(index)
        return false
    }

    /// Takes the oldest element, or returns `nil` when the ring is empty. Lock-free.
    func popOrNil() -> Element? {
        takeFrom(allocated.dequeue())
    }

    /// Takes the oldest element of a closed ring, or returns `nil` only when no element can ever arrive
    /// in it again. A ring is given up only after this answers `nil`: until then an element may still
    /// arrive in it. Lock-free.
    func popDrained() -> Element? {
        takeFrom(allocated.dequeue(byThreshold: false))
    }

    /// The element of the slot `index` named, which came off the allocated ring, so the slot holds a
    /// pushed element and no other participant holds it until the enqueue below hands it on. The slot is
    /// cleared so the queue keeps no copy of an element that has left it.
    private func takeFrom(_ index: Int) -> Element? {
        if index == scqNone {
            return nil
        }
        guard let value = slots[index] else {
            preconditionFailure("a slot named by the allocated ring holds a pushed element")
        }
        slots[index] = nil
        _ = free.enqueue(index)
        return value
    }

    /// A producer handle, or `nil` when every producer place is taken: the ring cannot be shared by more
    /// enqueuers than it has slots. Drop the handle, or call its `release()`, to give the place back.
    public func producer() -> ScqProducer<Element>? {
        takePlace(producers, limit: ringSlots) ? ScqProducer(queue: self) : nil
    }

    /// A consumer handle, or `nil` when every consumer place is taken. Drop the handle, or call its
    /// `release()`, to give the place back.
    public func consumer() -> ScqConsumer<Element>? {
        takePlace(consumers, limit: ringSlots) ? ScqConsumer(queue: self) : nil
    }

    fileprivate func push(_ value: Element) -> PushStatus {
        let index = free.dequeue()
        if index == scqNone {
            return .full
        }
        // The index came off the free ring, so no other participant holds this slot until the enqueue
        // below hands it on.
        slots[index] = value
        _ = allocated.enqueue(index)
        return .ok
    }
}

/// A producing side of a `Scq`. It holds one of the queue's producer places until it is released.
public final class ScqProducer<Element: Sendable>: @unchecked Sendable {
    private var queue: Scq<Element>?

    fileprivate init(queue: Scq<Element>) {
        self.queue = queue
    }

    deinit {
        release()
    }

    /// Another producer of the same queue, or `nil` when every place is taken.
    public func tryClone() -> ScqProducer<Element>? {
        guard let queue else {
            preconditionFailure("the producer was released")
        }
        return queue.producer()
    }

    /// Takes `value`, or reports `.full` when the queue holds its capacity, or while other participants'
    /// operations hold the slots it is short of. Lock-free: some operation completes in a bounded number
    /// of steps whatever the other participants are doing.
    public func tryPush(_ value: Element) -> PushStatus {
        guard let queue else {
            preconditionFailure("the producer was released")
        }
        return queue.push(value)
    }

    /// The capacity of the queue this side fills.
    public var capacity: Int {
        guard let queue else {
            preconditionFailure("the producer was released")
        }
        return queue.capacity
    }

    /// Gives the producer place back. The handle is not usable afterwards.
    public func release() {
        if let queue {
            queue.producers.wrappingAdd(-1, ordering: .sequentiallyConsistent)
        }
        queue = nil
    }
}

/// A consuming side of a `Scq`. It holds one of the queue's consumer places until it is released.
public final class ScqConsumer<Element: Sendable>: @unchecked Sendable {
    private var queue: Scq<Element>?

    fileprivate init(queue: Scq<Element>) {
        self.queue = queue
    }

    deinit {
        release()
    }

    /// Another consumer of the same queue, or `nil` when every place is taken.
    public func tryClone() -> ScqConsumer<Element>? {
        guard let queue else {
            preconditionFailure("the consumer was released")
        }
        return queue.consumer()
    }

    /// Takes the oldest element, or returns `nil` when the queue is empty. Lock-free.
    public func tryPop() -> Element? {
        guard let queue else {
            preconditionFailure("the consumer was released")
        }
        return queue.popOrNil()
    }

    /// The capacity of the queue this side drains.
    public var capacity: Int {
        guard let queue else {
            preconditionFailure("the consumer was released")
        }
        return queue.capacity
    }

    /// Gives the consumer place back. The handle is not usable afterwards.
    public func release() {
        if let queue {
            queue.consumers.wrappingAdd(-1, ordering: .sequentiallyConsistent)
        }
        queue = nil
    }
}
