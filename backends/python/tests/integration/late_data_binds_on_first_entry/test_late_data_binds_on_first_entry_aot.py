# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 5.3 / Appendix D enterStates, late binding: first entry only — Python AOT.

``s`` is entered, its ``v`` changed to 5, ``s`` left and entered again; its
``<onentry>`` records the ``v`` it sees each time. Measured 2026-09-26, this
channel already kept the first-entry rule in its runtime; the fixture pins it.

Fixture: ``integration_resources/late_data_binds_on_first_entry/late_data_binds_on_first_entry.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_late_data_binds_on_first_entry_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import late_data_binds_on_first_entry_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.LateDataBindsOnFirstEntryState
_Event = _sm.LateDataBindsOnFirstEntryEvent


def test_a_state_binds_its_data_only_on_its_first_entry() -> None:
    engine = _sm.create_engine()
    engine.initialize()
    assert _State.IDLE in engine.active_configuration(), "the run has to start in `idle`"

    # Enter `s`, change its `v`, leave it, enter it again — one step each.
    for event in (_Event.GO, _Event.BUMP, _Event.BACK, _Event.GO):
        engine.send_event(event)

    p = engine.policy
    seen = (
        f"entries={p.entries()} seen={p.seen()} contentSeen={p.content_seen()} "
        "(wanted 2 / 15 / 7)"
    )
    assert _State.S in engine.active_configuration(), (
        f"the second `go` has to leave the machine in `s`. {seen}"
    )
    assert p.entries() == 2, f"both entries of `s` must have run. {seen}"
    assert p.seen() == 15, (
        "`s` saw v=1 on its first entry and must see the 5 it was changed to on its second: 11 is a "
        f"processor that binds late data on every entry, and no value at all one that never binds it. {seen}"
    )
    assert p.content_seen() == 7, (
        f"`c` is bound from inline content, not an expr, and must be bound too. {seen}"
    )
