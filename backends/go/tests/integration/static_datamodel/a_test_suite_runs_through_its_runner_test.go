// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// A test suite generated in Go (docs/SCE_TEST_PROFILE.md §11.6 to §11.8,
// §12.2, §12.6 slice 6), run end to end by the profile's runner,
// testprofile.RunSuite. The suite, its test cases and their configurations are
// the tables sce-codegen writes from the fixtures under
// `sce-build/tests/fixtures/test_profile/runtime/` into a package each beside
// this file, and the components are the machines it writes from the components
// there, each in the package its configuration's table imports it from (regen:
// `scripts/regen_static_datamodel_go.sh`). Only the system adapter, which plays
// the system under test, is written here; the same suite is also replayed from
// the committed trace of `answers_the_job` (docs/SCE_TTCN3_LANE.md §10.5)
// through testprofile.RunSuiteWithTraces, and from every committed trace, each
// to its expected log, through the runner in place (testprofile.RunRunner) and
// through the suite's runner program (§4) as a process.
//
// Each test is the twin of the Rust case of the same name in
// backends/rust/tests/tests/a_test_suite_runs_through_its_runner.rs, named
// `TestATestSuiteRunsThroughItsRunner_<Case>` after it in Pascal case
// (`runs_every_test_case_of_a_world_with_the_verdict_it_expects` is
// TestATestSuiteRunsThroughItsRunner_RunsEveryTestCaseOfAWorldWithTheVerdictItExpects),
// itself the twin of the C++ gtest in
// tests/integration/ATestSuiteRunsEndToEndTest.cpp and of the Kotlin
// ATestSuiteRunsThroughItsRunnerTest.kt, with the same suite, world, verdicts
// and `NOW`.

package static_datamodel

import (
	"bytes"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"sort"
	"strings"
	"testing"

	sce "github.com/newmassrael/sce-go-runtime"
	"github.com/newmassrael/sce-go-runtime/testprofile"

	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/answers_the_job"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/asking_configuration"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/job_reply"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/job_request"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/reply_ok"
	"github.com/newmassrael/sce-go-tests/integration/static_datamodel/runtime_suite"
)

// suiteMillisecond is a millisecond of `NOW`, which counts microseconds.
const suiteMillisecond testprofile.Duration = 1000

// suiteSecond is a second of `NOW`.
const suiteSecond testprofile.Duration = 1000 * suiteMillisecond

// suiteAnswerDelay is how long the system under test takes to answer a
// request.
const suiteAnswerDelay testprofile.Duration = 500

// suitePending is a reply the system under test will hand the run at its
// instant.
type suitePending struct {
	at      testprofile.Instant
	message testprofile.Message
}

// suiteResponder is the system under test on the system port `sut`: it answers
// each request, suiteAnswerDelay after it, with a reply of code for the job the
// request names, and records each job it was asked for in requested, which the
// test keeps across the test cases of a run while each run holds its own
// adapter.
type suiteResponder struct {
	t         *testing.T
	code      uint32
	requested *[]uint32
	pending   []suitePending
}

func (r *suiteResponder) Send(systemPort string, message testprofile.Message, now testprofile.Instant) {
	if systemPort != "sut" {
		r.t.Errorf("a request is sent on the system port sut, not %s", systemPort)
	}
	request, ok := testprofile.ValueAs[job_request.JobRequestPayload](message)
	if !ok {
		r.t.Errorf("a request travels as its record's value: %v", message)
		return
	}
	*r.requested = append(*r.requested, request.Job)
	reply := job_reply.JobReplyPayload{Job: request.Job, Code: r.code}
	// The reply's canonical form in the test log is the one the unit of a
	// template over its record writes.
	r.pending = append(r.pending, suitePending{
		at:      now + suiteAnswerDelay,
		message: testprofile.NewMessage(job_reply.EventName, reply, reply_ok.TYPE, reply_ok.Canonical),
	})
}

func (r *suiteResponder) NextInput() (testprofile.Instant, bool) {
	if len(r.pending) == 0 {
		return 0, false
	}
	return r.pending[0].at, true
}

