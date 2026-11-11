// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The recorded input trace (docs/SCE_TTCN3_LANE.md §10), held to its wire
// schema `schemas/sce-test-trace.v1.schema.json` and to the texts and refusals
// of the Rust module's own tests (backends/rust/runtime/src/test/trace.rs),
// each named after the Rust case in Pascal case, and its replay through
// RunSuiteWithTraces, with a configuration written here.

package testprofile

import (
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	"github.com/newmassrael/sce-go-runtime/testprofile/canonical"
)

// oneInput is a trace of one input, the reply of 0 to job 7 at 500.
const oneInput = `{"inputs":[{"at":"500","port":"sut","type":"job_reply",` +
	`"value":{"code":"0","job":"7"}}],` +
	`"schema":"sce-test-trace.v1","suite":"runtime_suite",` +
	`"test_case":"answers_the_job"}`

// The schema pins the name of the shape the runtime reads, and its status is
// the runtime's.
func TestSchemaFileDeclaresStatus(t *testing.T) {
	raw, err := os.ReadFile(filepath.Join("..", "..", "..", "..", "schemas", "sce-test-trace.v1.schema.json"))
	if err != nil {
		t.Fatalf("the trace's schema: %v", err)
	}
	schema, err := sce.ParseJSON(string(raw))
	if err != nil {
		t.Fatalf("the trace's schema is JSON: %v", err)
	}
	member := func(value any, name string) any {
		object, isObject := value.(*sce.JSONObject)
		if !isObject {
			return nil
		}
		held, _ := object.Get(name)
		return held
	}
	if got := member(schema, "x-sce-schema-status"); got != TestTraceSchemaStatus {
		t.Errorf("schemas/sce-test-trace.v1.schema.json x-sce-schema-status is %v, not "+
			"TestTraceSchemaStatus; SCE_WIRE_CONTRACTS.md requires one commit to move both", got)
	}
	if got := member(member(member(schema, "properties"), "schema"), "const"); got != TestTraceSchema {
		t.Errorf("the schema pins the shape %v, not the one the runtime reads", got)
	}
}

// A trace is read from its canonical text, with or without the line feed a
// file ends in, and written back as the same bytes.
func TestATraceIsReadFromItsCanonicalTextAndWrittenBack(t *testing.T) {
	trace, err := ParseTrace(oneInput)
	if err != nil {
		t.Fatalf("a canonical trace: %v", err)
	}
	if trace.Suite != "runtime_suite" || trace.TestCase != "answers_the_job" {
		t.Errorf("the trace is of %s %s", trace.Suite, trace.TestCase)
	}
	if len(trace.Inputs) != 1 {
		t.Fatalf("inputs %v", trace.Inputs)
	}
	input := trace.Inputs[0]
	if input.At != 500 || input.Port != "sut" || input.MessageType != "job_reply" {
		t.Errorf("input %v", input)
	}
	if value, err := sce.WriteJCS(input.Value); err != nil || value != `{"code":"0","job":"7"}` {
		t.Errorf("the value is %s, %v", value, err)
	}
	if got := trace.CanonicalText(); got != oneInput {
		t.Errorf("written back as %s", got)
	}
	withLineFeed, err := ParseTrace(oneInput + "\n")
	if err != nil || !reflect.DeepEqual(withLineFeed, trace) {
		t.Errorf("the line feed a file ends in: %v, %v", withLineFeed, err)
	}
}

