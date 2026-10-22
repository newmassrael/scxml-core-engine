# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""Saving a ``datamodel="sce-static"`` machine and restoring it (SCE Accepted
Subset section 2.15, docs/adr/0045).

The steps every generated ``save_at`` and ``restore_at`` take alike; the generated
machine adds only its own variables, its histories and its running children
between them. The Python twin of ``sce.SaveSession``, ``sce.JudgeSession`` and
``sce.EnterSession`` (Go, ``engine_saved.go``), of the saved-state half of
``sce_rust_runtime::saved_state`` (Rust) and of Kotlin's ``StateMachineEngine``.

What a generated policy of a document that saves adds to ``StatePolicy`` is what
only the document knows about the sessions and invocations a saved state names:

``running_invokes()``
    the id the document gives each ``<invoke>`` whose child session is running,
    in document order (§scxml-6.4);
``invoke_starts()``
    how many times each ``<invoke>`` of the document has started in this session,
    for each started at least once, in document order;
``invoke_owner(id)``
    the state that holds the ``<invoke>`` ``id``, and ``None`` for an id the
    document does not invoke;
``host_invoke_owner(processor_type, id)``
    the state that holds the host-run ``<invoke>`` of this type and id, and
    ``None`` for one the document does not have a host invoker run (§scxml-6.4.1);
``judge_child(child)``
    refuses ``child``, a running child a format-2 state lists, unless its session
    is a state of the document the child names, to any depth (docs/adr/0045,
    section 3, step 1), building nothing;
``resume_children(engine, children, wall_now_ms)``
    resumes each of ``children`` on ``engine``, in the order given, each built as
    starting it builds it and restored where it stood in place of initialized: no
    ``<onentry>``, no ``<data>``, no ``<param>`` or namelist evaluated, no
    ``idlocation`` written, nothing sent;
``restart_invokes(engine, ids)``
    starts each of ``ids`` again from its beginning, as entering its state starts
    it, which is what a format-1 state asks for of a child it names by id alone.
