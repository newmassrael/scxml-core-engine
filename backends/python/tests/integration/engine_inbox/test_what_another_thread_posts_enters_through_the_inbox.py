# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""docs/adr/0084, ARCHITECTURE.md "Engine Threading": a Python engine is
confined to the thread that drives it, and what another thread has for it
enters through its inbox (`Engine.inbox`).

The contract this replaces told a host to guard the engine with a lock of its
own, and a host that ended a host-run invocation from its worker thread mutated
the running-invocation table beside a macrostep. The cases hold the inbox to
the rule:

* many threads post endings, external events and Mesh deliveries while
  macrosteps run, and what the machine takes, and what is left running, are
  those of a single-threaded replay of the same posts;
* an ending posted for a start that was cancelled, or started again, is refused
  by the token check, as a direct one is (§scxml-6.4);
* a Mesh delivery is applied as `apply_mesh_call` applies it, on the session it
  names;
* a post is taken before the run starts and refused once the run is stopped or
  has ended, and what the inbox holds then is applied as a direct call would be.

The machine is written by hand, as the smallest one with a host-run invocation:
`working` (initial) starts `job` on entry and cancels it on exit; `leave` goes
to `idle` and `go` back; `finish` ends the run in the top-level final `done`.
`ping`, `done.invoke.job` and `error.invoke.job` are taken in `working` and
`idle` and recorded with their `_event.data`. It needs no script, so no
generated module stands behind it.