// Each change of one member of a trace it first reads is refused, by name.
func TestATraceThatIsNotOfItsShapeIsRefusedByName(t *testing.T) {
	changed := func(from, to string) string {
		if !strings.Contains(oneInput, from) {
			t.Fatalf("%s is in the trace", from)
		}
		return strings.Replace(oneInput, from, to, 1)
	}
	for _, refusal := range []struct {
		text     string
		expected string
	}{
		{changed("sce-test-trace.v1", "sce-test-trace.v2"), "schema is not sce-test-trace.v1"},
		{changed(`"500"`, "500"), "the instant of input 0"},
		{changed(`"500"`, `"0500"`), "the instant of input 0"},
		{changed(`"500"`, `"-5"`), "the instant of input 0"},
		{changed(`"500"`, `"9223372036854775808"`), "past the microseconds"},
		{changed(`"port":"sut"`, `"port":""`), "the port of input 0"},
		{changed(`"type"`, `"kind"`), "input 0 has no member 'kind'"},
		{changed(`,"suite":"runtime_suite"`, ""), "the trace has no 'suite'"},
		{changed(`{"inputs"`, `{ "inputs"`), "one canonical text"},
		{changed("[{", "{"), "not JSON"},
	} {
		_, err := ParseTrace(refusal.text)
		if err == nil {
			t.Errorf("read %s", refusal.text)
			continue
		}
		if !strings.Contains(err.Error(), refusal.expected) {
			t.Errorf("%s: %s", refusal.expected, err)
		}
	}
	outOfOrder := `{"inputs":[{"at":"5","port":"sut","type":"t","value":{}},` +
		`{"at":"4","port":"sut","type":"t","value":{}}],` +
		`"schema":"sce-test-trace.v1","suite":"s","test_case":"c"}`
	_, err := ParseTrace(outOfOrder)
	if err == nil || !strings.Contains(err.Error(), "input 1, at 4, comes after an input at 5") {
		t.Errorf("out of order: %v", err)
	}
}

// A number past the range of a double has no canonical text, so a trace whose
// value holds one is refused as not written in it, not a panic of the reader.
// The twin of the Rust and Kotlin cases of the same name.
func TestATraceWhoseValueHoldsANumberPastADoubleIsRefused(t *testing.T) {
	from := `{"code":"0","job":"7"}`
	if !strings.Contains(oneInput, from) {
		t.Fatalf("%s is in the trace", from)
	}
	_, err := ParseTrace(strings.Replace(oneInput, from, "1e400", 1))
	if err == nil {
		t.Fatal("a value with no canonical text is read")
	}
	if got := err.Error(); got != "the trace is not written in its one canonical text, "+
		"the JSON Canonicalization Scheme" {
		t.Errorf("refused as %s", got)
	}
}

// An input on a port the configuration does not have, or of a type its port
// does not take, is refused, naming which.
func TestAnInputTheConfigurationDoesNotTakeIsRefusedByName(t *testing.T) {
	if got := NotAnInput("sut", "job_request", []string{"sut"}).Error(); got !=
		"'job_request' is not on the `in` list of the type of the system port 'sut'" {
		t.Errorf("a type the port does not take: %s", got)
	}
	if got := NotAnInput("oracle", "job_reply", []string{"sut"}).Error(); got !=
		"'oracle' is not a system port of the test case's configuration" {
		t.Errorf("a port the configuration does not have: %s", got)
	}
	trace, err := ParseTrace(oneInput)
	if err != nil {
		t.Fatalf("a canonical trace: %v", err)
	}
	for _, read := range []InputReader{NoInputs, nil} {
		_, err = NewTraceAdapter(trace, read)
		if err == nil || err.Error() !=
			"input 0, at 500: 'sut' is not a system port of the test case's configuration" {
			t.Errorf("no system port: %v", err)
		}
	}
}

// pongInputs is the reader of a configuration whose system port `sut` takes
// the record `empty`, read as the message `pong`.
func pongInputs(systemPort, typeName string, value any) (Message, error) {
	if systemPort == "sut" && typeName == "empty" {
		if _, err := canonical.ReadRecord(value, nil, "empty"); err != nil {
			return Message{}, err
		}
		return message("pong"), nil
	}
	return Message{}, NotAnInput(systemPort, typeName, []string{"sut"})
}

