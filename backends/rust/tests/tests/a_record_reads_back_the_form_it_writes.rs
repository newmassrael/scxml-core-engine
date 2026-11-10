// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The reader and the writer of the canonical form a test configuration's unit
// generates for each record its system ports take (docs/SCE_TTCN3_LANE.md
// §10.3, §10.5; docs/SCE_TEST_PROFILE.md §12.5), compiled here over records of
// every form: order_placed (a list of records with a closed enum, an optional
// text and integer, a record with a truth value, a union of a case with a
// record, one with none and one with a byte string), job_status (a list of
// integers, optional integers) and form_extras (an enum of an open set, a list
// of optional elements, a 64-bit integer, a list with a lower count, an
// optional list and a list of lists). The units are generated from
// `sce-build/tests/fixtures/test_profile/trace_forms/` (regen:
// `scripts/regen_static_datamodel_rust.sh`).
//
// A value written by `canonical_<record>` and read back by `read_<record>` is
// the value it was, through the canonical text; each malformed spelling of a
// form is refused, naming where it stands; and a trace of one record of each
// type replays to a log that holds each value as the trace does.

use sce_portable_bytes::{BoundedList, SceString};
use sce_rust_runtime::json::{self, Value};
use sce_rust_runtime::test::{
    run_suite_with_traces, Execute, Features, ModuleParameters, Suite, Trace, TraceInput,
    TraceRefusal, Verdict,
};
use sce_rust_tests::integration::static_datamodel::{
    card_payment, form_extras, forms_configuration, job_status, order_customer, order_line,
    order_payment, order_placed, paint_color, replays_every_form, shape_kind_open, voucher_payment,
};

type Order = order_placed::OrderPlacedPayload;
type OrderLine = order_line::OrderLinePayload;
type Customer = order_customer::OrderCustomerPayload;
type Payment = order_payment::OrderPayment;
type Card = card_payment::CardPaymentPayload;
type Voucher = voucher_payment::VoucherPaymentPayload;
type Color = paint_color::PaintColor;
type Status = job_status::JobStatusPayload;
type Extras = form_extras::FormExtrasPayload;
type Shape = shape_kind_open::ShapeKindOpen;

/// The writer of a record's canonical form, and its reader.
type Writer<V> = fn(&V) -> Value;
type Reader<V> = fn(&Value) -> Result<V, TraceRefusal>;

/// `written` as a text of at most `N` bytes, which it fits.
fn text<const N: usize>(written: &str) -> SceString<N> {
    SceString::from_text(written).expect("the text fits its bound")
}

/// `items` as a list of at most `N` elements, which they fit.
fn list<T: Clone, const N: usize>(items: &[T]) -> BoundedList<T, N> {
    BoundedList::from_slice(items).expect("the elements fit their bound")
}

/// An order paid by `payment`: two lines, one with a text of a character of
/// two bytes and an escaped quote, the widest order id, a note and no
/// priority.
fn order(payment: Payment) -> Order {
    Order {
        order_id: u32::MAX,
        lines: list(&[
            OrderLine {
                sku: text("A-1"),
                quantity: 2,
                color: Color::Green,
            },
            OrderLine {
                sku: text("\"q\" \u{e9}"),
                quantity: u16::MAX,
                color: Color::Blue,
            },
        ]),
        note: Some(text("ring twice")),
        priority: None,
        customer: Customer {
            full_name: text("Ada"),
            vip: true,
        },
        payment,
    }
}

/// The three cases of the payment: one of a record, one of none, one of a
/// byte string.
fn payments() -> [Payment; 3] {
    [
        Payment::Card(Card {
            last4: text("4242"),
            amount: 1,
        }),
        Payment::Cash,
        Payment::Voucher(Voucher {
            voucher_code: vec![0x00, 0xab, 0xff],
        }),
    ]
}

fn statuses() -> [Status; 2] {
    [
        Status {
            elapsed_ms: 0,
            note: None,
            codes: list(&[]),
            tag: Some(u8::MAX),
            retry: None,
            count: 7,
        },
        Status {
            elapsed_ms: 12,
            note: Some(text("late")),
            codes: list(&[3, 9, 5, 0]),
            tag: None,
            retry: Some(0),
            count: 0,
        },
    ]
}

/// The extras: a declared variant and a number no variant declares, a list
/// with an absent element, the least 64-bit integer, a list at its lower
/// count and at its bound, an optional list present and absent, and a list of
/// lists with an empty one.
fn extras() -> [Extras; 2] {
    [
        Extras {
            shape: Shape::Circle,
            slots: list(&[Some(1), None, Some(u16::MAX)]),
            least: i64::MIN,
            flags: list(&[true]),
            spare: Some(list(&[0, u8::MAX])),
            grid: list(&[list(&[1, 2]), list(&[])]),
        },
        Extras {
            shape: Shape::from_underlying(9),
            slots: list(&[]),
            least: i64::MAX,
            flags: list(&[false, true]),
            spare: None,
            grid: list(&[]),
        },
    ]
}

