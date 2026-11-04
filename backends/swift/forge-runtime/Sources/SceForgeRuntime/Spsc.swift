// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Synchronization

/// A bounded queue of exactly `capacity` elements for one producer and one consumer: the `bounded` row
/// for that cardinality (SCE Protocol-Synthesis RFC section synth-5-P), a Lamport ring, wait-free on both
/// sides.
///
/// **Lap indices, not counters.** Each side's index runs over `0..<2N` and the slot it names is the index
/// modulo N. Two laps are what tell a full ring from an empty one without spending a slot: the indices are
/// equal exactly when the ring is empty and N apart exactly when it is full, so the ring holds exactly the
/// capacity declared. An index never reaches 2N, so nothing wraps however long the queue runs.
///
/// **Ordering.** The producer publishes a filled slot by storing its index with release and the consumer
/// loads that index with acquire before it reads the slot, so the write of an element happens-before its
/// pop; the consumer returns a slot the same way in the other direction. Each side keeps the other's index
/// as last read, and reads the shared one again only when the cached one says the ring is full (producer)
/// or empty (consumer).
///
/// **Sendable.** The class is `@unchecked Sendable`, and that is sound: after the initializer it mutates
/// only the two indices, which are atomics, and the slots, each of which belongs to exactly one side at a
/// time (the producer to the slots outside `head..<tail`, the consumer to the slots inside it); the
/// protocol's one assumption, a single producer and a single consumer, is what `producer()` and
/// `consumer()` enforce. The pointer is never reallocated and never escapes the class.
///
/// An initializer given a `capacity` below one, or past what the lap indices can count, stops the program:
/// it is a defect of the caller and not an input.
public final class Spsc<Element: Sendable>: @unchecked Sendable {
    /// The number of elements the queue holds when full.
    public let capacity: Int

    /// The lap index of the next slot the consumer pops. Written only by the consumer.
    private let head = Atomic<Int>(0)

    /// The lap index of the next slot the producer fills. Written only by the producer.
    private let tail = Atomic<Int>(0)

    fileprivate let producerClaimed = Atomic<Bool>(false)
    fileprivate let consumerClaimed = Atomic<Bool>(false)

    /// The size of the lap-index space, twice the capacity.
    fileprivate let laps: Int
    private let slots: UnsafeMutablePointer<Element?>

    public init(capacity: Int) {
        precondition(capacity >= 1, "a queue's capacity must be at least one element")
        let (doubled, overflow) = capacity.multipliedReportingOverflow(by: 2)
        precondition(!overflow, "a queue of \(capacity) elements has lap indices that cannot be counted")
        self.capacity = capacity
        self.laps = doubled
        let storage = UnsafeMutablePointer<Element?>.allocate(capacity: capacity)
        storage.initialize(repeating: nil, count: capacity)
        self.slots = storage
    }

    deinit {
        slots.deinitialize(count: capacity)
        slots.deallocate()
    }

    /// The lap index after `index`.
    fileprivate func next(_ index: Int) -> Int {
        index + 1 == laps ? 0 : index + 1
    }

    /// The position in the storage that a lap index names.
    private func slot(_ index: Int) -> Int {
        index >= capacity ? index - capacity : index
    }

    /// How many elements lie between a consumer index and a producer index.
    fileprivate func occupancy(_ head: Int, _ tail: Int) -> Int {
        tail >= head ? tail - head : tail + laps - head
    }

    /// The producing side, or `nil` while another is held: a ring shared by two producers is not this
    /// algorithm. Drop the handle, or call its `release()`, to give the place back.
    public func producer() -> SpscProducer<Element>? {
        if producerClaimed.exchange(true, ordering: .acquiring) {
            return nil
        }
        return SpscProducer(queue: self, tail: tail.load(ordering: .acquiring), cachedHead: head.load(ordering: .acquiring))
    }

