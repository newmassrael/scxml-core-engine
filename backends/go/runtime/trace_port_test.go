// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//go:build sce_trace_port

package sce

// The trace port's writer and recorder (trace_port.go; docs/adr/0115, D4 and
// D8), held to the cases of the Rust runtime's trace_port module and of the C11
// lane's c11_unit_trace_port. Compiled only with the build tag sce_trace_port:
//
//	go test -tags sce_trace_port -run 'Trace' .

import (
	"reflect"
	"testing"
)

func traceStepOf(kind TraceKind) TraceStep {
	return TraceStep{
		Kind:          kind,
		Event:         "go.on",
		Fired:         []TraceFired{{Source: "s0", Index: 2}},
		Exited:        []string{"s01", "s0"},
		Entered:       []string{"s1"},
		Configuration: []string{"s1"},
		Outcome:       "pass",
	}
}

func TestTraceEachKindWritesTheFieldsTheFormatAsksOfIt(t *testing.T) {
	cases := []struct {
		step TraceStep
		want string
	}{
		{traceStepOf(TraceStart),
			`{"kind": "start", "entered": ["s1"], "configuration": ["s1"]}`},
		{traceStepOf(TraceEventless),
			`{"kind": "eventless", "fired": [{"source": "s0", "index": 2}], "exited": ["s01", "s0"], "entered": ["s1"], "configuration": ["s1"]}`},
		{traceStepOf(TraceInternal),
			`{"kind": "internal", "event": "go.on", "fired": [{"source": "s0", "index": 2}], "exited": ["s01", "s0"], "entered": ["s1"], "configuration": ["s1"]}`},
		{traceStepOf(TraceExternal),
			`{"kind": "external", "event": "go.on", "fired": [{"source": "s0", "index": 2}], "exited": ["s01", "s0"], "entered": ["s1"], "configuration": ["s1"]}`},
		{traceStepOf(TraceStop),
			`{"kind": "stop", "exited": ["s01", "s0"], "outcome": "pass"}`},
	}
	for _, c := range cases {
		if got := c.step.JSON(); got != c.want {
			t.Errorf("a %s step is written\n%s\nwant\n%s", c.step.Kind, got, c.want)
		}
	}
	unended := traceStepOf(TraceStop)
	unended.Outcome = ""
	want := `{"kind": "stop", "exited": ["s01", "s0"], "outcome": null}`
	if got := unended.JSON(); got != want {
		t.Errorf("a stop that ends in no top-level final is written\n%s\nwant\n%s", got, want)
	}
}

func TestTraceARunIsWrittenWithItsVersionAndLaneOneStepToALine(t *testing.T) {
	var trace RunTrace
	want := "{\n  \"version\": 1,\n  \"lane\": \"go\",\n  \"steps\": []\n}\n"
	if got := trace.JSON(); got != want {
		t.Errorf("a run of no step is written\n%q\nwant\n%q", got, want)
	}
	trace.Push(traceStepOf(TraceStart))
	trace.Push(traceStepOf(TraceStop))
	want = "{\n  \"version\": 1,\n  \"lane\": \"go\",\n  \"steps\": [\n    " +
		`{"kind": "start", "entered": ["s1"], "configuration": ["s1"]},` + "\n    " +
		`{"kind": "stop", "exited": ["s01", "s0"], "outcome": "pass"}` + "\n  ]\n}\n"
	if got := trace.JSON(); got != want {
		t.Errorf("a run of two steps is written\n%q\nwant\n%q", got, want)
	}
	if got := len(trace.Steps()); got != 2 {
		t.Errorf("the run keeps %d steps, want 2", got)
	}
}

func TestTraceANameIsWrittenAsAJSONString(t *testing.T) {
	quoted := traceStepOf(TraceStart)
	quoted.Entered = []string{`a"b\c`}
	quoted.Configuration = nil
	want := `{"kind": "start", "entered": ["a\"b\\c"], "configuration": []}`
	if got := quoted.JSON(); got != want {
		t.Errorf("a name with a quote and a backslash is written\n%s\nwant\n%s", got, want)
	}
}

// A recorder tells its port each step once the step ends, with what was added
// between, and the configuration only of a step that leaves one.
func TestTraceARecorderTellsItsPortEachStepOnceItEnds(t *testing.T) {
	var told []TraceStep
	var recorder traceRecorder
	recorder.set(func(step TraceStep) { told = append(told, step) })

	recorder.begin(TraceInternal, "e")
	if !recorder.isOpen() {
		t.Fatal("a step begun with a port set is not open")
	}
	recorder.fired("s0", 1)
	recorder.exited("s0")
	recorder.entered("s1")
	if len(told) != 0 {
		t.Fatalf("the port was told %v before the step ended", told)
	}
	recorder.end([]string{"s1"}, "ignored")
	if recorder.isOpen() {
		t.Fatal("a step that ended is still open")
	}

	recorder.begin(TraceStop, "")
	recorder.exited("s1")
	recorder.end([]string{"ignored"}, "pass")

	want := []TraceStep{
		{
			Kind:          TraceInternal,
			Event:         "e",
			Fired:         []TraceFired{{Source: "s0", Index: 1}},
			Exited:        []string{"s0"},
			Entered:       []string{"s1"},
			Configuration: []string{"s1"},
		},
		{
			Kind:    TraceStop,
			Exited:  []string{"s1"},
			Outcome: "pass",
		},
	}
	if !reflect.DeepEqual(told, want) {
		t.Fatalf("the port was told\n%#v\nwant\n%#v", told, want)
	}
}

// With no port set nothing is opened, and what the engine adds is kept nowhere;
// a cleared port is told nothing more.
func TestTraceARecorderWithNoPortKeepsNothing(t *testing.T) {
	var recorder traceRecorder
	recorder.begin(TraceStart, "")
	if recorder.isOpen() {
		t.Fatal("a step was opened with no port set")
	}
	recorder.entered("s0")
	recorder.end([]string{"s0"}, "")

	told := 0
	recorder.set(func(TraceStep) { told++ })
	recorder.begin(TraceStart, "")
	recorder.set(nil)
	if recorder.isOpen() {
		t.Fatal("a cleared port left its step open")
	}
	recorder.end([]string{"s0"}, "")
	if told != 0 {
		t.Fatalf("a cleared port was told %d steps", told)
	}
}
