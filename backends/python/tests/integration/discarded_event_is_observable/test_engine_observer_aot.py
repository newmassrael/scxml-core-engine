# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""``Engine.set_observer``: what a host can learn about HOW a machine took an event.

``discarded_external_events`` says that an event went nowhere. It cannot say which
transition took one that did not, nor that an ``error.execution`` was raised in the
middle of a round and who answered it. A host that wants to explain a machine — why
a position was written at 0 ms where ten seconds were expected — needs the sequence,
and the observer is that sequence: an event taken, a transition selected, a state left
or entered, an event nothing took.

The same fixture as ``test_discarded_event_is_observable_aot.py`` because it holds every
outcome in one small document:

  ``poke``    self transition      exits and re-enters ``idle``
  ``nudge``   targetless internal  runs its content, exits and enters nothing
  ``go``      idle -> busy
  ``settle``  matches nothing in ``idle`` (discarded), then busy -> done

The claims, each with the nearest case that must not be read the same way:

  order       the events and transitions arrive in the order the machine took them
  a name      a transition is told by the descriptor it listens for, and a targetless
              one says it has no target
  a discard   an event nothing took is told apart from one that was handled
  untouched   a machine with an observer reaches the same configurations and counts as one
              without, and one detached stops being told
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import discarded_event_is_observable_sm as _sm  # noqa: E402 — path inserted above

_Event = _sm.DiscardedEventIsObservableEvent


class _Told:
    """An observer that keeps what it is told, as `(kind, fields)`."""

    def __init__(self) -> None:
        self.steps = []

    def __call__(self, kind, fields) -> None:
        self.steps.append((kind, dict(fields)))

    def kinds(self):
        return [kind for kind, _ in self.steps]

    def take(self):
        taken, self.steps = self.steps, []
        return taken


def _started(observer=None):
    engine = _sm.create_engine()
    engine.initialize()
    if observer is not None:
        engine.set_observer(observer)
    return engine


def test_a_move_is_told_as_the_event_the_transition_then_the_exit_and_the_entry() -> None:
    told = _Told()
    engine = _started(told)

    engine.send_event(_Event.GO)

    assert told.steps == [
        ("event", {"name": "go"}),
        ("transition", {"source": "idle", "event": "go", "targets": ["busy"], "internal": False}),
        ("exit", {"state": "idle"}),
        ("enter", {"state": "busy"}),
    ]


def test_a_targetless_transition_is_told_without_an_exit_or_an_entry() -> None:
    """`nudge` runs its content and leaves the configuration alone — the observer must
    not invent a move for it."""
    told = _Told()
    engine = _started(told)

    engine.send_event(_Event.NUDGE)

    assert told.kinds() == ["event", "transition"]
    assert told.steps[1][1]["targets"] == [], "a transition with no target says so"
    assert told.steps[1][1]["event"] == "nudge"


def test_a_self_transition_is_an_exit_and_an_entry_of_the_same_state() -> None:
    told = _Told()
    engine = _started(told)

    engine.send_event(_Event.POKE)

    assert told.kinds() == ["event", "transition", "exit", "enter"]
    assert told.steps[2][1] == told.steps[3][1] == {"state": "idle"}


def test_an_event_nothing_took_is_told_as_discarded_and_not_as_handled() -> None:
    told = _Told()
    engine = _started(told)

    engine.send_event(_Event.SETTLE)

    assert told.steps == [("event", {"name": "settle"}), ("discarded", {"name": "settle"})]


def test_an_observer_changes_nothing_the_machine_does() -> None:
    """Same events, with and without a tap: same configuration, same counts."""
    plain, watched = _started(), _started(_Told())
    for event in (_Event.POKE, _Event.NUDGE, _Event.SETTLE, _Event.GO, _Event.SETTLE):
        plain.send_event(event)
        watched.send_event(event)

    assert str(plain.current_state) == str(watched.current_state) == "done"
    assert plain.discarded_external_events() == watched.discarded_external_events() == 1
    assert plain.policy.pokes() == watched.policy.pokes() == 1
    assert plain.policy.nudges() == watched.policy.nudges() == 1


def test_a_detached_observer_is_told_nothing_more() -> None:
    told = _Told()
    engine = _started(told)
    engine.send_event(_Event.POKE)
    told.take()

    engine.set_observer(None)
    engine.send_event(_Event.NUDGE)

    assert told.steps == []


def test_an_observer_that_raises_is_not_swallowed() -> None:
    """A trace that silently stopped would read as "nothing happened"."""
    def broken(kind, fields):
        raise RuntimeError("the tap broke")

    engine = _started(broken)
    try:
        engine.send_event(_Event.NUDGE)
    except RuntimeError as exc:
        assert "the tap broke" in str(exc)
    else:
        raise AssertionError("the observer's failure was swallowed")