"""

from __future__ import annotations

from typing import Any, Dict, Generic, List, Optional, Sequence, Tuple, TypeVar

from .configuration import (
    ConfigurationRejection,
    HistoryRejection,
    HistoryRejectionKind,
    validate_configuration,
    validate_history,
)
from .engine import (
    RESTORED_DEADLINE_AT,
    RESTORED_DEADLINE_PASSED,
    RESTORED_NO_DEADLINE,
)
from .event import EventMetadata, EventWithMetadata, event_data_text
from .host_processor import HostInvokeRequest, HostSendRequest
from .scheduler import ScheduledRoute
from .saved_state import (
    ACT_CHILD,
    ACT_HOST,
    ACT_INTERNAL,
    ACT_PARENT,
    ACT_RAISE,
    SAVED_FORMAT_2,
    SavedAct,
    SavedChild,
    SavedEvent,
    SavedHistory,
    SavedHostInvoke,
    SavedInvokes,
    SavedRevision,
    SavedSend,
    SavedShape,
    SavedState,
    StateRefusal,
    refusal_within,
)

__all__ = [
    "enter_session",
    "judge_session",
    "nothing_in_transit",
    "restore_history",
    "save_history",
    "save_session",
]

S = TypeVar("S")
E = TypeVar("E")

_INT64_MAX = (1 << 63) - 1

# What the engine's `_scheduler` entries a route names are called, as the
# generated send site spells them (`Engine.schedule_routed_send`).
_ROUTE_INTERNAL = "internal"
_ROUTE_INVOCATION = "invocation"
_ROUTE_PARENT = "parent"

_EVENT_TYPES = ("internal", "external", "platform")


def _saturating_add_ms(base_ms: int, add_ms: int) -> int:
    """`base_ms` and `add_ms` added, held to what a signed 64-bit count can say."""
    return min(base_ms + add_ms, _INT64_MAX)


def _due_on(wall_now_ms: int, now_ms: int, ready_at_ms: int) -> int:
    """The wall-clock moment, in milliseconds since the Unix epoch, a thing that
    comes due at `ready_at_ms` of the engine's clock comes due at, given that
    `wall_now_ms` is what time it is on the wall now and `now_ms` on the engine's
    clock: none earlier than now, for one already past."""
    return _saturating_add_ms(wall_now_ms, max(0, ready_at_ms - now_ms))


def _saves(policy: Any) -> bool:
    return hasattr(policy, "running_invokes")


def nothing_in_transit(parent_queue: Optional[Sequence[Any]], invoke_id: str) -> None:
    """Refuse a save of a child whose ``#_parent`` events still wait in `parent_queue`,
    neither in the child's past nor in the parent's queue. The generated parent
    drains the buffer after every drive of the child, so a tree saved at a boundary
    of its root holds nothing in transit, and this is never reached by generated
    code: a refusal is a defect of the generator, never a timing a host can meet,
    and leaving the event out would lose it."""
    if not parent_queue:
        return
    raise StateRefusal(
        f"{len(parent_queue)} event(s) the child '{invoke_id}' sent to its parent are in "
        "transit, which the generated parent drains after every drive of its child"
    )


def _clone_params(params: Dict[str, List[str]]) -> Dict[str, List[str]]:
    return {name: list(values) for name, values in params.items()}


# ── Save ─────────────────────────────────────────────────────────────────────


def save_session(
    engine: Any,
    shape: SavedShape,
    variables: Dict[str, Any],
    history: List[SavedHistory],
    children: List[SavedChild],
    wall_now_ms: int,
) -> SavedState:
    """`engine`'s saved state, a machine of the document whose saved state is bound
    to `shape`, with the `variables`, the `history` and the running `children` its
    generated ``save_at`` read. The configuration is written in document order, and
    every waiting delayed ``<send>`` is written as the moment it comes due on the
    wall clock whose reading now is `wall_now_ms`.

    A machine any of whose record schemas or enums is past its first revision names
    each such revision in ``revisions`` and writes the revised shape; any other
    writes the first shape and no ``revisions``. The state is format 2: the
    machine's own id, how many times each ``<invoke>`` has started, and children,
    each running child with its whole state, which the generated machine saved
    through the child's own document, in document order.

    Refused for a machine that is not running (never started, or ended at a
    top-level ``<final>``, where there is nothing left to resume), for one whose
    last macrostep stopped at the microstep ceiling (its configuration is not a
    settled one, and a restore would resume it as if it were), for one with no
    session id, and when `children` are not the children the machine is running."""
    policy = engine.policy
    if not _saves(policy):
        raise StateRefusal("this machine has no saved state, so a session it runs cannot be saved")
    if not engine.is_running:
        raise StateRefusal("the machine is not running: it was never started, or it has ended")
    if engine._macrostep_truncated:
        raise StateRefusal(
            "the machine's last macrostep stopped at the microstep ceiling, so it does not "
            "stand at a settled configuration"
        )
    session_id = engine._session_id
    if not session_id:
        raise StateRefusal("the machine has no session id, which a saved state names it by")
    # What the generated machine saved is every child it runs, and nothing else:
    # one left out would restore a parent whose child is gone.
    running = list(policy.running_invokes())
    saved_ids = [child.id for child in children]
    if running != saved_ids:
        raise StateRefusal(
            f"the machine runs the children {running!r} and the save holds {saved_ids!r}, "
            "a defect of its generated save"
        )
    # Read before anything is built, so a delayed send this state cannot carry
    # refuses the save instead of leaving it out.
    pending = _save_pending(engine, policy, wall_now_ms)
    # Document order, so the same machine saves the same text on every backend.
    active = sorted(engine.active_configuration(), key=policy.get_document_order)
    configuration = [policy.get_state_name(state) for state in active]
    # The revision of each record schema and each enum past its first, whose
    # digest list is empty exactly when it is in its first revision: the record
    # schemas first, then the enums, each in the order the generator gave them.
    revisions: List[SavedRevision] = []
    for record in shape.records:
        if record.revisions:
            revisions.append(SavedRevision(record.event, record.revision, record.revisions[-1]))
    for lineage in shape.enums:
        if lineage.revisions:
            revisions.append(SavedRevision(lineage.key, lineage.revision, lineage.revisions[-1]))
    state = SavedState(
        shape=shape.format2_revised if revisions else shape.format2_first,
        revisions=revisions,
        configuration=configuration,
        current=policy.get_state_name(engine.current_state),
        variables=variables,
        history=history,
        pending=pending,
        invokes=SavedInvokes(resumed=True, children=list(children)),
        session_id=session_id,
        invoke_starts=list(policy.invoke_starts()),
        host_invokes=_save_host_invokes(engine, wall_now_ms),
        host_invoke_token=engine._next_host_invoke_token,
        auto_send_seq=engine._auto_send_seq,
    )
    for queued in engine._external_queue:
        # §scxml-5.10: a queued event is saved under the name it ARRIVED under, so a
        # restore gives the machine the same `_event.name`.
        metadata = queued.metadata
        state.external.append(
            SavedEvent(
                name=metadata.name or policy.get_event_name(queued.event),
                data=event_data_text(metadata.data),
                type=metadata.event_type,
                send_id=metadata.send_id,
                origin=metadata.origin,
                origin_type=metadata.origin_type,
                invoke_id=metadata.invoke_id,
                host_invoke_token=metadata.host_invoke_token,
            )
        )
    return state


def _running_host_invokes(engine: Any) -> List[Tuple[HostInvokeRequest, Optional[int]]]:
    """The host-run invocations that are running, or re-armed and waiting to start,
    for a save: each with the request it was started with and the moment of the
    engine's clock its deadline comes due, if it has one. Ordered by ``(type, id)``,
    so every backend writes them in the same order."""
    pending = engine._scheduler.pending()
    running: List[Tuple[HostInvokeRequest, Optional[int]]] = []
    for started in engine._started_host_invokes.values():
        due_at: Optional[int] = None
        for entry in pending:
            deadline = entry.host_invoke_deadline
            if deadline is not None and deadline.token == started.token:
                due_at = entry.due_ms
                break
        running.append((started.request, due_at))
    for restored in engine._restored_host_invokes:
        due_at = None
        if restored.deadline == RESTORED_DEADLINE_AT:
            due_at = restored.at_ms
        elif restored.deadline == RESTORED_DEADLINE_PASSED:
            due_at = engine.now_ms
        running.append((restored.request, due_at))
    running.sort(key=lambda entry: (entry[0].processor_type, entry[0].invoke_id))
    return running


def _save_host_invokes(engine: Any, wall_now_ms: int) -> List[SavedHostInvoke]:
    """Every host-run invocation of `engine` that is running, or was re-armed by a
    restore and has yet to start, as a saved state holds it: the request it was
    started with and the wall-clock moment its deadline comes due."""
    now = engine.now_ms
    hosts: List[SavedHostInvoke] = []
    for request, due_at in _running_host_invokes(engine):
        hosts.append(
            SavedHostInvoke(
                processor_type=request.processor_type,
                invoke_id=request.invoke_id,
                src=request.src,
                params=_clone_params(request.params),
                data=request.event_data,
                content=request.content,
                due=None if due_at is None else _due_on(wall_now_ms, now, due_at),
            )
        )
    return hosts


def _save_pending(engine: Any, policy: Any, wall_now_ms: int) -> List[SavedSend]:
    """Every delayed send of `engine` still waiting, as a saved state holds it:
    written as the wall-clock moment it comes due, in the order the machine would
    deliver them.

    A send to the parent or to a child of this session is held as the act that
    delivers it to the session the restored tree holds there (docs/adr/0045). A send
    the format cannot carry, one routed to a session by its id, refuses the save. A
    document that makes one is generated without the save API, so no generated
    machine reaches this; the refusal is for a machine built by hand, where leaving
    the entry out would restore a machine that never delivers it and say nothing.

    The deadline of a running host-run invocation is not a send: it is saved with
    the invocation, as its ``due``, and left out here. One that belongs to no
    running invocation is refused like a send the format cannot carry.

    Python holds no ``origin`` for a waiting send: the event a delayed send makes
    names the session that sent it, as every backend's does, so what is saved is
    that session's id."""
    now = engine.now_ms
    session_id = engine._session_id
    sends: List[SavedSend] = []
    for entry in engine._scheduler.pending():
        due = _due_on(wall_now_ms, now, entry.due_ms)
        data = event_data_text(entry.data)
        if entry.host_invoke_deadline is not None:
            deadline = entry.host_invoke_deadline
            started = engine._started_host_invokes.get((deadline.processor_type, deadline.invoke_id))
            if started is not None and started.token == deadline.token:
                continue
            raise StateRefusal(
                f"the deadline of the invocation '{deadline.invoke_id}' belongs to no invocation "
                "this machine is running"
            )
        if entry.host_send is not None:
            request = entry.host_send
            act = SavedAct(
                kind=ACT_HOST,
                processor_type=request.processor_type,
                event=request.event_name,
                target=request.target,
                content=request.content,
                params=_clone_params(request.params),
                send_id=request.send_id,
                data=request.event_data,
                invoke_id=request.invoke_id,
            )
        elif entry.route is None:
            # §scxml-5.10: a send is saved under the name it was sent under, which is
            # its member's own unless a computed one cut back to a longer name.
            act = SavedAct(
                kind=ACT_RAISE,
                event=entry.name or policy.get_event_name(entry.event),
                data=data,
                send_id=entry.sendid,
                origin=session_id,
            )
        elif entry.route.kind == _ROUTE_INTERNAL:
            act = SavedAct(
                kind=ACT_INTERNAL,
                event=entry.route.event_name or policy.get_event_name(entry.event),
                data=data,
                send_id=entry.sendid,
                origin=session_id,
            )
        # docs/adr/0045: the parent and a child of this session are sessions the
        # saved tree holds.
        elif entry.route.kind == _ROUTE_PARENT:
            act = SavedAct(
                kind=ACT_PARENT,
                event=entry.route.event_name,
                data=data,
                send_id=entry.sendid,
                origin=session_id,
            )
        elif entry.route.kind == _ROUTE_INVOCATION:
            act = SavedAct(
                kind=ACT_CHILD,
                invoke_id=entry.route.invoke_id,
                event=entry.route.event_name,
                data=data,
                send_id=entry.sendid,
                origin=session_id,
            )
        else:
            raise StateRefusal(
                f"the delayed send '{entry.sendid}' is routed to the session '{entry.route.session_id}', "
                "a session a saved state does not carry"
            )
        sends.append(SavedSend(due=due, act=act))
    return sends


