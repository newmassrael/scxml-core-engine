// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
// SCE-MAP: order_line_bounded.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

/// The test template `order_line_bounded` (docs/SCE_TEST_PROFILE.md §11.4): a caseless enum of
/// the module, as an algorithm's unit is, so the `Value`, `matches` and `value` of two templates
/// never share a name, and a template another one references is called through its enum.
public enum OrderLineBounded {
    /// The type the template matches: `record:OrderLine`, as the unit of its type declares it.
    public typealias Value = OrderLinePayload

    /// Whether `value` matches the template (ES 201 873-1 Annex B.1,
    /// docs/SCE_TEST_PROFILE.md §6.1).
    ///
    /// `param_limit` is the formal parameter `limit`, of type `uint16`.
    public static func matches(_ value: Value, _ param_limit: UInt16) -> Bool {
        return value.sku == "A-1" && QuantityAtMost.matches(value.quantity, param_limit) && value.color == PaintColor.blue
    }
}
