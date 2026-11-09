// SCE-GENERATED — DO NOT EDIT
// source-hash: 16d76c0f9ed42e7e13706b7e291287d0861ab6ac1014d873b4e04ff755d59e01
// SCE-MAP: runtime_suite.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-suite")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

import SceRuntime
import SceTestProfile

/// The test suite `runtime_suite` (docs/SCE_TEST_PROFILE.md §11.8) as the runner reads it
/// (`SceTestProfile.runSuite`), the member of a caseless enum of the module as every unit's are.
public enum RuntimeSuite {
    /// The suite: each module parameter at its default, held as the saved value of its declared
    /// type, and each `<sce:execute>` in run order, with its test case, its guard in microseconds
    /// and the selection that runs it in a world of the suite's features.
    public static func suite() -> SceTestProfile.Suite {
        var moduleParameters = SceTestProfile.ModuleParameters()
        moduleParameters.set("job", SceRuntime.SavedValues.of(7 as UInt32))
        return SceTestProfile.Suite(
            name: "runtime_suite",
            moduleParameters: moduleParameters,
            executes: [
                SceTestProfile.Execute(
                    AnswersTheJob.testCase(),
                    guardAt: 10000000,
                    selection: nil),
                SceTestProfile.Execute(
                    ServesAPtc.testCase(),
                    guardAt: nil,
                    selection: { world in SceTestProfile.enabled(world, "ptcs") }),
                SceTestProfile.Execute(
                    KeepsAPtc.testCase(),
                    guardAt: nil,
                    selection: { world in SceTestProfile.enabled(world, "ptcs") && SceTestProfile.enabled(world, "removal") }),
                SceTestProfile.Execute(
                    AnswersBeforeTheGuard.testCase(),
                    guardAt: 100,
                    selection: { world in SceTestProfile.enabled(world, "guard") || SceTestProfile.enabled(world, "removal") }),
            ])
    }
}