def save_history(policy: Any, engine: Any, declared: Sequence[str]) -> List[SavedHistory]:
    """What each of `declared`, the ids of the document's ``<history>`` elements, has
    recorded in `engine`, as a saved state holds it: state ids in document order (the
    order one machine writes is every backend's), keyed by the history's id and
    ordered by it. A history that has recorded nothing is left out."""
    saved: List[SavedHistory] = []
    for history_id in declared:
        recorded = engine._history.get(history_id)
        if recorded is None:
            continue
        states = sorted(recorded, key=policy.get_document_order)
        saved.append(SavedHistory(history_id, [policy.get_state_name(state) for state in states]))
    saved.sort(key=lambda entry: entry.id)
    return saved


def restore_history(
    policy: Any, saved: SavedState, declared: Sequence[str]
) -> List[Tuple[str, List[Any]]]:
    """What `saved` records for each of `declared`, read as the states of this
    document, or the refusal that says which value is not one.

    Every value is judged before any is applied, so a state with one bad history
    restores none of them. A history the document does not declare, one named
    twice, and a value its history could not have recorded (a state outside its
    parent, a compound state with two active children) are refused: a resumed
    machine entered through a history must land in a configuration its document
    could have been in."""
    restored: List[Tuple[str, List[Any]]] = []
    for at, recorded in enumerate(saved.history):
        if any(earlier.id == recorded.id for earlier in saved.history[:at]):
            raise StateRefusal(f"the saved state records the history '{recorded.id}' twice")
        if recorded.id not in declared:
            raise StateRefusal(
                f"the saved state records the history '{recorded.id}', which the document does not declare"
            )
        states: List[Any] = []
        for name in recorded.states:
            state = policy.get_state_from_name(name)
            if state is None:
                raise StateRefusal(
                    f"the history '{recorded.id}' records '{name}', which the document does not name"
                )
            states.append(state)
        verdict = validate_history(
            policy,
            policy.get_history_parent(recorded.id),
            policy.get_history_type(recorded.id) == "deep",
            states,
        )
        if verdict.kind is not HistoryRejectionKind.ACCEPTED:
            raise StateRefusal(f"the history '{recorded.id}' records {_describe_history(policy, verdict)}")
        restored.append((recorded.id, states))
    return restored


