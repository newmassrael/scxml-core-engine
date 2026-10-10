// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The list of SCQ rings is built from the rings of scq.go, so it exists on the
// same architectures: the ones whose 64-bit atomics Go implements natively.

//go:build amd64 || arm64 || loong64 || mips64 || mips64le || ppc64 || ppc64le || riscv64 || s390x || wasm

package queue

import "sync/atomic"

// ringSegment is one segment of an Lscq: a ring and the link to the next.
type ringSegment[T any] struct {
	// next is the segment after this one, or nil while this is the newest.
	// Linked once, by a compare-and-swap.
	next atomic.Pointer[ringSegment[T]]
	ring *Scq[T]
}

// Lscq is the `segmented` row for any cardinality other than one producer and
// one consumer (SCE Protocol-Synthesis RFC §synth-5-P): LSCQ, a list of SCQ
// rings, lock-free on both sides and bounded only by the allocator it is given.
//
// The algorithm is Nikolaev's LSCQ (DISC 2019, section 6): a Michael-Scott list
// whose nodes are SCQ rings instead of single elements. Producers push into the
// newest ring (tail); consumers pop from the oldest (head). A ring that cannot
// take another element is closed: nothing is ever pushed into it again. The
// producer that found it closed asks the allocator for a segment, puts its
// element in it before anyone can see it, and links it behind the closed one with
// a compare-and-swap on that ring's next; a producer that loses that race hands
// its segment back and pushes into the winner's. A consumer that finds the
// oldest ring empty and closed moves head to its successor.
//
// Each ring is the SCQ data queue of Scq with the close bit its tail carries. A
// ring is closed by the first push that finds it without a free slot, and an
// enqueue that takes its ticket after the close is refused, which is what makes
// the order across segments exact. A ring is left only after popDrained, which
// walks the head to the closed tail and either takes the element an enqueue
// still holding a ticket filled or makes its entry unusable; without it a
// producer that passed a "not closed" check and stalled could push into a
// segment a consumer had already emptied and left, and the element would be
// lost.
//
// Reclamation is the collector's. A segment can be read by a goroutine after
// another has moved head past it, and the collector keeps it alive for that
// goroutine: no hazard-pointer domain is needed, which is the one thing a
// collected runtime does not build that the Rust, C++ and C11 ones do. A segment
// is given back to the allocator's account by the consumer whose compare-and-swap
// moved head past it, and only by that one.
//
// Push and pop are lock-free: a retry means another participant moved head, tail
// or a ring, and the helping step lets a push finish what a stalled one started.
// A push is as strong as the allocator it calls, which NewLscq holds to the
// declared Progress; a refusal reports PushOutOfMemory and leaves the element
// with the caller.
//
// Each segment's ring has RingSlots slots, a power of two at least Segment and
// at least the most producers or consumers that work the queue at once: its
// empty test is justified for at most that many enqueuers and that many
// dequeuers on one ring (Nikolaev 2019, section 5.1). So the queue hands out at
// most RingSlots producer handles and as many consumer handles at a time.
type Lscq[T any] struct {
	// head is the oldest segment.
	head atomic.Pointer[ringSegment[T]]
	// tail is the newest segment, or one behind it while a producer has linked
	// a successor and not yet moved this.
	tail atomic.Pointer[ringSegment[T]]

	producers atomic.Uint64
	consumers atomic.Uint64

	segment   int
	ringSlots int
	allocator SegmentAllocator
}

// newRingSegment builds a segment: a ring of segment elements over ringSlots
// slots.
func newRingSegment[T any](segment, ringSlots int) *ringSegment[T] {
	return &ringSegment[T]{ring: NewScq[T](segment, ringSlots)}
}

// NewLscq returns an empty queue of segments of segment elements over
// allocator, or false when the allocator will not give the first segment. It
// panics for a segment or ringSlots no ring can have (see NewScq), and when the
// allocator's Progress is below required, the progress the document declared for
// it.
func NewLscq[T any](segment, ringSlots int, allocator SegmentAllocator, required Progress) (*Lscq[T], bool) {
	checkShape(segment, ringSlots)
	requireProgress(allocator, required)
	if !allocator.Allocate() {
		return nil, false
	}
	first := newRingSegment[T](segment, ringSlots)
	q := &Lscq[T]{segment: segment, ringSlots: ringSlots, allocator: allocator}
	q.head.Store(first)
	q.tail.Store(first)
	return q, true
}

// Segment is the number of elements one segment holds.
func (q *Lscq[T]) Segment() int { return q.segment }

