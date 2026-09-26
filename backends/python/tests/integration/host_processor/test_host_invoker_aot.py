# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.4.1 — Python compile+run gate for an ``<invoke type>`` the HOST runs.

The clause leaves the invokable set to the platform in the same words 6.2.5
uses for ``<send>``, so the set is open by design. SCE implemented the SCXML
processor and refused everything else with ``error.execution``. The send half of
that gap was repaid across six backends; this one stayed Rust-only, and the
generator refused ``--host-invoker`` for Python by name rather than emit a start
nothing could service.

The refusal was honest, which is what made it a coverage debt rather than a
silent drop. Now the Python runtime carries the registry
(``Engine.register_invoker``) and this file is the channel that says so.

An invoke is not a send: it has a LIFETIME. The scenarios below hold the
outcomes apart, because the configuration alone cannot:

* a registered invoker is STARTED with what the document wrote;
* leaving the state CANCELS it — the half no configuration assertion can see,
  because the machine looks correct whether or not the host was told to stop;
* a cancel is delivered once, and only for an invocation that started;
* a declared type with nothing registered raises ``error.execution``.

Fixture: ``sce-build/tests/fixtures/host_processor/statechart_host_invoker.scxml``,
the same document the Rust, C++ and Go channels drive, generated WITH
``--host-invoker x-sce-host``. The declaration is load-bearing: without it
codegen emits the refusal and every case below would measure the refusal
instead of the feature.

Regeneration (after fixture or template edit):
  ``scripts/regen_host_processor_python.sh``
