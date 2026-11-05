# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 5.10.1 + C.1 (docs/adr/0090, test 1): the origin of an event the
SCXML Event I/O Processor delivers is the address its sender published, on every
route a session sends by, and a reply to it arrives — Python AOT.

A self-send (written with the alias ``type="scxml"``), a child's send to
``#_parent`` and a parent's send to ``#_<invokeid>``: each time ``_event.origin``
is the sender's ``_ioprocessors`` SCXML location, ``_event.origintype`` is the
processor's URI, and a reply sent with ``targetexpr="_event.origin"
typeexpr="_event.origintype"`` arrives — the child's reply to its parent's
location included, which reaches the parent as ``#_parent`` does.

Fixture: ``integration_resources/an_origin_is_the_address_its_sender_published/an_origin_is_the_address_its_sender_published.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_an_origin_is_the_address_its_sender_published_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import an_origin_is_the_address_its_sender_published_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.AnOriginIsTheAddressItsSenderPublishedState


def test_an_origin_is_the_address_its_sender_published() -> None:
    engine = _sm.create_engine()
    engine.initialize()
    # A reply that reaches nobody leaves the run waiting; the bound turns that
    # into the state the run stopped in rather than a hang.
    elapsed = 0
    while not engine.reached_final and elapsed < 3000:
        engine.advance_time(10)
        elapsed += 10

    p = engine.policy
    assert engine.terminal_state == _State.DONE, (
        f"the run must end in `done`; it stopped in {engine.current_state}, where a "
        "reply sent to an origin that reached nobody leaves it waiting"
    )
    observed = {
        "selfOk": (p.self_ok(), 1),
        "selfReplied": (p.self_replied(), 1),
        "childOk": (p.child_ok(), 1),
        "childReplied": (p.child_replied(), 1),
        "parentOk": (p.parent_ok(), 1),
        "parentReplied": (p.parent_replied(), 1),
    }
    wrong = {name: got for name, (got, want) in observed.items() if got != want}
    assert not wrong, f"observed {wrong}, want { {n: w for n, (_, w) in observed.items()} }"
