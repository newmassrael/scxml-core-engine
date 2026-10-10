// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package queue

// Progress is what an operation guarantees about how long it takes whatever
// the other participants are doing. Ordered by strength: a stronger guarantee
// compares greater, so "at least LockFree" is >= LockFree.
type Progress uint8

const (
	// Blocking may wait for another participant.
	Blocking Progress = iota
	// LockFree means some participant's operation completes in a bounded number
	// of steps.
	LockFree
	// WaitFree means every operation completes in a bounded number of steps.
	WaitFree
)

// String is the word a queue document spells the progress with.
func (p Progress) String() string {
	switch p {
	case Blocking:
		return "blocking"
	case LockFree:
		return "lock-free"
	case WaitFree:
		return "wait-free"
	}
	return "unknown"
}

// SegmentAllocator is the allocator a `segmented` queue is injected with (SCE
// Protocol-Synthesis RFC §synth-5-P, Storage modes): the one thing that bounds
// how much such a queue holds.
//
// A collected runtime has no blocks to hand out and take back: the segment is a
// Go value, and the collector frees it once nothing reaches it. What the
// allocator keeps is the decision. Allocate says whether the queue may have one
// more segment; Deallocate says that a segment has left the queue, or was built
// and never linked into it. A pool, an arena with a byte budget or a counter
// that stands in for a memory ceiling is an allocator here as it is in the
// languages that address memory. Because the queue is generic over its element
// and an interface method is not, the allocator is asked and the queue builds
// the segment; the allocator never holds one.
//
// It states the progress its own operations give, because the progress check of
// a queue document runs at build time and the allocator arrives only at run
// time (RFC §synth-5-P, Segment allocator progress is declared in the
// document). The queue refuses at construction an allocator that gives less than
// the document declared. Operations are called from several goroutines at once,
// so an implementation is safe for that.
type SegmentAllocator interface {
	// Progress is the progress Allocate and Deallocate give. The queue's push is
	// no stronger than this.
	Progress() Progress
	// Allocate reports whether one more segment may be had. A refusal is the
	// queue's PushOutOfMemory.
	Allocate() bool
	// Deallocate takes a segment back into account. The memory is the
	// collector's, and a goroutine still inside an operation on the segment
	// keeps it alive until that operation ends; so this is bookkeeping.
	Deallocate()
}

// requireProgress panics when the allocator gives less progress than the
// document declared for it: a queue that would claim more than its allocator
// gives does not build.
func requireProgress(allocator SegmentAllocator, required Progress) {
	if allocator.Progress() < required {
		panic("queue: the allocator gives " + allocator.Progress().String() +
			", less than the " + required.String() + " the document declared for it")
	}
}
