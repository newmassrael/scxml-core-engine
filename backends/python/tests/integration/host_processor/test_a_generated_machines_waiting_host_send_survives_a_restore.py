# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""SCE Accepted Subset §2.15, "Saving and restoring": a delayed ``<send>`` the HOST
serves is a wait a saved machine carries, with the request it will make when the wait
ends, measured through the machine a generator wrote.

``statechart_static_delayed_host_send.scxml`` is a ``datamodel="sce-static"``
document, generated with the processor declaration: the send is armed by the code the
generator emitted, with its two typed ``<param>``s read from the machine's fields, and
saved and restored by the ``save_at`` and ``restore`` the generator emitted. The Rust,
Kotlin and Go halves are ``a_generated_machines_waiting_host_send_survives_a_restore.rs``,
``AGeneratedMachinesWaitingHostSendSurvivesARestoreTest.kt`` and
``a_generated_machines_waiting_host_send_survives_a_restore_test.go``. The text this
backend writes is the format-2 ``saved/statechart_static_delayed_host_send_waiting.json``
(docs/adr/0045), its root given the id the fixtures name it by; the format-1 text of
the same run, under ``saved/format1/``, still restores.

The run ``start``, ``bump`` changes ``job`` after the send was armed, so the request
the wait carries (``job`` 7) is told from one evaluated again when the wait ends
(``job`` 8): a restore hands the host the request the document made, and the machine
itself carries on with the ``job`` it had.

Driven entirely on the engine's virtual clock: no case sleeps, and none can be decided
by how loaded the build machine is.
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))
# The checked operations a lowered expression calls are the forge runtime's.
sys.path.insert(0, str(_HERE.parents[2] / "forge-runtime"))

from sce_runtime import saved_state_from_json  # noqa: E402 -- path inserted above

import statechart_static_delayed_host_send_sm as _sm  # noqa: E402 -- path inserted above

_State = _sm.StatechartStaticDelayedHostSendState
_DECLARED_TYPE = "x-sce-host"
# The session id the shared format-2 fixtures give their root.
_ROOT = "fixture-root"
# The wall-clock moment the shared instance was saved at.
_SAVED_AT_MS = 1_700_000_000_000
# How long the document's send waits, and how much of that has gone by when the
# machine is saved.
_WAIT_MS = 500
_WAITED_MS = 100
_SAVED = _HERE.parents[4] / "sce-build" / "tests" / "fixtures" / "host_processor" / "saved"


def _shared(format1: bool = False) -> str:
    directory = _SAVED / "format1" if format1 else _SAVED
    return (directory / "statechart_static_delayed_host_send_waiting.json").read_text(encoding="utf-8").strip()


class _Host:
    """What a restored machine's handler is asked: the engine's reading of "now" at the
    moment, and the request."""

    def __init__(self) -> None:
        self.calls: list = []
        self.now = 0

    def register(self, engine) -> "_Host":
        def handler(request):
            self.calls.append((self.now, request))
            return []

        engine.register_event_processor(_DECLARED_TYPE, handler)
        return self

    def advance_to(self, engine, to_ms: int) -> None:
        assert to_ms >= self.now, "time does not run backwards in these cases"
        delta, self.now = to_ms - self.now, to_ms
        engine.advance_time(delta)


def _drive(engine, name: str) -> None:
    engine.send_event(engine.policy.resolve_event_by_name(name))
    engine.advance_time(0)


def _waiting():
    """A machine that armed its send with `job` 7, saw `bump`, and has waited
    `_WAITED_MS` of the `_WAIT_MS`."""
    engine = _sm.create_engine(session_id=_ROOT)
    engine.register_event_processor(_DECLARED_TYPE, lambda request: [])
    engine.initialize()
    _drive(engine, "start")
    _drive(engine, "bump")
    engine.advance_time(_WAITED_MS)
    assert engine.current_state == _State.WAITING, f"the machine stands in {engine.current_state}"
    return engine


