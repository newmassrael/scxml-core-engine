// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The records of the test log and its canonical text
// (docs/SCE_TEST_PROFILE.md §12.5), held to the wire schema
// `schemas/sce-test-log.v1.schema.json` and to the bytes the Rust module's own
// tests expect (backends/rust/runtime/src/test/test_log.rs), each named after
// the Rust case in Pascal case.

package testprofile

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	"github.com/newmassrael/sce-go-runtime/testprofile/canonical"
)

// testLogSchema is the text of the test log's wire schema, from this
// package's directory.
func testLogSchema(t *testing.T) string {
	t.Helper()
	raw, err := os.ReadFile(filepath.Join("..", "..", "..", "..", "schemas", "sce-test-log.v1.schema.json"))
	if err != nil {
		t.Fatalf("the test log's schema: %v", err)
	}
	return string(raw)
}

// The schema pins the name of the shape the runtime writes.
func TestTheSchemaPinsTheShapeTheRuntimeWrites(t *testing.T) {
	parsed, err := sce.ParseJSON(testLogSchema(t))
	if err != nil {
		t.Fatalf("the test log's schema is JSON: %v", err)
	}
	member := func(value any, name string) any {
		object, ok := value.(*sce.JSONObject)
		if !ok {
			t.Fatalf("no object holds %q", name)
		}
		held, _ := object.Get(name)
		return held
	}
	shape := member(member(member(parsed, "properties"), "schema"), "const")
	if shape != TestLogSchema {
		t.Fatalf("the schema pins %v, the runtime writes %s", shape, TestLogSchema)
	}
}

// Every kind the runtime writes is one the schema names, each once.
func TestEveryKindIsOneTheSchemaNames(t *testing.T) {
	schema := testLogSchema(t)
	for _, event := range []TestLogEvent{
		TestCaseStarted{},
		TestCaseEnded{},
		ComponentCreated{},
		ComponentDone{},
		MessageSent{},
		MessageReceived{},
		TimerStarted{},
		TimerStopped{},
		TimerTimedOut{},
		VerdictSet{},
		Logged{},
	} {
		named := `"const": "` + event.Kind() + `"`
		if count := strings.Count(schema, named); count != 1 {
			t.Errorf("the schema names the kind %s %d times", event.Kind(), count)
		}
	}
}

// A record of each shape: `at` and every integer as the text of its digits, an
// absent reason omitted, the members sorted when written.
func TestARecordIsWrittenWithTheMembersOfItsKind(t *testing.T) {
	for _, c := range []struct {
		event TestLogEvent
		want  string
	}{
		{
			TimerStarted{Component: "mtc", Timer: "reply", Duration: 1_000_000},
			`{"at":"250","component":"mtc","duration_us":"1000000","kind":"timer-started","timer":"reply"}`,
		},
		{
			VerdictSet{Component: "echo", Verdict: VerdictInconclusive},
			`{"at":"250","component":"echo","kind":"verdict-set","verdict":"inconc"}`,
		},
		{
			VerdictSet{Component: "echo", Verdict: VerdictFail, Reason: "late"},
			`{"at":"250","component":"echo","kind":"verdict-set","reason":"late","verdict":"fail"}`,
		},
		{
			MessageReceived{
				Component:   "mtc",
				Port:        "sut",
				MessageType: "job_reply",
				Value:       canonical.Record(canonical.Field("job", canonical.Integer(uint32(7)))),
			},
			`{"at":"250","component":"mtc","kind":"message-received","port":"sut","type":"job_reply","value":{"job":"7"}}`,
		},
	} {
		got, err := sce.WriteJCS(TestLogRecord{At: 250, Event: c.event}.ToValue())
		if err != nil {
			t.Fatalf("a record is canonical: %v", err)
		}
		if got != c.want {
			t.Errorf("wrote %s, want %s", got, c.want)
		}
	}
}

// The log of a run: its shape, suite, test case and verdict beside its
// records, all sorted, with no whitespace.
func TestALogIsItsShapeSuiteCaseVerdictAndRecords(t *testing.T) {
	result := TestCaseResult{
		Verdict: VerdictError,
		Records: []TestLogRecord{
			{At: 0, Event: TestCaseStarted{}},
			{At: 0, Event: TestCaseEnded{Verdict: VerdictError}},
		},
	}
	want := `{"records":[{"at":"0","kind":"test-case-started"},` +
		`{"at":"0","kind":"test-case-ended","verdict":"error"}],` +
		`"schema":"sce-test-log.v1","suite":"a_suite","test_case":"a_case","verdict":"error"}`
	if got := result.CanonicalLog("a_suite", "a_case"); got != want {
		t.Fatalf("wrote %s, want %s", got, want)
	}
}
