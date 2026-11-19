// SCE-GENERATED — DO NOT EDIT
// source-hash: 29930ac89dec3dc0e7e43a6f704b2e6cf68fc8fc08b99018af81358618b73767
// source: sends_to_a_removed_ptc.scxml
// SCE-MAP: sends_to_a_removed_ptc.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

import SceTestProfile

/// The test case `sends_to_a_removed_ptc` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it, the
/// members of a caseless enum of the module as every unit's are.
public enum SendsToARemovedPtc {
    /// The test case's name in the suite and in the test log.
    public static let name: String = "sends_to_a_removed_ptc"

    /// The entry the runner reads: the test case's name, its configuration and the machines of its
    /// components, the module parameters those components read, and the reader of the inputs a
    /// recorded trace of it carries (docs/SCE_TTCN3_LANE.md §10.5).
    public static func testCase() -> SceTestProfile.TestCase {
        SceTestProfile.TestCase(
            name: Self.name,
            configuration: RemovingConfiguration.configuration(),
            machines: { componentType in RemovingConfiguration.machine(componentType) },
            moduleParameters: RemovingConfiguration.moduleParameters(),
            inputs: RemovingConfiguration.input(systemPort:typeName:value:))
    }
}