def _restored(text: str, elapsed_ms: int):
    engine = _sm.restore(saved_state_from_json(text), wall_now_ms=_SAVED_AT_MS + elapsed_ms)
    return engine, _Host().register(engine)


def _job_of(request) -> str:
    """The `job` of a request, from the text the wire carries it as."""
    values = request.params.get("job", [])
    return values[0] if values else ""


def test_a_waiting_send_is_saved_with_the_request_it_was_armed_with() -> None:
    saved = _sm.save_at(_waiting(), _SAVED_AT_MS)
    assert len(saved.pending) == 1, "one send is waiting"
    entry = saved.pending[0]
    assert entry.due == _SAVED_AT_MS + _WAIT_MS - _WAITED_MS
    act = entry.act
    assert (act.kind, act.processor_type, act.event, act.target, act.send_id) == (
        "host",
        _DECLARED_TYPE,
        "audit",
        "job://report",
        "a",
    ), "a host-served send"
    # Armed with `job` 7: `bump` ran after, and the request does not follow it.
    assert act.params == {"job": ["7"], "label": ["report"]}
    # The text every backend writes for the same machine, byte for byte. It also says
    # the machine itself carries the `job` it has now (8), apart from the 7 the request
    # was armed with.
    assert saved.to_json() == _shared()


def test_a_restored_send_is_made_when_its_moment_comes_with_the_job_it_was_armed_with() -> None:
    # Back 50 ms after the save: the send has 350 ms left of the 400 it had.
    engine, host = _restored(_shared(), 50)
    assert engine.current_state == _State.WAITING
    host.advance_to(engine, 349)
    assert host.calls == [], "nothing is due yet"
    host.advance_to(engine, 350)
    assert len(host.calls) == 1, "the send is made once"
    at, request = host.calls[0]
    assert at == 350, "at the moment it came due"
    assert (request.processor_type, request.event_name, request.target, request.send_id) == (
        _DECLARED_TYPE,
        "audit",
        "job://report",
        "a",
    )
    assert _job_of(request) == "7", "the request the document made, not one evaluated again with the job the machine holds now"
    assert request.event_data == '{"job":7,"label":"report"}'


def test_a_waiting_send_saved_in_format_one_is_made_when_its_moment_comes() -> None:
    # The format-1 text of the same run, which a backend wrote before trees.
    engine, host = _restored(_shared(format1=True), 50)
    host.advance_to(engine, 350)
    assert [_job_of(request) for _, request in host.calls] == ["7"]


def test_a_send_already_due_when_the_machine_comes_back_is_made_when_it_is_driven() -> None:
    # A minute away: the wait ran out while the process was dead.
    engine, host = _restored(_shared(), 60_000)
    assert host.calls == [], "restoring makes no send"
    host.advance_to(engine, 0)
    assert len(host.calls) == 1 and host.calls[0][0] == 0


def test_a_restored_send_can_still_be_cancelled_by_its_id() -> None:
    engine, host = _restored(_shared(), 0)
    _drive(engine, "cancel")
    host.advance_to(engine, 10_000)
    assert host.calls == [], "the cancel found the send the saved state carried"


def test_a_restored_machine_carries_on_with_the_job_it_had() -> None:
    # `job` is 8 in the saved state; a `bump` after the restore makes it 9, and a new
    # wait armed from `idle` reads the machine's own field. The restored wait is over
    # (at 400 ms) before the second is armed: a send under an id still waiting is
    # replaced by one backend and left beside it by another, and this case is not about
    # that.
    engine, host = _restored(_shared(), 0)
    host.advance_to(engine, 400)
    _drive(engine, "stop")
    _drive(engine, "bump")
    _drive(engine, "start")
    host.advance_to(engine, 10_000)
    assert [_job_of(request) for _, request in host.calls] == ["7", "9"], (
        "the restored wait first, then the one armed with the job the restored field held"
    )


def test_a_saved_state_round_trips_through_its_own_text() -> None:
    engine, _ = _restored(_shared(), 0)
    assert _sm.save_at(engine, _SAVED_AT_MS).to_json() == _shared()
