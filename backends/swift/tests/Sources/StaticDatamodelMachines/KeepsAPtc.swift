// SCE-GENERATED — DO NOT EDIT
// source-hash: 16d76c0f9ed42e7e13706b7e291287d0861ab6ac1014d873b4e04ff755d59e01
// SCE-MAP: keeps_a_ptc.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

import SceTestProfile

/// The test case `keeps_a_ptc` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it, the
/// members of a caseless enum of the module as every unit's are.
public enum KeepsAPtc {
    /// The test case's name in the suite and in the test log.
    public static let name: String = "keeps_a_ptc"

    /// The entry the runner reads: the test case's name, its configuration and the machines of its
    /// components, the module parameters those components read, and the reader of the inputs a
    /// recorded trace of it carries (docs/SCE_TTCN3_LANE.md §10.5).
    public static func testCase() -> SceTestProfile.TestCase {
        SceTestProfile.TestCase(
            name: Self.name,
            configuration: KeepingConfiguration.configuration(),
            machines: { componentType in KeepingConfiguration.machine(componentType) },
            moduleParameters: KeepingConfiguration.moduleParameters(),
            inputs: KeepingConfiguration.input(systemPort:typeName:value:))
    }
}