func (r *suiteResponder) TakeInputsDue(now testprofile.Instant) []testprofile.SystemInput {
	var due []testprofile.SystemInput
	for len(r.pending) > 0 && r.pending[0].at <= now {
		due = append(due, testprofile.SystemInput{SystemPort: "sut", Message: r.pending[0].message})
		r.pending = r.pending[1:]
	}
	return due
}

// suiteRun is the suite's run in a world of features, answered with code by a
// fresh suiteResponder for each test case, with overrides in place of the
// defaults of the module parameters they give; nil overrides none.
func suiteRun(
	t *testing.T,
	features testprofile.Features,
	code uint32,
	requested *[]uint32,
	overrides *testprofile.ModuleParameters,
) []testprofile.CaseResult {
	t.Helper()
	adapters := func(string) testprofile.SystemAdapter {
		return &suiteResponder{t: t, code: code, requested: requested}
	}
	results, err := testprofile.RunSuite(runtime_suite.Suite(), features, adapters, overrides)
	if err != nil {
		t.Fatalf("the suite declares every module parameter it is given: %v", err)
	}
	return results
}

// suiteNames are the names of the test cases results reports, in run order.
func suiteNames(results []testprofile.CaseResult) []string {
	names := make([]string, 0, len(results))
	for _, result := range results {
		names = append(names, result.TestCase)
	}
	return names
}

// suiteLogged reports whether the test log of result holds a record of kind.
func suiteLogged(result testprofile.CaseResult, kind testprofile.LogKind) bool {
	for _, record := range result.Result.Log {
		if record.Kind == kind {
			return true
		}
	}
	return false
}

// P11, P12, P13, P14: in a world with every feature the suite runs every test
// case in its order, and each ends with the verdict it expects: a job answered
// in time passes, a PTC served passes, a removed PTC's inconclusive outweighs
// its MTC's pass, and a guard shorter than the answer ends in error.
func TestATestSuiteRunsThroughItsRunner_RunsEveryTestCaseOfAWorldWithTheVerdictItExpects(t *testing.T) {
	var requested []uint32
	results := suiteRun(t, testprofile.FeaturesOf("ptcs", "removal", "guard"), 0, &requested, nil)
	want := []string{"answers_the_job", "serves_a_ptc", "keeps_a_ptc", "answers_before_the_guard"}
	if got := suiteNames(results); !reflect.DeepEqual(got, want) {
		t.Fatalf("the suite runs %v", got)
	}
	outcome := make([]testprofile.TestCaseResult, 0, len(results))
	for _, result := range results {
		outcome = append(outcome, result.Result)
	}
	if outcome[0].Verdict != testprofile.VerdictPass {
		t.Errorf("answers_the_job: %v: %v", outcome[0].Verdict, outcome[0].Log)
	}
	if outcome[0].EndedAt != suiteAnswerDelay {
		t.Errorf("the reply is taken in the round it arrives in: ended at %d", outcome[0].EndedAt)
	}
	if outcome[1].Verdict != testprofile.VerdictPass {
		t.Errorf("serves_a_ptc: %v: %v", outcome[1].Verdict, outcome[1].Log)
	}
	if outcome[1].EndedAt != 0 {
		t.Errorf("a branch fired in every round, so time never moved: ended at %d", outcome[1].EndedAt)
	}
	if outcome[2].Verdict != testprofile.VerdictInconclusive {
		t.Errorf("keeps_a_ptc: %v: %v", outcome[2].Verdict, outcome[2].Log)
	}
	if outcome[2].EndedAt != 15*suiteMillisecond {
		t.Errorf("keeps_a_ptc ended at %d", outcome[2].EndedAt)
	}
	if outcome[3].Verdict != testprofile.VerdictError {
		t.Errorf("answers_before_the_guard: %v: %v", outcome[3].Verdict, outcome[3].Log)
	}
	if outcome[3].EndedAt != 100 {
		t.Errorf("the guard, 0.1ms, ends the case before the answer: ended at %d", outcome[3].EndedAt)
	}
	if !suiteLogged(results[3], testprofile.LogError) {
		t.Errorf("the guard is logged as an error: %v", outcome[3].Log)
	}
	if !reflect.DeepEqual(requested, []uint32{7, 7}) {
		t.Errorf("each asking case asks for the job the suite's module parameter gives: %v", requested)
	}
}

