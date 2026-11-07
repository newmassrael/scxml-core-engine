// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The Rust units of the test profile's templates (docs/SCE_TEST_PROFILE.md §6,
// §11.4, §12.6 slice 6), matched against concrete values: each mechanism of
// ES 201 873-1 Annex B.1 the profile holds, true and false, the value of each
// specific template matching its own template, and a union matched by the
// variant of its case (docs/adr/0092). The committed units under
// `src/integration/static_datamodel/` are generated from the fixtures under
// `sce-build/tests/fixtures/test_profile/` and `sce-build/tests/fixtures/union/`
// (regen: `scripts/regen_static_datamodel_rust.sh`); the C++ suite holds the
// same templates to the same values (`tests/integration/ATestTemplateMatchesItsValuesTest.cpp`).

use sce_portable_bytes::{BoundedList, SceString};
use sce_rust_tests::integration::static_datamodel::{
    card_payment, card_within, elapsed_at_most, every_matcher, job_report, job_report_of,
    job_status, order_customer, order_line, order_line_bounded, order_line_exact, order_payment,
    order_placed, order_placed_match, paint_color, report_within, shape_is_circle, shape_radius,
    shape_rect, union_shape, voucher_is,
};

type JobStatus = job_status::JobStatusPayload;
type OrderLine = order_line::OrderLinePayload;
type Card = card_payment::CardPaymentPayload;
type Color = paint_color::PaintColor;
type Order = order_placed::OrderPlacedPayload;
type Payment = order_payment::OrderPayment;
type Shape = union_shape::UnionShape;

/// U+00E9, one character of two UTF-8 bytes.
const E_ACUTE: &str = "\u{e9}";

/// The limit every_matcher's range and reference are given.
const LIMIT: u32 = 100;

/// `written` as a text of at most `N` bytes, which it fits.
fn text<const N: usize>(written: &str) -> SceString<N> {
    SceString::from_text(written).expect("the text fits its bound")
}

/// `items` as the codes of a job status, which hold at most four.
fn codes(items: &[u8]) -> BoundedList<u8, 4> {
    BoundedList::from_slice(items).expect("the codes fit their bound")
}

/// A job status every_matcher matches with the limit 100: each field at a value
/// its matcher holds, each optional field absent.
fn matching() -> JobStatus {
    JobStatus {
        elapsed_ms: 1,
        note: None,
        codes: codes(&[3, 9, 5]),
        tag: None,
        retry: None,
        count: 100,
    }
}

#[test]
fn a_status_each_of_whose_fields_its_matcher_holds_matches() {
    assert!(every_matcher::matches(&matching(), LIMIT));
}

/// value-list (B.1.2.1) of a specific value and a range (B.1.2.5), both bounds
/// inclusive, the upper one a formal parameter.
#[test]
fn a_value_list_matches_any_of_its_items_and_a_range_holds_its_bounds() {
    for elapsed in [1, 10, 50, LIMIT] {
        let status = JobStatus {
            elapsed_ms: elapsed,
            ..matching()
        };
        assert!(every_matcher::matches(&status, LIMIT), "{elapsed}");
    }
    for elapsed in [0, 2, 9, LIMIT + 1] {
        let status = JobStatus {
            elapsed_ms: elapsed,
            ..matching()
        };
        assert!(!every_matcher::matches(&status, LIMIT), "{elapsed}");
    }
}

/// any-value with ifpresent (B.1.4.2) and a length restriction (B.1.4.1) on an
/// optional text: absent, or present with at most eight characters.
#[test]
fn ifpresent_takes_an_absent_field_and_a_length_counts_characters() {
    let with_note = |note: Option<&str>| JobStatus {
        note: note.map(text::<32>),
        ..matching()
    };
    assert!(every_matcher::matches(&with_note(None), LIMIT));
    assert!(every_matcher::matches(&with_note(Some("abcdefgh")), LIMIT));
    assert!(!every_matcher::matches(
        &with_note(Some("abcdefghi")),
        LIMIT
    ));
    // Eight characters of two bytes each: sixteen bytes, eight characters.
    assert!(every_matcher::matches(
        &with_note(Some(E_ACUTE.repeat(8).as_str())),
        LIMIT
    ));
    assert!(!every_matcher::matches(
        &with_note(Some(E_ACUTE.repeat(9).as_str())),
        LIMIT
    ));
}

