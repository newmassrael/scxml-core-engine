// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//go:build amd64 || arm64 || loong64 || mips64 || mips64le || ppc64 || ppc64le || riscv64 || s390x || wasm

package queue

// The SCQ queue's runs written as histories (SCE Protocol-Synthesis RFC
// §synth-5-P, verification layer 2), under the architectures the queue exists
// on. See history_test.go for what is recorded and how.

import (
	"fmt"
	"runtime"
	"sync"
	"sync/atomic"
	"testing"
	"time"
)

// recordScqRun records `producers` producers and `consumers` consumers on real
// goroutines through the SCQ queue, each producer pushing perProducer distinct
// values.
func recordScqRun(t *testing.T, capacity, ringSlots, producers, consumers int, perProducer uint64) historyDocument {
	t.Helper()
	q := NewScq[uint64](capacity, ringSlots)
	var clock atomic.Uint64
	tick := func() uint64 { return clock.Add(1) }
	deadline := time.Now().Add(120 * time.Second)
	total := uint64(producers) * perProducer
	var delivered atomic.Uint64
	logs := make([][]historyOperation, producers+consumers)

	var wg sync.WaitGroup
	for p := 0; p < producers; p++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			producer, _ := q.Producer()
			defer producer.Release()
			for i := uint64(0); i < perProducer; i++ {
				value := uint64(p)*perProducer + i + 1
				for {
					if time.Now().After(deadline) {
						t.Error("the elements did not all arrive before the deadline")
						return
					}
					invoked := tick()
					status := producer.TryPush(value)
					returned := tick()
					if status == PushOK {
						logs[p] = append(logs[p], historyOperation{"push", valueOf(value), "pushed", invoked, returned})
						break
					}
					logs[p] = append(logs[p], historyOperation{"push", valueOf(value), "full", invoked, returned})
					runtime.Gosched()
				}
			}
		}()
	}
	for c := 0; c < consumers; c++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			consumer, _ := q.Consumer()
			defer consumer.Release()
			mine := producers + c
			for delivered.Load() < total {
				if time.Now().After(deadline) {
					t.Error("the elements did not all arrive before the deadline")
					return
				}
				invoked := tick()
				value, ok := consumer.TryPop()
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
	return historyDocument{Capacity: capacity, Refusal: refusalWhileSlotsAreHeld, Participants: logs}
}

func TestTheScqRunsAreWrittenAsHistories(t *testing.T) {
	dir, ok := historyDir(t)
	if !ok {
		return
	}
	// The shapes are the Rust arm's. A ring is at least as large as the number
	// of participants working it, so the small capacities ride on rings sized
	// for their threads.
	shapes := []struct {
		capacity, ringSlots, producers, consumers int
		perProducer                               uint64
	}{
		{1, 2, 2, 2, 150},
		{3, 4, 2, 2, 150},
		{8, 8, 2, 2, 150},
		{2, 4, 3, 1, 120},
		{4, 4, 1, 3, 120},
		{5, 8, 2, 2, 150},
	}
	for _, s := range shapes {
		for run := 0; run < runsPerScqShape; run++ {
			document := recordScqRun(t, s.capacity, s.ringSlots, s.producers, s.consumers, s.perProducer)
			name := fmt.Sprintf("go_scq_n%d_r%d_p%d_c%d_%d", s.capacity, s.ringSlots, s.producers, s.consumers, run)
			writeHistory(t, dir, name, document)
		}
	}
}
