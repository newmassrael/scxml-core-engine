# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
* the external queue takes events in the order they arrived: a post made while
  a macrostep runs is applied before an event that macrostep then puts on the
  queue itself, a `<send>` to itself or a host-run invocation's `done.invoke`
  (W3C test 201's shape);
* an ending posted for a start that was cancelled, or started again, is refused
  by the token check, as a direct one is (§scxml-6.4);
* a Mesh delivery is applied as `apply_mesh_call` applies it, on the session it
  names;
* a post is taken before the run starts and refused once the run is stopped or
  has ended, and what the inbox holds then is applied as a direct call would be;
* an ending posted for a child session is on that session's queue from the
  moment it is posted, so the child takes it before an event its parent sends it
  later in the same macrostep (docs/adr/0084, D2 as amended at step 10);
* each run of the inbox under posters on several threads is recorded as an
  inbox history (docs/adr/0147, D3): every post with what the inbox answered,
  every item the owner put on the external queue as the engine's inbox port
  tells it, and the end of the run, against one clock. Each history is held
  here to the arrival order the drain before an event of the owner's own keeps,
  and written where SCE_INBOX_HISTORY_DIR names, for
  `sce-codegen check-inbox-history` to judge linearizable to
  docs/formal/InboxService.tla (scripts/gates/w3c-python.sh runs it).

The last case drives `static_invoke_ending_order.scxml`, the machine
`scripts/materialize.sh python-static-datamodel` generates from
`sce-build/tests/fixtures/static_datamodel/`, imported when the case runs.

The machine of the other cases is written by hand, as the smallest one with a
host-run invocation:
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

import importlib
import json
import os
import sys
import threading
import time
from enum import Enum
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Sequence, Tuple, TypeVar

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "runtime"))
# The tests directory, so the generated machines are importable by the name they
# have there, and the forge runtime they import.
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "forge-runtime"))

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
    on_start: Optional[Callable[["Engine[_State, _Event]"], Optional[HostInvokeResponse]]] = None,
) -> Tuple["Engine[_State, _Event]", _InboxProbe, List[str]]:
    """A machine of its own, its host's log (``start:<token>`` and
    ``cancel:<token>``) and the host's invoker registered: a start is answered
    with nothing, so `job` runs until it is ended or cancelled. `on_cancel` is
    what the host also does when it is told to stop; `on_start` what it does
    when it is told to start, and its answer is the start's."""
    script_engine = LuaScriptEngine()
    script_engine.initialize()
    policy = _InboxProbe()
    engine: "Engine[_State, _Event]" = Engine(policy, script_engine, session_id=_SESSION)
    log: List[str] = []

    def invoker(event: HostInvokeEvent) -> Optional[HostInvokeResponse]:
        response: Optional[HostInvokeResponse] = None
        if event.start is not None:
            log.append(f"start:{event.start.token}")
            if on_start is not None:
                response = on_start(engine)
        if event.cancel is not None:
            log.append(f"cancel:{event.cancel.token}")
            if on_cancel is not None:
                on_cancel(engine)
        return response

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