// P2, P7: a system under test that answers with a failure code fails the test
// case when its timer runs out. A world with no feature runs only the test
// case no selection guards.
func TestATestSuiteRunsThroughItsRunner_FailsAJobTheSystemUnderTestAnswersWithAFailure(t *testing.T) {
	var requested []uint32
	results := suiteRun(t, testprofile.FeaturesOf(), 3, &requested, nil)
	if got := suiteNames(results); !reflect.DeepEqual(got, []string{"answers_the_job"}) {
		t.Fatalf("the suite runs %v", got)
	}
	if results[0].Result.Verdict != testprofile.VerdictFail {
		t.Errorf("verdict %v: %v", results[0].Result.Verdict, results[0].Result.Log)
	}
	if results[0].Result.EndedAt != suiteSecond {
		t.Errorf("time advances to the timer's instant: ended at %d", results[0].Result.EndedAt)
	}
	if !suiteLogged(results[0], testprofile.LogTimeout) {
		t.Errorf("the timeout is logged: %v", results[0].Result.Log)
	}
}

// suiteAnswersTheJobLog is the test log of the run of answers_the_job that
// every runtime writes, the one file the Rust, C++ and Kotlin runs are held to
// too (`a_generated_test_case_writes_its_test_log` in
// backends/rust/tests/tests/a_test_suite_runs_through_its_runner.rs), so the
// agreement of the backends is one set of bytes. The file ends in a newline,
// which no canonical text holds.
func suiteAnswersTheJobLog(t *testing.T) string {
	t.Helper()
	raw, err := os.ReadFile(filepath.Join("..", "..", "..", "..", "..", "sce-build", "tests", "fixtures",
		"test_profile", "runtime", "expected_logs", "answers_the_job.json"))
	if err != nil {
		t.Fatalf("the shared test log: %v", err)
	}
	text, ended := strings.CutSuffix(string(raw), "\n")
	if !ended {
		t.Fatal("the shared test log ends in one newline")
	}
	return text
}

// docs/SCE_TEST_PROFILE.md §12.5: the test log of a generated test case, in
// its one canonical text. The request the component sends carries the value
// its template's unit writes, the job of the suite's module parameter; the
// reply arrives at 500 and is taken in that round, which stops the timer,
// passes and ends the MTC.
func TestATestSuiteRunsThroughItsRunner_AGeneratedTestCaseWritesItsTestLog(t *testing.T) {
	var requested []uint32
	results := suiteRun(t, testprofile.FeaturesOf(), 0, &requested, nil)
	if got := suiteNames(results); !reflect.DeepEqual(got, []string{"answers_the_job"}) {
		t.Fatalf("the suite runs %v", got)
	}
	got, want := results[0].CanonicalLog("runtime_suite"), suiteAnswersTheJobLog(t)
	if got != want {
		t.Fatalf("the test log is\n%s\nnot the shared\n%s", got, want)
	}
}

// suiteTracesDir is the directory of the suite's committed traces
// (docs/SCE_TTCN3_LANE.md §10.2), the input the Responder above gives
// answers_the_job when it answers with success, as a file every backend's
// player reads.
func suiteTracesDir() string {
	return filepath.Join("..", "..", "..", "..", "..", "sce-build", "tests", "fixtures", "test_profile", "runtime",
		"traces")
}

