// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//go:build sce_trace_port

package sce

// The trace port: what a run tells a test of each step Appendix D takes, so the
// run can be judged a behaviour of the document's model (docs/adr/0115, D4 and
// D8).
//
// A step is the start of the interpreter, a microstep on eventless transitions,
// the taking of an internal or an external event (with the microstep it
// selects, or none), or the exit of the interpreter: the steps
// docs/formal/AppendixD.tla names Start, Eventless, Internal, External and
// Stop. Each is told once it is over, with what its record told holds there:
// the transitions it took by their rows, the states it exited in the order it
// exited them, the states it entered in the order it entered them, and the
// configuration it left, or, for a stop, the top-level <final> the run ended
// in. The macrosteps are the runs of steps from a start or an external event to
// the next one.
//
// What the port is told is written in the one run trace format every lane
// writes (RunTrace.JSON, tests/forge/conformance/run_trace.schema.json, read by
// sce-build's run_trace), and `sce-codegen formal traces` judges the file
// against the model it names.
//
// The whole of it is the build tag sce_trace_port, which no build asks for
// unless a test names it (`go test -tags sce_trace_port`): a build without it
// compiles trace_port_off.go in this file's place, whose recorder is a field of
// no size and whose calls do nothing and are removed by the compiler, so such
// an engine keeps nothing of the port and spends nothing on it. With the tag
// and no port set, each step asks one nil check. The port reads and is never
// asked anything, so a machine with a port set takes the steps one without
// takes.
//
// An event from outside, and a delayed send that comes due, are steps of the
// environment and not of the interpreter (AppendixD's Arrive and Due): the
// engine cannot tell where an event on its external queue came from, and the
// external step that takes it names it.
//
// The cross-language siblings are the Rust runtime's trace_port module
// (feature trace-port) and the C11 runtime's sce/trace_port.h and
// sce/trace_json.h; this file is written to their shape, and RunTrace.JSON lays
// a run out byte for byte as theirs do.

import (
	"sort"
	"strconv"
	"strings"
)

// TraceFormatVersion is the version of the run trace format RunTrace.JSON
// writes, the TRACE_FORMAT_VERSION its reader in sce-build checks.
const TraceFormatVersion = 1

// TraceLane is the lane this runtime is, as a run trace names it.
const TraceLane = "go"

// TraceKind is which step of Appendix D a TraceStep is.
type TraceKind int

const (
	// TraceStart is AppendixD's Start: the entry into the initial
	// configuration.
	TraceStart TraceKind = iota
	// TraceEventless is AppendixD's Eventless: a microstep on eventless
	// transitions.
	TraceEventless
	// TraceInternal is AppendixD's Internal: an event taken off the internal
	// queue, and the microstep it selects, if it selects one.
	TraceInternal
	// TraceExternal is AppendixD's External: an event taken off the external
	// queue, and the microstep it selects, if it selects one.
	TraceExternal
	// TraceStop is AppendixD's Stop: every state still active exited.
	TraceStop
)

// String is the step's name in the run trace format.
func (k TraceKind) String() string {
	switch k {
	case TraceStart:
		return "start"
	case TraceEventless:
		return "eventless"
	case TraceInternal:
		return "internal"
	case TraceExternal:
		return "external"
	case TraceStop:
		return "stop"
	}
	return ""
}

// TraceFired is a transition a step took, by the pair its row of the emitted
// transition table is named by (docs/adr/0089): the state it is written in and
// its position among that state's own transitions in document order.
type TraceFired struct {
	// Source is the state the transition is written in, by its id.
	Source string
	// Index is its position among Source's own transitions, from 0.
	Index uint32
}

// TraceStep is one step a run took, as the port is told it.
//
// Each kind says what the run trace format asks of it and no more: a start what
// it entered and the configuration; a microstep, eventless or on an event, what
// it took, exited and entered and the configuration; an event step the event
// too; a stop what it exited and the top-level final state the run ended in.
// What a kind does not say is empty. No id and no event name is empty, so an
// empty Event or Outcome is the absence of one. The step and its lists are the
// port's to keep: the engine does not touch them again.
type TraceStep struct {
	// Kind is which step.
	Kind TraceKind
	// Event is the event an internal or external step took, by the name
	// `_event.name` holds while it is taken: the whole name it was raised or
	// sent under, which is not the name of the event of the document it
	// matched when a door resolved a longer name, or one the document never
	// writes, to it.
	Event string
	// Fired is the transitions the step's microstep took, in the order they
	// were selected.
	Fired []TraceFired
	// Exited is the states exited, in the order they were exited.
	Exited []string
	// Entered is the states entered, in the order they were entered.
	Entered []string
	// Configuration is the configuration the step left, in document order;
	// empty for a stop.
	Configuration []string
	// Outcome is the top-level <final> the run ended in, of a stop that ends
	// one; empty otherwise.
	Outcome string
}