@pytest.mark.parametrize(
    ("own", "expected"),
    [
        ("send", ["ping:posted", "ping:own"]),
        ("done.invoke", ["ping:posted", "done:own"]),
    ],
)
def test_an_event_posted_before_the_owners_own_external_event_is_taken_first(
    own: str, expected: List[str]
) -> None:
    # W3C test 201's shape, without the HTTP listener: inside the macrostep that
    # entered `working`, the host told to start `job` waits for a thread that
    # posts `ping` (as a BasicHTTP send waits for the listener that delivers
    # its event), and the owner then puts an event of its own on the external
    # queue: a `<send>` to itself, or the `done.invoke` of a start the host
    # answers as done. The post arrived first, so it is taken first. The join
    # is the only ordering; nothing sleeps.
    answers: List[bool] = []

    def start(engine: "Engine[_State, _Event]") -> Optional[HostInvokeResponse]:
        answers.append(_post_from_another_thread(engine, _ping("posted")))
        if own == "send":
            engine.send_to_self(_Event.PING, data="own")
            return None
        return HostInvokeResponse(done_data="own")

    engine, policy, _log = _probe(on_start=start)
    engine.initialize()

    assert answers == [True], "the start was not asked, or the inbox refused the post"
    assert policy.taken == expected, (
        "the owner's own event overtook an event posted before it"
    )
    assert engine.inbox.refused_posts() == 0


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
    # A post naming a session the tree does not hold is refused when it is
    # posted, and says so (docs/adr/0084, D2 as amended at step 10).
    assert not _post_from_another_thread(engine, _ending(second, True, "astray", session="nowhere")), (
        "a post naming a session the tree does not hold was taken"
    )
    assert engine.inbox.refused_posts() == 1
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
    # A post naming a session the tree does not hold is refused when it is posted.
    astray = MeshRaise("ping", EventMetadata(data="astray"))
    assert not _post_from_another_thread(engine, InboxMeshDelivery(astray, session_id="nowhere")), (
        "a delivery naming a session the tree does not hold was taken"
    )
    assert engine.inbox.refused_posts() == 1
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


# ── an ending posted for a child session (docs/adr/0084, D2 at step 10) ─────

#: The standard host type the fixture's child asks, and its invocation's id.
_CLOCK = "sce-host-clock-now"
_CLOCK_ID = "clock"
#: A completion of the clock the child's result record reads.
_NOW = '{"error":"","utc_ms":1700000000000,"monotonic_ms":5}'
#: The id the fixture's root is built under, and the session `worker` is: the
#: root's id and the id its first start is started under.
_ORDER_ROOT = "fixture-root"
_ORDER_WORKER = "fixture-root.working.1.worker"
#: How many drives a case gives the machine to settle before it says it did not.
_DRIVES = 16


class _EndingOrderHost:
    """The host of `static_invoke_ending_order`. It runs the clock and does not
    answer, recording each start; on `beforeQuestion`, which the parent performs
    inside the macrostep that then sends `question` to `worker`, it posts the
    clock's ending, naming the session that started it, from a thread of its own
    joined before the action returns, and records whether the inbox took it."""

    def __init__(self) -> None:
        self.engine: Optional[Engine] = None
        self.starts: List[HostInvokeRequest] = []
        self.posted: List[bool] = []

    def serve(self, event: HostInvokeEvent) -> Optional[HostInvokeResponse]:
        if event.start is not None:
            self.starts.append(event.start)
        return None

    def before_question(self) -> None:
        if self.engine is None or not self.starts:
            self.posted.append(False)
            return
        request = self.starts[-1]
        ending = InboxHostInvokeEnding(
            HostInvokeEnding(_CLOCK, _CLOCK_ID, request.token, True, _NOW),
            session_id=request.session_id,
        )
        self.posted.append(_post_from_another_thread(self.engine, ending))


def _drive_until(engine: Engine, reached: Callable[[], bool], what: str) -> None:
    """Drive `engine` without moving its clock until `reached` holds."""
    for _ in range(_DRIVES):
        if reached():
            return
        engine.advance_time(0)
    assert reached(), f"{what} was not reached in {_DRIVES} drives"


def _drive_until_quiet(engine: Engine) -> None:
    """Drive `engine` without moving its clock until nothing is owed a drive now."""
    engine.advance_time(0)
    for _ in range(_DRIVES):
        if engine.time_until_next_scheduled_ms() != 0:
            return
        engine.advance_time(0)
    raise AssertionError(f"the machine still owed a drive after {_DRIVES} drives")


