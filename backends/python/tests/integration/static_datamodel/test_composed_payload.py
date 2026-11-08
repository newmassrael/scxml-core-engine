# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A payload value of any bounded type, in the Python runtime (docs/adr/0086).

A value is read at its path, or refused there before anything of it is kept:
every element kind, at list depths 1 to 3, each bound 2, with an empty value, a
maximal one, one past the bound at the innermost and the outermost list, a bad
element, ``null`` where the element is not optional, an unknown key and an older
revision (section 14.3). The oracle is the construction: each case is built with
the value or the path it must come to. A maximal value is written at exactly the
size the calculus gives it (section 2.3) and reads back as itself, and a value a
host builds is refused in its record's ``__post_init__`` (section 5.4).
"""
from __future__ import annotations

import json
import sys
from dataclasses import dataclass
from enum import IntEnum
from typing import Any, List, Tuple

import pytest

from sce_runtime.event_payload import (
    BoundError,
    Bytes,
    ListOf,
    OptionalOf,
    Raw,
    RecordOf,
    RefusalCode,
    Single,
    Text,
    TypedPayloadError,
    Whole,
    WideOf,
    check_value,
    lift,
    list_json,
    record_json,
    value_json,
    wide_json,
)

BOUND = 2
TEXT_BOUND = 3
CONTROLS = "\u0001" * TEXT_BOUND


class _Level(IntEnum):
    """An enum as a `sce-static` machine declares one."""

    LOW = 0
    MEDIUM = 1
    HIGH = 2

    @property
    def sce_name(self) -> str:
        return {0: "low", 1: "medium", 2: "high"}[int(self)]


_REC = RecordOf((("a", Whole(0, 255)), ("b", Text(TEXT_BOUND)), ("c", Whole(0, 255))),
                defaults='{"c":9}')
_REC_LIST = RecordOf((("tags", ListOf(Text(TEXT_BOUND), BOUND)),))

# Each element kind: its type, its maximal value, a bad element of it with the path
# below the element it is refused at and the sentence, and the size of its largest
# JSON text (docs/adr/0086, section 2.1).
_KINDS = {
    "uint8": (Whole(0, 255), 255, ("256000000000", "", "does not fit the width"), 3),
    "uint16": (Whole(0, 65535), 65535, ("256000000000", "", "does not fit the width"), 5),
    "uint32": (Whole(0, (1 << 32) - 1), (1 << 32) - 1, ("256000000000", "", "does not fit the width"), 10),
    "uint64": (WideOf(0, (1 << 64) - 1), (1 << 64) - 1,
               ('"-1"', "", "is not a whole number written as digits"), 22),
    "int8": (Whole(-128, 127), -128, ("1.5", "", "is not a whole number"), 4),
    "int16": (Whole(-(1 << 15), (1 << 15) - 1), -(1 << 15), ("1.5", "", "is not a whole number"), 6),
    "int32": (Whole(-(1 << 31), (1 << 31) - 1), -(1 << 31), ("1.5", "", "is not a whole number"), 11),
    "int64": (WideOf(-(1 << 63), (1 << 63) - 1), -(1 << 63), ("9007199254740992", "", "past 2^53 - 1"), 22),
    # Seventeen digits after `0.00000`: the longest spelling of a number, which only the
    # decimal band from 1e-6 to 1e-5 reaches. The single crosses as the double it widens to.
    "float32": (Single, -1.2345677760094986e-06, ("1e39", "", "does not fit the width"), 25),
    "float64": (float, -1.2345678901234567e-06, ("true", "", "is not a number"), 25),
    "bool": (bool, False, ("0", "", "is not a truth value"), 5),
    "enum": (_Level, _Level.MEDIUM, ('"lowest"', "", "is not a variant of"), 2 + len("medium")),
    "string[3]": (Text(TEXT_BOUND), CONTROLS, ('"abcd"', "", "is 4 bytes, past the 3 its type declares"),
                  2 + 6 * TEXT_BOUND),
    "bytes[3]": (Bytes(TEXT_BOUND), b"\x01\x01\x01", ('"\\u0100"', "", "carries a character above U+00FF"),
                 2 + 6 * TEXT_BOUND),
    # `null` (4 bytes) is longer than any uint8's digits, so the maximal value is absent.
    "optional<uint8>": (OptionalOf(Whole(0, 255)), None, ("256000000000", "", "does not fit the width"), 4),
    "record": (_REC, {"a": 255, "b": CONTROLS, "c": 255},
               ('{"a":"x","b":""}', "/a", "is not a whole number"),
               2 + (3 + 1 + 3) + (3 + 1 + 2 + 6 * TEXT_BOUND) + (3 + 1 + 3) + 2),
    "record of a list": (_REC_LIST, {"tags": [CONTROLS, CONTROLS]},
                         ('{"tags":["","","x"]}', "/tags", "holds 3 elements, past the 2"),
                         2 + (6 + 1 + 2 + BOUND * (2 + 6 * TEXT_BOUND) + (BOUND - 1))),
}


def _type(kind: str, depth: int) -> Any:
    declared = _KINDS[kind][0]
    for _ in range(depth):
        declared = ListOf(declared, BOUND)
    return declared


def _maximal(kind: str, depth: int) -> Any:
    value = _KINDS[kind][1]
    for _ in range(depth):
        value = [value] * BOUND
    return value


def _largest(kind: str, depth: int) -> int:
    size = _KINDS[kind][3]
    for _ in range(depth):
        size = 2 + BOUND * size + (BOUND - 1)
    return size


def _write(declared: Any, held: Any) -> Raw:
    """``held`` as JSON, written by the runtime's writers, one call per constructor;
    a record's members are handed in reverse to show that the writer orders them."""
    if isinstance(declared, ListOf):
        return list_json(held, lambda item: _write(declared.declared, item))
    if isinstance(declared, OptionalOf):
        return value_json(None) if held is None else _write(declared.declared, held)
    if isinstance(declared, RecordOf):
        kinds = dict(declared.fields)
        return record_json({name: _write(kinds[name], held[name]) for name in reversed(list(held))})
    if isinstance(declared, WideOf):
        return wide_json(held)
    if isinstance(held, IntEnum):
        return value_json(held.sce_name)
    return value_json(held)


