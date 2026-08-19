# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 3.1.2: "If no transition matches in any state, the event is
discarded" — and the host that fed it in can find out. Python AOT path.

Three outcomes leave the configuration identical, so no accessor that
existed before this fixture separates them:

  ``poke``    self transition       handled (exits and re-enters ``idle``)
  ``nudge``   targetless internal   handled (actions only, no exit/entry)
  ``settle``  no matching           DISCARDED — the host's event went nowhere

The C++ Interpreter answers all three (``processEvent``'s
``TransitionResult.success`` and ``getStatistics().failedTransitions``);
the generated engines computed the same fact at the same point of
Appendix D's ``mainEventLoop`` and dropped it.

``nudge`` is in the fixture because the engines' own "did anything
happen" answer is a different fact: it reports whether the configuration
changed, and a targetless internal transition changes nothing after
running its actions.

Fixture: ``integration_resources/discarded_event_is_observable/discarded_event_is_observable.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_discarded_event_is_observable_python.sh``
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import discarded_event_is_observable_sm as _sm  # noqa: E402 — path inserted above

_Event = _sm.DiscardedEventIsObservableEvent


def _started():
    engine = _sm.create_engine()
    engine.initialize()
    return engine


def test_an_event_no_active_state_answered_is_counted() -> None:
    """The axis: an event the machine knows but no active state answers."""
    engine = _started()
    assert engine.discarded_external_events() == 0, (
        "nothing has been discarded before the first event"
    )

    # `settle` is declared in `busy`, so it is in the machine's vocabulary
    # and the host can name it — it just matches nothing in `idle`.
    engine.send_event(_Event.SETTLE)

    assert engine.discarded_external_events() == 1, (
        "`settle` came off the external queue in `idle`, where no transition "
        "matches it. W3C SCXML 3.1.2 discards it; the host that queued it has "
        "no other way to learn its event went nowhere"
    )
    assert str(engine.current_state) == "idle", (
        "a discarded event must not move the machine; it is in "
        f"{engine.current_state!s}"
    )


def test_a_handled_event_is_not_counted() -> None:
    """The other half — including the handled event that changes nothing."""
    engine = _started()

    engine.send_event(_Event.POKE)
    assert engine.policy.pokes() == 1, (
        "`poke`'s self transition did not run, so nothing below is measuring "
        "a handled event"
    )
    assert engine.discarded_external_events() == 0, (
        "`poke` matched a self transition — handled, and the configuration is "
        "unchanged only because the transition returns to its own source"
    )

    engine.send_event(_Event.NUDGE)
    assert engine.policy.nudges() == 1, "`nudge`'s targetless transition did not run"
    assert engine.discarded_external_events() == 0, (
        "`nudge` matched a targetless internal transition: its actions ran and "
        "no state was exited or entered, which is why the count cannot be "
        "keyed off whether the configuration changed"
    )


def test_the_discard_is_not_derivable_from_any_other_accessor() -> None:
    """Why the query has to exist: every pre-existing accessor answers the
    same for a handled event and a discarded one."""
    engine = _started()

    engine.send_event(_Event.POKE)
    handled = (
        str(engine.current_state),
        sorted(str(s) for s in engine.active_configuration()),
        engine.is_running,
        engine.reached_final,
    )

    engine.send_event(_Event.SETTLE)
    discarded = (
        str(engine.current_state),
        sorted(str(s) for s in engine.active_configuration()),
        engine.is_running,
        engine.reached_final,
    )

    assert handled == discarded, (
        "this fixture exists because these two are indistinguishable through "
        "the accessors a host had; if they ever differ, the fixture stopped "
        "measuring what it claims"
    )
    assert engine.discarded_external_events() == 1, (
        "the two are indistinguishable through every other accessor, so the "
        "count is the only thing that separates them"
    )


def test_the_engine_names_the_event_it_discarded() -> None:
    """A count says something went nowhere; this says which thing did."""
    engine = _started()
    assert engine.last_discarded_event() is None, "nothing has been discarded yet"

    engine.send_event(_Event.SETTLE)

    assert engine.last_discarded_event() == _Event.SETTLE, (
        "the engine counted a discard but cannot say which event it was"
    )


def test_an_event_the_machine_has_moved_past_is_counted() -> None:
    """The supervisor's actual failure mode: the machine moved on and the
    events the host keeps sending no longer match anything."""
    engine = _started()
    engine.send_event(_Event.GO)
    assert str(engine.current_state) == "busy", (
        "`go` should have moved the machine out of `idle`"
    )

    engine.send_event(_Event.POKE)
    assert engine.discarded_external_events() == 1, (
        "the machine left `idle`, so `poke` no longer matches — the host that "
        "kept sending it is exactly who the count is for"
    )
    assert engine.last_discarded_event() == _Event.POKE
