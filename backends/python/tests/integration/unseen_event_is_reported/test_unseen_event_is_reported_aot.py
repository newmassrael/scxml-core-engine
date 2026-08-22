# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 3.13 + Appendix D: an event handed to a machine that has already
stopped is never looked at, and the host that sent it can find out — Python AOT.

Appendix D's main event loop exits when the machine reaches a top-level final
state. Refusing what arrives afterwards is the clause; saying nothing about it
is not. The silence is expensive because it looks like the two outcomes a host
can already read::

    dequeued, no transition matched            -> discarded_external_events
    dequeued, matched, guard said no           -> nothing, correctly
    never dequeued - the machine had stopped   -> this

Fixture: ``integration_resources/unseen_event_is_reported/unseen_event_is_reported.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_unseen_event_is_reported_python.sh``
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import unseen_event_is_reported_sm as _sm  # noqa: E402 — path inserted above

_Event = _sm.UnseenEventIsReportedEvent


def _started():
    engine = _sm.create_engine()
    engine.initialize()
    return engine


def test_an_event_delivered_after_the_machine_stopped_is_counted() -> None:
    """The axis: an event the host queued after the machine stopped."""
    engine = _started()
    assert engine.unseen_external_events() == 0, (
        "nothing has been refused before the first event"
    )

    engine.send_event(_Event.POKE)
    assert engine.policy.pokes() == 1, (
        "`poke`'s transition did not run, so nothing below is measuring a "
        "machine that was working first"
    )

    engine.send_event(_Event.FINISH)
    assert engine.reached_final, (
        "`finish` should have taken the machine to its top-level final state"
    )
    assert engine.unseen_external_events() == 0, (
        "`finish` was itself dequeued and handled — the machine stopped BECAUSE "
        "of it, which is not the same as stopping before it"
    )

    engine.send_event(_Event.POKE)

    assert engine.unseen_external_events() == 1, (
        "the host queued `poke` on a machine that had reached its final state. "
        "W3C SCXML Appendix D's loop had already ended, so the event was never "
        "dequeued; before this count the host had no way to learn that"
    )
    assert engine.policy.pokes() == 1, (
        "the refused delivery ran the document's transition anyway — the count "
        "would then be reporting something that did not happen"
    )


def test_the_refusal_is_not_derivable_from_any_other_accessor() -> None:
    """Every other accessor answers the same before and after the refusal."""
    engine = _started()
    engine.send_event(_Event.FINISH)

    def observable():
        return (
            str(engine.current_state),
            sorted(str(s) for s in engine.active_configuration()),
            engine.is_running,
            engine.reached_final,
            engine.discarded_external_events(),
            engine.policy.pokes(),
        )

    before = observable()
    engine.send_event(_Event.POKE)
    after = observable()

    assert before == after, (
        "this fixture exists because a refused delivery is indistinguishable "
        "through the accessors a host had; if they ever differ, the fixture "
        "stopped measuring what it claims"
    )
    assert engine.unseen_external_events() == 1, (
        "the two readings agree on everything else, so this count is the only "
        "thing that separates `the machine never looked` from `it looked and "
        "nothing matched`"
    )


def test_a_discard_and_a_refusal_are_counted_separately() -> None:
    """A discard and a refusal are different facts, each with its own count."""
    engine = _started()

    engine.send_event(_Event.POKE)
    assert engine.discarded_external_events() == 0, (
        "`poke` matched a targetless transition; nothing was discarded"
    )
    assert engine.unseen_external_events() == 0, (
        "the machine was running, so nothing was refused either"
    )

    engine.send_event(_Event.FINISH)
    engine.send_event(_Event.POKE)

    assert (engine.discarded_external_events(), engine.unseen_external_events()) == (0, 1), (
        "a refusal must not be reported as a discard: the first says the "
        "machine looked and nothing matched, the second says it never looked, "
        "and a host acts differently on each"
    )


def test_the_engine_names_the_event_it_never_looked_at() -> None:
    """A count says an event went unlooked-at; this says which one."""
    engine = _started()
    assert engine.last_unseen_event() is None, "nothing has been refused yet"

    engine.send_event(_Event.FINISH)
    engine.send_event(_Event.POKE)
    assert engine.last_unseen_event() == _Event.POKE, (
        "the engine counted a refusal but cannot say which event it refused"
    )

    engine.send_event(_Event.FINISH)
    assert (engine.unseen_external_events(), engine.last_unseen_event()) == (
        2,
        _Event.FINISH,
    ), "the count is a count, not a flag, and the name follows the refusals"