// docs/SCE_TTCN3_LANE.md §10.5: the generated suite replayed from the committed
// trace of `answers_the_job` writes the test log every runtime writes of it,
// byte for byte: the configuration's unit reads the reply from its canonical
// form and writes it back the same, and the clock advances to the trace's
// instant as it did to the Responder's. The twin of
// `a_recorded_trace_replays_to_the_test_log_every_runtime_writes` (Rust).
func TestATestSuiteRunsThroughItsRunner_ARecordedTraceReplaysToTheTestLogEveryRuntimeWrites(t *testing.T) {
	raw, err := os.ReadFile(filepath.Join(suiteTracesDir(), "answers_the_job.json"))
	if err != nil {
		t.Fatalf("the committed trace: %v", err)
	}
	results, err := testprofile.RunSuiteWithTraces(runtime_suite.Suite(), testprofile.FeaturesOf(),
		func(string) (testprofile.Trace, error) { return testprofile.ParseTrace(string(raw)) }, nil)
	if err != nil {
		t.Fatalf("the suite declares every module parameter it is given: %v", err)
	}
	if got := suiteNames(results); !reflect.DeepEqual(got, []string{"answers_the_job"}) {
		t.Fatalf("the suite runs %v", got)
	}
	if results[0].Result.Verdict != testprofile.VerdictPass {
		t.Errorf("verdict %v: %v", results[0].Result.Verdict, results[0].Result.Log)
	}
	if got, want := results[0].CanonicalLog("runtime_suite"), suiteAnswersTheJobLog(t); got != want {
		t.Fatalf("the test log is\n%s\nnot the shared\n%s", got, want)
	}
}

// §10.2, §10.5: the configuration's unit reads the reply from its canonical
// form into the record its components take, and writes it back the same; an
// input on a port it does not have, of a type the port does not take, or of a
// value that does not belong to the type is refused by name.
func TestATestSuiteRunsThroughItsRunner_TheConfigurationReadsTheInputsOfARecordedTrace(t *testing.T) {
	value, err := sce.ParseJSON(`{"code":"0","job":"7"}`)
	if err != nil {
		t.Fatal(err)
	}
	reply, err := asking_configuration.ReadJobReply(value)
	if err != nil || reply != (job_reply.JobReplyPayload{Job: 7, Code: 0}) {
		t.Fatalf("the reply reads as %v, %v", reply, err)
	}
	if text, err := sce.WriteJCS(asking_configuration.CanonicalJobReply(reply)); err != nil ||
		text != `{"code":"0","job":"7"}` {
		t.Errorf("the reply is written back as %s, %v", text, err)
	}
	message, err := asking_configuration.Input("sut", "job_reply", value)
	if err != nil {
		t.Fatalf("an input the configuration takes: %v", err)
	}
	if message.Event != job_reply.EventName || message.TypeName() != "job_reply" {
		t.Errorf("the message is %v of %s", message, message.TypeName())
	}
	if held, ok := testprofile.ValueAs[job_reply.JobReplyPayload](message); !ok || held != reply {
		t.Errorf("the message carries %v", held)
	}
	refusedAs := func(systemPort, typeName string, value any) string {
		t.Helper()
		_, err := asking_configuration.Input(systemPort, typeName, value)
		if err == nil {
			t.Fatalf("%s %s is read", systemPort, typeName)
		}
		return err.Error()
	}
	if got := refusedAs("sut", "job_request", nil); got !=
		"'job_request' is not on the `in` list of the type of the system port 'sut'" {
		t.Errorf("a type the port does not take: %s", got)
	}
	if got := refusedAs("oracle", "job_reply", nil); got !=
		"'oracle' is not a system port of the test case's configuration" {
		t.Errorf("a port the configuration does not have: %s", got)
	}
	malformed, err := sce.ParseJSON(`{"code":"0","job":"07"}`)
	if err != nil {
		t.Fatal(err)
	}
	if got := refusedAs("sut", "job_reply", malformed); got !=
		"job_reply.job: '07' is not the text of an integer's decimal digits" {
		t.Errorf("a value that is not of its type: %s", got)
	}
}

// suiteProgramRun is what one run of the runner program printed and the status
// it exited with.
type suiteProgramRun struct {
	status int
	out    string
	err    string
}

// suiteRunInPlace is the suite's runner program run in place
// (testprofile.RunRunner), given args after the directories of the committed
// traces and of the logs.
func suiteRunInPlace(logs string, args ...string) suiteProgramRun {
	return suiteRunInPlaceFrom(suiteTracesDir(), logs, args...)
}

