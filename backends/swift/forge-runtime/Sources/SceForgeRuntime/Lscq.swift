// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Synchronization

/// One segment: a ring and the link to the next. The queue names a segment by the address of the object,
/// which is also the address the hazard slots and the domain's list hold.
///
/// The segment owns its successor: `next` holds one retain of it, given up when this segment ends, so a
/// segment is alive while the queue's head or an earlier segment names it.
final class LscqSegment<Element: Sendable>: Retirable {
    /// The segment after this one (an owned retain), or nil while this is the newest. Written once, by the
    /// producer whose compare-and-swap links a successor.
    let next = Atomic<UnsafeMutableRawPointer?>(nil)

    let ring: Scq<Element>

    /// What gave the segment, and what takes it back when the domain gives it up.
    private let allocator: any SegmentAllocator

    init(segment: Int, ringSlots: Int, allocator: any SegmentAllocator) {
        ring = Scq<Element>(capacity: segment, ringSlots: ringSlots)
        self.allocator = allocator
        super.init()
    }

    deinit {
        endChain(after: next.exchange(nil, ordering: .acquiring)) { (segment: LscqSegment<Element>) in
            segment.next.exchange(nil, ordering: .acquiring)
        }
    }

    /// The segment goes back to the allocator, once, when the domain finds nothing naming it.
    override func willReclaim() {
        allocator.deallocate(self)
    }
}

/// The `segmented` row for any cardinality other than one producer and one consumer (SCE Protocol-Synthesis
/// RFC section synth-5-P): LSCQ, a list of SCQ rings, lock-free on both sides and bounded only by the
/// allocator it is given.
///
/// **The algorithm** is Nikolaev's LSCQ (DISC 2019, section 6): a Michael-Scott list whose nodes are SCQ
/// rings instead of single elements. Producers push into the newest ring (`tail`); consumers pop from the
/// oldest (`head`). A ring that cannot take another element is closed: nothing is ever pushed into it
/// again. The producer that found it closed takes a segment from the allocator, puts its element in it
/// before anyone can see it, and links it behind the closed one with a compare-and-swap on that ring's
/// `next`; a producer that loses that race gives its segment back and pushes into the winner's. A consumer
/// that finds the oldest ring empty and closed moves `head` to its successor and retires it.
///
/// **Each ring is the SCQ data queue** of `Scq.swift`, which keeps its elements in a slot array and moves
/// their indices through two rings, so an element is written into its slot once and read out of it once.
///
/// **Closing.** The close is a bit in the allocated ring's tail, set with a fetch-or, and an enqueue takes
/// its ticket with a fetch-add that returns the bit: a ticket granted before the close can still be used,
/// one after it is refused. That is what makes the order across segments exact.
///
/// **Retiring a ring waits for the elements in flight.** A consumer sees a ring empty through SCQ's
/// threshold, which does not look at a ticket an enqueue holds and has not yet filled. So the consumer that
/// finds the oldest ring empty and closed pops it once more with `Scq.popDrained`, which walks the head to
/// the closed tail and either takes the element such an enqueue filled or makes its entry unusable, and
/// only when that answers empty does it move `head`.
///
/// **Reclamation.** Swift has no collector, and a segment is reference counted, so two things keep it
/// alive: the references, and a hazard for the window between reading a pointer and retaining it. The list
/// owns the references: each segment's `next` holds one retain of its successor, and the `head` slot holds
/// one retain of the oldest segment. `tail` holds none: the newest segment is held by the one before it, or
/// by `head`. The consumer whose compare-and-swap moves `head` past a segment retires it: the retain the
/// `head` slot held on it goes to the hazard domain, which gives the segment back to the allocator and ends
/// that retain once no participant's hazard names it. A stalled participant that still holds the segment
/// keeps only that segment alive (RFC section synth-5-P, *Reclamation domain*).
///
/// **Where a hazard is validated.** A hazard protects a segment only if the location it is validated
/// against is the one the unlinking thread writes before the scan runs. A consumer validates against
/// `head`, which the retiring consumer writes. A producer validates against `tail`, which that consumer does
/// not write: left alone, `tail` can still name a segment that `head` has left and that has been retired
/// (the producer that linked its successor has not yet moved `tail`), so a producer could publish its
/// hazard after the scan read its slot, find `tail` unchanged, and use the segment once the scan gave it up.
/// So a consumer moves `tail` off the segment, to its successor, before it moves `head` past it, and every
/// write of `head` and `tail` is sequentially consistent. Then `tail` does not name a segment by the time it
/// is retired, and any producer whose check still saw it there published its hazard before the retirement,
/// which the scan reads after it.
///
/// **Progress.** Push and pop are lock-free: a retry means another participant moved `head`, `tail` or a
/// ring, and the helping step lets a push finish what a stalled one started. A push is as strong as the
/// allocator it calls (`SegmentAllocator.progress`, held to the declared rank by `create`); a refusal
/// reports `.outOfMemory` and leaves the element with the caller. Before it is reported the domain is
/// asked to give back the retired segments no hazard names (`takeSegment`), because they hold the
/// allocator's budget until a scan, and a scan runs only on a retirement.
///
/// **Sizes.** Each ring is an `Scq` of `segment` elements over `ringSlots` slots, a power of two at least
/// `segment` and at least the most producers or consumers that work the queue at once. The domain has a slot
/// per live handle, both sides together.
///
/// **Sendable.** The class is `@unchecked Sendable`: after the initializer it mutates nothing but atomics,
/// each ring is touched by many participants through its own atomic operations, and a segment's memory is
/// kept alive by the references above and the hazards the handles hold.
public final class Lscq<Element: Sendable>: @unchecked Sendable {
    /// The elements one segment holds.
    public let segment: Int

