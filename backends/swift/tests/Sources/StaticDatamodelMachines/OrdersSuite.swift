// SCE-GENERATED — DO NOT EDIT
// source-hash: 12a7dc72335a819ac5590a1b6b46aee0f5b664147671271baaaa11524ee4c7b9
// SCE-MAP: orders_suite.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-suite")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

import SceTestProfile

/// The test suite `orders_suite` (docs/SCE_TEST_PROFILE.md §11.8) as the runner reads it
/// (`SceTestProfile.runSuite`), the member of a caseless enum of the module as every unit's are.
public enum OrdersSuite {
    /// The suite: each module parameter at its default, held as the saved value of its declared
    /// type, and each `<sce:execute>` in run order, with its test case, its guard in microseconds
    /// and the selection that runs it in a world of the suite's features.
    public static func suite() -> SceTestProfile.Suite {
        let moduleParameters = SceTestProfile.ModuleParameters()
        return SceTestProfile.Suite(
            name: "orders_suite",
            moduleParameters: moduleParameters,
            executes: [
                SceTestProfile.Execute(
                    OrdersCase.testCase(),
                    guardAt: 1000000,
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
        throw SceTestProfile.undeclaredParameter(id)
    }

    /// The runner program of the suite `orders_suite` (docs/SCE_TTCN3_LANE.md §4): every test
    /// case the world of `--world` selects, run against its recorded input trace from
    /// `--trace-dir`, its canonical test log written to `--log-dir` (`SceTestProfile.runnerMain`).
    /// An executable target whose `main.swift` calls it is the program.
    public static func main() -> Never {
        SceTestProfile.runnerMain(suite: suite(), parameter: moduleParameter(id:literal:))
    }
}
