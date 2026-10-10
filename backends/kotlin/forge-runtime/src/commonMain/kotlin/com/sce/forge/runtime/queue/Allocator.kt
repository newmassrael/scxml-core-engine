// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.forge.runtime.queue

/**
 * What an operation guarantees about how long it takes whatever the other
 * participants are doing. Ordered by strength: a stronger guarantee compares
 * greater, so "at least `LockFree`" is `>= Progress.LockFree`.
 */
public enum class Progress {
    /** May wait for another participant. */
    Blocking,

    /** Some participant's operation completes in a bounded number of steps. */
    LockFree,

    /** Every operation completes in a bounded number of steps. */
    WaitFree,
}

/**
 * The allocator a `segmented` queue is injected with (SCE Protocol-Synthesis RFC
 * §synth-5-P, *Storage modes*): the one thing that bounds how much such a queue
 * holds.
 *
 * A collected runtime has no blocks to hand out and take back: the segment is an
 * object, and the collector frees it once nothing reaches it. What the allocator
 * keeps is the decision. [allocate] says whether the queue may have one more
 * segment, and builds it only if so; [deallocate] says that a segment has left
 * the queue. A pool, an arena with a byte budget or a counter that stands in for
 * a memory ceiling is an allocator here as it is in the languages that address
 * memory.
 *
 * It states the progress its own operations give as [progress], because the
 * progress check of a queue document runs at build time and the allocator
 * arrives only at run time (RFC §synth-5-P, *Segment allocator progress is
 * declared in the document*). The queue refuses at construction an allocator
 * that gives less than the document declared. Operations are called from several
 * threads at once, so an implementation is thread-safe.
 */
public interface SegmentAllocator {
    /** The progress [allocate] and [deallocate] give. The queue's push is no stronger than this. */
    public val progress: Progress

    /**
     * The segment [make] builds, or `null` when no segment can be given. A
     * refusal is the queue's [PushStatus.OutOfMemory]. [make] runs only once the
     * segment is granted, and at most once, so a refusal builds nothing.
     */
    public fun <S : Any> allocate(make: () -> S): S?

    /**
     * Takes a segment back into account: it has left the queue, or it was built
     * and never linked into it. The memory is the collector's, not the
     * allocator's, and a participant still inside an operation on the segment
     * keeps it alive until that operation ends; so this is bookkeeping, and an
     * allocator must not hand the segment out again.
     */
    public fun deallocate(segment: Any)
}