def _ending_order_started() -> Tuple[Engine, _EndingOrderHost]:
    """The fixture's root, built under `_ORDER_ROOT` with the clock served, and
    driven until `worker` has started the clock."""
    module = importlib.import_module("integration.static_datamodel.static_invoke_ending_order_sm")
    host = _EndingOrderHost()
    engine = module.create_engine(host, session_id=_ORDER_ROOT)
    host.engine = engine
    engine.register_invoker(_CLOCK, host.serve)
    engine.initialize()
    _drive_until(engine, lambda: bool(host.starts), "`worker` starting the clock")
    assert [start.session_id for start in host.starts] == [_ORDER_WORKER], (
        "the clock was not started once, by `worker`"
    )
    return engine, host


def test_an_ending_posted_for_a_child_before_its_parents_send_is_taken_before_that_send() -> None:
    engine, host = _ending_order_started()

    engine.send_event(engine.policy.get_event_from_name("ask"))
    _drive_until_quiet(engine)

    assert host.posted == [True], "the host was not asked, or the inbox refused the ending it posted"
    stamped_at, answer_at = engine.policy.v_stamped_at, engine.policy.v_answer_at
    assert stamped_at == 1 and answer_at == 2, (
        "`worker` took the parent's `question` before the clock's ending, though the "
        f"ending was posted first: the parent heard `stamped` at {stamped_at} and "
        f"`answer` at {answer_at}, where 1 and 2 were owed"
    )


def test_an_ending_posted_for_a_child_wakes_the_roots_owner_and_its_next_drive_applies_it() -> None:
    engine, host = _ending_order_started()
    woken = [0]

    def waker() -> None:
        woken[0] += 1

    # Installed on the root, as a host does: the tree has one waker.
    engine.inbox.wake_with(waker)
    request = host.starts[-1]
    ending = InboxHostInvokeEnding(
        HostInvokeEnding(_CLOCK, _CLOCK_ID, request.token, True, _NOW),
        session_id=_ORDER_WORKER,
    )
    assert _post_from_another_thread(engine, ending), "the inbox refused an ending for a session the tree holds"
    assert woken[0] == 1, "a post to the child's inbox did not wake the tree's owner"
    # A host asleep on the next deadline is told the child's post is due now.
    assert engine.time_until_next_scheduled_ms() == 0, "the child's post was not owed a drive"

    # The root's own drive, which reaches the child.
    engine.advance_time(0)

    assert (engine.policy.v_stamped_at, engine.policy.v_answer_at) == (1, 0), (
        "the drive the host made on the root did not apply the ending posted for its child"
    )
    # The ending ended the clock: a second one for the same start is refused by
    # the token check when it is applied, and raises nothing.
    assert _post_from_another_thread(engine, ending)
    engine.advance_time(0)
    assert engine.policy.v_heard == 1, "an ending of a start already ended was taken"
    assert engine.inbox.refused_posts() == 0


# ── each run of the inbox, as an inbox history (docs/adr/0147, D3) ──────────

#: The number the history names the probe's one session by, and the session the
#: tree does not hold; the id a post naming that session gives.
_HISTORY_SESSION = 0
_HISTORY_ABSENT = 1
_HISTORY_ABSENT_ID = "inbox-history-absent"
#: Threads that post in a recorded run, beside the owner, and the posts each makes.
_HISTORY_POSTERS = 4
_HISTORY_PER_POSTER = 120
#: Own events the owner raises at most in a recorded run, and the first one's
#: value, past every input a poster posts.
_HISTORY_OWN_PER_RUN = 40
_HISTORY_OWN_BASE = 1_000_000
#: The input of the first post made once the run has ended, past every other input.
_HISTORY_LATE_INPUT = 900_000
#: Runs recorded, each written as one history: the schedules that matter are a
#: small fraction of those a machine gives.
_HISTORY_RUNS = 4

_T = TypeVar("_T")


class _HistoryClock:
    """The one clock every thread of a recorded run reads, immediately before
    and immediately after each operation it records: a read-modify-write chain
    on one lock orders the readings with the calls between them, so "returned
    before invoked" in the record means it in the run, and no two readings are
    equal."""

    def __init__(self) -> None:
        self._lock = threading.Lock()
        self._ticks = 0

    def tick(self) -> int:
        with self._lock:
            self._ticks += 1
            return self._ticks


