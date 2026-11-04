// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/// What an operation guarantees about how long it takes whatever the other participants are doing.
/// Ordered by strength: a stronger guarantee compares greater, so "at least lock-free" is
/// `>= .lockFree`.
///
/// Named for the queue, not `Progress`: a consumer that imports Foundation has a `Progress` of its own, and
/// two public types of one name make every use of it ambiguous.
public enum QueueProgress: Int, Comparable, Sendable {
    /// May wait for another participant.
    case blocking = 0

    /// Some participant's operation completes in a bounded number of steps.
    case lockFree = 1

    /// Every operation completes in a bounded number of steps.
    case waitFree = 2

    public static func < (lhs: QueueProgress, rhs: QueueProgress) -> Bool {
        lhs.rawValue < rhs.rawValue
    }
}

/// The allocator a `segmented` queue is injected with (SCE Protocol-Synthesis RFC section synth-5-P,
/// *Storage modes*): the one thing that bounds how much such a queue holds.
///
/// A segment is an object, and the queue owns it by reference counting: it is freed once the queue and
/// every participant working on it have let go. What the allocator keeps is the decision. `allocate` says
/// whether the queue may have one more segment, and builds it only if so; `deallocate` says that a segment
/// has left the queue. A pool, an arena with a byte budget or a counter that stands in for a memory ceiling
/// is an allocator here as it is in the languages that address memory.
///
/// It states the progress its own operations give as `progress`, because the progress check of a queue
/// document runs at build time and the allocator arrives only at run time (RFC section synth-5-P, *Segment
/// allocator progress is declared in the document*). The queue refuses at construction an allocator that
/// gives less than the document declared. Operations are called from several threads at once, so an
/// implementation is thread-safe.
public protocol SegmentAllocator: Sendable {
    /// The progress `allocate` and `deallocate` give. The queue's push is no stronger than this.
    var progress: QueueProgress { get }

    /// The segment `make` builds, or `nil` when no segment can be given. A refusal is the queue's
    /// `PushStatus.outOfMemory`. `make` runs only once the segment is granted, and at most once, so a
    /// refusal builds nothing.
    func allocate<Segment: AnyObject>(_ make: () -> Segment) -> Segment?

    /// Takes a segment back into account: it has left the queue, or it was built and never linked into
    /// it. The memory is the reference count's, not the allocator's, and a participant still inside an
    /// operation on the segment keeps it alive until that operation ends; so this is bookkeeping, and an
    /// allocator must not hand the segment out again.
    func deallocate(_ segment: AnyObject)
}
