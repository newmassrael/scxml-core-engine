# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""docs/adr/0096 — Python AOT path: a generated machine whose script session
cannot be established does not start, and its host is told why.

The script engine is a double over the real Lua engine. It fails, on request,
one of the three steps that establish a session — creating it, loading the
shared library into it, binding the system variables — and counts every call
that runs a document's expressions, so "no executable content ran" is read from
the engine the machine runs its content in.

Fixture: ``integration_resources/a_child_may_send_many_events_in_one_tick/a_child_may_send_many_events_in_one_tick.scxml``,
reused for its shape: a parent in ``s0`` with ``<data ticks>`` that invokes an
inline child, whose ``<onentry>`` sends ``tick`` to the parent 110 times. The
child declares ``<data>`` of its own, so it has a script session of its own to
fail. A start that ran would initialize ``ticks``, enter ``s0`` and start the
child, asking the engine for the child's session; a child that ran would send
``tick``.

Regeneration (after fixture or template edit):
  ``scripts/regen_a_child_may_send_many_events_in_one_tick_python.sh`` (local)
"""
from __future__ import annotations

import sys
from enum import Enum
from pathlib import Path
from typing import Any, List, Tuple

import pytest

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import a_child_may_send_many_events_in_one_tick_sm as _sm  # noqa: E402 — path inserted above
from sce_runtime import (  # noqa: E402
    ConfigurationRejection,
    EventMetadata,
    InboxMeshDelivery,
    LuaScriptEngine,
    MeshRaise,
    RootStartRefusal,
    ScriptError,
    ScriptLibraryNotLoaded,
    ScriptSessionStage,
    ScriptSessionUnavailable,
)

_State = _sm.AChildMaySendManyEventsInOneTickState
_Event = _sm.AChildMaySendManyEventsInOneTickEvent

# The id the machine is built under, so the child's id is known: its parent's
# and the id of the start, `<state>.<start>.<invoke>` (docs/adr/0045).
_ROOT = "root"
_CHILD = f"{_ROOT}.s0.1.chatty"


class _Fail(Enum):
    """Which step of establishing a session the double fails."""

    #: The double is the Lua engine, counted.
    NEVER = "never"
    #: Every session's creation fails as the engine's own failure.
    CREATE = "create"
    #: Every session's creation fails as a shared library that did not load.
    LIBRARY = "library"
    #: Every bind of the system variables fails.
    SYSTEM_VARIABLES = "system-variables"
    #: The first session asked for, the parent's, is created; every later one,
    #: a child's, fails to be.
    CREATE_AFTER_FIRST = "create-after-first"


class _SessionDouble(LuaScriptEngine):
    """The Lua engine, with one step of establishing a session failing on
    request and every call that runs the document's expressions counted. The
    methods it does not name are the Lua engine's, unchanged."""

    def __init__(self, fail: _Fail) -> None:
        super().__init__()
        self.fail = fail
        #: Every session id `create_session` was asked for, in order.
        self.asked: List[str] = []
        #: Every session id `destroy_session` was asked for, in order.
        self.destroyed: List[str] = []
        #: The calls that run a document's expressions or write its datamodel.
        self.ran = 0
        #: Each event bound as `_event`: the session, its name and its data.
        self.bound: List[Tuple[str, str, str]] = []

    def create_session(self, session_id: str) -> None:
        self.asked.append(session_id)
        if self.fail is _Fail.CREATE:
            raise RuntimeError("the double creates no session")
        if self.fail is _Fail.LIBRARY:
            raise ScriptLibraryNotLoaded("ECMAScript semantics", "the double loads no library")
        if self.fail is _Fail.CREATE_AFTER_FIRST and len(self.asked) > 1:
            raise RuntimeError("the double creates no second session")
        super().create_session(session_id)

    def destroy_session(self, session_id: str) -> None:
        self.destroyed.append(session_id)
        super().destroy_session(session_id)

    def setup_system_variables(self, session_id, session_name, io_processors):  # type: ignore[override]
        if self.fail is _Fail.SYSTEM_VARIABLES:
            raise ScriptError("the double binds no system variable")
        super().setup_system_variables(session_id, session_name, io_processors)

    def execute_script(self, session_id, script):  # type: ignore[override]
        self.ran += 1
        return super().execute_script(session_id, script)

    def evaluate_expression(self, session_id, expression):  # type: ignore[override]
        self.ran += 1
        return super().evaluate_expression(session_id, expression)

    def declare_variable(self, session_id, name, initial_value):  # type: ignore[override]
        self.ran += 1
        super().declare_variable(session_id, name, initial_value)

    def set_variable(self, session_id, name, value):  # type: ignore[override]
        self.ran += 1
        super().set_variable(session_id, name, value)

    def set_variable_as_dom(self, session_id, name, xml_content):  # type: ignore[override]
        self.ran += 1
        super().set_variable_as_dom(session_id, name, xml_content)

    def set_current_event(self, session_id, args):  # type: ignore[override]
        self.ran += 1
        self.bound.append((session_id, args.event_name, args.event_data))
        return super().set_current_event(session_id, args)


