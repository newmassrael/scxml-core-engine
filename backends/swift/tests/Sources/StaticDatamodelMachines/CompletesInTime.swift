// SCE-GENERATED — DO NOT EDIT
// source-hash: 4154e2a94e5761146caffd9e99419e570b7123aab1b11fb20a04685ab39515fd
// SCE-MAP: completes_in_time.scxml:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

import SceTestProfile

/// The test case `completes_in_time` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it, the
/// members of a caseless enum of the module as every unit's are.
public enum CompletesInTime {
    /// The test case's name in the suite and in the test log.
    public static let name: String = "completes_in_time"

    /// The entry the runner reads: the test case's name, its configuration and the machines of its
    /// components, the module parameters those components read, and the reader of the inputs a
    /// recorded trace of it carries (docs/SCE_TTCN3_LANE.md §10.5).
    public static func testCase() -> SceTestProfile.TestCase {
        SceTestProfile.TestCase(
            name: Self.name,
            configuration: TesterAndHelper.configuration(),
            machines: { componentType in TesterAndHelper.machine(componentType) },
            moduleParameters: TesterAndHelper.moduleParameters(),
            inputs: TesterAndHelper.input(systemPort:typeName:value:))
    }
}