def _data(value: str) -> str:
    return '{"v":' + value + ',"zz":{"unknown":[1,null]}}'


def _innermost(depth: int) -> str:
    return "v" + f"/{BOUND - 1}" * (depth - 1)


def _with_last_leaf(declared: Any, held: Any, leaf: str) -> str:
    if not isinstance(declared, ListOf):
        return leaf
    parts = [_write(declared.declared, item) for item in held]
    parts[-1] = _with_last_leaf(declared.declared, held[-1], leaf)
    return "[" + ",".join(parts) + "]"


def _one_past(declared: ListOf, held: List[Any], level: int) -> str:
    parts = [_write(declared.declared, item) for item in held]
    if level == 1:
        parts.append(_write(declared.declared, held[0]))
    else:
        parts[-1] = _one_past(declared.declared, held[-1], level - 1)
    return "[" + ",".join(parts) + "]"


def _set_last_leaf(held: Any, leaf: Any) -> Any:
    if not isinstance(held, list):
        return leaf
    return held[:-1] + [_set_last_leaf(held[-1], leaf)]


def _lift(declared: Any, data: Any, defaults: str | None = None) -> Any:
    (value,) = lift(data, (("v", declared),), defaults)
    return value


_PAST = "holds 3 elements, past the 2 its type declares"


