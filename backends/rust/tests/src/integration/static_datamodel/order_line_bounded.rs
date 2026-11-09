// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
#![doc = "SCE-MAP: order_line_bounded.scxml:5 :: _forge_body"]
// SCE-MAP: order_line_bounded.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

/// The type the template `order_line_bounded` matches: `record:OrderLine`, as the
/// unit of its type declares it.
#[allow(dead_code)]
pub type Value = super::order_line::OrderLinePayload;

/// Whether `value` matches the template `order_line_bounded` (ES 201 873-1 Annex B.1,
/// docs/SCE_TEST_PROFILE.md §6.1).
/// - `param_limit`: the formal parameter `limit`, of type `uint16`.
#[allow(dead_code, unused_variables)]
pub fn matches(value: &Value, param_limit: u16) -> bool {
    value.sku == "A-1"
        && super::quantity_at_most::matches(&value.quantity, param_limit)
        && value.color == super::paint_color::PaintColor::Blue
}

/// The name of the record `Value` is, as the types module of the suite's TTCN-3 export names it
/// (docs/SCE_TEST_PROFILE.md §9.5): the `type` of a message of it in the test log (§12.5).
#[allow(dead_code)]
pub const TYPE: &str = "order_line";

/// `value` in the canonical form of the test log (docs/SCE_TEST_PROFILE.md §12.5): each
/// integer as the text of its digits, a byte string in upper-case hex, an enum and a case
/// of a union by their declared names, an absent field omitted. A send of this template
/// carries it beside the value it sends (`sce_rust_runtime::test::message_value`).
#[allow(dead_code)]
pub fn canonical(value: &Value) -> ::sce_rust_runtime::json::Value {
    ::sce_rust_runtime::test::canonical::record([
        (
            "sku",
            Some(::sce_rust_runtime::test::canonical::text(
                value.sku.as_str(),
            )),
        ),
        (
            "quantity",
            Some(::sce_rust_runtime::test::canonical::integer(value.quantity)),
        ),
        (
            "color",
            Some(::sce_rust_runtime::test::canonical::enumerated(
                value.color.declared_name(),
            )),
        ),
    ])
}
