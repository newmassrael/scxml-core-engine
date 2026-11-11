// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package testprofile

import (
	"fmt"
	"strconv"
	"strings"

	sce "github.com/newmassrael/sce-go-runtime"
	"github.com/newmassrael/sce-go-runtime/testprofile/canonical"
)

// The recorded input trace of a test case and its replay
// (docs/SCE_TTCN3_LANE.md §10; docs/SCE_TEST_PROFILE.md §3.2, §3.3, §12.4): the
// inputs of one run from the system under test, each at its logical instant,
// `schemas/sce-test-trace.v1.schema.json`, and the system adapter that plays
// them, TraceAdapter.
//
// A trace is one text of the JSON Canonicalization Scheme, so that it has one
// spelling and two traces are equal exactly when their bytes are:
// `{"inputs":[{"at","port","type","value"}, …],"schema","suite","test_case"}`.
// `at` is `NOW` in microseconds as the text of its digits, `port` a system port
// of the test case's configuration, and `type` and `value` the record type's
// name and the value in the canonical form a message record of the test log
// writes (§12.5). The inputs stand in order of `at`; the inputs of one instant
// in the order they are appended to their ports' queues (§3.2 step 2).
//
// The value of an input is read by its type: the reader of the test case's
// configuration (InputReader, generated as `Input` in the configuration's unit)
// knows which records each system port takes and reads each with the reader of
// its record (the readers of package canonical). A trace that does not fit is
// refused whole before the run starts, which then ends with `error` (P13).
//
// Cross-language siblings: `sce_rust_runtime::test::trace` (Rust), which reads
// and refuses the same texts in the same words, `SCE::Test::Trace` (C++),
// `com.sce.runtime.test.Trace` (Kotlin).

// TestTraceSchema is the name of the trace's shape, its `schema`.
const TestTraceSchema = "sce-test-trace.v1"

// TestTraceSchemaStatus is the stability status of the trace's wire schema,
// pinned to the schema file's own `x-sce-schema-status` by a test of this
// package and by `schema_file_declares_status` of the Rust runtime.
// `SCE_WIRE_CONTRACTS.md` requires one commit to move both.
const TestTraceSchemaStatus = "pre-release"

// TraceRefusal is why a trace, or an input of one, is not read: the caller's
// input, refused by name before anything of the run happens.
type TraceRefusal struct {
	// Reason says why.
	Reason string
}

func (r *TraceRefusal) Error() string {
	return r.Reason
}

// NotAnInput is the refusal of an input of the record type typeName on
// systemPort, which the configuration whose system ports are systemPorts does
// not take: the port is none of them, or its type's `in` list does not name
// the type (docs/SCE_TTCN3_LANE.md §10.2).
func NotAnInput(systemPort, typeName string, systemPorts []string) *TraceRefusal {
	for _, declared := range systemPorts {
		if declared == systemPort {
			return &TraceRefusal{Reason: fmt.Sprintf(
				"'%s' is not on the `in` list of the type of the system port '%s'", typeName, systemPort)}
		}
	}
	return &TraceRefusal{Reason: fmt.Sprintf(
		"'%s' is not a system port of the test case's configuration", systemPort)}
}

// InputReader is the reader of the inputs of a test case's configuration: the
// message an input of the record type typeName on the system port systemPort
// carries, its value read from the canonical form, or why it is not one.
//
// Generated as `Input` in the unit of a test configuration, which names the
// records each of its system ports takes.
type InputReader func(systemPort, typeName string, value any) (Message, error)

// NoInputs is the reader of a configuration with no system port: it refuses
// every input, naming its port.
func NoInputs(systemPort, typeName string, _ any) (Message, error) {
	return Message{}, NotAnInput(systemPort, typeName, nil)
}

// TraceInput is one input of a trace.
type TraceInput struct {
	// At is `NOW` when it is due.
	At Instant
	// Port is the system port it arrives on.
	Port string
	// MessageType is the name of its record type, as the test log names it.
	MessageType string
	// Value is its value, in the canonical form of the test log.
	Value any
}

// Trace is a recorded input trace of one test case of one suite.
type Trace struct {
	// Suite is the suite's name.
	Suite string
	// TestCase is the test case's name.
	TestCase string
	// Inputs are the inputs, in order of At, those of one instant in the
	// order they are appended to their ports.
	Inputs []TraceInput
}

