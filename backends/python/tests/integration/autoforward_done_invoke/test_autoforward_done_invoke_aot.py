# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.4 autoforward carries ``done.invoke.<id>`` — Python AOT path.

Appendix D's ``mainEventLoop`` forwards every event it dequeues from the
external queue to each ``autoforward`` child without testing the event's
name; the sole exclusion is the cancel event, expressed as control flow.
§6.4.2 places ``done.invoke.<id>`` on the external queue of the invoking
session — "the external service ... MUST return a special event
'done.invoke.id' to the external event queue of the invoking process" — so
a sibling child that is still running must receive it.

Fixture: ``integration_resources/autoforward_done_invoke/autoforward_done_invoke.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_autoforward_done_invoke_python.sh`` (local)
  ``sce-codegen generate-integration -l python --stem autoforward_done_invoke`` (CI)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import autoforward_done_invoke_sm as _sm  # noqa: E402 — path inserted above


def test_autoforward_done_invoke_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    elapsed = 0
    while not engine.reached_final and elapsed < 100:
        engine.advance_time(10)
        elapsed += 10

    assert engine.reached_final, (
        "autoforward_done_invoke did not reach a top-level <final> within "
        f"100 ms; last leaf={engine.current_state} — the watcher child reported "
        "neither verdict, so `done.invoke.inv_short` never reached the parent's "
        "external queue at all"
    )
    actual = str(engine.current_state)
    assert actual == "pass", (
        f"autoforward_done_invoke reached <final id={actual!r}>; expected 'pass' "
        "— the watcher saw only `probe`, so `done.invoke.inv_short` was withheld "
        "from a live `autoforward` child. W3C Appendix D `mainEventLoop` forwards "
        "every event dequeued from the external queue and excludes only the cancel "
        "event, and §6.4.2 places `done.invoke.<id>` on that queue: no name-based "
        "platform-event filter belongs on the forwarding path"
    )
