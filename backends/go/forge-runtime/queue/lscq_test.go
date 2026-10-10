// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//go:build amd64 || arm64 || loong64 || mips64 || mips64le || ppc64 || ppc64le || riscv64 || s390x || wasm

package queue

// The properties of the list of SCQ rings that the contract scenarios do not
// reach (SCE Protocol-Synthesis RFC §synth-5-P): a ring that stays closed, the
// allocator's refusal and hand-back, the places of each side, the slot a pop
// clears, and that real goroutines lose nothing and reorder nothing between
// them.

import (
	"runtime"
	"sync"
	"sync/atomic"
	"testing"
	"time"
)

type lscqSubject struct{ q *Lscq[tracked] }

func (s lscqSubject) capacity() int { panic("a segmented queue has no capacity to ask for") }

func (s lscqSubject) push(element tracked) PushStatus {
	producer, ok := s.q.Producer()
	if !ok {
		panic("a scenario holds one handle at a time")
	}
	defer producer.Release()
	return producer.TryPush(element)
}

func (s lscqSubject) pop() (tracked, bool) {
	consumer, ok := s.q.Consumer()
	if !ok {
		panic("a scenario holds one handle at a time")
	}
	defer consumer.Release()
	return consumer.TryPop()
}

func init() {
	// The SCQ row exists here, so the contract arm runs its segmented
	// scenarios over a list of rings.
	newLscqSubject = func(segment int, allocator SegmentAllocator) subject {
		q, built := NewLscq[tracked](segment, ceilPow2(segment), allocator, LockFree)
		if !built {
			panic("the allocator refused the first segment")
		}
		return lscqSubject{q}
	}
}

func TestARingStaysClosedOnceAPushFoundItWithoutAFreeSlot(t *testing.T) {
	ring := NewScq[uint64](2, 2)
	if !ring.pushOrClose(1) || !ring.pushOrClose(2) {
		t.Fatal("a ring with two free slots takes two elements")
	}
	if ring.pushOrClose(3) {
		t.Fatal("no free slot closes the ring")
	}
	if got, ok := ring.pop(); !ok || got != 1 {
		t.Fatalf("pop gave (%d, %v), want 1", got, ok)
	}
	if ring.pushOrClose(4) {
		t.Fatal("a closed ring takes nothing more, though a slot is free again")
	}
	if got, ok := ring.popDrained(); !ok || got != 2 {
		t.Fatalf("popDrained gave (%d, %v), want 2", got, ok)
	}
	if _, ok := ring.popDrained(); ok {
		t.Fatal("a closed ring that has been drained will never hold an element")
	}
	if ring.pushOrClose(5) {
		t.Fatal("a closed ring is closed for good")
	}
	if _, ok := ring.popDrained(); ok {
		t.Fatal("a refused push left an element in the ring")
	}
}

func TestAnLscqRefusesAnAllocatorThatGivesLessThanTheDocumentDeclared(t *testing.T) {
	allocator := newBudgetAllocator(100)
	for _, required := range []Progress{Blocking, LockFree} {
		if _, built := NewLscq[uint64](2, 2, allocator, required); !built {
			t.Fatalf("an allocator that gives %v is enough for %v", allocator.Progress(), required)
		}
	}
	mustPanic(t, "a queue that claims more than its allocator gives", func() {
		NewLscq[uint64](2, 2, allocator, WaitFree)
	})
}

func TestAnLscqRefusesAShapeItCannotRunBeforeAskingTheAllocator(t *testing.T) {
	allocator := unlimitedAllocator()
	mustPanic(t, "no segment", func() { NewLscq[uint64](0, 1, allocator, Blocking) })
	mustPanic(t, "a ring size that is not a power of two", func() { NewLscq[uint64](2, 3, allocator, Blocking) })
	mustPanic(t, "a ring smaller than the segment", func() { NewLscq[uint64](4, 2, allocator, Blocking) })
	if allocator.granted() != 0 {
		t.Fatal("a bad shape is not an allocator's refusal")
	}
	if _, built := NewLscq[uint64](2, 2, newBudgetAllocator(0), Blocking); built {
		t.Fatal("an allocator with no segment to give built a queue")
	}
}

