// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The runner program's command line and its readers of a module parameter's
// literal (docs/SCE_TTCN3_LANE.md §4): the Go half of the Rust module's own
// tests (backends/rust/runtime/src/test/runner.rs), each named after the Rust
// case in Pascal case. The program over a generated suite is run by
// backends/go/tests/integration/static_datamodel/a_test_suite_runs_through_its_runner_test.go.

package testprofile

import (
	"bytes"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
)

// The command line every backend's runner takes.
func TestTheCommandLineIsReadAsEveryRunnerReadsIt(t *testing.T) {
	options, err := ParseRunnerOptions([]string{
		"--trace-dir", "traces", "--log-dir", "logs", "--world", "ptcs,guard",
		"--module-parameter", "job=9", "--module-parameter", "label=a=b",
	})
	if err != nil {
		t.Fatalf("the program's command line: %v", err)
	}
	want := RunnerOptions{
		TraceDir: "traces",
		LogDir:   "logs",
		World:    FeaturesOf("ptcs", "guard"),
		ModuleParameters: []ParameterLiteral{
			{ID: "job", Literal: "9"},
			{ID: "label", Literal: "a=b"},
		},
	}
	if !reflect.DeepEqual(options, want) {
		t.Errorf("read %+v, want %+v", options, want)
	}
	bare, err := ParseRunnerOptions([]string{"--log-dir", "l", "--trace-dir", "t"})
	if err != nil || len(bare.World) != 0 || len(bare.ModuleParameters) != 0 {
		t.Errorf("no world and no parameter: %+v, %v", bare, err)
	}
	for _, refusal := range []struct {
		line []string
		why  string
	}{
		{[]string{"--log-dir", "l"}, "--trace-dir is not given"},
		{[]string{"--trace-dir", "t"}, "--log-dir is not given"},
		{[]string{"--trace-dir", "t", "--log-dir", "l", "--trace-dir", "u"}, "--trace-dir is given twice"},
		{[]string{"--trace-dir", "t", "--log-dir", "l", "--seed", "1"}, "--seed is no option"},
		{[]string{"--trace-dir"}, "--trace-dir is given no value"},
		{[]string{"--trace-dir", "t", "--log-dir", "l", "--module-parameter", "job"}, "is not <id>=<value>"},
		{[]string{"--trace-dir", "t", "--log-dir", "l", "--module-parameter", "job=1",
			"--module-parameter", "job=2"}, "'job' is given twice"},
	} {
		_, err := ParseRunnerOptions(refusal.line)
		if err == nil || !strings.Contains(err.Error(), refusal.why) {
			t.Errorf("%s: %v", refusal.why, err)
		}
	}
}

// A module parameter's literal is read as the saved value of its type, and one
// that is no value of it is refused, naming the parameter.
func TestAModuleParameterIsReadAsTheSavedValueOfItsType(t *testing.T) {
	expect := func(got ModuleParameterValue, err error, want any) {
		t.Helper()
		if err != nil || !reflect.DeepEqual(got, want) {
			t.Errorf("read %v, %v, want %v", got, err, want)
		}
	}
	refused := func(_ ModuleParameterValue, err error) string {
		t.Helper()
		if err == nil {
			t.Fatal("a literal that is no value of its type is read")
		}
		return err.Error()
	}
	got, err := IntegerParameter("job", "9", sce.SavedWriteUint32)
	expect(got, err, sce.SavedWriteUint32(9))
	got, err = IntegerParameter("wide", "18446744073709551615", sce.SavedWriteUint64)
	expect(got, err, sce.SavedWriteUint64(18446744073709551615))
	if why := refused(IntegerParameter("job", "256", sce.SavedWriteUint8)); !strings.HasPrefix(why, "job: ") {
		t.Errorf("past uint8: %s", why)
	}
	refused(IntegerParameter("job", "09", sce.SavedWriteUint32))
	got, err = BoolParameter("armed", "true")
	expect(got, err, sce.SavedWriteBool(true))
	refused(BoolParameter("armed", "yes"))
	got, err = TextParameter("nightly")
	expect(got, err, sce.SavedWriteString("nightly"))
	got, err = EnumParameter("green")
	expect(got, err, sce.SavedWriteString("green"))
	got, err = BytesParameter("key", "0aFF")
	expect(got, err, sce.SavedWriteBytes([]byte{0x0a, 0xff}))
	refused(BytesParameter("key", "0a0"))
	refused(BytesParameter("key", "0g"))
	refused(BytesParameter("key", "+a"))
	if got := UndeclaredParameter("limit").Error(); got != "the suite declares no module parameter 'limit'" {
		t.Errorf("an undeclared parameter: %s", got)
	}
}

