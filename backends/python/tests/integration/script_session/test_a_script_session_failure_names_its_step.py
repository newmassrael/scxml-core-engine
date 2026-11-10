# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""docs/adr/0096: the values a refused start hands its host, and the Lua
engine's half of the refusal, held on their own.

A refusal names the step that failed in the word every backend writes, the
sentence is the one every backend writes, and a creation failure is told apart
from a shared library that did not load wherever in the chain of causes the
library failure sits. The Lua engine names a chunk of the shared library that
does not load, and keeps no session for it; a session it does not hold is a
defect, not an error of the document. The generated machine that hands these
over is held in
``a_child_may_send_many_events_in_one_tick/test_a_machine_whose_script_session_fails_does_not_start.py``.

No document drives these cases, so nothing is generated for them.
"""
from __future__ import annotations

import sys
from pathlib import Path

import pytest

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

from sce_runtime import (  # noqa: E402 — path inserted above
    ConfigurationRejection,
    ConfigurationVerdict,
    LuaScriptEngine,
    RootStartRefusal,
    ScriptError,
    ScriptLibraryNotLoaded,
    ScriptSessionFailure,
    ScriptSessionStage,
    ScriptSessionUnavailable,
    SessionNotFoundError,
)
from sce_runtime.scripting import lua_engine as _lua_engine  # noqa: E402


def test_each_step_is_named_by_the_word_every_backend_writes() -> None:
    assert str(ScriptSessionStage.CREATE) == "create"
    assert str(ScriptSessionStage.LIBRARY) == "library"
    assert str(ScriptSessionStage.SYSTEM_VARIABLES) == "system-variables"


def test_a_failure_is_the_sentence_every_backend_writes() -> None:
    failure = ScriptSessionFailure(ScriptSessionStage.CREATE, "out of memory")
    assert str(failure) == (
        "the script session could not be established at step 'create': out of memory"
    )
    refused = ScriptSessionUnavailable(failure)
    assert refused.failure is failure
    assert str(refused) == str(failure)


def test_a_library_that_did_not_load_is_the_library_step_wherever_it_sits_in_the_causes() -> None:
    library = ScriptLibraryNotLoaded("JSON builtins", "syntax error")
    assert str(library) == "failed to load JSON builtins: syntax error"
    assert library.library == "JSON builtins" and library.reason == "syntax error"
    assert ScriptSessionFailure.of_creation(library).stage is ScriptSessionStage.LIBRARY

    # Raised from the library failure, and raised while handling it: both are
    # the library step.
    try:
        try:
            raise library
        except ScriptLibraryNotLoaded as cause:
            raise RuntimeError("the session could not be made") from cause
    except RuntimeError as wrapped:
        assert ScriptSessionFailure.of_creation(wrapped).stage is ScriptSessionStage.LIBRARY
    try:
        try:
            raise library
        except ScriptLibraryNotLoaded:
            raise RuntimeError("the session could not be made")
    except RuntimeError as wrapped:
        assert ScriptSessionFailure.of_creation(wrapped).stage is ScriptSessionStage.LIBRARY


def test_any_other_creation_failure_is_the_create_step() -> None:
    failure = ScriptSessionFailure.of_creation(MemoryError("no room for a Lua state"))
    assert failure.stage is ScriptSessionStage.CREATE
    assert failure.message == "no room for a Lua state"


def test_a_failed_bind_is_the_system_variables_step() -> None:
    failure = ScriptSessionFailure.of_system_variables(ScriptError("refused"))
    assert failure.stage is ScriptSessionStage.SYSTEM_VARIABLES
    assert failure.message == "refused"


def test_a_failure_that_says_nothing_is_named_by_its_class() -> None:
    assert ScriptSessionFailure.of_creation(MemoryError()).message == "MemoryError"


def test_a_refused_child_is_reported_with_its_invoke_and_the_step() -> None:
    failure = ScriptSessionFailure(ScriptSessionStage.SYSTEM_VARIABLES, "refused")
    assert failure.child_start_refusal("worker") == (
        "<invoke id='worker'> did not start: its child's script session failed at step "
        "'system-variables'"
    )


def test_a_root_start_is_refused_for_a_parent_or_for_the_session() -> None:
    assert RootStartRefusal.NEEDS_PARENT.reason == (
        "the machine sends to #_parent and was started with no parent session"
    )
    assert isinstance(RootStartRefusal.NEEDS_PARENT, RootStartRefusal.NeedsParent)
    assert str(RootStartRefusal.NEEDS_PARENT) == RootStartRefusal.NEEDS_PARENT.reason

    failure = ScriptSessionFailure(ScriptSessionStage.LIBRARY, "missing")
    refusal = RootStartRefusal.ScriptSession(failure)
    assert isinstance(refusal, RootStartRefusal)
    assert refusal.failure is failure
    assert refusal.reason == str(failure)
    assert refusal != RootStartRefusal.NEEDS_PARENT
    assert refusal == RootStartRefusal.ScriptSession(failure)


def test_an_entry_refused_for_the_session_carries_the_failure_in_its_verdict() -> None:
    failure = ScriptSessionFailure(ScriptSessionStage.CREATE, "out of memory")
    verdict = ConfigurationVerdict(ConfigurationRejection.SCRIPT_SESSION_UNAVAILABLE, failure)
    assert not verdict.accepted
    assert verdict.reason == str(failure)
    assert str(verdict) == str(failure)

    accepted = ConfigurationVerdict.ACCEPTED
    assert accepted.accepted and accepted.rejection is ConfigurationRejection.NONE
    assert accepted.script_session_failure is None

    refused = ConfigurationVerdict(ConfigurationRejection.EMPTY)
    assert not refused.accepted
    assert refused.reason == ConfigurationRejection.EMPTY.value


def _copy_library_file(into: Path, file_name: str) -> None:
    """Copy `file_name` of the shared library's real directory into `into`."""
    real = Path(_lua_engine._SHARED_SCRIPTING_DIR) / file_name
    (into / file_name).write_text(real.read_text(encoding="utf-8"), encoding="utf-8")


