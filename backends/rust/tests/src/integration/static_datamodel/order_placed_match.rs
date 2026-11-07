// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
#![doc = "SCE-MAP: order_placed_match.scxml:7 :: _forge_body"]
// SCE-MAP: order_placed_match.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

/// The type the template `order_placed_match` matches: `record:OrderPlaced`, as the
/// unit of its type declares it.
#[allow(dead_code)]
pub type Value = super::order_placed::OrderPlacedPayload;

/// Whether `value` matches the template `order_placed_match` (ES 201 873-1 Annex B.1,
/// docs/SCE_TEST_PROFILE.md §6.1).
/// - `param_limit`: the formal parameter `limit`, of type `uint16`.
#[allow(dead_code, unused_variables)]
pub fn matches(value: &Value, param_limit: u16) -> bool {
    value.lines.len() == 2
        && value.lines[0].sku == "A-1"
        && super::quantity_at_most::matches(&value.lines[0].quantity, param_limit)
        && value.lines[0].color == super::paint_color::PaintColor::Green
        && super::order_line_exact::matches(&value.lines[1])
        && value.lines.len() <= 4
        && (value.note.is_none()
            || (value.note.is_some()
                && (match &value.note {
                    Some(present_0) => present_0.chars().count() <= 8,
                    None => true,
                })))
        && (value.priority.is_none()
            || (match &value.priority {
                Some(present_0) => *present_0 >= 1 && *present_0 <= 5,
                None => false,
            }))
        && value.customer.vip
        && (matches!(&value.payment, super::order_payment::OrderPayment::Card(case_0) if case_0.last4.chars().count() >= 4 && case_0.amount >= 1)
            || matches!(&value.payment, super::order_payment::OrderPayment::Cash))
}
