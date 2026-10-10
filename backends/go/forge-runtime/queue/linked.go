// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package queue

import "sync/atomic"

// lamportSegment is one segment: slots filled once, in order. A Lamport ring
// that is never lapped, so it needs neither the second lap that tells a full
// ring from an empty one nor a free slot.
type lamportSegment[T any] struct {
	// written is how many slots, from the first, hold an element the producer
	// published. Written only by the producer.
	written atomic.Uint64
	// next is the segment after this one, or nil while this is the newest.
	// Stored once, by the producer.
	next  atomic.Pointer[lamportSegment[T]]
	slots []T
}

// LinkedLamport is the `segmented` row for one producer and one consumer (SCE
// Protocol-Synthesis RFC §synth-5-P): linked Lamport rings, wait-free on both
// sides, bounded only by the allocator it is given.
//
// The queue is a list of segments of Segment elements. The producer fills the
// newest segment and, when it is full, takes a new one from the allocator and
// links it behind; the consumer reads the oldest segment and, when it has read
// all of it, follows the link and hands the segment back to the allocator. Each
// side keeps its own position and shares it with the other only through the two
// things the other must see: how many slots of a segment are filled, and the
// link to the next segment.
//
// Pop is wait-free. Push is wait-free but for the one step that can take a
// segment: the allocator. The queue is therefore as strong as the allocator the
// document declares, which NewLinkedLamport holds to the declared Progress. A
// push the allocator refuses reports PushOutOfMemory and leaves the element with
// the caller.
//
// Ordering. The producer publishes a filled slot by storing the count of filled
// slots and the consumer reads that count before it reads the slot, so the write
// of an element happens-before the pop that returns it. The link is stored and
// read the same way: everything the producer did to a segment before linking it
// happens-before the consumer's use of the next one. A segment the consumer has
// left is the collector's; no reclamation scheme is needed.
type LinkedLamport[T any] struct {
	// head is the segment the consumer reads. Written only by the consumer.
	head atomic.Pointer[lamportSegment[T]]
	// headRead is how many elements of the head segment the consumer has taken.
	// Written only by the consumer.
	headRead atomic.Uint64
	// tail is the segment the producer fills. Written only by the producer.
	tail atomic.Pointer[lamportSegment[T]]

	producerTaken atomic.Bool
	consumerTaken atomic.Bool

	segment   int
	allocator SegmentAllocator
}

// NewLinkedLamport returns an empty queue of segments of segment elements over
// allocator, or false when the allocator will not give the first segment. It
// panics when segment is below one, and when the allocator's Progress is below
// required, the progress the document declared for it.
func NewLinkedLamport[T any](segment int, allocator SegmentAllocator, required Progress) (*LinkedLamport[T], bool) {
	if segment < 1 {
		panic("queue: a segment holds at least one element")
	}
	requireProgress(allocator, required)
	if !allocator.Allocate() {
		return nil, false
	}
	first := &lamportSegment[T]{slots: make([]T, segment)}
	q := &LinkedLamport[T]{segment: segment, allocator: allocator}
	q.head.Store(first)
	q.tail.Store(first)
	return q, true
}

// Segment is the number of elements one segment holds.
func (q *LinkedLamport[T]) Segment() int { return q.segment }

// Producer returns the producing side, or false while another is held: a list
// shared by two producers is not this algorithm. Release the handle to give the
// place back.
func (q *LinkedLamport[T]) Producer() (*LinkedProducer[T], bool) {
	if !q.producerTaken.CompareAndSwap(false, true) {
		return nil, false
	}
	return &LinkedProducer[T]{queue: q}, true
}

// Consumer returns the consuming side, or false while another is held. Release
// the handle to give the place back.
func (q *LinkedLamport[T]) Consumer() (*LinkedConsumer[T], bool) {
	if !q.consumerTaken.CompareAndSwap(false, true) {
		return nil, false
	}
	return &LinkedConsumer[T]{queue: q}, true
}

// LinkedProducer is the producing side of a LinkedLamport. There is one at a
// time, and it is for one goroutine at a time.
type LinkedProducer[T any] struct {
	queue *LinkedLamport[T]
}

// TryPush takes value, or reports PushOutOfMemory when the newest segment is
// full and the allocator will not give another; the element is the caller's
// either way. Wait-free when the allocator is.
func (p *LinkedProducer[T]) TryPush(value T) PushStatus {
	q := p.queue
	// The tail and the filled count are this side's: only the producer writes
	// them.
	tail := q.tail.Load()
	written := int(tail.written.Load())
	if written == q.segment {
		if !q.allocator.Allocate() {
			return PushOutOfMemory
		}
		fresh := &lamportSegment[T]{slots: make([]T, q.segment)}
		// This is the only store of the link, and the producer never touches
		// the segment again after it.
		tail.next.Store(fresh)
		q.tail.Store(fresh)
		tail = fresh
		written = 0
	}
	// Slot written is past every filled slot, so the consumer does not read it
	// until the store below publishes it.
	tail.slots[written] = value
	tail.written.Store(uint64(written + 1))
	return PushOK
}

// Segment is the number of elements one segment of the queue this side fills
// holds.
func (p *LinkedProducer[T]) Segment() int { return p.queue.segment }

// Release gives the producer place back. The handle is not usable afterwards.
func (p *LinkedProducer[T]) Release() {
	if p.queue != nil {
		p.queue.producerTaken.Store(false)
		p.queue = nil
	}
}

// LinkedConsumer is the consuming side of a LinkedLamport. There is one at a
// time, and it is for one goroutine at a time.
type LinkedConsumer[T any] struct {
	queue *LinkedLamport[T]
}

// TryPop takes the oldest element, or reports false when the queue is empty.
// Wait-free.
func (c *LinkedConsumer[T]) TryPop() (T, bool) {
	q := c.queue
	var none T
	head := q.head.Load()
	read := int(q.headRead.Load())
	if read == q.segment {
		// Every element of the head segment is taken. Its successor exists once
		// the producer has linked it, and the producer never touches this
		// segment after that, so it is ours to give back.
		next := head.next.Load()
		if next == nil {
			return none, false
		}
		q.allocator.Deallocate()
		head = next
		read = 0
		q.head.Store(head)
		q.headRead.Store(0)
	}
	if uint64(read) >= head.written.Load() {
		return none, false
	}
	// Slot read is below the published count, so it holds an element, and only
	// this side reads it. It is cleared so the queue keeps no reference to an
	// element that has left it.
	value := head.slots[read]
	head.slots[read] = none
	q.headRead.Store(uint64(read + 1))
	return value, true
}

// Segment is the number of elements one segment of the queue this side drains
// holds.
func (c *LinkedConsumer[T]) Segment() int { return c.queue.segment }

// Release gives the consumer place back. The handle is not usable afterwards.
func (c *LinkedConsumer[T]) Release() {
	if c.queue != nil {
		c.queue.consumerTaken.Store(false)
		c.queue = nil
	}
}
