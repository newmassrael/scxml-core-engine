# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 3.7: only a top-level ``<final>`` ends the session — Python AOT path.

Appendix D ``enterStates`` sets ``running = false`` for a ``<final>`` only when
``isSCXMLElement(s.parent)``; otherwise it queues ``done.state.<parent>`` and the
machine carries on. ``is_final_state`` is therefore the structural question —
"is this state a ``<final>`` element" — while the engine's completion flag
answers "has this session ended", and only the latter may gate completion, the
completion callback, and the ``done.invoke.<id>`` a parent emits.

The fixture rests in the nested final rather than passing through it: a machine
that continues within the same macrostep is only ever sampled at the end, where
a right and a wrong predicate agree.

Fixture: ``integration_resources/nested_final_not_terminal/nested_final_not_terminal.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_nested_final_not_terminal_python.sh`` (local)
  ``sce-codegen generate-integration -l python --stem nested_final_not_terminal`` (CI)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import nested_final_not_terminal_sm as _sm  # noqa: E402 — path inserted above


def test_nested_final_not_terminal_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    assert str(engine.current_state) == "phaseDone", (
        "the fixture is supposed to come to rest in the nested <final>; it rested "
        f"in {engine.current_state!s} instead, so nothing below tests what it claims"
    )
    assert not engine.reached_final, (
        "the engine reported completion while resting in `phaseDone`, a <final> "
        "nested inside `phase`. W3C SCXML Appendix D enterStates ends the session "
        "only when the final's parent is the <scxml> element — a nested one "
        "finishes its compound state and queues done.state.phase, leaving the "
        "machine live"
    )

    engine.send_event(_sm.NestedFinalNotTerminalEvent.RESUME)

    elapsed = 0
    while not engine.reached_final and elapsed < 100:
        engine.advance_time(10)
        elapsed += 10

    assert engine.reached_final, (
        "the machine did not complete after `resume`; last leaf="
        f"{engine.current_state!s}"
    )
    assert str(engine.current_state) == "pass", (
        "`resume` did not carry the machine out of the nested final to the "
        f"top-level one; it reached {engine.current_state!s}"
    )
