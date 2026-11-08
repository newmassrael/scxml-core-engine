// SCE-GENERATED — DO NOT EDIT
// source-hash: 011d804f887126b888ea6ba5b4213d56e0f5c8740555a154c83e9b8d0105c4b6
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
    /// components, and the module parameters those components read.
    public static func testCase() -> SceTestProfile.TestCase {
        SceTestProfile.TestCase(
            name: Self.name,
            configuration: ServingConfiguration.configuration(),
            machines: { componentType in ServingConfiguration.machine(componentType) },
            moduleParameters: ServingConfiguration.moduleParameters())
    }
}
