# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML B.2.8.1: a payload the datamodel could not read arrives as a
space-normalized string, and the host that built it can find out — Python AOT.

The clause gives a payload three readings and names the third "otherwise".
That word is where a belief leaves the system quietly. A host serializes
``{"done":true}``, something truncates it to ``{"done":``, and the clause is
satisfied: the content becomes a string. The document then evaluates
``_event.data.done``, finds nothing, and takes the transition it would have
taken had the host sent a payload with no ``done`` field at all. Nothing is
raised — the fallback is CORRECT behaviour, not an error — so before this
fixture nothing anywhere said it had happened.

These two deliveries are what no pre-existing accessor separates::

    answer  {"done":              the payload never parsed
    answer  {"ready":true}        it parsed; `done` is genuinely absent

Fixture: ``integration_resources/undecodable_payload_is_reported/undecodable_payload_is_reported.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_undecodable_payload_is_reported_python.sh``
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import undecodable_payload_is_reported_sm as _sm  # noqa: E402 — path inserted above
from sce_runtime import EventMetadata  # noqa: E402 — path inserted above

_Event = _sm.UndecodablePayloadIsReportedEvent

# Content that announces an object and stops. The shape a truncated write, a
# half-flushed buffer or a serializer that died mid-record produces.
TRUNCATED_OBJECT = '{"done":'
# The same failure announced with `[`, under the other event name, so a channel
# that reports "the last event" rather than "the last event that lost a
# payload" cannot pass by accident.
TRUNCATED_ARRAY = "[1,2"
# W3C test 562 sends exactly this shape and requires it to arrive as a string.
# Counting it would make the statistic fire on documents that are working.
PROSE = "just a sentence"
# What the host meant to send.
INTACT_OBJECT = '{"done":true}'


def _started():
    engine = _sm.create_engine()
    engine.initialize()
    return engine


def _deliver(engine, event, payload: str) -> None:
    engine.send_event(event, EventMetadata(data=payload))


def test_a_payload_that_announced_structure_and_did_not_parse_is_counted() -> None:
    """The axis: content that asked for the structured reading and did not get
    it is counted."""
    engine = _started()
    assert engine.undecodable_payloads() == 0, (
        "nothing has been delivered before the first event"
    )

    _deliver(engine, _Event.ANSWER, TRUNCATED_OBJECT)

    assert engine.policy.answers() == 1, (
        "the `answer` transition did not run, so nothing below is measuring a "
        "delivery that reached the document"
    )
    assert engine.undecodable_payloads() == 1, (
        f"the host sent `{TRUNCATED_OBJECT}`, which announces an object and does "
        "not parse as one. W3C SCXML B.2.8.1 correctly delivers it as a string; "
        "the host that built it has no other way to learn its payload stopped "
        "being structure"
    )
    assert str(engine.current_state) == "waiting", (
        "the reading a payload got must not change which transition fired; the "
        f"machine is in {engine.current_state!s}"
    )


def test_prose_and_a_payload_that_parsed_are_not_counted() -> None:
    """The other half. A count that also counts success cannot be used to
    detect failure, and the reading the clause calls "otherwise" is the NORMAL
    outcome for a document whose author wrote prose."""
    engine = _started()

    _deliver(engine, _Event.NOTE, PROSE)
    assert engine.policy.notes() == 1, "the `note` transition did not run"
    assert engine.undecodable_payloads() == 0, (
        f"`{PROSE}` is the third reading working as W3C SCXML B.2.8.1 specifies "
        "and as W3C test 562 requires. A diagnostic that fires when nothing is "
        "wrong is one nobody reads"
    )

    _deliver(engine, _Event.ANSWER, INTACT_OBJECT)
    assert str(engine.current_state) == "accepted", (
        f"the guard `_event.data.done` did not hold for `{INTACT_OBJECT}`, so the "
        "structured reading did not happen and the zero below would be proving "
        f"nothing (machine is in {engine.current_state!s})"
    )
    assert engine.undecodable_payloads() == 0, (
        "a payload that parsed was counted as one that did not"
    )


def test_the_loss_is_not_derivable_from_any_other_accessor() -> None:
    """Why the query has to exist: the two deliveries the fixture's comment
    names are identical through every accessor a host had."""
    broken = _started()
    _deliver(broken, _Event.ANSWER, TRUNCATED_OBJECT)

    intact = _started()
    # Valid JSON, and `done` is genuinely absent — the innocent explanation an
    # operator has to rule out.
    _deliver(intact, _Event.ANSWER, '{"ready":true}')

    def observable(engine):
        return (
            str(engine.current_state),
            sorted(str(s) for s in engine.active_configuration()),
            engine.is_running,
            engine.reached_final,
            engine.policy.answers(),
        )

    assert observable(broken) == observable(intact), (
        "this fixture exists because a lost payload and an absent field are "
        "indistinguishable through the accessors a host had; if they ever "
        "differ, the fixture stopped measuring what it claims"
    )
    assert (broken.undecodable_payloads(), intact.undecodable_payloads()) == (1, 0), (
        "the two runs agree on everything else, so this count is the only thing "
        "that separates a broken sender from a working one"
    )


def test_the_engine_names_the_delivery_that_lost_its_payload() -> None:
    """A count says a payload was lost; a host debugging a stalled supervisor
    needs to know which delivery lost it."""
    engine = _started()
    assert engine.last_undecodable_payload() is None, (
        "nothing has been delivered yet"
    )

    _deliver(engine, _Event.ANSWER, TRUNCATED_OBJECT)
    assert engine.last_undecodable_payload() == _Event.ANSWER, (
        "the engine counted a lost payload but cannot say which delivery lost it"
    )

    # A second loss, under the other event name: the accessor has to track the
    # last event THAT LOST A PAYLOAD, not the last event.
    _deliver(engine, _Event.NOTE, TRUNCATED_ARRAY)
    assert engine.undecodable_payloads() == 2, "the count is a count, not a flag"
    assert engine.last_undecodable_payload() == _Event.NOTE

    # And a delivery that succeeds must leave both alone — otherwise the last
    # name would drift to whatever arrived most recently.
    _deliver(engine, _Event.ANSWER, INTACT_OBJECT)
    assert str(engine.current_state) == "accepted", (
        "the intact payload did not take the guarded transition, so the check "
        "below is not measuring a successful delivery"
    )
    assert (engine.undecodable_payloads(), engine.last_undecodable_payload()) == (
        2,
        _Event.NOTE,
    ), "a delivery that parsed moved a record that belongs to one that did not"