// suiteRunInPlaceFrom is the suite's runner program run in place, given args
// after the directories traces and logs.
func suiteRunInPlaceFrom(traces, logs string, args ...string) suiteProgramRun {
	var out, err bytes.Buffer
	line := append([]string{"--trace-dir", traces, "--log-dir", logs}, args...)
	status := testprofile.RunRunner(runtime_suite.Suite(), runtime_suite.ModuleParameter, line, &out, &err)
	return suiteProgramRun{status: status, out: out.String(), err: err.String()}
}

// suiteEveryFeature is the world that selects every test case of the suite, so
// that the runner replays every trace of the directory.
const suiteEveryFeature = "ptcs,removal,guard"

// suiteEveryVerdict is what the runner prints in the world of every feature:
// each case of the suite, in its order, with the verdict it ends with.
const suiteEveryVerdict = "answers_the_job pass\nserves_a_ptc pass\nkeeps_a_ptc inconc\n" +
	"answers_before_the_guard error\n"

// suiteExpectedLogsDir is the directory of the expected log of each committed
// trace, `<test case>.json` each: the canonical log a reviewed run wrote of it,
// with one newline after it.
func suiteExpectedLogsDir() string {
	return filepath.Join("..", "..", "..", "..", "..", "sce-build", "tests", "fixtures", "test_profile", "runtime",
		"expected_logs")
}

// suiteFileNames are the names of the files in dir, sorted.
func suiteFileNames(t *testing.T, dir string) []string {
	t.Helper()
	entries, err := os.ReadDir(dir)
	if err != nil {
		t.Fatalf("the directory %s is read: %v", dir, err)
	}
	names := make([]string, 0, len(entries))
	for _, entry := range entries {
		names = append(names, entry.Name())
	}
	sort.Strings(names)
	return names
}

// suiteEveryLogIsItsExpectedLog holds every log the runner wrote into logs,
// `<case>.log.json`, to the expected log of its case byte for byte, and the
// runner to have written a log for every expected log and nothing else
// (docs/SCE_TTCN3_LANE.md §12, §13.1 S2). The twin of
// `assert_every_log_is_its_expected_log` (Rust).
func suiteEveryLogIsItsExpectedLog(t *testing.T, logs string) {
	t.Helper()
	var expected []string
	for _, name := range suiteFileNames(t, suiteExpectedLogsDir()) {
		testCase, ok := strings.CutSuffix(name, ".json")
		if !ok {
			t.Errorf("the expected log %s is a .json file", name)
		}
		expected = append(expected, testCase)
	}
	var written []string
	for _, name := range suiteFileNames(t, logs) {
		testCase, ok := strings.CutSuffix(name, ".log.json")
		written = append(written, testCase)
		if !ok {
			t.Errorf("the runner wrote %s, which is no log of a case", name)
			continue
		}
		raw, err := os.ReadFile(filepath.Join(suiteExpectedLogsDir(), testCase+".json"))
		if err != nil {
			t.Errorf("the case %s has no expected log: %v", testCase, err)
			continue
		}
		want, ended := strings.CutSuffix(string(raw), "\n")
		if !ended {
			t.Errorf("the expected log of %s ends in one newline", testCase)
		}
		if got := suiteLogText(t, filepath.Join(logs, name)); got != want {
			t.Errorf("the log of %s is\n%s\nnot its expected log\n%s", testCase, got, want)
		}
	}
	if !reflect.DeepEqual(written, expected) {
		t.Errorf("the runner wrote a log of each case that has an expected log, and of no other: wrote %v, "+
			"expected %v", written, expected)
	}
}

// suiteLogText is the text of the log file at path, failing the test where it
// is not written.
func suiteLogText(t *testing.T, path string) string {
	t.Helper()
	raw, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("the log of the case is written: %v", err)
	}
	return string(raw)
}

