// SCE-GENERATED — DO NOT EDIT
// source-hash: 011d804f887126b888ea6ba5b4213d56e0f5c8740555a154c83e9b8d0105c4b6
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