// RingSlots is the number of slots of each segment's index rings.
func (q *Lscq[T]) RingSlots() int { return q.ringSlots }

// Producer returns a producer handle, or false when every producer place is
// taken: a ring cannot be shared by more enqueuers than it has slots. Release
// the handle to give the place back.
func (q *Lscq[T]) Producer() (*LscqProducer[T], bool) {
	if !takePlace(&q.producers, uint64(q.ringSlots)) {
		return nil, false
	}
	return &LscqProducer[T]{queue: q}, true
}

// Consumer returns a consumer handle, or false when every consumer place is
// taken. Release the handle to give the place back.
func (q *Lscq[T]) Consumer() (*LscqConsumer[T], bool) {
	if !takePlace(&q.consumers, uint64(q.ringSlots)) {
		return nil, false
	}
	return &LscqConsumer[T]{queue: q}, true
}

// LscqProducer is a producing side of an Lscq. It holds one of the queue's
// producer places until Release.
type LscqProducer[T any] struct {
	queue *Lscq[T]
}

// TryClone returns another producer of the same queue, or false when every
// place is taken.
func (p *LscqProducer[T]) TryClone() (*LscqProducer[T], bool) { return p.queue.Producer() }

// TryPush takes value, or reports PushOutOfMemory when the newest segment is
// closed and the allocator will not give another; the element is the caller's
// either way. Lock-free when the allocator is.
func (p *LscqProducer[T]) TryPush(value T) PushStatus {
	q := p.queue
	for {
		tail := q.tail.Load()
		if next := tail.next.Load(); next != nil {
			// A producer linked a successor and has not moved tail: finish its
			// step.
			q.tail.CompareAndSwap(tail, next)
			continue
		}
		if tail.ring.pushOrClose(value) {
			return PushOK
		}
		// The segment is closed. Put the element in a segment of our own before
		// anyone can see it, then try to link it behind this one.
		if !q.allocator.Allocate() {
			return PushOutOfMemory
		}
		fresh := newRingSegment[T](q.segment, q.ringSlots)
		// A new ring is open and has a free slot, so it takes an element.
		if !fresh.ring.pushOrClose(value) {
			panic("queue: a new ring refused its first element")
		}
		if tail.next.CompareAndSwap(nil, fresh) {
			q.tail.CompareAndSwap(tail, fresh)
			return PushOK
		}
		// Another producer linked first. The segment nobody saw goes back to the
		// allocator's account, and the element, which this side still holds, is
		// pushed behind theirs.
		q.allocator.Deallocate()
	}
}

// Segment is the number of elements one segment of the queue this side fills
// holds.
func (p *LscqProducer[T]) Segment() int { return p.queue.segment }

// Release gives the producer place back. The handle is not usable afterwards.
func (p *LscqProducer[T]) Release() {
	if p.queue != nil {
		p.queue.producers.Add(^uint64(0))
		p.queue = nil
	}
}

// LscqConsumer is a consuming side of an Lscq. It holds one of the queue's
// consumer places until Release.
type LscqConsumer[T any] struct {
	queue *Lscq[T]
}

// TryClone returns another consumer of the same queue, or false when every
// place is taken.
func (c *LscqConsumer[T]) TryClone() (*LscqConsumer[T], bool) { return c.queue.Consumer() }

// TryPop takes the oldest element, or reports false when the queue is empty.
// Lock-free.
func (c *LscqConsumer[T]) TryPop() (T, bool) {
	q := c.queue
	for {
		head := q.head.Load()
		if value, ok := head.ring.pop(); ok {
			return value, true
		}
		next := head.next.Load()
		if next == nil {
			var none T
			return none, false
		}
		// The segment is closed. An enqueue that took a ticket before the close
		// may still be filling its entry, so the empty answer above does not yet
		// show the segment holds nothing and never will.
		if value, ok := head.ring.popDrained(); ok {
			return value, true
		}
		if q.head.CompareAndSwap(head, next) {
			// This consumer moved head past the segment, so it alone gives it
			// back.
			q.allocator.Deallocate()
		}
	}
}

// Segment is the number of elements one segment of the queue this side drains
// holds.
func (c *LscqConsumer[T]) Segment() int { return c.queue.segment }

// Release gives the consumer place back. The handle is not usable afterwards.
func (c *LscqConsumer[T]) Release() {
	if c.queue != nil {
		c.queue.consumers.Add(^uint64(0))
		c.queue = nil
	}
}
