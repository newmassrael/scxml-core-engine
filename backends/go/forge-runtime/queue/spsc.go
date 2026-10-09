// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package queue

import "sync/atomic"

// Spsc is a bounded queue of exactly capacity elements for one producer and
// one consumer: the `bounded` row for that cardinality (RFC §synth-5-P), a
// Lamport ring, wait-free on both sides.
//
// Lap indices, not counters. Each side's index runs over 0..2N and the slot it
// names is the index modulo N. Two laps are what tell a full ring from an
// empty one without spending a slot: the indices are equal exactly when the
// ring is empty and N apart exactly when it is full, so the ring holds
// exactly the capacity declared. An index never exceeds 2N, so nothing wraps
// however long the queue runs.
//
// Ordering. The producer publishes a filled slot by storing its index and the
// consumer reads that index before it reads the slot, so the write of an
// element happens-before its pop; the consumer returns a slot the same way in
// the other direction. Each side keeps the other's index as last read, and
// reads the shared one again only when the cached one says the ring is full
// (producer) or empty (consumer).
type Spsc[T any] struct {
	// head is the lap index of the next slot the consumer pops. Written only
	// by the consumer.
	head paddedUint64
	// tail is the lap index of the next slot the producer fills. Written only
	// by the producer.
	tail paddedUint64

	producerClaimed atomic.Bool
	consumerClaimed atomic.Bool

	capacity uint64
	// laps is the size of the lap-index space, twice the capacity.
	laps  uint64
	slots []T
}

// NewSpsc returns an empty queue of exactly capacity elements. It panics when
// capacity is below one. The lap indices run over twice the capacity in a
// uint64, which an int capacity can never overflow, so no other size is
// refused.
func NewSpsc[T any](capacity int) *Spsc[T] {
	if capacity < 1 {
		panic("queue: a queue's capacity must be at least one element")
	}
	return &Spsc[T]{
		capacity: uint64(capacity),
		laps:     2 * uint64(capacity),
		slots:    make([]T, capacity),
	}
}

// Capacity is the number of elements the queue holds, exactly.
func (q *Spsc[T]) Capacity() int { return int(q.capacity) }

// next is the lap index after index.
func (q *Spsc[T]) next(index uint64) uint64 {
	if index+1 == q.laps {
		return 0
	}
	return index + 1
}

// slot is the position in the slice that a lap index names.
func (q *Spsc[T]) slot(index uint64) uint64 {
	if index >= q.capacity {
		return index - q.capacity
	}
	return index
}

// occupancy is how many elements lie between a consumer index and a producer
// index.
func (q *Spsc[T]) occupancy(head, tail uint64) uint64 {
	if tail >= head {
		return tail - head
	}
	return tail + q.laps - head
}

// Producer returns the producing side, or false while another is held: a ring
// shared by two producers is not this algorithm. Release the handle to give
// the place back.
func (q *Spsc[T]) Producer() (*SpscProducer[T], bool) {
	if q.producerClaimed.Swap(true) {
		return nil, false
	}
	return &SpscProducer[T]{
		queue:      q,
		tail:       q.tail.v.Load(),
		cachedHead: q.head.v.Load(),
	}, true
}

// Consumer returns the consuming side, or false while another is held.
// Release the handle to give the place back.
func (q *Spsc[T]) Consumer() (*SpscConsumer[T], bool) {
	if q.consumerClaimed.Swap(true) {
		return nil, false
	}
	return &SpscConsumer[T]{
		queue:      q,
		head:       q.head.v.Load(),
		cachedTail: q.tail.v.Load(),
	}, true
}

// SpscProducer is the producing side of an Spsc. There is one at a time, and
// it is for one goroutine at a time.
type SpscProducer[T any] struct {
	queue *Spsc[T]
	// tail is this side's own index; the shared one only publishes it.
	tail uint64
	// cachedHead is the consumer's index as this side last read it.
	cachedHead uint64
}

// TryPush takes value, or reports PushFull when the queue holds its capacity.
// Wait-free: a bounded number of steps whatever the consumer is doing.
func (p *SpscProducer[T]) TryPush(value T) PushStatus {
	q := p.queue
	if q.occupancy(p.cachedHead, p.tail) == q.capacity {
		// Read the consumer's index again: its read of the slot about to be
		// reused happens-before the write below.
		p.cachedHead = q.head.v.Load()
		if q.occupancy(p.cachedHead, p.tail) == q.capacity {
			return PushFull
		}
	}
	// The slot at tail is outside [head, tail), so the consumer does not read
	// it until the store below publishes it.
	q.slots[q.slot(p.tail)] = value
	p.tail = q.next(p.tail)
	// The write of the slot happens-before the pop of any consumer that reads
	// this index.
	q.tail.v.Store(p.tail)
	return PushOK
}

// Capacity is the capacity of the queue this side fills.
func (p *SpscProducer[T]) Capacity() int { return p.queue.Capacity() }

// Release gives the producer place back. The handle is not usable afterwards.
func (p *SpscProducer[T]) Release() {
	if p.queue != nil {
		p.queue.producerClaimed.Store(false)
		p.queue = nil
	}
}

// SpscConsumer is the consuming side of an Spsc. There is one at a time, and
// it is for one goroutine at a time.
type SpscConsumer[T any] struct {
	queue *Spsc[T]
	head  uint64
	// cachedTail is the producer's index as this side last read it.
	cachedTail uint64
}

// TryPop takes the oldest element, or reports false when the queue is empty.
// Wait-free: a bounded number of steps whatever the producer is doing.
func (c *SpscConsumer[T]) TryPop() (T, bool) {
	q := c.queue
	if c.head == c.cachedTail {
		// Read the producer's index again: its write of the slot happens-before
		// the read below.
		c.cachedTail = q.tail.v.Load()
		if c.head == c.cachedTail {
			var none T
			return none, false
		}
	}
	// The slot at head is inside [head, tail), so it holds a pushed element,
	// and the producer does not reuse it until the store below returns it. The
	// slot is cleared so the queue keeps no reference to an element that has
	// left it.
	index := q.slot(c.head)
	value := q.slots[index]
	var zero T
	q.slots[index] = zero
	c.head = q.next(c.head)
	// The read of the slot happens-before the producer's reuse of it, by any
	// producer that reads this index.
	q.head.v.Store(c.head)
	return value, true
}

// Capacity is the capacity of the queue this side drains.
func (c *SpscConsumer[T]) Capacity() int { return c.queue.Capacity() }

// Release gives the consumer place back. The handle is not usable afterwards.
func (c *SpscConsumer[T]) Release() {
	if c.queue != nil {
		c.queue.consumerClaimed.Store(false)
		c.queue = nil
	}
}
