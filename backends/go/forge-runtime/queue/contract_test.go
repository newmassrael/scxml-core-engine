// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package queue

// The Go arm of the `queue` kind's layer 1 (SCE Protocol-Synthesis RFC
// §synth-5-P): every scenario in tests/forge/conformance/queue_contract.json,
// the same ones the Rust and C++ arms run, against the runtime queue its
// storage row names. A row this arm has no runtime for is refused by name,
// never skipped silently.

import (
	"encoding/json"
	"os"
	"strconv"
	"testing"
)

// contractPath is the contract relative to this package's directory.
const contractPath = "../../../../tests/forge/conformance/queue_contract.json"

// tracked is the element a scenario hands over.
type tracked struct{ value uint64 }

// subject is what a scenario asks of a queue, so one reading of the steps
// serves every runtime this arm has. Each runtime reaches its producer and
// consumer its own way; the scenarios do not know how.
type subject interface {
	capacity() int
	push(element tracked) PushStatus
	pop() (tracked, bool)
}

type spscSubject struct{ q *Spsc[tracked] }

func (s spscSubject) capacity() int { return s.q.Capacity() }

func (s spscSubject) push(element tracked) PushStatus {
	producer, ok := s.q.Producer()
	if !ok {
		panic("a scenario holds one handle at a time")
	}
	defer producer.Release()
	return producer.TryPush(element)
}

func (s spscSubject) pop() (tracked, bool) {
	consumer, ok := s.q.Consumer()
	if !ok {
		panic("a scenario holds one handle at a time")
	}
	defer consumer.Release()
	return consumer.TryPop()
}

// newScqSubject builds the SCQ subject for a capacity. It is nil on a GOARCH
// that has no SCQ row (scq_test.go sets it under the row's build constraint),
// and a scenario that needs it is then refused by name.
var newScqSubject func(capacity int) subject

// ceilPow2 is the ring size of an SCQ row for a capacity: the capacity rounded
// up to a power of two.
func ceilPow2(n int) int {
	size := 1
	for size < n {
		size *= 2
	}
	return size
}

type contractStep struct {
	Op     string          `json:"op"`
	Value  *uint64         `json:"value"`
	Expect json.RawMessage `json:"expect"`
}

type contractScenario struct {
	ID        string         `json:"id"`
	Storage   string         `json:"storage"`
	Producers string         `json:"producers"`
	Consumers string         `json:"consumers"`
	Capacity  int            `json:"capacity"`
	Steps     []contractStep `json:"steps"`
}

type contractFile struct {
	Version   int                `json:"version"`
	Scenarios []contractScenario `json:"scenarios"`
}

func runScenario(t *testing.T, s subject, scenario contractScenario) {
	t.Helper()
	for index, step := range scenario.Steps {
		context := scenario.ID + " step " + strconv.Itoa(index)
		switch step.Op {
		case "capacity":
			var want int
			if err := json.Unmarshal(step.Expect, &want); err != nil {
				t.Fatalf("%s: capacity expects a number: %v", context, err)
			}
			if got := s.capacity(); got != want {
				t.Fatalf("%s: capacity %d, want %d", context, got, want)
			}
		case "push":
			if step.Value == nil {
				t.Fatalf("%s: a push names its value", context)
			}
			var want string
			if err := json.Unmarshal(step.Expect, &want); err != nil {
				t.Fatalf("%s: a push expects \"ok\" or \"full\": %v", context, err)
			}
			got := s.push(tracked{value: *step.Value})
			switch {
			case got == PushOK && want == "ok":
			case got == PushFull && want == "full":
			default:
				t.Fatalf("%s: push gave %d, want %s", context, got, want)
			}
		case "pop":
			element, ok := s.pop()
			var wantEmpty string
			if json.Unmarshal(step.Expect, &wantEmpty) == nil {
				if wantEmpty != "empty" || ok {
					t.Fatalf("%s: expected the queue to be empty, popped %v", context, element)
				}
				continue
			}
			var want uint64
			if err := json.Unmarshal(step.Expect, &want); err != nil {
				t.Fatalf("%s: a pop expects a number or \"empty\": %v", context, err)
			}
			if !ok || element.value != want {
				t.Fatalf("%s: popped (%v, %v), want %d", context, element.value, ok, want)
			}
		case "destroy":
			// Destruction of held elements is a step for languages with
			// destructors. A Go queue owns no element the collector does not
			// also own; what Go owes instead is that a popped slot is cleared,
			// which TestALamportRingClearsAPoppedSlot and
			// TestAnScqRingClearsAPoppedSlot check directly.
			t.Logf("%s: \"destroy\" is a destructor step; Go has none", context)
			return
		default:
			t.Fatalf("%s: unknown op %q", context, step.Op)
		}
	}
}

func TestEveryContractScenarioHolds(t *testing.T) {
	raw, err := os.ReadFile(contractPath)
	if err != nil {
		t.Fatalf("cannot read the contract: %v", err)
	}
	var contract contractFile
	if err := json.Unmarshal(raw, &contract); err != nil {
		t.Fatalf("the contract is not JSON: %v", err)
	}
	if contract.Version != 1 {
		t.Fatalf("this arm reads version 1 of the contract, got %d", contract.Version)
	}
	if len(contract.Scenarios) == 0 {
		t.Fatal("a contract with no scenarios checks nothing")
	}

	for _, scenario := range contract.Scenarios {
		scenario := scenario
		t.Run(scenario.ID, func(t *testing.T) {
			switch {
			case scenario.Storage == "bounded" && scenario.Producers == "one" && scenario.Consumers == "one":
				runScenario(t, spscSubject{NewSpsc[tracked](scenario.Capacity)}, scenario)
			case scenario.Storage == "bounded":
				// Any other cardinality selects the SCQ row (the RFC's
				// selection table), so the three combinations share one
				// runtime.
				if newScqSubject == nil {
					t.Skipf("the SCQ row is absent on this GOARCH (scq.go's build constraint); refused by name: %s", scenario.ID)
				}
				runScenario(t, newScqSubject(scenario.Capacity), scenario)
			case scenario.Storage == "intrusive":
				// A collected language has no element to link in place, and the
				// allocation an intrusive list exists to avoid is the collector's.
				// The generator refuses the row for Go by name
				// (queue/storage-runtime-missing), so there is no runtime to run.
				t.Skipf("the intrusive row is not lowered to Go, and the generator refuses it by name: %s", scenario.ID)
			default:
				t.Fatalf("the Go arm has no runtime for the row %s/%s/%s", scenario.Storage, scenario.Producers, scenario.Consumers)
			}
		})
	}
}
