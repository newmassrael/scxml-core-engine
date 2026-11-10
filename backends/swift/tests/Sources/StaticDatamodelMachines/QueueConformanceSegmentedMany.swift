// SCE-GENERATED — DO NOT EDIT
// source-hash: 982595e044dab87d2a02655ddc0eb9f9f68babd78e0aa16f5305c5e6c8b4eb30
// SCE-MAP: queue_conformance_segmented_many.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="queue")
// Runtime: SceForgeRuntime
// Do not edit — regenerate from the source SCXML file.
//
// SCE Protocol-Synthesis RFC section synth-5-P queue: a first-in first-out queue that hands elements
// from one thread to another. The document states the contract (element type, many
// producer(s), many consumer(s), `lock-free`) and names no algorithm; the
// algorithm below is what the selection table gives that contract.
//
// Selected: LSCQ (a list of SCQ rings). Push is lock-free, pop is lock-free.
//
// The queue is segmented (`<sce:segmented segment="3" allocator-progress="lock-free"/>`):
// it grows by segments of 3 element(s) taken from a `SegmentAllocator` the caller builds it
// with, and holds as many elements as that allocator allows. Swift reference counts a segment, so what
// the allocator decides is only whether the queue may have one more segment, and when the queue gives
// one back: at once for a ring read by one consumer, and through a hazard domain for a list of rings
// read by several.
//
// Operations (from `Lscq`):
//   makeQueueConformanceSegmentedMany(allocator:)  the queue, or nil when the allocator refused the first segment; stops
//                            the program when the allocator gives less progress than the document
//                            declared for it
//   producer() / consumer()  a handle, or nil when every place of that side is taken; release() gives
//                            the place back, and so does the end of the handle
//   tryPush(v) -> PushStatus .ok takes the element; .outOfMemory means the allocator refused a
//                            segment, and the element stays with the caller
//   tryPop() -> Element?     nil when the queue is empty
//   segment
//
// The ring's 64-bit entries are `Synchronization.Atomic<Int>`. The runtime compiles only for the
// processors whose `Int` is 64 bits wide and whose 64-bit atomics are lock-free, and says why on the
// others (RFC section synth-5-P, Counter width).

import SceForgeRuntime

/// What the document declared, and what the algorithm the selection table gave it provides. A
/// namespace of its own, so that the units of one module do not meet in constants of the same name.
public enum QueueConformanceSegmentedManyContract {
    /// The elements one segment holds (`<sce:segmented segment>`).
    public static let segment: Int = 3

    /// What the document declared the allocator gives (`allocator-progress`). The queue is built only
    /// over an allocator whose progress is at least this: a queue that would claim more than its
    /// allocator gives does not build.
    public static let allocatorProgress: QueueProgress = .lockFree

    /// `allocatorProgress` as the document spells it.
    public static let allocatorProgressWord: String = "lock-free"

    /// The size of each of the two index rings of each segment: a power of two, at least `segment` and
    /// at least `participants`. The algorithm's empty test is justified only for as many producers or
    /// consumers as a ring has slots (Nikolaev 2019, section 5.1), so the ring is sized by both.
    public static let ringSlots: Int = 4

    /// The most threads that hold a handle on one side at the same time (`<sce:participants>`). The
    /// queue hands out no more handles than `ringSlots` on either side.
    public static let participants: Int = 2

    /// How many operations a participant may be delayed between reading a ring entry and acting on it
    /// before it could be misled: 2^62 for the 64-bit entries this ring keeps, which at 10^9
    /// operations a second is 146 years (RFC section synth-5-P, Counter width).
    public static let wrapBoundOps: Int = scqWrapBoundOps

    /// The slots of the hazard domain the queue builds for itself: one for every handle that can be
    /// alive, both sides together (RFC section synth-5-P, *Reclamation domain*).
    public static let hazardSlots: Int = 4

    /// What the document requires of push and pop (`<sce:progress>`).
    public static let declaredProgress: String = "lock-free"

    /// What the selected algorithm gives push.
    public static let pushProgress: String = "lock-free"

    /// What the selected algorithm gives pop.
    public static let popProgress: String = "lock-free"

    /// The algorithm the selection table gave this contract.
    public static let algorithm: String = "LSCQ (a list of SCQ rings)"
}

/// The queue. It grows by segments the allocator gives; an element that has left it is not referenced
/// by it, and a segment it has left goes back to the allocator.
public typealias QueueConformanceSegmentedMany = Lscq<QueueConformanceEvent>

/// An empty queue of segments of `QueueConformanceSegmentedManyContract.segment` elements over `allocator`, or nil when
/// the allocator will not give the first segment. Stops the program when the allocator's progress is
/// below `QueueConformanceSegmentedManyContract.allocatorProgress`.
public func makeQueueConformanceSegmentedMany(allocator: any SegmentAllocator) -> QueueConformanceSegmentedMany? {
    Lscq<QueueConformanceEvent>.create(
        segment: QueueConformanceSegmentedManyContract.segment,
        ringSlots: QueueConformanceSegmentedManyContract.ringSlots,
        allocator: allocator,
        domain: HazardDomain(slots: QueueConformanceSegmentedManyContract.hazardSlots),
        required: QueueConformanceSegmentedManyContract.allocatorProgress)
}
