# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""§scxml-B-2-1 / §scxml-B-2-8-1 — XML in the data model is a DOM
structure, not three method names.

The expectations are not this file's. They live in
`tests/ecmascript/dom_read_surface.json`, one claim per case with the DOM
clause that backs it, and the two C++ engines, the three Kotlin engines,
the Go binding and the frontend read the same file — a per-backend copy
drifts toward the backend that reads it, which is the blindness that let
seven bindings disagree with one specification. Measured 2026-08-18,
every read in it answered nil on all seven: what they carried was
`getElementsByTagName`, `getAttribute` and a non-standard `getTagName`,
which are the two names the W3C IRP suite reads plus one.

The `lua` spelling is what this backend is asked, and it is all it ever
sees: Python never receives the author's ECMAScript, because the frontend
lowered it at build time.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, List, Tuple

import pytest

from sce_runtime.scripting.i_script_engine import ScriptValueKind
from sce_runtime.scripting.lua_engine import LuaScriptEngine

# The repository root, from this file's own location: the shared table is
# named by the same path every other reader uses.
_REPO_ROOT = Path(__file__).resolve().parents[4]
_TABLE = _REPO_ROOT / "tests" / "ecmascript" / "dom_read_surface.json"


def _load() -> Tuple[Dict[str, str], List[Dict[str, Any]]]:
    assert _TABLE.is_file(), f"cannot read the shared table at {_TABLE}"
    table = json.loads(_TABLE.read_text(encoding="utf-8"))
    cases = table["cases"]
    # A floor, not an equality: adding a case must not have to touch this
    # number, but a table that stopped being read must not pass either.
    assert len(cases) >= 30, (
        f"the shared table produced only {len(cases)} case(s), so this is not "
        "measuring the surface it claims to"
    )
    return table["documents"], cases


_DOCUMENTS, _CASES = _load()


def _answer(value: Any) -> Any:
    """What the engine answered, as a plain Python value.

    A whole number arrives as INT or DOUBLE depending on how Lua held it,
    and which of the two a `nodeType` is is not part of the DOM contract —
    so both become a float here rather than the comparison being loosened
    per case.
    """
    if value.kind in (ScriptValueKind.NULL, ScriptValueKind.UNDEFINED):
        return None
    if value.kind == ScriptValueKind.BOOL:
        return value.bool_val
    if value.kind == ScriptValueKind.INT:
        return float(value.int_val)
    if value.kind == ScriptValueKind.DOUBLE:
        return value.double_val
    if value.kind == ScriptValueKind.STRING:
        return value.string_val
    return value


def _expected(expect: Dict[str, Any]) -> Any:
    if "number" in expect:
        return float(expect["number"])
    if "string" in expect:
        return expect["string"]
    if "bool" in expect:
        return expect["bool"]
    if "empty" in expect:
        return None
    raise AssertionError(f"case has no readable expectation: {expect}")


@pytest.mark.parametrize(
    "case", _CASES, ids=[case["lua"] for case in _CASES]
)
def test_dom_read_surface_answers_dom_level_1_core(case: Dict[str, Any]) -> None:
    xml = _DOCUMENTS[case["document"]]
    engine = LuaScriptEngine()
    engine.create_session("dom")
    try:
        engine.set_variable_as_dom("dom", "var1", xml)
        answered = _answer(engine.evaluate_expression("dom", case["lua"]))
        want = _expected(case["expect"])
        assert answered == want, (
            f"{case['lua']} answered {answered!r}, {case['clause']} says {want!r}"
        )
    finally:
        engine.destroy_session("dom")


def test_a_document_that_does_not_parse_falls_to_the_string_reading() -> None:
    """W3C B.2's last rung, which the DOM rung must not swallow.

    minidom raises ExpatError rather than returning None, and a raise here
    would turn an unparseable payload into an engine error instead of the
    space-normalized string the clause asks for.
    """
    engine = LuaScriptEngine()
    engine.create_session("dom")
    try:
        engine.set_variable_as_dom("dom", "var1", "<books><unclosed></books>")
        answered = engine.evaluate_expression("dom", "var1")
        assert answered.kind == ScriptValueKind.STRING
        assert answered.string_val == "<books><unclosed></books>"
    finally:
        engine.destroy_session("dom")