This directory is not a fixture stem: it holds the runtime's inbox to its
contract over a machine written by hand, and no document drives it. It sits
under `integration/` so the Python lane's run of that directory collects it.
"""
from __future__ import annotations

import sys
import threading
import time
from enum import Enum
from pathlib import Path
from typing import Callable, Dict, List, Optional, Sequence, Tuple

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "runtime"))

from sce_runtime import (  # noqa: E402
    EnabledTransition,
    Engine,
    EntryTarget,
    EventMetadata,
    HostInvokeEnding,
    HostInvokeEvent,
    HostInvokeRequest,
    HostInvokeResponse,
    InboxExternalEvent,
    InboxExternalEventByName,
    InboxHostInvokeEnding,
    InboxInput,
    InboxMeshDelivery,
    LuaScriptEngine,
    MeshRaise,
    PendingInvoke,
    StatePolicy,
    StateRefusal,
    StateTarget,
)

#: The host-run invocation's type, its id, and the session id both the run and
#: its replay are built with, so an ending addressed to the session reaches
#: either.
_TYPE = "x-sce-probe"
_JOB = "job"
_SESSION = "inbox-probe"

#: Bounded far above anything a case waits for, so a stall names its stage.
_BOUND_SECONDS = 10.0


class _State(Enum):
    WORKING = "working"
    IDLE = "idle"
    DONE = "done"


class _Event(Enum):
    NULL = ""
    GO = "go"
    LEAVE = "leave"
    FINISH = "finish"
    PING = "ping"
    JOB_DONE = "done.invoke.job"
    JOB_FAILED = "error.invoke.job"


_EVENT_BY_NAME: Dict[str, _Event] = {event.value: event for event in _Event if event.value}
_STATE_BY_NAME: Dict[str, _State] = {state.value: state for state in _State}
_DOCUMENT_ORDER: Dict[_State, int] = {_State.WORKING: 0, _State.IDLE: 1, _State.DONE: 2}

#: Each recorded event's transition index, and what its record begins with.
_RECORDED: Dict[_Event, Tuple[int, str]] = {
    _Event.PING: (2, "ping"),
    _Event.JOB_DONE: (3, "done"),
    _Event.JOB_FAILED: (4, "failed"),
}
_RECORD_BY_INDEX: Dict[int, str] = {index: record for index, record in _RECORDED.values()}


class _InboxProbe(StatePolicy[_State, _Event]):
    """The machine the module docstring describes. `taken` is what it took, in
    order, each as ``<record>:<_event.data>``; written by the owner only."""

    def __init__(self) -> None:
        self.taken: List[str] = []
        self._data: object = ""

    def initial_state(self) -> _State:
        return _State.WORKING

    def get_document_initial_targets(self) -> Sequence[EntryTarget]:
        return (StateTarget(_State.WORKING),)

    def is_final_state(self, state: _State) -> bool:
        return state is _State.DONE

    def get_parent(self, state: _State) -> Optional[_State]:
        return None

    def get_state_name(self, state: _State) -> str:
        return state.value

    def get_state_from_name(self, name: str) -> Optional[_State]:
        return _STATE_BY_NAME.get(name)

    def get_event_name(self, event: _Event) -> str:
        return event.value

    def null_event(self) -> _Event:
        return _Event.NULL

    def first_enabled_transition(
        self, state: _State, event: _Event, engine: "Engine[_State, _Event]"
    ) -> Optional[EnabledTransition[_State]]:
        if state is _State.DONE:
            return None
        if event is _Event.GO and state is _State.IDLE:
            return EnabledTransition(
                source=state, targets=(StateTarget(_State.WORKING),), transition_index=0, event="go"
            )
        if event is _Event.LEAVE and state is _State.WORKING:
            return EnabledTransition(
                source=state, targets=(StateTarget(_State.IDLE),), transition_index=1, event="leave"
            )
        if event is _Event.FINISH and state is _State.WORKING:
            return EnabledTransition(
                source=state, targets=(StateTarget(_State.DONE),), transition_index=5, event="finish"
            )
        recorded = _RECORDED.get(event)
        if recorded is None:
            return None
        return EnabledTransition(
            source=state, targets=(), transition_index=recorded[0], event=event.value
        )

    def execute_entry_actions(
        self, state: _State, engine: "Engine[_State, _Event]", is_default_entry: bool
    ) -> None:
        return None

    def execute_exit_actions(self, state: _State, engine: "Engine[_State, _Event]") -> None:
        return None

    def execute_transition_content(
        self, state: _State, transition_index: int, engine: "Engine[_State, _Event]"
    ) -> None:
        record = _RECORD_BY_INDEX.get(transition_index)
        if record is not None:
            self.taken.append(f"{record}:{self._data}")

    def get_document_order(self, state: _State) -> int:
        return _DOCUMENT_ORDER[state]

    def set_current_event(self, event: _Event, metadata: EventMetadata) -> None:
        self._data = metadata.data

    def get_event_from_name(self, event_name: str) -> Optional[_Event]:
        return _EVENT_BY_NAME.get(event_name)

    def host_invoke_ids(self) -> Tuple[str, ...]:
        return (_JOB,)

    def defer_invokes_on_entry(self, state: _State, engine: "Engine[_State, _Event]") -> None:
        if state is _State.WORKING:
            engine._pending_invokes.append(PendingInvoke(_JOB, _State.WORKING.value, _JOB))

    def cancel_invokes_for_state(self, state: _State, engine: "Engine[_State, _Event]") -> None:
        if state is not _State.WORKING:
            return
        engine._pending_invokes = [
            pending for pending in engine._pending_invokes if pending.document_id != _JOB
        ]
        engine.cancel_host_invoke(_TYPE, _JOB)

    def execute_pending_invokes(self, engine: "Engine[_State, _Event]") -> None:
        pending = list(engine._pending_invokes)
        engine._pending_invokes.clear()
        for invoke in pending:
            if invoke.document_id == _JOB and _State.WORKING in engine.active_configuration():
                engine.perform_host_invoke(HostInvokeRequest(processor_type=_TYPE, invoke_id=_JOB))


def _probe(
    on_cancel: Optional[Callable[["Engine[_State, _Event]"], None]] = None,
) -> Tuple["Engine[_State, _Event]", _InboxProbe, List[str]]:
    """A machine of its own, its host's log (``start:<token>`` and
    ``cancel:<token>``) and the host's invoker registered: a start is answered
    with nothing, so `job` runs until it is ended or cancelled. `on_cancel` is
    what the host also does when it is told to stop."""
    script_engine = LuaScriptEngine()
    script_engine.initialize()
    policy = _InboxProbe()
    engine: "Engine[_State, _Event]" = Engine(policy, script_engine, session_id=_SESSION)
    log: List[str] = []

    def invoker(event: HostInvokeEvent) -> Optional[HostInvokeResponse]:
        if event.start is not None:
            log.append(f"start:{event.start.token}")
        if event.cancel is not None:
            log.append(f"cancel:{event.cancel.token}")
            if on_cancel is not None:
                on_cancel(engine)
        return None

    engine.register_invoker(_TYPE, invoker)
    return engine, policy, log


def _latest_token(log: List[str]) -> int:
    """The token of the latest start `log` recorded."""
    for line in reversed(log):
        if line.startswith("start:"):
            return int(line[len("start:"):])
    raise AssertionError(f"`job` never started: {log}")


def _ending(token: int, done: bool, data: str, session: Optional[str] = None) -> InboxHostInvokeEnding:
    return InboxHostInvokeEnding(HostInvokeEnding(_TYPE, _JOB, token, done, data), session_id=session)


def _ping(data: str) -> InboxExternalEvent:
    return InboxExternalEvent(_Event.PING, EventMetadata(data=data))


def _post_of(producer: int, k: int, token: int) -> InboxInput:
    """What producer `producer` posts `k`-th: external events, typed and by
    name, a Mesh delivery addressed to the session, and now and then an ending
    of start `token`, completed or failed, of the machine or addressed to its
    session. Each data names its thread and its place, so the record says which
    post it came from."""
    data = f"p{producer}-{k}"
    kind = k % 7
    if kind == 3:
        return _ending(token, True, data)
    if kind == 5:
        return _ending(token, False, data, session=_SESSION)
    if kind == 1:
        return InboxExternalEventByName("ping", data=data)
    if kind == 6:
        return InboxMeshDelivery(MeshRaise("ping", EventMetadata(data=data)), session_id=_SESSION)
    return _ping(data)


def _expected_from(posted: List[InboxInput]) -> List[str]:
    """What the machine must take from `posted`, in the order the posts were
    taken: every event, and of the endings only the first, since it ends `job`
    and the token check refuses the rest."""
    ended = False
    expected: List[str] = []
    for posted_input in posted:
        if isinstance(posted_input, InboxHostInvokeEnding):
            if not ended:
                ended = True
                ending = posted_input.ending
                expected.append(("done:" if ending.done else "failed:") + ending.data)
        elif isinstance(posted_input, InboxExternalEvent):
            assert posted_input.metadata is not None
            expected.append(f"ping:{posted_input.metadata.data}")
        elif isinstance(posted_input, InboxExternalEventByName):
            expected.append(f"ping:{posted_input.data}")
        else:
            call = posted_input.call
            assert isinstance(call, MeshRaise)
            expected.append(f"ping:{call.metadata.data}")
    return expected


def _post_from_many_threads(
    engine: "Engine[_State, _Event]",
    token: int,
    producers: int,
    per_producer: int,
    wake: threading.Semaphore,
    while_running: Callable[[], None],
) -> List[InboxInput]:
    """Start `producers` threads, release them together, and have each post
    `per_producer` inputs to `engine`. The order the inbox took them in is
    recorded under one lock with the post, so it is the order of the inbox
    itself. `while_running` is the owner's work for as long as any producer
    runs; each producer releases `wake` once it has stopped counting itself as
    running, so an owner asleep on `wake` is not left waiting by the last one."""
    posted: List[InboxInput] = []
    failures: List[str] = []
    order = threading.Lock()
    running = [producers]
    together = threading.Barrier(producers + 1, timeout=_BOUND_SECONDS)

    def produce(producer: int) -> None:
        try:
            together.wait()
            for k in range(per_producer):
                posted_input = _post_of(producer, k, token)
                with order:
                    if engine.inbox.post(posted_input):
                        posted.append(posted_input)
                    else:
                        failures.append(f"post {producer}/{k} was refused")
                # Hand the interpreter to another thread, so posts land between
                # and during the owner's macrosteps rather than in one burst.
                time.sleep(0)
        except BaseException as failure:  # noqa: BLE001 — reported below
            with order:
                failures.append(f"producer {producer}: {failure!r}")
        finally:
            with order:
                running[0] -= 1
            wake.release()

    threads = [
        threading.Thread(target=produce, args=(producer,), name=f"inbox-producer-{producer}")
        for producer in range(producers)
    ]
    for thread in threads:
        thread.start()
    together.wait()

    def any_running() -> bool:
        with order:
            return running[0] > 0

    while any_running():
        while_running()
    for thread in threads:
        thread.join(_BOUND_SECONDS)
        assert not thread.is_alive(), f"{thread.name} never finished posting"
    assert failures == [], f"an open inbox refused posts: {failures}"
    with order:
        return list(posted)


def _replay(posted: List[InboxInput]) -> Tuple[List[str], List[str]]:
    """`posted` replayed on a machine of its own by one thread: every post, then
    one drive."""
    engine, policy, log = _probe()
    engine.initialize()
    for posted_input in posted:
        assert engine.inbox.post(posted_input), "the replay's inbox refused a post"
    engine.advance_time(0)
    engine.stop()
    return list(policy.taken), list(log)


def _post_from_another_thread(engine: "Engine[_State, _Event]", posted_input: InboxInput) -> bool:
    """Post `posted_input` from a thread of its own, as a host's worker does,
    and answer whether the inbox took it."""
    answers: List[bool] = []
    thread = threading.Thread(target=lambda: answers.append(engine.inbox.post(posted_input)))
    thread.start()
    thread.join(_BOUND_SECONDS)
    assert not thread.is_alive(), "the post on another thread never returned"
    assert len(answers) == 1, "the post on another thread raised"
    return answers[0]


def test_posts_from_many_threads_during_macrosteps_are_taken_as_a_single_threaded_replay_takes_them() -> None:
    engine, policy, log = _probe()
    # The waker is what tells this owner there is something to apply; the owner
    # sleeps on it between drives rather than on a clock.
    wake = threading.Semaphore(0)
    engine.inbox.wake_with(wake.release)
    engine.initialize()
    token = _latest_token(log)

    def drive() -> None:
        wake.acquire(timeout=_BOUND_SECONDS)
        engine.advance_time(0)

    posted = _post_from_many_threads(engine, token, producers=8, per_producer=60, wake=wake, while_running=drive)
    # What the last producer posted after the owner's last drive.
    engine.advance_time(0)
    # The invocation table: the first ending ended `job`, so the stop that exits
    # `working` has nothing to cancel.
    engine.stop()

    assert policy.taken == _expected_from(posted), (
        "the machine did not take the posts in the order the inbox took them"
    )
    assert log == [f"start:{token}"], f"an ending was not applied, or a refused one was: {log}"
    replay_taken, replay_log = _replay(posted)
    assert policy.taken == replay_taken, "the run differs from a single-threaded replay of its posts"
    assert log == replay_log, "the invocations differ from a single-threaded replay of the posts"


def test_a_posted_ending_of_a_cancelled_or_restarted_start_is_refused_as_a_direct_one_is() -> None:
    engine, policy, log = _probe()
    engine.initialize()
    first = _latest_token(log)
    # `leave` exits `working`, which cancels the first start.
    engine.send_event(_Event.LEAVE)
    assert log == [f"start:{first}", f"cancel:{first}"]

    assert _post_from_another_thread(engine, _ending(first, True, "after-cancel"))
    engine.advance_time(0)
    assert policy.taken == [], "a cancelled start's posted ending was taken"
    assert not engine.complete_host_invoke(_TYPE, _JOB, first, "direct"), (
        "a cancelled start's direct ending was taken"
    )

    # `go` enters `working` again: the same invocation, a new start.
    engine.send_event(_Event.GO)
    second = _latest_token(log)
    assert second != first, "a restart reused the first start's token"

    assert _post_from_another_thread(engine, _ending(first, False, "stale"))
    # A session the tree does not hold takes nothing, and the drive that applies
    # it is not refused for it.
    assert _post_from_another_thread(engine, _ending(second, True, "astray", session="nowhere"))
    # Addressed by the session id, as a host serving a tree ends one.
    assert _post_from_another_thread(engine, _ending(second, True, "fresh", session=_SESSION))
    engine.advance_time(0)
    assert policy.taken == ["done:fresh"], "the first start's ending was taken for the second's"
    assert not engine.fail_host_invoke(_TYPE, _JOB, first, "direct"), (
        "the first start's direct ending was taken"
    )
    with pytest.raises(StateRefusal):
        engine.fail_host_invoke_in(_SESSION, _TYPE, _JOB, first, "direct")


def test_a_mesh_delivery_is_posted_and_applied_as_apply_mesh_call_applies_it() -> None:
    engine, policy, log = _probe()
    engine.initialize()
    token = _latest_token(log)

    peer = MeshRaise("ping", EventMetadata(data="peer"))
    assert _post_from_another_thread(engine, InboxMeshDelivery(peer))
    # Addressed by the session id: this machine.
    addressed = MeshRaise("ping", EventMetadata(data="addressed"))
    assert _post_from_another_thread(engine, InboxMeshDelivery(addressed, session_id=_SESSION))
    # A session the tree does not hold takes nothing.
    astray = MeshRaise("ping", EventMetadata(data="astray"))
    assert _post_from_another_thread(engine, InboxMeshDelivery(astray, session_id="nowhere"))
    engine.advance_time(0)

    assert policy.taken == ["ping:peer", "ping:addressed"]
    assert log == [f"start:{token}"]


def test_a_post_before_the_run_is_taken_by_it_and_one_after_the_stop_is_refused() -> None:
    engine, policy, _log = _probe()
    woken = [0]

    def waker() -> None:
        woken[0] += 1

    engine.inbox.wake_with(waker)
    assert engine.inbox.post(_ping("early"))
    assert woken[0] == 1, "a taken post did not wake the owner"
    assert policy.taken == [], "a post was applied before the run started"
    # A host asleep on the next deadline is told the post is due now.
    assert engine.time_until_next_scheduled_ms() == 0
    # Applied at the first macrostep boundary of the run.
    engine.initialize()
    assert policy.taken == ["ping:early"]
    assert engine.time_until_next_scheduled_ms() is None, "an applied post was still owed a drive"

    engine.stop()
    assert not engine.inbox.post(_ping("late")), "a stopped engine's inbox took a post"
    assert woken[0] == 1, "a refused post woke the owner"
    assert engine.inbox.refused_posts() == 1

    # A run started again has an open inbox.
    engine.initialize()
    assert engine.inbox.post(InboxExternalEventByName("ping", data="again"))
    engine.advance_time(0)
    assert policy.taken == ["ping:early", "ping:again"]


def test_what_the_inbox_holds_when_the_run_is_stopped_is_counted_unseen() -> None:
    engine, policy, _log = _probe()
    engine.initialize()
    assert engine.inbox.post(_ping("held"))
    engine.stop()

    assert policy.taken == []
    assert engine.unseen_external_events() == 1, "a post the stop found was not applied"
    assert engine.last_unseen_event() is _Event.PING


def test_what_is_posted_while_the_run_ends_is_counted_unseen_and_the_inbox_closes() -> None:
    # The host, told to stop `job` as `finish` leaves `working` for the final
    # `done`, posts from inside that last macrostep.
    def post_on_cancel(engine: "Engine[_State, _Event]") -> None:
        assert engine.inbox.post(_ping("while-ending"))

    engine, policy, log = _probe(on_cancel=post_on_cancel)
    engine.initialize()
    token = _latest_token(log)
    engine.send_event(_Event.FINISH)

    assert engine.reached_final, "`finish` did not end the run"
    assert log == [f"start:{token}", f"cancel:{token}"]
    assert policy.taken == []
    assert engine.unseen_external_events() == 1, "the post made while the run ended was not applied"
    assert not engine.inbox.post(_ping("after")), "an ended run's inbox took a post"