def _cases() -> List[Tuple[str, Any, Any, Any]]:
    """Every case: its label, its type, its data, and what it must come to (a
    value, or a ``(path, sentence)`` refusal marked by ``_Refused``)."""
    cases: List[Tuple[str, Any, Any, Any]] = []
    for kind, (_, _, (bad, below, sentence), _) in _KINDS.items():
        for depth in (1, 2, 3):
            declared = _type(kind, depth)
            full = _maximal(kind, depth)
            leaf = f"{_innermost(depth)}/{BOUND - 1}"

            def case(name: str, data: Any, want: Any) -> None:
                cases.append((f"{kind} at depth {depth}, {name}", declared, data, want))

            case("empty", _data("[]"), [])
            case("maximal", _data(_write(declared, full)), full)
            case("one past the innermost bound", _data(_one_past(declared, full, depth)),
                 _Refused(_innermost(depth), _PAST))
            case("one past the outermost bound", _data(_one_past(declared, full, 1)), _Refused("v", _PAST))
            if kind == "optional<uint8>":
                # The maximal list of optionals is every element absent; a present
                # one among them is read as present.
                case("a present element among absent ones", _data(_with_last_leaf(declared, full, "7")),
                     _set_last_leaf(full, 7))
            else:
                case("null where the element is not optional", _data(_with_last_leaf(declared, full, "null")),
                     _Refused(leaf, "is null, and its type is not optional"))
            case("a bad element", _data(_with_last_leaf(declared, full, bad)), _Refused(leaf + below, sentence))
            # A key no schema declares, inside a record element and beside the
            # field, is stepped over.
            unknown = _write(declared, full)
            if kind == "record":
                unknown = _with_last_leaf(
                    declared, full, r'{"zz":[{"q":1}],"c":255,"b":"\u0001\u0001\u0001","a":255}')
            case("an unknown key", _data(unknown), full)
            # An older revision: a record element without the field its schema's
            # later revision added reads that field's default, and a payload
            # without a list a later revision added reads it empty.
            if kind == "record":
                case("an element of an older revision", _data(_with_last_leaf(declared, full, '{"a":1,"b":""}')),
                     _set_last_leaf(full, {"a": 1, "b": "", "c": 9}))
            else:
                case("a delivery of the revision before the list", ('{"zz":1}', '{"v":[]}'), [])
    return cases


class _Refused:
    __slots__ = ("path", "sentence")

    def __init__(self, path: str, sentence: str) -> None:
        self.path = path
        self.sentence = sentence

    def __repr__(self) -> str:
        return f"refused at {self.path!r}: {self.sentence}"


_CASES = _cases()


def test_the_cross_product_holds_every_case() -> None:
    assert len(_CASES) == 17 * 3 * 8


@pytest.mark.parametrize("label,declared,data,want", _CASES, ids=[c[0] for c in _CASES])
def test_every_element_kind_at_every_depth_is_read_or_refused_at_its_path(
        label: str, declared: Any, data: Any, want: Any) -> None:
    del label
    if isinstance(data, tuple):
        delivered, defaults = data
    else:
        delivered, defaults = data, None
    if isinstance(want, _Refused):
        with pytest.raises(TypedPayloadError) as refused:
            _lift(declared, delivered, defaults)
        reason = str(refused.value)
        assert f"'{want.path}'" in reason and want.sentence in reason, reason
    else:
        assert _lift(declared, delivered, defaults) == want


@pytest.mark.parametrize("kind", list(_KINDS))
@pytest.mark.parametrize("depth", [0, 1, 2, 3])
def test_a_maximal_value_is_written_at_exactly_its_largest_size(kind: str, depth: int) -> None:
    declared = _type(kind, depth)
    full = _maximal(kind, depth)
    text = _write(declared, full)
    size = _largest(kind, depth)
    assert len(text.encode("utf-8")) == size, text
    assert _lift(declared, _data(text)) == full


