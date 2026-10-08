# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""sce_forge_runtime.content_line — the same properties the Rust runtime's own
tests pin (backends/rust/forge-runtime/src/content_line.rs): unfolding wherever
the sender cut, names read without regard to case, a nested component and an
undeclared property skipped whole, truncation told from malformation, TEXT
escapes, a value held to its type, and a writer that folds before the unit that
would pass 75 octets and refuses a value a line could not carry. What a
generated codec does with these is the conformance harness's
(codec_content_line_event), which this file does not repeat."""

import pytest

from sce_forge_runtime import content_line as cl
from sce_forge_runtime.codec import BytearraySink, NeedMoreBytes


def _read_all(text: bytes):
    reader = cl.ContentLineReader.begin(text, "VEVENT")
    seen = []
    while True:
        line = reader.next_property()
        if line is None:
            return seen
        if line.is_named("UID"):
            seen.append(("UID", line.read_string(32, False)))
        elif line.is_named("SUMMARY"):
            seen.append(("SUMMARY", line.read_string(32, True)))


def _one(value: bytes):
    return _read_all(b"BEGIN:VEVENT\r\nSUMMARY:" + value + b"\r\nEND:VEVENT\r\n")


def test_a_component_is_read_property_by_property_and_stops_after_its_end():
    text = (
        b"BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:a1\r\nSUMMARY:hi\r\n"
        b"END:VEVENT\r\nEND:VCALENDAR\r\n"
    )
    reader = cl.ContentLineReader.begin(text, "VEVENT")
    seen = 0
    while (line := reader.next_property()) is not None:
        assert line.is_named("UID") or line.is_named("SUMMARY")
        seen += 1
    assert seen == 2
    assert text[reader.consumed() :] == b"END:VCALENDAR\r\n"


def test_names_are_matched_without_regard_to_case_and_lf_ends_a_line():
    assert _read_all(b"begin:vevent\nuid:x\nend:Vevent\n") == [("UID", "x")]


def test_a_fold_is_removed_wherever_the_sender_cut():
    assert _read_all(b"BEGIN:VEVENT\r\nUI\r\n D:ab\r\n\tcd\r\n e\r\nEND:VEVENT\r\n") == [("UID", "abcde")]
    assert _one(b"\xc3\r\n \xa9") == [("SUMMARY", "\u00e9")]


def test_a_nested_component_and_an_undeclared_property_are_skipped_whole():
    text = (
        b'BEGIN:VEVENT\r\nX-NOISE;A="b:c":d\r\nBEGIN:VALARM\r\nUID:inner\r\nBEGIN:X\r\nEND:X\r\n'
        b"END:VALARM\r\nUID:outer\r\nEND:VEVENT\r\n"
    )
    assert _read_all(text) == [("UID", "outer")]


@pytest.mark.parametrize(
    "text",
    [
        b"",
        b"BEGIN:VCALENDAR\r\n",
        b"BEGIN:VEVENT",
        b"BEGIN:VEVENT\r\nUID:a",
        b"BEGIN:VEVENT\r\nUID:a\r\n",
        b"BEGIN:VEVENT\r\nUID:a\r\nEND:VEVEN",
        b"BEGIN:VEVENT\r\nBEGIN:VALARM\r\nEND:VALARM\r\n",
    ],
)
def test_an_input_that_ends_early_needs_more_bytes(text):
    with pytest.raises(NeedMoreBytes):
        _read_all(text)


def test_the_end_line_needs_no_line_break_after_it():
    assert _read_all(b"BEGIN:VEVENT\r\nUID:a\r\nEND:VEVENT") == [("UID", "a")]


@pytest.mark.parametrize(
    "text",
    [
        b"BEGIN:VEVENT\r\n\r\nEND:VEVENT\r\n",
        b"BEGIN:VEVENT\r\nUID\r\nEND:VEVENT\r\n",
        b"BEGIN:VEVENT\r\n=x:y\r\nEND:VEVENT\r\n",
        b"BEGIN:VEVENT\r\nUID:a\r\nEND:VTODO\r\n",
        b"BEGIN:VEVENT\r\nUID;TZID:a\r\nEND:VEVENT\r\n",
        b'BEGIN:VEVENT\r\nUID;TZID="a:b\r\nEND:VEVENT\r\n',
        b'BEGIN:VEVENT\r\nUID;TZID="a"b:c\r\nEND:VEVENT\r\n',
        b'BEGIN:VEVENT\r\nUID;TZID=a"b:c\r\nEND:VEVENT\r\n',
    ],
)
def test_a_line_the_grammar_does_not_admit_is_malformed(text):
    with pytest.raises(cl.LineMalformed):
        _read_all(text)


def test_a_text_unescapes_and_a_bad_escape_is_refused():
    assert _one(b"a\\\\b\\;c\\,d\\ne\\Nf;g,h") == [("SUMMARY", "a\\b;c,d\ne\nf;g,h")]
    for bad in (b"a\\xb", b"a\\"):
        with pytest.raises(cl.LineBadEscape):
            _one(bad)
    # Without sce:value="text" a backslash is a byte like another.
    reader = cl.ContentLineReader.begin(b"BEGIN:VEVENT\r\nUID:a\\nb\r\nEND:VEVENT\r\n", "VEVENT")
    assert reader.next_property().read_string(8, False) == "a\\nb"


@pytest.mark.parametrize("bad", [b"a\x01b", b"a\rb", b"a\x7fb", b"a\xffb"])
def test_a_control_character_and_bad_utf8_are_refused(bad):
    with pytest.raises(cl.LineBadValue):
        _one(bad)


def test_a_tab_and_a_value_at_its_bound_are_read_and_a_longer_one_is_refused():
    assert _one(b"a\tb") == [("SUMMARY", "a\tb")]
    assert _one(b"x" * 32) == [("SUMMARY", "x" * 32)]
    with pytest.raises(cl.LineTooLong):
        _one(b"x" * 33)
    # The size is that of the unescaped text.
    assert _one(b"\\;" * 32) == [("SUMMARY", ";" * 32)]


def test_parameters_are_read_by_name_quoted_or_not_and_the_rest_are_skipped():
    text = (
        b'BEGIN:VEVENT\r\nDTSTART;X-A=1;TZID="Europe/Seoul:KST";X-B="p,q",r:20260101T000000\r\n'
        b"END:VEVENT\r\n"
    )
    reader = cl.ContentLineReader.begin(text, "VEVENT")
    line = reader.next_property()
    assert line.is_named("dtstart")
    tzid = None
    while line.next_param():
        if line.param_is("tzid"):
            tzid = line.read_param_string(32)
    assert tzid == "Europe/Seoul:KST"
    assert line.read_string(32, False) == "20260101T000000"
    assert reader.next_property() is None


@pytest.mark.parametrize(
    "param, error",
    [
        (b"TZID=a,b", cl.LineBadValue),
        (b'TZID="a","b"', cl.LineBadValue),
        (b"TZID=abcdefghi", cl.LineTooLong),
        (b"TZID=a\x02", cl.LineBadValue),
    ],
)
def test_a_parameter_of_two_values_or_past_its_bound_is_refused_where_it_is_read(param, error):
    text = b"BEGIN:VEVENT\r\nDTSTART;" + param + b":x\r\nEND:VEVENT\r\n"
    line = cl.ContentLineReader.begin(text, "VEVENT").next_property()
    assert line.next_param()
    with pytest.raises(error):
        line.read_param_string(8)


def _property(text: bytes) -> "cl.ContentLineProperty":
    return cl.ContentLineReader.begin(b"BEGIN:VEVENT\r\nN:" + text + b"\r\nEND:VEVENT\r\n", "VEVENT").next_property()


def test_integers_hold_their_type_or_are_refused():
    assert _property(b"255").read_uint(255) == 255
    assert _property(b"+007").read_uint(255) == 7
    for bad in (b"256", b"-0", b"", b"1x", b" 1", b"9" * 41):
        with pytest.raises(cl.LineBadValue):
            _property(bad).read_uint(2**64 - 1 if bad == b"9" * 41 else 255)
    assert _property(b"18446744073709551615").read_uint(2**64 - 1) == 2**64 - 1
    with pytest.raises(cl.LineBadValue):
        _property(b"18446744073709551616").read_uint(2**64 - 1)
    assert _property(b"-128").read_int(-128, 127) == -128
    assert _property(b"-0").read_int(-128, 127) == 0
    with pytest.raises(cl.LineBadValue):
        _property(b"-129").read_int(-128, 127)


def test_booleans_are_true_or_false_in_either_case():
    assert _property(b"true").read_bool() is True
    assert _property(b"False").read_bool() is False
    for bad in (b"yes", b"TRUEE"):
        with pytest.raises(cl.LineBadValue):
            _property(bad).read_bool()


def test_a_value_is_read_once():
    line = _property(b"a")
    line.read_string(4, False)
    with pytest.raises(cl.LineMalformed):
        line.read_string(4, False)


def _written(write) -> str:
    out = bytearray()
    w = cl.ContentLineWriter(BytearraySink(out), "VEVENT")
    w.begin()
    write(w)
    w.finish()
    return out.decode("utf-8")


def test_a_property_is_written_with_its_parameters_and_a_value_that_is_escaped():
    def write(w):
        w.property("UID")
        w.string("a1", False, 8)
        w.property("DTSTART")
        w.param("TZID", "Europe/Seoul", 64)
        w.param("X-Q", "a:b", 8)
        w.string("20260101T000000", False, 32)
        w.property("SUMMARY")
        w.string("a\\b;c,d\ne", True, 32)
        w.property("N")
        w.uint(2**64 - 1)
        w.property("M")
        w.integer(-(2**63))
        w.property("B")
        w.boolean(True)

    assert _written(write) == (
        "BEGIN:VEVENT\r\nUID:a1\r\nDTSTART;TZID=Europe/Seoul;X-Q=\"a:b\":20260101T000000\r\n"
        "SUMMARY:a\\\\b\\;c\\,d\\ne\r\nN:18446744073709551615\r\nM:-9223372036854775808\r\nB:TRUE\r\n"
        "END:VEVENT\r\n"
    )


def test_a_line_is_cut_before_the_unit_that_would_pass_75_octets():
    def write(w):
        w.property("SUMMARY")
        w.string("x" * 200, False, 256)

    lines = _written(write).split("\r\n")
    assert len(lines) == 6  # five lines and the empty tail
    # "SUMMARY:" takes 8 of the first 75 octets, leaving 67 of the 200; each
    # continuation line carries its space and 74 more.
    assert len(lines[1]) == 75
    assert len(lines[2]) == 75 and lines[2][0] == " "
    assert len(lines[3]) == 1 + 200 - 67 - 74 and lines[3][0] == " "


def test_a_cut_never_splits_a_character_or_an_escape():
    def write(w):
        w.property("S")
        w.string("x" * 71 + "\u00e9\u00e9", False, 256)
        w.property("T")
        w.string("x" * 72 + ";", True, 256)

    text = _written(write)
    assert "S:" + "x" * 71 + "\u00e9\r\n \u00e9\r\n" in text
    assert "T:" + "x" * 72 + "\r\n \\;\r\n" in text
    reader = cl.ContentLineReader.begin(text.encode("utf-8"), "VEVENT")
    assert reader.next_property().read_string(80, False) == "x" * 71 + "\u00e9\u00e9"
    assert reader.next_property().read_string(80, True) == "x" * 72 + ";"


def test_a_value_a_line_could_not_carry_is_refused_before_it_is_written():
    out = bytearray()
    w = cl.ContentLineWriter(BytearraySink(out), "VEVENT")
    w.begin()
    w.property("P")
    before = len(out)
    for call, error in (
        (lambda: w.string("a\r\nATTENDEE:x", False, 64), cl.LineBadValue),
        (lambda: w.string("a\nb", False, 64), cl.LineBadValue),
        (lambda: w.string("a\rb", True, 64), cl.LineBadValue),
        (lambda: w.string("abcd", False, 3), cl.LineTooLong),
        (lambda: w.string("a\ud800b", False, 64), cl.LineBadValue),
        (lambda: w.param("Q", 'a"b', 8), cl.LineBadValue),
        (lambda: w.param("Q", "a\nb", 8), cl.LineBadValue),
        (lambda: w.param("Q", "abcd", 3), cl.LineTooLong),
    ):
        with pytest.raises(error):
            call()
    assert len(out) == before


@pytest.mark.parametrize("value", ["", "plain", "a;b,c\\d\ne", "caf\u00e9 \U0001F600", "y" * 150])
def test_what_is_written_is_read_back(value):
    def write(w):
        w.property("SUMMARY")
        w.string(value, True, 256)

    text = _written(write).encode("utf-8")
    reader = cl.ContentLineReader.begin(text, "VEVENT")
    assert reader.next_property().read_string(256, True) == value
    assert reader.next_property() is None
    assert reader.consumed() == len(text)
