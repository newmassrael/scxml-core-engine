// SCE-GENERATED — DO NOT EDIT
// source-hash: 12a7dc72335a819ac5590a1b6b46aee0f5b664147671271baaaa11524ee4c7b9
#![doc = "SCE-MAP: nested_configuration.scxml:4 :: _forge_body"]
// SCE-MAP: nested_configuration.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-configuration")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

/// The test configuration `nested_configuration` (docs/SCE_TEST_PROFILE.md §11.6) as the
/// profile's runtime reads it: the MTC's component type and each PTC's, the ports and timers
/// each type declares, each `<sce:connect>` and each `<sce:map>`.
pub fn configuration() -> ::sce_rust_runtime::test::Configuration {
    ::sce_rust_runtime::test::Configuration {
        mtc_type: "nested_tester".to_string(),
        ptc_types: ::std::collections::BTreeMap::new(),
        ports_of_type: ::std::collections::BTreeMap::from([(
            "nested_tester".to_string(),
            vec!["sut".to_string()],
        )]),
        timers_of_type: ::std::collections::BTreeMap::from([(
            "nested_tester".to_string(),
            vec!["tick".to_string(), "limit".to_string()],
        )]),
        connections: Vec::new(),
        mappings: vec![(
            ::sce_rust_runtime::test::Endpoint::new("mtc", "sut"),
            "sut".to_string(),
        )],
    }
}

/// The machine of each component type of the configuration: the component the profile's
/// runtime drives, generated beside its test component's machine (docs/SCE_TEST_PROFILE.md
/// §12.3). None for a type the configuration does not name, which the run ends with `error`
/// where it is created.
pub fn machine(
    component_type: &str,
) -> Option<Box<dyn ::sce_rust_runtime::test::ComponentMachine>> {
    let machine: Box<dyn ::sce_rust_runtime::test::ComponentMachine> = match component_type {
        "nested_tester" => Box::<super::nested_tester_sm::NestedTesterComponent>::default(),
        _ => return None,
    };
    Some(machine)
}

/// The module parameters the components of the configuration read
/// (docs/SCE_TEST_PROFILE.md §11.5), each of which a suite that runs it gives a value.
pub fn module_parameters() -> Vec<String> {
    Vec::new()
}

