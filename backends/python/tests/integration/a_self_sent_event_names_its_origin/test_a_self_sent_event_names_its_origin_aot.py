# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML C.1: a self-sent event names its origin — Python AOT.

An event a session sends to itself carries this session's published location
as `_event.origin`, a reply to it arrives, and a target expression that
evaluates to nothing raises error.communication and delivers nothing.

Fixture: ``integration_resources/a_self_sent_event_names_its_origin/a_self_sent_event_names_its_origin.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_a_self_sent_event_names_its_origin_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import a_self_sent_event_names_its_origin_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.ASelfSentEventNamesItsOriginState


def test_a_self_sent_event_names_its_origin() -> None:
    engine = _sm.create_engine()
    engine.initialize()
    engine.advance_time(1_000)

    p = engine.policy
    assert engine.terminal_state == _State.DONE, "the run must end in `done`"
    observed = {
        "immediateOk": (p.immediate_ok(), 1),
        "replied": (p.replied(), 1),
        "delayedOk": (p.delayed_ok(), 1),
        "unreachable": (p.unreachable(), 1),
        "strayed": (p.strayed(), 0),
    }
    wrong = {name: got for name, (got, want) in observed.items() if got != want}
    assert not wrong, f"observed {wrong}, want { {n: w for n, (_, w) in observed.items()} }"
