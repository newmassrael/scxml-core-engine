// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Synchronization

/// One segment: `slots` filled once, in order. A Lamport ring that is never lapped, so it needs neither
/// the second lap that tells a full ring from an empty one nor a free slot.
///
/// The segment owns its successor: `next` holds one retain of it, given up when this segment ends, so a
/// segment is alive while the queue's head or tail names it or an earlier segment is.
final class LamportSegment<Element: Sendable>: @unchecked Sendable {
    /// How many slots, from the first, hold an element the producer published. Written only by the
    /// producer.
    let written = Atomic<Int>(0)

    /// The segment after this one (an owned retain), or `nil` while this is the newest. Stored once, by
    /// the producer.
    let next = Atomic<UnsafeMutableRawPointer?>(nil)

    let size: Int
    let slots: UnsafeMutablePointer<Element?>

    init(size: Int) {
        self.size = size
        let storage = UnsafeMutablePointer<Element?>.allocate(capacity: size)
        storage.initialize(repeating: nil, count: size)
        slots = storage
    }

    deinit {
        slots.deinitialize(count: size)
        slots.deallocate()
        endChain(after: next.exchange(nil, ordering: .acquiring)) { (segment: LamportSegment<Element>) in
            segment.next.exchange(nil, ordering: .acquiring)
        }
    }

    /// Links `successor` behind this segment: the one store of the link, after which the producer never
    /// touches this segment again.
    func link(_ successor: LamportSegment<Element>) {
        let pointer = Unmanaged.passRetained(successor).toOpaque()
        next.store(pointer, ordering: .releasing)
    }

    /// The successor, or `nil` while this is the newest.
    func successor() -> LamportSegment<Element>? {
        guard let pointer = next.load(ordering: .acquiring) else {
            return nil
        }
        return Unmanaged<LamportSegment<Element>>.fromOpaque(pointer).takeUnretainedValue()
    }
}

/// The `segmented` row for one producer and one consumer (SCE Protocol-Synthesis RFC section synth-5-P):
/// linked Lamport rings, wait-free on both sides, bounded only by the allocator it is given.
///
/// The queue is a list of segments of `segment` elements. The producer fills the newest segment and, when
/// it is full, takes a new one from the allocator and links it behind; the consumer reads the oldest
/// segment and, when it has read all of it, follows the link and hands the segment back to the allocator.
/// Each side keeps its own position and shares it with the other only through the two things the other
/// must see: how many slots of a segment are filled, and the link to the next segment.
///
/// Pop is wait-free. Push is wait-free but for the one step that can take a segment: the allocator. The
/// queue is therefore as strong as the allocator the document declares, which `create` holds to the
/// declared `QueueProgress`. A push the allocator refuses reports `.outOfMemory` and leaves the element with
/// the caller.
///
/// **Ordering.** The producer publishes a filled slot by storing the count of filled slots with release
/// and the consumer loads that count with acquire before it reads the slot, so the write of an element
/// happens-before the pop that returns it. The link is stored and read the same way: everything the
/// producer did to a segment before linking it happens-before the consumer's use of the next one.
///
/// **Reclamation.** A segment the consumer has left is released by the queue and ends when nothing else
/// holds it. The producer holds the newest segment, so it never reads one that has ended; the consumer
/// holds the oldest, and the oldest holds the link to the next. No participant reads a segment it does not
/// hold, so no reclamation scheme beyond the reference count is needed.
///
/// **Sendable.** The class is `@unchecked Sendable`, and that is sound: after the initializer the head, the
/// read count and the tail are touched only by the one handle of their side (the producer's `tail`, the
/// consumer's `head` and `headRead`), a place is taken with an acquiring exchange and given back with a
/// releasing store, so a successor handle sees what its predecessor left, and the segments are shared only
/// through atomics.
public final class LinkedLamport<Element: Sendable>: @unchecked Sendable {
    /// The elements one segment holds.
    public let segment: Int

    fileprivate let allocator: any SegmentAllocator

    /// The segment the consumer reads. Written only by the consumer.
    fileprivate var head: LamportSegment<Element>

    /// How many elements of the head segment the consumer has taken. Written only by the consumer.
    fileprivate var headRead: Int = 0

    /// The segment the producer fills. Written only by the producer.
    fileprivate var tail: LamportSegment<Element>

    fileprivate let producerClaimed = Atomic<Bool>(false)
    fileprivate let consumerClaimed = Atomic<Bool>(false)

    private init(segment: Int, allocator: any SegmentAllocator, first: LamportSegment<Element>) {
        self.segment = segment
        self.allocator = allocator
        self.head = first
        self.tail = first
    }

