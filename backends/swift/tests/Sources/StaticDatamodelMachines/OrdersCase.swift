// SCE-GENERATED — DO NOT EDIT
// source-hash: 12a7dc72335a819ac5590a1b6b46aee0f5b664147671271baaaa11524ee4c7b9
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
