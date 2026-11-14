// SCE-GENERATED — DO NOT EDIT
// source-hash: 1ee4c478fcd6af0bb675edd6c8e90348072aad4cd4e5fb9446a5e52706545b90
// SCE-MAP: queue_conformance_spsc.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="queue")
// Runtime: SceForgeRuntime
// Do not edit — regenerate from the source SCXML file.
//
// SCE Protocol-Synthesis RFC section synth-5-P queue: a first-in first-out queue that hands elements
// from one thread to another. The document states the contract (element type, one
// producer(s), one consumer(s), `wait-free`) and names no algorithm; the
// algorithm below is what the selection table gives that contract.
//
// Selected: Lamport ring. Push is wait-free, pop is wait-free.
//
// Operations (from `Spsc`):
//   producer() / consumer()  a handle, or nil when every place of that side is taken; release() gives
//                            the place back, and so does the end of the handle
//   tryPush(v) -> PushStatus .ok takes the element; .full does not
//   tryPop() -> Element?     nil when the queue is empty
//   capacity

import SceForgeRuntime

/// What the document declared, and what the algorithm the selection table gave it provides. A
/// namespace of its own, so that the units of one module do not meet in constants of the same name.
public enum QueueConformanceSpscContract {
    /// The most elements the queue holds (`<sce:bounded>`).
    public static let capacity: Int = 4

    /// What the document requires of push and pop (`<sce:progress>`).
    public static let declaredProgress: String = "wait-free"

    /// What the selected algorithm gives push.
    public static let pushProgress: String = "wait-free"

    /// What the selected algorithm gives pop.
    public static let popProgress: String = "wait-free"

    /// The algorithm the selection table gave this contract.
    public static let algorithm: String = "Lamport ring"
}

/// The queue. Its storage is one allocation, made when the queue is built and never again; an element
/// that has left it is not referenced by it.
public typealias QueueConformanceSpsc = Spsc<QueueConformanceEvent>

/// An empty queue of `QueueConformanceSpscContract.capacity` elements.
public func makeQueueConformanceSpsc() -> QueueConformanceSpsc {
    Spsc<QueueConformanceEvent>(capacity: QueueConformanceSpscContract.capacity)
}