func TestAnLscqTakesExactlyASegmentIntoARingAndOnlyTheNewestSegmentSurvivesADrain(t *testing.T) {
	for _, segment := range []int{1, 2, 3, 5} {
		allocator := unlimitedAllocator()
		q, _ := NewLscq[uint64](segment, ceilPow2(segment), allocator, LockFree)
		producer, _ := q.Producer()
		consumer, _ := q.Consumer()
		var next, expected uint64
		for round := 0; round < 100; round++ {
			for i := 0; i < 4*segment+1; i++ {
				if producer.TryPush(next) != PushOK {
					t.Fatalf("segment %d: push %d", segment, next)
				}
				next++
			}
			for {
				got, ok := consumer.TryPop()
				if !ok {
					break
				}
				if got != expected {
					t.Fatalf("segment %d: popped %d, wanted %d", segment, got, expected)
				}
				expected++
			}
			if allocator.live() != 1 {
				t.Fatalf("segment %d: only the newest segment is still the queue's, got %d", segment, allocator.live())
			}
		}
		if next != expected {
			t.Fatalf("segment %d: %d pushed, %d popped", segment, next, expected)
		}
		// The first segment, then four more in every round: the segment a round
		// starts in takes a segment's worth and the rest need four.
		if allocator.granted() != 401 {
			t.Fatalf("segment %d: every round needed four more segments, granted %d", segment, allocator.granted())
		}
	}
}

func TestAnLscqRefusedASegmentLeavesTheQueueAsItWasAndLosesNothingPushed(t *testing.T) {
	q, _ := NewLscq[uint64](2, 2, newBudgetAllocator(2), LockFree)
	producer, _ := q.Producer()
	consumer, _ := q.Consumer()
	for i := uint64(0); i < 4; i++ {
		if producer.TryPush(i) != PushOK {
			t.Fatalf("push %d", i)
		}
	}
	for attempt := 0; attempt < 3; attempt++ {
		if producer.TryPush(99) != PushOutOfMemory {
			t.Fatal("refused again, and each time the same")
		}
	}
	for want := uint64(0); want < 3; want++ {
		if got, ok := consumer.TryPop(); !ok || got != want {
			t.Fatalf("pop gave (%d, %v), want %d", got, ok, want)
		}
	}
	if producer.TryPush(4) != PushOK {
		t.Fatal("the segment the consumer left is given back: room again")
	}
	for want := uint64(3); want <= 4; want++ {
		if got, ok := consumer.TryPop(); !ok || got != want {
			t.Fatalf("pop gave (%d, %v), want %d", got, ok, want)
		}
	}
	if _, ok := consumer.TryPop(); ok {
		t.Fatal("drained, yet a pop found an element")
	}
}

func TestAnLscqSideHandsOutNoMoreHandlesThanItsRingHasSlots(t *testing.T) {
	q, _ := NewLscq[uint64](2, 4, unlimitedAllocator(), LockFree)
	var producers []*LscqProducer[uint64]
	for i := 0; i < 4; i++ {
		producer, ok := q.Producer()
		if !ok {
			t.Fatalf("producer %d of 4", i)
		}
		producers = append(producers, producer)
	}
	if _, ok := q.Producer(); ok {
		t.Fatal("a ring cannot be shared by more enqueuers than it has slots")
	}
	if _, ok := producers[0].TryClone(); ok {
		t.Fatal("every place is taken")
	}
	producers[0].Release()
	clone, ok := producers[1].TryClone()
	if !ok {
		t.Fatal("a released place is free again")
	}
	clone.Release()
	var consumers []*LscqConsumer[uint64]
	for i := 0; i < 4; i++ {
		consumer, ok := q.Consumer()
		if !ok {
			t.Fatalf("consumer %d of 4", i)
		}
		consumers = append(consumers, consumer)
	}
	if _, ok := q.Consumer(); ok {
		t.Fatal("a ring cannot be shared by more dequeuers than it has slots")
	}
	if _, ok := consumers[0].TryClone(); ok {
		t.Fatal("every place is taken")
	}
}

