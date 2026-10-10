// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package conformance

// The linked Lamport row of the queue kind's generated packages (SCE
// Protocol-Synthesis RFC §synth-5-P), compiled and used: the segmented queue for
// one producer and one consumer, which exists on every architecture.
// queue_generated_lscq_test.go holds the list of SCQ rings, under the build
// constraint that row has.

import (
	"strings"
	"sync/atomic"
	"testing"

	element "github.com/newmassrael/sce-forge-runtime/conformance/generated/queue_conformance_event"
	linked "github.com/newmassrael/sce-forge-runtime/conformance/generated/queue_conformance_segmented"
	"github.com/newmassrael/sce-forge-runtime/queue"
)

// segmentBudget is the allocator a generated segmented queue is injected with in
// these tests: it gives at most limit segments at once, and states the progress
// it is told to.
type segmentBudget struct {
	limit    int64
	progress queue.Progress
	out      atomic.Int64
}

func (b *segmentBudget) Progress() queue.Progress { return b.progress }

func (b *segmentBudget) Allocate() bool {
	for {
		seen := b.out.Load()
		if seen >= b.limit {
			return false
		}
		if b.out.CompareAndSwap(seen, seen+1) {
			return true
		}
	}
}

func (b *segmentBudget) Deallocate() { b.out.Add(-1) }

func lockFreeBudget(limit int64) *segmentBudget {
	return &segmentBudget{limit: limit, progress: queue.LockFree}
}

// The generated alias is the runtime's queue over the element package's type.
var _ *queue.LinkedLamport[element.QueueConformanceEvent] = func() *linked.QueueConformanceSegmented {
	q, _ := linked.NewQueueConformanceSegmented(lockFreeBudget(1))
	return q
}()

// What the generated package states about its storage is checked when this file
// is compiled: a constant the template computes wrongly stops the build.
const _ = uint(linked.Segment - 3) // the segment is the document's: three

func TestTheLinkedLamportPackageStatesWhatTheDocumentRequiredAndWhatItGives(t *testing.T) {
	if linked.Segment != 3 {
		t.Fatalf("Segment = %d, want the document's 3", linked.Segment)
	}
	if linked.DeclaredProgress != "lock-free" || linked.PushProgress != "lock-free" || linked.PopProgress != "wait-free" {
		t.Fatalf("progress %q / %q / %q: a push no stronger than its allocator, a wait-free pop",
			linked.DeclaredProgress, linked.PushProgress, linked.PopProgress)
	}
	if !strings.Contains(linked.Algorithm, "Lamport") {
		t.Fatalf("one producer and one consumer select linked Lamport rings: %q", linked.Algorithm)
	}
	if linked.AllocatorProgress != queue.LockFree || linked.AllocatorProgressWord != "lock-free" {
		t.Fatalf("the allocator progress is the document's: %v %q", linked.AllocatorProgress, linked.AllocatorProgressWord)
	}
	q, built := linked.NewQueueConformanceSegmented(lockFreeBudget(1))
	if !built || q.Segment() != linked.Segment {
		t.Fatalf("the queue reports segment %d, the package states %d", q.Segment(), linked.Segment)
	}
}

func TestTheLinkedLamportPackageHandsElementsOverAcrossSegmentsAndReportsARefusedSegment(t *testing.T) {
	// Two segments of three: six elements, then the allocator refuses.
	q, built := linked.NewQueueConformanceSegmented(lockFreeBudget(2))
	if !built {
		t.Fatal("the allocator gives the first segment")
	}
	producer, okProducer := q.Producer()
	consumer, okConsumer := q.Consumer()
	if !okProducer || !okConsumer {
		t.Fatal("a fresh queue hands out a handle a side")
	}
	if _, ok := consumer.TryPop(); ok {
		t.Fatal("a new queue is empty")
	}
	for i := 0; i < 2*linked.Segment; i++ {
		if producer.TryPush(element.QueueConformanceEvent{SensorId: 1, Value: uint16(i)}) != queue.PushOK {
			t.Fatalf("push %d fits the two segments", i)
		}
	}
	if producer.TryPush(element.QueueConformanceEvent{SensorId: 9, Value: 99}) != queue.PushOutOfMemory {
		t.Fatal("the allocator refuses a third segment")
	}
	for i := 0; i < 2*linked.Segment; i++ {
		got, ok := consumer.TryPop()
		if !ok || got.SensorId != 1 || got.Value != uint16(i) {
			t.Fatalf("pop %d: got (%+v, %v): first in, first out", i, got, ok)
		}
	}
	if _, ok := consumer.TryPop(); ok {
		t.Fatal("drained")
	}
	if producer.TryPush(element.QueueConformanceEvent{SensorId: 1, Value: 100}) != queue.PushOK {
		t.Fatal("a segment the consumer gave back is room again")
	}
}

func TestTheLinkedLamportPackageRefusesAnAllocatorThatGivesLessThanTheDocumentDeclared(t *testing.T) {
	func() {
		defer func() {
			if recover() == nil {
				t.Fatal("a blocking allocator built a queue whose document declared lock-free")
			}
		}()
		linked.NewQueueConformanceSegmented(&segmentBudget{limit: 1, progress: queue.Blocking})
	}()
	if _, built := linked.NewQueueConformanceSegmented(lockFreeBudget(0)); built {
		t.Fatal("an allocator with no segment to give built a queue")
	}
}
