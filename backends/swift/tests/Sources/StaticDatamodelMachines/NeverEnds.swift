// SCE-GENERATED — DO NOT EDIT
// source-hash: 86c6dd30cc120de03266ccea6cca0d437b9db87ab00fd8a8b7241bb2567cde50
// source: never_ends.scxml
// SCE-MAP: never_ends.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

import SceTestProfile

/// The test case `never_ends` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it, the
/// members of a caseless enum of the module as every unit's are.
public enum NeverEnds {
    /// The test case's name in the suite and in the test log.
    public static let name: String = "never_ends"

    /// The entry the runner reads: the test case's name, its configuration and the machines of its
    /// components, the module parameters those components read, and the reader of the inputs a
    /// recorded trace of it carries (docs/SCE_TTCN3_LANE.md §10.5).
    public static func testCase() -> SceTestProfile.TestCase {
        SceTestProfile.TestCase(
            name: Self.name,
            configuration: StalledConfiguration.configuration(),
            machines: { componentType in StalledConfiguration.machine(componentType) },
            moduleParameters: StalledConfiguration.moduleParameters(),
            inputs: StalledConfiguration.input(systemPort:typeName:value:))
    }
}
