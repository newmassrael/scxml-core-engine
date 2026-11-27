// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//go:build sce_trace_port

// Package run_trace records runs of the Go lane as the traces
// `sce-codegen formal traces` judges behaviours of the document's model
// (docs/adr/0115, D4 and D8). The twin of the Rust lane's
// a_run_is_a_behaviour_of_its_model (backends/rust/tests) and of the C11
// lane's c11_a_run_of_test_*_is_the_recorded_run_of_its_model programs.
//
// Each test runs one W3C document that docs/formal has a model of (W3CTest388,
// W3CTest570, W3CTest403c over AppendixD.tla), the machine `generate-w3c -l go`
// writes under generated/ for the conformance suite, with the engine's trace
// port set (sce.Engine.SetTracePort), and holds what the port was told to the
// recorded run docs/formal/traces.json declares of that model, byte for byte.
// The judge reads the recorded file, so a run that moves is seen here first,
// and a recorded run that no longer is the lane's is never judged in its place.
// A Go run of a document is judged against the same generated model as the
// Rust run of it, and these documents leave a run's steps no choice, so the two
// recorded runs differ only in their lane, or the judge refuses one.
//
// The trace port is compiled only with the build tag sce_trace_port, so this
// package is too, and it reads the W3C machines the suite generates: from
// backends/go/tests, after `sce-codegen generate-w3c -l go`,
//
//	go test -tags sce_trace_port ./run_trace/ -v -count=1
//
// which the gate w3c-go runs. Recording again, after a change that moves a run
// on purpose: the same with UPDATE_EXPECT=1, then
// `sce-codegen formal traces --manifest docs/formal/traces.json`, and read the
// diff of the recorded runs.
package run_trace

import (
	"os"
	"path/filepath"
	"reflect"
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	"github.com/newmassrael/sce-go-tests/generated/test388"
	"github.com/newmassrael/sce-go-tests/generated/test403c"
	"github.com/newmassrael/sce-go-tests/generated/test570"
	scegotest "github.com/newmassrael/sce-go-tests/harness"
	computed "github.com/newmassrael/sce-go-tests/integration/a_computed_event_name_is_matched_like_any_other"
)

// recordedPath is the recorded run of model's document by this lane, as the
// traces manifest names it, from this package's directory.
func recordedPath(model string) string {
	return filepath.Join("..", "..", "..", "..", "docs", "formal", "traces", model+".go.json")
}

// run is the trace port's account of one run of policy's machine to its end,
// and the top-level final state the run ended in.
func run[S comparable, E comparable](t *testing.T, policy sce.StatePolicy[S, E]) (*sce.RunTrace, S, bool) {
	t.Helper()
	trace := &sce.RunTrace{}
	engine := sce.NewEngine[S, E](policy)
	engine.SetTracePort(trace.Push)
	if err := engine.Initialize(); err != nil {
		t.Fatalf("the machine did not start: %v", err)
	}
	if !engine.RunUntilCompletion(3*time.Second, 10*time.Millisecond) {
		t.Fatal("the run reached no final state")
	}
	ended, ok := engine.TerminalState()
	return trace, ended, ok
}

// holdsToTheRecordedRun holds trace to the recorded run of model, or records it
// again when UPDATE_EXPECT is set.
func holdsToTheRecordedRun(t *testing.T, model string, trace *sce.RunTrace) {
	t.Helper()
	text := trace.JSON()
	path := recordedPath(model)
	if _, set := os.LookupEnv("UPDATE_EXPECT"); set {
		if err := os.WriteFile(path, []byte(text), 0o644); err != nil {
			t.Fatalf("%s is not writable: %v", path, err)
		}
		return
	}
	recorded, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("%s cannot be read (%v): record it with "+
			"`UPDATE_EXPECT=1 go test -tags sce_trace_port ./run_trace/` in backends/go/tests", path, err)
	}
	if string(recorded) != text {
		at := 0
		for at < len(recorded) && at < len(text) && recorded[at] == text[at] {
			at++
		}
		t.Fatalf("the run of %s is not the recorded one that `sce-codegen formal traces` judges: "+
			"they part at byte %d.\nThe run:\n%s", model, at, text)
	}
}

// startsAndStops holds what the first and the last step of a recorded run must
// say: the run starts, ends in its pass state, and the interpreter's exit leaves
// that state alone.
func startsAndStops(t *testing.T, trace *sce.RunTrace) {
	t.Helper()
	steps := trace.Steps()
	if len(steps) == 0 {
		t.Fatal("the run told no step")
	}
	if steps[0].Kind != sce.TraceStart {
		t.Errorf("the run begins with a %s step, not a start", steps[0].Kind)
	}
	last := steps[len(steps)-1]
	if last.Kind != sce.TraceStop {
		t.Errorf("the run ends with a %s step, not a stop", last.Kind)
	}
	if last.Outcome != "pass" {
		t.Errorf("the stop names the outcome %q, not pass", last.Outcome)
	}
	if !reflect.DeepEqual(last.Exited, []string{"pass"}) {
		t.Errorf("the stop exits %v, not pass alone", last.Exited)
	}
}

