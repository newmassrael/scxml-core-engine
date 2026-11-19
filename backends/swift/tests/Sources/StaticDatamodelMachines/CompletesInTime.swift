// SCE-GENERATED — DO NOT EDIT
// source-hash: 5610f916540cc7a66f5ffd5f3c581afae0c543595d6e705745f49fe56e124377
// source: completes_in_time.scxml
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