def test_a_top_level_text_and_byte_string_are_held_to_their_bound_when_read() -> None:
    data = {"note": "abcde", "blob": "abcde", "t": "éé", "n": None}
    with pytest.raises(TypedPayloadError, match="^'note' is 5 bytes, past the 4 its type declares$"):
        lift(data, (("note", Text(4)),))
    with pytest.raises(TypedPayloadError, match="^'blob' is 5 bytes, past the 4 its type declares$"):
        lift(data, (("blob", Bytes(4)),))
    # A text's bound counts UTF-8 bytes: two characters, four bytes.
    with pytest.raises(TypedPayloadError):
        lift(data, (("t", Text(3)),))
    assert lift(data, (("t", Text(4)),)) == ["éé"]
    with pytest.raises(TypedPayloadError, match="^'n' is null, and its type is not optional$"):
        lift(data, (("n", WideOf(-(1 << 63), (1 << 63) - 1)),))


def test_a_list_past_its_bound_is_refused_before_an_element_is_read() -> None:
    # The first element is no uint8: a reader that decoded it before it held the
    # bound would refuse it at 'v/0' instead.
    with pytest.raises(TypedPayloadError, match="^'v' holds 3 elements, past the 2 its type declares$"):
        lift({"v": ["x", 1, 2]}, (("v", ListOf(Whole(0, 255), 2)),))
    # A list within its bound reads every element, in order.
    assert lift({"v": [2, 1]}, (("v", ListOf(Whole(0, 255), 2)),)) == [[2, 1]]
    with pytest.raises(TypedPayloadError, match="^'v' is not a list$"):
        lift({"v": "x"}, (("v", ListOf(Whole(0, 255), 2)),))


def test_a_list_short_of_its_lower_count_is_refused_before_an_element_is_read() -> None:
    # The one element is no uint8: a reader that decoded it before it held the
    # lower count would refuse it at 'v/0' instead (docs/adr/0086, section 15.5).
    with pytest.raises(TypedPayloadError,
                       match="^'v' holds 1 elements, fewer than the 2 its type declares$") as short:
        lift({"v": ["x"]}, (("v", ListOf(Whole(0, 255), 3, least=2)),))
    assert (short.value.code, short.value.pointer, short.value.bound) == (RefusalCode.TOO_FEW, "/v", 2)
    assert lift({"v": [2, 1]}, (("v", ListOf(Whole(0, 255), 3, least=2)),)) == [[2, 1]]
    with pytest.raises(TypedPayloadError) as empty:
        lift({"v": []}, (("v", ListOf(Whole(0, 255), 3, least=1)),))
    assert (empty.value.code, empty.value.bound) == (RefusalCode.TOO_FEW, 1)
    # A list that states no lower count reads the empty list.
    assert lift({"v": []}, (("v", ListOf(Whole(0, 255), 3)),)) == [[]]


def test_a_list_written_short_of_its_lower_count_is_refused() -> None:
    # docs/adr/0086, section 15.6: the check a record's __post_init__ runs.
    check_value("items", ListOf(Whole(0, 255), 3, least=2), [1, 2])
    with pytest.raises(BoundError,
                       match="^'items/1/tags' holds 0 elements, fewer than the 1 its type declares$"):
        check_value("items/1/tags", ListOf(Whole(0, 255), 3, least=1), [])
    with pytest.raises(BoundError, match="^'items' holds 4 elements, past the 3 its type declares$"):
        check_value("items", ListOf(Whole(0, 255), 3, least=2), [1, 2, 3, 4])


def test_a_list_of_elements_with_nothing_to_hold_is_held_to_its_counts_alone() -> None:
    # The generator names the element `None` where it holds nothing of its own
    # to check: a record, which its own class checked where it was made, or an
    # enum member. Any element passes, and the counts are held.
    check_value("items", ListOf(None, 2), [object(), _Tagged("x", [])])
    check_value("levels", ListOf(None, 2), [_Level.LOW])
    with pytest.raises(BoundError, match="^'items' holds 3 elements, past the 2 its type declares$"):
        check_value("items", ListOf(None, 2), [object(), object(), object()])
    with pytest.raises(BoundError, match="^'items' holds 0 elements, fewer than the 1 its type declares$"):
        check_value("items", ListOf(None, 2, least=1), [])
    # A truth value, a real and an enum type hold no bound of their own either.
    check_value("on", bool, True)
    check_value("x", float, 1e308)
    check_value("level", _Level, _Level.HIGH)


