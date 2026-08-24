# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML G.7 ``<sce:action>`` — Python compile+run gate for native host dispatch.

Fixture: ``sce-build/tests/fixtures/event_schema/statechart_native_action.scxml``,
the same document the Rust, Go, Kotlin, C++ and C11 channels drive. The
generated policy takes a ``StatechartNativeActionActions`` implementation at
construction and reaches the script engine for nothing at all — the construct
is engine-free by definition, and that is what this gate measures at runtime
rather than in the emitted text.

What the cases measure:

* ``append_fragment_payload`` reads two typed ``_event.data`` fields (a
  ``bytes`` payload, a ``uint32`` offset) bound from the event's typed payload;
* ``reset_slot`` takes no arguments;
* ``on_idle_entry`` and ``on_assembling_exit`` appear in NO transition, so they
  prove the engine-free entry/exit path and that an eventless-only action still
  gets a generated Protocol method;
* an event sent BY NAME carries no typed payload, and the arg-bearing action
  must not fire against a zero value it would take for data. That one is the
  half a configuration assertion cannot see — the machine reaches
  ``assembling`` either way.

Regeneration (after fixture or template edit):
  ``scripts/regen_native_action.sh``
"""
from __future__ import annotations

import sys
from pathlib import Path
from typing import List, Tuple

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import statechart_native_action_sm as _sm  # noqa: E402 — path inserted above


class Recorder:
    """Host implementation of the generated operations.

    Records each dispatch so a case can assert the engine-free call path fired
    with the arguments the event carried.
    """

    def __init__(self) -> None:
        self.appended: List[Tuple[bytes, int]] = []
        self.resets = 0
        self.idle_entries = 0
        self.assembling_exits = 0

    def append_fragment_payload(self, payload: bytes, offset: int) -> None:
        self.appended.append((bytes(payload), offset))

    def reset_slot(self) -> None:
        self.resets += 1

    def on_idle_entry(self) -> None:
        self.idle_entries += 1

    def on_assembling_exit(self) -> None:
        self.assembling_exits += 1


def _started(host: Recorder):
    engine = _sm.create_engine(host)
    engine.initialize()
    return engine


def test_native_action_dispatches_typed_payload_to_host_protocol():
    host = Recorder()
    engine = _started(host)

    assert engine.current_state == _sm.StatechartNativeActionState.IDLE
    # `<onentry>` of the initial state fires on entry — the engine-free
    # entry-effect path, with no transition having to carry the action.
    assert host.idle_entries == 1, "on_idle_entry must fire on the initial entry to idle"

    # Per-event typed inject: `fragment.received` with a bytes payload and an
    # offset. The transition fires append_fragment_payload.
    # `send_event` processes to stability on this engine, so there is no
    # separate step to drive.
    _sm.raise_fragment_received(engine, b"abc", 7)

    assert engine.current_state == _sm.StatechartNativeActionState.ASSEMBLING
    assert host.appended == [(b"abc", 7)], (
        f"append_fragment_payload must receive the typed _event.data payload "
        f"and offset natively; got {host.appended!r}"
    )

    # `reset` fires the no-argument reset_slot and returns to idle. Exiting
    # `assembling` fires its `<onexit>` effect; re-entering `idle` fires
    # `<onentry>` a second time.
    engine.send_event(_sm.StatechartNativeActionEvent.RESET)

    assert engine.current_state == _sm.StatechartNativeActionState.IDLE
    assert host.resets == 1, "reset_slot must have fired once"
    assert host.assembling_exits == 1, "on_assembling_exit must fire when leaving assembling"
    assert host.idle_entries == 2, "re-entering idle must fire its <onentry> again"


def test_native_action_does_not_fire_without_its_typed_payload():
    """An event sent by NAME carries no typed payload.

    The transition still fires — the guard is the event name — but the
    arg-bearing action has nothing to read, and handing the host a zeroed
    buffer it would take for data is the one outcome this seam must never
    produce. Asserted on the host's record rather than on the configuration,
    because the machine reaches ``assembling`` either way.
    """
    host = Recorder()
    engine = _started(host)

    engine.send_event(_sm.StatechartNativeActionEvent.FRAGMENT_RECEIVED)

    assert engine.current_state == _sm.StatechartNativeActionState.ASSEMBLING
    assert host.appended == [], (
        "append_fragment_payload fired without a typed payload to read"
    )
    # The eventless effects still ran: they read no payload, so nothing about
    # this delivery should have stopped them.
    assert host.idle_entries == 1
