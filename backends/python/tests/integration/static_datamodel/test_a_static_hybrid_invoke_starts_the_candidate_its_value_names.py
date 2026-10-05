# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""SCE Accepted Subset §2.13, "Hybrid `<invoke>`": an `<invoke srcexpr>` that
declares `sce:candidates` starts the document its value names (W3C SCXML 6.4),
by the document's stem, and hands it the invoke's arguments, each to the
variable of the same name that candidate declares (W3C SCXML 6.4.3) -- Python.

``static_invoke_hybrid.scxml`` runs four phases, each in a state of its own:

  first    `file:static_hybrid_first.scxml`: handed start = 7 and enabled =
           true, and ends. `extra` is evaluated and left out, the candidate
           declaring none.
  second   an absolute path to `static_hybrid_second.scxml`: handed start = 7
           and extra = 3, and ends. `enabled` is left out.
  lossy    `./static_hybrid_first.scxml` with an `extra` no 32-bit field can
           hold: reported as `error.execution` and left out, and the child
           still starts and ends.
  missing  a document the invoke did not declare: `error.execution`, and no
           child starts, so no `done.invoke` follows.

A candidate handed what the OTHER one declares would never end, and the run
would stop short of `over`. The Rust, Kotlin and Go halves are
`a_static_hybrid_invoke_starts_the_candidate_its_value_names.rs`,
`AStaticHybridInvokeStartsTheCandidateItsValueNamesTest.kt` and
`a_static_hybrid_invoke_starts_the_candidate_its_value_names_test.go`.

The machines are the ones ``scripts/regen_static_datamodel_python.sh``
generates; a candidate is a module beside its parent.
"""
from __future__ import annotations

import importlib
import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
# The tests directory, so this package is importable by the name it has there.
sys.path.insert(0, str(_HERE.parents[1]))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))
sys.path.insert(0, str(_HERE.parents[2] / "forge-runtime"))


def _settled():
    """The machine, started, with its children run and reported: the engine runs
    synchronously, and a child that ends during its own `initialize` has reported
    by the time the parent's start returns, so the whole run is over when
    `initialize` is."""
    module = importlib.import_module("integration.static_datamodel.static_invoke_hybrid_sm")
    engine = module.create_engine()
    engine.initialize()
    return engine


def test_each_phase_starts_the_candidate_its_value_names_and_ends() -> None:
    engine = _settled()
    # Each phase's `done.invoke` adds a power of ten of its own, so the sum says
    # WHICH candidates ended: `first` (1), `second` (10) and the retry of
    # `first` that carried an argument it could not hold (100).
    assert engine.policy.completed() == 111
    assert engine.reached_final, "the last phase named no declared candidate, so the run is over"


def test_an_argument_is_evaluated_whatever_the_candidate_keeps() -> None:
    engine = _settled()
    # `lossy` hands an `extra` that overflows to a candidate that declares none
    # (one error), and `missing` names no declared candidate (the other).
    assert engine.policy.errors() == 2


def test_a_value_naming_no_declared_candidate_starts_nothing() -> None:
    engine = _settled()
    # `done.invoke.missing_run` would add 1000: nothing started to send it.
    assert engine.policy.completed() < 1000
