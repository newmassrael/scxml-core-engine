// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The SCQ row of the queue kind's generated packages (SCE Protocol-Synthesis
// RFC §synth-5-P), under the same architectures as the package the generator
// writes for it and the runtime it names: the list is the generator's
// QUEUE_SCQ_GO_ARCHITECTURES, which a test in sce-build holds equal to the
// runtime's.

//go:build amd64 || arm64 || loong64 || mips64 || mips64le || ppc64 || ppc64le || riscv64 || s390x || wasm

package conformance

import (
	"strings"
	"sync"
	"sync/atomic"
	"testing"
	"time"

	element "github.com/newmassrael/sce-forge-runtime/conformance/generated/queue_conformance_event"
	scq "github.com/newmassrael/sce-forge-runtime/conformance/generated/queue_conformance_scq"
	"github.com/newmassrael/sce-forge-runtime/queue"
)

// The generated alias is the runtime's queue over the element package's type.
var _ *queue.Scq[element.QueueConformanceEvent] = scq.NewQueueConformanceScq()

// What the generated package states about its storage is checked when this
// file is compiled: a constant the template computes wrongly stops the build.
const (
	_ = uint(scq.RingSlots - scq.Capacity)     // the ring is at least the capacity
	_ = uint(scq.RingSlots - scq.Participants) // and at least the participants
)

func TestTheScqPackageSizesItsRingsFromCapacityAndParticipants(t *testing.T) {
	if scq.Capacity != 6 || scq.Participants != 3 {
		t.Fatalf("Capacity, Participants = %d, %d, want the document's 6, 3", scq.Capacity, scq.Participants)
	}
	if scq.RingSlots != 8 {
		t.Fatalf("RingSlots = %d, want the next power of two at or above max(capacity, participants): 8", scq.RingSlots)
	}
	if scq.RingSlots&(scq.RingSlots-1) != 0 {
		t.Fatalf("RingSlots = %d is not a power of two", scq.RingSlots)
	}
	for name, got := range map[string]string{
		"DeclaredProgress": scq.DeclaredProgress,
		"PushProgress":     scq.PushProgress,
		"PopProgress":      scq.PopProgress,
	} {
		if got != "lock-free" {
			t.Fatalf("%s = %q, want lock-free", name, got)
		}
	}
	if !strings.Contains(scq.Algorithm, "SCQ") {
		t.Fatalf("many producers and consumers select SCQ: %q", scq.Algorithm)
	}
	if scq.WrapBoundOps != uint64(1)<<62 {
		t.Fatalf("WrapBoundOps = %d, want 2^62", uint64(scq.WrapBoundOps))
	}
	if q := scq.NewQueueConformanceScq(); q.Capacity() != scq.Capacity {
		t.Fatalf("the queue reports capacity %d, the package states %d", q.Capacity(), scq.Capacity)
	}
}

func TestTheScqPackageHoldsExactlyItsCapacityAtRestAndKeepsOrder(t *testing.T) {
	q := scq.NewQueueConformanceScq()
	producer, okProducer := q.Producer()
	consumer, okConsumer := q.Consumer()
	if !okProducer || !okConsumer {
		t.Fatal("a fresh queue hands out a handle a side")
	}
	for i := 0; i < scq.Capacity; i++ {
		if producer.TryPush(element.QueueConformanceEvent{SensorId: 2, Value: uint16(i)}) != queue.PushOK {
			t.Fatalf("push %d of %d must fit", i, scq.Capacity)
		}
	}
	if producer.TryPush(element.QueueConformanceEvent{SensorId: 9, Value: 99}) != queue.PushFull {
		t.Fatal("a seventh element does not fit a queue of six")
	}
	for i := 0; i < scq.Capacity; i++ {
		got, ok := consumer.TryPop()
		if !ok || got.SensorId != 2 || got.Value != uint16(i) {
			t.Fatalf("pop %d: got (%+v, %v): first in, first out", i, got, ok)
		}
	}
	if _, ok := consumer.TryPop(); ok {
		t.Fatal("drained")
	}
}

func TestTheScqPackageHandsOutNoMoreHandlesThanItsRingHasSlots(t *testing.T) {
	q := scq.NewQueueConformanceScq()
	held := 0
	for {
		if _, ok := q.Producer(); !ok {
			break
		}
		held++
		if held > scq.RingSlots {
			t.Fatal("more producer handles than ring slots")
		}
	}
	if held != scq.RingSlots {
		t.Fatalf("got %d producer handles, want the ring's %d", held, scq.RingSlots)
	}
}

// Two producers and two consumers on real goroutines: nothing is lost, nothing
// is duplicated. Under -race this is also the data-race check the RFC asks of
// Go.
func TestTheScqPackageLosesNothingBetweenGoroutines(t *testing.T) {
	const perProducer = 5000
	const total = 2 * perProducer
	q := scq.NewQueueConformanceScq()
	var taken atomic.Int64
	deadline := time.Now().Add(60 * time.Second)
	var mu sync.Mutex
	seen := map[[2]uint16]int{}

	var wg sync.WaitGroup
	for c := 0; c < 2; c++ {
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
					mu.Lock()
					seen[[2]uint16{uint16(e.SensorId), e.Value}]++
					mu.Unlock()
				}
			}
		}()
	}
	for sensor := uint8(1); sensor <= 2; sensor++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			producer, _ := q.Producer()
			defer producer.Release()
			for i := uint16(0); i < perProducer; i++ {
				for producer.TryPush(element.QueueConformanceEvent{SensorId: sensor, Value: i}) != queue.PushOK {
				}
			}
		}()
	}
	wg.Wait()

	if len(seen) != total {
		t.Fatalf("got %d distinct elements, want %d", len(seen), total)
	}
	for key, count := range seen {
		if count != 1 {
			t.Fatalf("element %v arrived %d times", key, count)
		}
	}
}
