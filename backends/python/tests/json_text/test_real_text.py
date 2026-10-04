# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A float is spelled the way ECMAScript spells it on every engine
(ARCHITECTURE.md, "JSON Number Text (Single Source of Truth)").

The cases live in `tests/json_text/real_text.json`, the table every engine's
writer is measured against, each as the IEEE 754 bit pattern of the value so
that no number parser stands between the table and the double. This backend
spells a float three ways — `number_text`, `ScriptValue.to_json_literal` and
`ScriptValue.to_wire_string` — and all three are held to the table here.
"""

from __future__ import annotations

import json
import struct
from pathlib import Path
from typing import Any, Dict, List

import pytest

from sce_runtime.number_text import number_text
from sce_runtime.scripting.i_script_engine import ScriptValue

# The repository root, from this file's own location: the shared table is
# named by the same path every other reader uses.
_REPO_ROOT = Path(__file__).resolve().parents[4]
_TABLE = _REPO_ROOT / "tests" / "json_text" / "real_text.json"


def _load() -> List[Dict[str, Any]]:
    assert _TABLE.is_file(), f"cannot read the shared table at {_TABLE}"
    cases = json.loads(_TABLE.read_text(encoding="utf-8"))["cases"]
    # A floor, not an equality: adding a case must not have to touch this
    # number, but a table that stopped being read must not pass either.
    assert len(cases) >= 40, f"the shared real-text table produced only {len(cases)} case(s)"
    return cases


def _double(case: Dict[str, Any]) -> float:
    return struct.unpack(">d", bytes.fromhex(case["bits"]))[0]


@pytest.mark.parametrize("case", _load(), ids=lambda case: case["name"])
def test_a_float_is_spelled_as_ecmascript_spells_it(case: Dict[str, Any]) -> None:
    assert number_text(_double(case)) == case["text"]


@pytest.mark.parametrize("case", _load(), ids=lambda case: case["name"])
def test_a_float_in_json_is_spelled_the_same_way(case: Dict[str, Any]) -> None:
    assert ScriptValue.of(_double(case)).to_json_literal() == case["text"]


@pytest.mark.parametrize("case", _load(), ids=lambda case: case["name"])
def test_a_float_as_an_untyped_param_is_spelled_the_same_way(case: Dict[str, Any]) -> None:
    assert ScriptValue.of(_double(case)).to_wire_string() == case["text"]


@pytest.mark.parametrize(
    "value, wire",
    [(float("nan"), "NaN"), (float("inf"), "Infinity"), (float("-inf"), "-Infinity")],
)
def test_a_float_that_is_not_finite_has_no_json_spelling_and_a_wire_one(
    value: float, wire: str
) -> None:
    assert ScriptValue.of(value).to_json_literal() == "null"
    assert ScriptValue.of(value).to_wire_string() == wire
