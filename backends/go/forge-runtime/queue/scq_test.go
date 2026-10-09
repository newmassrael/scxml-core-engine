// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//go:build amd64 || arm64 || loong64 || mips64 || mips64le || ppc64 || ppc64le || riscv64 || s390x || wasm

package queue

import (
	"sort"
	"sync"
	"sync/atomic"
	"testing"
	"time"
)

type scqSubject struct{ q *Scq[tracked] }

func (s scqSubject) capacity() int { return s.q.Capacity() }

func (s scqSubject) push(element tracked) PushStatus {
	producer, ok := s.q.Producer()
	if !ok {
		panic("a scenario holds one handle at a time")
	}
	defer producer.Release()
	return producer.TryPush(element)
}

func (s scqSubject) pop() (tracked, bool) {
	consumer, ok := s.q.Consumer()
	if !ok {
		panic("a scenario holds one handle at a time")
	}
	defer consumer.Release()
	return consumer.TryPop()
}

func init() {
	// The SCQ row exists here, so the contract arm runs its scenarios.
	newScqSubject = func(capacity int) subject {
		return scqSubject{NewScq[tracked](capacity, ceilPow2(capacity))}
	}
}

// mapTicket and unmap are inverse bijections on the entries, for rings below
// and above the size at which the rotation starts.
func TestTheEntryMapIsABijection(t *testing.T) {
	for _, slots := range []uint64{1, 2, 4, 8, 64} {
		r := newRing(slots, 0)
		seen := make([]bool, r.entryCount)
		for ticket := uint64(0); ticket < r.entryCount; ticket++ {
			position := r.mapTicket(ticket)
			if position >= r.entryCount {
				t.Fatalf("R=%d ticket %d maps outside the ring: %d", slots, ticket, position)
			}
			if seen[position] {
				t.Fatalf("R=%d: two tickets share entry %d", slots, position)
			}
			seen[position] = true
			if r.unmap(position) != ticket {
				t.Fatalf("R=%d: unmap does not invert map at ticket %d", slots, ticket)
			}
		}
	}
}

// A single-goroutine run of rounds fills and drains of a queue of capacity n:
// exact capacity, FIFO order, and the same again after the ring's cycles have
// moved many times.
func fillsAndDrains(t *testing.T, capacity, ringSlots, rounds int) {
	t.Helper()
	q := NewScq[uint64](capacity, ringSlots)
	producer, _ := q.Producer()
	consumer, _ := q.Consumer()
	var next, expected uint64
	for round := 0; round < rounds; round++ {
		for i := 0; i < capacity; i++ {
			if producer.TryPush(next) != PushOK {
				t.Fatalf("N=%d R=%d round %d: a queue below capacity refused a push", capacity, ringSlots, round)
			}
			next++
		}
		if producer.TryPush(next) != PushFull {
			t.Fatalf("N=%d R=%d round %d: a queue at capacity took a push", capacity, ringSlots, round)
		}
		for i := 0; i < capacity; i++ {
			got, ok := consumer.TryPop()
			if !ok || got != expected {
				t.Fatalf("N=%d R=%d: got (%d, %v), want %d", capacity, ringSlots, got, ok, expected)
			}
			expected++
		}
		if _, ok := consumer.TryPop(); ok {
			t.Fatalf("N=%d R=%d round %d: a drained queue is not empty", capacity, ringSlots, round)
		}
	}
}

func TestAnScqQueueFillsAndDrainsAcrossManyCycles(t *testing.T) {
	fillsAndDrains(t, 1, 1, 2000)
	fillsAndDrains(t, 3, 4, 2000)
	fillsAndDrains(t, 4, 4, 2000)
	fillsAndDrains(t, 5, 8, 2000)
	fillsAndDrains(t, 16, 16, 500)
}

// A side hands out no more handles than the ring has slots, and a place comes
// back when its handle is released.
func TestAnScqSideHandsOutNoMoreHandlesThanItHasPlaces(t *testing.T) {
	q := NewScq[int](2, 4)
	var producers []*ScqProducer[int]
	for {
		p, ok := q.Producer()
		if !ok {
			break
		}
		producers = append(producers, p)
		if len(producers) > 4 {
			t.Fatal("more producer handles than ring slots")
		}
	}
	if len(producers) != 4 {
		t.Fatalf("got %d producer handles, want 4: the ring is correct for as many participants a side as it has slots", len(producers))
	}
	if _, ok := producers[0].TryClone(); ok {
		t.Fatal("a clone past the places is refused")
	}
	producers[3].Release()
	if _, ok := q.Producer(); !ok {
		t.Fatal("a released handle gives its place back")
	}
}

