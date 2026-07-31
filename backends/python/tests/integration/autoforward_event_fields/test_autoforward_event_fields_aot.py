# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.4 autoforward field preservation on the Python AOT
local-invoke path.

W3C §6.4 requires the parent to forward an exact copy of every external
event to an ``<invoke autoforward="true">`` child. The public IRP suite
never checks the copy's contents: test229 only asserts the event name
crosses, and test230 is a manual test whose field comparison is done by a
human reading two log dumps. A forward stripped down to the bare event
name — or one carrying the payload but not ``_event.origin`` /
``_event.invokeid`` — passes both.

Fixture: ``integration_resources/autoforward_event_fields/autoforward_event_fields.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_autoforward_event_fields_python.sh`` (local)
  ``sce-codegen generate-integration -l python --stem autoforward_event_fields`` (CI)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import autoforward_event_fields_sm as _sm  # noqa: E402 — path inserted above


def test_autoforward_event_fields_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    elapsed = 0
    while not engine.reached_final and elapsed < 100:
        engine.advance_time(10)
        elapsed += 10

    assert engine.reached_final, (
        "autoforward_event_fields did not reach a top-level <final> within "
        f"100 ms; last leaf={engine.current_state} — the child never received "
        "the forwarded `childToParent`, so no done.invoke.inv_echo was emitted"
    )
    actual = str(engine.current_state)
    assert actual == "pass", (
        f"autoforward_event_fields reached <final id={actual!r}>; expected 'pass' "
        "— the child reported `stripped`, so the autoforwarded copy of "
        "`childToParent` lost `_event.data.value`, `_event.origin` or "
        "`_event.invokeid`. W3C §6.4 requires an exact copy: "
        "forward_to_autoforward_children must carry the source event's "
        "EventMetadata, not just its name and payload"
    )
