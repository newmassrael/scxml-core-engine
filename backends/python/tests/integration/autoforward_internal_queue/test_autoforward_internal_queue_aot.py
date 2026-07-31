# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.4 autoforward skips internal-queue events — Python AOT path.

Appendix D's ``mainEventLoop`` forwards only what it dequeues from the
external queue; the internal drain above it has no forwarding step at all.
§6.2 raises ``error.execution`` onto the internal queue when ``<send>``
names an unsupported type, so it must never reach an ``autoforward``
child — and it must be excluded by where it was raised, not by a filter
that recognises its name.

Sibling of ``autoforward_done_invoke``, which pins the positive half.
Together they leave no room for a name-based filter: one fails if
``done.invoke`` is withheld, the other if ``error.execution`` leaks.

Fixture: ``integration_resources/autoforward_internal_queue/autoforward_internal_queue.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_autoforward_internal_queue_python.sh`` (local)
  ``sce-codegen generate-integration -l python --stem autoforward_internal_queue`` (CI)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import autoforward_internal_queue_sm as _sm  # noqa: E402 — path inserted above


def test_autoforward_internal_queue_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    elapsed = 0
    while not engine.reached_final and elapsed < 100:
        engine.advance_time(10)
        elapsed += 10

    assert engine.reached_final, (
        "autoforward_internal_queue did not reach a top-level <final> within "
        f"100 ms; last leaf={engine.current_state} — the watcher child reported "
        "neither verdict, so neither `error.execution` nor `probe` reached it"
    )
    actual = str(engine.current_state)
    assert actual == "pass", (
        f"autoforward_internal_queue reached <final id={actual!r}>; expected 'pass' "
        "— the watcher saw `error.execution`, so an internal-queue event was "
        "autoforwarded. W3C Appendix D `mainEventLoop` forwards only what it "
        "dequeues from the external queue, and §6.2 raises `error.execution` onto "
        "the internal one: check that the event was not routed onto the external "
        "queue for some unrelated reason, which would leak it past any name-blind "
        "forward"
    )
