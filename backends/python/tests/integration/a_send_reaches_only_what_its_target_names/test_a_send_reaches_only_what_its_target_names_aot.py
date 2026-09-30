# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.2 + 6.4 + C.1: a <send> reaches what its target names — Python AOT.

A type the platform does not support is refused before the payload is read;
`#_<invokeid>` and `#_scxml_<sessionid>` that name nothing reachable raise
error.communication and end the block; the bare `#_scxml_` is this session's
own queue; and a `<content expr>` crosses to the child and back as a value.

Fixture: ``integration_resources/a_send_reaches_only_what_its_target_names/a_send_reaches_only_what_its_target_names.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_a_send_reaches_only_what_its_target_names_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import a_send_reaches_only_what_its_target_names_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.ASendReachesOnlyWhatItsTargetNamesState


def test_a_root_start_of_a_machine_that_needs_a_parent_is_refused() -> None:
    """W3C SCXML 6.2.4: this document sends to `#_parent`, so a host that runs
    machines as roots and asks for the refusal gets it — and nothing starts.
    The plain `initialize` below runs the same machine; the refusal is opt-in."""
    from sce_runtime import RootStartRefusal

    engine = _sm.create_engine()
    assert engine.root_start_refusal() is RootStartRefusal.NEEDS_PARENT
    assert engine.initialize_as_root() is RootStartRefusal.NEEDS_PARENT
    assert not engine.is_running, "a refused root start must start nothing"
    assert RootStartRefusal.NEEDS_PARENT.reason == (
        "the machine sends to #_parent and was started with no parent session"
    )


def test_a_send_reaches_only_what_its_target_names() -> None:
    engine = _sm.create_engine()
    engine.initialize()
    elapsed = 0
    while not engine.reached_final and elapsed < 300:
        engine.advance_time(10)
        elapsed += 10

    p = engine.policy
    assert engine.terminal_state == _State.DONE, "the run must end in `done`"
    observed = {
        "execErrors": (p.exec_errors(), 1),
        "commErrors": (p.comm_errors(), 5),
        "afterRefused": (p.after_refused(), 0),
        "afterNobody": (p.after_nobody(), 0),
        "afterStranger": (p.after_stranger(), 0),
        "afterOrphan": (p.after_orphan(), 0),
        "afterOrphanExpr": (p.after_orphan_expr(), 0),
        "afterOrphanExprLater": (p.after_orphan_expr_later(), 0),
        "bareArrived": (p.bare_arrived(), 1),
        "pongOk": (p.pong_ok(), 1),
    }
    wrong = {name: got for name, (got, want) in observed.items() if got != want}
    assert not wrong, f"observed {wrong}, want { {n: w for n, (_, w) in observed.items()} }"
