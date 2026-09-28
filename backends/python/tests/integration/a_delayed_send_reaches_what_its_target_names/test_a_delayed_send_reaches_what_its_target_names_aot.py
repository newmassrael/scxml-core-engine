# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.2 + 6.4 + C.1: a delayed <send> reaches what its target names — Python AOT.

A delay postpones the send and does not change where it goes: `#_internal`
reaches the internal queue, `#_parent` and `#_<invokeid>` reach that session
carrying the payload, an invocation that ended before the delay elapsed is
reported with error.communication, and a session this processor cannot reach
is reported when the send is made.

Fixture: ``integration_resources/a_delayed_send_reaches_what_its_target_names/a_delayed_send_reaches_what_its_target_names.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_a_delayed_send_reaches_what_its_target_names_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import a_delayed_send_reaches_what_its_target_names_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.ADelayedSendReachesWhatItsTargetNamesState


def test_a_delayed_send_reaches_what_its_target_names() -> None:
    engine = _sm.create_engine()
    engine.initialize()
    elapsed = 0
    while not engine.reached_final and elapsed < 1000:
        engine.advance_time(10)
        elapsed += 10

    p = engine.policy
    assert engine.terminal_state == _State.DONE, "the run must end in `done`"
    observed = {
        "order": (p.order(), 31),
        "innerInternal": (p.inner_internal(), 1),
        "lateOk": (p.late_ok(), 1),
        "lateCount": (p.late_count(), 1),
        "pongOk": (p.pong_ok(), 1),
        "commErrors": (p.comm_errors(), 2),
        "lostArrived": (p.lost_arrived(), 0),
        "afterStranger": (p.after_stranger(), 0),
    }
    wrong = {name: got for name, (got, want) in observed.items() if got != want}
    assert not wrong, f"observed {wrong}, want { {n: w for n, (_, w) in observed.items()} }"