/// `value` written by `write` in its canonical text, and read back from that
/// text by `read`.
fn round_trip<V: PartialEq + std::fmt::Debug>(value: &V, write: Writer<V>, read: Reader<V>) {
    let written = json::write_canonical(&write(value)).expect("a canonical value");
    let parsed = json::parse(&written).expect("the canonical text is JSON");
    assert_eq!(read(&parsed).as_ref(), Ok(value), "{written}");
}

/// docs/SCE_TEST_PROFILE.md §12.5: a value of each form reads back from the
/// text its writer gives as the value it was.
#[test]
fn a_value_of_each_form_reads_back_from_its_canonical_text() {
    for payment in payments() {
        round_trip(
            &order(payment),
            forms_configuration::canonical_order_placed,
            forms_configuration::read_order_placed,
        );
    }
    for status in statuses() {
        round_trip(
            &status,
            forms_configuration::canonical_job_status,
            forms_configuration::read_job_status,
        );
    }
    for extra in extras() {
        round_trip(
            &extra,
            forms_configuration::canonical_form_extras,
            forms_configuration::read_form_extras,
        );
    }
}

/// The reason `read` refuses the canonical text of `value` with `from`
/// replaced once by `to` for.
fn refused<V>(value: &V, write: Writer<V>, read: Reader<V>, from: &str, to: &str) -> String {
    let written = json::write_canonical(&write(value)).expect("a canonical value");
    assert!(written.contains(from), "`{from}` in {written}");
    let changed = written.replacen(from, to, 1);
    let parsed = json::parse(&changed).expect("the changed text is JSON");
    match read(&parsed) {
        Ok(_) => panic!("{changed} is read"),
        Err(refusal) => refusal.reason().to_string(),
    }
}

/// docs/SCE_TTCN3_LANE.md §10.2: a malformed spelling of each form is refused,
/// naming the place it stands at: the record's type, each field by its id and
/// `[]` for an element of a list.
#[test]
fn a_malformed_spelling_of_each_form_is_refused_by_name() {
    let [card, cash, voucher] = payments();
    let card = order(card);
    let cash = order(cash);
    let voucher = order(voucher);
    let of_order = |value: &Order, from: &str, to: &str| {
        refused(
            value,
            forms_configuration::canonical_order_placed,
            forms_configuration::read_order_placed,
            from,
            to,
        )
    };
    let [extra, _] = extras();
    let of_extras = |from: &str, to: &str| {
        refused(
            &extra,
            forms_configuration::canonical_form_extras,
            forms_configuration::read_form_extras,
            from,
            to,
        )
    };
    let [status, _] = statuses();
    let of_status = |from: &str, to: &str| {
        refused(
            &status,
            forms_configuration::canonical_job_status,
            forms_configuration::read_job_status,
            from,
            to,
        )
    };
    let long_name = format!("\"full_name\":\"{}\"", "x".repeat(25));
    for (reason, expected) in [
        (
            of_order(&card, "\"order_id\":\"4294967295\"", "\"order_id\":\"007\""),
            "order_placed.order_id: '007' is not the text of an integer's decimal digits",
        ),
        (
            of_order(&card, "\"quantity\":\"2\"", "\"quantity\":\"65536\""),
            "order_placed.lines[].quantity: 65536 is past the range of its type",
        ),
        (
            of_order(&card, "\"vip\":true", "\"vip\":\"true\""),
            "order_placed.customer.vip: a text is not a truth value",
        ),
        (
            of_order(&card, "\"full_name\":\"Ada\"", &long_name),
            "order_placed.customer.full_name: a text of 25 bytes is past the 24 its type holds",
        ),
        (
            of_order(&card, "\"color\":\"green\"", "\"color\":\"purple\""),
            "order_placed.lines[].color: 'purple' names no variant of its enum",
        ),
        (
            of_order(&card, "\"note\":\"ring twice\"", "\"note\":null"),
            "order_placed.note: null is not a text",
        ),
        (
            of_order(&card, "{\"customer\":", "{\"rush\":\"1\",\"customer\":"),
            "order_placed: 'rush' is no field of its record",
        ),
        (
            of_order(&card, "\"order_id\":\"4294967295\",", ""),
            "order_placed.order_id: the field is absent, and its type is not optional",
        ),
        (
            of_order(
                &card,
                "\"lines\":[",
                "\"lines\":[{\"color\":\"red\",\"quantity\":\"1\",\"sku\":\"s\"},\
                 {\"color\":\"red\",\"quantity\":\"1\",\"sku\":\"s\"},\
                 {\"color\":\"red\",\"quantity\":\"1\",\"sku\":\"s\"},",
            ),
            "order_placed.lines: a list of 5 elements is outside its bounds, 0 to 4",
        ),
        (
            of_order(&card, "{\"card\":", "{\"cheque\":"),
            "order_placed.payment: 'cheque' names no case of its union",
        ),
        (
            of_order(
                &card,
                "\"last4\":\"4242\"}}",
                "\"last4\":\"4242\"},\"cash\":{}}",
            ),
            "order_placed.payment: a value of a union is an object of one member, and this one \
             has 2",
        ),
        (
            of_order(&cash, "{\"cash\":{}}", "{\"cash\":{\"x\":\"1\"}}"),
            "order_placed.payment.cash: a case that holds no record is the empty object",
        ),
        (
            of_order(
                &voucher,
                "\"voucher_code\":\"00ABFF\"",
                "\"voucher_code\":\"00abff\"",
            ),
            "order_placed.payment.voucher_code: '00abff' is not octets in upper-case hex, two \
             digits each",
        ),
        (
            of_order(
                &voucher,
                "\"voucher_code\":\"00ABFF\"",
                "\"voucher_code\":\"0011223344\"",
            ),
            "order_placed.payment.voucher_code: a byte string of 5 octets is past the 4 its type \
             holds",
        ),
        (
            of_status("\"tag\":\"255\"", "\"tag\":\"256\""),
            "job_status.tag: 256 is past the range of its type",
        ),
        (
            of_status("\"codes\":[]", "\"codes\":[\"x\"]"),
            "job_status.codes[]: 'x' is not the text of an integer's decimal digits",
        ),
        (
            of_extras("\"shape\":\"circle\"", "\"shape\":\"1\""),
            "form_extras.shape: 1 is the number of a declared variant, which is written by its \
             name",
        ),
        (
            of_extras("\"shape\":\"circle\"", "\"shape\":\"hexagon\""),
            "form_extras.shape: 'hexagon' names no variant of its enum, and is no number",
        ),
        (
            of_extras("\"slots\":[\"1\",null,", "\"slots\":[\"1\",\"x\","),
            "form_extras.slots[]: 'x' is not the text of an integer's decimal digits",
        ),
        (
            of_extras("\"flags\":[true]", "\"flags\":[]"),
            "form_extras.flags: a list of 0 elements is outside its bounds, 1 to 2",
        ),
        (
            of_extras("\"grid\":[[\"1\",\"2\"]", "\"grid\":[[\"1\",\"2\",\"3\"]"),
            "form_extras.grid[]: a list of 3 elements is outside its bounds, 0 to 2",
        ),
        (
            of_extras(
                "\"least\":\"-9223372036854775808\"",
                "\"least\":\"-9223372036854775809\"",
            ),
            "form_extras.least: -9223372036854775809 is past the range of its type",
        ),
        (
            of_extras("\"spare\":[\"0\",\"255\"]", "\"spare\":null"),
            "form_extras.spare: null is not a list, which is an array",
        ),
    ] {
        assert_eq!(reason, expected);
    }
}

