// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
#![doc = "SCE-MAP: order_placed_match.scxml:7 :: _forge_body"]
// SCE-MAP: order_placed_match.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
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

/// The name of the record `Value` is, as the types module of the suite's TTCN-3 export names it
/// (docs/SCE_TEST_PROFILE.md §9.5): the `type` of a message of it in the test log (§12.5).
#[allow(dead_code)]
pub const TYPE: &str = "order_placed";

/// `value` in the canonical form of the test log (docs/SCE_TEST_PROFILE.md §12.5): each
/// integer as the text of its digits, a byte string in upper-case hex, an enum and a case
/// of a union by their declared names, an absent field omitted. A send of this template
/// carries it beside the value it sends (`sce_rust_runtime::test::message_value`).
#[allow(dead_code)]
pub fn canonical(value: &Value) -> ::sce_rust_runtime::json::Value {
    ::sce_rust_runtime::test::canonical::record([
        (
            "order_id",
            Some(::sce_rust_runtime::test::canonical::integer(value.order_id)),
        ),
        (
            "lines",
            Some(::sce_rust_runtime::test::canonical::list(
                value.lines.as_slice().iter().map(|item_0| {
                    ::sce_rust_runtime::test::canonical::record([
                        (
                            "sku",
                            Some(::sce_rust_runtime::test::canonical::text(
                                item_0.sku.as_str(),
                            )),
                        ),
                        (
                            "quantity",
                            Some(::sce_rust_runtime::test::canonical::integer(
                                item_0.quantity,
                            )),
                        ),
                        (
                            "color",
                            Some(::sce_rust_runtime::test::canonical::enumerated(
                                item_0.color.declared_name(),
                            )),
                        ),
                    ])
                }),
            )),
        ),
        (
            "note",
            value
                .note
                .as_ref()
                .map(|present_0| ::sce_rust_runtime::test::canonical::text(present_0.as_str())),
        ),
        (
            "priority",
            value
                .priority
                .as_ref()
                .map(|present_0| ::sce_rust_runtime::test::canonical::integer(*present_0)),
        ),
        (
            "customer",
            Some(::sce_rust_runtime::test::canonical::record([
                (
                    "full_name",
                    Some(::sce_rust_runtime::test::canonical::text(
                        value.customer.full_name.as_str(),
                    )),
                ),
                (
                    "vip",
                    Some(::sce_rust_runtime::test::canonical::boolean(
                        value.customer.vip,
                    )),
                ),
            ])),
        ),
        (
            "payment",
            Some(match &value.payment {
                super::order_payment::OrderPayment::Card(case_0) => {
                    ::sce_rust_runtime::test::canonical::union_case(
                        "card",
                        Some(::sce_rust_runtime::test::canonical::record([
                            (
                                "last4",
                                Some(::sce_rust_runtime::test::canonical::text(
                                    case_0.last4.as_str(),
                                )),
                            ),
                            (
                                "amount",
                                Some(::sce_rust_runtime::test::canonical::integer(case_0.amount)),
                            ),
                        ])),
                    )
                }
                super::order_payment::OrderPayment::Cash => {
                    ::sce_rust_runtime::test::canonical::union_case("cash", None)
                }
                super::order_payment::OrderPayment::Voucher(case_0) => {
                    ::sce_rust_runtime::test::canonical::union_case(
                        "voucher",
                        Some(::sce_rust_runtime::test::canonical::record([(
                            "voucher_code",
                            Some(::sce_rust_runtime::test::canonical::bytes(
                                case_0.voucher_code.as_slice(),
                            )),
                        )])),
                    )
                }
            }),
        ),
    ])
}
