# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

_State = _sm.ADelayedHttpSendIsPostedWhenDueState


def test_a_delayed_http_send_is_posted_when_due() -> None:
    engine = _sm.create_engine()
    # The host stands in for the HTTP transport: it keeps what it was handed.
    posted = []
    engine.set_http_send_callback(lambda request: posted.append(request))
    engine.initialize()

    def events():
        return [request.event_name for request in posted]

    assert events() == ["now"], "only the undelayed send is POSTed at once"

    engine.advance_time(99)
    assert events() == ["now"], "a send delayed 100ms is not POSTed at 99ms"

    engine.advance_time(1)
    assert events() == ["now", "later"], (
        "it is POSTed when the delay has elapsed, and the cancelled one never is"
    )
    assert posted[1].target == "http://sce.invalid/later"
    assert posted[1].send_id == "later"

    engine.advance_time(100)
    assert events() == ["now", "later", "dynamic"]
    assert posted[2].target == "http://sce.invalid/dynamic", "a targetexpr is read when the send is made"
    assert posted[2].params.get("k") == ["v"]

    engine.advance_time(100)
    assert engine.terminal_state == _State.DONE, "the run must end in `done`"
    assert events() == ["now", "later", "dynamic"]
