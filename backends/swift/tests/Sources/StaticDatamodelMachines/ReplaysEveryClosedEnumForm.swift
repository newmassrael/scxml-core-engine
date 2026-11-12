// SCE-GENERATED — DO NOT EDIT
// source-hash: 75f53f6deefa1a9e19646e018ad72cacc30c4c7ca7f7f96f21be9f17f58cfdae
// SCE-MAP: replays_every_closed_enum_form.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

import SceTestProfile

/// The test case `replays_every_closed_enum_form` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it, the
/// members of a caseless enum of the module as every unit's are.
public enum ReplaysEveryClosedEnumForm {
    /// The test case's name in the suite and in the test log.
    public static let name: String = "replays_every_closed_enum_form"

    /// The entry the runner reads: the test case's name, its configuration and the machines of its
    /// components, the module parameters those components read, and the reader of the inputs a
    /// recorded trace of it carries (docs/SCE_TTCN3_LANE.md §10.5).
    public static func testCase() -> SceTestProfile.TestCase {
        SceTestProfile.TestCase(
            name: Self.name,
            configuration: ClosedEnumFormsConfiguration.configuration(),
            machines: { componentType in ClosedEnumFormsConfiguration.machine(componentType) },
            moduleParameters: ClosedEnumFormsConfiguration.moduleParameters(),
            inputs: ClosedEnumFormsConfiguration.input(systemPort:typeName:value:))
    }
}
