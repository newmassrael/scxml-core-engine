# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.2: a <send> delay is read as one CSS2 time — Python AOT.

Fixture: ``integration_resources/a_delay_is_a_css2_time/a_delay_is_a_css2_time.scxml``.

The engine's clock is advanced a full minute: both valid delays fire in the
order their milliseconds give, and a refused message, had it been scheduled
under some default wait, would have arrived and moved ``bad``.

Regeneration (after fixture or template edit):
  ``scripts/regen_a_delay_is_a_css2_time_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import a_delay_is_a_css2_time_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.ADelayIsACss2TimeState


def test_each_delay_is_read_as_one_css2_time() -> None:
    engine = _sm.create_engine()
    engine.initialize()
    engine.advance_time(60_000)

    p = engine.policy
    assert engine.terminal_state == _State.DONE, "`a` must carry the run to `done`"
    observed = {
        "errors": (p.errors(), 2),
        "after": (p.after(), 0),
        "bad": (p.bad(), 0),
        "aAfterB": (p.a_after_b(), 1),
    }
    wrong = {name: got for name, (got, want) in observed.items() if got != want}
    assert not wrong, f"observed {wrong}, want { {n: w for n, (_, w) in observed.items()} }"
