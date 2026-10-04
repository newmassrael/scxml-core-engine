# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A computed event name is matched like any other (§scxml-3.12.1, §scxml-5.10) — Python AOT.

A ``<send eventexpr>`` names its event at run time, so the document cannot have
written the name: it is delivered as the event the document's names resolve it
to (its own, the longest token prefix of it the document writes, or its
wildcard), and ``_event.name`` is the whole name. The document sends six, over
the external and the internal queue, now and after a delay, and takes each only
when it is told the whole name.

Fixture: integration_resources/a_computed_event_name_is_matched_like_any_other/a_computed_event_name_is_matched_like_any_other.scxml
(canonical, shared with the C++ / Rust / Go / Kotlin / C11 channels).

Regeneration (after fixture or template edit):
  ``scripts/regen_a_computed_event_name_is_matched_like_any_other_python.sh`` (local)
  ``sce-codegen generate-integration -l python --stem a_computed_event_name_is_matched_like_any_other`` (CI)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import a_computed_event_name_is_matched_like_any_other_sm as _sm  # noqa: E402 — path inserted above


def test_a_computed_event_name_is_matched_like_any_other_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    # Two sends wait 20 ms each; 10 ms steps give the run all the time it can use.
    elapsed = 0
    while not engine.reached_final and elapsed < 200:
        engine.advance_time(10)
        elapsed += 10

    assert engine.reached_final, (
        "a_computed_event_name_is_matched_like_any_other did not reach a top-level <final> "
        f"within 200 ms; last leaf={engine.current_state} — a computed name was dropped "
        "(`request.new`, `other.thing`, `request.again`, `last.one`), or matched but told "
        "shorter than the name it was sent under"
    )
    actual = str(engine.terminal_state)
    assert actual == "pass", (
        f"a_computed_event_name_is_matched_like_any_other reached <final id={actual!r}>; expected 'pass'"
    )