// docs/SCE_TTCN3_LANE.md §4: the suite's runner program runs each test case its
// world selects against its trace and writes `<case>.log.json`, the canonical
// log with nothing after it, and prints each case's verdict. In the world of
// every feature it replays every trace of the directory, and each log it writes
// is its case's expected log (§12, §13.1 S2). A module parameter's value on its
// command line replaces its default, and one the suite does not declare, or
// that is no value of its type, is refused with exit status 2 before any case
// runs. The twin of `the_suites_runner_program_writes_the_log_of_each_trace`
// (Rust).
func TestATestSuiteRunsThroughItsRunner_TheSuitesRunnerProgramWritesTheLogOfEachTrace(t *testing.T) {
	logs := filepath.Join(t.TempDir(), "logs")
	run := suiteRunInPlace(logs, "--world", suiteEveryFeature)
	if run.status != 0 {
		t.Fatalf("status %d: %s", run.status, run.err)
	}
	if run.out != suiteEveryVerdict {
		t.Errorf("the standard output is %q", run.out)
	}
	suiteEveryLogIsItsExpectedLog(t, logs)

	overridden := filepath.Join(t.TempDir(), "logs")
	given := suiteRunInPlace(overridden, "--world", "", "--module-parameter", "job=9")
	if given.status != 0 {
		t.Fatalf("status %d: %s", given.status, given.err)
	}
	log := suiteLogText(t, filepath.Join(overridden, "answers_the_job.log.json"))
	if !strings.Contains(log,
		`"kind":"message-sent","port":"sut","type":"job_request","value":{"job":"9"}`) {
		t.Errorf("the request names the job of the value given: %s", log)
	}

	for _, refusal := range []struct {
		literal string
		why     string
	}{
		{"limit=1", "the suite declares no module parameter 'limit'"},
		{"job=seven", "--module-parameter job=seven: job: 'seven' is not the text of an integer's decimal digits"},
	} {
		refused := filepath.Join(t.TempDir(), "logs")
		attempt := suiteRunInPlace(refused, "--module-parameter", refusal.literal)
		if attempt.status != 2 {
			t.Errorf("%s: status %d", refusal.literal, attempt.status)
		}
		if !strings.Contains(attempt.err, refusal.why) || !strings.HasSuffix(attempt.err, testprofile.RunnerUsage+"\n") {
			t.Errorf("%s: %s", refusal.literal, attempt.err)
		}
		if _, err := os.Stat(refused); !os.IsNotExist(err) {
			t.Errorf("no case ran, so no log was written: %v", err)
		}
	}
}

// P13, §4: a test case whose trace is missing ends with `error` and its log is
// written, the reason on the standard error, and the program exits 0, since
// the verdict is the log's. The directory of the traces holds the committed
// trace of answers_the_job alone, so serves_a_ptc's is missing.
func TestATestSuiteRunsThroughItsRunner_ATestCaseWhoseTraceIsMissingEndsInErrorAndItsLogIsWritten(t *testing.T) {
	traces := t.TempDir()
	raw, err := os.ReadFile(filepath.Join(suiteTracesDir(), "answers_the_job.json"))
	if err != nil {
		t.Fatalf("the committed trace: %v", err)
	}
	if err := os.WriteFile(filepath.Join(traces, "answers_the_job.json"), raw, 0o644); err != nil {
		t.Fatalf("the trace is copied: %v", err)
	}
	logs := filepath.Join(t.TempDir(), "logs")
	run := suiteRunInPlaceFrom(traces, logs, "--world", "ptcs")
	if run.status != 0 {
		t.Fatalf("status %d: %s", run.status, run.err)
	}
	if run.out != "answers_the_job pass\nserves_a_ptc error\n" {
		t.Errorf("the standard output is %q", run.out)
	}
	want := `{"records":[{"at":"0","kind":"test-case-started"},{"at":"0","kind":"test-case-ended",` +
		`"verdict":"error"}],"schema":"sce-test-log.v1","suite":"runtime_suite",` +
		`"test_case":"serves_a_ptc","verdict":"error"}`
	if got := suiteLogText(t, filepath.Join(logs, "serves_a_ptc.log.json")); got != want {
		t.Errorf("the log of the case with no trace is %s", got)
	}
	if !strings.Contains(run.err, "serves_a_ptc: mtc: the trace is refused: ") ||
		!strings.Contains(run.err, "is not read") {
		t.Errorf("the missing trace is named on the standard error: %s", run.err)
	}
}