def test_a_record_value_names_its_missing_field_and_reads_its_later_fields_from_defaults() -> None:
    data = {"r": {"a": 1, "zz": [None]}, "s": 5}
    with pytest.raises(TypedPayloadError, match="^the event's data has no 'r/b'$"):
        lift(data, (("r", RecordOf((("a", Whole(0, 255)), ("b", Whole(0, 255))))),))
    assert lift(data, (("r", RecordOf((("b", Whole(0, 255)),), defaults='{"b": 9}')),)) == [{"b": 9}]
    with pytest.raises(TypedPayloadError, match="^'s' is not a record$"):
        lift(data, (("s", RecordOf((("a", Whole(0, 255)),))),))
    with pytest.raises(TypedPayloadError, match="defaults are not a JSON object"):
        lift(data, (("r", RecordOf((("a", Whole(0, 255)),), defaults="[]")),))


@dataclass
class _Tagged:
    """A record a host builds, checked in its ``__post_init__`` at every depth, as
    the generator writes it."""

    label: str
    tags: List[str]

    def __post_init__(self) -> None:
        check_value("label", Text(8), self.label)
        check_value("tags", ListOf(Text(3), 2), self.tags)


def test_a_record_built_by_its_reader_is_the_record_class() -> None:
    declared = RecordOf((("label", Text(8)), ("tags", ListOf(Text(3), 2))), make=_Tagged)
    (tagged,) = lift({"t": {"label": "x", "tags": ["ab"]}}, (("t", declared),))
    assert tagged == _Tagged("x", ["ab"])


def test_a_list_and_a_record_are_written_in_their_order_and_by_their_key_bytes() -> None:
    assert list_json([], value_json) == "[]"
    assert list_json([2, 1], value_json) == "[2,1]"
    assert record_json({"b": 2, "aa": 1, "B": 3}) == '{"B":3,"aa":1,"b":2}'
    assert json.loads(record_json({"x": list_json([1, None], value_json)})) == {"x": [1, None]}


def test_a_value_a_host_builds_is_refused_where_it_is_made() -> None:
    assert _Tagged("eight888", ["abc", "ab"]).tags == ["abc", "ab"]
    with pytest.raises(BoundError, match="^'label' is 9 bytes, past the 8 its type declares$"):
        _Tagged("nine99999", [])
    with pytest.raises(BoundError, match="^'tags' holds 3 elements, past the 2 its type declares$"):
        _Tagged("a", ["a", "b", "c"])
    with pytest.raises(BoundError, match="^'tags/1' is 4 bytes, past the 3 its type declares$"):
        _Tagged("a", ["a", "abcd"])
    with pytest.raises(BoundError, match="^'blob' is 5 bytes, past the 4 its type declares$"):
        check_value("blob", Bytes(4), b"\x00" * 5)
    with pytest.raises(BoundError, match="does not fit the width"):
        check_value("n", Whole(0, 255), 256)
    check_value("due", OptionalOf(Text(1)), None)
    with pytest.raises(BoundError, match="^'due' is 2 bytes"):
        check_value("due", OptionalOf(Text(1)), "ab")
    check_value("r", _REC, {"a": 1, "b": "", "c": 9})
    with pytest.raises(BoundError, match="^'r/b' is 4 bytes"):
        check_value("r", _REC, {"a": 1, "b": "abcd", "c": 9})


if __name__ == "__main__":
    sys.exit(pytest.main([__file__]))
