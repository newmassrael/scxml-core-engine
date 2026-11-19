// SCE-GENERATED — DO NOT EDIT
// source-hash: da8cfaa8149c596ce55c8e1b8c2853ee2c20bb8f1999394376d21de2c156738a
// source: takes_before_its_exits.scxml
// SCE-MAP: takes_before_its_exits.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

import SceTestProfile

/// The test case `takes_before_its_exits` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it, the
/// members of a caseless enum of the module as every unit's are.
public enum TakesBeforeItsExits {
    /// The test case's name in the suite and in the test log.
    public static let name: String = "takes_before_its_exits"

    /// The entry the runner reads: the test case's name, its configuration and the machines of its
    /// components, the module parameters those components read, and the reader of the inputs a
    /// recorded trace of it carries (docs/SCE_TTCN3_LANE.md §10.5).
    public static func testCase() -> SceTestProfile.TestCase {
        SceTestProfile.TestCase(
            name: Self.name,
            configuration: LeavingConfiguration.configuration(),
            machines: { componentType in LeavingConfiguration.machine(componentType) },
            moduleParameters: LeavingConfiguration.moduleParameters(),
            inputs: LeavingConfiguration.input(systemPort:typeName:value:))
    }
}
