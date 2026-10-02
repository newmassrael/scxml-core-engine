# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.2.4 — a ``<send>`` whose route is read from an open question.

A draft routes a notice with ``targetexpr="callerTarget"`` because a send needs
a target, while the specification never says who the caller is, and marks the
data item as an open question (``sce:unresolved``). Played, the send fails with
``error.communication`` (W3C SCXML C.1) and nothing answers it. The engine
already counts that (``unhandled_error_events``); what it could not say is that
the failure is the owner's question showing through and not a fault of the
document, and a host that plays the design to an owner needs exactly that.

The generated machine raises the failures OF THE ROUTE with the questions the
route rests on (``raise_internal(..., rests_on=...)``); the engine queues them
with the ``error.*`` event and gives them back for the last one nothing answered
(``last_unhandled_error_rests_on``). Outcomes the fixture holds apart, each
raised by a send:

  send.open       route read from an open question  -> ("caller-target",)
  send.badtype    TYPE read from an open question   -> ("caller-target",)
  send.badaddress / noroute / badroute / badtypeexpr, each a failure of the route
                                                    -> ("caller-target",)
  send.assumed    route read from ``sce:assumed``   -> ()  applied, not a question
  send.plain      route read from a plain data item -> ()
  send.literal    no computed route                 -> ()
  send.badevent   sound route that rests on an open question, the EVENT NAME fails -> ()
  send.baddelay   the same, the DELAY fails                                        -> ()
  send.badnamelist the same, the NAMELIST fails                                    -> ()

