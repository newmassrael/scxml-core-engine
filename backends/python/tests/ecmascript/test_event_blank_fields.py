# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""§scxml-5.10.1 and §scxml-B-2-8 — what an `_event` field the Processor
left blank reads as.

§scxml-5.10.1 has the Processor leave `sendid`, `origin`, `origintype` and
`invokeid` blank in named cases, and §scxml-B-2-8 makes blank ECMAScript
undefined, not the empty string. A Lua session holds undefined as nil.

The expectations are not this file's. They live in
`tests/ecmascript/event_blank_fields.json`, one event per case with the
sentence that decides it, and every engine reads the same file, beside its
reader of `event_data_readings.json`. An absent member of a case's `event`
is a field the Processor left blank, handed to the binding as the empty
string, which is how `SetCurrentEventArgs` carries it.

This binding's answer before the repair, read from the code on 2026-11-07:
`sendid` and `invokeid` were left unset when empty and so read nil, but
`origin` and `origintype` were bound as the empty string for every event,
so `_event.origin == nil` answered false for an internal or a platform
event.

The `lua` spelling is what this backend is asked, and it is all it ever
sees: the frontend lowered the author's ECMAScript at build time.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, List, Mapping

import pytest

from sce_runtime.scripting.i_script_engine import (
    ScriptValueKind,
    SetCurrentEventArgs,
)
from sce_runtime.scripting.lua_engine import LuaScriptEngine

# The repository root, from this file's own location: the shared table is
# named by the same path every other reader uses.
_REPO_ROOT = Path(__file__).resolve().parents[4]
_TABLE = _REPO_ROOT / "tests" / "ecmascript" / "event_blank_fields.json"


@dataclass(frozen=True)
class _Case:
    group: str
    args: SetCurrentEventArgs
    lua: str
    clause: str
    expect: bool


def _text(owner: Mapping[str, Any], key: str, required: bool) -> str:
    """A text member of a case. An absent optional member is a blank field;
    an absent required one, or one that is not a string, is refused rather
    than read as blank."""
    if key not in owner:
        assert not required, f"a case lacks its `{key}`: {owner}"
        return ""
    value = owner[key]
    assert isinstance(value, str), f"`{key}` is not a string in {owner}"
    return value


def _expected(case: Mapping[str, Any]) -> bool:
    """The case's answer. An expectation this reader cannot read is refused,
    not skipped: a skipped case is a case that passes."""
    expect = case.get("expect")
    assert isinstance(expect, dict) and isinstance(expect.get("bool"), bool), (
        f"no readable `expect.bool` in {case}"
    )
    return expect["bool"]


def _read(case: Dict[str, Any]) -> _Case:
    event = case.get("event")
    assert isinstance(event, dict), f"every case names its event: {case}"
    return _Case(
        group=_text(case, "group", True),
        args=SetCurrentEventArgs(
            event_name=_text(event, "name", True),
            event_type=_text(event, "type", True),
            send_id=_text(event, "sendid", False),
            origin=_text(event, "origin", False),
            origin_type=_text(event, "origintype", False),
            invoke_id=_text(event, "invokeid", False),
        ),
        lua=_text(case, "lua", True),
        clause=_text(case, "clause", True),
        expect=_expected(case),
    )


def _load() -> List[_Case]:
    assert _TABLE.is_file(), f"cannot read the shared table at {_TABLE}"
    held = json.loads(_TABLE.read_text(encoding="utf-8"))["cases"]
    # A floor, not an equality: adding a case must not have to touch this
    # number, but a table that stopped being read must not pass either.
    assert len(held) >= 11, (
        f"the shared blank-field table holds only {len(held)} case(s), so "
        "this is not measuring the surface it claims to"
    )
    read = [_read(case) for case in held]
    assert len(read) == len(held), (
        f"read {len(read)} of the {len(held)} case(s) the table holds"
    )
    return read


_CASES = _load()


# Each case is its own test, so every disagreement is reported, not just the
# first: a binding that drops `origin` always and one that binds it as ""
# fail different cases.
@pytest.mark.parametrize(
    "case",
    _CASES,
    ids=[f"{index}-{case.group}" for index, case in enumerate(_CASES)],
)
def test_a_blank_event_field_reads_as_nil(case: _Case) -> None:
    engine = LuaScriptEngine()
    engine.create_session("blank")
    try:
        engine.set_current_event("blank", case.args)
        answered = engine.evaluate_expression("blank", case.lua)
        assert answered.kind == ScriptValueKind.BOOL, (
            f"{case.lua} answered a {answered.kind}, not a boolean "
            f"({case.clause})"
        )
        assert answered.bool_val is case.expect, (
            f"{case.lua} answered {answered.bool_val!r}, {case.clause} says "
            f"{case.expect!r}"
        )
    finally:
        engine.destroy_session("blank")
