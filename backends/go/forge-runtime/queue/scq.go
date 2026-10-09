// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The architectures listed here are the ones whose 64-bit atomics Go
// implements natively. On a 32-bit architecture it may implement them with a
// spinlock (`mips`, `mipsle`), which a ring built on lock-free entries cannot
// be built on: the SCQ row is absent there, and the generator refuses it
// (RFC §synth-5-P, Counter width).

//go:build amd64 || arm64 || loong64 || mips64 || mips64le || ppc64 || ppc64le || riscv64 || s390x || wasm

package queue

import "sync/atomic"

// WrapBoundOps is how many operations a participant may be delayed between
// reading an entry and acting on it before it could be misled: 2^(w-2) for a
// word of w bits, independent of the capacity. An entry's cycle is compared
// with a ticket's modulo 2^c, where c = w - 1 - log2(2R), so a delay is safe
// until the entry's cycle has moved by half that range, 2^(c-1); each cycle
// takes 2R operations; the product is 2^(w-2). At 10^9 operations a second
// 2^62 is 146 years (RFC §synth-5-P, Counter width).
const WrapBoundOps uint64 = 1 << 62

// maxRingSlots is the largest ring that leaves an entry enough bits for its
// cycle.
const maxRingSlots = 1 << 30

// before reports whether x is before y in the order that wraps: the signed
// difference. Tickets and cycles are compared this way throughout.
func before(x, y uint64) bool { return int64(x-y) < 0 }

// atOrBefore reports whether x is y or before it, in the same order.
func atOrBefore(x, y uint64) bool { return int64(x-y) <= 0 }

// ring is one SCQ ring of R slots (R a power of two) and 2R entries.
//
// An entry is cycle | IsSafe | index: the index in the low log2(2R) bits, the
// IsSafe bit above it, the cycle in the rest. An entry whose index bits are
// all ones holds nothing.
type ring struct {
	// head is the next ticket a dequeue takes.
	head paddedUint64
	// threshold is how many failed dequeues remain before the ring may be
	// called empty; negative means it is empty.
	threshold paddedInt64
	// tail is the next ticket an enqueue takes.
	tail    paddedUint64
	entries []atomic.Uint64

	// entryCount is the number of entries, 2R; also the value of the IsSafe
	// bit.
	entryCount uint64
	// indexMask covers the bits an index occupies.
	indexMask uint64
	// lowMask covers the index bits and the IsSafe bit: everything below the
	// cycle.
	lowMask uint64
	// entryOrder is log2(2R).
	entryOrder uint
	// thresholdAfterEnqueue is 3R - 1: that many failed dequeues are enough to
	// show that an empty-looking ring is empty.
	thresholdAfterEnqueue int64
}

// lineShift is how many entries one 64-byte cache line holds, as a power of
// two. Consecutive tickets are spread over lines by rotating the ticket's bits
// by this much; a ring smaller than a line does not rotate.
const lineShift = 3

// spins is how often a dequeue that found an entry still empty looks again
// before it makes the entry unusable for the enqueue that owns it. Bounded, so
// the operation stays lock-free.
const spins = 10000

// newRing returns a ring of slots slots holding the indices 0..filled, oldest
// first.
func newRing(slots, filled uint64) *ring {
	entryCount := 2 * slots
	var order uint
	for 1<<order < entryCount {
		order++
	}
	r := &ring{
		entries:               make([]atomic.Uint64, entryCount),
		entryCount:            entryCount,
		indexMask:             entryCount - 1,
		lowMask:               2*entryCount - 1,
		entryOrder:            order,
		thresholdAfterEnqueue: 3*int64(slots) - 1,
	}
	for position := range r.entries {
		r.entries[position].Store(r.initial(uint64(position), filled))
	}
	if filled == 0 {
		r.threshold.v.Store(-1)
	} else {
		r.threshold.v.Store(r.thresholdAfterEnqueue)
	}
	r.tail.v.Store(filled)
	return r
}

// mapTicket is the entry a ticket names: the ticket rotated so that
// consecutive tickets land on different cache lines.
func (r *ring) mapTicket(ticket uint64) uint64 {
	if r.entryOrder >= lineShift {
		return ((ticket & r.indexMask) >> (r.entryOrder - lineShift)) | ((ticket << lineShift) & r.indexMask)
	}
	return ticket & r.indexMask
}

// unmap is the ticket mapTicket sends to this entry.
func (r *ring) unmap(position uint64) uint64 {
	if r.entryOrder >= lineShift {
		return ((position & r.indexMask) >> lineShift) | ((position << (r.entryOrder - lineShift)) & r.indexMask)
	}
	return position & r.indexMask
}

// initial is what the entry at position holds in a ring built with filled
// indices: the first filled tickets' entries hold index ticket in cycle 0,
// safe; the rest hold nothing, in the cycle before it.
func (r *ring) initial(position, filled uint64) uint64 {
	ticket := r.unmap(position)
	if ticket < filled {
		return r.entryCount + ticket
	}
	return ^uint64(0)
}

