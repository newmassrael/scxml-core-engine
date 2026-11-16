// SCE-GENERATED — DO NOT EDIT
// source-hash: 982a23a5ec15b8216f29e9e8e4ba6c1047dee0d66671c7835c2776a4de5a13d2
// SCE-MAP: sets_no_verdict.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

import SceTestProfile

/// The test case `sets_no_verdict` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it, the
/// members of a caseless enum of the module as every unit's are.
public enum SetsNoVerdict {
    /// The test case's name in the suite and in the test log.
    public static let name: String = "sets_no_verdict"

    /// The entry the runner reads: the test case's name, its configuration and the machines of its
    /// components, the module parameters those components read, and the reader of the inputs a
    /// recorded trace of it carries (docs/SCE_TTCN3_LANE.md §10.5).
    public static func testCase() -> SceTestProfile.TestCase {
        SceTestProfile.TestCase(
            name: Self.name,
            configuration: UnsetConfiguration.configuration(),
            machines: { componentType in UnsetConfiguration.machine(componentType) },
            moduleParameters: UnsetConfiguration.moduleParameters(),
            inputs: UnsetConfiguration.input(systemPort:typeName:value:))
    }
}
