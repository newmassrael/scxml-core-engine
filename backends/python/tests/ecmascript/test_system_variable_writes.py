# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""§scxml-5.10 and §scxml-B-2-8 — a system variable cannot be written.

§scxml-5.10: "The Processor MUST cause any attempt to change the value of a
system variable to fail and MUST place the error 'error.execution' on the
internal event queue." §scxml-B-2-8 makes `_event` an ECMAScript object, so
its fields are part of its value. The engine's half of that sentence is the
failure: a write the binding refuses raises a `ScriptError`, and the
generated machine turns every `ScriptError` into `error.execution`.

The expectations are not this file's. They live in
`tests/ecmascript/system_variable_writes.json`, one write per case with the
sentence that decides it, and every engine reads the same file: a session,
the event it is processing, and for each case a write that must fail exactly
when the case says `refused`, followed by a read (`after`) that must answer
true whether or not the write was refused. A refused write that left its
mark is a disagreement, and so is a permitted one that did not happen.

This binding's answer before the repair, read from the code on 2026-11-08:
every case was accepted. `_event` was a plain table and `_sessionid`,
`_name` and `_ioprocessors` plain globals, so `_event.name = 'x'` renamed
the event the machine was processing and `_sessionid = 'x'` re-addressed
the session.

The `lua` spelling is what this backend is asked, and it is all it ever
sees: the frontend lowered the author's ECMAScript at build time.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, List, Mapping

from sce_runtime.io_processors import IoProcessorDescriptor
from sce_runtime.scripting.i_script_engine import (
    ScriptError,
    ScriptValue,
    ScriptValueKind,
    SetCurrentEventArgs,
)
from sce_runtime.scripting.lua_engine import LuaScriptEngine

# The repository root, from this file's own location: the shared table is
# named by the same path every other reader uses.
_REPO_ROOT = Path(__file__).resolve().parents[4]
_TABLE = _REPO_ROOT / "tests" / "ecmascript" / "system_variable_writes.json"

# The four names §scxml-5.10 reserves.
_SYSTEM_VARIABLES = ("_event", "_sessionid", "_name", "_ioprocessors")


@dataclass(frozen=True)
class _Session:
    sessionid: str
    name: str
    ioprocessors: Dict[str, str]


@dataclass(frozen=True)
class _Case:
    group: str
    write_lua: str
    refused: bool
    after_lua: str
    clause: str


def _text(owner: Mapping[str, Any], key: str) -> str:
    """A required text member. An absent one, or one that is not a string,
    is refused rather than read as empty: a case this reader cannot read is
    a case that would pass."""
    assert key in owner, f"`{key}` is missing from {owner}"
    value = owner[key]
    assert isinstance(value, str), f"`{key}` is not a string in {owner}"
    return value


def _read_session(held: Mapping[str, Any]) -> _Session:
    session = held.get("session")
    assert isinstance(session, dict), "the table names no `session`"
    processors = session.get("ioprocessors")
    assert isinstance(processors, dict) and processors, (
        f"the session names no `ioprocessors`: {session}"
    )
    for name, location in processors.items():
        assert isinstance(location, str), (
            f"the location of `{name}` is not a string: {processors}"
        )
    return _Session(
        sessionid=_text(session, "sessionid"),
        name=_text(session, "name"),
        ioprocessors=dict(processors),
    )


def _read_event(held: Mapping[str, Any]) -> SetCurrentEventArgs:
    """The event every case is processing. `data` is payload text, the form
    `SetCurrentEventArgs` carries it in."""
    event = held.get("event")
    assert isinstance(event, dict), "the table names no `event`"
    return SetCurrentEventArgs(
        event_name=_text(event, "name"),
        event_data=_text(event, "data"),
        event_type=_text(event, "type"),
        send_id=_text(event, "sendid"),
        origin=_text(event, "origin"),
        origin_type=_text(event, "origintype"),
        invoke_id=_text(event, "invokeid"),
    )


def _read_case(case: Mapping[str, Any]) -> _Case:
    refused = case.get("refused")
    assert isinstance(refused, bool), f"no readable `refused` in {case}"
    return _Case(
        group=_text(case, "group"),
        write_lua=_text(case, "write_lua"),
        refused=refused,
        after_lua=_text(case, "after_lua"),
        clause=_text(case, "clause"),
    )


def _load() -> "tuple[_Session, SetCurrentEventArgs, List[_Case]]":
    assert _TABLE.is_file(), f"cannot read the shared table at {_TABLE}"
    held = json.loads(_TABLE.read_text(encoding="utf-8"))
    cases = held.get("cases")
    assert isinstance(cases, list), "the table holds no `cases` list"
    # A floor, not an equality: adding a case must not have to touch this
    # number, but a table that stopped being read must not pass either.
    assert len(cases) >= 18, (
        f"the shared system-variable table holds only {len(cases)} case(s), "
        "so this is not measuring the surface it claims to"
    )
    read = [_read_case(case) for case in cases]
    assert len(read) == len(cases), (
        f"read {len(read)} of the {len(cases)} case(s) the table holds"
    )
    # Both answers occur, or the table cannot tell a binding that refuses
    # everything from one that refuses what it should.
    assert {case.refused for case in read} == {True, False}, (
        "the table no longer holds both refused and permitted writes"
    )
    return _read_session(held), _read_event(held), read