def _describe_history(policy: Any, verdict: HistoryRejection[Any]) -> str:
    """A history refusal in the document's own vocabulary."""

    def name(state: Any) -> str:
        return policy.get_state_name(state)

    kind = verdict.kind
    if kind is HistoryRejectionKind.EMPTY:
        return "no state, and a history records at least one"
    if kind is HistoryRejectionKind.DUPLICATE:
        return f"'{name(verdict.state)}' twice"
    if kind is HistoryRejectionKind.NOT_BELOW:
        return f"'{name(verdict.state)}', which is not below its parent '{name(verdict.parent)}'"
    if kind is HistoryRejectionKind.NOT_A_CHILD:
        return (
            f"'{name(verdict.state)}', which is not a child of '{name(verdict.parent)}' "
            "(a shallow history records the children)"
        )
    if kind is HistoryRejectionKind.NOT_ATOMIC:
        return f"'{name(verdict.state)}', which has children (a deep history records atomic states)"
    return f"states of no configuration: {verdict.arity}"


# ── Judge and enter ──────────────────────────────────────────────────────────


class _ReadSend:
    """A waiting send read against the document: the event it names, resolved, so a
    restore that cannot resolve one refuses before it arms any."""

    __slots__ = ("due", "act", "event")

    def __init__(self, due: int, act: SavedAct, event: Any) -> None:
        self.due = due
        self.act = act
        self.event = event


