// SCE-GENERATED — DO NOT EDIT
// source-hash: f013d3b0b2a48f3e291b7f6d7530af9e24c9739f646741af3fa82d15412d15d9
// source: nested_suite.scxml
// SCE-MAP: nested_suite.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-suite")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package nested_suite

import (
	"github.com/newmassrael/sce-go-runtime/testprofile"
	sce_nested_case "github.com/newmassrael/sce-go-tests/integration/static_datamodel/nested_case"
)

// Suite is the test suite `nested_suite` (docs/SCE_TEST_PROFILE.md §11.8) as the runner reads
// it (testprofile.RunSuite): each module parameter at its default, held as the saved value of its
// declared type, and each `<sce:execute>` in run order, with its test case, its guard and the
// selection that runs it in a world of the suite's features.
func Suite() testprofile.Suite {
	moduleParameters := &testprofile.ModuleParameters{}
	return testprofile.Suite{
		Name:             "nested_suite",
		ModuleParameters: moduleParameters,
		Executes: []testprofile.Execute{
			{
				TestCase:  sce_nested_case.TestCase(),
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
func ModuleParameter(id, _ string) (testprofile.ModuleParameterValue, error) {
	return nil, testprofile.UndeclaredParameter(id)
}

// Main is the runner program of the suite `nested_suite` (docs/SCE_TTCN3_LANE.md §4): every test
// case the world of `--world` selects, run against its recorded input trace from `--trace-dir`, its
// canonical test log written to `--log-dir` (testprofile.RunnerMain), after which the process exits.
// A module makes it a program with a `package main` whose `main` calls it.
func Main() {
	testprofile.RunnerMain(Suite(), ModuleParameter)
}