// traceMembers is the members of value, an object whose members are names
// exactly, each in the order of names; refused, naming what, otherwise.
func traceMembers(value any, names []string, what string) ([]any, error) {
	object, isObject := value.(*sce.JSONObject)
	if !isObject {
		return nil, &TraceRefusal{Reason: what + " is not an object"}
	}
	for _, member := range object.Keys() {
		known := false
		for _, name := range names {
			if name == member {
				known = true
				break
			}
		}
		if !known {
			return nil, &TraceRefusal{Reason: fmt.Sprintf("%s has no member '%s'", what, member)}
		}
	}
	found := make([]any, 0, len(names))
	for _, name := range names {
		member, present := object.Get(name)
		if !present {
			return nil, &TraceRefusal{Reason: fmt.Sprintf("%s has no '%s'", what, name)}
		}
		found = append(found, member)
	}
	return found, nil
}

// traceName is the text value, which is not empty, at what.
func traceName(value any, what string) (string, error) {
	text, isText := value.(string)
	if !isText || text == "" {
		return "", &TraceRefusal{Reason: what + " is not a name"}
	}
	return text, nil
}

// traceInstant is an instant: the text of the decimal digits of a count of
// microseconds from the start of the test case, with no leading zero, within
// `NOW`'s range.
func traceInstant(value any, what string) (Instant, error) {
	digits, isText := value.(string)
	if !isText || !canonical.DecimalText(digits) || strings.HasPrefix(digits, "-") {
		return 0, &TraceRefusal{Reason: what +
			" is not an instant, the text of the digits of a count of microseconds"}
	}
	count, err := strconv.ParseInt(digits, 10, 64)
	if err != nil {
		return 0, &TraceRefusal{Reason: fmt.Sprintf(
			"%s (%s) is past the microseconds the clock counts to", what, digits)}
	}
	return count, nil
}

// ParseTrace is the trace text holds: the JSON Canonicalization Scheme of one
// trace, which a file may follow with one line feed, as a text file ends.
//
// Refused with a *TraceRefusal, naming why, where the text is not JSON, the
// shape is not `sce-test-trace.v1`, the inputs are out of the order of their
// instants, or the text is not the trace's one canonical spelling. Whether
// each input fits the suite is the configuration's to read (NewTraceAdapter).
func ParseTrace(text string) (Trace, error) {
	body := strings.TrimSuffix(text, "\n")
	value, err := sce.ParseJSON(body)
	if err != nil {
		return Trace{}, &TraceRefusal{Reason: "the trace is " + err.Error()}
	}
	members, err := traceMembers(value, []string{"schema", "suite", "test_case", "inputs"}, "the trace")
	if err != nil {
		return Trace{}, err
	}
	if schema, isText := members[0].(string); !isText || schema != TestTraceSchema {
		return Trace{}, &TraceRefusal{Reason: "the trace's schema is not " + TestTraceSchema}
	}
	items, isArray := members[3].([]any)
	if !isArray {
		return Trace{}, &TraceRefusal{Reason: "the trace's inputs are not an array"}
	}
	read := make([]TraceInput, 0, len(items))
	for index, item := range items {
		what := fmt.Sprintf("input %d", index)
		fields, err := traceMembers(item, []string{"at", "port", "type", "value"}, what)
		if err != nil {
			return Trace{}, err
		}
		at, err := traceInstant(fields[0], "the instant of "+what)
		if err != nil {
			return Trace{}, err
		}
		if len(read) > 0 && read[len(read)-1].At > at {
			return Trace{}, &TraceRefusal{Reason: fmt.Sprintf(
				"%s, at %d, comes after an input at %d: the inputs stand in order of their instants",
				what, at, read[len(read)-1].At)}
		}
		port, err := traceName(fields[1], "the port of "+what)
		if err != nil {
			return Trace{}, err
		}
		messageType, err := traceName(fields[2], "the type of "+what)
		if err != nil {
			return Trace{}, err
		}
		read = append(read, TraceInput{At: at, Port: port, MessageType: messageType, Value: fields[3]})
	}
	suite, err := traceName(members[1], "the trace's suite")
	if err != nil {
		return Trace{}, err
	}
	testCase, err := traceName(members[2], "the trace's test case")
	if err != nil {
		return Trace{}, err
	}
	trace := Trace{Suite: suite, TestCase: testCase, Inputs: read}
	// A value with no canonical text, a number past the range of a double
	// among them, is a trace not written in it either: it is refused here as
	// one, never handed to CanonicalText, which holds that a trace has that
	// text.
	written, err := sce.WriteJCS(trace.canonicalValue())
	if err != nil || written != body {
		return Trace{}, &TraceRefusal{Reason: "the trace is not written in its one canonical text, " +
			"the JSON Canonicalization Scheme"}
	}
	return trace, nil
}