_SESSION, _EVENT, _CASES = _load()


def _open(engine: LuaScriptEngine) -> str:
    """A session set up as the table describes it, processing its event."""
    sid = _SESSION.sessionid
    engine.create_session(sid)
    engine.setup_system_variables(
        sid,
        _SESSION.name,
        [
            IoProcessorDescriptor(name=name, location=location)
            for name, location in _SESSION.ioprocessors.items()
        ],
    )
    engine.set_current_event(sid, _EVENT)
    return sid


def _answer(engine: LuaScriptEngine, sid: str, lua: str) -> str:
    """What `lua` answers, as a sentence: an empty string when it answered
    true, otherwise what it answered instead."""
    try:
        value = engine.evaluate_expression(sid, lua)
    except ScriptError as exc:
        return f"raised {exc}"
    if value.kind is not ScriptValueKind.BOOL:
        return f"answered a {value.kind.value}, not a boolean"
    return "" if value.bool_val else "answered false"


def test_a_write_fails_exactly_when_the_shared_table_says() -> None:
    disagreements: List[str] = []
    for index, case in enumerate(_CASES):
        engine = LuaScriptEngine()
        sid = _open(engine)
        try:
            try:
                engine.execute_script(sid, case.write_lua)
                refused = False
            except ScriptError:
                refused = True
            # Any other exception is not the engine's error result and is
            # left to fail the test as it is.
            label = f"[{index} {case.group}] {case.write_lua!r}"
            if refused != case.refused:
                disagreements.append(
                    f"{label} was {'refused' if refused else 'accepted'}, "
                    f"{case.clause} says {'refused' if case.refused else 'accepted'}"
                )
            after = _answer(engine, sid, case.after_lua)
            if after:
                disagreements.append(
                    f"{label} then {case.after_lua!r} {after} ({case.clause})"
                )
        finally:
            engine.destroy_session(sid)
    assert not disagreements, "\n".join(disagreements)


def _reads_unchanged(engine: LuaScriptEngine, sid: str) -> List[str]:
    """The system variables, read back against the table's session and
    event: one line per value that is not the one bound."""
    processor, location = next(iter(_SESSION.ioprocessors.items()))
    checks = (
        ("_sessionid", f"_sessionid == {json.dumps(_SESSION.sessionid)}"),
        ("_name", f"_name == {json.dumps(_SESSION.name)}"),
        (
            "_ioprocessors",
            f"_ioprocessors[{json.dumps(processor)}].location"
            f" == {json.dumps(location)}",
        ),
        ("_event", f"_event.name == {json.dumps(_EVENT.event_name)}"),
    )
    return [
        f"{name}: {lua!r} {answer}"
        for name, lua in checks
        for answer in [_answer(engine, sid, lua)]
        if answer
    ]


def test_a_variable_the_document_names_is_never_a_system_variable() -> None:
    """The writes whose name the document chooses (`<data id>`, a
    `<foreach item>`, a DOM `<data>`) reach the engine by name, not as
    script text, and §scxml-5.10 refuses them all the same."""
    writes = (
        (
            "declare_variable",
            lambda e, s, n: e.declare_variable(s, n, ScriptValue.of("x")),
        ),
        (
            "set_variable",
            lambda e, s, n: e.set_variable(s, n, ScriptValue.of("x")),
        ),
        (
            "set_variable_as_dom",
            lambda e, s, n: e.set_variable_as_dom(s, n, "<x/>"),
        ),
    )
    disagreements: List[str] = []
    for name in _SYSTEM_VARIABLES:
        for method, write in writes:
            engine = LuaScriptEngine()
            sid = _open(engine)
            try:
                try:
                    write(engine, sid, name)
                    disagreements.append(f"{method}({name!r}) was accepted")
                except ScriptError:
                    pass
                if engine.has_variable(sid, name):
                    disagreements.append(
                        f"{method}({name!r}) was refused but declared the name"
                    )
                disagreements.extend(
                    f"after {method}({name!r}), {line}"
                    for line in _reads_unchanged(engine, sid)
                )
            finally:
                engine.destroy_session(sid)
    assert not disagreements, "\n".join(disagreements)


def test_a_system_variable_reads_back_whole() -> None:
    """`_event` and `_ioprocessors` are read-only views whose fields are
    behind them; read out by name, each is still the value that was bound."""
    engine = LuaScriptEngine()
    sid = _open(engine)
    try:
        event = engine.get_variable(sid, "_event").to_python()
        assert event == {
            "name": _EVENT.event_name,
            "type": _EVENT.event_type,
            "sendid": _EVENT.send_id,
            "origin": _EVENT.origin,
            "origintype": _EVENT.origin_type,
            "invokeid": _EVENT.invoke_id,
            "data": json.loads(_EVENT.event_data),
        }, f"_event read back as {event!r}"
        processors = engine.get_variable(sid, "_ioprocessors").to_python()
        assert processors == {
            name: {"location": location}
            for name, location in _SESSION.ioprocessors.items()
        }, f"_ioprocessors read back as {processors!r}"
    finally:
        engine.destroy_session(sid)