/// docs/SCE_TTCN3_LANE.md §10.5: a trace of one record of each type the system
/// port takes, two at the start and one later, replays to the listener, which
/// takes one a round and passes; the log holds each value as the trace does.
#[test]
fn a_trace_of_every_form_replays_to_the_values_it_holds() {
    let [card, _, _] = payments();
    let [status, _] = statuses();
    let [extra, _] = extras();
    let input = |at: i64, message_type: &str, value: Value| TraceInput {
        at,
        port: "sut".to_string(),
        message_type: message_type.to_string(),
        value,
    };
    let trace = Trace {
        suite: "forms_suite".to_string(),
        test_case: replays_every_form::NAME.to_string(),
        inputs: vec![
            input(
                0,
                "order_placed",
                forms_configuration::canonical_order_placed(&order(card)),
            ),
            input(
                0,
                "job_status",
                forms_configuration::canonical_job_status(&status),
            ),
            input(
                250,
                "form_extras",
                forms_configuration::canonical_form_extras(&extra),
            ),
        ],
    };
    let text = trace.canonical_text();
    let suite = Suite {
        name: "forms_suite".to_string(),
        module_parameters: ModuleParameters::default(),
        executes: vec![Execute {
            test_case: replays_every_form::test_case(),
            guard: None,
            selection: None,
        }],
    };
    let results = run_suite_with_traces(
        &suite,
        &Features::default(),
        |_: &str| Trace::parse(&text),
        &ModuleParameters::default(),
    )
    .expect("no override is given");
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].result.verdict,
        Verdict::Pass,
        "{:?}",
        results[0].result.log
    );
    assert_eq!(results[0].result.ended_at, 250);
    let log = results[0].canonical_log("forms_suite");
    for input in &trace.inputs {
        let value = json::write_canonical(&input.value).expect("a canonical value");
        let received = format!(
            "\"kind\":\"message-received\",\"port\":\"sut\",\"type\":\"{}\",\"value\":{value}",
            input.message_type
        );
        assert!(log.contains(&received), "`{received}` in {log}");
    }
}