class _RecordedPost:
    """One post a poster observed: through the tree's directory (`addressed`)
    or the session's own handle, taken or refused."""

    __slots__ = ("session", "input", "addressed", "taken", "invoked", "returned")

    def __init__(
        self, session: int, value: int, addressed: bool, taken: bool, invoked: int, returned: int
    ) -> None:
        self.session = session
        self.input = value
        self.addressed = addressed
        self.taken = taken
        self.invoked = invoked
        self.returned = returned

    @property
    def reason(self) -> Optional[str]:
        """Why a refused post was refused. The inbox answers whether it took a
        post, not why, so the reason is the one a post of its kind can have:
        `unknown` for one naming the session the tree does not hold, and
        `ended` for one reaching the session the tree holds, whose inbox closes
        with its run. The checker holds the refusal to the place the post
        linearized at, so a post refused while the session ran is not
        linearizable as `ended`."""
        if self.taken:
            return None
        return "unknown" if self.session == _HISTORY_ABSENT else "ended"

    def document(self) -> Dict[str, Any]:
        """The post in the inbox history's form."""
        posted: Dict[str, Any] = {
            "call": "post",
            "session": self.session,
            "input": self.input,
            "door": "addressed" if self.addressed else "handle",
            "outcome": "taken" if self.taken else "refused",
        }
        if self.reason is not None:
            posted["reason"] = self.reason
        posted["invoked"] = self.invoked
        posted["returned"] = self.returned
        return posted


class _OwnerRecorder:
    """What the owner of a recorded run did, as its engine's inbox port
    (`sce_runtime.InboxPort`) tells it: each item it put on the external queue,
    read from the run's clock around it, and the end of the run, read around the
    close of the inbox, from which every post is refused. An event of the
    owner's own is invoked where the port is first told it, before the drain it
    begins with, and the inputs that drain applies lie within it. Told on the
    owner only."""

    def __init__(self, clock: _HistoryClock) -> None:
        self._clock = clock
        self.operations: List[Dict[str, Any]] = []
        self._own_invoked = 0
        self._posted_invoked = 0
        self._close_invoked = 0

    def enqueue_begins(self, event: Any, metadata: EventMetadata, posted: bool) -> None:
        now = self._clock.tick()
        if posted:
            self._posted_invoked = now
        else:
            self._own_invoked = now

    def enqueue_ends(self, event: Any, metadata: EventMetadata, posted: bool) -> None:
        self.operations.append(
            {
                "call": "enqueue",
                "session": _HISTORY_SESSION,
                "item": int(metadata.data),
                "source": "posted" if posted else "own",
                "invoked": self._posted_invoked if posted else self._own_invoked,
                "returned": self._clock.tick(),
            }
        )

    def close_begins(self) -> None:
        self._close_invoked = self._clock.tick()

    def close_ends(self) -> None:
        self.operations.append(
            {
                "call": "end",
                "session": _HISTORY_SESSION,
                "invoked": self._close_invoked,
                "returned": self._clock.tick(),
            }
        )


def _history_ping_naming(session_id: str, value: int) -> InboxMeshDelivery:
    """`ping` carrying `value`, for the tree's directory to post to the session
    `session_id` names."""
    return InboxMeshDelivery(MeshRaise("ping", EventMetadata(data=str(value))), session_id=session_id)


def _record_post(
    clock: _HistoryClock, engine: "Engine[_State, _Event]", session: int, value: int, posted_input: InboxInput
) -> _RecordedPost:
    """Post `posted_input`, input `value` of `session`, to `engine`'s inbox,
    recorded with `clock` read immediately around the post. An input that names
    a session goes through the tree's directory, and any other through the
    session's own handle."""
    invoked = clock.tick()
    taken = engine.inbox.post(posted_input)
    returned = clock.tick()
    addressed = isinstance(posted_input, InboxMeshDelivery)
    return _RecordedPost(session, value, addressed, taken, invoked, returned)


