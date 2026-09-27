# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.2 + 4.9: a bad <send> argument discards its message — Python AOT.

Fixture: ``integration_resources/a_bad_send_argument_discards_its_message/a_bad_send_argument_discards_its_message.scxml``.

The engine's clock is advanced a full minute before ``finish``: a channel that
scheduled the message whose ``delayexpr`` failed, under some default delay,
delivers it within that minute and moves ``sent``.

Regeneration (after fixture or template edit):
  ``scripts/regen_a_bad_send_argument_discards_its_message_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import a_bad_send_argument_discards_its_message_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.ABadSendArgumentDiscardsItsMessageState
_Event = _sm.ABadSendArgumentDiscardsItsMessageEvent


def test_each_bad_argument_discards_its_message() -> None:
    engine = _sm.create_engine()
    engine.initialize()
    engine.advance_time(60_000)
    engine.send_event(_Event.FINISH)

    p = engine.policy
    assert engine.terminal_state == _State.DONE, "`finish` must carry the run to `done`"
    observed = {"errors": (p.errors(), 6), "sent": (p.sent(), 0), "after": (p.after(), 0)}
    wrong = {name: got for name, (got, want) in observed.items() if got != want}
    assert not wrong, f"observed {wrong}, want { {n: w for n, (_, w) in observed.items()} }"
