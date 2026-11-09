# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The Python units of the test profile's templates (docs/SCE_TEST_PROFILE.md §6,
§11.4, §12.6 slice 6), matched against concrete values: each mechanism of
ES 201 873-1 Annex B.1 the profile holds, true and false, and the value of each
specific template matching its own template.

The Python half of the Rust suite's ``a_test_template_matches_its_values.rs``,
of the Kotlin ``ATestTemplateMatchesItsValuesTest.kt`` and of the Go
``a_test_template_matches_its_values_test.go``, which hold the same templates to
the same values, each test named ``test_<case>`` after the Rust case. Python
lowers no union, so the two templates over one, and their cases, have no twin
here: the generator refuses them on Python
(``sce-build/tests/a_test_template_is_generated_in_python.rs``). The modules
beside this file are generated from the fixtures under
``sce-build/tests/fixtures/test_profile/`` by
``scripts/regen_static_datamodel_python.sh`` and are not committed. Every
character outside ASCII is written as its escape.
"""
from __future__ import annotations

import dataclasses
import sys
from pathlib import Path
from typing import List

_HERE = Path(__file__).resolve().parent
# The tests directory, so this package is importable by the name it has there.
sys.path.insert(0, str(_HERE.parents[1]))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))
sys.path.insert(0, str(_HERE.parents[2] / "forge-runtime"))

from integration.static_datamodel import (  # noqa: E402 -- path inserted above
    card_payment,
    card_within,
    elapsed_at_most,
    every_matcher,
    job_report,
    job_report_of,
    job_status,
    order_line,
    order_line_bounded,
    order_line_exact,
    paint_color,
    report_within,
    voucher_is,
    voucher_payment,
)
from sce_runtime.saved_json import write_jcs  # noqa: E402

#: U+00E9, one character of two UTF-8 bytes.
E_ACUTE = "é"

#: The limit every_matcher's range and reference are given.
LIMIT = 100


def matching() -> job_status.JobStatusPayload:
    """A job status every_matcher matches with the limit 100: each field at a
    value its matcher holds, each optional field absent."""
    return job_status.JobStatusPayload(
        elapsed_ms=1,
        note=None,
        codes=[3, 9, 5],
        tag=None,
        retry=None,
        count=100,
    )


def test_a_status_each_of_whose_fields_its_matcher_holds_matches() -> None:
    assert every_matcher.matches(matching(), LIMIT)


def test_a_value_list_matches_any_of_its_items_and_a_range_holds_its_bounds() -> None:
    """value-list (B.1.2.1) of a specific value and a range (B.1.2.5), both
    bounds inclusive, the upper one a formal parameter."""
    for elapsed in (1, 10, 50, LIMIT):
        assert every_matcher.matches(dataclasses.replace(matching(), elapsed_ms=elapsed), LIMIT), elapsed
    for elapsed in (0, 2, 9, LIMIT + 1):
        assert not every_matcher.matches(dataclasses.replace(matching(), elapsed_ms=elapsed), LIMIT), elapsed


def test_ifpresent_takes_an_absent_field_and_a_length_counts_characters() -> None:
    """any-value with ifpresent (B.1.4.2) and a length restriction (B.1.4.1) on
    an optional text: absent, or present with at most eight characters."""
    cases = [
        (None, True),
        ("abcdefgh", True),
        ("abcdefghi", False),
        # Eight characters of two bytes each: sixteen bytes, eight characters.
        (E_ACUTE * 8, True),
        (E_ACUTE * 9, False),
    ]
    for note, held in cases:
        status = dataclasses.replace(matching(), note=note)
        assert every_matcher.matches(status, LIMIT) is held, note


def test_a_list_match_holds_its_count_and_each_element_in_order() -> None:
    """list-match (B.1.1): exactly three elements, the first 3, the second any
    value (B.1.3.1), the third not 0 (complement, B.1.2.2)."""
    matched: List[List[int]] = [[3, 9, 5], [3, 0, 1]]
    for items in matched:
        assert every_matcher.matches(dataclasses.replace(matching(), codes=items), LIMIT), items
    refused: List[List[int]] = [[3, 9, 0], [4, 9, 5], [3, 9], [3, 9, 5, 5], []]
    for items in refused:
        assert not every_matcher.matches(dataclasses.replace(matching(), codes=items), LIMIT), items


def test_any_value_or_none_takes_both_and_omit_only_an_absent_field() -> None:
    """any-value-or-none (B.1.2.4) and omit (B.1.2.8)."""
    assert every_matcher.matches(dataclasses.replace(matching(), tag=7), LIMIT), (
        "a present tag matches any-value-or-none"
    )
    assert every_matcher.matches(matching(), LIMIT), "an absent tag matches any-value-or-none"
    assert not every_matcher.matches(dataclasses.replace(matching(), retry=1), LIMIT), (
        "a present retry does not match omit"
    )


def test_a_reference_matches_by_its_template_with_the_argument_it_binds() -> None:
    """template-ref (-1 §15.1): the referenced template's match, its formal
    parameter bound to this template's own."""
    status = dataclasses.replace(matching(), count=150)
    assert not every_matcher.matches(status, LIMIT), "a count past the limit"
    assert every_matcher.matches(status, 150), "a count at the limit"
    assert elapsed_at_most.matches(150, 150)
    assert not elapsed_at_most.matches(151, 150)


def test_a_specific_templates_value_matches_its_template() -> None:
    """A specific template's value (§6.2) matches the template, and a value that
    differs in one field does not."""
    report = job_report.value()
    assert report == job_status.JobStatusPayload(
        elapsed_ms=0,
        note="started on time",
        codes=[7],
        tag=None,
        retry=None,
        count=1,
    )
    assert job_report.matches(report)
    assert not job_report.matches(dataclasses.replace(report, note=None)), "an absent note"
    assert not job_report.matches(dataclasses.replace(report, retry=0)), "a present retry"
    assert not job_report.matches(dataclasses.replace(report, count=2)), "another count"


