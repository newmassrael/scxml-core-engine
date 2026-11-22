// SCE-GENERATED — DO NOT EDIT
// source-hash: 024765d6c5a9465ebcc4b76c5357cb34091f80825e777a07d3bf66603cfc0a59
// source: boundary_suite.scxml
// SCE-MAP: boundary_suite.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-suite")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

import SceRuntime
import SceTestProfile

/// The test suite `boundary_suite` (docs/SCE_TEST_PROFILE.md §11.8) as the runner reads it
/// (`SceTestProfile.runSuite`), the member of a caseless enum of the module as every unit's are.
public enum BoundarySuite {
    /// The suite: each module parameter at its default, held as the saved value of its declared
    /// type, and each `<sce:execute>` in run order, with its test case, its guard in microseconds
    /// and the selection that runs it in a world of the suite's features.
    public static func suite() -> SceTestProfile.Suite {
        var moduleParameters = SceTestProfile.ModuleParameters()
        moduleParameters.set("job", SceRuntime.SavedValues.of(7 as UInt32))
        return SceTestProfile.Suite(
            name: "boundary_suite",
            moduleParameters: moduleParameters,
            executes: [
                SceTestProfile.Execute(
                    ExchangesSignedNumbers.testCase(),
                    guardAt: 5000000,
                    selection: nil),
                SceTestProfile.Execute(
                    AnswersAnotherJob.testCase(),
                    guardAt: 5000000,
                    selection: nil),
                SceTestProfile.Execute(
                    SetsNoVerdict.testCase(),
                    guardAt: nil,
                    selection: nil),
                SceTestProfile.Execute(
                    NeverEnds.testCase(),
                    guardAt: nil,
                    selection: nil),
                SceTestProfile.Execute(
                    SendsToARemovedPtc.testCase(),
                    guardAt: nil,
                    selection: nil),
                SceTestProfile.Execute(
                    TakesBeforeItsExits.testCase(),
                    guardAt: 5000000,
                    selection: nil),
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
        case "job": return try SceRuntime.SavedValues.of(SceTestProfile.integerParameter(id, literal, UInt32.self))
        default: throw SceTestProfile.undeclaredParameter(id)
        }
    }

    /// The runner program of the suite `boundary_suite` (docs/SCE_TTCN3_LANE.md §4): every test
    /// case the world of `--world` selects, run against its recorded input trace from
    /// `--trace-dir`, its canonical test log written to `--log-dir` (`SceTestProfile.runnerMain`).
    /// An executable target whose `main.swift` calls it is the program.
    public static func main() -> Never {
        SceTestProfile.runnerMain(suite: suite(), parameter: moduleParameter(id:literal:))
    }
}
