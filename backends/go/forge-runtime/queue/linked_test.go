// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package queue

// The properties of the linked Lamport rings that the contract scenarios do not
// reach (SCE Protocol-Synthesis RFC §synth-5-P): the allocator held to the
// progress the document declared, the places of each side, the segments given
// back to the allocator's account, the slot a pop clears, and that two
// goroutines lose nothing and reorder nothing between them.

import (
	"runtime"
	"testing"
	"time"
)

// mustPanic fails the test unless fn panics.
func mustPanic(t *testing.T, what string, fn func()) {
	t.Helper()
	defer func() {
		if recover() == nil {
			t.Fatalf("%s did not panic", what)
		}
	}()
	fn()
}

func TestASegmentedQueueRefusesAnAllocatorThatGivesLessThanTheDocumentDeclared(t *testing.T) {
	allocator := newBudgetAllocator(100)
	for _, required := range []Progress{Blocking, LockFree} {
		if _, built := NewLinkedLamport[uint64](2, allocator, required); !built {
			t.Fatalf("an allocator that gives %v is enough for %v", allocator.Progress(), required)
		}
	}
	mustPanic(t, "a queue that claims more than its allocator gives", func() {
		NewLinkedLamport[uint64](2, allocator, WaitFree)
	})
	if allocator.live() != 2 {
		t.Fatalf("the two queues built hold one segment each, and the refused one asked for none: %d", allocator.live())
	}
}

func TestASegmentedQueueRefusesAShapeItCannotRunBeforeAskingTheAllocator(t *testing.T) {
	allocator := unlimitedAllocator()
	mustPanic(t, "a segment of no elements", func() { NewLinkedLamport[uint64](0, allocator, Blocking) })
	if allocator.granted() != 0 {
		t.Fatal("a bad shape is not an allocator's refusal")
	}
}

func TestASegmentedQueueThatCannotHaveItsFirstSegmentIsNotBuilt(t *testing.T) {
	if _, built := NewLinkedLamport[uint64](2, newBudgetAllocator(0), Blocking); built {
		t.Fatal("an allocator with no segment to give built a queue")
	}
}

func TestALinkedQueueGivesEachSegmentBackAsTheConsumerLeavesIt(t *testing.T) {
	allocator := unlimitedAllocator()
	q, _ := NewLinkedLamport[uint64](3, allocator, LockFree)
	producer, _ := q.Producer()
	consumer, _ := q.Consumer()
	for i := uint64(0); i < 4*3+1; i++ {
		if producer.TryPush(i) != PushOK {
			t.Fatalf("push %d", i)
		}
	}
	if allocator.live() != 5 {
		t.Fatalf("thirteen elements of three are in five segments, got %d", allocator.live())
	}
	for i := uint64(0); i < 4*3+1; i++ {
		if got, ok := consumer.TryPop(); !ok || got != i {
			t.Fatalf("pop %d gave (%d, %v)", i, got, ok)
		}
	}
	if _, ok := consumer.TryPop(); ok {
		t.Fatal("drained, yet a pop found an element")
	}
	if allocator.live() != 1 {
		t.Fatalf("the consumer gave back every segment it left, got %d still out", allocator.live())
	}
}

func TestALinkedQueueHoldsWhatTheAllocatorAllowsAndIsRoomAgainOnceASegmentIsGivenBack(t *testing.T) {
	q, _ := NewLinkedLamport[uint64](2, newBudgetAllocator(2), LockFree)
	producer, _ := q.Producer()
	consumer, _ := q.Consumer()
	for i := uint64(0); i < 4; i++ {
		if producer.TryPush(i) != PushOK {
			t.Fatalf("push %d", i)
		}
	}
	for attempt := 0; attempt < 2; attempt++ {
		if producer.TryPush(4) != PushOutOfMemory {
			t.Fatal("the allocator refuses a third segment, and a refusal changes nothing")
		}
	}
	for want := uint64(0); want < 3; want++ {
		if got, ok := consumer.TryPop(); !ok || got != want {
			t.Fatalf("pop gave (%d, %v), want %d", got, ok, want)
		}
	}
	if producer.TryPush(4) != PushOK {
		t.Fatal("the segment the consumer left was given back: room again")
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

func TestALinkedQueueHasOnePlaceASide(t *testing.T) {
	q, _ := NewLinkedLamport[uint64](2, unlimitedAllocator(), Blocking)
	producer, ok := q.Producer()
	if !ok {
		t.Fatal("a fresh queue hands out a producer")
	}
	if _, ok := q.Producer(); ok {
		t.Fatal("a second producer is not this algorithm")
	}
	consumer, ok := q.Consumer()
	if !ok {
		t.Fatal("a fresh queue hands out a consumer")
	}
	if _, ok := q.Consumer(); ok {
		t.Fatal("a second consumer is not this algorithm")
	}
	producer.Release()
	consumer.Release()
	producer.Release() // releasing twice is harmless
	if _, ok := q.Producer(); !ok {
		t.Fatal("a released place is free again")
	}
	if _, ok := q.Consumer(); !ok {
		t.Fatal("a released place is free again")
	}
}

func TestALinkedQueueClearsAPoppedSlot(t *testing.T) {
	type payload struct{ _ [64]byte }
	q, _ := NewLinkedLamport[*payload](2, unlimitedAllocator(), Blocking)
	producer, _ := q.Producer()
	consumer, _ := q.Consumer()
	if producer.TryPush(&payload{}) != PushOK {
		t.Fatal("push")
	}
	if got, ok := consumer.TryPop(); !ok || got == nil {
		t.Fatal("pop")
	}
	head := q.head.Load()
	for slot, held := range head.slots {
		if held != nil {
			t.Fatalf("slot %d still holds an element that has left the queue", slot)
		}
	}
}

func TestALinkedQueueLosesNothingBetweenGoroutines(t *testing.T) {
	allocator := unlimitedAllocator()
	q, _ := NewLinkedLamport[uint64](4, allocator, LockFree)
	producer, _ := q.Producer()
	consumer, _ := q.Consumer()
	const total = 50000
	failed := make(chan string, 2)
	done := make(chan struct{}, 2)
	deadline := time.Now().Add(120 * time.Second)
	go func() {
		defer func() { done <- struct{}{} }()
		for value := uint64(0); value < total; value++ {
			if producer.TryPush(value) != PushOK {
				failed <- "an allocator with no limit refused a segment"
				return
			}
		}
	}()
	go func() {
		defer func() { done <- struct{}{} }()
		expected := uint64(0)
		for expected < total {
			if time.Now().After(deadline) {
				failed <- "the elements did not all arrive before the deadline"
				return
			}
			got, ok := consumer.TryPop()
			switch {
			case !ok:
				runtime.Gosched()
			case got != expected:
				failed <- "popped out of order"
				return
			default:
				expected++
			}
		}
	}()
	<-done
	<-done
	select {
	case message := <-failed:
		t.Fatal(message)
	default:
	}
	if allocator.live() != 1 {
		t.Fatalf("every segment but the newest went back, got %d still out", allocator.live())
	}
}