def _over_double(fail: _Fail) -> Tuple[Any, _SessionDouble]:
    """The fixture's machine, built under `_ROOT` over a double that fails as
    `fail` says, and the double, to read afterwards."""
    double = _SessionDouble(fail)
    double.initialize()
    return _sm.create_engine(script_engine=double, session_id=_ROOT), double


def _assert_nothing_started(engine: Any, double: _SessionDouble, fail: _Fail) -> None:
    """A refused start left nothing behind: not running, no final state
    reached, no state entered, no content run and no session kept. Entering
    `s0` would have started its child and asked the engine for the child's
    session, so only the machine's own was asked for."""
    assert not engine.is_running, f"{fail.value}: a refused start leaves the machine not running"
    assert engine.terminal_state is None, f"{fail.value}: a refused start reaches no final state"
    assert engine.active_configuration() == set(), f"{fail.value}: a refused start enters no state"
    assert double.ran == 0, (
        f"{fail.value}: a refused start evaluates nothing (no <data>, no <onentry>, no cond), "
        f"and {double.ran} calls ran"
    )
    assert not double.has_session(_ROOT), f"{fail.value}: a refused start keeps no session"
    assert all(asked == _ROOT for asked in double.asked), (
        f"{fail.value}: a refused start enters no state, and a child's session was asked for: "
        f"{double.asked}"
    )
    assert engine.policy.ticks() is None, (
        f"{fail.value}: a refused start declares no datamodel, so `ticks` cannot be read"
    )


def test_a_start_over_the_double_that_fails_nothing_runs_the_fixture() -> None:
    # The control: the double fails nothing, so the machine is the fixture's,
    # and its child's ticks arrive. Without it the cases below could pass over
    # a double that breaks every machine.
    engine, double = _over_double(_Fail.NEVER)
    engine.initialize()
    engine.send_event(_Event.FINISH)

    seen = f"ticks={engine.policy.ticks()} (wanted 110)"
    assert engine.terminal_state == _State.DONE, f"`finish` carries the run to `done`. {seen}"
    assert engine.policy.ticks() == 110, f"every tick the child sent arrived. {seen}"
    assert double.ran > 0, "the machine ran its content through the double"
    assert double.asked == [_ROOT, _CHILD], (
        f"the parent's session and its child's were asked for: {double.asked}"
    )


@pytest.mark.parametrize(
    "fail, stage",
    [
        (_Fail.CREATE, ScriptSessionStage.CREATE),
        (_Fail.LIBRARY, ScriptSessionStage.LIBRARY),
        (_Fail.SYSTEM_VARIABLES, ScriptSessionStage.SYSTEM_VARIABLES),
    ],
)
def test_a_start_whose_session_cannot_be_established_is_refused_with_the_step(
    fail: _Fail, stage: ScriptSessionStage
) -> None:
    engine, double = _over_double(fail)
    with pytest.raises(ScriptSessionUnavailable) as refused:
        engine.initialize()
    failure = refused.value.failure
    assert failure.stage is stage, f"{fail.value}: the refusal names the step {failure.stage}"
    assert str(refused.value) == (
        f"the script session could not be established at step '{stage.value}': {failure.message}"
    ), f"{fail.value}: the refusal is the sentence every backend writes"
    _assert_nothing_started(engine, double, fail)


@pytest.mark.parametrize("fail", [_Fail.CREATE, _Fail.LIBRARY, _Fail.SYSTEM_VARIABLES])
def test_a_refused_start_is_refused_again_rather_than_run_without_a_session(fail: _Fail) -> None:
    # A session that failed is not marked established: the next start asks for
    # it again and is refused again, rather than finding it made and running
    # the machine over no session.
    engine, double = _over_double(fail)
    with pytest.raises(ScriptSessionUnavailable):
        engine.initialize()
    with pytest.raises(ScriptSessionUnavailable):
        engine.initialize()
    assert double.asked == [_ROOT, _ROOT], (
        f"{fail.value}: each start asks for the session once: {double.asked}"
    )
    _assert_nothing_started(engine, double, fail)


def test_a_session_whose_system_variables_cannot_be_bound_is_released() -> None:
    engine, double = _over_double(_Fail.SYSTEM_VARIABLES)
    with pytest.raises(ScriptSessionUnavailable):
        engine.initialize()
    assert double.asked == [_ROOT], f"the machine asked for its own session once: {double.asked}"
    assert double.destroyed == [_ROOT], (
        f"the session the failed bind was made in is destroyed, and {double.destroyed} were"
    )