    /// An empty queue of segments of `segment` elements over `allocator`, or `nil` when the allocator will
    /// not give the first segment. A `segment` below one element, and an allocator whose `progress` is
    /// below `required`, the progress the document declared for it, stop the program: they are a defect of
    /// the caller and not an input, and a queue that would claim more than its allocator gives does not
    /// build.
    public static func create(
        segment: Int, allocator: any SegmentAllocator, required: QueueProgress
    ) -> LinkedLamport<Element>? {
        precondition(segment >= 1, "a segment holds at least one element")
        precondition(
            allocator.progress >= required,
            "the allocator gives \(allocator.progress), less than the \(required) the document declared for it")
        guard let first = allocator.allocate({ LamportSegment<Element>(size: segment) }) else {
            return nil
        }
        return LinkedLamport(segment: segment, allocator: allocator, first: first)
    }

    /// The producing side, or `nil` while another is held: a list shared by two producers is not this
    /// algorithm. Drop the handle, or call its `release()`, to give the place back.
    public func producer() -> LinkedLamportProducer<Element>? {
        if producerClaimed.exchange(true, ordering: .acquiring) {
            return nil
        }
        return LinkedLamportProducer(queue: self)
    }

    /// The consuming side, or `nil` while another is held. Drop the handle, or call its `release()`, to
    /// give the place back.
    public func consumer() -> LinkedLamportConsumer<Element>? {
        if consumerClaimed.exchange(true, ordering: .acquiring) {
            return nil
        }
        return LinkedLamportConsumer(queue: self)
    }

    fileprivate func push(_ value: Element) -> PushStatus {
        // The tail and the filled count are this side's: only the producer writes them.
        var tail = self.tail
        var written = tail.written.load(ordering: .relaxed)
        if written == segment {
            let size = segment
            guard let fresh = allocator.allocate({ LamportSegment<Element>(size: size) }) else {
                return .outOfMemory
            }
            // This is the only store of the link, and the producer never touches the segment again after
            // it.
            tail.link(fresh)
            self.tail = fresh
            tail = fresh
            written = 0
        }
        // Slot `written` is past every filled slot, so the consumer does not read it until the store
        // below publishes it.
        tail.slots[written] = value
        tail.written.store(written + 1, ordering: .releasing)
        return .ok
    }

    fileprivate func pop() -> Element? {
        var head = self.head
        var read = headRead
        if read == segment {
            // Every element of the head segment is taken. Its successor exists once the producer has
            // linked it, and the producer never touches this segment after that, so it is ours to give
            // back.
            guard let next = head.successor() else {
                return nil
            }
            allocator.deallocate(head)
            head = next
            read = 0
            self.head = head
            headRead = 0
        }
        if read >= head.written.load(ordering: .acquiring) {
            return nil
        }
        // Slot `read` is below the published count, so it holds an element, and only this side reads it.
        // It is cleared so the queue keeps no copy of an element that has left it.
        guard let value = head.slots[read] else {
            preconditionFailure("a slot below the published count holds a pushed element")
        }
        head.slots[read] = nil
        headRead = read + 1
        return value
    }
}

/// The producing side of a `LinkedLamport`. There is one at a time, and it is for one thread at a time.
public final class LinkedLamportProducer<Element: Sendable>: @unchecked Sendable {
    private var queue: LinkedLamport<Element>?

    fileprivate init(queue: LinkedLamport<Element>) {
        self.queue = queue
    }

    deinit {
        release()
    }

    /// Takes `value`, or reports `.outOfMemory` when the newest segment is full and the allocator will not
    /// give another; the element is the caller's either way. Wait-free when the allocator is.
    public func tryPush(_ value: Element) -> PushStatus {
        guard let queue else {
            preconditionFailure("the producer was released")
        }
        return queue.push(value)
    }

    /// The elements one segment of the queue this side fills holds.
    public var segment: Int {
        guard let queue else {
            preconditionFailure("the producer was released")
        }
        return queue.segment
    }

    /// Gives the producer place back. The handle is not usable afterwards.
    public func release() {
        queue?.producerClaimed.store(false, ordering: .releasing)
        queue = nil
    }
}

/// The consuming side of a `LinkedLamport`. There is one at a time, and it is for one thread at a time.
public final class LinkedLamportConsumer<Element: Sendable>: @unchecked Sendable {
    private var queue: LinkedLamport<Element>?

    fileprivate init(queue: LinkedLamport<Element>) {
        self.queue = queue
    }

    deinit {
        release()
    }

    /// Takes the oldest element, or returns `nil` when the queue is empty. Wait-free.
    public func tryPop() -> Element? {
        guard let queue else {
            preconditionFailure("the consumer was released")
        }
        return queue.pop()
    }

    /// The elements one segment of the queue this side drains holds.
    public var segment: Int {
        guard let queue else {
            preconditionFailure("the consumer was released")
        }
        return queue.segment
    }

    /// Gives the consumer place back. The handle is not usable afterwards.
    public func release() {
        queue?.consumerClaimed.store(false, ordering: .releasing)
        queue = nil
    }
}
