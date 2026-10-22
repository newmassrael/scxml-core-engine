# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""SCE Accepted Subset §2.15, "Saving and restoring": an ``<invoke>`` the HOST runs
is part of what a machine is doing (§scxml-6.4.1), and the process that ran it is
gone with the process that saved it. A saved state holds each running host-run
invocation with the request it was started with and the deadline it had left, and a
restore starts it again from that request, never evaluating the element a second
time, with ``restarted`` set, a new token and what was left of the deadline.

``statechart_static_host_invoke.scxml`` hands a job to the host from ``working``.
``done.invoke.h`` and ``error.invoke.h`` each leave it and record a digit in
``seen``, so the number says which arrived. The Rust, Kotlin and Go halves are
``a_saved_host_invocation_is_started_again_after_a_restore.rs``,
``ASavedHostInvocationIsStartedAgainAfterARestoreTest.kt`` and
``a_saved_host_invocation_is_started_again_after_a_restore_test.go``.

Driven entirely on the engine's virtual clock: no case sleeps, and none can be decided
by how loaded the build machine is. The shared instance
``saved/statechart_static_host_invoke_running.json`` is the format-2 text every backend
writes (docs/adr/0045); the format-1 text of the same run, under ``saved/format1/``,
still restores.
"""
from __future__ import annotations

import sys
from pathlib import Path

import pytest

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))
# The checked operations a lowered expression calls are the forge runtime's.
sys.path.insert(0, str(_HERE.parents[2] / "forge-runtime"))

from sce_runtime import StateRefusal, saved_state_from_json  # noqa: E402 -- path inserted above

import statechart_static_host_invoke_sm as _sm  # noqa: E402 -- path inserted above

_State = _sm.StatechartStaticHostInvokeState
_HOST_TYPE = "x-sce-host"
# The wall-clock moment the shared instance was saved at.
_SAVED_AT_MS = 1_700_000_000_000
_SAVED = _HERE.parents[4] / "sce-build" / "tests" / "fixtures" / "host_processor" / "saved"


def _shared(format1: bool = False) -> str:
    directory = _SAVED / "format1" if format1 else _SAVED
    return (directory / "statechart_static_host_invoke_running.json").read_text(encoding="utf-8").strip()


class _Host:
    """What the host was told, in order: a start or a cancel."""

    def __init__(self) -> None:
        self.log: list = []

    def register(self, engine) -> "_Host":
        def invoker(event):
            if event.start is not None:
                self.log.append(("start", event.start))
            if event.cancel is not None:
                self.log.append(("cancel", event.cancel))
            return None

        engine.register_invoker(_HOST_TYPE, invoker)
        return self

    def starts(self) -> list:
        return [entry for kind, entry in self.log if kind == "start"]


def _same_request(a, b) -> bool:
    """Whether two requests say the same thing: everything but the token the engine
    assigns, the session that made it and the mark a restart carries."""
    return (a.processor_type, a.invoke_id, a.src, a.params, a.event_data, a.content) == (
        b.processor_type,
        b.invoke_id,
        b.src,
        b.params,
        b.event_data,
        b.content,
    )


def _working():
    """A machine standing in `working`, its invocation started with token 0, given the
    session id the shared format-2 fixtures name their root by."""
    engine = _sm.create_engine(session_id="fixture-root")
    host = _Host().register(engine)
    engine.initialize()
    engine.send_event(engine.policy.resolve_event_by_name("start"))
    engine.advance_time(0)
    return engine, host


def _try_restore(text: str, elapsed_ms: int):
    engine = _sm.restore(saved_state_from_json(text), wall_now_ms=_SAVED_AT_MS + elapsed_ms)
    return engine


def _restored(text: str, elapsed_ms: int):
    """`text` restored `elapsed_ms` of wall-clock time after it was saved, its host
    invoker registered the way a host does it: on the engine the restore returned."""
    engine = _try_restore(text, elapsed_ms)
    return engine, _Host().register(engine)


def _standing_in(engine) -> set:
    return set(engine.active_configuration())


def test_a_machine_is_saved_with_the_request_its_invocation_was_started_with() -> None:
    engine, host = _working()
    first = host.starts()[0]
    assert not first.restarted, "the document's own start is not a restart"
    assert first.token == 0

    saved = _sm.save_at(engine, _SAVED_AT_MS)
    assert len(saved.host_invokes) == 1, "one running invocation"
    entry = saved.host_invokes[0]
    assert (entry.processor_type, entry.invoke_id, entry.src, entry.content, entry.data) == (
        _HOST_TYPE,
        "h",
        first.src,
        first.content,
        first.event_data,
    )
    # The deadline is the invocation's own, not a send waiting to be delivered.
    assert entry.due == _SAVED_AT_MS + 5000
    assert saved.pending == []
    # One start has been made, so the next token is 1.
    assert saved.host_invoke_token == 1
    # The text every backend writes for the same machine, byte for byte.
    assert saved.to_json() == _shared()


@pytest.mark.parametrize("format1", [False, True])
def test_a_restore_asks_nothing_of_the_host_until_the_machine_is_driven(format1: bool) -> None:
    # From the format-2 text, and from the format-1 text of the same run.
    engine, host = _restored(_shared(format1), 1000)
    assert host.log == [], "the restore itself calls nobody"
    assert _standing_in(engine) == {_State.WORKING}
    # The first macrostep starts it, where an invocation the document entered would
    # have started.
    engine.advance_time(0)
    assert len(host.starts()) == 1


def test_a_restored_invocation_is_started_again_from_the_request_it_was_saved_with() -> None:
    _, original = _working()
    first = original.starts()[0]

    engine, host = _restored(_shared(), 1000)
    engine.advance_time(0)
    assert len(host.starts()) == 1
    again = host.starts()[0]
    assert again.restarted, "the host is told this is a resumption"
    assert again.token == 1, "the next token the saved state carried on"
    assert _same_request(again, first), "the same request, not one evaluated again"


def test_a_restored_invocation_keeps_the_deadline_it_had_left() -> None:
    # Saved with 5000 ms to run, restored 1000 ms later: 4000 ms are left.
    engine, host = _restored(_shared(), 1000)
    engine.advance_time(0)
    engine.advance_time(3999)
    assert engine.policy.v_seen == 0, "not due yet"
    assert _standing_in(engine) == {_State.WORKING}
    engine.advance_time(1)
    assert engine.policy.v_seen == 2, "error.invoke.h, at 4000 ms"
    assert _standing_in(engine) == {_State.FAILED}
    # The host is told to stop what it was given, with the token it was given.
    kind, last = host.log[-1]
    assert kind == "cancel" and last.token == 1 and last.invoke_id == "h", host.log


def test_a_deadline_that_passed_while_the_machine_was_away_ends_the_invocation_unstarted() -> None:
    # 6 s after the save, and the deadline was at 5 s.
    engine, host = _restored(_shared(), 6000)
    engine.advance_time(0)
    assert host.log == [], "the host is not asked to begin what it would be told to stop"
    assert engine.policy.v_seen == 2, "the document hears error.invoke.h"
    assert _standing_in(engine) == {_State.FAILED}


@pytest.mark.parametrize("elapsed_ms", [1000, 6000])
def test_a_restored_machine_owing_a_start_says_it_needs_a_tick_now(elapsed_ms: int) -> None:
    # A host drives a machine by asking when it next needs a tick and sleeping that
    # long (`Engine.time_until_next_scheduled_ms`). A restored invocation owes the
    # machine a tick before anything else: it is started in the first macrostep, and
    # the deadline it keeps is armed there. A machine that answered "nothing" would be
    # left asleep for good, holding an invocation no one had started.
    engine, _ = _restored(_shared(), elapsed_ms)
    assert engine.time_until_next_scheduled_ms() == 0
    engine.advance_time(0)


def test_what_the_first_run_handed_a_host_cannot_answer_for_the_second() -> None:
    # Token 0 belonged to the process that saved the machine. A late reply carrying
    # it is a different process's, and is ignored (§scxml-6.4).
    engine, host = _restored(_shared(), 1000)
    engine.advance_time(0)
    token = host.starts()[0].token
    assert not engine.complete_host_invoke(_HOST_TYPE, "h", 0, "late"), "a reply of the first run is accepted"
    engine.advance_time(0)
    assert engine.policy.v_seen == 0 and _standing_in(engine) == {_State.WORKING}
    assert engine.complete_host_invoke(_HOST_TYPE, "h", token, "done"), "the reply of this run is refused"
    engine.advance_time(0)
    assert engine.policy.v_seen == 1 and _standing_in(engine) == {_State.AFTER}


def test_a_completion_still_queued_when_the_machine_was_saved_is_accepted_after_the_restore() -> None:
    engine, host = _working()
    # The host answers; the completion is on the external queue, stamped with the
    # start's token, and the machine has not been driven through it.
    assert engine.complete_host_invoke(_HOST_TYPE, "h", 0, "done"), "the completion is accepted"
    saved = _sm.save_at(engine, _SAVED_AT_MS)
    assert saved.host_invokes == [], "the invocation ended"
    assert [(e.name, e.host_invoke_token) for e in saved.external] == [("done.invoke.h", 0)], (
        "one queued completion carrying its token"
    )
    assert [kind for kind, _ in host.log] == ["start"], "started once, never cancelled"

    again, again_host = _restored(saved.to_json(), 0)
    again.advance_time(0)
    assert again.policy.v_seen == 1, "taken once, not refused"
    assert _standing_in(again) == {_State.AFTER}
    assert again_host.log == [], "an ended invocation is not started again"
    assert again.refused_host_invoke_completions() == 0, "the completion was refused"


def test_a_completion_the_engine_was_about_to_refuse_is_still_refused_after_the_restore() -> None:
    # A completion that carries no token did not come through `complete_host_invoke`,
    # so the engine refuses it when it is dequeued, because it may be a cancelled run's
    # reply. This engine refuses one at the door, so a state of its own never queues
    # one; another backend's may, and the restored machine refuses it as that one
    # would.
    shared = _shared()
    queued = (
        '"external":[{"name":"done.invoke.h","data":"","type":"external","sendid":"","origin":"",'
        '"origintype":"","invokeid":"h","hostinvoketoken":null}]'
    )
    text = shared.replace('"external":[]', queued, 1)
    assert text != shared, "the queue was changed"
    saved = saved_state_from_json(text)
    assert saved.external[0].host_invoke_token is None, "the event carries no token"

    again, _ = _restored(text, 0)
    again.advance_time(0)
    assert again.refused_host_invoke_completions() == 1
    assert again.policy.v_seen == 0 and _standing_in(again) == {_State.WORKING}


def test_an_invocation_cancelled_before_it_was_started_again_is_never_started() -> None:
    engine, host = _restored(_shared(), 1000)
    # Nothing was started, so there is nothing to tell the host to stop.
    assert not engine.cancel_host_invoke(_HOST_TYPE, "h"), "there is no run to cancel"
    engine.advance_time(0)
    assert host.log == []


def test_a_machine_restored_and_saved_again_before_it_was_driven_keeps_its_invocation() -> None:
    engine, _ = _restored(_shared(), 1000)
    again = _sm.save_at(engine, _SAVED_AT_MS + 1000)
    # The same request, due at the same wall-clock moment, and the same counter.
    assert again.to_json() == _shared()


def test_a_restart_nobody_can_run_is_the_error_a_start_nobody_can_run_is() -> None:
    engine = _try_restore(_shared(), 1000)
    # No invoker registered by the first macrostep.
    engine.advance_time(0)
    assert engine.unhandled_error_events() == 1, "error.execution"


@pytest.mark.parametrize(
    "what, old, new, expected",
    [
        ("an id the document does not invoke", '"id":"h"', '"id":"stranger"', "which the document does not have a host invoker run"),
        ("a type the document does not hand that id to", '"type":"x-sce-host"', '"type":"x-other"', "which the document does not have a host invoker run"),
    ],
)
def test_a_saved_state_naming_a_host_invocation_the_document_cannot_have_run_is_refused(
    what: str, old: str, new: str, expected: str
) -> None:
    shared = _shared()
    with pytest.raises(StateRefusal) as refused:
        _try_restore(shared.replace(old, new, 1), 0)
    assert expected in str(refused.value), what


def test_a_host_invocation_whose_state_the_configuration_is_not_in_is_refused() -> None:
    shared = _shared()
    text = shared.replace('"configuration":["working"]', '"configuration":["idle"]', 1).replace(
        '"current":"working"', '"current":"idle"', 1
    )
    with pytest.raises(StateRefusal) as refused:
        _try_restore(text, 0)
    assert "whose state the saved configuration does not stand in" in str(refused.value)


def test_an_invocation_named_twice_is_refused() -> None:
    shared = _shared()
    start = shared.index('"hostinvokes":[') + len('"hostinvokes":[')
    end = start + shared[start:].index('],"hostinvoketoken"')
    one = shared[start:end]
    twice = shared.replace(one, one + "," + one, 1)
    with pytest.raises(StateRefusal) as refused:
        _try_restore(twice, 0)
    assert "which an earlier entry already names" in str(refused.value)


@pytest.mark.parametrize(
    "what, old, new, expected",
    [
        ("no hostinvokes", '"hostinvokes":[', '"hostinvokes_":[', "has no 'hostinvokes'"),
        ("no hostinvoketoken", '"hostinvoketoken":"1",', "", "has no 'hostinvoketoken'"),
        ("a token that is a number", '"hostinvoketoken":"1"', '"hostinvoketoken":1', "'hostinvoketoken' is not a text"),
        (
            "a token past a signed 64-bit count",
            '"hostinvoketoken":"1"',
            '"hostinvoketoken":"9223372036854775808"',
            "is not a whole number a token can be",
        ),
        ("a deadline that is not a moment", '"due":"1700000005000"', '"due":"soon"', "is not a whole number of milliseconds"),
        ("a deadline that is a number", '"due":"1700000005000"', '"due":1700000005000', "'hostinvokes[0].due' is neither a text nor null"),
    ],
)
def test_the_host_invoke_fields_are_required_and_must_be_what_they_say(
    what: str, old: str, new: str, expected: str
) -> None:
    shared = _shared()
    assert old in shared, what
    with pytest.raises(StateRefusal) as refused:
        saved_state_from_json(shared.replace(old, new, 1))
    assert expected in str(refused.value), what