// pongCase is a test case whose MTC sends `ping` on its mapped port and
// passes when `pong` arrives within a second.
func pongCase(t *testing.T) TestCase {
	configuration := mapped("Tester")
	configuration.TimersOfType = byType([2]string{"Tester", "reply"})
	m := machines{}.with("Tester", steps{
		onStart: func(context *ComponentContext, _ *scripted) {
			context.Send("p", message("ping"))
			context.StartTimer("reply", second)
		},
		onSnapshot: repliesInTime(t),
	})
	return TestCase{Name: "pongs", Configuration: configuration, Machines: m.factory(), Inputs: pongInputs}
}

// pongTrace is the trace of pongCase in the suite suite, whose input is
// spelled value.
func pongTrace(suite, value string) string {
	return `{"inputs":[{"at":"500","port":"sut","type":"empty","value":` + value + `}],` +
		`"schema":"sce-test-trace.v1","suite":"` + suite + `","test_case":"pongs"}`
}

// docs/SCE_TTCN3_LANE.md §10.5: a trace replayed through RunSuiteWithTraces
// hands each input to its port at its instant, which the clock advances to,
// and the player records what the components send to the system ports.
func TestATraceReplaysItsInputsAtTheirInstants(t *testing.T) {
	trace, err := ParseTrace(pongTrace("pong_suite", "{}"))
	if err != nil {
		t.Fatalf("a canonical trace: %v", err)
	}
	adapter, err := NewTraceAdapter(trace, pongInputs)
	if err != nil {
		t.Fatalf("the trace fits: %v", err)
	}
	result := NewTestCaseRun(pongCase(t).Configuration, pongCase(t).Machines, adapter, NoGuard, nil).Run()
	if result.Verdict != VerdictPass || result.EndedAt != 500 {
		t.Fatalf("verdict %v at %d: %v", result.Verdict, result.EndedAt, result.Log)
	}
	sent := adapter.Sent()
	if len(sent) != 1 || sent[0].At != 0 || sent[0].SystemPort != "sut" || sent[0].Message.Event != "ping" {
		t.Errorf("sent %v", sent)
	}

	suite := Suite{Name: "pong_suite", Executes: []Execute{execute(pongCase(t), NoGuard)}}
	results, err := RunSuiteWithTraces(suite, FeaturesOf(), func(string) (Trace, error) {
		return ParseTrace(pongTrace("pong_suite", "{}"))
	}, nil)
	if err != nil {
		t.Fatal(err)
	}
	if len(results) != 1 || results[0].Result.Verdict != VerdictPass {
		t.Fatalf("results %v", results)
	}
}

// P13: a test case whose trace is not given, is of another suite, or does not
// fit its configuration ends with error before it starts, naming why.
func TestATestCaseWhoseTraceDoesNotFitEndsInError(t *testing.T) {
	suite := Suite{Name: "pong_suite", Executes: []Execute{execute(pongCase(t), NoGuard)}}
	for _, refusal := range []struct {
		trace func(string) (Trace, error)
		why   string
	}{
		{func(string) (Trace, error) { return ParseTrace("") }, "the trace is refused: the trace is not JSON"},
		{func(string) (Trace, error) { return ParseTrace(pongTrace("other_suite", "{}")) },
			"the trace is of the test case 'pongs' of the suite 'other_suite'"},
		{func(string) (Trace, error) { return ParseTrace(pongTrace("pong_suite", `{"n":"1"}`)) },
			"the trace is refused: input 0, at 500: empty: 'n' is no field of its record"},
	} {
		results, err := RunSuiteWithTraces(suite, FeaturesOf(), refusal.trace, nil)
		if err != nil {
			t.Fatal(err)
		}
		if len(results) != 1 || results[0].Result.Verdict != VerdictError {
			t.Fatalf("%s: results %v", refusal.why, results)
		}
		log := results[0].Result.Log
		if len(log) != 1 || !strings.HasPrefix(log[0].Detail, refusal.why) {
			t.Errorf("%s: log %v", refusal.why, log)
		}
	}
}