"""
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import List, Optional

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

from sce_runtime import (  # noqa: E402
    HOST_INVOKE_DEADLINE_PARAM,
    HostInvokeEvent,
    HostInvokeResponse,
    parse_host_invoke_deadline_ms,
)

import statechart_host_invoker_sm as _sm  # noqa: E402 — path inserted above

# The type the fixture was compiled for.
# ``scripts/regen_host_processor_python.sh`` passes this same string to
# ``--host-invoker``; a test registering a different one would measure nothing
# and pass, so the ``refused`` counter is asserted rather than the registration
# trusted.
DECLARED_TYPE = "x-sce-host"

Event = _sm.StatechartHostInvokerEvent


def _counter(engine, name: str) -> int:
    """The fixture's ``<assign>``s are the only witness: several of these
    outcomes leave the machine in the same state, so the configuration cannot
    tell them apart."""
    value = getattr(engine.policy, name)()
    assert value is not None, f"the fixture declares `{name}` and the machine could not read it"
    return value


def _recording_invoker(log: List[str]):
    """A recording invoker. Answers a completion on start so the
    ``done.invoke`` path is exercised too, and records both arms so the ORDER
    is assertable."""

    def handler(ev: HostInvokeEvent) -> Optional[HostInvokeResponse]:
        if ev.start is not None:
            within = ev.start.params.get("within", ["absent"])[0]
            log.append(
                f"START id={ev.start.invoke_id} type={ev.start.processor_type} "
                f"src={ev.start.src} within={within}"
            )
            return HostInvokeResponse(done_data="ok")
        if ev.cancel is not None:
            log.append(f"CANCEL id={ev.cancel.invoke_id}")
        return None

    return handler


def _running_invoker(log: List[str], starts: List[tuple]):
    """An invoker whose work outlives the call: it answers nothing on start,
    so each invocation stays running until the test completes or cancels it.
    Records the lines `_recording_invoker` does, and every start's token for
    a later ``Engine.complete_host_invoke``."""

    def handler(ev: HostInvokeEvent) -> Optional[HostInvokeResponse]:
        if ev.start is not None:
            log.append(f"START id={ev.start.invoke_id}")
            starts.append((ev.start.invoke_id, ev.start.token))
        if ev.cancel is not None:
            log.append(f"CANCEL id={ev.cancel.invoke_id}")
        return None

    return handler


def _token_of(starts: List[tuple], invoke_id: str) -> int:
    """The token of the latest start of `invoke_id`."""
    for started_id, token in reversed(starts):
        if started_id == invoke_id:
            return token
    raise AssertionError(f"`{invoke_id}` never started")


def _running():
    """A machine whose invocations stay running, already initialized."""
    engine = _sm.create_engine()
    log: List[str] = []
    starts: List[tuple] = []
    engine.register_invoker(DECLARED_TYPE, _running_invoker(log, starts))
    engine.initialize()
    return engine, log, starts


def _started(with_invoker: bool = True):
    """A machine with the invoker each case decides on, already initialized.

    Registration happens BEFORE ``initialize()``: the fixture's invoke runs at
    the end of the entry macrostep, so an invoker registered afterwards would
    be measuring a run that had already refused."""
    engine = _sm.create_engine()
    log: List[str] = []
    if with_invoker:
        engine.register_invoker(DECLARED_TYPE, _recording_invoker(log))
    engine.initialize()
    return engine, log


def test_a_registered_invoker_is_started_with_what_the_document_wrote() -> None:
    engine, log = _started()

    # The fixture counts a completion only when its `_event.invokeid` names the
    # invocation (W3C SCXML 5.10.1), so each counter is that assertion too.
    assert _counter(engine, "started") == 1, (
        "done.invoke.probe never reached the document, or arrived without its invokeid"
    )
    assert _counter(engine, "started2") == 1, (
        "done.invoke.probe2 never reached the document, or arrived without its invokeid"
    )
    assert _counter(engine, "refused") == 0, "a started invocation also raised error.execution"
    # The false-positive guard: ordinary entry content must still run. Without
    # it a change that broke the entry chain while leaving the invoke arm
    # working would read as a pass.
    assert _counter(engine, "entered") == 1, "the entry chain stopped running"

    assert len(log) == 2, f"invoker calls: {log}"
    # `src` and `<param>` are how W3C SCXML 6.4.1 lets the document say WHAT to
    # invoke and with what. A request carrying neither would let a document
    # name an invocation it cannot describe. Each invocation is started as
    # itself: `probe2` begins with `probe`.
    assert log[0] == (
        f"START id=probe type={DECLARED_TYPE} src=pane://turn within=2500"
    ), f"the start request lost part of what the document wrote: {log[0]}"
    assert log[1] == (
        f"START id=probe2 type={DECLARED_TYPE} src=pane://other within=absent"
    ), f"the second invocation was not started as itself: {log[1]}"


def test_what_the_request_says_is_evaluated_when_the_invocation_starts() -> None:
    """W3C SCXML 6.4.1: ``srcexpr``, ``namelist``, ``<param expr>`` and
    ``<content expr>`` are read from the data model when the invocation
    starts. The request used to carry the literal params alone, so a document
    that computed what to invoke handed the host an empty description."""
    engine = _sm.create_engine()
    starts = []

    def handler(ev: HostInvokeEvent) -> Optional[HostInvokeResponse]:
        if ev.start is not None:
            starts.append(ev.start)
        return None

    engine.register_invoker(DECLARED_TYPE, handler)
    engine.initialize()
    starts.clear()  # `probe` / `probe2`, which the case above already reads
    engine.send_external(Event.EVALUATE)
    engine.advance_time(0)

    by_id = {s.invoke_id: s for s in starts}
    # `req3`'s srcexpr cannot be evaluated, so it is never started.
    assert set(by_id) == {"req", "req2"}, f"started: {sorted(by_id)}"
    req = by_id["req"]
    assert req.src == "pane://dyn", f"srcexpr was not evaluated: {req.src!r}"
    # A repeated name keeps both values in document order; the `<param>` that
    # failed is absent (W3C SCXML 5.7.1) while the invocation still started.
    assert req.params == {"n": ["7"], "twice": ["a", "8"]}, f"params: {req.params}"
    assert by_id["req2"].content == "body:7", f"content: {by_id['req2'].content!r}"
    # One error.execution for the dropped `<param>`, one for `req3`.
    assert _counter(engine, "dropped") == 2, "a failed evaluation was not reported"


def test_leaving_the_state_cancels_the_invocation() -> None:
    """The invocation ends with the state that started it. Without this the
    host is told to begin work and never told to stop — which no configuration
    assertion can detect, because the machine looks correct either way.

    Still running when the state exits — a completed invocation has nothing
    left to cancel (the case after this one)."""
    engine, log, _starts = _running()
    # `send_external` + a macrostep is this backend's delivery pair; there is
    # no single-call `process_event` here, and the sibling channels drive their
    # machines the same way.
    engine.send_external(Event.LEAVE)
    engine.advance_time(0)

    assert _counter(engine, "ended") == 1, "the machine never left the invoking state"
    # Both invocations end with the state, each told once.
    for invoke_id in ("probe", "probe2"):
        cancels = log.count(f"CANCEL id={invoke_id}")
        assert cancels == 1, f"cancel for {invoke_id} reached the invoker {cancels} times: {log}"


def test_cancel_is_not_delivered_for_an_invocation_that_never_started() -> None:
    """A cancel is delivered once, and only for an invocation that started.

    The engine, not the emitted code, owns that judgement: the exit chain calls
    ``cancel_host_invoke`` unconditionally, so if the engine did not track what
    started, a state that exits before its macrostep settles would have the
    host tearing down work it never began.

    Asserted at the engine surface rather than through the fixture, for the
    reason the Rust channel records: driving the machine cannot produce the
    "never started" case, because every host call that advances it runs a
    macrostep and the pending invoke executes at the end of that macrostep."""
    engine = _sm.create_engine()
    log: List[str] = []
    starts: List[tuple] = []
    engine.register_invoker(DECLARED_TYPE, _running_invoker(log, starts))

    assert not engine.cancel_host_invoke(
        DECLARED_TYPE, "probe"
    ), "a cancel was reported for an invocation that never started"
    assert log == [], f"the invoker was called for an invocation that never started: {log}"

    # Now let one start, cancel it, and cancel again: the second call has
    # nothing left to do. A registry that answered twice would have the host
    # tear down the same work twice.
    engine.initialize()
    assert engine.cancel_host_invoke(
        DECLARED_TYPE, "probe"
    ), "a started invocation reported nothing to cancel"
    assert not engine.cancel_host_invoke(
        DECLARED_TYPE, "probe"
    ), "the same invocation was cancelled twice"
    cancels = [e for e in log if e.startswith("CANCEL")]
    assert len(cancels) == 1, f"cancel reached the invoker {len(cancels)} times: {log}"


def _deliver(engine, event) -> None:
    """This backend's delivery pair: enqueue, then run a macrostep."""
    engine.send_external(event)
    engine.advance_time(0)


