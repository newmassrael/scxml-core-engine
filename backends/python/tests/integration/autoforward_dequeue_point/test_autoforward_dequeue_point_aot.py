# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.4 autoforward happens at the external dequeue — Python AOT path.

Appendix D's ``mainEventLoop`` forwards one statement after
``externalQueue.dequeue()`` and before ``selectTransitions``, and §6.4.2 says
the same in prose: the parent forwards "at the point at which it removes it
from the external event queue". Forwarding where the event is queued instead
breaks run-to-completion — the child sees event N before the parent has
processed 1..N-1.

Siblings ``autoforward_done_invoke`` and ``autoforward_internal_queue`` pin
*which* events are forwarded and are deliberately blind to *when*; this one
pins the position and nothing else.

Fixture: ``integration_resources/autoforward_dequeue_point/autoforward_dequeue_point.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_autoforward_dequeue_point_python.sh`` (local)
  ``sce-codegen generate-integration -l python --stem autoforward_dequeue_point`` (CI)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import autoforward_dequeue_point_sm as _sm  # noqa: E402 — path inserted above


def test_autoforward_dequeue_point_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    elapsed = 0
    while not engine.reached_final and elapsed < 100:
        engine.advance_time(10)
        elapsed += 10

    assert engine.reached_final, (
        "autoforward_dequeue_point did not reach a top-level <final> within "
        f"100 ms; last leaf={engine.current_state} — the probe child reported "
        "neither verdict, so `second` never reached it"
    )
    actual = str(engine.current_state)
    assert actual == "pass", (
        f"autoforward_dequeue_point reached <final id={actual!r}>; expected 'pass' "
        "— the probe child saw `second` before `mark`, so both events were handed "
        "over while the parent was still executing the transition that queued them. "
        "W3C Appendix D `mainEventLoop` forwards one statement after "
        "`externalQueue.dequeue()`, and §6.4.2 puts it \"at the point at which it "
        "removes it from the external event queue\": forwarding at the enqueue lets "
        "the child run ahead of the parent by a whole event"
    )
