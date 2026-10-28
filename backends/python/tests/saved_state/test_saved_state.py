# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The saved-state model of the Python runtime: the texts every backend reads
(docs/adr/0045, section 8), the refusals each member that breaks a format is held
to, the value types a state is made of, and the upgrade of a state saved before a
record schema or an enum was revised (docs/adr/0044). Cross-language sibling:
``backends/go/runtime/saved_state_test.go``."""

from __future__ import annotations

import re
from pathlib import Path
from typing import Dict

import pytest

from sce_runtime.saved_json import JsonNumber, parse_json, write_json
from sce_runtime.saved_state import (
    AddedField,
    AddedVariant,
    EnumHolder,
    EnumLineage,
    RecordHolder,
    RecordLineage,
    SAVED_FORMAT_1,
    SAVED_FORMAT_2,
    SavedInvokes,
    SavedRevision,
    SavedShape,
    SavedState,
    StateRefusal,
    WidenedField,
    refusal_within,
    saved_state_from_json,
    upgrade_saved_state,
)
from sce_runtime.saved_values import (
    saved_read_bool,
    saved_read_bounded_string,
    saved_read_bytes,
    saved_read_float32,
    saved_read_float64,
    saved_read_int8,
    saved_read_int64,
    saved_read_list,
    saved_read_string,
    saved_read_uint8,
    saved_read_uint32,
    saved_read_uint64,
    saved_write_bytes,
    saved_write_float32,
    saved_write_float64,
    saved_write_int64,
    saved_write_uint8,
    saved_write_uint64,
)
from sce_runtime import session_id

_SAVED = Path(__file__).resolve().parents[4] / "sce-build" / "tests" / "fixtures" / "static_datamodel" / "saved"


def shared_saved_texts() -> Dict[Path, str]:
    """The shared saved-state texts every backend reads: format 2 in ``saved/`` and
    the unchanged format 1 in ``saved/format1/``."""
    texts: Dict[Path, str] = {}
    for directory in (_SAVED, _SAVED / "format1"):
        for path in sorted(directory.glob("*.json")):
            texts[path] = path.read_text(encoding="utf-8").strip()
    assert len(texts) >= 20, f"the shared saved states lost instances: {len(texts)} read"
    return texts


@pytest.mark.parametrize("path", sorted(shared_saved_texts()), ids=lambda p: f"{p.parent.name}/{p.name}")
def test_every_shared_saved_text_reads_and_writes_back_as_it_was(path: Path) -> None:
    text = shared_saved_texts()[path]
    assert saved_state_from_json(text).to_json() == text


def test_a_format_one_text_is_of_format_one_and_holds_no_session() -> None:
    saved = saved_state_from_json(shared_saved_texts()[_SAVED / "format1" / "static_invoke_working.json"])
    assert saved.format() == SAVED_FORMAT_1
    assert saved.invokes.ids
    assert saved.session_id == ""
    assert saved.bound_session_id() is None


def test_a_tree_text_holds_every_child_with_its_session() -> None:
    saved = saved_state_from_json(shared_saved_texts()[_SAVED / "static_invoke_tree_working.json"])
    assert saved.format() == SAVED_FORMAT_2
    assert saved.invokes.children
    child = saved.invokes.children[0]
    assert child.session.session_id == f"{saved.session_id}.{child.invoke_id}"
    assert saved.invoke_start(child.id) == child.start


_MINIMAL = (
    '"shape":"s","sessionid":"root","configuration":["a"],"current":"a","variables":{},'
    '"history":{},"pending":[],"invokes":[],"invokestarts":{},"hostinvokes":[],"hostinvoketoken":"0",'
    '"sendseq":"0","external":[]'
)


@pytest.mark.parametrize(
    "text, reason",
    [
        ('{"format":', "the saved state is not JSON (it ends where a value was expected)"),
        ("[]", "the saved state is not an object"),
        ("{}", "the saved state has no 'format'"),
        ('{"format":3}', "the saved state is format 3, and this runtime reads formats 1 and 2"),
        ('{"format":"2"}', 'the saved state is format "2", and this runtime reads formats 1 and 2'),
        (
            '{"format":2,' + _MINIMAL.replace('"root"', '"a b"', 1) + "}",
            "'sessionid' (a b) is not an XML Nmtoken, which a session id is",
        ),
        (
            '{"format":2,' + _MINIMAL.replace('"invokestarts":{}', '"invokestarts":{"w":"01"}', 1) + "}",
            "'invokestarts.w' (01) is not a count of starts, a whole number from 1",
        ),
        (
            '{"format":2,' + _MINIMAL.replace('"hostinvoketoken":"0"', '"hostinvoketoken":"x"', 1) + "}",
            "'hostinvoketoken' (x) is not a whole number a token can be",
        ),
        (
            '{"format":2,' + _MINIMAL + ',"revisions":{}}',
            "'revisions' names no schema; a state whose record schemas are all in their first revision leaves it out",
        ),
        (
            '{"format":1,"shape":"s","configuration":["a"],"current":"a","variables":{},"history":{},'
            '"pending":[{"due":"5","act":"parent","event":"e","data":"","sendid":"","origin":""}],"invokes":[],'
            '"hostinvokes":[],"hostinvoketoken":"0","sendseq":"0","external":[]}',
            "'pending[0].act' is 'parent', which is not raise, internal or host, the acts of format 1",
        ),
        (
            '{"format":2,' + _MINIMAL.replace('"pending":[]', '"pending":[{"due":"5","act":"fax"}]', 1) + "}",
            "'pending[0].act' is 'fax', which is not raise, internal, host, parent or child",
        ),
    ],
)
def test_a_text_the_formats_do_not_carry_is_refused_for_the_member_that_breaks_it(text: str, reason: str) -> None:
    with pytest.raises(StateRefusal) as refused:
        saved_state_from_json(text)
    assert str(refused.value) == reason


def test_a_child_session_is_refused_by_its_path() -> None:
    def with_session(session: str) -> str:
        return '{"id":"w","start":"1","invokeid":"w","document":"w","session":{' + session + "}}"

    def holding(child: str) -> str:
        return _MINIMAL.replace('"invokes":[]', '"invokes":[' + child + "]", 1)

    bad = with_session(_MINIMAL.replace('"configuration":["a"]', '"configuration":[1]', 1))
    with pytest.raises(StateRefusal) as refused:
        saved_state_from_json('{"format":2,' + holding(bad) + "}")
    assert str(refused.value) == "the session at invokes[0].session: 'configuration' is not a text"
    with pytest.raises(StateRefusal) as grandchild:
        saved_state_from_json('{"format":2,' + holding(with_session(holding(bad))) + "}")
    assert (
        str(grandchild.value)
        == "the session at invokes[0].session.invokes[0].session: 'configuration' is not a text"
    )


def test_a_child_that_states_a_format_is_refused() -> None:
    child = '{"id":"w","start":"1","invokeid":"w","document":"w","session":{"format":2,' + _MINIMAL + "}}"
    text = '{"format":2,' + _MINIMAL.replace('"invokes":[]', '"invokes":[' + child + "]", 1) + "}"
    with pytest.raises(StateRefusal) as refused:
        saved_state_from_json(text)
    assert str(refused.value) == "'invokes[0].session' states a format, which only the root of a tree does"


def test_a_json_number_keeps_the_spelling_it_was_written_in() -> None:
    text = '{"a":[1,-0,1.50,1E+2,9223372036854775808],"b":null,"c":"\\u00e9\\ud83d\\ude00"}'
    # A character written as a \u escape is written as the character.
    assert write_json(parse_json(text)) == '{"a":[1,-0,1.50,1E+2,9223372036854775808],"b":null,"c":"é😀"}'


def test_a_json_object_keeps_the_order_its_members_were_written_in() -> None:
    assert "".join(parse_json('{"z":1,"a":2,"m":3}')) == "zam"


@pytest.mark.parametrize(
    "text, want",
    [
        ('{"a":1,"a":2}', 'not JSON (the field "a" appears twice)'),
        ("1 2", "more than one JSON value"),
        ("[1,]", "not JSON (unexpected ']')"),
        ('"\\q"', "not JSON (unknown escape '\\q')"),
        ('"\\u12"', "not JSON (a \\u escape is short)"),
        ('"\\u12zz"', "not JSON (a \\u escape is not hex)"),
        ("-", "not JSON (a number has no digits)"),
        ('{"a" 1}', "not JSON (a ':' was expected)"),
        ("tru", "not JSON (expected 'true')"),
    ],
)
def test_a_text_that_is_not_one_json_value_is_refused(text: str, want: str) -> None:
    from sce_runtime.saved_json import JsonError

    with pytest.raises(JsonError) as refused:
        parse_json(text)
    assert str(refused.value) == want


def test_a_lone_surrogate_reads_as_the_replacement_character() -> None:
    assert parse_json('"\\ud800x\\udc00"') == "\ufffdx\ufffd"


def test_every_value_type_is_written_as_its_saved_form_and_read_back_as_it_was() -> None:
    assert write_json(saved_write_uint8(200)) == "200"
    assert write_json(saved_write_int64(-9223372036854775808)) == '"-9223372036854775808"'
    assert write_json(saved_write_uint64(18446744073709551615)) == '"18446744073709551615"'
    assert write_json(saved_write_float32(0.30000001192092896)) == "0.30000001192092896"
    assert write_json(saved_write_float64(5.0)) == "5"
    assert write_json(saved_write_float64(1e21)) == "1e+21"
    assert write_json(saved_write_bytes(bytes([0, 0xE9, 0xFF]))) == '"\\u0000\u00e9\u00ff"'
    reads = [
        (saved_read_uint8, "255", 255),
        (saved_read_int8, "-128", -128),
        (saved_read_uint32, "4294967295", 4294967295),
        (saved_read_int64, '"-9223372036854775808"', -9223372036854775808),
        (saved_read_uint64, '"18446744073709551615"', 18446744073709551615),
        (saved_read_float64, "1.5", 1.5),
        (saved_read_float32, '"-Infinity"', float("-inf")),
        (saved_read_float64, "1e999", float("inf")),
        (saved_read_bool, "true", True),
    ]
    for read, text, want in reads:
        assert read(parse_json(text), "x") == want, text


def test_a_32_bit_real_is_rounded_to_the_single_it_is() -> None:
    assert saved_read_float32(parse_json("0.3"), "x") == 0.30000001192092896


@pytest.mark.parametrize(
    "text, read, want",
    [
        ("256", saved_read_uint8, "'x' (256) is not a whole number its type can hold"),
        ("-1", saved_read_uint8, "'x' (-1) is not a whole number its type can hold"),
        ("1.5", saved_read_uint8, "'x' (1.5) is not a whole number its type can hold"),
        ('"1"', saved_read_uint8, "'x' is not a number"),
        ("null", saved_read_uint8, "'x' is absent (null), and its type is not optional"),
        ("5", saved_read_int64, "'x' is not a 64-bit whole number written as a text"),
        (
            '"9223372036854775808"',
            saved_read_int64,
            "'x' (9223372036854775808) is not a whole number its type can hold",
        ),
        ("1", saved_read_string, "'x' is not a text"),
        ('"true"', saved_read_bool, "'x' is not a truth value"),
    ],
)
def test_a_value_of_another_kind_or_another_width_is_refused_not_converted(text: str, read, want: str) -> None:
    with pytest.raises(StateRefusal) as refused:
        read(parse_json(text), "x")
    assert str(refused.value) == want


def test_a_text_and_a_list_are_held_to_the_bounds_the_machine_has() -> None:
    with pytest.raises(StateRefusal) as past_text:
        saved_read_bounded_string("h\u00e9llo", "t", 5)
    assert str(past_text.value) == "'t' holds 6 UTF-8 bytes, past the 5 it is bounded by"
    with pytest.raises(StateRefusal) as no_byte:
        saved_read_bytes("a\u0100", "b", 8)
    assert str(no_byte.value) == "'b' holds U+0100, which is past U+00FF and so no byte"
    with pytest.raises(StateRefusal) as past_bytes:
        saved_read_bytes("abc", "b", 2)
    assert str(past_bytes.value) == "'b' holds 3 bytes, past the 2 it is bounded by"
    items = parse_json("[1,2,300]")
    with pytest.raises(StateRefusal) as element:
        saved_read_list(items, "l", 8, saved_read_uint8)
    assert str(element.value) == "'l[2]' (300) is not a whole number its type can hold"
    with pytest.raises(StateRefusal) as past_list:
        saved_read_list(items, "l", 2, saved_read_uint8)
    assert str(past_list.value) == "'l' holds 3 elements, past the 2 it is bounded by"


def test_an_upgrade_gives_a_record_the_defaults_of_the_fields_a_later_revision_added() -> None:
    digest = "a" * 64
    shape = SavedShape(
        first="first",
        revised="revised",
        format2_first="first2",
        format2_revised="revised2",
        records=[
            RecordLineage(
                event="shown",
                revision=2,
                revisions=[digest],
                added=[AddedField(id="weekday", since=2, default="0")],
                holders=[RecordHolder("one", False), RecordHolder("many", True)],
            )
        ],
    )
    given = '{"one":{"year":2027},"many":[{"year":1},{"year":2}]}'
    saved = SavedState(shape="first2", variables=parse_json(given), invokes=SavedInvokes(resumed=True))
    upgraded = upgrade_saved_state(saved, shape)
    assert (
        write_json(upgraded.variables)
        == '{"one":{"year":2027,"weekday":0},"many":[{"year":1,"weekday":0},{"year":2,"weekday":0}]}'
    )
    assert write_json(saved.variables) == given, "the upgrade changed the state it was given"

    # A record that already holds a field its revision does not declare.
    saved.variables = parse_json('{"one":{"year":1,"weekday":3},"many":[]}')
    with pytest.raises(StateRefusal) as held:
        upgrade_saved_state(saved, shape)
    assert str(held.value) == "'one' holds 'weekday', which revision 1 of 'shown' does not declare: revision 2 added it"

    # A state of another shape.
    saved.shape = "other"
    with pytest.raises(StateRefusal) as other:
        upgrade_saved_state(saved, shape)
    assert str(other.value) == "the saved state is of a document of shape other, and this machine's is first2"

    # A revision later than the machine's.
    saved.shape = "revised2"
    saved.revisions = [SavedRevision(key="shown", revision=3, fields=digest)]
    with pytest.raises(StateRefusal) as later:
        upgrade_saved_state(saved, shape)
    assert str(later.value) == (
        "the saved state was saved at revision 3 of 'shown', later than this machine's 2: "
        "a machine does not restore what it does not know"
    )


def test_a_null_saved_before_the_field_was_widened_is_refused() -> None:
    # docs/adr/0055, section 6: the third revision of `shown` widened `year`, a
    # field of the first, and `weekday`, which the second added, to optional.
    second, third = "2" * 64, "3" * 64
    shape = SavedShape(
        first="first",
        revised="revised",
        format2_first="first2",
        format2_revised="revised2",
        records=[
            RecordLineage(
                event="shown",
                revision=3,
                revisions=[second, third],
                added=[AddedField(id="weekday", since=2, default="0")],
                widened=[
                    WidenedField(id="year", since=1, optional_since=3),
                    WidenedField(id="weekday", since=2, optional_since=3),
                ],
                holders=[RecordHolder("one", False), RecordHolder("many", True)],
            )
        ],
    )

    def saved_at(revision: int, one: str, many: str) -> SavedState:
        saved = SavedState(
            shape="first2" if revision == 1 else "revised2",
            variables=parse_json(f'{{"one":{one},"many":{many}}}'),
            invokes=SavedInvokes(resumed=True),
        )
        if revision > 1:
            saved.revisions = [SavedRevision(key="shown", revision=revision, fields=second if revision == 2 else third)]
        return saved

    # A record variable, a field of the first revision, saved at the first.
    with pytest.raises(StateRefusal) as variable:
        upgrade_saved_state(saved_at(1, '{"year":null}', "[]"), shape)
    assert str(variable.value) == (
        "'one.year' is null, which revision 1 of 'shown' holds as a value: revision 3 widened it"
    )
    # An element of a list of records.
    with pytest.raises(StateRefusal) as element:
        upgrade_saved_state(saved_at(1, '{"year":4}', '[{"year":1},{"year":null}]'), shape)
    assert "'many[1].year' is null" in str(element.value)
    # A field the second revision added, saved at the second.
    with pytest.raises(StateRefusal) as added:
        upgrade_saved_state(saved_at(2, '{"year":4,"weekday":null}', "[]"), shape)
    assert str(added.value) == (
        "'one.weekday' is null, which revision 2 of 'shown' holds as a value: revision 3 widened it"
    )
    # The field the second added, held by a record saved at the first, is refused as
    # a field that revision does not declare, which is the truer reason.
    with pytest.raises(StateRefusal) as undeclared:
        upgrade_saved_state(saved_at(1, '{"year":4,"weekday":null}', "[]"), shape)
    assert "'one' holds 'weekday'" in str(undeclared.value)

    # Saved at the revision that widened both, `null` is absent.
    kept = saved_at(3, '{"year":null,"weekday":null}', '[{"year":null,"weekday":2}]')
    assert write_json(upgrade_saved_state(kept, shape).variables) == write_json(kept.variables)
    # A record saved before the widening, each field present, restores as it was.
    earlier = upgrade_saved_state(saved_at(1, '{"year":4}', '[{"year":1}]'), shape)
    assert write_json(earlier.variables) == '{"one":{"year":4,"weekday":0},"many":[{"year":1,"weekday":0}]}'
    # What is no record where one is held is left to the variable's own read.
    assert write_json(upgrade_saved_state(saved_at(1, "null", "[{}]"), shape).variables) == (
        '{"one":null,"many":[{"weekday":0}]}'
    )


def test_an_enum_variant_a_later_revision_added_is_refused_in_a_state_saved_before_it() -> None:
    shape = SavedShape(
        first="",
        revised="",
        format2_first="first2",
        format2_revised="revised2",
        enums=[
            EnumLineage(
                key="enum:status",
                revision=2,
                revisions=["b" * 64],
                added=[AddedVariant(name="Done", since=2)],
                holders=[EnumHolder(variable="item", field="status"), EnumHolder(variable="state")],
            )
        ],
    )
    saved = SavedState(
        shape="first2",
        variables=parse_json('{"item":{"status":"Done"},"state":"Open"}'),
        invokes=SavedInvokes(resumed=True),
    )
    with pytest.raises(StateRefusal) as refused:
        upgrade_saved_state(saved, shape)
    assert str(refused.value) == (
        "'item.status' holds 'Done', which revision 1 of 'enum:status' does not declare: revision 2 added it"
    )


def test_a_session_id_is_an_xml_nmtoken_and_a_drawn_one_is_a_version_4_uuid() -> None:
    for id in ("fixture-root", "a.b:c_d", "\u00e91", "x\u00b7y"):
        assert session_id.refusal_of(id) is None, id
    assert session_id.refusal_of("") == (
        "the session id is empty, and a session id is one name character or more (an XML Nmtoken)"
    )
    assert session_id.refusal_of("a b") == (
        "the session id holds U+0020, which is not a name character: a session id is an XML Nmtoken "
        "(letters, digits, '.', '-', '_', ':')"
    )
    pattern = re.compile(r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$")
    first, second = session_id.draw(), session_id.draw()
    assert pattern.match(first) and session_id.is_nmtoken(first) and first != second


def test_a_refusal_inside_a_child_names_the_session_it_is_about() -> None:
    inner = refusal_within(StateRefusal("'x' is not a text"), "invokes[1].session")
    assert str(inner) == "the session at invokes[1].session: 'x' is not a text"
    outer = refusal_within(inner, "invokes[0].session")
    assert str(outer) == "the session at invokes[0].session.invokes[1].session: 'x' is not a text"