// JSON is the step as one object of the run trace format: the fields its kind
// says, in the order the format lists them.
func (s TraceStep) JSON() string {
	var out strings.Builder
	s.writeJSON(&out)
	return out.String()
}

func (s TraceStep) writeJSON(out *strings.Builder) {
	out.WriteString(`{"kind": `)
	writeTraceText(out, s.Kind.String())
	switch s.Kind {
	case TraceStart:
		writeTraceNames(out, "entered", s.Entered)
		writeTraceNames(out, "configuration", s.Configuration)
	case TraceEventless, TraceInternal, TraceExternal:
		if s.Kind != TraceEventless {
			out.WriteString(`, "event": `)
			writeTraceText(out, s.Event)
		}
		out.WriteString(`, "fired": [`)
		for at, fired := range s.Fired {
			if at > 0 {
				out.WriteString(", ")
			}
			out.WriteString(`{"source": `)
			writeTraceText(out, fired.Source)
			out.WriteString(`, "index": `)
			out.WriteString(strconv.FormatUint(uint64(fired.Index), 10))
			out.WriteByte('}')
		}
		out.WriteByte(']')
		writeTraceNames(out, "exited", s.Exited)
		writeTraceNames(out, "entered", s.Entered)
		writeTraceNames(out, "configuration", s.Configuration)
	case TraceStop:
		writeTraceNames(out, "exited", s.Exited)
		out.WriteString(`, "outcome": `)
		if s.Outcome != "" {
			writeTraceText(out, s.Outcome)
		} else {
			out.WriteString("null")
		}
	}
	out.WriteByte('}')
}

// writeTraceText writes text as a JSON string, in the escapes every engine
// writes (EscapeJSONString).
func writeTraceText(out *strings.Builder, text string) {
	out.WriteByte('"')
	out.WriteString(EscapeJSONString(text))
	out.WriteByte('"')
}

func writeTraceNames(out *strings.Builder, field string, names []string) {
	out.WriteString(", ")
	writeTraceText(out, field)
	out.WriteString(": [")
	for at, name := range names {
		if at > 0 {
			out.WriteString(", ")
		}
		writeTraceText(out, name)
	}
	out.WriteByte(']')
}

// RunTrace is the steps of one run, kept as the port is told them, and written
// in the run trace format. The zero value is a trace of no step yet.
type RunTrace struct {
	steps []TraceStep
}

// Push keeps step, after the steps kept before it.
func (r *RunTrace) Push(step TraceStep) {
	r.steps = append(r.steps, step)
}

// Steps is the steps kept, in the order the run took them.
func (r *RunTrace) Steps() []TraceStep {
	return r.steps
}

// JSON is the run in the run trace format: its version, its lane and its
// steps, one step to a line, laid out as the Rust runtime's
// trace_port::RunTrace writes it, so two lanes' runs of one document differ,
// line for line, only where the runs do.
func (r *RunTrace) JSON() string {
	var out strings.Builder
	out.WriteString("{\n  \"version\": ")
	out.WriteString(strconv.Itoa(TraceFormatVersion))
	out.WriteString(",\n  \"lane\": ")
	writeTraceText(&out, TraceLane)
	out.WriteString(",\n  \"steps\": [")
	for at, step := range r.steps {
		if at == 0 {
			out.WriteString("\n    ")
		} else {
			out.WriteString(",\n    ")
		}
		step.writeJSON(&out)
	}
	if len(r.steps) == 0 {
		out.WriteString("]\n}\n")
	} else {
		out.WriteString("\n  ]\n}\n")
	}
	return out.String()
}

// traceRecorder is the port a test set, if any, and the step being told.
//
// A step is opened where Appendix D begins it and closed where it ends; the
// transitions, exits and entries between are added to it as the engine takes
// each. With no port set, nothing is opened, so nothing is kept; and nothing is
// added while no step is open, so an exit or an entry that is no part of a step
// is told to no one. The zero value has no port and no step.
type traceRecorder struct {
	port func(TraceStep)
	step TraceStep
	open bool
}

// set tells port each step from now on; nil tells no port anything. A step
// left open is dropped either way.
func (r *traceRecorder) set(port func(TraceStep)) {
	r.port = port
	r.step = TraceStep{}
	r.open = false
}

// begin opens a step of kind, which took the event named event when it is an
// event step, in place of any step left open; nothing when no port is set.
func (r *traceRecorder) begin(kind TraceKind, event string) {
	if r.port == nil {
		return
	}
	r.step = TraceStep{Kind: kind, Event: event}
	r.open = true
}

// isOpen is whether a step is being told.
func (r *traceRecorder) isOpen() bool {
	return r.open
}

