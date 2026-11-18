// SCE-GENERATED — DO NOT EDIT
// source-hash: 4fc3202cd282b89817f9b6bc17d27c57e6f71fb81651493178a84da56c1cae6a
// SCE-MAP: job_suite.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-suite")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

package job_suite

import (
	sce "github.com/newmassrael/sce-go-runtime"
	"github.com/newmassrael/sce-go-runtime/testprofile"
	sce_completes_in_time "github.com/newmassrael/sce-go-tests/integration/static_datamodel/completes_in_time"
)

// Suite is the test suite `job_suite` (docs/SCE_TEST_PROFILE.md §11.8) as the runner reads
// it (testprofile.RunSuite): each module parameter at its default, held as the saved value of its
// declared type, and each `<sce:execute>` in run order, with its test case, its guard and the
// selection that runs it in a world of the suite's features.
func Suite() testprofile.Suite {
	moduleParameters := &testprofile.ModuleParameters{}
	moduleParameters.Set("elapsed_limit", sce.SavedWriteUint32(500))
	moduleParameters.Set("run_label", sce.SavedWriteString("nightly"))
	moduleParameters.Set("key", sce.SavedWriteBytes([]byte{0x0a, 0xff}))
	return testprofile.Suite{
		Name:             "job_suite",
		ModuleParameters: moduleParameters,
		Executes: []testprofile.Execute{
			{
				TestCase:  sce_completes_in_time.TestCase(),
				Guard:     testprofile.GuardOf(10000000),
				Selection: func(world testprofile.Features) bool { return testprofile.Enabled(world, "job-timeout") && testprofile.Enabled(world, "job-retry") },
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
	case "elapsed_limit":
		return testprofile.IntegerParameter(id, literal, sce.SavedWriteUint32)
	case "run_label":
		return testprofile.TextParameter(literal)
	case "key":
		return testprofile.BytesParameter(id, literal)
	}
	return nil, testprofile.UndeclaredParameter(id)
}

// Main is the runner program of the suite `job_suite` (docs/SCE_TTCN3_LANE.md §4): every test
// case the world of `--world` selects, run against its recorded input trace from `--trace-dir`, its
// canonical test log written to `--log-dir` (testprofile.RunnerMain), after which the process exits.
// A module makes it a program with a `package main` whose `main` calls it.
func Main() {
	testprofile.RunnerMain(Suite(), ModuleParameter)
}