// CanonicalText is the trace in its one canonical text, the JSON
// Canonicalization Scheme, with no line feed after it.
//
// It panics when a value holds a number past the range of a double, a text
// that is not UTF-8 or a value of a type no JSON value is held as, which no
// canonical value holds and ParseTrace refuses.
func (t Trace) CanonicalText() string {
	text, err := sce.WriteJCS(t.canonicalValue())
	if err != nil {
		panic(fmt.Sprintf("the trace of %s %s is not canonical: %v", t.Suite, t.TestCase, err))
	}
	return text
}

// canonicalValue is the trace as the one JSON value its canonical text
// writes.
func (t Trace) canonicalValue() *sce.JSONObject {
	inputs := make([]any, 0, len(t.Inputs))
	for _, input := range t.Inputs {
		record := sce.NewJSONObject()
		record.Set("at", strconv.FormatInt(input.At, 10))
		record.Set("port", input.Port)
		record.Set("type", input.MessageType)
		record.Set("value", input.Value)
		inputs = append(inputs, record)
	}
	trace := sce.NewJSONObject()
	trace.Set("schema", TestTraceSchema)
	trace.Set("suite", t.Suite)
	trace.Set("test_case", t.TestCase)
	trace.Set("inputs", inputs)
	return trace
}

// SystemOutput is what a component sent to a system port, as a TraceAdapter
// records it.
type SystemOutput struct {
	// At is `NOW` when it was sent.
	At Instant
	// SystemPort is the system port it was sent to.
	SystemPort string
	// Message is the message.
	Message Message
}

// traceDue is an input read and waiting for its instant.
type traceDue struct {
	at         Instant
	systemPort string
	message    Message
}

// TraceAdapter is a system adapter that plays a trace (docs/SCE_TTCN3_LANE.md
// §10.5): at each instant it delivers the inputs due then, in the trace's
// order, to their system ports (docs/SCE_TEST_PROFILE.md §3.2 step 2), and it
// records what the components send to the system ports.
//
// It announces the instant of its next input (NextInput), so when no branch
// can fire the run's clock advances to the earlier of that instant and the
// next timer expiry or the guard (§3.3, §12.4 step 4), and the run is the same
// whenever it is played.
type TraceAdapter struct {
	inputs []traceDue
	sent   []SystemOutput
}

// NewTraceAdapter is the player of trace, each input's value read by read, the
// reader of the test case's configuration; a nil read is NoInputs. Refused
// whole with a *TraceRefusal, naming the input, where one does not fit the
// configuration: its port is no system port of it, its type is not on the
// `in` list of that port's type, or its value does not belong to its type
// (docs/SCE_TTCN3_LANE.md §10.2).
func NewTraceAdapter(trace Trace, read InputReader) (*TraceAdapter, error) {
	if read == nil {
		read = NoInputs
	}
	inputs := make([]traceDue, 0, len(trace.Inputs))
	for index, input := range trace.Inputs {
		message, err := read(input.Port, input.MessageType, input.Value)
		if err != nil {
			return nil, &TraceRefusal{Reason: fmt.Sprintf("input %d, at %d: %s", index, input.At, err.Error())}
		}
		inputs = append(inputs, traceDue{at: input.At, systemPort: input.Port, message: message})
	}
	return &TraceAdapter{inputs: inputs}, nil
}

// Sent is what the components have sent to the system ports, in the order
// they sent it. The caller keeps the adapter, which the run only holds, so it
// reads this after the run.
func (a *TraceAdapter) Sent() []SystemOutput {
	sent := make([]SystemOutput, len(a.sent))
	copy(sent, a.sent)
	return sent
}

// Send records message, sent to systemPort at now.
func (a *TraceAdapter) Send(systemPort string, message Message, now Instant) {
	a.sent = append(a.sent, SystemOutput{At: now, SystemPort: systemPort, Message: message})
}

// NextInput is the instant of the earliest input not yet delivered.
func (a *TraceAdapter) NextInput() (Instant, bool) {
	if len(a.inputs) == 0 {
		return 0, false
	}
	return a.inputs[0].at, true
}

// TakeInputsDue is every input due at or before now, in the trace's order.
func (a *TraceAdapter) TakeInputsDue(now Instant) []SystemInput {
	var due []SystemInput
	for len(a.inputs) > 0 && a.inputs[0].at <= now {
		due = append(due, SystemInput{SystemPort: a.inputs[0].systemPort, Message: a.inputs[0].message})
		a.inputs = a.inputs[1:]
	}
	return due
}