/// list-match (B.1.1): exactly three elements, the first 3, the second any
/// value (B.1.3.1), the third not 0 (complement, B.1.2.2).
#[test]
fn a_list_match_holds_its_count_and_each_element_in_order() {
    let held: [&[u8]; 2] = [&[3, 9, 5], &[3, 0, 1]];
    for items in held {
        let status = JobStatus {
            codes: codes(items),
            ..matching()
        };
        assert!(every_matcher::matches(&status, LIMIT), "{items:?}");
    }
    let refused: [&[u8]; 5] = [&[3, 9, 0], &[4, 9, 5], &[3, 9], &[3, 9, 5, 5], &[]];
    for items in refused {
        let status = JobStatus {
            codes: codes(items),
            ..matching()
        };
        assert!(!every_matcher::matches(&status, LIMIT), "{items:?}");
    }
}

/// any-value-or-none (B.1.2.4) and omit (B.1.2.8).
#[test]
fn any_value_or_none_takes_both_and_omit_only_an_absent_field() {
    let tagged = JobStatus {
        tag: Some(7),
        ..matching()
    };
    assert!(every_matcher::matches(&tagged, LIMIT));
    assert!(every_matcher::matches(&matching(), LIMIT));
    let retried = JobStatus {
        retry: Some(1),
        ..matching()
    };
    assert!(!every_matcher::matches(&retried, LIMIT));
}

/// template-ref (-1 §15.1): the referenced template's match, its formal
/// parameter bound to this template's own.
#[test]
fn a_reference_matches_by_its_template_with_the_argument_it_binds() {
    let status = JobStatus {
        count: 150,
        ..matching()
    };
    assert!(!every_matcher::matches(&status, LIMIT));
    assert!(every_matcher::matches(&status, 150));
    assert!(elapsed_at_most::matches(&150, 150));
    assert!(!elapsed_at_most::matches(&151, 150));
}

/// A specific template's value (§6.2) matches the template, and a value that
/// differs in one field does not.
#[test]
fn a_specific_templates_value_matches_its_template() {
    let report = job_report::value();
    assert_eq!(report.elapsed_ms, 0);
    assert_eq!(report.note.as_deref(), Some("started on time"));
    assert_eq!(report.codes.as_slice(), &[7]);
    assert_eq!(report.tag, None);
    assert_eq!(report.retry, None);
    assert_eq!(report.count, 1);
    assert!(job_report::matches(&report));

    let absent = JobStatus {
        note: None,
        ..report.clone()
    };
    assert!(!job_report::matches(&absent));
    let retried = JobStatus {
        retry: Some(0),
        ..report.clone()
    };
    assert!(!job_report::matches(&retried));
    let counted = JobStatus { count: 2, ..report };
    assert!(!job_report::matches(&counted));
}

/// A specific template whose values are partly its parameters.
#[test]
fn a_specific_templates_value_holds_the_parameters_it_is_given() {
    let note: SceString<32> = text("late");
    let report = job_report_of::value(5, &note, 9);
    assert_eq!(report.elapsed_ms, 5);
    assert_eq!(report.note.as_ref(), Some(&note));
    assert_eq!(report.codes.as_slice(), &[9, 255]);
    assert_eq!(report.tag, None);
    assert_eq!(report.retry, Some(2));
    assert_eq!(report.count, 4_294_967_295);
    assert!(job_report_of::matches(&report, 5, &note, 9));
    assert!(!job_report_of::matches(&report, 6, &note, 9));
    assert!(!job_report_of::matches(&report, 5, &text("early"), 9));
    assert!(!job_report_of::matches(&report, 5, &note, 8));
}

