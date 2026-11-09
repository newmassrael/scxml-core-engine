// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
// SCE-MAP: order_placed_match.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

import SceRuntime
import SceTestProfile

/// The test template `order_placed_match` (docs/SCE_TEST_PROFILE.md §11.4): a caseless enum of
/// the module, as an algorithm's unit is, so the `Value`, `matches` and `value` of two templates
/// never share a name, and a template another one references is called through its enum.
public enum OrderPlacedMatch {
    /// The type the template matches: `record:OrderPlaced`, as the unit of its type declares it.
    public typealias Value = OrderPlacedPayload

    /// Whether `value` matches the template (ES 201 873-1 Annex B.1,
    /// docs/SCE_TEST_PROFILE.md §6.1).
    ///
    /// `param_limit` is the formal parameter `limit`, of type `uint16`.
    public static func matches(_ value: Value, _ param_limit: UInt16) -> Bool {
        return value.lines.count == 2 && value.lines[0].sku == "A-1" && QuantityAtMost.matches(value.lines[0].quantity, param_limit) && value.lines[0].color == PaintColor.green && OrderLineExact.matches(value.lines[1]) && value.lines.count <= 4 && (value.note == nil || (value.note != nil && value.note.map { present_0 -> Bool in SceTestProfile.textLength(present_0) <= 8 } ?? true)) && (value.priority == nil || value.priority.map { present_0 -> Bool in present_0 >= 1 && present_0 <= 5 } ?? false) && value.customer.vip && (value.payment.kind == PaymentKind.card && (Optional(value.payment.provenCard).map { case_0 -> Bool in SceTestProfile.textLength(case_0.last4) >= 4 && case_0.amount >= 1 } ?? false) || value.payment.kind == PaymentKind.cash)
    }

    /// The name of the record `Value` is, as the types module of the suite's TTCN-3 export names it
    /// (docs/SCE_TEST_PROFILE.md §9.5): the `type` of a message of it in the test log (§12.5).
    public static let TYPE: String = "order_placed"

    /// `value` in the canonical form of the test log (docs/SCE_TEST_PROFILE.md §12.5): each integer
    /// as the text of its digits, a byte string in upper-case hex, an enum and a case of a union by
    /// their declared names, an absent field omitted. A send of this template carries it beside the
    /// value it sends (`SceTestProfile.messageValue`).
    public static func canonical(_ value: Value) -> SceRuntime.JSONValue {
        return SceTestProfile.Canonical.record(("order_id", SceTestProfile.Canonical.integer(value.order_id)), ("lines", SceTestProfile.Canonical.list(value.lines.map { item_0 -> SceRuntime.JSONValue in SceTestProfile.Canonical.record(("sku", SceTestProfile.Canonical.text(item_0.sku)), ("quantity", SceTestProfile.Canonical.integer(item_0.quantity)), ("color", SceTestProfile.Canonical.enumerated(item_0.color.declaredName))) })), ("note", value.note.map { present_0 -> SceRuntime.JSONValue in SceTestProfile.Canonical.text(present_0) }), ("priority", value.priority.map { present_0 -> SceRuntime.JSONValue in SceTestProfile.Canonical.integer(present_0) }), ("customer", SceTestProfile.Canonical.record(("full_name", SceTestProfile.Canonical.text(value.customer.full_name)), ("vip", SceTestProfile.Canonical.boolean(value.customer.vip)))), ("payment", { () -> SceRuntime.JSONValue in switch value.payment { case OrderPayment.card(let case_0): return SceTestProfile.Canonical.unionCase("card", SceTestProfile.Canonical.record(("last4", SceTestProfile.Canonical.text(case_0.last4)), ("amount", SceTestProfile.Canonical.integer(case_0.amount)))); case OrderPayment.cash: return SceTestProfile.Canonical.unionCase("cash", nil); case OrderPayment.voucher(let case_0): return SceTestProfile.Canonical.unionCase("voucher", SceTestProfile.Canonical.record(("voucher_code", SceTestProfile.Canonical.bytes(case_0.voucher_code)))) } }()))
    }
}
