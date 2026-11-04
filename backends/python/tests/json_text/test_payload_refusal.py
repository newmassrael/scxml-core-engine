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
    ListOf,
    OptionalOf,
    RecordOf,
    RefusalCode,
    Single,
    Text,
    TypedPayloadError,
    Whole,
    WideOf,
    lift,
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
        return ListOf(_declared(ty["list"]), ty["max"])
    if "record" in ty:
        return RecordOf([(id_, _declared(field)) for id_, field in ty["record"].items()])
    raise AssertionError(f"a type of the table names no constructor: {ty!r}")


@pytest.mark.parametrize("case", _load(), ids=lambda case: case["name"])
def test_every_case_is_refused_with_its_code_pointer_and_bound(case: Dict[str, Any]) -> None:
    spec = [("v", _declared(case["type"]))]
    want = case.get("refusal")
    if want is None:
        lift(case["data"], spec)
        return
    with pytest.raises(TypedPayloadError) as refused:
        lift(case["data"], spec)
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


def test_a_repeated_name_is_refused_where_json_loads_would_keep_the_last() -> None:
    with pytest.raises(TypedPayloadError) as refused:
        lift('{"v": 1, "v": 2}', [("v", int, 0, 255)])
    assert refused.value.code is RefusalCode.NOT_JSON
    assert refused.value.pointer == ""
    assert refused.value.bound is None
