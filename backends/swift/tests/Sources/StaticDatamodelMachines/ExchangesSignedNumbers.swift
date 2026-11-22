// SCE-GENERATED — DO NOT EDIT
// source-hash: 00c1f8c7bb3a656c3341b9eb282dcf73da0bff38f7ef65b9a28acd04193ec22b
// source: exchanges_signed_numbers.scxml
// SCE-MAP: exchanges_signed_numbers.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

import SceTestProfile

/// The test case `exchanges_signed_numbers` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it, the
/// members of a caseless enum of the module as every unit's are.
public enum ExchangesSignedNumbers {
    /// The test case's name in the suite and in the test log.
    public static let name: String = "exchanges_signed_numbers"

    /// The entry the runner reads: the test case's name, its configuration and the machines of its
    /// components, the module parameters those components read, and the reader of the inputs a
    /// recorded trace of it carries (docs/SCE_TTCN3_LANE.md §10.5).
    public static func testCase() -> SceTestProfile.TestCase {
        SceTestProfile.TestCase(
            name: Self.name,
            configuration: SignedConfiguration.configuration(),
            machines: { componentType in SignedConfiguration.machine(componentType) },
            moduleParameters: SignedConfiguration.moduleParameters(),
            inputs: SignedConfiguration.input(systemPort:typeName:value:))
    }
}
