// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// datamodel="sce-static" (docs/SCE_ACCEPTED_SUBSET.md §2.15) under generated Go:
// a variable is a field of the policy, every expression was lowered to Go at
// build time, and no script engine is built.
//
// The scenarios here are the ones Kotlin, Rust, C++ and the Interpreter replay —
// `sce-build/tests/fixtures/static_datamodel/scenarios/<machine>.json`, whose
// expected values are derived from the document, not observed from a backend —
// so the engines are held to one answer. A scenario is a list of steps, each an
// external event with its payload and what the machine must hold after it runs
// to quiescence: its current state and any of its published variables.
//
// The machines are the ones scripts/regen_static_datamodel_go.sh commits, one
// package each. Those with no scenario here are imported for their effect: a
// machine nobody built would be a machine nobody type-checked.

package static_datamodel

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"

	_ "github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_history"
	_ "github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_timers"

	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_block_ends"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_counter"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_host_call"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_host_call_arguments"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/static_overflow"
)

// scenarioDir is where the scenarios live, from this package's directory.
const scenarioDir = "../../../../../sce-build/tests/fixtures/static_datamodel/scenarios"

// machine is one generated machine as a scenario sees it: events by their
// document name, and the published variables by theirs.
type machine struct {
	send      func(name, data string)
	state     func() string
	ended     func() bool
	variables map[string]func() any
}

// drive starts `policy` and returns it as a machine.
func drive[S interface {
	comparable
	String() string
}, E comparable](policy sce.StatePolicy[S, E], variables map[string]func() any) machine {
	engine := sce.NewEngine[S, E](policy)
	engine.Initialize()
	return machine{
		send: func(name, data string) {
			engine.RaiseExternalByName(name, data)
			engine.Step()
		},
		state: func() string {
			active := engine.GetActiveStates()
			if len(active) != 1 {
				return fmt.Sprintf("<%d active states>", len(active))
			}
			return active[0].String()
		},
		ended:     engine.IsInFinalState,
		variables: variables,
	}
}

// asJSON is `value` as the scenario writes it: a JSON value, so a `uint32` and
// the number a scenario states compare equal whatever their Go types.
func asJSON(t *testing.T, value any) any {
	t.Helper()
	text, err := json.Marshal(value)
	if err != nil {
		t.Fatalf("a variable is not JSON: %v", err)
	}
	var out any
	if err := json.Unmarshal(text, &out); err != nil {
		t.Fatalf("a variable's JSON does not read back: %v", err)
	}
	return out
}

// replay runs scenario `name` against `m`. Every step names an event (or none,
// for the machine as started) and what it must hold afterwards.
func replay(t *testing.T, name string, m machine) {
	t.Helper()
	text, err := os.ReadFile(filepath.Join(scenarioDir, name+".json"))
	if err != nil {
		t.Fatalf("scenario %s is not readable: %v", name, err)
	}
	var scenario struct {
		Steps []struct {
			Note   string          `json:"note"`
			Event  *string         `json:"event"`
			Data   json.RawMessage `json:"data"`
			Expect struct {
				State     *string        `json:"state"`
				Ended     bool           `json:"ended"`
				Variables map[string]any `json:"variables"`
			} `json:"expect"`
		} `json:"steps"`
	}
	if err := json.Unmarshal(text, &scenario); err != nil {
		t.Fatalf("scenario %s is not a scenario: %v", name, err)
	}
	if len(scenario.Steps) == 0 {
		t.Fatalf("scenario %s has no steps, which judges nothing", name)
	}
	for n, step := range scenario.Steps {
		if step.Event != nil {
			data := ""
			if len(step.Data) > 0 {
				data = string(step.Data)
			}
			m.send(*step.Event, data)
		}
		where := func() string { return fmt.Sprintf("%s step %d: %s", name, n, step.Note) }
		if step.Expect.Ended {
			if !m.ended() {
				t.Errorf("%s: the machine ended in a top-level <final>", where())
			}
			continue
		}
		if step.Expect.State != nil {
			if got := m.state(); got != *step.Expect.State {
				t.Errorf("%s: the current state is %q, not %q", where(), got, *step.Expect.State)
			}
		}
		for variable, want := range step.Expect.Variables {
			read, ok := m.variables[variable]
			if !ok {
				t.Errorf("%s: the driver publishes no variable %q", where(), variable)
				continue
			}
			if got := asJSON(t, read()); !reflect.DeepEqual(got, want) {
				t.Errorf("%s: variable %q is %v, not %v", where(), variable, got, want)
			}
		}
	}
}

func counter() machine {
	policy := static_counter.NewStaticCounterPolicy()
	policy.SessionID = sce.GenerateSessionID()
	return drive[static_counter.StaticCounterState, static_counter.StaticCounterEvent](&policy, map[string]func() any{
		"count": func() any { return policy.Count() },
		"ready": func() any { return policy.Ready() },
	})
}

