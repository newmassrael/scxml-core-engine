# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML Appendix D: pending invokes start before the external dequeue — Python AOT path.

``mainEventLoop`` completes the macrostep on eventless and internal
transitions alone, then runs ``invoke(inv)`` for every state entered on the
last iteration, and only then reaches ``externalQueue.dequeue()``. The
external queue is named exactly once in that loop and it is after the invokes.

An engine that folds the external drain into its macrostep completion loop
consumes whatever ``<onentry>`` queued for the parent itself while the invoked
children do not yet exist, so an ``autoforward`` child misses every event the
parent queued on the way in. That is a lost event, not a reordered one.

The sibling ``autoforward_dequeue_point`` pins *where in the loop* the forward
sits and is deliberately blind to this axis: there the child opens the
exchange, so it is alive before anything is queued. Here the parent queues
first and the child starts second.

Fixture: ``integration_resources/invoke_precedes_external_dequeue/invoke_precedes_external_dequeue.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_invoke_precedes_external_dequeue_python.sh`` (local)
  ``sce-codegen generate-integration -l python --stem invoke_precedes_external_dequeue`` (CI)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import invoke_precedes_external_dequeue_sm as _sm  # noqa: E402 — path inserted above


def test_invoke_precedes_external_dequeue_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    elapsed = 0
    while not engine.reached_final and elapsed < 100:
        engine.advance_time(10)
        elapsed += 10

    assert engine.reached_final, (
        "invoke_precedes_external_dequeue did not reach a top-level <final> within "
        f"100 ms; last leaf={engine.current_state} — the watching child answered "
        "neither verdict, so `probe` never reached it"
    )
    actual = str(engine.current_state)
    assert actual == "pass", (
        f"invoke_precedes_external_dequeue reached <final id={actual!r}>; expected "
        "'pass' — the watching child answered `probe` from `waiting`, so it never saw "
        "`kick`. The parent drained its external queue before starting the invoke, and "
        "the event `<onentry>` had queued for itself was consumed while no child "
        "existed. W3C Appendix D `mainEventLoop` runs `invoke(inv)` for every state "
        "entered on the last iteration before it reaches `externalQueue.dequeue()`, so "
        "an autoforward child is live for the whole external queue"
    )