def test_a_completed_invocation_is_not_cancelled() -> None:
    """§scxml-6.4: ``done.invoke`` says the invoked process is over, so
    leaving the state afterwards has nothing to stop. Both invocations here
    complete synchronously; neither is cancelled."""
    engine, log = _started()
    _deliver(engine, Event.LEAVE)

    assert _counter(engine, "started") == 1
    assert not [e for e in log if e.startswith("CANCEL")], (
        f"a completed invocation was cancelled: {log}"
    )


def test_a_late_completion_is_accepted_exactly_once() -> None:
    """A host that finishes later reports it with the start's token, and the
    completion is taken once: a second report of the same run finds nothing,
    and the state's exit then cancels only the invocation still running."""
    engine, log, starts = _running()
    assert _counter(engine, "started") == 0

    token = _token_of(starts, "probe")
    assert engine.complete_host_invoke(DECLARED_TYPE, "probe", token, "ok"), (
        "a running invocation's completion was refused"
    )
    engine.advance_time(0)
    # The fixture counts it only when `_event.invokeid` names the invocation
    # (§scxml-5.10.1), so this is that assertion too.
    assert _counter(engine, "started") == 1

    assert not engine.complete_host_invoke(DECLARED_TYPE, "probe", token, "again"), (
        "the same run completed twice"
    )
    engine.advance_time(0)
    assert _counter(engine, "started") == 1

    _deliver(engine, Event.LEAVE)
    assert [e for e in log if e.startswith("CANCEL")] == ["CANCEL id=probe2"], (
        f"only the invocation still running is cancelled: {log}"
    )


