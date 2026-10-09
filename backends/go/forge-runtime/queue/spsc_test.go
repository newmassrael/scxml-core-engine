// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package queue

import (
	"sync"
	"testing"
	"time"
)

// A Lamport ring keeps order across many laps of its index space.
func TestALamportRingKeepsOrderAcrossManyLaps(t *testing.T) {
	q := NewSpsc[uint64](3)
	producer, _ := q.Producer()
	consumer, _ := q.Consumer()
	var next, expected uint64
	for i := 0; i < 20000; i++ {
		if producer.TryPush(next) != PushOK {
			t.Fatalf("push %d: no room for one", i)
		}
		next++
		if i%3 == 2 {
			for k := 0; k < 3; k++ {
				got, ok := consumer.TryPop()
				if !ok || got != expected {
					t.Fatalf("pop: got (%d, %v), want %d", got, ok, expected)
				}
				expected++
			}
		}
	}
}

// A queue at capacity refuses the next push and still holds what it held.
func TestALamportRingRefusesAtCapacityAndKeepsWhatItHolds(t *testing.T) {
	q := NewSpsc[int](2)
	producer, _ := q.Producer()
	consumer, _ := q.Consumer()
	if producer.TryPush(1) != PushOK || producer.TryPush(2) != PushOK {
		t.Fatal("a queue below capacity takes a push")
	}
	if producer.TryPush(3) != PushFull {
		t.Fatal("a queue at capacity refuses the next push")
	}
	for _, want := range []int{1, 2} {
		if got, ok := consumer.TryPop(); !ok || got != want {
			t.Fatalf("got (%d, %v), want %d", got, ok, want)
		}
	}
	if _, ok := consumer.TryPop(); ok {
		t.Fatal("a drained queue is empty")
	}
}

// A side has one place, and a place comes back when its handle is released.
func TestALamportRingHasOnePlaceASide(t *testing.T) {
	q := NewSpsc[int](2)
	producer, ok := q.Producer()
	if !ok {
		t.Fatal("the one producer place is free")
	}
	if _, again := q.Producer(); again {
		t.Fatal("a ring shared by two producers is not the Lamport ring")
	}
	consumer, ok := q.Consumer()
	if !ok {
		t.Fatal("the consumer place is its own")
	}
	if _, again := q.Consumer(); again {
		t.Fatal("a ring shared by two consumers is not the Lamport ring")
	}
	producer.Release()
	consumer.Release()
	if _, ok := q.Producer(); !ok {
		t.Fatal("releasing the producer gives its place back")
	}
	if _, ok := q.Consumer(); !ok {
		t.Fatal("releasing the consumer gives its place back")
	}
}

// A handle taken again sees what the earlier one left: its indices are read
// from the queue, not assumed.
func TestANewLamportHandleContinuesWhereTheLastStopped(t *testing.T) {
	q := NewSpsc[int](4)
	push := func(v int) {
		p, _ := q.Producer()
		defer p.Release()
		if p.TryPush(v) != PushOK {
			t.Fatalf("push %d", v)
		}
	}
	pop := func() (int, bool) {
		c, _ := q.Consumer()
		defer c.Release()
		return c.TryPop()
	}
	push(7)
	push(8)
	a, okA := pop()
	b, okB := pop()
	if !okA || !okB || a != 7 || b != 8 {
		t.Fatalf("got (%d, %v) and (%d, %v), want 7 then 8", a, okA, b, okB)
	}
}

// Go has no destructor, so what it owes is that a queue keeps no reference to
// an element that has left it: the slot a pop read is cleared.
func TestALamportRingClearsAPoppedSlot(t *testing.T) {
	q := NewSpsc[*int](2)
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

// One producer and one consumer goroutine: order and count. Under -race this
// is also the data-race check the RFC asks of Go.
func TestALamportRingLosesNothingBetweenGoroutines(t *testing.T) {
	const count = 200000
	q := NewSpsc[uint32](16)
	got := make([]uint32, 0, count)
	deadline := time.Now().Add(60 * time.Second)

	var wg sync.WaitGroup
	wg.Add(2)
	go func() {
		defer wg.Done()
		consumer, _ := q.Consumer()
		defer consumer.Release()
		for len(got) < count {
			if time.Now().After(deadline) {
				t.Error("the elements did not all arrive before the deadline")
				return
			}
			if v, ok := consumer.TryPop(); ok {
				got = append(got, v)
			}
		}
	}()
	go func() {
		defer wg.Done()
		producer, _ := q.Producer()
		defer producer.Release()
		for i := uint32(0); i < count; i++ {
			for producer.TryPush(i) != PushOK {
			}
		}
	}()
	wg.Wait()

	if len(got) != count {
		t.Fatalf("got %d elements, want %d", len(got), count)
	}
	for i, v := range got {
		if v != uint32(i) {
			t.Fatalf("element %d is %d: not in the order pushed", i, v)
		}
	}
}
