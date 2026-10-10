// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The list-of-SCQ-rings row of the queue kind's generated packages (SCE
// Protocol-Synthesis RFC §synth-5-P), under the same architectures as the package
// the generator writes for it and the runtime it names: the list is the
// generator's QUEUE_SCQ_GO_ARCHITECTURES, which a test in sce-build holds equal
// to the runtime's.

//go:build amd64 || arm64 || loong64 || mips64 || mips64le || ppc64 || ppc64le || riscv64 || s390x || wasm

package conformance

import (
	"strings"
	"testing"

	element "github.com/newmassrael/sce-forge-runtime/conformance/generated/queue_conformance_event"
	lscq "github.com/newmassrael/sce-forge-runtime/conformance/generated/queue_conformance_segmented_many"
	"github.com/newmassrael/sce-forge-runtime/queue"
)

// The generated alias is the runtime's queue over the element package's type.
var _ *queue.Lscq[element.QueueConformanceEvent] = func() *lscq.QueueConformanceSegmentedMany {
	q, _ := lscq.NewQueueConformanceSegmentedMany(lockFreeBudget(1))
	return q
}()

// What the generated package states about its storage is checked when this file
// is compiled: a constant the template computes wrongly stops the build.
const (
	_ = uint(lscq.RingSlots - lscq.Segment)      // the ring is at least the segment
	_ = uint(lscq.RingSlots - lscq.Participants) // and at least the participants
)

func TestTheLscqPackageSizesItsRingsFromSegmentAndParticipants(t *testing.T) {
	if lscq.Segment != 3 || lscq.Participants != 2 {
		t.Fatalf("Segment, Participants = %d, %d, want the document's 3, 2", lscq.Segment, lscq.Participants)
	}
	if lscq.RingSlots != 4 {
		t.Fatalf("RingSlots = %d, want the next power of two at or above max(segment, participants): 4", lscq.RingSlots)
	}
	if lscq.DeclaredProgress != "lock-free" || lscq.PushProgress != "lock-free" || lscq.PopProgress != "lock-free" {
		t.Fatalf("progress %q / %q / %q, want lock-free on both sides", lscq.DeclaredProgress, lscq.PushProgress, lscq.PopProgress)
	}
	if !strings.Contains(lscq.Algorithm, "LSCQ") {
		t.Fatalf("many producers and consumers select LSCQ: %q", lscq.Algorithm)
	}
	if lscq.AllocatorProgress != queue.LockFree {
		t.Fatalf("the allocator progress is the document's: %v", lscq.AllocatorProgress)
	}
	if lscq.WrapBoundOps != uint64(1)<<62 {
		t.Fatalf("WrapBoundOps = %d, want 2^62", uint64(lscq.WrapBoundOps))
	}
	q, built := lscq.NewQueueConformanceSegmentedMany(lockFreeBudget(1))
	if !built || q.Segment() != lscq.Segment || q.RingSlots() != lscq.RingSlots {
		t.Fatal("the queue reports the shape the package states")
	}
}

func TestTheLscqPackageHandsElementsOverAcrossSegmentsAndGivesEachSegmentBack(t *testing.T) {
	allocator := lockFreeBudget(1 << 30)
	q, built := lscq.NewQueueConformanceSegmentedMany(allocator)
	if !built {
		t.Fatal("the allocator gives the first segment")
	}
	producer, okProducer := q.Producer()
	consumer, okConsumer := q.Consumer()
	if !okProducer || !okConsumer {
		t.Fatal("a fresh queue hands out a handle a side")
	}
	const total = 4*lscq.Segment + 1
	if _, ok := consumer.TryPop(); ok {
		t.Fatal("a new queue is empty")
	}
	for i := 0; i < total; i++ {
		if producer.TryPush(element.QueueConformanceEvent{SensorId: 2, Value: uint16(i)}) != queue.PushOK {
			t.Fatalf("push %d", i)
		}
	}
	for i := 0; i < total; i++ {
		got, ok := consumer.TryPop()
		if !ok || got.SensorId != 2 || got.Value != uint16(i) {
			t.Fatalf("pop %d: got (%+v, %v): first in, first out", i, got, ok)
		}
	}
	if _, ok := consumer.TryPop(); ok {
		t.Fatal("drained")
	}
	if allocator.out.Load() != 1 {
		t.Fatalf("only the newest segment is still the queue's, got %d", allocator.out.Load())
	}
}

func TestTheLscqPackageHandsOutNoMoreHandlesThanItsRingHasSlots(t *testing.T) {
	q, _ := lscq.NewQueueConformanceSegmentedMany(lockFreeBudget(1))
	held := 0
	for {
		if _, ok := q.Producer(); !ok {
			break
		}
		held++
		if held > lscq.RingSlots {
			t.Fatal("more producer handles than ring slots")
		}
	}
	if held != lscq.RingSlots {
		t.Fatalf("got %d producer handles, want the ring's %d", held, lscq.RingSlots)
	}
}

func TestTheLscqPackageRefusesAnAllocatorThatGivesLessThanTheDocumentDeclared(t *testing.T) {
	func() {
		defer func() {
			if recover() == nil {
				t.Fatal("a blocking allocator built a queue whose document declared lock-free")
			}
		}()
		lscq.NewQueueConformanceSegmentedMany(&segmentBudget{limit: 1, progress: queue.Blocking})
	}()
	if _, built := lscq.NewQueueConformanceSegmentedMany(lockFreeBudget(0)); built {
		t.Fatal("an allocator with no segment to give built a queue")
	}
}