The last three are faults of the document that no answer to the route's question
would mend, and so they are not attributed to it (measured 2026-10-02: an earlier
attribution by send id laid every error of such a send to the route's question).
The error the document does answer is not unhandled, so nothing is given back for
it.

Fixture: ``sce-build/tests/fixtures/route_rests_on_a_question/route_rests_on_a_question.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_route_rests_on_a_question_python.sh``
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import route_rests_on_a_question_sm as _sm  # noqa: E402 — path inserted above
from sce_runtime.datamodel_read import read_int  # noqa: E402
from sce_runtime.scripting import LuaScriptEngine  # noqa: E402

_Event = _sm.RouteRestsOnAQuestionEvent

#: The id the fixture marks `openRoute` with, spelled here and not read back: a
#: test that asked the machine for its own question would agree with any id.
QUESTION = "caller-target"


def _started():
    script_engine = LuaScriptEngine()
    script_engine.initialize()
    engine = _sm.create_engine(script_engine=script_engine)
    engine.initialize()
    return engine, script_engine


def _answered(engine, script_engine) -> int:
    """How many errors the document's own handler took: the `<assign>` is the
    only witness that a transition answered, every outcome leaving the machine
    in a state the configuration alone cannot tell from the others."""
    value = read_int(script_engine, engine.policy._session_id, "answered")
    assert value is not None, "the fixture declares `answered` in its datamodel"
    return value


def _held(engine) -> dict:
    """The questions the engine still holds for errors that are queued.

    ⚠ A private attribute, read on purpose: an entry is dropped when the error
    carrying it leaves the queue, and an engine that kept every one would
    answer every public question the same way while growing for as long as the
    machine fails. Bounded by what is queued is the contract, and nothing public
    states it."""
    return engine._queued_route_failures


def test_nothing_is_given_back_before_an_error() -> None:
    engine, _ = _started()
    assert engine.unhandled_error_events() == 0
    assert engine.last_unhandled_error_rests_on() == ()


def test_an_error_from_a_route_read_from_an_open_question_gives_the_question_back() -> None:
    engine, _ = _started()
    engine.send_event(_Event.SEND_OPEN)
    assert engine.unhandled_error_events() == 1, (
        "the send raised an error and nothing answered it, or there is "
        "nothing below to attribute"
    )
    assert engine.last_unhandled_error_rests_on() == (QUESTION,)


def test_a_value_chosen_without_an_answer_is_not_a_question() -> None:
    engine, _ = _started()
    engine.send_event(_Event.SEND_ASSUMED)
    assert engine.unhandled_error_events() == 1
    assert engine.last_unhandled_error_rests_on() == ()


def test_a_plain_route_gives_nothing_back() -> None:
    engine, _ = _started()
    engine.send_event(_Event.SEND_PLAIN)
    assert engine.unhandled_error_events() == 1
    assert engine.last_unhandled_error_rests_on() == ()


def test_a_type_read_from_an_open_question_gives_the_question_back() -> None:
    """The route is its type as much as its target: a `typeexpr` that names a
    processor this platform does not support fails the ROUTE, and it rests on the
    question the type was read from."""
    engine, _ = _started()
    engine.send_event(_Event.SEND_BADTYPE)
    assert engine.unhandled_error_events() == 1
    assert engine.last_unhandled_error_rests_on() == (QUESTION,)


def test_every_way_the_route_itself_fails_gives_the_question_back() -> None:
    """The route fails when its address is one this processor cannot use, when it
    evaluates to nothing, when its expression fails to evaluate, and when its type
    expression does. Each is a failure of the route, so each rests on the question."""
    for name in ("SEND_BADADDRESS", "SEND_NOROUTE", "SEND_BADROUTE", "SEND_BADTYPEEXPR"):
        engine, _ = _started()
        engine.send_event(getattr(_Event, name))
        assert engine.unhandled_error_events() == 1, (
            f"{name} raised no error nothing answered, so there is nothing to attribute"
        )
        assert engine.last_unhandled_error_rests_on() == (QUESTION,), name


def test_an_event_name_that_fails_is_not_laid_to_the_route_s_question() -> None:
    """The route is sound and rests on an open question; the event name reads a
    variable nobody declared. No answer to the route's question would mend that,
    so the error carries none: the fault is the document's."""
    engine, _ = _started()
    engine.send_event(_Event.SEND_BADEVENT)
    assert engine.unhandled_error_events() == 1, (
        "the event name failed and nothing answered it, or there is nothing "
        "below to attribute"
    )
    assert engine.last_unhandled_error_rests_on() == ()


def test_a_delay_that_fails_is_not_laid_to_the_route_s_question() -> None:
    engine, _ = _started()
    engine.send_event(_Event.SEND_BADDELAY)
    assert engine.unhandled_error_events() == 1
    assert engine.last_unhandled_error_rests_on() == ()


def test_a_namelist_that_fails_is_not_laid_to_the_route_s_question() -> None:
    engine, _ = _started()
    engine.send_event(_Event.SEND_BADNAMELIST)
    assert engine.unhandled_error_events() == 1
    assert engine.last_unhandled_error_rests_on() == ()


def test_a_literal_send_that_fails_after_an_open_one_is_not_given_its_question() -> None:
    """Each unhandled error is given the questions it was raised with, and one
    raised with none carries none: it does not inherit the last error's. A wrong
    question named is worse than none."""
    engine, _ = _started()
    engine.send_event(_Event.SEND_OPEN)
    assert engine.last_unhandled_error_rests_on() == (QUESTION,)
    engine.send_event(_Event.SEND_LITERAL)
    assert engine.unhandled_error_events() == 2
    assert engine.last_unhandled_error_rests_on() == ()


def test_a_question_is_given_back_for_the_error_that_came_from_its_send() -> None:
    """The other order: the plain failure first, the open one after. Each
    unhandled error replaces what the last one gave back, in both directions."""
    engine, _ = _started()
    engine.send_event(_Event.SEND_PLAIN)
    assert engine.last_unhandled_error_rests_on() == ()
    engine.send_event(_Event.SEND_OPEN)
    assert engine.unhandled_error_events() == 2
    assert engine.last_unhandled_error_rests_on() == (QUESTION,)


def test_an_error_the_document_answers_is_not_unhandled_and_gives_nothing_back() -> None:
    engine, se = _started()
    engine.send_event(_Event.LISTEN)
    engine.send_event(_Event.SEND_OPEN)
    assert _answered(engine, se) == 1, (
        "the document's handler did not take the error, so nothing below is "
        "measuring an error the document answered"
    )
    assert engine.unhandled_error_events() == 0
    assert engine.last_unhandled_error_rests_on() == ()
    assert _held(engine) == {}, "the question outlived the error that carried it"


def test_a_route_that_resolves_fails_nothing_and_holds_nothing() -> None:
    """An open question on a route that works raises no error, so no question is
    attributed and none is held: an entry exists only for an error that is queued,
    and is removed when that error is taken off the queue."""
    engine, _ = _started()
    engine.send_event(_Event.SEND_REACHABLE)
    assert engine.unhandled_error_events() == 0
    assert engine.last_unhandled_error_rests_on() == ()
    assert _held(engine) == {}, "a send that failed nothing left a question held"


def test_nothing_is_held_once_an_unhandled_error_has_been_taken() -> None:
    engine, _ = _started()
    engine.send_event(_Event.SEND_OPEN)
    assert engine.last_unhandled_error_rests_on() == (QUESTION,)
    assert _held(engine) == {}, "the question outlived the error that carried it"