    /// The slots of each segment's rings.
    public let ringSlots: Int

    fileprivate let allocator: any SegmentAllocator

    /// What the segments the consumers retire are handed to. Held, so that it outlives every handle.
    fileprivate let domain: HazardDomain

    /// The oldest segment (the slot owns one retain of it). Moved forward by the consumer that finds it
    /// empty and closed.
    let head = Atomic<UnsafeMutableRawPointer?>(nil)

    /// The newest segment, or the one before it while a producer is between linking a successor and moving
    /// `tail`. Holds no retain.
    let tail = Atomic<UnsafeMutableRawPointer?>(nil)

    /// How many producer and consumer handles are alive.
    fileprivate let producers = Atomic<Int>(0)
    fileprivate let consumers = Atomic<Int>(0)

    private init(
        segment: Int, ringSlots: Int, allocator: any SegmentAllocator, domain: HazardDomain,
        first: LscqSegment<Element>
    ) {
        self.segment = segment
        self.ringSlots = ringSlots
        self.allocator = allocator
        self.domain = domain
        head.store(Unmanaged.passRetained(first).toOpaque(), ordering: .sequentiallyConsistent)
        tail.store(Unmanaged.passUnretained(first).toOpaque(), ordering: .sequentiallyConsistent)
    }

    /// An empty queue of segments of `segment` elements over `allocator` and `domain`, or nil when the
    /// allocator will not give the first segment. A shape no ring can have, and an allocator whose
    /// `progress` is below `required`, the progress the document declared for it, stop the program: they are
    /// a defect of the caller and not an input, and a queue that would claim more than its allocator gives
    /// does not build. The shape is checked before the allocator is asked, so a bad one is not reported as
    /// the allocator's refusal.
    public static func create(
        segment: Int, ringSlots: Int, allocator: any SegmentAllocator, domain: HazardDomain,
        required: QueueProgress
    ) -> Lscq<Element>? {
        Scq<Element>.checkShape(capacity: segment, ringSlots: ringSlots)
        precondition(
            allocator.progress >= required,
            "the allocator gives \(allocator.progress), less than the \(required) the document declared for it")
        guard
            let first = allocator.allocate({
                LscqSegment<Element>(segment: segment, ringSlots: ringSlots, allocator: allocator)
            })
        else {
            return nil
        }
        return Lscq(segment: segment, ringSlots: ringSlots, allocator: allocator, domain: domain, first: first)
    }

    deinit {
        // No handle is alive, so every segment from the head is this queue's alone. Each goes back to the
        // allocator once; the retain the `head` slot holds ends the chain, which then ends without
        // recursion.
        let first = head.load(ordering: .acquiring)
        var pointer = first
        while let current = pointer {
            let node = Unmanaged<LscqSegment<Element>>.fromOpaque(current).takeUnretainedValue()
            pointer = node.next.load(ordering: .acquiring)
            allocator.deallocate(node)
        }
        if let first {
            Unmanaged<LscqSegment<Element>>.fromOpaque(first).release()
        }
        // The segments consumers retired are on the domain's list and may still be waiting for a scan; they
        // go back before the allocator they name can go.
        domain.flush()
    }

