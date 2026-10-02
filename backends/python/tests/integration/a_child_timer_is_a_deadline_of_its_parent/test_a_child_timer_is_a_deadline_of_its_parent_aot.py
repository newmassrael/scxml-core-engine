# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A child session's ``<send delay>`` is a deadline of the machine that invoked
it — Python AOT.

A host that drives a scheduler-owning machine asks the engine when it next
needs a tick and sleeps that long. The parent's tick advances every running
child by the same step, so a moment a child needs is a moment the host must not
step over. An answer that counts only the parent's own scheduler tells a host
with nothing of the parent's armed that there is nothing to wait for, and the
child's timer is never fired.

This runtime counts its children (``time_until_next_scheduled_ms`` takes the
nearest of its own scheduler and each unfinished child's), and this driver is
what says so for the generated machine; the Rust, Go and C++ AOT drivers beside
it are the ones that found the runtimes which did not.

Virtual time throughout: no case sleeps, and each move lands exactly on the
deadline the engine reported, which is the use the answer exists for.

Fixture: ``tests/integration/a_child_timer_is_a_deadline_of_its_parent.scxml`` —
not under ``integration_resources/``, because a stem there is a seven-channel
contract and C11 and the C++ Interpreter expose no next-deadline query to ask.

Regeneration (after fixture or template edit):
  ``scripts/regen_a_child_timer_is_a_deadline_of_its_parent_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import a_child_timer_is_a_deadline_of_its_parent_sm as _sm  # noqa: E402 — path inserted above


def _started():
    engine = _sm.create_engine()
    engine.initialize()
    return engine


def test_fixture_is_scheduler_driven() -> None:
    """The fixture is only meaningful on a scheduler-driven machine, and the
    policy is where a consumer reads that without running anything."""
    engine = _started()
    assert engine.policy.needs_event_scheduler(), (
        "the child arms <send delay>s and the parent ticks it; a policy that "
        "does not report needs_event_scheduler means the document lost them, "
        "and every assertion below would be measuring the wrong machine"
    )


def test_engine_names_its_childs_first_timer_as_its_own_next_deadline() -> None:
    """The parent arms nothing, so the only deadline in the run is the child's
    first timer, 200 ms after the child started."""
    engine = _started()
    assert str(engine.current_state) == "waiting", (
        f"the parent should be waiting on its child; it is in {engine.current_state!s}"
    )
    assert engine.time_until_next_scheduled_ms() == 200, (
        "the child armed <send delay=\"200ms\"> when it started and the parent "
        "ticks the child, so that deadline is the parent's. An answer of None "
        "tells a host there is nothing to wait for while a running child's "
        "timer is pending"
    )


def test_host_walking_time_by_the_answer_reaches_the_end_of_the_child() -> None:
    """The child arms its second timer when the first fires, so the whole run
    is two moves of 200 ms and the engine has to name the second one only after
    the first has been taken."""
    engine = _started()
    walked = []
    while True:
        due = engine.time_until_next_scheduled_ms()
        if due is None:
            break
        assert len(walked) < 8, f"the engine keeps naming deadlines: {walked}"
        walked.append(due)
        engine.advance_time(due)

    assert walked == [200, 200], (
        "each move should land on the child's next timer, the second of which "
        "is armed by the first firing"
    )
    assert str(engine.terminal_state) == "finished", (
        "the child's last timer ended it, so the parent should have taken "
        "done.invoke.kid and finished"
    )
    assert engine.time_until_next_scheduled_ms() is None, (
        "nothing is armed once the child has ended"
    )