/// An enum literal, and a reference to a specific template, which builds the
/// field with that template's value.
#[test]
fn an_enum_literal_and_a_reference_to_a_specific_template_build_and_match() {
    let line = order_line_exact::value();
    assert_eq!(line.sku, "A-1");
    assert_eq!(line.quantity, 2);
    assert_eq!(line.color, Color::Red);
    assert!(order_line_exact::matches(&line));

    let blue = OrderLine {
        color: Color::Blue,
        ..line.clone()
    };
    assert!(!order_line_exact::matches(&blue));
    let three = OrderLine {
        quantity: 3,
        ..line
    };
    assert!(!order_line_exact::matches(&three));
}

/// A reference whose template matches a range up to the argument it binds.
#[test]
fn a_reference_to_a_range_holds_the_bound_its_argument_gives() {
    let line = OrderLine {
        sku: text("A-1"),
        quantity: 5,
        color: Color::Blue,
    };
    assert!(order_line_bounded::matches(&line, 5));
    assert!(!order_line_bounded::matches(&line, 4));
    let green = OrderLine {
        color: Color::Green,
        ..line
    };
    assert!(!order_line_bounded::matches(&green, 5));
}

/// A template that is specific in itself and references one that is not
/// matches by the referenced template's range.
#[test]
fn a_template_matches_through_a_reference_to_one_that_is_not_specific() {
    let status = JobStatus {
        count: 60,
        ..job_report::value()
    };
    assert!(report_within::matches(&status, 60));
    assert!(!report_within::matches(&status, 59));
}

/// A length restriction on a text counts characters, so a text whose bytes
/// pass its upper bound matches while its characters do not; a range with a
/// literal lower bound.
#[test]
fn a_length_on_a_text_counts_a_multi_byte_character_as_one() {
    let card = |last4: &str, amount: u32| Card {
        last4: text(last4),
        amount,
    };
    // Two characters in three bytes, and in four.
    for last4 in [format!("{E_ACUTE}1"), E_ACUTE.repeat(2), "123".to_string()] {
        assert!(
            card_within::matches(&card(last4.as_str(), 50), 100),
            "{last4}"
        );
    }
    for last4 in ["1".to_string(), E_ACUTE.to_string(), "1234".to_string()] {
        assert!(
            !card_within::matches(&card(last4.as_str(), 50), 100),
            "{last4}"
        );
    }
    assert!(card_within::matches(&card("12", 1), 100));
    assert!(!card_within::matches(&card("12", 0), 100));
    assert!(card_within::matches(&card("12", 100), 100));
    assert!(!card_within::matches(&card("12", 101), 100));
}

/// A byte string written in hex.
#[test]
fn a_byte_string_literal_is_its_octets() {
    let voucher = voucher_is::value();
    assert_eq!(voucher.voucher_code, [0x00, 0xff, 0x10, 0xa0]);
    assert!(voucher_is::matches(&voucher));
    let mut other = voucher.clone();
    other.voucher_code[3] = 0xa1;
    assert!(!voucher_is::matches(&other));
}

/// union-match (§11.4): a union matches in the case its matcher names, and in
/// no other, whatever the case holds.
#[test]
fn a_union_matches_in_the_case_its_matcher_names() {
    let circle = Shape::Circle(shape_radius::ShapeRadiusPayload { radius: 3 });
    assert!(shape_is_circle::matches(&circle));
    assert!(!shape_is_circle::matches(&Shape::Dot));
    let rect = Shape::Rect(shape_rect::ShapeRectPayload {
        width: 1,
        height: 2,
    });
    assert!(!shape_is_circle::matches(&rect));
}

/// The quantity limit order_placed_match is given.
const QUANTITY_LIMIT: u16 = 5;