    /// The consuming side, or `nil` while another is held. Drop the handle, or call its `release()`, to
    /// give the place back.
    public func consumer() -> SpscConsumer<Element>? {
        if consumerClaimed.exchange(true, ordering: .acquiring) {
            return nil
        }
        return SpscConsumer(queue: self, head: head.load(ordering: .acquiring), cachedTail: tail.load(ordering: .acquiring))
    }

    // The accessors below are the handles' own: they are the only readers and writers of the shared
    // indices and slots, in the orderings the algorithm states.

    fileprivate func loadHead() -> Int { head.load(ordering: .acquiring) }
    fileprivate func loadTail() -> Int { tail.load(ordering: .acquiring) }
    fileprivate func publishTail(_ value: Int) { tail.store(value, ordering: .releasing) }
    fileprivate func publishHead(_ value: Int) { head.store(value, ordering: .releasing) }

    fileprivate func put(_ value: Element, at index: Int) {
        slots[slot(index)] = value
    }

    /// Takes the element out of the slot, clearing it so the queue keeps no copy of an element that has
    /// left it.
    fileprivate func take(at index: Int) -> Element {
        let position = slot(index)
        guard let value = slots[position] else {
            preconditionFailure("a slot inside head..<tail holds a pushed element")
        }
        slots[position] = nil
        return value
    }
}

/// The producing side of a `Spsc`. There is one at a time, and it is for one thread at a time.
public final class SpscProducer<Element: Sendable>: @unchecked Sendable {
    private var queue: Spsc<Element>?
    /// This side's own index; the shared one only publishes it.
    private var tail: Int
    /// The consumer's index as this side last read it.
    private var cachedHead: Int

    fileprivate init(queue: Spsc<Element>, tail: Int, cachedHead: Int) {
        self.queue = queue
        self.tail = tail
        self.cachedHead = cachedHead
    }

    deinit {
        release()
    }

    /// Takes `value`, or reports `.full` when the queue holds its capacity. Wait-free: a bounded number of
    /// steps whatever the consumer is doing.
    public func tryPush(_ value: Element) -> PushStatus {
        guard let queue else {
            preconditionFailure("the producer was released")
        }
        if queue.occupancy(cachedHead, tail) == queue.capacity {
            // Read the consumer's index again: its read of the slot about to be reused happens-before
            // the write below.
            cachedHead = queue.loadHead()
            if queue.occupancy(cachedHead, tail) == queue.capacity {
                return .full
            }
        }
        // The slot at tail is outside head..<tail, so the consumer does not read it until the store
        // below publishes it.
        queue.put(value, at: tail)
        tail = queue.next(tail)
        // The write of the slot happens-before the pop of any consumer that loads this index.
        queue.publishTail(tail)
        return .ok
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
        queue?.producerClaimed.store(false, ordering: .releasing)
        queue = nil
    }
}

/// The consuming side of a `Spsc`. There is one at a time, and it is for one thread at a time.
public final class SpscConsumer<Element: Sendable>: @unchecked Sendable {
    private var queue: Spsc<Element>?
    private var head: Int
    /// The producer's index as this side last read it.
    private var cachedTail: Int

    fileprivate init(queue: Spsc<Element>, head: Int, cachedTail: Int) {
        self.queue = queue
        self.head = head
        self.cachedTail = cachedTail
    }

    deinit {
        release()
    }

    /// Takes the oldest element, or returns `nil` when the queue is empty. Wait-free: a bounded number of
    /// steps whatever the producer is doing.
    public func tryPop() -> Element? {
        guard let queue else {
            preconditionFailure("the consumer was released")
        }
        if head == cachedTail {
            // Read the producer's index again: its write of the slot happens-before the read below.
            cachedTail = queue.loadTail()
            if head == cachedTail {
                return nil
            }
        }
        // The slot at head is inside head..<tail, so it holds a pushed element, and the producer does not
        // reuse it until the store below returns it.
        let value = queue.take(at: head)
        head = queue.next(head)
        // The read of the slot happens-before the producer's reuse of it, by any producer that loads this
        // index.
        queue.publishHead(head)
        return value
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
        queue?.consumerClaimed.store(false, ordering: .releasing)
        queue = nil
    }
}
