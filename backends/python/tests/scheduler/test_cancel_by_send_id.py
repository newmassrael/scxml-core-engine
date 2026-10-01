# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.3 — what `<cancel sendid>` reaches, asked of
``sce_runtime.scheduler.Scheduler`` directly.

`<cancel>` cancels a delayed event that an earlier `<send>` queued and that is
still queued. It is not a standing order about an id: a `<send>` made AFTER the
cancel, with the same id, is a new send and is delivered.

Measured 2026-10-01 on a retry machine: a state that arms a timer on entry
(``<send id="t" delay="200ms"/>``) and cancels it on exit (``<cancel sendid="t"/>``),
re-entered by its own timeout transition, delivered its first timer and never
its second. The exit ran the cancel at the moment the first timer fired, when
nothing was pending under ``t``; the scheduler remembered the id anyway and
dropped the next entry that carried it. The machine sat in its waiting state
for ever, sent its request twice instead of three times, and never reported the
timeout. The Rust and Go schedulers remove what is pending and remember
nothing, and these cases are the answers they give (``cancel_event`` in
``backends/rust/runtime/src/engine.rs``, ``CancelEvent`` in
``backends/go/runtime/scheduler.go``).

This directory is deliberately NOT a fixture stem: it holds the scheduler's
semantics to hand-worked answers, which no document drives.
``scripts/gates/w3c-python.sh`` names it explicitly for that reason.
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "runtime"))

from sce_runtime.scheduler import Scheduler  # noqa: E402


def drained(scheduler: Scheduler, now_ms: int) -> list:
    """Every entry due at `now_ms`, as the engine takes them: one at a time."""
    out = []
    while True:
        entry = scheduler.pop_due(now_ms)
        if entry is None:
            return out
        out.append((entry.due_ms, entry.sendid))


def test_a_send_made_after_a_cancel_of_its_id_is_delivered():
    """The retry machine, reduced: the timer fires, the exit cancels its id
    while nothing is pending, the re-entry arms the same id again."""
    s = Scheduler()
    s.schedule(200, "t", "deadline")
    assert drained(s, 200) == [(200, "t")]
    s.cancel("t")                      # onexit, with nothing pending under "t"
    s.schedule(400, "t", "deadline")   # onentry of the re-entered state
    assert drained(s, 400) == [(400, "t")]


def test_a_cancel_removes_the_pending_entry_it_names():
    s = Scheduler()
    s.schedule(200, "t", "deadline")
    s.cancel("t")
    assert drained(s, 10_000) == []
    assert len(s) == 0


def test_a_cancel_removes_every_pending_entry_under_the_id():
    s = Scheduler()
    s.schedule(100, "t", "a")
    s.schedule(300, "t", "b")
    s.schedule(200, "u", "c")
    s.cancel("t")
    assert drained(s, 10_000) == [(200, "u")]


def test_entries_still_come_out_in_deadline_order_after_a_cancel_removes_the_front():
    """Taking an entry out of a heap's array can leave the array no longer a
    heap. These five land in the array as [100, 300, 200, 500, 400]; removing
    the 100 leaves [300, 200, 500, 400], whose front is not the earliest."""
    s = Scheduler()
    for due, sendid in ((100, "x"), (400, "y"), (200, "z"), (500, "w"), (300, "v")):
        s.schedule(due, sendid, sendid)
    s.cancel("x")
    assert drained(s, 10_000) == [(200, "z"), (300, "v"), (400, "y"), (500, "w")]


def test_a_cancel_leaves_other_ids_pending():
    s = Scheduler()
    s.schedule(100, "t", "a")
    s.schedule(200, "u", "b")
    s.cancel("t")
    assert drained(s, 10_000) == [(200, "u")]


def test_an_empty_id_cancels_nothing_and_an_empty_id_entry_is_not_cancellable():
    s = Scheduler()
    s.schedule(100, "", "anonymous")
    s.cancel("")
    assert drained(s, 100) == [(100, "")]


def test_a_cancel_of_an_id_nothing_ever_used_changes_nothing():
    s = Scheduler()
    s.cancel("never")
    s.schedule(100, "never", "a")
    assert drained(s, 100) == [(100, "never")]


def test_the_next_deadline_skips_what_was_cancelled():
    s = Scheduler()
    s.schedule(100, "t", "a")
    s.schedule(300, "u", "b")
    s.cancel("t")
    assert s.peek_next_due_ms() == 300
    s.cancel("u")
    assert s.peek_next_due_ms() is None


def test_a_cancel_between_two_due_entries_reaches_the_later_one():
    """The settle timer: two entries come due together, the first one's macrostep
    cancels the second. They are taken one at a time so the cancel lands."""
    s = Scheduler()
    s.schedule(100, "first", "a")
    s.schedule(100, "second", "b")
    entry = s.pop_due(100)
    assert entry.sendid == "first"
    s.cancel("second")
    assert s.pop_due(100) is None
