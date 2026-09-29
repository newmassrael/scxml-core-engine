# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 4.9: an error ends the block it was raised in, and no other — Python AOT.

Fixture: ``integration_resources/an_error_ends_the_block_it_was_raised_in/an_error_ends_the_block_it_was_raised_in.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_an_error_ends_the_block_it_was_raised_in_python.sh`` (local)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import an_error_ends_the_block_it_was_raised_in_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.AnErrorEndsTheBlockItWasRaisedInState
_Event = _sm.AnErrorEndsTheBlockItWasRaisedInEvent


def test_a_root_start_of_a_machine_that_needs_no_parent_runs() -> None:
    """The control for the refused root start: this document never sends to
    `#_parent`, so a root-start policy lets it start, and the checked start
    runs it as `initialize` would."""
    engine = _sm.create_engine()
    assert engine.root_start_refusal() is None
    assert engine.initialize_as_root() is None
    assert engine.is_running, "a root start that is not refused must start the machine"


def test_each_error_ends_only_its_own_block() -> None:
    engine = _sm.create_engine()
    engine.initialize()
    for event in (_Event.T, _Event.FINISH):
        engine.send_event(event)

    p = engine.policy
    assert engine.terminal_state == _State.DONE, "`finish` must carry the run to `done`"
    observed = {
        "errors": (p.errors(), 10),
        "afterAssign": (p.after_assign(), 0),
        "afterScript": (p.after_script(), 0),
        "afterLog": (p.after_log(), 0),
        "afterCancel": (p.after_cancel(), 0),
        "afterIfInner": (p.after_if_inner(), 0),
        "afterIf": (p.after_if(), 0),
        "afterSingle": (p.after_single(), 0),
        "afterTrans": (p.after_trans(), 0),
        "initRan": (p.init_ran(), 1),
        "pairs": (p.pairs(), 4),
        "sum": (p.sum(), 90),
        "ifThen": (p.if_then(), 0),
        "ifElse": (p.if_else(), 1),
        "afterIfCond": (p.after_if_cond(), 0),
        "elseifThen": (p.elseif_then(), 0),
        "elseifElse": (p.elseif_else(), 1),
        "afterElseifCond": (p.after_elseif_cond(), 0),
        "afterNestedIf": (p.after_nested_if(), 0),
        "afterOuterIf": (p.after_outer_if(), 0),
    }
    wrong = {name: got for name, (got, want) in observed.items() if got != want}
    assert not wrong, f"observed {wrong}, want { {n: w for n, (_, w) in observed.items()} }"
