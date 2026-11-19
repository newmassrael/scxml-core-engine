// SCE-GENERATED — DO NOT EDIT
// source-hash: 7abea7973bcdc2d068064b2dd6062cfe96348ca03b00db9f639c12ef4c2a89b3
// source: answers_another_job.scxml
// SCE-MAP: answers_another_job.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

import SceTestProfile

/// The test case `answers_another_job` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it, the
/// members of a caseless enum of the module as every unit's are.
public enum AnswersAnotherJob {
    /// The test case's name in the suite and in the test log.
    public static let name: String = "answers_another_job"

    /// The entry the runner reads: the test case's name, its configuration and the machines of its
    /// components, the module parameters those components read, and the reader of the inputs a
    /// recorded trace of it carries (docs/SCE_TTCN3_LANE.md §10.5).
    public static func testCase() -> SceTestProfile.TestCase {
        SceTestProfile.TestCase(
            name: Self.name,
            configuration: AskingConfiguration.configuration(),
            machines: { componentType in AskingConfiguration.machine(componentType) },
            moduleParameters: AskingConfiguration.moduleParameters(),
            inputs: AskingConfiguration.input(systemPort:typeName:value:))
    }
}