func TestAnLscqClearsAPoppedSlot(t *testing.T) {
	type payload struct{ _ [64]byte }
	q, _ := NewLscq[*payload](2, 2, unlimitedAllocator(), LockFree)
	producer, _ := q.Producer()
	consumer, _ := q.Consumer()
	if producer.TryPush(&payload{}) != PushOK {
		t.Fatal("push")
	}
	if got, ok := consumer.TryPop(); !ok || got == nil {
		t.Fatal("pop")
	}
	for slot, held := range q.head.Load().ring.slots {
		if held != nil {
			t.Fatalf("slot %d still holds an element that has left the queue", slot)
		}
	}
}

func TestAnLscqLosesNothingAndReordersNothingBetweenGoroutines(t *testing.T) {
	// The allocator allows only a few segments at once, so producers meet
	// refusals and wait for the consumers to give segments back: the refusal
	// and the hand-back run against each other as well.
	allocator := newBudgetAllocator(6)
	const producers, consumers = 3, 3
	const perProducer = 20000
	q, _ := NewLscq[uint64](4, 4, allocator, LockFree)
	const total = uint64(producers * perProducer)
	var delivered atomic.Uint64
	deadline := time.Now().Add(120 * time.Second)
	// Each consumer keeps the last value it saw from each producer: one
	// producer's elements reach one consumer in the order they were pushed.
	seen := make([][producers]int64, consumers)
	counts := make([][producers]uint64, consumers)
	for c := range seen {
		for p := range seen[c] {
			seen[c][p] = -1
		}
	}
	var wg sync.WaitGroup
	var failures atomic.Int64
	for p := 0; p < producers; p++ {
		producer, ok := q.Producer()
		if !ok {
			t.Fatal("a place for every producer")
		}
		wg.Add(1)
		go func() {
			defer wg.Done()
			defer producer.Release()
			for i := uint64(0); i < perProducer; i++ {
				for producer.TryPush(uint64(p)*perProducer+i) != PushOK {
					if time.Now().After(deadline) {
						failures.Add(1)
						return
					}
					runtime.Gosched()
				}
			}
		}()
	}
	for c := 0; c < consumers; c++ {
		consumer, ok := q.Consumer()
		if !ok {
			t.Fatal("a place for every consumer")
		}
		wg.Add(1)
		go func() {
			defer wg.Done()
			defer consumer.Release()
			for delivered.Load() < total {
				if time.Now().After(deadline) {
					failures.Add(1)
					return
				}
				got, ok := consumer.TryPop()
				if !ok {
					runtime.Gosched()
					continue
				}
				from := int(got / perProducer)
				if int64(got) <= seen[c][from] {
					failures.Add(1)
				}
				seen[c][from] = int64(got)
				counts[c][from]++
				delivered.Add(1)
			}
		}()
	}
	wg.Wait()
	if failures.Load() != 0 {
		t.Fatalf("%d failure(s): a reordered element or the deadline", failures.Load())
	}
	for p := 0; p < producers; p++ {
		var sum uint64
		for c := 0; c < consumers; c++ {
			sum += counts[c][p]
		}
		if sum != perProducer {
			t.Fatalf("producer %d: %d of %d elements arrived", p, sum, perProducer)
		}
	}
	if live := allocator.live(); live < 1 || live > 6 {
		t.Fatalf("the allocator's ceiling held and every other segment went back, got %d", live)
	}
}
