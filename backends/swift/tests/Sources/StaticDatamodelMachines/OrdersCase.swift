// SCE-GENERATED — DO NOT EDIT
// source-hash: be94512ba39db699a331b27cc31692bda58f20eb3a2d7dac3ed4ad84dab78fc5
// source: orders_case.scxml
// SCE-MAP: orders_case.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-case")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

import SceTestProfile

/// The test case `orders_case` (docs/SCE_TEST_PROFILE.md §11.7) as the runner reads it, the
/// members of a caseless enum of the module as every unit's are.
public enum OrdersCase {
    /// The test case's name in the suite and in the test log.
    public static let name: String = "orders_case"

    /// The entry the runner reads: the test case's name, its configuration and the machines of its
    /// components, the module parameters those components read, and the reader of the inputs a
    /// recorded trace of it carries (docs/SCE_TTCN3_LANE.md §10.5).
    public static func testCase() -> SceTestProfile.TestCase {
        SceTestProfile.TestCase(
            name: Self.name,
            configuration: OrdersConfiguration.configuration(),
            machines: { componentType in OrdersConfiguration.machine(componentType) },
            moduleParameters: OrdersConfiguration.moduleParameters(),
            inputs: OrdersConfiguration.input(systemPort:typeName:value:))
    }
}
