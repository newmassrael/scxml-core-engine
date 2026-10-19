# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""docs/adr/0038, decision 3: a host states in its own type whether it performs
the duties of a machine, and a machine that states a duty refuses a host that
only simulates them unless it is built for one on purpose -- Python.

``static_host_duty.scxml`` states the duty ``start_keepalive``, released by
``stop_keepalive`` on the ``<onexit>`` of ``engaged``, and the notice
``note_engaged``. Its host protocol carries ``realization``; ``create_engine`` and
the policy raise ``ValueError`` for a host that does not perform, and take
``accept_simulated_host=True`` as the caller saying that it means to.
``static_host_call.scxml`` states no duty, so its host declares no realization and
is taken as it always was.

The Go and Kotlin halves are ``a_host_states_whether_it_performs_test.go`` and
``StaticDatamodelTest.kt``. The machines are the ones
``scripts/regen_static_datamodel_python.sh`` generates.
"""
from __future__ import annotations

import importlib
import sys
from pathlib import Path

import pytest

_HERE = Path(__file__).resolve().parent
# The tests directory, so this package is importable by the name it has there.
sys.path.insert(0, str(_HERE.parents[1]))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))
sys.path.insert(0, str(_HERE.parents[2] / "forge-runtime"))

from sce_runtime import ActionRealization  # noqa: E402 -- path inserted above


class _DutyHost:
    """A host of ``static_host_duty`` that says what it does with the duties, in
    its own type, and keeps the names of the calls it was given."""

    def __init__(self, realization) -> None:
        self.realization = realization
        self.calls: list[str] = []

    def start_keepalive(self) -> None:
        self.calls.append("start_keepalive")

    def note_engaged(self) -> None:
        self.calls.append("note_engaged")

    def stop_keepalive(self) -> None:
        self.calls.append("stop_keepalive")


class _SilentHost:
    """A host of ``static_host_duty`` that declares nothing about what it does."""

    def start_keepalive(self) -> None:
        pass

    def note_engaged(self) -> None:
        pass

    def stop_keepalive(self) -> None:
        pass


class _NoRealizationHost:
    """A host of ``static_host_call``, a machine with no duty: its protocol has no
    ``realization``, so this host has none."""

    def __init__(self) -> None:
        self.calls: list[str] = []

    def show_attempts(self, count: int, exhausted: bool) -> None:
        self.calls.append(f"{count},{str(exhausted).lower()}")


def _module(machine: str):
    return importlib.import_module(f"integration.static_datamodel.{machine}_sm")


def _raise_external(engine, name: str) -> None:
    engine.send_event(engine.policy.get_event_from_name(name))


def _engage_and_release(engine) -> None:
    engine.initialize()
    _raise_external(engine, "engage")
    _raise_external(engine, "release")


_TAKEN_AND_GIVEN_BACK = ["start_keepalive", "note_engaged", "stop_keepalive"]


def test_a_host_that_performs_the_duties_is_accepted_and_called() -> None:
    host = _DutyHost(ActionRealization.PERFORMS)
    _engage_and_release(_module("static_host_duty").create_engine(host))
    assert host.calls == _TAKEN_AND_GIVEN_BACK, host.calls


def test_a_host_that_simulates_the_duties_is_refused_at_construction() -> None:
    host = _DutyHost(ActionRealization.SIMULATES)
    with pytest.raises(ValueError) as refusal:
        _module("static_host_duty").create_engine(host)
    message = str(refusal.value)
    assert "StaticHostDuty" in message, f"the message names the machine: {message}"
    assert "only simulates" in message, f"the message says the host only simulates: {message}"
    assert "accept_simulated_host" in message, f"the message names the opt-in: {message}"
    assert host.calls == [], "a refused host is never called"


def test_the_policy_refuses_a_host_that_simulates_as_create_engine_does() -> None:
    host = _DutyHost(ActionRealization.SIMULATES)
    with pytest.raises(ValueError):
        _module("static_host_duty").Policy(host)


def test_a_host_that_declares_no_realization_is_refused() -> None:
    with pytest.raises(ValueError):
        _module("static_host_duty").create_engine(_SilentHost())


def test_the_opt_in_accepts_a_host_that_simulates_the_duties() -> None:
    host = _DutyHost(ActionRealization.SIMULATES)
    engine = _module("static_host_duty").create_engine(host, accept_simulated_host=True)
    _engage_and_release(engine)
    assert host.calls == _TAKEN_AND_GIVEN_BACK, host.calls


def test_a_second_positional_argument_is_the_script_engine_and_not_the_opt_in() -> None:
    # The second positional argument is the script engine, as it is for every
    # machine; the statement that a simulated host is meant is by name alone.
    host = _DutyHost(ActionRealization.SIMULATES)
    with pytest.raises(ValueError):
        _module("static_host_duty").create_engine(host, None)


def test_a_machine_with_no_duty_takes_a_host_that_declares_no_realization() -> None:
    host = _NoRealizationHost()
    engine = _module("static_host_call").create_engine(host)
    engine.initialize()
    assert host.calls == ["0,false"], host.calls