// enqueue puts index on the ring. The ring never refuses: at most R indices
// circulate through it, and an enqueue that finds its entry unusable takes the
// next ticket.
func (r *ring) enqueue(index uint64) {
	stored := index ^ r.indexMask
	for {
		tail := r.tail.v.Add(1) - 1
		ticketCycle := (tail << 1) | r.lowMask
		slot := &r.entries[r.mapTicket(tail)]
		entry := slot.Load()
		// The entry is ours to fill when it is empty, from an earlier cycle,
		// and either safe or not yet passed by the head.
		for {
			entryCycle := entry | r.lowMask
			usable := before(entryCycle, ticketCycle) &&
				(entry == entryCycle ||
					(entry == (entryCycle^r.entryCount) && atOrBefore(r.head.v.Load(), tail)))
			if !usable {
				break
			}
			if slot.CompareAndSwap(entry, ticketCycle^stored) {
				if r.threshold.v.Load() != r.thresholdAfterEnqueue {
					r.threshold.v.Store(r.thresholdAfterEnqueue)
				}
				return
			}
			// The entry changed; judge what it is now.
			entry = slot.Load()
		}
	}
}

// fetchOr sets the bits of mask in the entry. A compare-and-swap loop: the
// Or method of sync/atomic needs a newer Go than the toolchain this runtime
// is held to builds with.
func fetchOr(slot *atomic.Uint64, mask uint64) {
	for {
		old := slot.Load()
		if slot.CompareAndSwap(old, old|mask) {
			return
		}
	}
}

// catchUp moves tail back to head after dequeues overshot an empty ring, so
// that the next enqueue does not start a cycle ahead of what is read.
func (r *ring) catchUp(tail, head uint64) {
	for !r.tail.v.CompareAndSwap(tail, head) {
		head = r.head.v.Load()
		tail = r.tail.v.Load()
		if !before(tail, head) {
			break
		}
	}
}

// look reads the entry a dequeue's ticket names until it is settled: the
// index when the entry was this ticket's (the entry is left empty in the same
// cycle), false when the ticket found nothing and the caller goes on to the
// empty check.
func (r *ring) look(slot *atomic.Uint64, ticketCycle uint64) (uint64, bool) {
	looked := 0
	for {
		entry := slot.Load()
		for {
			entryCycle := entry | r.lowMask
			if entryCycle == ticketCycle {
				// The entry is this ticket's: take its index and leave the
				// entry empty in the same cycle.
				fetchOr(slot, r.indexMask)
				return entry & r.indexMask, true
			}
			var replacement uint64
			if (entry | r.entryCount) != entryCycle {
				// It holds an index of another cycle: mark it unsafe, so no
				// enqueue of this cycle's ticket fills the entry behind the
				// head. Already unsafe: nothing to do.
				replacement = entry &^ r.entryCount
				if entry == replacement {
					return 0, false
				}
			} else {
				// It is empty. An enqueue holding this cycle's ticket may be
				// about to fill it; look a few times, then move the entry to
				// this cycle so that enqueue fails.
				looked++
				if looked <= spins {
					break // look again from a fresh read
				}
				replacement = ticketCycle ^ ((^entry) & r.entryCount)
			}
			if !before(entryCycle, ticketCycle) {
				return 0, false
			}
			if slot.CompareAndSwap(entry, replacement) {
				return 0, false
			}
			entry = slot.Load()
		}
	}
}

// dequeue takes the oldest index off the ring, or reports false when it is
// empty.
func (r *ring) dequeue() (uint64, bool) {
	if r.threshold.v.Load() < 0 {
		return 0, false
	}
	for {
		head := r.head.v.Add(1) - 1
		ticketCycle := (head << 1) | r.lowMask
		if index, ok := r.look(&r.entries[r.mapTicket(head)], ticketCycle); ok {
			return index, true
		}

		tail := r.tail.v.Load()
		if atOrBefore(tail, head+1) {
			r.catchUp(tail, head+1)
			r.threshold.v.Add(-1)
			return 0, false
		}
		// Add returns the new value; the value before was positive exactly
		// when the new one is not below zero.
		if r.threshold.v.Add(-1) < 0 {
			return 0, false
		}
	}
}

// Scq is a bounded queue of at most capacity elements for any number of
// producers and any number of consumers, lock-free. Exactly capacity fit when
// nothing else is running.
//
// The elements live in a slice of capacity slots. Two rings of indices make it
// a data queue: the free ring holds the indices of the slots nobody is using
// and starts with all of them; the allocated ring holds the indices of the
// slots that hold an element, in the order they were filled, and starts empty.
// A push takes an index from the free ring (none: the queue is full), writes
// its element into that slot and puts the index on the allocated ring; a pop
// does the reverse. The rings carry only indices.
//
// The ring has ringSlots slots, a power of two at least capacity, and at least
// the most producers or consumers that will work the queue at once. The
// algorithm's empty test is justified for at most ringSlots enqueuers and at
// most ringSlots dequeuers working one ring at once (Nikolaev 2019, section
// 5.1); beyond that a completed push can leave the threshold at -1 with its
// element in the ring and every pop reports the queue empty until another push
// completes. So a queue hands out at most ringSlots producer handles and at
// most ringSlots consumer handles at a time.
//
// A push may be refused while others hold slots: a slot's index goes back to
// the free ring only after the pop that took the element has read it, and a
// push holds the index it took until it has published its element. Waiting
// for the operation that holds the slot would not be lock-free, since that
// participant may be stopped. The shortfall is at most one slot per other
// participant.
type Scq[T any] struct {
	// allocated holds the indices of the slots that hold an element, oldest
	// first.
	allocated *ring
	// free holds the indices of the slots nobody is using.
	free *ring

	producers atomic.Uint64
	consumers atomic.Uint64

	capacity  int
	ringSlots uint64
	slots     []T
}

