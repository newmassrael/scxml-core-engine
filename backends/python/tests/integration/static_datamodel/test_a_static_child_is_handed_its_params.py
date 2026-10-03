# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""SCE Accepted Subset §2.15, "Child sessions": an `<invoke type="scxml">` hands
its child the values its `<param>`s and `namelist` name (W3C SCXML 6.4.1), each
to the child's variable of the same name, of that variable's own type -- Python.

``static_invoke_params.scxml`` invokes `worker`, which ends the moment it holds
`start = 7` (from a `<param>` reading `base`) and `enabled = true` (from the
`namelist`); handed less, it would wait and the parent would stay in `working`.
`base` is 4 when `working` is entered and the entry action adds 3, so 7 arrives
only if the value is read when the invoke executes, at the end of the macrostep.
`control`, the same child handed nothing, keeps its declared defaults and never
ends.

The Rust, Kotlin and Go halves are `a_static_child_is_handed_its_params.rs`,
`AStaticChildIsHandedItsParamsTest.kt` and
`a_static_child_is_handed_its_params_test.go`.

The machines are the ones ``scripts/regen_static_datamodel_python.sh``
generates; a child is a module beside its parent.
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


def _started_parent():
    """The parent, started, with its children run and reported."""
    module = importlib.import_module("integration.static_datamodel.static_invoke_params_sm")
    engine = module.create_engine()
    engine.initialize()
    return module, engine


def _in_state(engine, module, name: str) -> bool:
    return getattr(module.State, name) in engine.active_leaves


def test_a_child_is_handed_the_values_its_invoke_names() -> None:
    module, engine = _started_parent()
    # `worker` ended, so it held both values: the parent left `working` and
    # counted it.
    assert _in_state(engine, module, "PLAIN"), (
        "the child was handed `start` and `enabled`, so it ended: the parent is "
        f"still in {engine.active_leaves}"
    )
    assert engine.policy.completed() == 1


def test_a_child_handed_nothing_keeps_the_values_its_data_gave_it() -> None:
    module, engine = _started_parent()
    # `control` is the same child handed nothing: it still waits for 7 and true,
    # so `done.invoke.control` never counted. `watcher` was handed 7 and waits
    # for 8.
    assert _in_state(engine, module, "PLAIN"), f"the parent is not in `plain`: {engine.active_leaves}"
    assert engine.policy.completed() == 1


def test_a_child_is_handed_its_values_once_when_it_starts() -> None:
    module, engine = _started_parent()
    # `bump` makes `base` 8 while `watcher` runs with the 7 it was handed when it
    # started: a child is handed its values once, so it does not end.
    engine.send_event(engine.policy.get_event_from_name("bump"))
    assert engine.policy.completed() == 1, "`watcher` saw the later value"
