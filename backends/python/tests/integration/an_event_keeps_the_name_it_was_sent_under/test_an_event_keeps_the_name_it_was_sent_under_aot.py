# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The name an event arrives under (§scxml-5.10, §scxml-3.12.1) on the Python
AOT local-invoke path.

A transition on ``request`` takes ``request.new`` by whole-token matching, and
``_event.name`` is then the name the event was sent under, not the descriptor it
was matched through. The public IRP suite never reads a name the document does
not write, so a machine that is told the shorter one passes all of it.

Fixture: ``integration_resources/an_event_keeps_the_name_it_was_sent_under/an_event_keeps_the_name_it_was_sent_under.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_an_event_keeps_the_name_it_was_sent_under_python.sh`` (local)
  ``sce-codegen generate-integration -l python --stem an_event_keeps_the_name_it_was_sent_under`` (CI)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import an_event_keeps_the_name_it_was_sent_under_sm as _sm  # noqa: E402 — path inserted above


def test_an_event_keeps_the_name_it_was_sent_under_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    elapsed = 0
    while not engine.reached_final and elapsed < 100:
        engine.advance_time(10)
        elapsed += 10

    assert engine.reached_final, (
        "an_event_keeps_the_name_it_was_sent_under did not reach a top-level <final> "
        f"within 100 ms; last leaf={engine.current_state} — the child never heard "
        "`request.new`, so it never answered"
    )
    actual = str(engine.terminal_state)
    assert actual == "pass", (
        f"an_event_keeps_the_name_it_was_sent_under reached <final id={actual!r}>; "
        "expected 'pass' — the child reported `arrivedShortened`: `request.new` took "
        "the transition on `request` but `_event.name` told the child `request`. "
        "§scxml-5.10 makes the name the one the event was sent under"
    )
