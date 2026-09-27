# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.4: every event a child sends in one tick arrives — Python AOT.

Fixture: ``integration_resources/a_child_may_send_many_events_in_one_tick/a_child_may_send_many_events_in_one_tick.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_a_child_may_send_many_events_in_one_tick_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import a_child_may_send_many_events_in_one_tick_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.AChildMaySendManyEventsInOneTickState
_Event = _sm.AChildMaySendManyEventsInOneTickEvent


def test_every_event_the_child_sent_arrives() -> None:
    engine = _sm.create_engine()
    engine.initialize()
    engine.send_event(_Event.FINISH)

    p = engine.policy
    seen = f"ticks={p.ticks()} (wanted 110)"
    assert engine.terminal_state == _State.DONE, f"`finish` must carry the run to `done`. {seen}"
    assert p.ticks() == 110, f"every event the child sent arrives before `finish`. {seen}"
