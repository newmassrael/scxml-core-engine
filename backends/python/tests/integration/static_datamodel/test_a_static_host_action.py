# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""SCE Accepted Subset §2.15, "Host actions": a ``<sce:action>`` of a
``datamodel="sce-static"`` machine takes the machine's own variables as typed
arguments, each read when the call is made -- Python.

``static_host_call.scxml`` calls ``showAttempts(count, exhausted)`` from
``<onentry>``, where no event is in scope, so a host recording what it was asked
sees one call per entry of ``idle`` with the datamodel as it stood.

``static_host_call_arguments.scxml`` adds the failure: ``level`` is a ``uint8`` at
250, ``fine`` hands the host ``level + 1`` (251, which fits) and ``overflow``
``level + 10`` (260, which does not). A checked operation that overflows is a
failure and not a wrapped value (SCE_FORGE.md §3.4.1), so the host is NOT called
with 4 and ``error.execution`` is raised in the call's place, which the machine
counts.

The Rust, Kotlin, Go and C++ halves are ``StaticDatamodelTest.kt``,
``static_datamodel.rs``, ``static_scenarios_test.go`` and
``AStaticDatamodelRunsGeneratedCppTest.cpp``. The machines are the ones
``scripts/regen_static_datamodel_python.sh`` generates.
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


class _RecordingHost:
    """What a machine asked of its host, as ``(value...)`` text."""

    def __init__(self) -> None:
        self.calls: list[str] = []

    def show_attempts(self, count: int, exhausted: bool) -> None:
        self.calls.append(f"{count},{str(exhausted).lower()}")

    def report(self, next: int) -> None:  # noqa: A002 -- the host method's own name
        self.calls.append(str(next))


def _engine(machine: str, host: _RecordingHost):
    module = importlib.import_module(f"integration.static_datamodel.{machine}_sm")
    engine = module.create_engine(host)
    engine.initialize()
    return engine


def _raise_external(engine, name: str) -> None:
    engine.send_event(engine.policy.get_event_from_name(name))


def test_a_host_action_takes_typed_datamodel_arguments() -> None:
    host = _RecordingHost()
    engine = _engine("static_host_call", host)
    for _ in range(4):
        _raise_external(engine, "retry")
    # The fourth retry finds `attempts < 3` false and re-enters nothing.
    assert host.calls == ["0,false", "1,false", "2,false", "3,true"], host.calls


def test_an_argument_that_overflows_stops_the_call_and_raises_an_error() -> None:
    host = _RecordingHost()
    engine = _engine("static_host_call_arguments", host)

    _raise_external(engine, "fine")
    assert host.calls == ["251"], f"250 + 1 fits a uint8: the host heard {host.calls}"
    assert engine.policy.errors() == 0

    _raise_external(engine, "overflow")
    assert host.calls == ["251"], (
        f"250 + 10 does not fit, so the host is not called with 4: it heard {host.calls}"
    )
    assert engine.policy.errors() == 1, "error.execution was raised and the machine saw it"