/// An order order_placed_match matches with the quantity limit 5: two lines,
/// the second the exact one, no note and no priority, a VIP customer, paid by
/// a card of four characters.
fn order() -> Order {
    let first = OrderLine {
        sku: text("A-1"),
        quantity: 5,
        color: Color::Green,
    };
    Order {
        order_id: 7,
        lines: BoundedList::from_slice(&[first, order_line_exact::value()])
            .expect("the lines fit their bound"),
        note: None,
        priority: None,
        customer: order_customer::OrderCustomerPayload {
            full_name: text("Ada"),
            vip: true,
        },
        payment: Payment::Card(Card {
            last4: text("1234"),
            amount: 1,
        }),
    }
}

/// The order with `lines` instead of its own.
fn order_with_lines(lines: &[OrderLine]) -> Order {
    Order {
        lines: BoundedList::from_slice(lines).expect("the lines fit their bound"),
        ..order()
    }
}

/// A value list of two union matches: a card whose record the case's matcher
/// holds, and the case that holds no record; any other case, or a card its
/// matcher does not hold, matches neither.
#[test]
fn a_union_field_matches_one_of_the_cases_its_value_list_names() {
    assert!(order_placed_match::matches(&order(), QUANTITY_LIMIT));
    let cash = Order {
        payment: Payment::Cash,
        ..order()
    };
    assert!(order_placed_match::matches(&cash, QUANTITY_LIMIT));
    let voucher = Order {
        payment: Payment::Voucher(voucher_is::value()),
        ..order()
    };
    assert!(!order_placed_match::matches(&voucher, QUANTITY_LIMIT));
    for (last4, amount) in [("123", 1), ("1234", 0)] {
        let card = Order {
            payment: Payment::Card(Card {
                last4: text(last4),
                amount,
            }),
            ..order()
        };
        assert!(
            !order_placed_match::matches(&card, QUANTITY_LIMIT),
            "{last4} {amount}"
        );
    }
}

/// The other fields of the order: a list of records, a reference with the
/// argument it binds, an optional text with ifpresent and a length, an
/// optional number with ifpresent and a range, and a nested record's `bool`.
#[test]
fn every_field_of_an_order_is_held_to_its_matcher() {
    let first = |quantity: u16, color: Color| OrderLine {
        sku: text("A-1"),
        quantity,
        color,
    };
    let more = order_with_lines(&[first(6, Color::Green), order_line_exact::value()]);
    assert!(!order_placed_match::matches(&more, QUANTITY_LIMIT));
    assert!(order_placed_match::matches(&more, 6));
    let blue = order_with_lines(&[first(5, Color::Blue), order_line_exact::value()]);
    assert!(!order_placed_match::matches(&blue, QUANTITY_LIMIT));
    let inexact = order_with_lines(&[first(5, Color::Green), first(5, Color::Green)]);
    assert!(!order_placed_match::matches(&inexact, QUANTITY_LIMIT));
    let one = order_with_lines(&[first(5, Color::Green)]);
    assert!(!order_placed_match::matches(&one, QUANTITY_LIMIT));

    for (note, held) in [("abcdefgh", true), ("abcdefghi", false)] {
        let noted = Order {
            note: Some(text(note)),
            ..order()
        };
        assert_eq!(
            order_placed_match::matches(&noted, QUANTITY_LIMIT),
            held,
            "{note}"
        );
    }
    for (priority, held) in [(1, true), (5, true), (0, false), (6, false)] {
        let prioritised = Order {
            priority: Some(priority),
            ..order()
        };
        assert_eq!(
            order_placed_match::matches(&prioritised, QUANTITY_LIMIT),
            held,
            "{priority}"
        );
    }
    let ordinary = Order {
        customer: order_customer::OrderCustomerPayload {
            full_name: text("Ada"),
            vip: false,
        },
        ..order()
    };
    assert!(!order_placed_match::matches(&ordinary, QUANTITY_LIMIT));
}
