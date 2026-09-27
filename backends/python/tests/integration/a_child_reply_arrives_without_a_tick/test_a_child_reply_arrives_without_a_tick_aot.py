# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.4: a child's reply arrives without a clock tick — Python AOT.

Measured 2026-09-27, this channel collected a child's parent-bound events
only in `advance_time`, so a host driving the machine with `send_event`
alone saw its own `finish` overtake the reply.

Fixture: ``integration_resources/a_child_reply_arrives_without_a_tick/a_child_reply_arrives_without_a_tick.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_a_child_reply_arrives_without_a_tick_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import a_child_reply_arrives_without_a_tick_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.AChildReplyArrivesWithoutATickState
_Event = _sm.AChildReplyArrivesWithoutATickEvent


def test_the_childs_reply_arrives_before_the_hosts_next_event() -> None:
    engine = _sm.create_engine()
    engine.initialize()
    engine.send_event(_Event.FINISH)

    p = engine.policy
    seen = f"hellos={p.hellos()} (wanted 1)"
    assert engine.terminal_state == _State.DONE, f"`finish` must carry the run to `done`. {seen}"
    assert p.hellos() == 1, f"the child's start-time reply arrives before `finish`. {seen}"
