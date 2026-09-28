# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.2.4 + C.1: a `targetexpr` is routed as its literal is — Python AOT.

Whatever value a `targetexpr` yields is routed as the same value written in
`target` is, at once or after a delay: `#_internal`, `#_parent`,
`#_<invokeid>` and a child's `_event.origin` each reach what they name, a
session this processor cannot reach raises error.communication, and a value
the SCXML Event I/O Processor does not support raises error.execution.

Fixture: ``integration_resources/a_target_expression_is_routed_as_its_literal_is/a_target_expression_is_routed_as_its_literal_is.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_a_target_expression_is_routed_as_its_literal_is_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import a_target_expression_is_routed_as_its_literal_is_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.ATargetExpressionIsRoutedAsItsLiteralIsState


def test_a_target_expression_is_routed_as_its_literal_is() -> None:
    engine = _sm.create_engine()
    engine.initialize()
    elapsed = 0
    while not engine.reached_final and elapsed < 1000:
        engine.advance_time(10)
        elapsed += 10

    p = engine.policy
    assert engine.terminal_state == _State.DONE, "the run must end in `done`"
    observed = {
        "internalNow": (p.internal_now(), 1),
        "internalLater": (p.internal_later(), 1),
        "kidNow": (p.kid_now(), 1),
        "kidLater": (p.kid_later(), 1),
        "sessNow": (p.sess_now(), 1),
        "sessLater": (p.sess_later(), 1),
        "commErrors": (p.comm_errors(), 4),
        "execErrors": (p.exec_errors(), 2),
        "afterStranger": (p.after_stranger(), 0),
        "afterStrangerLater": (p.after_stranger_later(), 0),
        "afterOrphan": (p.after_orphan(), 0),
        "afterOrphanLater": (p.after_orphan_later(), 0),
        "afterBogus": (p.after_bogus(), 0),
        "afterBogusLater": (p.after_bogus_later(), 0),
    }
    wrong = {name: got for name, (got, want) in observed.items() if got != want}
    assert not wrong, f"observed {wrong}, want { {n: w for n, (_, w) in observed.items()} }"
