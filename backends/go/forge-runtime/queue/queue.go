// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Package queue is the runtime half of `sce:kind="queue"` (SCE
// Protocol-Synthesis RFC §synth-5-P) for Go: queues that hand elements from
// one goroutine to another.
//
// The document states a contract and names no algorithm; each algorithm the
// kind's selection table names lives here once. The contract every one of
// them keeps: a linearizable FIFO in which the push of an element
// happens-before the pop that returns it, never holding more than the
// capacity declared and holding exactly that when nothing is running. The
// SCQ row may refuse a push while another participant's operation holds a
// slot (at most one per participant); the Lamport row never does.
//
//   - Spsc is the `bounded` row for one producer and one consumer: a Lamport
//     ring, wait-free on both sides.
//   - Scq is the `bounded` row for any other cardinality: Nikolaev's SCQ data
//     queue (DISC 2019), lock-free on both sides, in the form of the
//     authors' single-width-CAS reference implementation (lfring_cas1.h,
//     dual 2-clause BSD / MIT). It exists only on a GOARCH whose 64-bit
//     atomics are native (scq.go), which is the set the RFC refuses that row
//     for elsewhere.
//
// The same two algorithms as the Rust and C++ runtimes. Memory order is not a
// parameter, and Go does not offer a choice: every operation of sync/atomic
// is sequentially consistent, which is stronger than either algorithm
// requires and still correct (RFC §synth-5-P, Backends).
//
// What differs from the languages with destructors and constant generics:
//
//   - The capacity is an argument of the constructor, because Go has no
//     array length that is a type parameter. The storage is one slice,
//     allocated by the constructor and never again: no operation allocates.
//   - A popped element's slot is cleared, so a queue holds no reference to an
//     element that has left it and the collector may reclaim it. A queue that
//     is dropped with elements in it drops them with it, because the
//     collector owns them; there is no destructor to run.
//   - A handle (Producer, Consumer) holds one place of its side until
//     Release is called. Go has no destructor to give the place back, so the
//     caller does, usually by defer.
//
// A refused push needs no hand-back: TryPush takes its element by value, so
// the caller still holds it.
package queue

import "sync/atomic"

// PushStatus says whether a push took its element. PushOutOfMemory is the
// `segmented` storage mode's, reported when its injected allocator refuses a
// segment; a `bounded` queue only ever reports PushFull.
type PushStatus uint8

const (
	// PushOK means the element is in the queue.
	PushOK PushStatus = iota
	// PushFull means the queue held its capacity, or (SCQ) other participants'
	// operations held the slots it was short of. The element was not taken.
	PushFull
	// PushOutOfMemory means a segmented queue's allocator refused a segment.
	PushOutOfMemory
)

// cacheLine is the size a hot index keeps to itself, so two indices that two
// cores update do not contend for one line. It costs a line each and changes
// nothing about correctness.
const cacheLine = 64

// paddedUint64 keeps an atomic counter on a cache line of its own.
type paddedUint64 struct {
	_ [cacheLine]byte
	v atomic.Uint64
	_ [cacheLine]byte
}

// paddedInt64 keeps a signed atomic counter on a cache line of its own.
type paddedInt64 struct {
	_ [cacheLine]byte
	v atomic.Int64
	_ [cacheLine]byte
}

// takePlace takes one of limit places of a side, if one is free. A
// compare-and-swap loop and not an add, so a refused request leaves the count
// as it was and no concurrent request can be refused on account of it.
func takePlace(alive *atomic.Uint64, limit uint64) bool {
	for {
		seen := alive.Load()
		if seen >= limit {
			return false
		}
		if alive.CompareAndSwap(seen, seen+1) {
			return true
		}
	}
}