class _JudgedSession:
    """A saved session read against the document of a policy, before anything is
    built: its configuration and current leaf, its external queue, its waiting
    sends and, for a format-1 state, the ``<invoke>`` ids to start again."""

    __slots__ = ("configuration", "current", "external", "pending", "restarts")

    def __init__(self) -> None:
        self.configuration: List[Any] = []
        self.current: Any = None
        self.external: List[EventWithMetadata[Any]] = []
        self.pending: List[_ReadSend] = []
        self.restarts: List[str] = []


def judge_session(policy: Any, saved: SavedState) -> None:
    """Refuse `saved` unless it can be a state of the document of `policy`, building
    nothing (docs/adr/0045, section 3, step 1): its configuration, its queue, its
    waiting sends, its running and host-run invocations, its count of starts, and each
    running child's session against the document that child runs
    (``policy.judge_child``), to any depth.

    The shape, the revisions, the history and the variables are the generated
    machine's to judge before this (``upgrade_saved_state``, ``restore_history``); a
    generated ``judge`` does both, and is what a parent's judge of a tree calls for
    each child it holds."""
    _judge(policy, saved)


def _judge(policy: Any, saved: SavedState) -> _JudgedSession:
    judged = _JudgedSession()
    for name in saved.configuration:
        state = policy.get_state_from_name(name)
        if state is None:
            raise StateRefusal(f"the document has no state '{name}'")
        judged.configuration.append(state)
    current = policy.get_state_from_name(saved.current)
    if current is None:
        raise StateRefusal(f"the document has no state '{saved.current}'")
    judged.current = current
    verdict = validate_configuration(policy, judged.configuration, current)
    if verdict is not ConfigurationRejection.NONE:
        raise StateRefusal(f"the saved configuration is refused: {verdict}")
    # Every queued event is read before anything is entered, so one this document
    # cannot name refuses the restore rather than half of the queue.
    for at, event in enumerate(saved.external):
        # The name a queued event arrived under (§scxml-5.10): the document's own
        # event for it, or the one it extends (§scxml-3.12.1).
        resolved = policy.resolve_event_by_name(event.name)
        if resolved is None:
            raise StateRefusal(f"external[{at}] is '{event.name}', which the document does not name")
        if event.type not in _EVENT_TYPES:
            raise StateRefusal(f"external[{at}] has the type '{event.type}', which is not an event type")
        metadata = EventMetadata(
            data=event.data,
            event_type=event.type,
            send_id=event.send_id,
            origin=event.origin,
            origin_type=event.origin_type,
            invoke_id=event.invoke_id,
            host_invoke_token=event.host_invoke_token,
            name=event.name if policy.get_event_name(resolved) != event.name else "",
        )
        judged.external.append(EventWithMetadata(event=resolved, metadata=metadata))
    # The same for a waiting send: an event this document does not name refuses the
    # restore, not the sends before it.
    for at, send in enumerate(saved.pending):
        judged.pending.append(_read_pending_send(policy, send, at))
    # The same for a running invocation: one the document does not have, one whose
    # state the saved configuration does not stand in, or one named twice would be a
    # child the saved machine could not have had.
    ids = saved.invokes.invoke_ids()
    for at, invoke_id in enumerate(ids):
        owner = policy.invoke_owner(invoke_id)
        if owner is None:
            raise StateRefusal(f"invokes[{at}] is '{invoke_id}', which the document does not invoke")
        if owner not in judged.configuration:
            raise StateRefusal(
                f"invokes[{at}] is '{invoke_id}', whose state the saved configuration does not stand in"
            )
        if invoke_id in ids[:at]:
            raise StateRefusal(f"invokes[{at}] is '{invoke_id}', which an earlier entry already names")
    # A count of starts names an `<invoke>` of the document.
    for started in saved.invoke_starts:
        if policy.invoke_owner(started.id) is None:
            raise StateRefusal(f"invokestarts counts '{started.id}', which the document does not invoke")
    if saved.invokes.resumed:
        for at, child in enumerate(saved.invokes.children):
            _judge_child_entry(policy, saved, child, at)
    else:
        judged.restarts = list(saved.invokes.ids)
    # And for a host-run one, by the `(type, id)` the document declares: one it does
    # not have, one whose state the saved configuration does not stand in, or one
    # named twice is a run the saved machine could not have had.
    for at, host in enumerate(saved.host_invokes):
        owner = policy.host_invoke_owner(host.processor_type, host.invoke_id)
        if owner is None:
            raise StateRefusal(
                f"hostinvokes[{at}] is '{host.invoke_id}' of type '{host.processor_type}', which the "
                "document does not have a host invoker run"
            )
        if owner not in judged.configuration:
            raise StateRefusal(
                f"hostinvokes[{at}] is '{host.invoke_id}', whose state the saved configuration does "
                "not stand in"
            )
        for earlier in saved.host_invokes[:at]:
            if earlier.processor_type == host.processor_type and earlier.invoke_id == host.invoke_id:
                raise StateRefusal(
                    f"hostinvokes[{at}] is '{host.invoke_id}', which an earlier entry already names"
                )
    return judged


