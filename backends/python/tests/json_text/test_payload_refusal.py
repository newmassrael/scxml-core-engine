# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""What every payload reader refuses, and how it names the refusal.

The cases live in `tests/json_text/payload_refusal.json`, the table every
runtime's reader is held to: each is read here through `lift`, and the code,
pointer and bound of each refusal are held to what the table says
(docs/adr/0087, section 9.1, item 3). Python's own `json.loads` keeps the last
value of a repeated name; the cases that repeat one are what hold this reader
to refusing it.
"""

from __future__ import annotations

import json
from enum import IntEnum
from pathlib import Path
from typing import Any, Dict, List

import pytest

from sce_runtime.event_payload import (
    Bytes,
    CaseOf,
    ListOf,
    OptionalOf,
    RecordOf,
    RefusalCode,
    Single,
    Text,
    TypedPayloadError,
    UnionOf,
    Whole,
    WideOf,
    lift,
    lift_record,
    lift_text,
    lift_too_deep,
    pointer_below,
)

# The repository root, from this file's own location: the shared table is
# named by the same path every other reader uses.
_REPO_ROOT = Path(__file__).resolve().parents[4]
_TABLE = _REPO_ROOT / "tests" / "json_text" / "payload_refusal.json"

_WHOLE = {
    "uint8": (0, (1 << 8) - 1),
    "uint16": (0, (1 << 16) - 1),
    "uint32": (0, (1 << 32) - 1),
    "int8": (-(1 << 7), (1 << 7) - 1),
    "int16": (-(1 << 15), (1 << 15) - 1),
    "int32": (-(1 << 31), (1 << 31) - 1),
}
_WIDE = {
    "uint64": (0, (1 << 64) - 1),
    "int64": (-(1 << 63), (1 << 63) - 1),
}
_SCALAR = {"float32": Single, "float64": float, "bool": bool, "string": str}


def _load() -> List[Dict[str, Any]]:
    assert _TABLE.is_file(), f"cannot read the shared table at {_TABLE}"
    cases = json.loads(_TABLE.read_text(encoding="utf-8"))["cases"]
    assert len(cases) >= 40, f"the table lost cases: {len(cases)}"
    return cases


def _enum_of(names: List[str]) -> type:
    """An enum as a `sce-static` machine declares one, of `names`."""
    described = IntEnum("Described", {f"V{index}": index for index in range(len(names))})
    described.sce_name = property(lambda member: names[int(member)])
    return described


def _declared(ty: Any) -> Any:
    """The type name `read_value` takes for the type the table describes."""
    if isinstance(ty, str):
        if ty in _WHOLE:
            return Whole(*_WHOLE[ty])
        if ty in _WIDE:
            return WideOf(*_WIDE[ty])
        return _SCALAR[ty]
    if isinstance(ty, list):
        return _enum_of(ty)
    if "optional" in ty:
        return OptionalOf(_declared(ty["optional"]))
    if "string" in ty:
        return Text(ty["string"])
    if "bytes" in ty:
        return Bytes(ty["bytes"])
    if "list" in ty:
        # The lower count, where the case states one (docs/adr/0086, section 15).
        return ListOf(_declared(ty["list"]), ty["max"], ty.get("min", 0))
    if "record" in ty:
        return RecordOf([(id_, _declared(field)) for id_, field in ty["record"].items()])
    if "union" in ty:
        # The tag's enum is the cases' names; an empty case is null, any other
        # the record it holds (docs/adr/0092, section 4).
        cases = ty["union"]["cases"]
        tag_type = _enum_of(list(cases))
        return UnionOf(ty["union"]["tag"], tag_type, [
            CaseOf(member, None if case is None else _declared(case))
            for member, case in zip(tag_type, cases.values())])
    raise AssertionError(f"a type of the table names no constructor: {ty!r}")


def _read(case: Dict[str, Any]) -> None:
    """The case read as the table says: a case that gives a depth as a
    `<sce:lift>` of the record of its one field `v` reads it (docs/adr/0087,
    section 2.1), its pointer held to ``pathBound`` bytes (the standard
    refusal's 256 where the case gives none), any other as a delivery."""
    spec = [("v", _declared(case["type"]))]
    if "depth" in case:
        lift(lift_text(case["data"], case["depth"], case.get("pathBound", 256)), spec)
    else:
        lift(case["data"], spec)


@pytest.mark.parametrize("case", _load(), ids=lambda case: case["name"])
def test_every_case_is_refused_with_its_code_pointer_and_bound(case: Dict[str, Any]) -> None:
    want = case.get("refusal")
    if want is None:
        _read(case)
        return
    with pytest.raises(TypedPayloadError) as refused:
        _read(case)
    got = refused.value
    assert (got.code.declared_name, got.pointer, got.bound) == (
        want["code"], want["pointer"], want["bound"]), str(got)


def test_every_code_is_the_variant_the_standard_enum_declares() -> None:
    document = (_REPO_ROOT / "stdlib" / "lift" / "refusal_code.scxml").read_text(encoding="utf-8")
    assert document.count("<sce:variant ") == len(RefusalCode)
    for code in RefusalCode:
        variant = f'<sce:variant name="{code.declared_name}" value="{int(code)}"/>'
        assert variant in document, variant
        assert RefusalCode.named(code.declared_name) is code


def test_a_pointer_escapes_tilde_and_solidus_in_each_token() -> None:
    assert pointer_below("", "items") == "/items"
    assert pointer_below("/items", "2") == "/items/2"
    assert pointer_below("", "a~b/c") == "/a~0b~1c"
    assert pointer_below("", "~1") == "/~01"
    assert pointer_below("", "") == "/"


@pytest.mark.parametrize("text, depth, path_bound, pointer", [
    ('{"title":"a","count":1}', 1, 256, None),
    ('{"title":["a"]}', 1, 256, "/title"),
    ('{"extra":{"a":1}}', 1, 256, "/extra"),
    ('{"\\u0074itle":[1]}', 1, 256, "/title"),
    ("[[1]]", 1, 256, "/0"),
    ("[1,2,[3]]", 1, 256, "/2"),
    ('{"extra":[[[[[[[', 1, 256, "/extra"),
    ('{"title":"[[{\\"[","count":1}', 1, 256, None),
    ('{"title":{"a":1}}', 2, 256, None),
    ('{"title":"[[', 1, 256, None),
    ('{"title":"x","extra":[1]}', 1, 256, "/extra"),
    ('{"a":{"b":[1]}}', 2, 256, "/a/b"),
    ('{"a":[0,{"b":[1]}]}', 3, 256, "/a/1/b"),
    ('{"a/b~c":[1]}', 1, 256, "/a~1b~0c"),
    ("{[1]}", 1, 256, ""),
    ('{"a":{"abcdef":[1]}}', 2, 4, "/a"),
    ('{"a":{"bc":[1]}}', 2, 5, "/a/bc"),
])
def test_a_lift_refuses_a_container_past_its_depth_before_it_reads_the_text(
        text: str, depth: int, path_bound: int, pointer: Any) -> None:
    """Over the brackets alone, before the text is read as JSON, naming the
    container it refused, held to the room of the refusal's path by whole
    tokens (docs/adr/0087, sections 2.1 and 2.4)."""
    refusal = lift_too_deep(text, depth, path_bound)
    if pointer is None:
        assert refusal is None, str(refusal)
        return
    assert refusal is not None
    assert (refusal.code, refusal.pointer, refusal.bound) == (RefusalCode.TOO_DEEP, pointer, depth)


def test_a_lift_reads_a_record_or_answers_its_refusal() -> None:
    record = RecordOf([("v", Whole(0, 255)), ("w", Whole(0, 255))], '{"w":7}', lambda v, w: (v, w))
    assert lift_record('{"v":1}', 1, 256, record) == ((1, 7), None)
    strict = RecordOf([("v", Whole(0, 255)), ("w", Whole(0, 255))])
    for text, code in [
        (" \t", RefusalCode.NOT_JSON),
        ("[[", RefusalCode.TOO_DEEP),
        ("[1]", RefusalCode.NOT_OBJECT),
        ('{"v":1,"v":2}', RefusalCode.NOT_JSON),
        ('{"v":256,"w":1}', RefusalCode.OUT_OF_RANGE),
        ('{"w":1}', RefusalCode.MISSING),
    ]:
        value, refusal = lift_record(text, 1, 256, strict)
        assert value is None and refusal is not None and refusal.code is code, text


def test_a_repeated_name_is_refused_where_json_loads_would_keep_the_last() -> None:
    with pytest.raises(TypedPayloadError) as refused:
        lift('{"v": 1, "v": 2}', [("v", int, 0, 255)])
    assert refused.value.code is RefusalCode.NOT_JSON
    assert refused.value.pointer == ""
    assert refused.value.bound is None
