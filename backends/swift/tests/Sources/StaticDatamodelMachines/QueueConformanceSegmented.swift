// SCE-GENERATED — DO NOT EDIT
// source-hash: a657d6d390827c00a8c65cc79818ec59a776eff477a713478c6b36adae5a7d61
// SCE-MAP: queue_conformance_segmented.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="queue")
// Runtime: SceForgeRuntime
// Do not edit — regenerate from the source SCXML file.
//
// SCE Protocol-Synthesis RFC section synth-5-P queue: a first-in first-out queue that hands elements
// from one thread to another. The document states the contract (element type, one
// producer(s), one consumer(s), `lock-free`) and names no algorithm; the
// algorithm below is what the selection table gives that contract.
//
// Selected: linked Lamport rings. Push is lock-free, pop is wait-free.
//
// The queue is segmented (`<sce:segmented segment="3" allocator-progress="lock-free"/>`):
// it grows by segments of 3 element(s) taken from a `SegmentAllocator` the caller builds it
// with, and holds as many elements as that allocator allows. Swift reference counts a segment, so what
// the allocator decides is only whether the queue may have one more segment, and when the queue gives
// one back: at once for a ring read by one consumer, and through a hazard domain for a list of rings
// read by several.
//
// Operations (from `LinkedLamport`):
//   makeQueueConformanceSegmented(allocator:)  the queue, or nil when the allocator refused the first segment; stops
//                            the program when the allocator gives less progress than the document
//                            declared for it
//   producer() / consumer()  a handle, or nil when every place of that side is taken; release() gives
//                            the place back, and so does the end of the handle
//   tryPush(v) -> PushStatus .ok takes the element; .outOfMemory means the allocator refused a
//                            segment, and the element stays with the caller
//   tryPop() -> Element?     nil when the queue is empty
//   segment

import SceForgeRuntime

/// What the document declared, and what the algorithm the selection table gave it provides. A
/// namespace of its own, so that the units of one module do not meet in constants of the same name.
public enum QueueConformanceSegmentedContract {
    /// The elements one segment holds (`<sce:segmented segment>`).
    public static let segment: Int = 3

    /// What the document declared the allocator gives (`allocator-progress`). The queue is built only
    /// over an allocator whose progress is at least this: a queue that would claim more than its
    /// allocator gives does not build.
    public static let allocatorProgress: QueueProgress = .lockFree

    /// `allocatorProgress` as the document spells it.
    public static let allocatorProgressWord: String = "lock-free"

    /// What the document requires of push and pop (`<sce:progress>`).
    public static let declaredProgress: String = "lock-free"

    /// What the selected algorithm gives push.
    public static let pushProgress: String = "lock-free"

    /// What the selected algorithm gives pop.
    public static let popProgress: String = "wait-free"

    /// The algorithm the selection table gave this contract.
    public static let algorithm: String = "linked Lamport rings"
}

/// The queue. It grows by segments the allocator gives; an element that has left it is not referenced
/// by it, and a segment it has left goes back to the allocator.
public typealias QueueConformanceSegmented = LinkedLamport<QueueConformanceEvent>

/// An empty queue of segments of `QueueConformanceSegmentedContract.segment` elements over `allocator`, or nil when
/// the allocator will not give the first segment. Stops the program when the allocator's progress is
/// below `QueueConformanceSegmentedContract.allocatorProgress`.
public func makeQueueConformanceSegmented(allocator: any SegmentAllocator) -> QueueConformanceSegmented? {
    LinkedLamport<QueueConformanceEvent>.create(
        segment: QueueConformanceSegmentedContract.segment,
        allocator: allocator,
        required: QueueConformanceSegmentedContract.allocatorProgress)
}