def _judge_child_entry(policy: Any, parent: SavedState, child: SavedChild, index: int) -> None:
    """Refuse `child`, the `index`-th running child of the format-2 session `parent`,
    unless it is the child that session could be running (docs/adr/0045, section 3,
    step 1): its start is the count the session holds for its ``<invoke>``, its
    session id is the session's and the id it was started under, and its own
    session is a state of the document it names (``policy.judge_child``)."""
    what = f"invokes[{index}]"
    count = parent.invoke_start(child.id)
    if child.start != count:
        raise StateRefusal(
            f"{what} is start {child.start} of '{child.id}', and the session counts {count} starts "
            "of it: a state re-entered cancels the earlier child before it starts the next one"
        )
    derived = f"{parent.session_id}.{child.invoke_id}"
    if child.session.session_id != derived:
        raise StateRefusal(
            f"{what}.session's id is '{child.session.session_id}', and a child of "
            f"'{parent.session_id}' started under '{child.invoke_id}' is '{derived}'"
        )
    try:
        policy.judge_child(child)
    except StateRefusal as err:
        raise refusal_within(err, what + ".session") from None


def _read_pending_send(policy: Any, send: SavedSend, index: int) -> _ReadSend:
    """`send`, the `index`-th of a saved state's ``pending``, with its event looked
    up in the document of `policy`."""
    read = _ReadSend(send.due, send.act, None)
    kind = send.act.kind
    if kind in (ACT_RAISE, ACT_INTERNAL):
        event = policy.resolve_event_by_name(send.act.event)
        if event is None:
            raise StateRefusal(f"pending[{index}] is '{send.act.event}', which the document does not name")
        read.event = event
    # The child is named by the `<invoke>` the target names, which must be one of
    # the document's: a send to any other is one the save would have been refused
    # for.
    elif kind == ACT_CHILD:
        if policy.invoke_owner(send.act.invoke_id) is None:
            raise StateRefusal(
                f"pending[{index}] is a send to the child '{send.act.invoke_id}', which the "
                "document does not invoke"
            )
    return read