// A counter counts to its flag and lets go: guards, `<assign>`, `<if>` and
// `<elseif>` are native, and `In()` asks the machine's own configuration.
func TestTheCounterCountsToItsFlagAndLetsGo(t *testing.T) {
	replay(t, "static_counter", counter())
}

func TestTheCounterStopsAtItsBoundAndRefusesGo(t *testing.T) {
	replay(t, "static_counter_bound", counter())
}

// A checked integer operation that overflows is a failure, not a wrapped value
// (§scxml-3.4.1): the variable keeps what it held, `error.execution` is raised,
// and a guard over the overflowing sum is false.
func TestAnOverflowingOperationFailsInsteadOfWrapping(t *testing.T) {
	policy := static_overflow.NewStaticOverflowPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_overflow", drive[static_overflow.StaticOverflowState, static_overflow.StaticOverflowEvent](&policy, map[string]func() any{
		"level":    func() any { return policy.Level() },
		"refusals": func() any { return policy.Refusals() },
	}))
}

// An error ends the block it stands in (W3C SCXML 4.9): the statements after a
// failed `<assign>`, after a failed `<if>` cond, or inside a branch that failed
// do not run, while the next block does.
func TestAnErrorEndsTheBlockItStandsIn(t *testing.T) {
	policy := static_block_ends.NewStaticBlockEndsPolicy()
	policy.SessionID = sce.GenerateSessionID()
	replay(t, "static_block_ends", drive[static_block_ends.StaticBlockEndsState, static_block_ends.StaticBlockEndsEvent](&policy, map[string]func() any{
		"a":           func() any { return policy.A() },
		"b":           func() any { return policy.B() },
		"afterAssign": func() any { return policy.AfterAssign() },
		"thenRan":     func() any { return policy.ThenRan() },
		"elseRan":     func() any { return policy.ElseRan() },
		"afterIf":     func() any { return policy.AfterIf() },
		"inBranch":    func() any { return policy.InBranch() },
		"afterBranch": func() any { return policy.AfterBranch() },
		"afterOk":     func() any { return policy.AfterOk() },
		"errors":      func() any { return policy.Errors() },
	}))
}

// recordingHost keeps what a machine asked of its host, as `(value…)` text.
type recordingHost struct{ calls []string }

func (h *recordingHost) ShowAttempts(count uint32, exhausted bool) {
	h.calls = append(h.calls, fmt.Sprintf("%d,%t", count, exhausted))
}

func (h *recordingHost) Report(next uint8) {
	h.calls = append(h.calls, fmt.Sprintf("%d", next))
}

// A host action takes the machine's variables as typed arguments, each read when
// the call is made: one call per entry of `idle`, with the datamodel as it
// stood.
func TestAHostActionTakesTypedDatamodelArguments(t *testing.T) {
	host := &recordingHost{}
	policy := static_host_call.NewStaticHostCallPolicy(host)
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[static_host_call.StaticHostCallState, static_host_call.StaticHostCallEvent](&policy)
	engine.Initialize()
	for i := 0; i < 4; i++ {
		engine.RaiseExternalByName("retry", "")
		engine.Step()
	}
	// The fourth retry finds `attempts < 3` false and re-enters nothing.
	want := []string{"0,false", "1,false", "2,false", "3,true"}
	if !reflect.DeepEqual(host.calls, want) {
		t.Errorf("the host heard %v, want %v", host.calls, want)
	}
}

// An argument that cannot be computed is a failure, not a wrapped value: the
// host is not called, and `error.execution` is raised in the call's place.
func TestAnArgumentThatOverflowsStopsTheCallAndRaisesAnError(t *testing.T) {
	host := &recordingHost{}
	policy := static_host_call_arguments.NewStaticHostCallArgumentsPolicy(host)
	policy.SessionID = sce.GenerateSessionID()
	engine := sce.NewEngine[static_host_call_arguments.StaticHostCallArgumentsState, static_host_call_arguments.StaticHostCallArgumentsEvent](&policy)
	engine.Initialize()

	engine.RaiseExternalByName("fine", "")
	engine.Step()
	if want := []string{"251"}; !reflect.DeepEqual(host.calls, want) {
		t.Errorf("250 + 1 fits a uint8: the host heard %v, want %v", host.calls, want)
	}
	if got := policy.Errors(); got != 0 {
		t.Errorf("errors = %d, want 0", got)
	}

	engine.RaiseExternalByName("overflow", "")
	engine.Step()
	if want := []string{"251"}; !reflect.DeepEqual(host.calls, want) {
		t.Errorf("250 + 10 does not fit, so the host is not called with 4: it heard %v, want %v", host.calls, want)
	}
	if got := policy.Errors(); got != 1 {
		t.Errorf("error.execution was raised and the machine saw it: errors = %d, want 1", got)
	}
}
