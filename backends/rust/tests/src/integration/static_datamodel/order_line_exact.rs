// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
#![doc = "SCE-MAP: order_line_exact.scxml:4 :: _forge_body"]
// SCE-MAP: order_line_exact.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

/// The type the template `order_line_exact` matches: `record:OrderLine`, as the
/// unit of its type declares it.
#[allow(dead_code)]
pub type Value = super::order_line::OrderLinePayload;

/// Whether `value` matches the template `order_line_exact` (ES 201 873-1 Annex B.1,
/// docs/SCE_TEST_PROFILE.md §6.1).
#[allow(dead_code, unused_variables)]
pub fn matches(value: &Value) -> bool {
    value.sku == "A-1"
        && super::quantity_two::matches(&value.quantity)
        && value.color == super::paint_color::PaintColor::Red
}

/// The value the specific template `order_line_exact` sends (docs/SCE_TEST_PROFILE.md
/// §6.2): every field it names at its value, every field it omits absent. It matches the
/// template by `matches`, given the same parameters.
#[allow(dead_code, unused_variables)]
pub fn value() -> Value {
    super::order_line::OrderLinePayload {
        sku: ::sce_portable_bytes::sce_text!(16, "A-1"),
        quantity: super::quantity_two::value(),
        color: super::paint_color::PaintColor::Red,
    }
}
