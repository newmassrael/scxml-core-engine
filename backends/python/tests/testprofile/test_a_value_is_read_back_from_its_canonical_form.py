# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The readers of the canonical form of a value (docs/SCE_TEST_PROFILE.md
§12.5; docs/SCE_TTCN3_LANE.md §10.3), ``sce_runtime.testprofile.from_canonical``:
each reads the one spelling its writer in ``canonical`` gives and refuses any
other by name.

Each case is the twin of the Rust case of the same name in
``backends/rust/runtime/src/test/from_canonical.rs`` and of the Kotlin case in
``TraceTest.kt``, named ``test_<case>`` after the Rust one, with the same
refusals. The last cases hold what only a Python value has to: a ``bool`` is an
``int`` there, and a text can hold a surrogate standing alone.
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any, Callable, List, Optional

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "runtime"))

from sce_runtime.saved_json import JsonNumber, parse_json, write_jcs  # noqa: E402 -- path inserted above
from sce_runtime.testprofile import TraceRefusal, canonical  # noqa: E402
from sce_runtime.testprofile import from_canonical as read  # noqa: E402


def reason(attempt: Callable[[], Any]) -> str:
    """The reason of the refusal ``attempt`` raises."""
    with pytest.raises(TraceRefusal) as refused:
        attempt()
    return refused.value.reason


def test_an_integer_is_read_from_the_text_of_its_digits_alone() -> None:
    """An integer is read from the one spelling the writer gives it, at every
    width, and any other spelling, or a value past its type, is refused, naming
    where it stands."""
    assert read.integer(canonical.integer(0), read.UINT8, "n") == 0
    assert read.integer(canonical.integer(18446744073709551615), read.UINT64, "n") == 18446744073709551615
    assert read.integer(canonical.integer(-9223372036854775808), read.INT64, "n") == -9223372036854775808
    # The digits of other scripts and an underscore, which `int` reads, are
    # no canonical spelling either.
    for spelling in ["007", "+7", "-0", "7.0", "", " 7", "1_0", "٧"]:
        refusal = reason(lambda: read.integer(spelling, read.UINT32, "job_reply.code"))
        assert refusal.startswith("job_reply.code: "), refusal
        assert "decimal digits" in refusal, (spelling, refusal)
    assert "past the range" in reason(lambda: read.integer("256", read.UINT8, "n"))
    assert "past the range" in reason(lambda: read.integer("-1", read.UINT32, "n"))
    assert "a number is not an integer" in reason(lambda: read.integer(JsonNumber("7"), read.UINT32, "n"))
    # Past a 128-bit integer, as the Rust reader of the same text refuses it.
    assert "decimal digits" in reason(lambda: read.integer("1" + "0" * 39, read.UINT64, "n"))


def test_a_bool_is_a_truth_value() -> None:
    assert read.boolean(canonical.boolean(True), "b") is True
    assert "a text is not a truth value" in reason(lambda: read.boolean("true", "b"))


def test_a_text_is_read_within_its_bound() -> None:
    """A text is read within its bound in bytes of UTF-8, not in characters."""
    assert read.text(canonical.text("abc"), 3, "t") == "abc"
    assert read.text("é", 2, "t") == "é"
    assert reason(lambda: read.text("abcd", 3, "t")) == "t: a text of 4 bytes is past the 3 its type holds"
    assert "a text of 2 bytes is past the 1" in reason(lambda: read.text("é", 1, "t"))
    assert "null is not a text" in reason(lambda: read.text(None, 3, "t"))


def test_a_byte_string_is_read_from_upper_case_hex() -> None:
    """A byte string is read from upper-case hex alone, the writer's spelling,
    and back to the same text."""
    written = canonical.byte_string(bytes([0x00, 0xAB, 0x0F, 0xFF]))
    octets = read.byte_string(written, 4, "b")
    assert octets == bytes([0x00, 0xAB, 0x0F, 0xFF])
    assert canonical.byte_string(octets) == written
    assert read.byte_string("", 0, "b") == b""
    # `bytes.fromhex` would read the lower case and the space.
    for spelling in ["00ab", "0", "0G", "00 AB"]:
        assert "upper-case hex" in reason(lambda: read.byte_string(spelling, 4, "b")), spelling
    assert "past the 3" in reason(lambda: read.byte_string(written, 3, "b"))