def test_a_completion_after_the_cancel_is_refused() -> None:
    """§scxml-6.4: once the state has exited, what the cancelled process
    sends is ignored. The host's reply arrives after the cancel and is
    refused."""
    engine, _log, starts = _running()
    token = _token_of(starts, "probe")
    _deliver(engine, Event.LEAVE)

    assert not engine.complete_host_invoke(DECLARED_TYPE, "probe", token, "late"), (
        "a cancelled run's completion was accepted"
    )
    engine.advance_time(0)
    assert _counter(engine, "started") == 0


def test_a_restarted_invoke_refuses_the_first_runs_reply() -> None:
    """Re-entering the state starts the same ``<invoke>`` again under the
    same id. The first run's late reply carries the first start's token and
    is refused; the second run's is accepted."""
    engine, _log, starts = _running()
    first = _token_of(starts, "probe")
    _deliver(engine, Event.LEAVE)
    _deliver(engine, Event.AGAIN)
    second = _token_of(starts, "probe")
    assert first != second, "a restart reused the first start's token"

    assert not engine.complete_host_invoke(DECLARED_TYPE, "probe", first, "stale"), (
        "the first run's reply was taken for the second run's"
    )
    engine.advance_time(0)
    assert _counter(engine, "started") == 0
    assert engine.complete_host_invoke(DECLARED_TYPE, "probe", second, "ok")
    engine.advance_time(0)
    assert _counter(engine, "started") == 1


def test_a_done_invoke_raised_the_old_way_is_refused_and_counted() -> None:
    """A host-run invocation's ``done.invoke`` raised through the ordinary
    external-event API skipped the running check, so the engine refuses it
    and counts the refusal. The metadata names the invocation, so without the
    refusal the fixture's guarded transition would take it."""
    engine, _log, _starts = _running()

    engine.send_external_by_name("done.invoke.probe", data="x", invoke_id="probe")
    engine.advance_time(0)

    assert _counter(engine, "started") == 0, (
        "a completion that skipped the running check reached the document"
    )
    assert engine.refused_host_invoke_completions() == 1


def test_an_idlocation_holds_the_id_the_host_is_handed() -> None:
    """W3C SCXML 6.4.1: an invoke with no id and an ``idlocation`` gets a
    generated ``stateid.platformid`` id, written to the location, handed to
    the host, and carried as ``_event.invokeid`` on the completion. The
    document names no specific ``done.invoke.<id>``, so the completion
    arrives as the generic ``done.invoke``, and ``matched`` counts it only
    when its invokeid is what the document stored."""
    engine, log = _started()
    _deliver(engine, Event.LEAVE)

    assert any(e.startswith("START id=done._invoke_0 ") for e in log), (
        f"the host was not handed the generated id: {log}"
    )
    assert _counter(engine, "matched") == 1, (
        "the completion did not arrive, or its invokeid is not what idlocation holds"
    )


def test_an_idlocation_is_assigned_like_a_location() -> None:
    """W3C SCXML 6.2.4 / 6.4.1: an ``idlocation`` is a location expression, so
    the id is written the way ``<assign>`` writes (5.4): ``slot.id`` and
    ``slot.sid`` are member paths, and land. ``n.nope.deeper`` cannot take a
    value, so each element raises error.execution and is abandoned (5.9.2) —
    the host is never asked to start that invoke, and that message is never
    sent."""
    engine, log = _started()
    _deliver(engine, Event.LOCATE)

    assert any(e.startswith("START id=locating._invoke_1 ") for e in log), (
        f"the member-path invoke was not started: {log}"
    )
    assert not any("locating._invoke_2" in e for e in log), (
        f"an invoke whose idlocation could not take the id was started: {log}"
    )

    raw = engine.policy.slot()
    assert raw is not None, "the fixture declares `slot` as an object"
    slot = json.loads(raw)
    assert slot["id"] == "locating._invoke_1", f"slot.id: {slot}"
    assert isinstance(slot["sid"], str) and slot["sid"], (
        f"slot.sid did not receive the send id: {slot}"
    )

    assert _counter(engine, "slotted") == 1
    assert _counter(engine, "pinged") == 1
    assert _counter(engine, "leaked") == 0, (
        "a send whose idlocation could not take the id was still sent"
    )
    assert _counter(engine, "lost") == 2