def test_a_refused_machine_takes_no_event() -> None:
    engine, double = _over_double(_Fail.CREATE)
    with pytest.raises(ScriptSessionUnavailable):
        engine.initialize()
    engine.send_event(_Event.FINISH)
    assert engine.unseen_external_events() == 1, (
        "the event is refused as one sent to a machine that is not running"
    )
    _assert_nothing_started(engine, double, _Fail.CREATE)


def test_the_checked_root_start_is_refused_for_the_session() -> None:
    engine, double = _over_double(_Fail.LIBRARY)
    refusal = engine.initialize_as_root()
    assert isinstance(refusal, RootStartRefusal.ScriptSession), (
        f"the root start answered {refusal!r}, not a refusal for its session"
    )
    assert refusal is not RootStartRefusal.NEEDS_PARENT, (
        "a refusal for the session is not one for a parent"
    )
    assert refusal.failure.stage is ScriptSessionStage.LIBRARY
    assert refusal.reason == str(refusal.failure), "the refusal's words are the failure's sentence"
    _assert_nothing_started(engine, double, _Fail.LIBRARY)


def test_a_resumed_start_whose_session_cannot_be_established_is_refused() -> None:
    engine, double = _over_double(_Fail.CREATE)
    verdict = engine.enter_at([_State.S0], _State.S0)
    assert verdict.rejection is ConfigurationRejection.SCRIPT_SESSION_UNAVAILABLE, (
        f"enter_at answered {verdict}"
    )
    assert not verdict.accepted
    assert verdict.script_session_failure is not None
    assert verdict.script_session_failure.stage is ScriptSessionStage.CREATE, (
        "the verdict carries the failure of the create step"
    )
    _assert_nothing_started(engine, double, _Fail.CREATE)


def test_a_second_start_makes_its_session_anew() -> None:
    # A start is a new run, and its datamodel is made anew: the session the run
    # the host stopped left readable is released, and a new one made.
    engine, double = _over_double(_Fail.NEVER)
    engine.initialize()
    engine.stop()
    assert not engine.is_running
    assert double.has_session(_ROOT), "a run that was stopped leaves its datamodel readable"
    assert engine.policy.ticks() is not None, "the stopped run's datamodel is still read"
    assert _ROOT not in double.destroyed

    engine.initialize()
    assert double.asked.count(_ROOT) == 2, f"the second start asked for its session: {double.asked}"
    assert _ROOT in double.destroyed, "the session of the run that ended was released"
    assert double.has_session(_ROOT), "the second start runs in the session it made"
    assert engine.is_running and engine.current_state == _State.S0


def test_a_child_whose_session_fails_leaves_its_parent_with_error_execution_and_no_child() -> None:
    engine, double = _over_double(_Fail.CREATE_AFTER_FIRST)
    engine.initialize()

    assert double.asked == [_ROOT, _CHILD], (
        f"the parent asked for its own session and the child's: {double.asked}"
    )
    assert not double.has_session(_CHILD), "the child whose session failed keeps none"
    # The tree holds no session under the child's id: a post naming it is
    # refused, as one naming no session of the tree is.
    assert not engine.inbox.post(
        InboxMeshDelivery(MeshRaise("tick", EventMetadata()), session_id=_CHILD)
    ), "the child whose start was refused is not kept as a session of the tree"
    # The invoke ends without a child, with error.execution, which no
    # transition of `s0` takes, and whose data names the invoke and the step.
    error_execution = engine.policy.get_event_from_name("error.execution")
    assert error_execution is not None, "the document names error.execution"
    assert engine.last_unhandled_error() == error_execution, (
        "the refused child is reported as error.execution"
    )
    assert engine.unhandled_error_events() == 1, "one error.execution for the one refused child"
    assert (
        _ROOT,
        "error.execution",
        "<invoke id='chatty'> did not start: its child's script session failed at step 'create'",
    ) in double.bound, f"the error's data names the invoke and the step: {double.bound}"
    # No child ran, so no `tick` and no `done.invoke` ever arrived.
    assert engine.policy.ticks() == 0, "no child ran, so no tick arrived"
    assert engine.discarded_external_events() == 0, (
        "no done.invoke was raised for the refused child"
    )

    # The parent itself runs on.
    assert engine.is_running, "the parent runs on without its child"
    assert engine.current_state == _State.S0, "the parent stays in `s0`"
    engine.send_event(_Event.FINISH)
    assert engine.terminal_state == _State.DONE, "`finish` carries the parent to `done`"
