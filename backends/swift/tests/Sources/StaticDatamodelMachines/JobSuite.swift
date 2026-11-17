// SCE-GENERATED — DO NOT EDIT
// source-hash: 4154e2a94e5761146caffd9e99419e570b7123aab1b11fb20a04685ab39515fd
// SCE-MAP: job_suite.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-suite")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

import SceRuntime
import SceTestProfile

/// The test suite `job_suite` (docs/SCE_TEST_PROFILE.md §11.8) as the runner reads it
/// (`SceTestProfile.runSuite`), the member of a caseless enum of the module as every unit's are.
public enum JobSuite {
    /// The suite: each module parameter at its default, held as the saved value of its declared
    /// type, and each `<sce:execute>` in run order, with its test case, its guard in microseconds
    /// and the selection that runs it in a world of the suite's features.
    public static func suite() -> SceTestProfile.Suite {
        var moduleParameters = SceTestProfile.ModuleParameters()
        moduleParameters.set("elapsed_limit", SceRuntime.SavedValues.of(500 as UInt32))
        moduleParameters.set("run_label", SceRuntime.SavedValues.of("nightly"))
        moduleParameters.set("key", SceRuntime.SavedValues.of([0x0a, 0xff] as [UInt8]))
        return SceTestProfile.Suite(
            name: "job_suite",
            moduleParameters: moduleParameters,
            executes: [
                SceTestProfile.Execute(
                    CompletesInTime.testCase(),
                    guardAt: 10000000,
                    selection: { world in SceTestProfile.enabled(world, "job-timeout") && SceTestProfile.enabled(world, "job-retry") }),
            ])
    }

    /// The value the runner is given for the module parameter `id` (docs/SCE_TTCN3_LANE.md §4,
    /// `--module-parameter id=value`), a literal of its declared type as the `value` of a
    /// `<sce:module-parameter>` writes one (docs/SCE_TEST_PROFILE.md §11.4), read as the saved value
    /// of that type.
    ///
    /// - Throws: `SceTestProfile.OverrideRefusal` naming why, where the suite declares no such
    ///   parameter or the literal is no value of its type.
    public static func moduleParameter(id: String, literal: String) throws(SceTestProfile.OverrideRefusal)
        -> SceTestProfile.ModuleParameterValue
    {
        switch id {
        case "elapsed_limit": return try SceRuntime.SavedValues.of(SceTestProfile.integerParameter(id, literal, UInt32.self))
        case "run_label": return SceTestProfile.textParameter(literal)
        case "key": return try SceTestProfile.bytesParameter(id, literal)
        default: throw SceTestProfile.undeclaredParameter(id)
        }
    }

    /// The runner program of the suite `job_suite` (docs/SCE_TTCN3_LANE.md §4): every test
    /// case the world of `--world` selects, run against its recorded input trace from
    /// `--trace-dir`, its canonical test log written to `--log-dir` (`SceTestProfile.runnerMain`).
    /// An executable target whose `main.swift` calls it is the program.
    public static func main() -> Never {
        SceTestProfile.runnerMain(suite: suite(), parameter: moduleParameter(id:literal:))
    }
}
