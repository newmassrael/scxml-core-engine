# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 5.10 + B.2: a payload a HOST injects reaches the datamodel — Python AOT.

The edge nothing measured. Every other integration fixture drives its machine
with an empty payload — measured 2026-08-16, the data argument was ``""`` in
every call on every channel until this one — so the host-to-datamodel boundary
was covered by no test at all. The W3C suite does not reach it either: its
payloads originate inside the document (``<send><content>``, ``<param>``,
``<donedata>``), a separate path in every backend.

Fixture: ``integration_resources/event_data_arrives_as_sent/event_data_arrives_as_sent.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_event_data_arrives_as_sent_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import event_data_arrives_as_sent_sm as _sm  # noqa: E402 — path inserted above
from sce_runtime import EventMetadata  # noqa: E402 — path inserted above

_State = _sm.EventDataArrivesAsSentState
_Event = _sm.EventDataArrivesAsSentEvent


def test_event_data_arrives_as_sent_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    entry = engine.active_configuration()
    assert _State.WAITING in entry, (
        f"fixture came up as {entry}; it is supposed to start in `waiting`, so "
        "nothing below is testing what it claims"
    )

    # A JSON object, the shape an embedder has when it holds structured data
    # and a state machine to give it to.
    engine.send_event(
        _Event.PAYLOAD,
        EventMetadata(data='{"milestone":"refined","turns":2}'),
    )

    after_payload = engine.active_configuration()
    assert _State.MANGLED not in after_payload, (
        "the host sent a JSON object and the guard `_event.data.milestone === "
        "'refined' && _event.data.turns === 2` did not hold, so the payload did not "
        f"arrive as an object with those properties (active: {after_payload})"
    )
    assert _State.HEARD in after_payload, (
        "the payload guard neither matched nor mismatched — the machine is not in "
        f"`heard` (active: {after_payload})"
    )

    # Text that is not JSON. The same call, and it must NOT be parsed into
    # something else: `hold the line` is the value the document compares
    # against, character for character.
    engine.send_event(_Event.NOTE, EventMetadata(data="hold the line"))

    after_note = engine.active_configuration()
    assert _State.GARBLED not in after_note, (
        "the host sent the text `hold the line` and `_event.data === 'hold the line'` "
        "did not hold, so a payload that is not JSON did not arrive as the string it "
        f"was sent as (active: {after_note})"
    )

    # Text that happens to be a valid expression. §scxml-B-2-8-1 gives the
    # payload three readings and none of them is "evaluate it": a payload is
    # what a host, a peer session or an HTTP sender put there, and running it
    # makes `_event.data` mean whatever the receiver's engine is written in.
    engine.send_event(_Event.ARITH, EventMetadata(data="2 + 3"))

    after_arith = engine.active_configuration()
    assert _State.EVALUATED not in after_arith, (
        "the host sent the text `2 + 3` and it arrived as 5 — the payload was run "
        f"rather than read (active: {after_arith})"
    )
    assert _State.DOCUMENTED in after_arith, (
        "the arithmetic-shaped payload neither matched nor mismatched "
        f"(active: {after_arith})"
    )

    # §scxml-B-2-8-1's XML rung, reached through the EVENT path. The `<data>`
    # path is `xml_data_is_a_dom_tree`'s and the two are lowered on separate
    # code in every backend.
    engine.send_event(
        _Event.DOC,
        # Leading whitespace on purpose: the reading is chosen by the first
        # NON-blank character, and a pretty-printed document is the ordinary
        # shape of one. The scan past it is small enough to look redundant.
        EventMetadata(data='\n  <books xmlns=""><book title="t1"/></books>'),
    )

    after_doc = engine.active_configuration()
    assert _State.FLATTENED not in after_doc, (
        "the host sent a well-formed XML document and "
        "`_event.data.documentElement.nodeName === 'books'` did not hold, so the "
        f"payload did not become the DOM structure the clause requires (active: {after_doc})"
    )

    # The sentence that closes the clause. Every `error.*` message this
    # repository raises names the SCXML construct that failed, so every one of
    # them has exactly this shape: it opens like a document and is not one.
    engine.send_event(_Event.BROKEN, EventMetadata(data="<assign>  to  detail failed"))

    after_broken = engine.active_configuration()
    assert _State.SWALLOWED not in after_broken, (
        "the host sent `<assign>  to  detail failed`, which opens with `<` and is not "
        "a valid XML document, so §scxml-B-2-8-1's closing MUST applies and the "
        f"reading is the space-normalized string (active: {after_broken})"
    )
    assert _State.SETTLED in after_broken, (
        "the malformed-XML payload neither matched nor mismatched "
        f"(active: {after_broken})"
    )