// suiteProgram runs the suite's runner program as a process of its own, given
// args after the directories of the traces and of the logs: this test binary,
// which TestMain turns into the program, runtime_suite.Main, the one
// cmd/runtime_suite_runner calls (suiteRunnerProgramEnv).
func suiteProgram(t *testing.T, logs string, args ...string) suiteProgramRun {
	t.Helper()
	line := append([]string{"--trace-dir", suiteTracesDir(), "--log-dir", logs}, args...)
	command := exec.Command(os.Args[0], line...)
	command.Env = append(os.Environ(), suiteRunnerProgramEnv+"=1")
	var out, err bytes.Buffer
	command.Stdout = &out
	command.Stderr = &err
	status := 0
	if failure := command.Run(); failure != nil {
		exited, isExit := failure.(*exec.ExitError)
		if !isExit {
			t.Fatalf("the runner program runs: %v", failure)
		}
		status = exited.ExitCode()
	}
	return suiteProgramRun{status: status, out: out.String(), err: err.String()}
}

// §4: the suite's runner is a program: run as a process of its own, it prints
// the verdict of each case on its standard output and nothing else, every
// diagnostic on its standard error, writes the log of each case, and exits with
// the status the command line asks for. The twin of the C++
// `ARunnerProgram.WritesTheLogOfEachTrace` and the Kotlin
// `theSuitesUnitIsTheRunnerProgram`.
func TestATestSuiteRunsThroughItsRunner_TheSuitesUnitIsTheRunnerProgram(t *testing.T) {
	logs := filepath.Join(t.TempDir(), "logs")
	run := suiteProgram(t, logs, "--world", suiteEveryFeature)
	if run.status != 0 {
		t.Fatalf("status %d: %s", run.status, run.err)
	}
	if run.out != suiteEveryVerdict {
		t.Errorf("the standard output is the contract's lines alone: %q", run.out)
	}
	// The standard error may hold the engine's diagnostics, whatever they say:
	// none of its lines is one of the standard output's.
	for _, line := range strings.Split(run.err, "\n") {
		if line != "" && strings.Contains(run.out, line+"\n") {
			t.Errorf("a line of the standard error is one of the standard output's: %s", line)
		}
	}
	suiteEveryLogIsItsExpectedLog(t, logs)

	overridden := filepath.Join(t.TempDir(), "logs")
	given := suiteProgram(t, overridden, "--world", "", "--module-parameter", "job=9")
	if given.status != 0 {
		t.Fatalf("status %d: %s", given.status, given.err)
	}
	log := suiteLogText(t, filepath.Join(overridden, "answers_the_job.log.json"))
	if !strings.Contains(log,
		`"kind":"message-sent","port":"sut","type":"job_request","value":{"job":"9"}`) {
		t.Errorf("the request names the job of the value given: %s", log)
	}

	undeclared := filepath.Join(t.TempDir(), "logs")
	refused := suiteProgram(t, undeclared, "--module-parameter", "limit=1")
	if refused.status != 2 {
		t.Errorf("status %d", refused.status)
	}
	if !strings.Contains(refused.err, "the suite declares no module parameter 'limit'") {
		t.Errorf("the refusal is on the standard error: %s", refused.err)
	}
	if _, err := os.Stat(undeclared); !os.IsNotExist(err) {
		t.Errorf("no case ran, so no log was written: %v", err)
	}
}