    /// A producer handle, or nil when `ringSlots` producers are already alive or the domain has no free
    /// slot. Drop the handle, or call its `release()`, to give its place back.
    public func producer() -> LscqProducer<Element>? {
        guard takePlace(producers, limit: ringSlots) else {
            return nil
        }
        guard let hazard = domain.acquire() else {
            producers.wrappingAdd(-1, ordering: .sequentiallyConsistent)
            return nil
        }
        return LscqProducer(queue: self, hazard: hazard)
    }

    /// A consumer handle, or nil when `ringSlots` consumers are already alive or the domain has no free
    /// slot. Drop the handle, or call its `release()`, to give its place back.
    public func consumer() -> LscqConsumer<Element>? {
        guard takePlace(consumers, limit: ringSlots) else {
            return nil
        }
        guard let hazard = domain.acquire() else {
            consumers.wrappingAdd(-1, ordering: .sequentiallyConsistent)
            return nil
        }
        return LscqConsumer(queue: self, hazard: hazard)
    }

    /// The segment a protected pointer names. A pointer a hazard returned names a live segment: `head` and
    /// `tail` always name one.
    private func segmentOf(_ pointer: UnsafeMutableRawPointer?) -> LscqSegment<Element> {
        guard let pointer else {
            preconditionFailure("the head and the tail always name a segment")
        }
        return Unmanaged<LscqSegment<Element>>.fromOpaque(pointer).takeUnretainedValue()
    }

    /// A new segment from the allocator, or nil when it will not give one even after the domain has given
    /// back what it can. A refusal is not yet the allocator's last word: the segments the consumers have
    /// retired hold its budget until the domain scans, and the scan runs on a retirement, so a queue whose
    /// consumers have drained it would never run one. A producer that is refused therefore asks the domain
    /// to give back what no hazard names, and asks the allocator once more. Without it a push could be
    /// refused for as long as nobody pops, however empty the queue is.
    private func takeSegment() -> LscqSegment<Element>? {
        let make = {
            LscqSegment<Element>(segment: self.segment, ringSlots: self.ringSlots, allocator: self.allocator)
        }
        if let fresh = allocator.allocate(make) {
            return fresh
        }
        domain.collect()
        return allocator.allocate(make)
    }

    fileprivate func push(_ element: Element, hazard: Hazard) -> PushStatus {
        defer { hazard.clear() }
        while true {
            let tailPointer = hazard.protect(tail)
            let tailSegment = segmentOf(tailPointer)
            if let next = tailSegment.next.load(ordering: .acquiring) {
                // A producer linked a successor and has not moved `tail`: finish its step.
                _ = tail.compareExchange(
                    expected: tailPointer, desired: next, ordering: .sequentiallyConsistent)
                continue
            }
            if tailSegment.ring.pushOrClose(element) {
                return .ok
            }
            // The segment is closed. Put the element in a segment of our own before anyone can see it, then
            // try to link it behind this one.
            guard let fresh = takeSegment() else {
                return .outOfMemory
            }
            let pushed = fresh.ring.pushOrClose(element)
            precondition(pushed, "a new ring is open and has a free slot, so it takes an element")
            // The link's retain is taken before the segment can be seen, and given up if the link is not
            // made.
            let link = Unmanaged.passRetained(fresh).toOpaque()
            let linked = tailSegment.next.compareExchange(
                expected: nil, desired: link, ordering: .acquiringAndReleasing
            ).exchanged
            if linked {
                _ = tail.compareExchange(
                    expected: tailPointer, desired: link, ordering: .sequentiallyConsistent)
                return .ok
            }
            // Another producer linked first. The segment nobody saw goes back, with the copy of the element
            // it holds (the element is still ours, so nothing is taken out of it), and the push goes behind
            // the winner's.
            Unmanaged<LscqSegment<Element>>.fromOpaque(link).release()
            allocator.deallocate(fresh)
        }
    }

