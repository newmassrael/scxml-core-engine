# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.2.4 + C.2: a delayed BasicHTTP send is POSTed when due — Python AOT.

A delay is a property of the send, not of the processor it names, so a
delayed BasicHTTP send is POSTed when the delay has elapsed — not at once,
not never — and `<cancel>` reaches it while it waits.

Fixture: ``integration_resources/a_delayed_http_send_is_posted_when_due/a_delayed_http_send_is_posted_when_due.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_a_delayed_http_send_is_posted_when_due_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import a_delayed_http_send_is_posted_when_due_sm as _sm  # noqa: E402 — path inserted above
from sce_runtime import HttpSendFailure  # noqa: E402 — path inserted above

_State = _sm.ADelayedHttpSendIsPostedWhenDueState
_Event = _sm.ADelayedHttpSendIsPostedWhenDueEvent


def test_a_basic_http_send_its_transport_could_not_deliver_is_error_communication() -> None:
    """W3C SCXML 6.2.4: a send the processor is unable to dispatch puts
    error.communication on the internal queue. The transport says the POST of
    `now` did not reach its server; the document declares no transition for
    the error, so it is the one unhandled error the engine counts."""
    engine = _sm.create_engine()

    def transport(request):
        if request.event_name == "now":
            raise HttpSendFailure("connection refused")
        return None

    engine.set_http_send_callback(transport)
    engine.initialize()
    assert engine.unhandled_error_events() == 1, (
        "the undelivered POST of `now` must raise exactly one error"
    )
    assert engine.last_unhandled_error() == _Event.ERROR_COMMUNICATION


def test_a_basic_http_send_with_no_transport_is_error_communication() -> None:
    """A machine given no BasicHTTP transport has no way to dispatch the send:
    each of the three sends made at once is reported, none dropped unsaid."""
    engine = _sm.create_engine()
    engine.initialize()
    assert engine.unhandled_error_events() == 3, (
        "the three sends made at once with no transport must each raise an error"
    )
    assert engine.last_unhandled_error() == _Event.ERROR_COMMUNICATION


def test_a_delayed_http_send_is_posted_when_due() -> None:
    engine = _sm.create_engine()
    # The host stands in for the HTTP transport: it keeps what it was handed.
    posted = []
    engine.set_http_send_callback(lambda request: posted.append(request))
    engine.initialize()

    def events():
        return [request.event_name for request in posted]

    # A zero wait, written or evaluated, is no deferral: the POST is made
    # before initialize() returns, with no tick to bring it out.
    assert events() == ["now", "zero", "zeroexpr"], (
        "the undelayed send and the two zero-delay sends are POSTed at once"
    )

    engine.advance_time(99)
    assert events() == ["now", "zero", "zeroexpr"], "a send delayed 100ms is not POSTed at 99ms"

    engine.advance_time(1)
    assert events() == ["now", "zero", "zeroexpr", "later"], (
        "it is POSTed when the delay has elapsed, and the cancelled one never is"
    )
    assert posted[3].target == "http://127.0.0.1:18081/later"
    assert posted[3].send_id == "later"

    engine.advance_time(100)
    assert events() == ["now", "zero", "zeroexpr", "later", "dynamic"]
    assert posted[4].target == "http://127.0.0.1:18081/dynamic", "a targetexpr is read when the send is made"
    assert posted[4].params.get("k") == ["v"]

    engine.advance_time(100)
    assert engine.terminal_state == _State.DONE, "the run must end in `done`"
    assert events() == ["now", "zero", "zeroexpr", "later", "dynamic"]
