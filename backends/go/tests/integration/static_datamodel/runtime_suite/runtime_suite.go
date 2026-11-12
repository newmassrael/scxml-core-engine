// SCE-GENERATED — DO NOT EDIT
// source-hash: 16d76c0f9ed42e7e13706b7e291287d0861ab6ac1014d873b4e04ff755d59e01
// SCE-MAP: runtime_suite.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-suite")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package runtime_suite

import (
	sce "github.com/newmassrael/sce-go-runtime"
	"github.com/newmassrael/sce-go-runtime/testprofile"
	sce_answers_the_job "github.com/newmassrael/sce-go-tests/integration/static_datamodel/answers_the_job"
	sce_serves_a_ptc "github.com/newmassrael/sce-go-tests/integration/static_datamodel/serves_a_ptc"
	sce_keeps_a_ptc "github.com/newmassrael/sce-go-tests/integration/static_datamodel/keeps_a_ptc"
	sce_answers_before_the_guard "github.com/newmassrael/sce-go-tests/integration/static_datamodel/answers_before_the_guard"
)

// Suite is the test suite `runtime_suite` (docs/SCE_TEST_PROFILE.md §11.8) as the runner reads
// it (testprofile.RunSuite): each module parameter at its default, held as the saved value of its
// declared type, and each `<sce:execute>` in run order, with its test case, its guard and the
// selection that runs it in a world of the suite's features.
func Suite() testprofile.Suite {
	moduleParameters := &testprofile.ModuleParameters{}
	moduleParameters.Set("job", sce.SavedWriteUint32(7))
	return testprofile.Suite{
		Name:             "runtime_suite",
		ModuleParameters: moduleParameters,
		Executes: []testprofile.Execute{
			{
				TestCase:  sce_answers_the_job.TestCase(),
				Guard:     testprofile.GuardOf(10000000),
				Selection: nil,
			},
			{
				TestCase:  sce_serves_a_ptc.TestCase(),
				Guard:     testprofile.NoGuard,
				Selection: func(world testprofile.Features) bool { return testprofile.Enabled(world, "ptcs") },
			},
			{
				TestCase:  sce_keeps_a_ptc.TestCase(),
				Guard:     testprofile.NoGuard,
				Selection: func(world testprofile.Features) bool { return testprofile.Enabled(world, "ptcs") && testprofile.Enabled(world, "removal") },
			},
			{
				TestCase:  sce_answers_before_the_guard.TestCase(),
				Guard:     testprofile.GuardOf(100),
				Selection: func(world testprofile.Features) bool { return testprofile.Enabled(world, "guard") || testprofile.Enabled(world, "removal") },
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

// Main is the runner program of the suite `runtime_suite` (docs/SCE_TTCN3_LANE.md §4): every test
// case the world of `--world` selects, run against its recorded input trace from `--trace-dir`, its
// canonical test log written to `--log-dir` (testprofile.RunnerMain), after which the process exits.
// A module makes it a program with a `package main` whose `main` calls it.
func Main() {
	testprofile.RunnerMain(Suite(), ModuleParameter)
}
