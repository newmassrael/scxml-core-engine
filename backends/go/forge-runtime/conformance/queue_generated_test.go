// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package conformance

// The queue kind's generated packages (SCE Protocol-Synthesis RFC §synth-5-P),
// compiled and used.
//
// sce-build's tests read what the generator writes for a queue and look for
// text in it, which shows the text is what the template says and nothing about
// whether the names in it are the names sce-forge-runtime/queue has. Here the
// package the generator writes for each bounded algorithm row is compiled into
// this test over the element package it imports, and the queue it defines is
// used: a rename in the runtime, or a constant the template computes wrongly,
// stops the build or fails an assertion. The runtime's own properties are
// queue's tests'; this file holds what the generated package adds.
//
// The packages come from tests/forge/resources/queue_conformance_*.scxml,
// written by generate.sh into the same tree as the numerical fixtures. This
// file holds the Lamport row; queue_generated_scq_test.go holds the SCQ row,
// under the build constraint that row has.

import (
	"strings"
	"testing"

	element "github.com/newmassrael/sce-forge-runtime/conformance/generated/queue_conformance_event"
	spsc "github.com/newmassrael/sce-forge-runtime/conformance/generated/queue_conformance_spsc"
	"github.com/newmassrael/sce-forge-runtime/queue"
)

// The generated alias is the runtime's queue over the element package's type.
var _ *queue.Spsc[element.QueueConformanceEvent] = spsc.NewQueueConformanceSpsc()

// What the generated package states about its storage is checked when this
// file is compiled: a constant the template computes wrongly stops the build.
const _ = uint(spsc.Capacity - 4) // the capacity is the document's: four

func TestTheLamportRingPackageStatesWhatTheDocumentRequiredAndWhatItGives(t *testing.T) {
	if spsc.Capacity != 4 {
		t.Fatalf("Capacity = %d, want the document's 4", spsc.Capacity)
	}
	for name, got := range map[string]string{
		"DeclaredProgress": spsc.DeclaredProgress,
		"PushProgress":     spsc.PushProgress,
		"PopProgress":      spsc.PopProgress,
	} {
		if got != "wait-free" {
			t.Fatalf("%s = %q, want wait-free", name, got)
		}
	}
	if !strings.Contains(spsc.Algorithm, "Lamport") {
		t.Fatalf("one producer and one consumer select the Lamport ring: %q", spsc.Algorithm)
	}
	if q := spsc.NewQueueConformanceSpsc(); q.Capacity() != spsc.Capacity {
		t.Fatalf("the queue reports capacity %d, the package states %d", q.Capacity(), spsc.Capacity)
	}
}

func TestTheLamportRingPackageHandsElementsOverInOrderAndRefusesWhenFull(t *testing.T) {
	q := spsc.NewQueueConformanceSpsc()
	producer, okProducer := q.Producer()
	consumer, okConsumer := q.Consumer()
	if !okProducer || !okConsumer {
		t.Fatal("a fresh queue hands out a handle a side")
	}

	if _, ok := consumer.TryPop(); ok {
		t.Fatal("a new queue is empty")
	}
	for i := 0; i < spsc.Capacity; i++ {
		if producer.TryPush(element.QueueConformanceEvent{SensorId: 1, Value: uint16(i)}) != queue.PushOK {
			t.Fatalf("push %d of %d must fit", i, spsc.Capacity)
		}
	}
	if producer.TryPush(element.QueueConformanceEvent{SensorId: 9, Value: 99}) != queue.PushFull {
		t.Fatal("a full queue refuses the push")
	}
	for i := 0; i < spsc.Capacity; i++ {
		got, ok := consumer.TryPop()
		if !ok || got.SensorId != 1 || got.Value != uint16(i) {
			t.Fatalf("pop %d: got (%+v, %v): first in, first out", i, got, ok)
		}
	}
	if _, ok := consumer.TryPop(); ok {
		t.Fatal("drained")
	}
}
