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


def _run_observed(clear: bool):
    """Run the machine to its end with an observer that keeps each transition it is
    told as ``(source, index)``, detached again before the run when ``clear``."""
    seen = []

    def observer(kind, fields):
        if kind == "transition":
            seen.append((fields["source"], fields["index"]))

    engine = _sm.create_engine()
    engine.set_observer(observer)
    if clear:
        engine.set_observer(None)
    engine.initialize()
    elapsed = 0
    while not engine.reached_final and elapsed < 200:
        engine.advance_time(10)
        elapsed += 10
    assert engine.reached_final and str(engine.terminal_state) == "pass", (
        f"the observed run did not end in `pass`; last leaf={engine.current_state}"
    )
    return seen


def test_the_observer_is_told_each_row_of_the_table_the_run_takes() -> None:
    """The document declares its transition table, and a run that reaches ``pass`` takes
    each of its six transitions once, in document order: the observer is told exactly
    those rows, each by the ``(from_state, index)`` its row of the table carries
    (docs/adr/0089)."""
    seen = _run_observed(clear=False)

    assert seen == [("s1", 0), ("s2", 0), ("s3", 0), ("s4", 0), ("s5", 0), ("s6", 0)], (
        f"the rows taken, in the order the run took them: {seen}"
    )
    rows = {(str(row.from_state), row.index) for row in _sm.TRANSITIONS}
    assert set(seen) <= rows, f"every pair the observer is told is a row of the table: {seen} against {rows}"
    assert rows <= set(seen), f"the run takes every row of the table: {rows} against {seen}"


def test_a_detached_observer_is_told_nothing_and_the_run_is_the_same() -> None:
    assert _run_observed(clear=True) == []