def _creation_refused(engine: LuaScriptEngine) -> ScriptLibraryNotLoaded:
    """The library failure `engine.create_session` raised, the engine holding
    no session for it."""
    with pytest.raises(ScriptLibraryNotLoaded) as refused:
        engine.create_session("s")
    assert not engine.has_session("s"), "a creation that failed keeps no session"
    assert ScriptSessionFailure.of_creation(refused.value).stage is ScriptSessionStage.LIBRARY
    return refused.value


def test_the_lua_engine_names_a_semantics_file_it_cannot_read(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setattr(_lua_engine, "_SHARED_SCRIPTING_DIR", str(tmp_path))
    assert _creation_refused(LuaScriptEngine()).library == "ECMAScript semantics"


def test_the_lua_engine_names_a_semantics_file_that_does_not_run(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    (tmp_path / "ecma_semantics.lua").write_text("this is not Lua", encoding="utf-8")
    _copy_library_file(tmp_path, "json_builtins.lua")
    monkeypatch.setattr(_lua_engine, "_SHARED_SCRIPTING_DIR", str(tmp_path))
    assert _creation_refused(LuaScriptEngine()).library == "ECMAScript semantics"


def test_the_lua_engine_requires_the_json_builtins(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # The JSON builtins were once loaded only when the file was there, which
    # hid a build that shipped half the library.
    _copy_library_file(tmp_path, "ecma_semantics.lua")
    monkeypatch.setattr(_lua_engine, "_SHARED_SCRIPTING_DIR", str(tmp_path))
    assert _creation_refused(LuaScriptEngine()).library == "JSON builtins"


def test_the_lua_engine_names_a_semantics_file_without_the_system_binder(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    # Both files load, and neither defines `_scxml_bind_system`, so the
    # accessors a session reaches its globals through cannot be made.
    (tmp_path / "ecma_semantics.lua").write_text("-- nothing\n", encoding="utf-8")
    (tmp_path / "json_builtins.lua").write_text("-- nothing\n", encoding="utf-8")
    monkeypatch.setattr(_lua_engine, "_SHARED_SCRIPTING_DIR", str(tmp_path))
    refused = _creation_refused(LuaScriptEngine())
    assert refused.library == "the session accessors"
    assert "_scxml_bind_system" in refused.reason


def test_the_lua_engine_makes_a_session_from_the_shared_library() -> None:
    # The control: the real directory loads, so the cases above are refused for
    # the chunk each one breaks and not for the engine.
    engine = LuaScriptEngine()
    engine.create_session("s")
    try:
        assert engine.has_session("s")
    finally:
        engine.destroy_session("s")


def test_a_session_the_engine_does_not_hold_is_not_an_error_of_the_document() -> None:
    # Content that reaches a session its start did not establish is a defect,
    # so it is not a `ScriptError` the generated code would take as
    # error.execution and carry on.
    assert not issubclass(SessionNotFoundError, ScriptError)
    engine = LuaScriptEngine()
    with pytest.raises(SessionNotFoundError):
        engine.execute_script("never-made", "x = 1")
