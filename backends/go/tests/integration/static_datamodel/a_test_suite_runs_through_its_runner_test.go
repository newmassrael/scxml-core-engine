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
// the system under test, is written here.
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
	"reflect"
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