// NewScq returns an empty queue of exactly capacity elements over rings of
// ringSlots slots. It panics when capacity is below one, when ringSlots is not
// a power of two, is below capacity, or is more than a ring can number.
func NewScq[T any](capacity, ringSlots int) *Scq[T] {
	if capacity < 1 {
		panic("queue: a queue's capacity must be at least one element")
	}
	if ringSlots < 1 || ringSlots&(ringSlots-1) != 0 {
		panic("queue: the ring size must be a power of two")
	}
	if ringSlots < capacity {
		panic("queue: the ring must have at least as many slots as the capacity")
	}
	if ringSlots > maxRingSlots {
		panic("queue: a ring this large leaves its entries too few bits for the cycle")
	}
	return &Scq[T]{
		allocated: newRing(uint64(ringSlots), 0),
		free:      newRing(uint64(ringSlots), uint64(capacity)),
		capacity:  capacity,
		ringSlots: uint64(ringSlots),
		slots:     make([]T, capacity),
	}
}

// Capacity is the number of elements the queue holds, exactly.
func (q *Scq[T]) Capacity() int { return q.capacity }

// Producer returns a producer handle, or false when every producer place is
// taken: the ring cannot be shared by more enqueuers than it has slots.
// Release the handle to give the place back.
func (q *Scq[T]) Producer() (*ScqProducer[T], bool) {
	if !takePlace(&q.producers, q.ringSlots) {
		return nil, false
	}
	return &ScqProducer[T]{queue: q}, true
}

// Consumer returns a consumer handle, or false when every consumer place is
// taken. Release the handle to give the place back.
func (q *Scq[T]) Consumer() (*ScqConsumer[T], bool) {
	if !takePlace(&q.consumers, q.ringSlots) {
		return nil, false
	}
	return &ScqConsumer[T]{queue: q}, true
}

// ScqProducer is a producing side of an Scq. It holds one of the queue's
// producer places until Release.
type ScqProducer[T any] struct {
	queue *Scq[T]
}

// TryClone returns another producer of the same queue, or false when every
// place is taken.
func (p *ScqProducer[T]) TryClone() (*ScqProducer[T], bool) { return p.queue.Producer() }

// TryPush takes value, or reports PushFull when the queue holds its capacity,
// or while other participants' operations hold the slots it is short of.
// Lock-free: some operation completes in a bounded number of steps whatever
// the other participants are doing.
func (p *ScqProducer[T]) TryPush(value T) PushStatus {
	q := p.queue
	index, ok := q.free.dequeue()
	if !ok {
		return PushFull
	}
	// The index came off the free ring, so no other participant holds this
	// slot until the enqueue below hands it on.
	q.slots[index] = value
	q.allocated.enqueue(index)
	return PushOK
}

// Capacity is the capacity of the queue this side fills.
func (p *ScqProducer[T]) Capacity() int { return p.queue.Capacity() }

// Release gives the producer place back. The handle is not usable afterwards.
func (p *ScqProducer[T]) Release() {
	if p.queue != nil {
		p.queue.producers.Add(^uint64(0))
		p.queue = nil
	}
}

// ScqConsumer is a consuming side of an Scq. It holds one of the queue's
// consumer places until Release.
type ScqConsumer[T any] struct {
	queue *Scq[T]
}

// TryClone returns another consumer of the same queue, or false when every
// place is taken.
func (c *ScqConsumer[T]) TryClone() (*ScqConsumer[T], bool) { return c.queue.Consumer() }

// TryPop takes the oldest element, or reports false when the queue is empty.
// Lock-free.
func (c *ScqConsumer[T]) TryPop() (T, bool) {
	q := c.queue
	index, ok := q.allocated.dequeue()
	if !ok {
		var none T
		return none, false
	}
	// The index came off the allocated ring, so the slot holds a pushed
	// element and no other participant holds the slot until the enqueue below
	// hands it on. The slot is cleared so the queue keeps no reference to an
	// element that has left it.
	value := q.slots[index]
	var zero T
	q.slots[index] = zero
	q.free.enqueue(index)
	return value, true
}

// Capacity is the capacity of the queue this side drains.
func (c *ScqConsumer[T]) Capacity() int { return c.queue.Capacity() }

// Release gives the consumer place back. The handle is not usable afterwards.
func (c *ScqConsumer[T]) Release() {
	if c.queue != nil {
		c.queue.consumers.Add(^uint64(0))
		c.queue = nil
	}
}
