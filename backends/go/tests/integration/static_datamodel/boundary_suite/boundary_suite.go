// SCE-GENERATED — DO NOT EDIT
// source-hash: 982a23a5ec15b8216f29e9e8e4ba6c1047dee0d66671c7835c2776a4de5a13d2
// SCE-MAP: boundary_suite.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-suite")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package boundary_suite

import (
	sce "github.com/newmassrael/sce-go-runtime"
	"github.com/newmassrael/sce-go-runtime/testprofile"
	sce_exchanges_signed_numbers "github.com/newmassrael/sce-go-tests/integration/static_datamodel/exchanges_signed_numbers"
	sce_answers_another_job "github.com/newmassrael/sce-go-tests/integration/static_datamodel/answers_another_job"
	sce_sets_no_verdict "github.com/newmassrael/sce-go-tests/integration/static_datamodel/sets_no_verdict"
	sce_never_ends "github.com/newmassrael/sce-go-tests/integration/static_datamodel/never_ends"
	sce_sends_to_a_removed_ptc "github.com/newmassrael/sce-go-tests/integration/static_datamodel/sends_to_a_removed_ptc"
	sce_takes_before_its_exits "github.com/newmassrael/sce-go-tests/integration/static_datamodel/takes_before_its_exits"
)

// Suite is the test suite `boundary_suite` (docs/SCE_TEST_PROFILE.md §11.8) as the runner reads
// it (testprofile.RunSuite): each module parameter at its default, held as the saved value of its
// declared type, and each `<sce:execute>` in run order, with its test case, its guard and the
// selection that runs it in a world of the suite's features.
func Suite() testprofile.Suite {
	moduleParameters := &testprofile.ModuleParameters{}
	moduleParameters.Set("job", sce.SavedWriteUint32(7))
	return testprofile.Suite{
		Name:             "boundary_suite",
		ModuleParameters: moduleParameters,
		Executes: []testprofile.Execute{
			{
				TestCase:  sce_exchanges_signed_numbers.TestCase(),
				Guard:     testprofile.GuardOf(5000000),
				Selection: nil,
			},
			{
				TestCase:  sce_answers_another_job.TestCase(),
				Guard:     testprofile.GuardOf(5000000),
				Selection: nil,
			},
			{
				TestCase:  sce_sets_no_verdict.TestCase(),
				Guard:     testprofile.NoGuard,
				Selection: nil,
			},
			{
				TestCase:  sce_never_ends.TestCase(),
				Guard:     testprofile.NoGuard,
				Selection: nil,
			},
			{
				TestCase:  sce_sends_to_a_removed_ptc.TestCase(),
				Guard:     testprofile.NoGuard,
				Selection: nil,
			},
			{
				TestCase:  sce_takes_before_its_exits.TestCase(),
				Guard:     testprofile.GuardOf(5000000),
				Selection: nil,
			},
		},
	}
}

// ModuleParameter is the value the runner is given for the module parameter id
// (docs/SCE_TTCN3_LANE.md §4, `--module-parameter id=value`), a literal of its declared type as the
// `value` of a `<sce:module-parameter>` writes one (docs/SCE_TEST_PROFILE.md §11.4), read as the
// saved value of that type; an error, naming why, where the suite declares no such parameter or the
// literal is no value of its type. It is a testprofile.ParameterReader.
func ModuleParameter(id, literal string) (testprofile.ModuleParameterValue, error) {
	switch id {
	case "job":
		return testprofile.IntegerParameter(id, literal, sce.SavedWriteUint32)
	}
	return nil, testprofile.UndeclaredParameter(id)
}

// Main is the runner program of the suite `boundary_suite` (docs/SCE_TTCN3_LANE.md §4): every test
// case the world of `--world` selects, run against its recorded input trace from `--trace-dir`, its
// canonical test log written to `--log-dir` (testprofile.RunnerMain), after which the process exits.
// A module makes it a program with a `package main` whose `main` calls it.
func Main() {
	testprofile.RunnerMain(Suite(), ModuleParameter)
}