// A ring that cannot run is refused when the queue is built.
func TestAnScqQueueRefusesAShapeItCannotRun(t *testing.T) {
	cases := []struct {
		name               string
		capacity, ringSlot int
	}{
		{"no capacity", 0, 4},
		{"a ring that is not a power of two", 3, 6},
		{"a ring smaller than the capacity", 8, 4},
	}
	for _, c := range cases {
		c := c
		t.Run(c.name, func(t *testing.T) {
			defer func() {
				if recover() == nil {
					t.Fatalf("NewScq(%d, %d) did not refuse", c.capacity, c.ringSlot)
				}
			}()
			NewScq[int](c.capacity, c.ringSlot)
		})
	}
}

// Go has no destructor, so what it owes is that a queue keeps no reference to
// an element that has left it: the slot a pop read is cleared.
func TestAnScqRingClearsAPoppedSlot(t *testing.T) {
	q := NewScq[*int](2, 2)
	producer, _ := q.Producer()
	consumer, _ := q.Consumer()
	value := 42
	if producer.TryPush(&value) != PushOK {
		t.Fatal("push")
	}
	if got, ok := consumer.TryPop(); !ok || got != &value {
		t.Fatalf("pop returned (%v, %v)", got, ok)
	}
	for position, held := range q.slots {
		if held != nil {
			t.Fatalf("slot %d still refers to a popped element", position)
		}
	}
}

type event struct {
	producer uint8
	sequence uint32
}

// Two producers and two consumers on real goroutines: nothing is lost,
// nothing is duplicated, and each consumer sees each producer's elements in
// the order they went in. The consumers stop when every element sent has been
// taken, not after a quiet spell, so a slow producer is never mistaken for a
// lost element; the deadline turns a genuinely lost element into a failure.
// Under -race this is also the data-race check the RFC asks of Go.
func TestTheScqQueueLosesNothingBetweenGoroutines(t *testing.T) {
	const perProducer = 20000
	const total = 2 * perProducer
	q := NewScq[event](8, 8)
	var taken atomic.Int64
	deadline := time.Now().Add(60 * time.Second)
	logs := make([][]event, 2)

	var wg sync.WaitGroup
	for c := 0; c < 2; c++ {
		c := c
		wg.Add(1)
		go func() {
			defer wg.Done()
			consumer, _ := q.Consumer()
			defer consumer.Release()
			for taken.Load() < total {
				if time.Now().After(deadline) {
					t.Error("the elements did not all arrive before the deadline")
					return
				}
				if e, ok := consumer.TryPop(); ok {
					taken.Add(1)
					logs[c] = append(logs[c], e)
				}
			}
		}()
	}
	for p := uint8(0); p < 2; p++ {
		p := p
		wg.Add(1)
		go func() {
			defer wg.Done()
			producer, _ := q.Producer()
			defer producer.Release()
			for i := uint32(0); i < perProducer; i++ {
				for producer.TryPush(event{producer: p, sequence: i}) != PushOK {
				}
			}
		}()
	}
	wg.Wait()

	perProducerSeen := map[uint8][]uint32{}
	for _, log := range logs {
		last := map[uint8]uint32{}
		seen := map[uint8]bool{}
		for _, e := range log {
			if seen[e.producer] && e.sequence <= last[e.producer] {
				t.Fatalf("producer %d's elements went backwards", e.producer)
			}
			last[e.producer], seen[e.producer] = e.sequence, true
			perProducerSeen[e.producer] = append(perProducerSeen[e.producer], e.sequence)
		}
	}
	for p := uint8(0); p < 2; p++ {
		values := perProducerSeen[p]
		sort.Slice(values, func(a, b int) bool { return values[a] < values[b] })
		if len(values) != perProducer {
			t.Fatalf("producer %d: got %d elements, want %d", p, len(values), perProducer)
		}
		for i, v := range values {
			if v != uint32(i) {
				t.Fatalf("producer %d: element %d is %d: every element exactly once", p, i, v)
			}
		}
	}
}