def enter_session(engine: Any, saved: SavedState, wall_now_ms: int) -> None:
    """Stand `engine`, a machine of the policy it holds whose variables already hold
    what `saved` holds, where `saved` names: the machine a host drives in place of
    one it would ``initialize``.

    No ``<onentry>`` runs and no ``<data>`` is evaluated (``Engine.enter_at``): the
    run being resumed already did both, and the host calls it caused cannot be made
    twice. A configuration that is not one of this document is refused, and nothing
    is entered.

    The engine was built under the id `saved` names, a format-2 state's own, which
    every child's id and a peer's address name (docs/adr/0045). `wall_now_ms` is what
    time it is on the wall clock the saved ``due``s were written against: a waiting
    send is armed to come due ``due - wall_now_ms`` after now, and one already due is
    armed to come due now, behind the ones due before it.

    A format-1 state's running children are started again from their beginning, last,
    so what a child sends as it starts stands behind what was already waiting. A
    format-2 state's are judged with the rest before anything is built, and resumed,
    each where it stood, once this session stands where it did
    (``policy.resume_children``)."""
    policy = engine.policy
    judged = _judge(policy, saved)
    verdict = engine.enter_at(judged.configuration, judged.current)
    if verdict is not ConfigurationRejection.NONE:
        raise StateRefusal(f"the saved configuration is refused: {verdict}")
    for queued in judged.external:
        engine._external_queue.append(queued)
    _arm_pending(engine, judged.pending, wall_now_ms)
    _rearm_host_invokes(engine, saved.host_invokes, wall_now_ms)
    engine._next_host_invoke_token = saved.host_invoke_token
    engine._auto_send_seq = saved.auto_send_seq
    if saved.invokes.resumed:
        policy.resume_children(engine, saved.invokes.children, wall_now_ms)
    else:
        # Last, so what a child sends as it starts stands behind what was already
        # waiting: the saved machine's queue was ahead of it.
        policy.restart_invokes(engine, judged.restarts)