def test_a_generic_done_invoke_raised_the_old_way_is_refused() -> None:
    """The generic ``done.invoke`` is a host completion too when its
    invokeid names a host-run invoke, so raised around
    ``complete_host_invoke`` it is refused like the specific name is."""
    engine, _log, _starts = _running()
    _deliver(engine, Event.LEAVE)

    engine.send_external_by_name("done.invoke", data="x", invoke_id="done._invoke_0")
    engine.advance_time(0)

    assert _counter(engine, "matched") == 0, (
        "a completion that skipped the running check reached the document"
    )
    assert engine.refused_host_invoke_completions() == 1


def test_a_declared_type_with_no_invoker_still_raises_error_execution() -> None:
    """The other half. The build declared the type, so codegen emitted a start
    — but nothing was registered, so no process was run. Same event as an
    unsupported type, because from the document's side it is the same fact.

    This is the case that keeps the repair honest: without it the feature could
    start nothing and the document would proceed as though its process were
    running."""
    engine, _log = _started(with_invoker=False)

    # One error.execution per invocation nobody ran.
    assert _counter(engine, "refused") == 2, "an unregistered invoker was silently treated as started"
    assert _counter(engine, "started") == 0, "done.invoke arrived for an invocation nobody ran"


def _timed():
    """A machine whose invoker answers nothing and records, per start, whether
    the request still carried the deadline param, driven into ``timed``. This
    backend's clock is virtual and starts at 0, so ``advance_time`` alone
    decides when a deadline comes due."""
    engine = _sm.create_engine()
    log: List[str] = []
    starts: List[tuple] = []

    def handler(ev: HostInvokeEvent) -> Optional[HostInvokeResponse]:
        if ev.start is not None:
            carried = HOST_INVOKE_DEADLINE_PARAM in ev.start.params
            log.append(f"START id={ev.start.invoke_id} deadline-param={carried}")
            starts.append((ev.start.invoke_id, ev.start.token))
        if ev.cancel is not None:
            log.append(f"CANCEL id={ev.cancel.invoke_id}")
        return None

    engine.register_invoker(DECLARED_TYPE, handler)
    engine.initialize()
    _deliver(engine, Event.TIME)
    return engine, log, starts


def test_a_deadline_that_passes_ends_the_invocation() -> None:
    """A deadline that passes while the invocation is still running ends it:
    the host is told to stop, the document receives ``error.invoke.slow`` with
    ``_event.data`` "deadline", and a reply afterwards is refused. The param is
    the engine's — the host never sees it."""
    engine, log, starts = _timed()
    assert "START id=slow deadline-param=False" in log, (
        f"the host was handed the deadline param, or `slow` never started: {log}"
    )

    # A `<cancel>` of the empty send id must not reach the deadline.
    _deliver(engine, Event.FORGET)
    engine.advance_time(49)
    assert _counter(engine, "expired") == 0, "expired early"
    engine.advance_time(1)
    assert _counter(engine, "expired") == 1
    assert "CANCEL id=slow" in log, f"the host was not told to stop: {log}"

    assert not engine.complete_host_invoke(
        DECLARED_TYPE, "slow", _token_of(starts, "slow"), "late"
    ), "a reply after the deadline was accepted"
    engine.advance_time(0)
    assert _counter(engine, "finished") == 0


def test_a_completion_before_the_deadline_disarms_it() -> None:
    """The discriminator: a completion before the deadline is the outcome, and
    the deadline that comes due afterwards does nothing — no cancel, no
    ``error.invoke``, and nothing left for the host to advance time toward."""
    engine, log, starts = _timed()
    assert engine.complete_host_invoke(
        DECLARED_TYPE, "slow", _token_of(starts, "slow"), "ok"
    )
    engine.advance_time(0)
    assert _counter(engine, "finished") == 1
    assert engine.time_until_next_scheduled_ms() is None, (
        "the disarmed deadline is still keeping the host advancing time"
    )

    engine.advance_time(100)
    assert _counter(engine, "expired") == 0
    assert "CANCEL id=slow" not in log, (
        f"a completed invocation was cancelled by its deadline: {log}"
    )


