// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package queue

// The segmented queues' runs written as histories (SCE Protocol-Synthesis RFC
// §synth-5-P, verification layer 2). See history_test.go for what is recorded
// and how. The recorder here is for any segmented queue: it is handed the
// handles' operations, so the list of SCQ rings (lscq_history_test.go, under the
// architectures it exists on) uses it as the linked Lamport rings do.
//
// A segmented queue has no capacity, and the recording gives none that could
// matter: the history is judged against a queue that holds every value, so a
// push is never refused, and an empty pop is judged exactly as for any other
// queue.

import (
	"fmt"
	"runtime"
	"sync"
	"sync/atomic"
	"testing"
	"time"
)

// runsPerSegmentedShape runs of each segmented shape are recorded, for the same
// reason as the SCQ shapes'.
const runsPerSegmentedShape = 25

// producing is a producing handle of whichever queue a run is on: what it
// pushes with and how it gives its place back.
type producing struct {
	push    func(uint64) PushStatus
	release func()
}

// consuming is a consuming handle of whichever queue a run is on.
type consuming struct {
	pop     func() (uint64, bool)
	release func()
}

// recordSegmentedRun records one goroutine for each of producers and for each
// of consumers, each producer pushing perProducer distinct values, judged
// against a sequential queue that holds all of them.
func recordSegmentedRun(t *testing.T, producers []producing, consumers []consuming, perProducer uint64) historyDocument {
	t.Helper()
	var clock atomic.Uint64
	tick := func() uint64 { return clock.Add(1) }
	deadline := time.Now().Add(120 * time.Second)
	total := uint64(len(producers)) * perProducer
	var delivered atomic.Uint64
	logs := make([][]historyOperation, len(producers)+len(consumers))

	var wg sync.WaitGroup
	for p := range producers {
		wg.Add(1)
		go func() {
			defer wg.Done()
			defer producers[p].release()
			for i := uint64(0); i < perProducer; i++ {
				value := uint64(p)*perProducer + i + 1
				for {
					if time.Now().After(deadline) {
						t.Error("the elements did not all arrive before the deadline")
						return
					}
					invoked := tick()
					status := producers[p].push(value)
					returned := tick()
					if status == PushOK {
						logs[p] = append(logs[p], historyOperation{"push", valueOf(value), "pushed", invoked, returned})
						break
					}
					// An allocator with no limit gives every segment: a refusal
					// would be the queue's fault.
					t.Errorf("a push reported %d where the allocator never refuses", status)
					return
				}
			}
		}()
	}
	for c := range consumers {
		wg.Add(1)
		go func() {
			defer wg.Done()
			defer consumers[c].release()
			mine := len(producers) + c
			for delivered.Load() < total {
				if time.Now().After(deadline) {
					t.Error("the elements did not all arrive before the deadline")
					return
				}
				invoked := tick()
				value, ok := consumers[c].pop()
				returned := tick()
				if ok {
					logs[mine] = append(logs[mine], historyOperation{"pop", valueOf(value), "popped", invoked, returned})
					delivered.Add(1)
				} else {
					logs[mine] = append(logs[mine], historyOperation{"pop", nil, "empty", invoked, returned})
					runtime.Gosched()
				}
			}
		}()
	}
	wg.Wait()
	return historyDocument{Capacity: int(total), Refusal: refusalAtCapacity, Participants: logs}
}

func TestTheLinkedLamportRunsAreWrittenAsHistories(t *testing.T) {
	dir, ok := historyDir(t)
	if !ok {
		return
	}
	for run := 0; run < runsPerSegmentedShape; run++ {
		allocator := unlimitedAllocator()
		q, built := NewLinkedLamport[uint64](4, allocator, LockFree)
		if !built {
			t.Fatal("the allocator gives a segment")
		}
		producer, okProducer := q.Producer()
		consumer, okConsumer := q.Consumer()
		if !okProducer || !okConsumer {
			t.Fatal("a fresh queue hands out a handle a side")
		}
		document := recordSegmentedRun(t,
			[]producing{{producer.TryPush, producer.Release}},
			[]consuming{{consumer.TryPop, consumer.Release}},
			600)
		if allocator.live() != 1 {
			t.Fatalf("the queue gave back every segment but its newest: %d still out", allocator.live())
		}
		writeHistory(t, dir, fmt.Sprintf("go_segmented_linked_lamport_n4_p1_c1_%d", run), document)
	}
}
