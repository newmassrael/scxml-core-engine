# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.4.1 + 6.4.3: a hybrid invoke carries its arguments — Python AOT.

An <invoke> whose child is named by an expression carries its arguments as one
whose child is fixed does. The value always names ``keeper``, and ``bare``
declares the one name ``keeper`` does not, so a pair seeded by the wrong
candidate's declarations is a leak ``keeper`` reports rather than an absence
the test has to infer.

Fixture: ``integration_resources/a_hybrid_invoke_carries_its_arguments/a_hybrid_invoke_carries_its_arguments.scxml``.

Regeneration:
  ``scripts/regen_a_hybrid_invoke_carries_its_arguments_python.sh`` (local)
  ``sce-codegen generate-integration -l python --stem a_hybrid_invoke_carries_its_arguments`` (CI)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import a_hybrid_invoke_carries_its_arguments_sm as _sm  # noqa: E402 — path inserted above

_State = _sm.AHybridInvokeCarriesItsArgumentsState


def test_each_argument_reaches_only_the_child_that_declares_it() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    elapsed = 0
    while not engine.reached_final and elapsed < 300:
        engine.advance_time(10)
        elapsed += 10

    assert engine.reached_final, (
        "the machine never completed; last leaf="
        f"{engine.current_state!s}. `refusedPhase` parks when its invoke raised nothing"
    )
    assert engine.terminal_state == _State.DONE, (
        "`failWrongChild` means `bare` ran; `failRefusedChildStarted` means an "
        f"unreadable namelist still started a child; reached {engine.terminal_state!s}"
    )
    p = engine.policy
    observed = {
        "errors": (p.errors(), 2),
        "started": (p.started(), 2),
        "paramsOk": (p.params_ok(), 1),
        "namelistOk": (p.namelist_ok(), 1),
    }
    wrong = {name: got for name, (got, want) in observed.items() if got != want}
    assert not wrong, f"observed {wrong}, want { {n: w for n, (_, w) in observed.items()} }"