def test_a_specific_templates_value_holds_the_parameters_it_is_given() -> None:
    """A specific template whose values are partly its parameters."""
    report = job_report_of.value(5, "late", 9)
    assert report == job_status.JobStatusPayload(
        elapsed_ms=5,
        note="late",
        codes=[9, 255],
        tag=None,
        retry=2,
        count=4_294_967_295,
    )
    assert job_report_of.matches(report, 5, "late", 9)
    assert not job_report_of.matches(report, 6, "late", 9), "another elapsed time"
    assert not job_report_of.matches(report, 5, "early", 9), "another note"
    assert not job_report_of.matches(report, 5, "late", 8), "another code"


def test_an_enum_literal_and_a_reference_to_a_specific_template_build_and_match() -> None:
    """An enum literal, and a reference to a specific template, which builds the
    field with that template's value."""
    line = order_line_exact.value()
    assert line == order_line.OrderLinePayload(sku="A-1", quantity=2, color=paint_color.PaintColor.RED)
    assert order_line_exact.matches(line)
    assert not order_line_exact.matches(dataclasses.replace(line, color=paint_color.PaintColor.BLUE)), (
        "another color"
    )
    assert not order_line_exact.matches(dataclasses.replace(line, quantity=3)), "another quantity"


def test_a_reference_to_a_range_holds_the_bound_its_argument_gives() -> None:
    """A reference whose template matches a range up to the argument it binds."""
    line = order_line.OrderLinePayload(sku="A-1", quantity=5, color=paint_color.PaintColor.BLUE)
    assert order_line_bounded.matches(line, 5), "a quantity at the bound"
    assert not order_line_bounded.matches(line, 4), "a quantity past the bound"
    green = dataclasses.replace(line, color=paint_color.PaintColor.GREEN)
    assert not order_line_bounded.matches(green, 5), "another color"


def test_a_template_matches_through_a_reference_to_one_that_is_not_specific() -> None:
    """A template that is specific in itself and references one that is not
    matches by the referenced template's range."""
    status = dataclasses.replace(job_report.value(), count=60)
    assert report_within.matches(status, 60), "a count at the limit"
    assert not report_within.matches(status, 59), "a count past the limit"


def test_a_length_on_a_text_counts_a_multi_byte_character_as_one() -> None:
    """A length restriction on a text counts characters, so a text whose bytes
    pass its upper bound matches while its characters do not; a range with a
    literal lower bound."""

    def card(last4: str, amount: int) -> card_payment.CardPaymentPayload:
        return card_payment.CardPaymentPayload(last4=last4, amount=amount)

    # Two characters in three bytes, and in four.
    for last4 in (E_ACUTE + "1", E_ACUTE * 2, "123"):
        assert card_within.matches(card(last4, 50), 100), last4
    for last4 in ("1", E_ACUTE, "1234"):
        assert not card_within.matches(card(last4, 50), 100), last4
    for amount, held in ((1, True), (0, False), (100, True), (101, False)):
        assert card_within.matches(card("12", amount), 100) is held, amount


def test_a_byte_string_literal_is_its_octets() -> None:
    """A byte string written in hex, compared by its octets rather than by the
    object that holds them."""
    voucher = voucher_is.value()
    assert voucher.voucher_code == bytes([0x00, 0xFF, 0x10, 0xA0])
    assert voucher_is.matches(voucher)
    # Another object of the same octets matches too.
    copy = bytes(bytearray(voucher.voucher_code))
    assert voucher_is.matches(voucher_payment.VoucherPaymentPayload(voucher_code=copy)), (
        "a copy of the octets matches"
    )
    other = bytearray(voucher.voucher_code)
    other[3] = 0xA1
    assert not voucher_is.matches(voucher_payment.VoucherPaymentPayload(voucher_code=bytes(other))), (
        "other octets do not match"
    )


def test_a_records_canonical_form_is_written_by_the_types_its_documents_declare() -> None:
    """docs/SCE_TEST_PROFILE.md §12.5: the unit's ``canonical`` writes a record by
    the types its documents declare, in the JSON Canonicalization Scheme: every
    integer as the text of its digits, a list's elements in order, an enum by its
    declared name, a byte string in upper-case hex, an absent field omitted and a
    present one written. ``TYPE`` is the unit name of the record's event schema.
    The Python twin of the Kotlin
    ``anOrdersCanonicalFormIsWrittenByTheTypesItsDocumentsDeclare``, over the
    records Python lowers, which hold no union."""
    report = job_report_of.value(5, "late", 9)
    assert write_jcs(job_report_of.canonical(report)) == (
        '{"codes":["9","255"],"count":"4294967295","elapsed_ms":"5","note":"late","retry":"2"}'
    )
    absent = dataclasses.replace(report, note=None, retry=None, tag=0)
    assert write_jcs(job_report_of.canonical(absent)) == (
        '{"codes":["9","255"],"count":"4294967295","elapsed_ms":"5","tag":"0"}'
    ), "an absent field is omitted, and a present zero is written"
    assert job_report_of.TYPE == "job_status"
    assert write_jcs(order_line_exact.canonical(order_line_exact.value())) == (
        '{"color":"red","quantity":"2","sku":"A-1"}'
    )
    assert order_line_exact.TYPE == "order_line"
    assert write_jcs(voucher_is.canonical(voucher_is.value())) == '{"voucher_code":"00FF10A0"}'
    assert voucher_is.TYPE == "voucher_payment"
