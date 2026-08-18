# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.2 + 6.3: a ``<cancel>`` still lands when the host moved
time in one coarse step — Python AOT path.

The scheduler is a min-heap keyed on ``due_ms`` and ``advance_time``
drains it. Draining it to exhaustion before running a macrostep is the
defect: a step that jumped over two due times holds both entries, and
appending both to the external queue makes the second undroppable
before the first one's transitions have run. The ``<cancel>`` then
executes against a queue the event has already left.

This backend is where the defect is sharpest, because the clock is
virtual: the host owns time outright, so the step size alone decides
the outcome. Measured 2026-08-19, the same document reached
``cancelLost`` at ``advance_time(250)`` and ``pass`` at
``advance_time(150)`` — determinism turning on a number nothing told
the host to pick.

Fixture: ``integration_resources/late_tick_honours_cancel/late_tick_honours_cancel.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_late_tick_honours_cancel_python.sh`` (local)
  ``sce-codegen generate-integration -l python --stem late_tick_honours_cancel`` (CI)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import late_tick_honours_cancel_sm as _sm  # noqa: E402 — path inserted above

# Past both `<send delay>`s in `waiting` (100 ms and 200 ms) in a single move.
PAST_BOTH_DEADLINES = 250


def _started():
    engine = _sm.create_engine()
    engine.initialize()
    return engine


def test_fixture_is_scheduler_driven() -> None:
    """The fixture is only meaningful on a scheduler-driven machine, and
    the policy is where a consumer reads that without running anything."""
    engine = _started()
    assert engine.policy.needs_event_scheduler(), (
        "the fixture arms two delayed <send>s; a policy that does not report "
        "needs_event_scheduler means the document lost them, and every "
        "assertion below would then be measuring the wrong machine"
    )


def test_cancel_survives_one_coarse_time_step() -> None:
    """One move of virtual time, past both due times, must still deliver
    ``poke`` first and let ``active``'s ``<cancel sendid="s1">`` drop
    ``settle``."""
    engine = _started()
    assert str(engine.current_state) == "waiting", (
        "the machine should be waiting on its two delayed sends; it is in "
        f"{engine.current_state!s}"
    )

    engine.advance_time(PAST_BOTH_DEADLINES)

    assert str(engine.current_state) != "cancelLost", (
        "`settle` was delivered even though `active`'s <cancel sendid=\"s1\"> ran "
        "first. Both entries were due when this step started, so the drain "
        "appended them to the external queue together and the cancel found "
        "nothing left to drop. W3C SCXML 6.3 cancels a send that has not been "
        "delivered — delivery is one entry per macrostep, not one heap-flush "
        "per advance_time"
    )

    # The verdict is itself scheduler-driven, so a channel whose scheduler
    # stopped working fails here rather than passing by never moving.
    elapsed = 0
    while not engine.reached_final and elapsed < 1000:
        engine.advance_time(20)
        elapsed += 20
    assert str(engine.current_state) == "pass", (
        f"the machine did not reach `pass` after the cancel; it is in "
        f"{engine.current_state!s}"
    )


def test_a_fine_step_reaches_the_same_verdict() -> None:
    """A host that steps between the two due times is the easy case, and
    it must keep working — the fix is about the coarse step, not about
    changing what a fine one does."""
    engine = _started()
    elapsed = 0
    while not engine.reached_final and elapsed < 1000:
        engine.advance_time(10)
        elapsed += 10
    assert str(engine.current_state) == "pass", (
        "a 10 ms step, which lands between the 100 ms and 200 ms due times, "
        f"must reach `pass`; it reached {engine.current_state!s}"
    )


def test_engine_says_how_far_time_must_move() -> None:
    """The due time the host would have to guess is one the engine can
    state — and on a virtual clock it is the exact amount to move."""
    engine = _started()

    due = engine.time_until_next_scheduled_ms()
    assert due == 100, (
        "the nearer of the two armed sends is 100 ms out; the engine answered "
        f"{due}, which would send a host past the earlier due time"
    )

    # Drive by the engine's own answer: every step lands exactly on a due
    # time, so no step can straddle two of them.
    for _ in range(20):
        if engine.reached_final:
            break
        step = engine.time_until_next_scheduled_ms()
        assert step is not None, (
            "the machine is not finished and nothing is scheduled, so a host "
            "driving by deadlines alone would stall here"
        )
        engine.advance_time(max(step, 1))

    assert str(engine.current_state) == "pass", (
        f"deadline-driven stepping did not reach `pass`; it reached "
        f"{engine.current_state!s}"
    )
    assert engine.time_until_next_scheduled_ms() is None, (
        "nothing is scheduled once the machine is finished, so no clock "
        "movement is owed"
    )