def _on_another_thread(work: Callable[[], _T]) -> _T:
    """Run `work` on a thread of its own, join it, and answer what it answered."""
    answers: List[_T] = []
    thread = threading.Thread(target=lambda: answers.append(work()))
    thread.start()
    thread.join(_BOUND_SECONDS)
    assert not thread.is_alive(), "the work on another thread never finished"
    assert len(answers) == 1, "the work on another thread raised"
    return answers[0]


def _raise_own(engine: "Engine[_State, _Event]", value: int) -> None:
    """Raise the own event `value` on the owner, as the host's own raise does."""
    engine.send_event(_Event.PING, EventMetadata(data=str(value)))


def _record_inbox_run() -> Tuple[List[Dict[str, Any]], List[List[_RecordedPost]]]:
    """One recorded run: the owner's operations and each poster's posts, each in
    program order.

    A post is taken and returns before the owner raises an event of its own, so
    every run holds that order. Then `_HISTORY_POSTERS` threads post through the
    session's own handle and, every fifth post, through the tree's directory,
    naming the session the tree does not hold while the run goes on and the
    probe's own session once it has ended, while the owner drives the machine,
    raises events of its own and stops the run once half the posts are made. A
    post through each door once the run has ended closes the record."""
    clock = _HistoryClock()
    recorder = _OwnerRecorder(clock)
    engine, _policy, _log = _probe()
    engine.set_inbox_port(recorder)
    engine.initialize()
    posters: List[List[_RecordedPost]] = [
        [_on_another_thread(lambda: _record_post(clock, engine, _HISTORY_SESSION, 1, _ping("1")))]
    ]
    _raise_own(engine, _HISTORY_OWN_BASE)

    lock = threading.Lock()
    made = [0]
    running = [_HISTORY_POSTERS]
    stopped = threading.Event()
    total = _HISTORY_POSTERS * _HISTORY_PER_POSTER
    recorded: List[List[_RecordedPost]] = [[] for _ in range(_HISTORY_POSTERS)]

    def post(poster: int) -> None:
        try:
            for k in range(_HISTORY_PER_POSTER):
                value = 2 + poster * _HISTORY_PER_POSTER + k
                if k % 5 != 4:
                    made_post = _record_post(clock, engine, _HISTORY_SESSION, value, _ping(str(value)))
                elif stopped.is_set():
                    made_post = _record_post(
                        clock, engine, _HISTORY_SESSION, value, _history_ping_naming(_SESSION, value)
                    )
                else:
                    made_post = _record_post(
                        clock, engine, _HISTORY_ABSENT, value, _history_ping_naming(_HISTORY_ABSENT_ID, value)
                    )
                recorded[poster].append(made_post)
                with lock:
                    made[0] += 1
                time.sleep(0)
        finally:
            with lock:
                running[0] -= 1

    threads = [
        threading.Thread(target=post, args=(poster,), name=f"inbox-history-poster-{poster}")
        for poster in range(_HISTORY_POSTERS)
    ]
    for thread in threads:
        thread.start()

    def any_running() -> bool:
        with lock:
            return running[0] > 0

    def half_made() -> bool:
        with lock:
            return made[0] * 2 >= total

    own = _HISTORY_OWN_BASE + 1
    round_ = 0
    while any_running():
        if not stopped.is_set():
            engine.advance_time(0)
            round_ += 1
            if round_ % 3 == 0 and own < _HISTORY_OWN_BASE + _HISTORY_OWN_PER_RUN:
                _raise_own(engine, own)
                own += 1
            if half_made():
                engine.stop()
                stopped.set()
        time.sleep(0)
    for thread in threads:
        thread.join(_BOUND_SECONDS)
        assert not thread.is_alive(), f"{thread.name} never finished posting"
    if not stopped.is_set():
        engine.advance_time(0)
        engine.stop()
        stopped.set()
    posters += recorded
    posters.append(
        _on_another_thread(
            lambda: [
                _record_post(clock, engine, _HISTORY_SESSION, _HISTORY_LATE_INPUT, _ping(str(_HISTORY_LATE_INPUT))),
                _record_post(
                    clock,
                    engine,
                    _HISTORY_SESSION,
                    _HISTORY_LATE_INPUT + 1,
                    _history_ping_naming(_SESSION, _HISTORY_LATE_INPUT + 1),
                ),
            ]
        )
    )
    return list(recorder.operations), posters