def test_an_enum_is_read_by_its_declared_name() -> None:
    """A closed enum is read by its declared names; an open one by its names
    and by the numbers no variant declares, a declared number being the second
    spelling of its variant."""
    declared = [("red", 0), ("green", 1)]
    assert read.enumerated("green", declared, "c") == 1
    assert "'blue' names no" in reason(lambda: read.enumerated("blue", declared, "c"))

    def open_value(raw: int) -> int:
        return raw

    assert read.open_enumerated("red", declared, read.UINT8, open_value, "c") == 0
    assert read.open_enumerated("7", declared, read.UINT8, open_value, "c") == 7
    assert "written by its name" in reason(lambda: read.open_enumerated("1", declared, read.UINT8, open_value, "c"))
    assert "past the range" in reason(lambda: read.open_enumerated("300", declared, read.UINT8, open_value, "c"))
    assert "and is no number" in reason(lambda: read.open_enumerated("hexagon", declared, read.UINT8, open_value, "c"))


def test_a_record_holds_its_declared_fields_once() -> None:
    """A record holds only the fields it declares; a field whose type is not
    optional is present, and one whose type is optional is read where it is
    present."""
    value = parse_json('{"code":"0","job":"7"}')
    held = read.record(value, ["job", "code"], "job_reply")
    assert read.integer(read.field(held, "job", "job_reply.job"), read.UINT32, "job") == 7
    assert read.optional_field(held, "note", lambda note: read.text(note, 8, "job_reply.note")) is None
    assert read.optional_field(held, "code", lambda code: read.integer(code, read.UINT32, "c")) == 0
    stray = parse_json('{"job":"7","jobs":"1"}')
    assert "'jobs' is no field" in reason(lambda: read.record(stray, ["job", "code"], "job_reply"))
    short = parse_json('{"job":"7"}')
    assert (
        reason(lambda: read.field(short, "code", "job_reply.code"))
        == "job_reply.code: the field is absent, and its type is not optional"
    )
    # An optional field present as null is refused by its type's reader.
    nulled = parse_json('{"note":null}')
    assert "null is not a text" in reason(lambda: read.optional_field(nulled, "note", lambda n: read.text(n, 8, "x")))
    assert "an array is not a record" in reason(lambda: read.record([], ["job"], "job_reply"))


def test_a_list_is_read_within_its_bounds() -> None:
    """A list is read within its bounds, an absent element as null."""
    value = parse_json('["1",null]')
    items = read.elements(value, 1, 2, "l")
    held: List[Optional[int]] = [read.nullable(item, lambda present: read.integer(present, read.UINT8, "l[]")) for item in items]
    assert held == [1, None]
    assert "outside its bounds, 3 to 4" in reason(lambda: read.elements(value, 3, 4, "l"))
    assert "an object is not a list" in reason(lambda: read.elements({}, 0, 1, "l"))


def test_a_union_is_one_member_named_by_its_case() -> None:
    """A union is one member named by its case; a case that holds no record is
    the empty object."""
    card = canonical.union_case("card", canonical.record(("amount", canonical.integer(1))))
    cases = ["card", "cash"]
    variant, value = read.union_case(card, cases, "p")
    assert variant == "card"
    assert write_jcs(value) == '{"amount":"1"}'
    cash = canonical.union_case("cash", None)
    _, empty = read.union_case(cash, cases, "p")
    read.no_record(empty, "p")
    assert "is the empty object" in reason(lambda: read.no_record(card, "p"))
    two = parse_json('{"card":{},"cash":{}}')
    assert "this one has 2" in reason(lambda: read.union_case(two, cases, "p"))
    assert reason(lambda: read.union_case(canonical.union_case("cheque", None), cases, "p")) == (
        "p: 'cheque' names no case of its union"
    )


def test_a_python_bool_is_no_integer_and_a_lone_surrogate_no_text() -> None:
    """What only a Python value has to be held to: ``True`` is an ``int``, and
    is still read as no integer and no text, and a text built in place that
    holds a surrogate standing alone is no text of UTF-8, refused rather than
    raising ``UnicodeEncodeError``."""
    assert "a truth value is not an integer" in reason(lambda: read.integer(True, read.UINT8, "n"))
    assert "a number is not a truth value" in reason(lambda: read.boolean(1, "b"))
    assert "no text of UTF-8" in reason(lambda: read.text("\ud800", 8, "t"))
