# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.4: autoforward is owed to the external event, not to the door it
came through — Python AOT path.

The four sibling ``autoforward_*`` stems all let the machine forward events it
queued for itself. This one hands it one from outside, through the engine's own
"here is an event" entry point, and asks whether the ``autoforward`` child sees
it. Appendix D's ``mainEventLoop`` binds the preliminary step (``applyFinalize``
plus the autoforward ``send``) to the external event it is about to select
transitions for, so an engine with a second door has to run the step at both or
the child goes blind to everything the host delivers.

Measured 2026-08-21: the C++ AOT engine had the step written inline in its queue
drain, so ``processEvent()`` skipped it. This engine's ``send_event`` appends to
the external queue and runs the main loop, so the drain is its only door and
this pins that — a later ``send_event`` that hands the event straight to the
transition selector would go red here.

Fixture: ``integration_resources/host_event_reaches_the_child/host_event_reaches_the_child.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_host_event_reaches_the_child_python.sh`` (local)
  ``sce-codegen generate-integration -l python --stem host_event_reaches_the_child`` (CI)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import host_event_reaches_the_child_sm as _sm  # noqa: E402 — path inserted above


def test_host_event_reaches_the_child_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    # The child opens the exchange, so let its `ready` move the parent into
    # `armed` — the one state that can be handed an event from outside.
    elapsed = 0
    while str(engine.current_state) != "armed" and elapsed < 100:
        engine.advance_time(10)
        elapsed += 10

    assert str(engine.current_state) == "armed", (
        f"parent parked in {engine.current_state!s}; expected 'armed' — the probe "
        "child never sent `ready`, so the fixture never reached the state where a "
        "host event can be handed over. That is a broken handshake, not a "
        "forwarding verdict"
    )

    # The axis: the host's own entry point.
    engine.send_event(_sm.HostEventReachesTheChildEvent.HOST_PING)

    elapsed = 0
    while not engine.reached_final and elapsed < 100:
        engine.advance_time(10)
        elapsed += 10

    assert engine.reached_final, (
        "host_event_reaches_the_child did not reach a top-level <final> within "
        f"100 ms; last leaf={engine.current_state} — the probe child answered "
        "neither verdict, so neither `hostPing` nor `marker` reached it"
    )
    actual = str(engine.current_state)
    assert actual == "pass", (
        f"host_event_reaches_the_child reached <final id={actual!r}>; expected 'pass' "
        "— the probe child answered `sawMarkerOnly`, so the event the host handed to "
        "`send_event` was never forwarded to it and the child only ever saw the "
        "`marker` the parent's own transition body sent. W3C Appendix D "
        "`mainEventLoop` runs the autoforward `send` against the external event "
        "before it selects transitions for it, whichever door the event arrived "
        "through, so an engine that runs that step only in its queue drain leaves an "
        "`autoforward` child blind to everything its host delivers"
    )