// §4: the program run in place writes each case's verdict on the standard
// output and its log in the log directory; a case whose trace is missing ends
// in error with its log written and the program exits 0, a command line that
// is not the runner's exits 2 with the usage line last, and so does a log
// directory that cannot be made.
func TestTheRunnerWritesTheLogOfEachTrace(t *testing.T) {
	traces := t.TempDir()
	if err := os.WriteFile(filepath.Join(traces, "pongs.json"), []byte(pongTrace("pong_suite", "{}")+"\n"),
		0o644); err != nil {
		t.Fatal(err)
	}
	suite := Suite{Name: "pong_suite", Executes: []Execute{
		execute(pongCase(t), NoGuard),
		execute(endsWith("unrecorded", "Unrecorded", VerdictPass), NoGuard),
	}}
	run := func(args ...string) (int, string, string) {
		var out, err bytes.Buffer
		status := RunRunner(suite, func(id, _ string) (ModuleParameterValue, error) {
			return nil, UndeclaredParameter(id)
		}, args, &out, &err)
		return status, out.String(), err.String()
	}
	logs := filepath.Join(t.TempDir(), "logs")
	status, out, diagnostics := run("--trace-dir", traces, "--log-dir", logs)
	if status != 0 {
		t.Fatalf("status %d: %s", status, diagnostics)
	}
	if out != "pongs pass\nunrecorded error\n" {
		t.Errorf("the standard output is %q", out)
	}
	if !strings.Contains(diagnostics, "unrecorded: mtc: the trace is refused: ") ||
		!strings.Contains(diagnostics, "is not read") {
		t.Errorf("the missing trace is named on the standard error: %s", diagnostics)
	}
	unrecorded, err := os.ReadFile(filepath.Join(logs, "unrecorded.log.json"))
	if err != nil || string(unrecorded) != `{"records":[{"at":"0","kind":"test-case-started"},`+
		`{"at":"0","kind":"test-case-ended","verdict":"error"}],"schema":"sce-test-log.v1",`+
		`"suite":"pong_suite","test_case":"unrecorded","verdict":"error"}` {
		t.Errorf("the log of a case with no trace: %s, %v", unrecorded, err)
	}
	if _, err := os.Stat(filepath.Join(logs, "pongs.log.json")); err != nil {
		t.Errorf("the log of the replayed case: %v", err)
	}

	status, out, diagnostics = run("--log-dir", logs)
	if status != 2 || out != "" || !strings.HasSuffix(diagnostics, RunnerUsage+"\n") {
		t.Errorf("a command line that is not the runner's: %d %q %q", status, out, diagnostics)
	}
	refusedLogs := filepath.Join(t.TempDir(), "logs")
	status, _, diagnostics = run("--trace-dir", traces, "--log-dir", refusedLogs, "--module-parameter", "limit=1")
	if status != 2 || !strings.Contains(diagnostics, "the suite declares no module parameter 'limit'") {
		t.Errorf("an undeclared parameter: %d %s", status, diagnostics)
	}
	if _, err := os.Stat(refusedLogs); !os.IsNotExist(err) {
		t.Errorf("no case ran, so no log was written: %v", err)
	}
	blocked := filepath.Join(t.TempDir(), "blocked")
	if err := os.WriteFile(blocked, nil, 0o644); err != nil {
		t.Fatal(err)
	}
	status, _, diagnostics = run("--trace-dir", traces, "--log-dir", blocked)
	if status != 2 || !strings.Contains(diagnostics, "is not made") {
		t.Errorf("a log directory that cannot be made: %d %s", status, diagnostics)
	}
}
