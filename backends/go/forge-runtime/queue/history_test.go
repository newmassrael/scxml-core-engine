// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package queue

// The Go arm's stress runs of the `queue` kind, written as histories (SCE
// Protocol-Synthesis RFC §synth-5-P, verification layer 2).
//
// These tests run the queues of this package on real goroutines, record every
// attempt each goroutine made, and write one history per run in the JSON form
// every backend writes (tests/forge/conformance/queue_history.schema.json).
// They judge nothing about linearizability: `sce-codegen check-queue-history`
// does, for this backend as for the others, so no backend carries a checker of
// its own. The Go gate (scripts/gates/forge-go.sh) sets SCE_QUEUE_HISTORY_DIR,
// runs these, then runs the command over what they wrote. Without the
// variable there is nowhere to write, and the tests are skipped by name.
//
// The recording follows the Rust arm's: one counter that every goroutine reads
// immediately before and immediately after each call, so a read-modify-write
// chain on one atomic orders the readings with the calls between them and
// "returned before invoked" in the record means it in the run. Every attempt is
// recorded, the refused pushes and the empty pops too, because those are
// results the checker must account for.

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"sync"
	"sync/atomic"
	"testing"
	"time"
)

// historyOperation is one observed operation in the JSON form. A value is
// named by a push and by a pop that popped, and by no pop that found the queue
// empty.
type historyOperation struct {
	Call     string  `json:"call"`
	Value    *uint64 `json:"value,omitempty"`
	Outcome  string  `json:"outcome"`
	Invoked  uint64  `json:"invoked"`
	Returned uint64  `json:"returned"`
}

type historyDocument struct {
	Version      int                  `json:"version"`
	Capacity     int                  `json:"capacity"`
	Refusal      string               `json:"refusal"`
	Participants [][]historyOperation `json:"participants"`
}

const (
	refusalAtCapacity        = "at-capacity"
	refusalWhileSlotsAreHeld = "while-slots-are-held"

	// runsPerSpscCapacity runs of each Lamport capacity are recorded. A
	// Lamport run pushes thousands of values and its history is megabytes, and
	// the ring has no schedule a second run reaches that the first does not.
	runsPerSpscCapacity = 3
	// valuesPerSpscRun values go through each Lamport run.
	valuesPerSpscRun = 2000
	// runsPerScqShape runs of each SCQ shape are recorded: short runs, many of
	// them, because the schedules that matter are a small fraction of those a
	// machine gives.
	runsPerScqShape = 25
)

// historyDir is where the histories are written, or false when nowhere is
// named.
func historyDir(t *testing.T) (string, bool) {
	t.Helper()
	dir := os.Getenv("SCE_QUEUE_HISTORY_DIR")
	if dir == "" {
		t.Skip("SCE_QUEUE_HISTORY_DIR names where the histories are written; it is unset")
		return "", false
	}
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatalf("cannot create %s: %v", dir, err)
	}
	return dir, true
}

func writeHistory(t *testing.T, dir, name string, document historyDocument) {
	t.Helper()
	document.Version = 1
	// A participant that made no attempt (a consumer that started after the
	// others had taken every element) is an empty list in the history, not
	// `null`: Go marshals a nil slice as null, which the format refuses.
	for i := range document.Participants {
		if document.Participants[i] == nil {
			document.Participants[i] = []historyOperation{}
		}
	}
	text, err := json.Marshal(document)
	if err != nil {
		t.Fatalf("%s: %v", name, err)
	}
	path := filepath.Join(dir, name+".json")
	if err := os.WriteFile(path, append(text, '\n'), 0o644); err != nil {
		t.Fatalf("cannot write %s: %v", path, err)
	}
}

func valueOf(v uint64) *uint64 { return &v }

// recordSpscRun records one producer and one consumer on two goroutines
// through the Lamport ring.
func recordSpscRun(t *testing.T, capacity int) historyDocument {
	t.Helper()
	q := NewSpsc[uint64](capacity)
	producer, okProducer := q.Producer()
	consumer, okConsumer := q.Consumer()
	if !okProducer || !okConsumer {
		t.Fatal("a fresh queue hands out a handle a side")
	}
	var clock atomic.Uint64
	tick := func() uint64 { return clock.Add(1) }
	deadline := time.Now().Add(120 * time.Second)

	var pushes, pops []historyOperation
	var wg sync.WaitGroup
	wg.Add(2)
	go func() {
		defer wg.Done()
		for value := uint64(1); value <= valuesPerSpscRun; value++ {
			for {
				if time.Now().After(deadline) {
					t.Error("the elements did not all arrive before the deadline")
					return
				}
				invoked := tick()
				status := producer.TryPush(value)
				returned := tick()
				if status == PushOK {
					pushes = append(pushes, historyOperation{"push", valueOf(value), "pushed", invoked, returned})
					break
				}
				pushes = append(pushes, historyOperation{"push", valueOf(value), "full", invoked, returned})
				runtime.Gosched()
			}
		}
	}()
	go func() {
		defer wg.Done()
		delivered := 0
		for delivered < valuesPerSpscRun {
			if time.Now().After(deadline) {
				t.Error("the elements did not all arrive before the deadline")
				return
			}
			invoked := tick()
			value, ok := consumer.TryPop()
			returned := tick()
			if ok {
				pops = append(pops, historyOperation{"pop", valueOf(value), "popped", invoked, returned})
				delivered++
			} else {
				pops = append(pops, historyOperation{"pop", nil, "empty", invoked, returned})
				runtime.Gosched()
			}
		}
	}()
	wg.Wait()
	return historyDocument{Capacity: capacity, Refusal: refusalAtCapacity, Participants: [][]historyOperation{pushes, pops}}
}

// A consumer that started after the others had taken every element made no
// attempt, and its log is a nil slice, which encoding/json writes as null. The
// history format refuses null, so the writer must turn it into the empty list.
// Run-dependent in the stress tests (one history in a hundred and sixty-two did
// it), so it is pinned here without a run.
func TestAParticipantThatMadeNoAttemptIsWrittenAsAnEmptyListNotNull(t *testing.T) {
	dir := t.TempDir()
	document := historyDocument{
		Capacity: 2,
		Refusal:  refusalWhileSlotsAreHeld,
		Participants: [][]historyOperation{
			{{"push", valueOf(1), "pushed", 1, 2}},
			nil,
			{{"pop", valueOf(1), "popped", 3, 4}},
		},
	}
	writeHistory(t, dir, "no_attempt", document)
	text, err := os.ReadFile(filepath.Join(dir, "no_attempt.json"))
	if err != nil {
		t.Fatal(err)
	}
	if strings.Contains(string(text), "null") {
		t.Fatalf("a participant with no attempt was written as null: %s", text)
	}
	if !strings.Contains(string(text), `"participants":[[{`) || !strings.Contains(string(text), `}],[],[{`) {
		t.Fatalf("the middle participant is not the empty list: %s", text)
	}
}

func TestTheLamportRingRunsAreWrittenAsHistories(t *testing.T) {
	dir, ok := historyDir(t)
	if !ok {
		return
	}
	for _, capacity := range []int{1, 2, 3, 8} {
		for run := 0; run < runsPerSpscCapacity; run++ {
			document := recordSpscRun(t, capacity)
			writeHistory(t, dir, fmt.Sprintf("go_spsc_n%d_%d", capacity, run), document)
		}
	}
}
