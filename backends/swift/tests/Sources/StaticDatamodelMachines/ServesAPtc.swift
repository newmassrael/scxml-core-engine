// SCE-GENERATED — DO NOT EDIT
// source-hash: 3017c13dd7240e1aba98f6a1abbd115bc5adb7716a46dd11821c4aedd3da1f9b
// source: serves_a_ptc.scxml
// SCE-MAP: serves_a_ptc.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

import SceTestProfile

/// The test case `serves_a_ptc` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it, the
/// members of a caseless enum of the module as every unit's are.
public enum ServesAPtc {
    /// The test case's name in the suite and in the test log.
    public static let name: String = "serves_a_ptc"

    /// The entry the runner reads: the test case's name, its configuration and the machines of its
    /// components, the module parameters those components read, and the reader of the inputs a
    /// recorded trace of it carries (docs/SCE_TTCN3_LANE.md §10.5).
    public static func testCase() -> SceTestProfile.TestCase {
        SceTestProfile.TestCase(
            name: Self.name,
            configuration: ServingConfiguration.configuration(),
            machines: { componentType in ServingConfiguration.machine(componentType) },
            moduleParameters: ServingConfiguration.moduleParameters(),
            inputs: ServingConfiguration.input(systemPort:typeName:value:))
    }
}