def test_a_deadline_that_is_not_milliseconds_starts_nothing() -> None:
    """§scxml-6.4.1: a deadline that is not a whole number of milliseconds is
    an argument that cannot be evaluated — error.execution, and the host is
    never asked to start the invocation."""
    engine, log, _starts = _timed()
    assert _counter(engine, "misdated") == 1
    assert not [e for e in log if e.startswith("START id=undated")], (
        f"an invocation with an unreadable deadline was started: {log}"
    )


def test_a_deadline_is_read_by_the_shared_table() -> None:
    """Every runtime reads a deadline's text by one grammar, held to one table.
    ``int``/``float`` would not do: they read ``1_000``, surrounding whitespace
    and non-ASCII digits, and a deadline this backend honours while another
    refuses it makes a document depend on where it was compiled."""
    table_path = (
        _HERE.parents[4]
        / "sce-build/tests/fixtures/host_processor/host_invoke_deadline_values.json"
    )
    table = json.loads(table_path.read_text(encoding="utf-8"))
    # A floor: an empty table would pass every assertion below.
    assert table["accepted"] and table["refused"], "the table is empty"
    for written, ms in table["accepted"]:
        assert parse_host_invoke_deadline_ms(written) == int(ms), repr(written)
    for written in table["refused"]:
        assert parse_host_invoke_deadline_ms(written) is None, (
            f"{written!r} was accepted"
        )


def test_an_invoker_registered_for_another_type_does_not_run_this_one() -> None:
    """Registering some other type does not run this one. The registry is
    keyed, and a lookup that fell back to "any invoker" would hand a document's
    process to one it never named."""
    engine = _sm.create_engine()
    log: List[str] = []
    engine.register_invoker("x-some-other-host", _recording_invoker(log))
    engine.initialize()

    assert _counter(engine, "started") == 0, "an invoker for a different type ran this one"
    assert _counter(engine, "refused") == 2, "the unregistered type was not reported"
    assert log == [], f"the other type's invoker was called: {log}"


def _typed_completion(done_data: str) -> tuple:
    """`perm` completed with `done_data` in a machine driven into `typed`:
    the counters `granted`, `denied` and `unreadable`."""
    engine, _log, starts = _running()
    _deliver(engine, Event.TYPE)
    assert engine.complete_host_invoke(
        DECLARED_TYPE, "perm", _token_of(starts, "perm"), done_data
    ), "a running invocation's completion was refused"
    engine.advance_time(0)
    return (
        _counter(engine, "granted"),
        _counter(engine, "denied"),
        _counter(engine, "unreadable"),
    )


def test_a_typed_completion_is_read_as_its_record() -> None:
    """SCE Accepted Subset §2.12: ``sce:result`` makes ``perm``'s completion
    a ``PermResult`` record, so its guards read ``granted`` as a typed field —
    true and false each select their own transition — and a completion whose
    data is not that record is refused as any typed payload the data does not
    fit is: error.execution, and neither guard fires."""
    assert _typed_completion('{"granted":true}') == (1, 0, 0), "granted"
    assert _typed_completion('{"granted":false}') == (0, 1, 0), "denied"
    assert _typed_completion("yes") == (0, 0, 1), "not the record"


class _PermHost:
    """A host implementing the generated interface; its work outlives the
    call."""

    def __init__(self) -> None:
        self.starts: List[tuple] = []
        self.cancels: List[int] = []

    def start_perm(self, request, token: int):
        self.starts.append((request, token))
        return None

    def cancel_perm(self, token: int) -> None:
        self.cancels.append(token)


