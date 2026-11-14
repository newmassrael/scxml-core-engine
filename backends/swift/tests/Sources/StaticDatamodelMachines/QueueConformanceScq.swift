// SCE-GENERATED — DO NOT EDIT
// source-hash: 1ee4c478fcd6af0bb675edd6c8e90348072aad4cd4e5fb9446a5e52706545b90
// SCE-MAP: queue_conformance_scq.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="queue")
// Runtime: SceForgeRuntime
// Do not edit — regenerate from the source SCXML file.
//
// SCE Protocol-Synthesis RFC section synth-5-P queue: a first-in first-out queue that hands elements
// from one thread to another. The document states the contract (element type, many
// producer(s), many consumer(s), `lock-free`) and names no algorithm; the
// algorithm below is what the selection table gives that contract.
//
// Selected: SCQ data queue. Push is lock-free, pop is lock-free.
//
// Operations (from `Scq`):
//   producer() / consumer()  a handle, or nil when every place of that side is taken; release() gives
//                            the place back, and so does the end of the handle
//   tryPush(v) -> PushStatus .ok takes the element; .full does not
//   tryPop() -> Element?     nil when the queue is empty
//   capacity
//
// The ring's 64-bit entries are `Synchronization.Atomic<Int>`. The runtime compiles only for the
// processors whose `Int` is 64 bits wide and whose 64-bit atomics are lock-free, and says why on the
// others (RFC section synth-5-P, Counter width).

import SceForgeRuntime

/// What the document declared, and what the algorithm the selection table gave it provides. A
/// namespace of its own, so that the units of one module do not meet in constants of the same name.
public enum QueueConformanceScqContract {
    /// The most elements the queue holds (`<sce:bounded>`).
    public static let capacity: Int = 6

    /// The size of each of the queue's two index rings: a power of two, at least `capacity` and at
    /// least `participants`. The algorithm's empty test is justified only for as many producers or
    /// consumers as the ring has slots (Nikolaev 2019, section 5.1), so the ring is sized by both.
    public static let ringSlots: Int = 8

    /// The most threads that hold a handle on one side at the same time (`<sce:participants>`). The
    /// queue hands out no more handles than `ringSlots` on either side.
    public static let participants: Int = 3

    /// How many operations a participant may be delayed between reading a ring entry and acting on it
    /// before it could be misled: 2^62 for the 64-bit entries this ring keeps, which at 10^9
    /// operations a second is 146 years (RFC section synth-5-P, Counter width).
    public static let wrapBoundOps: Int = scqWrapBoundOps

    /// What the document requires of push and pop (`<sce:progress>`).
    public static let declaredProgress: String = "lock-free"

    /// What the selected algorithm gives push.
    public static let pushProgress: String = "lock-free"

    /// What the selected algorithm gives pop.
    public static let popProgress: String = "lock-free"

    /// The algorithm the selection table gave this contract.
    public static let algorithm: String = "SCQ data queue"
}

/// The queue. Its storage is one allocation, made when the queue is built and never again; an element
/// that has left it is not referenced by it.
public typealias QueueConformanceScq = Scq<QueueConformanceEvent>

/// An empty queue of `QueueConformanceScqContract.capacity` elements.
public func makeQueueConformanceScq() -> QueueConformanceScq {
    Scq<QueueConformanceEvent>(
        capacity: QueueConformanceScqContract.capacity, ringSlots: QueueConformanceScqContract.ringSlots)
}