def _history_document(owner: List[Dict[str, Any]], posters: List[List[_RecordedPost]]) -> Dict[str, Any]:
    """A recorded run as an inbox history
    (tests/forge/conformance/inbox_history.schema.json): one session, whose
    inbox closes with its run and has no bound."""
    return {
        "version": 2,
        "sessions": [_HISTORY_SESSION],
        "closes": True,
        "owner": owner,
        "posters": [[made_post.document() for made_post in posts] for posts in posters],
    }


def _assert_run_keeps_the_doors_order(
    what: str, owner: List[Dict[str, Any]], posters: List[List[_RecordedPost]]
) -> None:
    """Hold a recorded run to what it was meant to exercise and to the order the
    door keeps, as far as one run's record says it: posts taken, refused as
    unknown and as ended, through both doors; own events entered and the run
    ended once; every input a post took entered once; and an event of the
    owner's own entered behind every input whose post returned before the
    event's enqueue began (docs/adr/0084, D2 as amended at step 9). Whether the
    whole record is linearizable is `sce-codegen check-inbox-history`'s to
    judge."""
    posts = [made_post for made in posters for made_post in made]
    assert any(made_post.taken for made_post in posts), f"{what}: no post was taken"
    assert any(made_post.reason == "unknown" for made_post in posts), f"{what}: no post was refused as unknown"
    assert any(made_post.reason == "ended" for made_post in posts), f"{what}: no post was refused as ended"
    assert any(made_post.addressed for made_post in posts) and any(
        not made_post.addressed for made_post in posts
    ), f"{what}: a door went unused"
    assert any(
        op["call"] == "enqueue" and op["source"] == "own" for op in owner
    ), f"{what}: the owner put no event of its own on the queue"
    assert sum(1 for op in owner if op["call"] == "end") == 1, f"{what}: the run did not end once"

    entered_at = {
        op["item"]: at for at, op in enumerate(owner) if op["call"] == "enqueue" and op["source"] == "posted"
    }
    taken = sorted(made_post.input for made_post in posts if made_post.taken)
    entered = sorted(op["item"] for op in owner if op["call"] == "enqueue" and op["source"] == "posted")
    assert entered == taken, f"{what}: the inputs that entered the queue are not the inputs the posts took"

    for at, op in enumerate(owner):
        if op["call"] != "enqueue" or op["source"] != "own":
            continue
        for made_post in posts:
            if not made_post.taken or made_post.returned >= op["invoked"]:
                continue
            assert entered_at[made_post.input] < at, (
                f"{what}: own event {op['item']} entered ahead of input {made_post.input}, whose post "
                "returned before the event's enqueue began"
            )


def test_each_run_of_the_inbox_is_recorded_as_a_history_that_keeps_the_doors_order() -> None:
    # Written where SCE_INBOX_HISTORY_DIR names, one file a run, for
    # `sce-codegen check-inbox-history` to judge; unset, the runs are held here
    # and written nowhere.
    named = os.environ.get("SCE_INBOX_HISTORY_DIR")
    directory = Path(named) if named else None
    if directory is not None:
        directory.mkdir(parents=True, exist_ok=True)
    for run in range(_HISTORY_RUNS):
        owner, posters = _record_inbox_run()
        _assert_run_keeps_the_doors_order(f"run {run}", owner, posters)
        if directory is not None:
            document = _history_document(owner, posters)
            (directory / f"python_inbox_run_{run}.json").write_text(
                json.dumps(document, separators=(",", ":")) + "\n"
            )