def _typed_host():
    """A machine driven into `typed` with a `_PermHost` registered through
    the generated adapter, and the untyped invokes of the same type served by
    `_running_invoker`."""
    engine = _sm.create_engine()
    host = _PermHost()
    log: List[str] = []
    _sm.register_x_sce_host_invoker(engine, host, _running_invoker(log, []))
    engine.initialize()
    _deliver(engine, Event.TYPE)
    return engine, host, log


def test_a_typed_request_reaches_its_invoker_as_its_record() -> None:
    """SCE Accepted Subset §2.12: through the generated interface a host is
    handed ``perm``'s request as its ``PermRequest`` record — the datamodel's
    values at their declared types — and completes it with a ``PermResult``,
    which the document reads as that record. An invoke of the same type the
    document does not type still reaches the host, through the fallback."""
    engine, host, log = _typed_host()
    assert len(host.starts) == 1, f"perm started {len(host.starts)} times"
    request, token = host.starts[0]
    assert request == _sm.PermRequest(scope="calendar", level=2)
    assert "START id=probe" in log, f"the untyped `probe` never reached the fallback: {log}"
    assert _sm.complete_perm(engine, token, _sm.PermResult(granted=True))
    engine.advance_time(0)
    assert _counter(engine, "granted") == 1
    assert _counter(engine, "unreadable") == 0
    # A completion is accepted once: the token now names nothing running.
    assert not _sm.complete_perm(engine, token, _sm.PermResult(granted=True))


def test_a_request_that_does_not_fit_its_record_starts_nothing() -> None:
    """SCE Accepted Subset §2.12, W3C SCXML 6.4.1: a request value its
    record's field cannot hold is an argument that cannot be evaluated.
    ``retype`` sets ``level`` to a text and re-enters ``typed``: the running
    start is cancelled, and the new one raises error.execution and is never
    handed to the host."""
    engine, host, _log = _typed_host()
    first = host.starts[0][1]
    _deliver(engine, Event.RETYPE)
    assert host.cancels == [first], "the running start was cancelled"
    assert len(host.starts) == 1, f"the misfit request was started: {host.starts}"
    assert _counter(engine, "unreadable") == 1


def test_a_request_field_is_checked_and_read_back_by_one_rule() -> None:
    """The start site's check and the adapter's reading are one rule: every
    value the check accepts is spelled as text its field's type parses back to
    the same value, and a value the field cannot hold is refused rather than
    narrowed."""
    import struct

    from sce_runtime import (
        HostInvokeRequest,
        RequestFieldType,
        ScriptValue,
        request_field,
        request_field_wire,
    )
    from sce_runtime.event_payload import TypedPayloadError

    def round_trip(value, kind: str, cap: int = 0):
        field_type = RequestFieldType(kind, cap)
        text = request_field_wire(ScriptValue.of(value), "f", field_type)
        return request_field(
            HostInvokeRequest(invoke_id="perm", params={"f": [text]}), "f", field_type
        )

    assert round_trip(255, "uint8") == 255
    assert round_trip(-3.0, "int16") == -3
    assert round_trip(0.1, "float64") == 0.1
    f32 = struct.unpack("<f", struct.pack("<f", 0.1))[0]
    assert round_trip(0.1, "float32") == f32
    assert round_trip(False, "bool") is False
    assert round_trip("a b", "string") == "a b"
    assert round_trip("ab", "bytes", 2) == b"ab"

    for value, kind, cap, why in [
        (256, "uint8", 0, "past the width"),
        (-1, "uint32", 0, "below zero"),
        (1.5, "int32", 0, "not whole"),
        (float("nan"), "float64", 0, "not finite"),
        (1e39, "float32", 0, "past float32"),
        ("2", "uint8", 0, "a text"),
        (1, "bool", 0, "a number"),
        (1, "string", 0, "a number"),
        ("abc", "bytes", 2, "past cap"),
        ("Ā", "bytes", 8, "no byte"),
    ]:
        try:
            request_field_wire(ScriptValue.of(value), "f", RequestFieldType(kind, cap))
        except TypedPayloadError:
            continue
        raise AssertionError(f"{why}: {value!r} as {kind} was accepted")