def _rearm_host_invokes(engine: Any, hosts: Sequence[SavedHostInvoke], wall_now_ms: int) -> None:
    """Re-arm each of `hosts` on `engine`, in the order a saved state lists them.
    None starts here: the host registers its invokers on the engine this restore
    returns, so each starts at the top of the first macrostep it drives
    (``Engine._start_restored_host_invokes``). A deadline comes due ``due -
    wall_now_ms`` after now on the engine's own clock; one already due ends the
    invocation without a start."""
    now = engine.now_ms
    for host in hosts:
        deadline, at_ms = RESTORED_NO_DEADLINE, 0
        if host.due is not None:
            if host.due <= wall_now_ms:
                deadline = RESTORED_DEADLINE_PASSED
            else:
                deadline, at_ms = RESTORED_DEADLINE_AT, _saturating_add_ms(now, host.due - wall_now_ms)
        engine.rearm_host_invoke(
            HostInvokeRequest(
                processor_type=host.processor_type,
                invoke_id=host.invoke_id,
                src=host.src,
                params=_clone_params(host.params),
                event_data=host.data,
                content=host.content,
            ),
            deadline,
            at_ms,
        )


def _arm_pending(engine: Any, pending: Sequence[_ReadSend], wall_now_ms: int) -> None:
    """Arm each of `pending`, already read, in the order a saved state lists them:
    each comes due ``due - wall_now_ms`` after the engine's now, and one that came
    due while the machine was away comes due now. Entries due at the same moment are
    armed in order and the scheduler delivers those in the order they were armed,
    which is how the saved machine would have."""
    now = engine.now_ms

    def ready_at(due: int) -> int:
        return _saturating_add_ms(now, max(0, due - wall_now_ms))

    scheduler = engine._scheduler
    for read in pending:
        act = read.act
        if act.kind == ACT_RAISE:
            scheduler.schedule(
                ready_at(read.due),
                act.send_id,
                read.event,
                act.data,
                name=engine.arrival_name_of(read.event, act.event),
            )
        elif act.kind == ACT_INTERNAL:
            scheduler.schedule(
                ready_at(read.due),
                act.send_id,
                read.event,
                act.data,
                route=_route(_ROUTE_INTERNAL, event_name=act.event),
            )
        elif act.kind == ACT_HOST:
            scheduler.schedule(
                ready_at(read.due),
                act.send_id,
                None,
                "",
                host_send=HostSendRequest(
                    processor_type=act.processor_type,
                    event_name=act.event,
                    target=act.target,
                    content=act.content,
                    params=_clone_params(act.params),
                    send_id=act.send_id,
                    event_data=act.data,
                    invoke_id=act.invoke_id,
                ),
            )
        elif act.kind == ACT_PARENT:
            scheduler.schedule(
                ready_at(read.due),
                act.send_id,
                None,
                act.data,
                route=_route(_ROUTE_PARENT, event_name=act.event),
            )
        elif act.kind == ACT_CHILD:
            scheduler.schedule(
                ready_at(read.due),
                act.send_id,
                None,
                act.data,
                route=_route(_ROUTE_INVOCATION, event_name=act.event, invoke_id=act.invoke_id),
            )


def _route(kind: str, event_name: str = "", invoke_id: str = "") -> ScheduledRoute:
    return ScheduledRoute(kind=kind, event_name=event_name, invoke_id=invoke_id)
