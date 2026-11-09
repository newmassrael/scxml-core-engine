# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The JSON Canonicalization Scheme (RFC 8785) of ``write_jcs``, which writes the
test log of the test profile (docs/SCE_TEST_PROFILE.md §12.5).

The first four cases are the twins of the Rust cases of the same names in
``backends/rust/runtime/src/json.rs`` and of the Kotlin
``JsonCanonicalSchemeTest.kt``, with the same values and the same bytes, so the
three writers are held to one text. The others hold what only a Python value
has to: a name held twice once read as UTF-16 code units, a surrogate that
stands alone, and a ``bool`` that is no number. Every character outside ASCII is
written as its escape.
"""

from __future__ import annotations

import pytest

from sce_runtime.saved_json import JsonNumber, NotCanonical, write_jcs, write_json


def test_canonical_members_are_sorted_by_their_utf16_code_units() -> None:
    """RFC 8785 section 3.2.3, the example of the RFC: members are sorted by
    their names as UTF-16 code units, so U+1F600, whose first unit is 0xD83D,
    comes before U+FB33, which an order of code points would put after it;
    U+0080 is written as it is, since only U+0000 to U+001F are escaped."""
    value = {
        "€": "Euro Sign",
        "\r": "Carriage Return",
        "דּ": "Hebrew Letter Dalet With Dagesh",
        "1": "One",
        "\U0001f600": "Emoji: Grinning Face",
        "\u0080": "Control",
        "ö": "Latin Small Letter O With Diaeresis",
    }
    assert write_jcs(value) == (
        '{"\\r":"Carriage Return","1":"One","\u0080":"Control",'
        '"ö":"Latin Small Letter O With Diaeresis","€":"Euro Sign",'
        '"\U0001f600":"Emoji: Grinning Face",'
        '"דּ":"Hebrew Letter Dalet With Dagesh"}'
    )
    assert sorted(value)[-1] == "\U0001f600", (
        "Python orders texts by code point, which puts U+1F600 last: why the scheme sorts "
        "by the UTF-16 encoding"
    )


def test_a_canonical_text_escapes_only_what_json_stringify_escapes() -> None:
    """RFC 8785 section 3.2.2.2: a text is escaped as ``JSON.stringify`` escapes
    it, the five short forms, ``\\u00xx`` in lower-case hex for the other
    controls, and nothing else: not ``/``, not U+007F, not a character past
    U+FFFF."""
    assert write_jcs('\b\t\n\f\r"\\\u0000\u001f\u007f/\U0001f600') == (
        '"\\b\\t\\n\\f\\r\\"\\\\\\u0000\\u001f\u007f/\U0001f600"'
    )


def test_a_canonical_value_has_no_whitespace_and_sorts_every_level() -> None:
    """Nested lists and dicts are written with no whitespace, each dict's
    members sorted at its own level, a list's items in their order."""
    value = {"b": [{"z": None, "a": False}, "y", []], "a": {}}
    assert write_jcs(value) == '{"a":{},"b":[{"a":false,"z":null},"y",[]]}'


def test_a_canonical_number_is_the_double_it_reads_as() -> None:
    """RFC 8785 section 3.2.2.3: a number is written as ECMAScript writes the
    double it reads as; one past the range of a double is refused."""
    for spelling, written in [
        ("1E3", "1000"),
        ("-0", "0"),
        ("0.10", "0.1"),
        ("1e21", "1e+21"),
        ("1e-7", "1e-7"),
    ]:
        assert write_jcs(JsonNumber(spelling)) == written, spelling
    with pytest.raises(NotCanonical) as refused:
        write_jcs(JsonNumber("1e400"))
    assert "1e400" in refused.value.reason


def test_a_value_that_is_no_json_value_is_refused() -> None:
    """What is no JSON value has no form in the scheme: a number not held as a
    ``JsonNumber``, a number in no JSON spelling, and a member named by
    something other than a text, at whatever depth."""
    with pytest.raises(NotCanonical) as bare:
        write_jcs([7])
    assert "int" in bare.value.reason
    with pytest.raises(NotCanonical) as spelled:
        write_jcs(JsonNumber("0x10"))
    assert "0x10" in spelled.value.reason
    with pytest.raises(NotCanonical) as named:
        write_jcs([{1: "one"}])
    assert "not a text" in named.value.reason


def test_a_bool_is_a_truth_value_and_never_a_number() -> None:
    """Python holds ``True`` as the integer 1; the scheme writes it as the truth
    value it is, while the integer 1, which no JSON value of the writer is
    held as, is refused."""
    assert write_jcs([True, False]) == "[true,false]"
    with pytest.raises(NotCanonical):
        write_jcs(1)


def test_a_pair_of_surrogates_is_the_character_it_encodes() -> None:
    """ECMAScript holds a text as UTF-16 code units: a high surrogate followed
    by a low one is the character past U+FFFF they encode, written as it is,
    and sorted, as a name, where that character's units sort."""
    assert write_jcs("😀") == '"\U0001f600"'
    assert write_jcs({"דּ": "b", "😀": "a"}) == '{"\U0001f600":"a","דּ":"b"}'


def test_a_surrogate_that_stands_alone_is_escaped() -> None:
    """``JSON.stringify`` writes a surrogate no pair holds as ``\\uXXXX`` in
    lower-case hex, which ``write_json``, ``json.dumps`` with
    ``ensure_ascii=False`` among them, leaves as it is."""
    assert write_jcs("a\ud800b\udfff") == '"a\\ud800b\\udfff"'
    assert write_jcs("\ude00\ud83d") == '"\\ude00\\ud83d"', "a low surrogate before a high one forms no pair"
    assert write_json("\ud800") == '"\ud800"'


def test_two_names_that_are_the_same_code_units_are_refused() -> None:
    """RFC 8785 section 3.1: an object names each member once. Python holds
    U+1F600 and its two surrogates as two texts, which are one name as code
    units, so the object they name is refused rather than written with one name
    twice."""
    one, two = "\U0001f600", chr(0xD83D) + chr(0xDE00)
    assert one != two, "Python holds the character and its surrogate pair apart"
    with pytest.raises(NotCanonical) as refused:
        write_jcs({one: "a", two: "b"})
    assert "appears twice" in refused.value.reason
