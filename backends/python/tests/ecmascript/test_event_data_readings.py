# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""§scxml-B-2-8-1 — which reading an arriving payload gets.

The clause names four readings and orders them: key-value pairs become
named properties; otherwise JSON becomes the corresponding object;
otherwise, "if the Processor can interpret the content as a valid XML
document, it MUST create the corresponding DOM structure"; and then the
sentence that closes it — "Otherwise, the Processor MUST treat the content
as a space-normalized string literal".

The expectations are not this file's. They live in
`tests/ecmascript/event_data_readings.json`, one payload per case with the
sentence of the clause that decides it, and the two C++ engines, the two
Kotlin engines, the Rust binding and the Go binding read the same file — a
per-backend copy drifts toward the backend that reads it, which is the
blindness that let eight engines give four different answers to one clause.

This binding is one of the two that already answered every case (measured
2026-08-19), so what it gets here is a regression witness rather than a
repair: its sibling `test_a_document_that_does_not_parse_falls_to_the_string_reading`
below is the same claim on the `<data>` path, and this one is the event
path the other six lost.

The `lua` spelling is what this backend is asked, and it is all it ever
sees: Python never receives the author's ECMAScript, because the frontend
lowered it at build time.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, List

import pytest

from sce_runtime.scripting.i_script_engine import (
    ScriptValueKind,
    SetCurrentEventArgs,
)
from sce_runtime.scripting.lua_engine import LuaScriptEngine

# The repository root, from this file's own location: the shared table is
# named by the same path every other reader uses.
_REPO_ROOT = Path(__file__).resolve().parents[4]
_TABLE = _REPO_ROOT / "tests" / "ecmascript" / "event_data_readings.json"


def _load() -> List[Dict[str, Any]]:
    assert _TABLE.is_file(), f"cannot read the shared table at {_TABLE}"
    cases = json.loads(_TABLE.read_text(encoding="utf-8"))["cases"]
    # A floor, not an equality: adding a case must not have to touch this
    # number, but a table that stopped being read must not pass either.
    assert len(cases) >= 8, (
        f"the shared reading table produced only {len(cases)} case(s), so this "
        "is not measuring the surface it claims to"
    )
    return cases


_CASES = _load()


def _answer(value: Any) -> Any:
    """What the engine answered, as a plain Python value.

    A whole number arrives as INT or DOUBLE depending on how Lua held it,
    and which of the two a decoded JSON number is is not part of the
    clause — so both become a float here rather than the comparison being
    loosened per case.
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
    "case", _CASES, ids=[case["payload"] for case in _CASES]
)
def test_event_data_reads_every_payload_the_clause_names(
    case: Dict[str, Any],
) -> None:
    engine = LuaScriptEngine()
    engine.create_session("reading")
    try:
        engine.set_current_event(
            "reading",
            SetCurrentEventArgs(
                event_name="brief",
                event_data=case["payload"],
                event_type="external",
            ),
        )
        answered = _answer(engine.evaluate_expression("reading", case["lua"]))
        want = _expected(case["expect"])
        assert answered == want, (
            f"payload {case['payload']!r}: {case['lua']} answered "
            f"{answered!r}, {case['clause']} says {want!r}"
        )
    finally:
        engine.destroy_session("reading")


def test_a_payload_that_is_a_call_leaves_the_session_alone() -> None:
    """The sharper half of the expression case.

    The shared table cannot ask it, because the side effect is spelled in
    the receiver's own language. Reading the payload gives back its own
    text; running it gives back `x` and, on the way, whatever else the
    sender named — `_event.data` is the one field a document takes from
    outside itself.
    """
    engine = LuaScriptEngine()
    engine.create_session("s")
    try:
        engine.execute_script("s", "breached = false")
        engine.set_current_event(
            "s",
            SetCurrentEventArgs(
                event_name="brief",
                event_data="(function() breached = true return 'x' end)()",
                event_type="external",
            ),
        )
        answered = engine.evaluate_expression("s", "breached")
        assert answered.kind == ScriptValueKind.BOOL
        assert answered.bool_val is False, (
            "the payload ran: a host, a peer session or an HTTP sender could "
            "write this session's globals by naming them in event data"
        )
    finally:
        engine.destroy_session("s")