/// The record `order_placed` read from the canonical form of the test log
/// (docs/SCE_TEST_PROFILE.md §12.5), as a recorded input trace holds it (docs/SCE_TTCN3_LANE.md
/// §10.3), or refused, naming where its value does not belong to its type. The inverse of
/// `canonical_order_placed`.
pub fn read_order_placed(
    value: &::sce_rust_runtime::json::Value,
) -> Result<super::order_placed::OrderPlacedPayload, ::sce_rust_runtime::test::TraceRefusal> {
    Ok({
        let record_0 = ::sce_rust_runtime::test::from_canonical::record(
            value,
            &[
                "order_id", "lines", "note", "priority", "customer", "payment",
            ],
            "order_placed",
        )?;
        super::order_placed::OrderPlacedPayload { order_id: ::sce_rust_runtime::test::from_canonical::integer(::sce_rust_runtime::test::from_canonical::field(record_0, "order_id", "order_placed.order_id")?, "order_placed.order_id")?, lines: { let mut items_0 = ::std::vec::Vec::new(); for item_0 in ::sce_rust_runtime::test::from_canonical::elements(::sce_rust_runtime::test::from_canonical::field(record_0, "lines", "order_placed.lines")?, 0, 4, "order_placed.lines")? { items_0.push({ let record_1 = ::sce_rust_runtime::test::from_canonical::record(item_0, &["sku", "quantity", "color"], "order_placed.lines[]")?; super::order_line::OrderLinePayload { sku: ::sce_rust_runtime::test::from_canonical::text(::sce_rust_runtime::test::from_canonical::field(record_1, "sku", "order_placed.lines[].sku")?, 16, "order_placed.lines[].sku")?, quantity: ::sce_rust_runtime::test::from_canonical::integer(::sce_rust_runtime::test::from_canonical::field(record_1, "quantity", "order_placed.lines[].quantity")?, "order_placed.lines[].quantity")?, color: ::sce_rust_runtime::test::from_canonical::enumerated(::sce_rust_runtime::test::from_canonical::field(record_1, "color", "order_placed.lines[].color")?, &[("red", super::paint_color::PaintColor::Red), ("green", super::paint_color::PaintColor::Green), ("blue", super::paint_color::PaintColor::Blue)], "order_placed.lines[].color")? } }); } ::sce_rust_runtime::test::from_canonical::list(items_0, "order_placed.lines")? }, note: ::sce_rust_runtime::test::from_canonical::optional_field(record_0, "note").map(|present_0| -> ::std::result::Result<_, ::sce_rust_runtime::test::TraceRefusal> { let read = ::sce_rust_runtime::test::from_canonical::text(present_0, 32, "order_placed.note")?; Ok(read) }).transpose()?, priority: ::sce_rust_runtime::test::from_canonical::optional_field(record_0, "priority").map(|present_0| -> ::std::result::Result<_, ::sce_rust_runtime::test::TraceRefusal> { let read = ::sce_rust_runtime::test::from_canonical::integer(present_0, "order_placed.priority")?; Ok(read) }).transpose()?, customer: { let record_1 = ::sce_rust_runtime::test::from_canonical::record(::sce_rust_runtime::test::from_canonical::field(record_0, "customer", "order_placed.customer")?, &["full_name", "vip"], "order_placed.customer")?; super::order_customer::OrderCustomerPayload { full_name: ::sce_rust_runtime::test::from_canonical::text(::sce_rust_runtime::test::from_canonical::field(record_1, "full_name", "order_placed.customer.full_name")?, 24, "order_placed.customer.full_name")?, vip: ::sce_rust_runtime::test::from_canonical::boolean(::sce_rust_runtime::test::from_canonical::field(record_1, "vip", "order_placed.customer.vip")?, "order_placed.customer.vip")? } }, payment: { let (variant_0, case_0) = ::sce_rust_runtime::test::from_canonical::union_case(::sce_rust_runtime::test::from_canonical::field(record_0, "payment", "order_placed.payment")?, &["card", "cash", "voucher"], "order_placed.payment")?; match variant_0 { "card" => super::order_payment::OrderPayment::Card({ let record_1 = ::sce_rust_runtime::test::from_canonical::record(case_0, &["last4", "amount"], "order_placed.payment")?; super::card_payment::CardPaymentPayload { last4: ::sce_rust_runtime::test::from_canonical::text(::sce_rust_runtime::test::from_canonical::field(record_1, "last4", "order_placed.payment.last4")?, 4, "order_placed.payment.last4")?, amount: ::sce_rust_runtime::test::from_canonical::integer(::sce_rust_runtime::test::from_canonical::field(record_1, "amount", "order_placed.payment.amount")?, "order_placed.payment.amount")? } }), "cash" => { ::sce_rust_runtime::test::from_canonical::no_record(case_0, "order_placed.payment.cash")?; super::order_payment::OrderPayment::Cash }, _ => super::order_payment::OrderPayment::Voucher({ let record_1 = ::sce_rust_runtime::test::from_canonical::record(case_0, &["voucher_code"], "order_placed.payment")?; super::voucher_payment::VoucherPaymentPayload { voucher_code: ::sce_rust_runtime::test::from_canonical::bytes(::sce_rust_runtime::test::from_canonical::field(record_1, "voucher_code", "order_placed.payment.voucher_code")?, 4, "order_placed.payment.voucher_code")? } }) } } }
    })
}

/// The record `order_placed` in the canonical form of the test log
/// (docs/SCE_TEST_PROFILE.md §12.5), which a message of it a trace hands the run carries.
pub fn canonical_order_placed(
    value: &super::order_placed::OrderPlacedPayload,
) -> ::sce_rust_runtime::json::Value {
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

/// The reader of the inputs of a recorded trace (docs/SCE_TTCN3_LANE.md §10.2,
/// `sce_rust_runtime::test::InputReader`): the message an input of the record type `type_name`
/// on the system port `system_port` carries, its value read from the canonical form. Refused,
/// naming why, where the configuration has no such system port, the `in` list of its type does
/// not name the type, or the value does not belong to it.
pub fn input(
    system_port: &str,
    type_name: &str,
    value: &::sce_rust_runtime::json::Value,
) -> Result<::sce_rust_runtime::test::Message, ::sce_rust_runtime::test::TraceRefusal> {
    match (system_port, type_name) {
        ("sut", "order_placed") => Ok(::sce_rust_runtime::test::Message::new(
            super::order_placed::EVENT_NAME,
            read_order_placed(value)?,
            "order_placed",
            canonical_order_placed,
        )),
        _ => Err(::sce_rust_runtime::test::TraceRefusal::not_an_input(
            system_port,
            type_name,
            &["sut"],
        )),
    }
}