    fileprivate func pop(hazard: Hazard) -> Element? {
        defer { hazard.clear() }
        while true {
            let headPointer = hazard.protect(head)
            let headSegment = segmentOf(headPointer)
            if let value = headSegment.ring.popOrNil() {
                return value
            }
            guard let next = headSegment.next.load(ordering: .acquiring) else {
                return nil
            }
            // The segment is closed. An enqueue that took a ticket before the close may still be filling its
            // entry, so the empty answer above does not yet show the segment holds nothing and never will.
            if let value = headSegment.ring.popDrained() {
                return value
            }
            // Take `tail` off the segment before it is unlinked from `head` (the type's documentation,
            // *Where a hazard is validated*). The successor exists, so `tail` may legally move to it; if
            // another participant already moved it, the exchange fails and reads that move.
            _ = tail.compareExchange(
                expected: headPointer, desired: next, ordering: .sequentiallyConsistent)
            // The `head` slot will own a retain of the successor. It is taken now, while the segment this
            // consumer holds keeps the successor alive through its link, and given up if the slot is not
            // moved.
            let successor = Unmanaged<LscqSegment<Element>>.fromOpaque(next).retain()
            if head.compareExchange(
                expected: headPointer, desired: next, ordering: .sequentiallyConsistent
            ).exchanged {
                // This consumer moved `head` past the segment, so it alone retires it. Its own hazard goes
                // first, or the scan the retirement may start would keep the segment for it. The retain the
                // slot held on the segment goes to the domain.
                hazard.clear()
                domain.retire(adopting: Unmanaged<Retirable>.fromOpaque(headPointer!))
            } else {
                successor.release()
            }
        }
    }
}

/// The producing side of an `Lscq`. It holds one producer place and one slot of the domain until it is
/// released, and it is for one thread at a time.
public final class LscqProducer<Element: Sendable>: @unchecked Sendable {
    private var queue: Lscq<Element>?
    private var hazard: Hazard?

    fileprivate init(queue: Lscq<Element>, hazard: Hazard) {
        self.queue = queue
        self.hazard = hazard
    }

    deinit {
        release()
    }

    /// Another producer of the same queue, or nil when every place or slot is taken.
    public func tryClone() -> LscqProducer<Element>? {
        guard let queue else {
            preconditionFailure("the producer was released")
        }
        return queue.producer()
    }

    /// Takes `value`, or reports `.outOfMemory` when the newest segment is closed and the allocator will not
    /// give another; the element is the caller's either way. Lock-free when the allocator is.
    public func tryPush(_ value: Element) -> PushStatus {
        guard let queue, let hazard else {
            preconditionFailure("the producer was released")
        }
        return queue.push(value, hazard: hazard)
    }

    /// The elements one segment of the queue this side fills holds.
    public var segment: Int {
        guard let queue else {
            preconditionFailure("the producer was released")
        }
        return queue.segment
    }

    /// Gives the producer's place and slot back. The handle is not usable afterwards.
    public func release() {
        hazard = nil
        if let queue {
            queue.producers.wrappingAdd(-1, ordering: .sequentiallyConsistent)
        }
        queue = nil
    }
}

/// The consuming side of an `Lscq`. It holds one consumer place and one slot of the domain until it is
/// released, and it is for one thread at a time.
public final class LscqConsumer<Element: Sendable>: @unchecked Sendable {
    private var queue: Lscq<Element>?
    private var hazard: Hazard?

    fileprivate init(queue: Lscq<Element>, hazard: Hazard) {
        self.queue = queue
        self.hazard = hazard
    }

    deinit {
        release()
    }

    /// Another consumer of the same queue, or nil when every place or slot is taken.
    public func tryClone() -> LscqConsumer<Element>? {
        guard let queue else {
            preconditionFailure("the consumer was released")
        }
        return queue.consumer()
    }

    /// Takes the oldest element, or returns nil when the queue is empty. Lock-free.
    public func tryPop() -> Element? {
        guard let queue, let hazard else {
            preconditionFailure("the consumer was released")
        }
        return queue.pop(hazard: hazard)
    }

    /// The elements one segment of the queue this side drains holds.
    public var segment: Int {
        guard let queue else {
            preconditionFailure("the consumer was released")
        }
        return queue.segment
    }

    /// Gives the consumer's place and slot back. The handle is not usable afterwards.
    public func release() {
        hazard = nil
        if let queue {
            queue.consumers.wrappingAdd(-1, ordering: .sequentiallyConsistent)
        }
        queue = nil
    }
}
