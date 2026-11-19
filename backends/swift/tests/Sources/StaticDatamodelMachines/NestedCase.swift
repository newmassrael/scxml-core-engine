// SCE-GENERATED — DO NOT EDIT
// source-hash: 1e281a8396692195e709bec100279034d644b5bab0a3c6d9b461fbae9b46fce6
// source: nested_case.scxml
// SCE-MAP: nested_case.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

import SceTestProfile

/// The test case `nested_case` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it, the
/// members of a caseless enum of the module as every unit's are.
public enum NestedCase {
    /// The test case's name in the suite and in the test log.
    public static let name: String = "nested_case"

    /// The entry the runner reads: the test case's name, its configuration and the machines of its
    /// components, the module parameters those components read, and the reader of the inputs a
    /// recorded trace of it carries (docs/SCE_TTCN3_LANE.md §10.5).
    public static func testCase() -> SceTestProfile.TestCase {
        SceTestProfile.TestCase(
            name: Self.name,
            configuration: NestedConfiguration.configuration(),
            machines: { componentType in NestedConfiguration.machine(componentType) },
            moduleParameters: NestedConfiguration.moduleParameters(),
            inputs: NestedConfiguration.input(systemPort:typeName:value:))
    }
}