// fired adds to the open step's microstep the transition written index-th in
// the state named source.
func (r *traceRecorder) fired(source string, index uint32) {
	if r.open {
		r.step.Fired = append(r.step.Fired, TraceFired{Source: source, Index: index})
	}
}

// exited adds to the open step the state named state, which it exits.
func (r *traceRecorder) exited(state string) {
	if r.open {
		r.step.Exited = append(r.step.Exited, state)
	}
}

// entered adds to the open step the state named state, which it enters.
func (r *traceRecorder) entered(state string) {
	if r.open {
		r.step.Entered = append(r.step.Entered, state)
	}
}

// end closes the open step and tells the port it, with the configuration it
// left or, for a stop, the top-level <final> the run ended in (empty for none).
func (r *traceRecorder) end(configuration []string, outcome string) {
	if !r.open {
		return
	}
	step := r.step
	r.step = TraceStep{}
	r.open = false
	if step.Kind == TraceStop {
		step.Outcome = outcome
	} else {
		step.Configuration = configuration
	}
	r.port(step)
}

// SetTracePort tells port each step this engine takes from now on, as the run
// trace format says it (docs/adr/0115, D4 and D8); nil stops it. A step is the
// start, each microstep on eventless transitions, each event taken off the
// internal or the external queue with the microstep it selects, and the exit
// of the interpreter, each with the transitions it took by their rows, the
// states it exited and entered in the order it exited and entered them, and the
// configuration it left (TraceStep).
//
// Each step is told once it is over. The port reads and is asked nothing: a
// machine with one set takes the steps one without takes. It runs on the
// goroutine that drives the engine, inside the macrostep, and must not call
// back into the engine; a panic in it is not recovered. Set it before
// Initialize to be told the start. A test sets one; RunTrace keeps what it is
// told and writes the file `sce-codegen formal traces` judges. Only a build
// with the tag sce_trace_port has it.
func (e *Engine[S, E]) SetTracePort(port func(TraceStep)) {
	e.trace.set(port)
}

// traceBeginStart opens the entry into the initial configuration.
func (e *Engine[S, E]) traceBeginStart() {
	e.trace.begin(TraceStart, "")
}

// traceBeginEventless opens a microstep on eventless transitions.
func (e *Engine[S, E]) traceBeginEventless() {
	e.trace.begin(TraceEventless, "")
}

// traceBeginEvent opens the taking of event off the external queue when
// external, or off the internal one, named by the name `_event.name` holds
// while it is taken: the one it arrived under when a door resolved it to a
// member called otherwise, the member's own when that is the name.
func (e *Engine[S, E]) traceBeginEvent(external bool, event EventWithMetadata[E]) {
	if e.trace.port == nil {
		return
	}
	kind := TraceInternal
	if external {
		kind = TraceExternal
	}
	name := event.Metadata.Name
	if name == "" {
		name = e.policy.GetEventName(event.Event)
	}
	e.trace.begin(kind, name)
}

// traceBeginStop opens the exit of the interpreter.
func (e *Engine[S, E]) traceBeginStop() {
	e.trace.begin(TraceStop, "")
}

// traceFired adds the transitions of the open step's microstep, by their rows,
// in selection order: the pairs the transition observer is told.
func (e *Engine[S, E]) traceFired(transitions []EnabledTransition[S, HistoryID]) {
	if !e.trace.isOpen() {
		return
	}
	for _, transition := range transitions {
		e.trace.fired(e.policy.GetStateName(transition.Source), uint32(transition.TransitionIndex))
	}
}

// traceExited adds state, which the open step exits, in this order.
func (e *Engine[S, E]) traceExited(state S) {
	if e.trace.isOpen() {
		e.trace.exited(e.policy.GetStateName(state))
	}
}

// traceEntered adds state, which the open step enters, in this order.
func (e *Engine[S, E]) traceEntered(state S) {
	if e.trace.isOpen() {
		e.trace.entered(e.policy.GetStateName(state))
	}
}

// traceEnd closes the open step and tells the port it: with the configuration
// it left, in document order, or for a stop with the top-level <final> the run
// ended in.
func (e *Engine[S, E]) traceEnd() {
	if !e.trace.isOpen() {
		return
	}
	if e.trace.step.Kind == TraceStop {
		var outcome string
		if ended, ok := e.TerminalState(); ok {
			outcome = e.policy.GetStateName(ended)
		}
		e.trace.end(nil, outcome)
		return
	}
	active := append([]S(nil), e.GetActiveStates()...)
	sort.SliceStable(active, func(i, j int) bool {
		return e.policy.GetDocumentOrder(active[i]) < e.policy.GetDocumentOrder(active[j])
	})
	configuration := make([]string, 0, len(active))
	for _, state := range active {
		configuration = append(configuration, e.policy.GetStateName(state))
	}
	e.trace.end(configuration, "")
}