// §11.8: a world runs the test cases whose selection it satisfies, in the
// suite's order: `ptcs` alone selects the served PTC and not the removed one,
// and `guard` selects the guarded case.
func TestATestSuiteRunsThroughItsRunner_RunsTheTestCasesAWorldSelects(t *testing.T) {
	var requested []uint32
	ptcs := suiteNames(suiteRun(t, testprofile.FeaturesOf("ptcs"), 0, &requested, nil))
	if !reflect.DeepEqual(ptcs, []string{"answers_the_job", "serves_a_ptc"}) {
		t.Errorf("a world of ptcs runs %v", ptcs)
	}
	guard := suiteNames(suiteRun(t, testprofile.FeaturesOf("guard"), 0, &requested, nil))
	if !reflect.DeepEqual(guard, []string{"answers_the_job", "answers_before_the_guard"}) {
		t.Errorf("a world of guard runs %v", guard)
	}
}

// -1 §8.2.1, §11.5: a value the caller gives the module parameter replaces the
// suite's default, and the component asks for the job it names.
func TestATestSuiteRunsThroughItsRunner_AsksForTheJobAnOverrideOfItsModuleParameterNames(t *testing.T) {
	var requested []uint32
	overrides := &testprofile.ModuleParameters{}
	overrides.Set("job", sce.SavedWriteUint32(9))
	results := suiteRun(t, testprofile.FeaturesOf(), 0, &requested, overrides)
	if len(results) != 1 {
		t.Fatalf("the suite runs %v", suiteNames(results))
	}
	if results[0].Result.Verdict != testprofile.VerdictPass {
		t.Errorf("verdict %v: %v", results[0].Result.Verdict, results[0].Result.Log)
	}
	if !reflect.DeepEqual(requested, []uint32{9}) {
		t.Errorf("the component asks for the job the override names: %v", requested)
	}
}

// P13: a value of the module parameter that is not of the type the component
// declares ends the test case in error before the component starts.
func TestATestSuiteRunsThroughItsRunner_EndsInErrorWhenAModuleParameterIsNotOfItsType(t *testing.T) {
	var requested []uint32
	overrides := &testprofile.ModuleParameters{}
	overrides.Set("job", sce.SavedWriteString("seven"))
	results := suiteRun(t, testprofile.FeaturesOf(), 0, &requested, overrides)
	if len(results) != 1 {
		t.Fatalf("the suite runs %v", suiteNames(results))
	}
	if results[0].Result.Verdict != testprofile.VerdictError {
		t.Errorf("verdict %v: %v", results[0].Result.Verdict, results[0].Result.Log)
	}
	if !suiteLogged(results[0], testprofile.LogError) {
		t.Errorf("the refused value is logged as an error: %v", results[0].Result.Log)
	}
	if len(requested) != 0 {
		t.Errorf("the component did not start, so it sent nothing: %v", requested)
	}
}

// §11.6, §11.7: the configuration's tables name the module parameters its
// components read, which a suite that runs it must give, and the test case
// names itself and its configuration.
func TestATestSuiteRunsThroughItsRunner_ReadsTheModuleParametersItsConfigurationNames(t *testing.T) {
	if got := asking_configuration.ModuleParameters(); !reflect.DeepEqual(got, []string{"job"}) {
		t.Errorf("the configuration's components read %v", got)
	}
	if got := asking_configuration.Configuration().MTCType; got != "param_asker" {
		t.Errorf("the MTC's type is %q", got)
	}
	if asking_configuration.Machine("nobody") != nil {
		t.Errorf("a type the configuration does not name has no machine")
	}
	if asking_configuration.Machine("param_asker") == nil {
		t.Errorf("the MTC's type has its machine")
	}
	if answers_the_job.Name != "answers_the_job" {
		t.Errorf("the unit names its test case %q", answers_the_job.Name)
	}
	testCase := answers_the_job.TestCase()
	if testCase.Name != answers_the_job.Name {
		t.Errorf("the test case is named %q, not its unit's name", testCase.Name)
	}
	if !reflect.DeepEqual(testCase.Configuration, asking_configuration.Configuration()) {
		t.Errorf("the test case runs %v", testCase.Configuration)
	}
	if !reflect.DeepEqual(testCase.ModuleParameters, []string{"job"}) {
		t.Errorf("the test case reads %v", testCase.ModuleParameters)
	}
}