// A run of W3C test388, whose document re-enters a state through a deep and a
// shallow history, is the recorded run of docs/formal/W3CTest388.tla.
func TestARunOfTest388IsTheRecordedRunOfItsModel(t *testing.T) {
	policy := test388.NewTest388Policy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	trace, ended, ok := run[test388.Test388State, test388.Test388Event](t, &policy)
	scegotest.AssertEndedIn(t, ended, ok, test388.Test388StatePass, "388")
	startsAndStops(t, trace)
	holdsToTheRecordedRun(t, "W3CTest388", trace)
}

// A run of W3C test570, whose document completes a <parallel> when its last
// region does and raises the done.state events in order, is the recorded run of
// docs/formal/W3CTest570.tla.
func TestARunOfTest570IsTheRecordedRunOfItsModel(t *testing.T) {
	policy := test570.NewTest570Policy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	trace, ended, ok := run[test570.Test570State, test570.Test570Event](t, &policy)
	scegotest.AssertEndedIn(t, ended, ok, test570.Test570StatePass, "570")
	startsAndStops(t, trace)
	holdsToTheRecordedRun(t, "W3CTest570", trace)
}

// A run of W3C test403c, whose document takes the optimal enabled transition
// set of a <parallel>, one region preempting another, is the recorded run of
// docs/formal/W3CTest403c.tla.
func TestARunOfTest403cIsTheRecordedRunOfItsModel(t *testing.T) {
	policy := test403c.NewTest403cPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	trace, ended, ok := run[test403c.Test403cState, test403c.Test403cEvent](t, &policy)
	scegotest.AssertEndedIn(t, ended, ok, test403c.Test403cStatePass, "403c")
	startsAndStops(t, trace)
	holdsToTheRecordedRun(t, "W3CTest403c", trace)
}

// tracedEvent is an event step as the run trace names it: its queue and the
// name of the event it took.
type tracedEvent struct {
	kind  sce.TraceKind
	event string
}

// An event step names its event by the name `_event.name` holds while it is
// taken, which the run trace format defines the field as: the whole name it was
// sent under, and not the event of the document a door resolved it to
// (`request`, or the wildcard). The document sends six events whose names it
// computes, three to its external queue and three to its internal one, as the
// Rust lane's an_event_step_names_the_event_by_the_name_it_was_sent_under and
// the C11 lane's c11_integration_trace_port hold of the same document.
func TestAnEventStepNamesTheEventByTheNameItWasSentUnder(t *testing.T) {
	policy := computed.NewAComputedEventNameIsMatchedLikeAnyOtherPolicy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	trace, ended, ok := run[computed.AComputedEventNameIsMatchedLikeAnyOtherState,
		computed.AComputedEventNameIsMatchedLikeAnyOtherEvent](t, &policy)
	scegotest.AssertEndedIn(t, ended, ok, computed.AComputedEventNameIsMatchedLikeAnyOtherStatePass,
		"a_computed_event_name_is_matched_like_any_other")
	startsAndStops(t, trace)

	var events []tracedEvent
	for _, step := range trace.Steps() {
		if step.Kind == sce.TraceInternal || step.Kind == sce.TraceExternal {
			events = append(events, tracedEvent{kind: step.Kind, event: step.Event})
		}
	}
	want := []tracedEvent{
		{sce.TraceExternal, "request.new"},
		{sce.TraceExternal, "other.thing"},
		{sce.TraceInternal, "request.again"},
		{sce.TraceInternal, "last.one"},
		{sce.TraceInternal, "request.third"},
		{sce.TraceExternal, "last.two"},
	}
	if !reflect.DeepEqual(events, want) {
		t.Fatalf("the event steps name %v, want %v", events, want)
	}
}

// A port set and then cleared is told nothing, and the run is the one an engine
// that never had one makes.
func TestAClearedPortIsToldNothingAndTheRunIsTheSame(t *testing.T) {
	policy := test388.NewTest388Policy()
	policy.SessionID = sce.GenerateSessionID()
	policy.ScriptEngine = scegotest.NewLuaEngine()
	engine := sce.NewEngine[test388.Test388State, test388.Test388Event](&policy)
	told := 0
	engine.SetTracePort(func(sce.TraceStep) { told++ })
	engine.SetTracePort(nil)
	if err := engine.Initialize(); err != nil {
		t.Fatalf("the machine did not start: %v", err)
	}
	if !engine.RunUntilCompletion(3*time.Second, 10*time.Millisecond) {
		t.Fatal("the run reached no final state")
	}
	ended, ok := engine.TerminalState()
	scegotest.AssertEndedIn(t, ended, ok, test388.Test388StatePass, "388")
	if told != 0 {
		t.Fatalf("a cleared port was told %d steps", told)
	}
}
