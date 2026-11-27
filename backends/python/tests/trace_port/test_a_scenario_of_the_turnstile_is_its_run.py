# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""Each scenario of the properties judge's turnstile, replayed by the Python
lane (docs/adr/0115, D15 and the converse of D8).

A scenario is the run TLC gave of a violation or a witness, with the events its
environment put on the external queue; this test gives the machine those events,
on the engine's virtual clock, with its trace port on, and holds the run the port
is told to the recorded run ``<name>.python.json`` beside the scenario, byte for
byte. ``sce_build::formal::replay`` judges each recorded run to be the
scenario's, step for step (``sce-build/tests/a_scenario_is_the_run_of_every_lane.rs``),
as it judges the Rust and the C11 lanes'. The Python twin of the Rust lane's
``a_scenario_of_the_turnstile_is_its_run`` and of the C11 lane's
``c11_a_scenario_of_the_turnstile_is_its_run``.

The harness gives an input once the machine has taken every step it can, which
is after the step the input names unless the model's environment put it on the
queue in the middle of a macrostep. A macrostep that also sends to its own
external queue after that point would take the two in another order here; the
judge then finds the runs part, which is a defect of this harness to settle, and
not a verdict about the document. A delayed send comes due by the clock moving
to the next deadline, which is the earliest: a scenario in which the model's
time makes a later one due first is one no clock replays, and the judge finds
that too.
That move makes every send of the deadline due at once, where the model may
make them due one at a time with steps between: a `due` input that finds no
send pending was given by the move before it, and gives nothing. Sends of one
deadline the model makes due around another event are a run no clock replays
either.

The machine is the formal fixture's (the python-integration tree's directory
``integration/formal_turnstile``); the scenarios are written by
``sce-codegen formal properties --scenarios
sce-build/tests/fixtures/formal_properties/scenarios``. Recording the lane's
runs again, after a change that moves a run on purpose:
  UPDATE_EXPECT=1 python -m pytest backends/python/tests/trace_port/test_a_scenario_of_the_turnstile_is_its_run.py

Fixture: ``sce-build/tests/fixtures/formal_properties/turnstile.scxml``
"""
from __future__ import annotations

import importlib
import json
import os
import sys
from dataclasses import dataclass
from pathlib import Path
from types import ModuleType
from typing import List

_TESTS = Path(__file__).resolve().parents[1]
_REPO_ROOT = Path(__file__).resolve().parents[4]
_SCENARIOS = _REPO_ROOT / "sce-build" / "tests" / "fixtures" / "formal_properties" / "scenarios"
sys.path.insert(0, str(_TESTS.parent / "runtime"))

from sce_runtime.trace_port import RunTrace  # noqa: E402

# What a scenario's file is named by after its name
# (``sce_build::formal::replay::SCENARIO_SUFFIX``).
_SCENARIO_SUFFIX = ".scenario.json"

# The members of an input of the run trace form, each once and no other.
_INPUT_MEMBERS = {"kind", "after", "event"}


@dataclass(frozen=True)
class _Input:
    """One event the environment put on the external queue during a scenario:
    ``arrive``, from outside, or ``due``, a delayed send come due, ``after``
    the count of steps of the run that were over when it came, from 1."""

    kind: str
    after: int
    event: str


def _turnstile() -> ModuleType:
    """The generated machine of the turnstile, imported as the generated
    modules beside it import one another."""
    directory = _TESTS / "integration" / "formal_turnstile"
    source = directory / "turnstile_sm.py"
    assert source.is_file(), (
        f"{source} is not there: generate it with `scripts/materialize.sh python-integration`"
    )
    if str(directory) not in sys.path:
        sys.path.insert(0, str(directory))
    return importlib.import_module("turnstile_sm")


def _scenario_names() -> List[str]:
    """Each scenario of the turnstile, by its name, in the order of the names."""
    assert _SCENARIOS.is_dir(), (
        f"{_SCENARIOS} is no directory: write the scenarios with "
        "`sce-codegen formal properties --scenarios`"
    )
    return sorted(
        path.name[: -len(_SCENARIO_SUFFIX)]
        for path in _SCENARIOS.iterdir()
        if path.name.endswith(_SCENARIO_SUFFIX) and len(path.name) > len(_SCENARIO_SUFFIX)
    )


def _inputs(name: str, text: str) -> List[_Input]:
    """The inputs of the scenario ``text``, which ``sce_build::run_trace`` reads
    of reference: every other member of the document is read there and stepped
    over here. An input with a member the form does not have, or without one it
    has, is refused rather than replayed in part."""
    document = json.loads(text)
    assert isinstance(document, dict), f"{name}: the scenario is no JSON object"
    listed = document.get("inputs", [])
    assert isinstance(listed, list), f"{name}: `inputs` is no array"
    inputs: List[_Input] = []
    for at, read in enumerate(listed):
        assert isinstance(read, dict) and set(read) == _INPUT_MEMBERS, (
            f"{name}: inputs[{at}] is not an object of a `kind`, an `after` and an `event`"
        )
        kind, after, event = read["kind"], read["after"], read["event"]
        assert kind in ("arrive", "due"), (
            f"{name}: inputs[{at}] is of the kind {kind!r}, which the form does not have"
        )
        assert isinstance(after, int) and not isinstance(after, bool) and after >= 1, (
            f"{name}: inputs[{at}] comes after {after!r}, which is no count of steps from 1"
        )
        assert isinstance(event, str) and event, f"{name}: inputs[{at}] names no event"
        inputs.append(_Input(kind, after, event))
    return inputs


def _replay(machine: ModuleType, name: str, inputs: List[_Input]) -> RunTrace:
    """The run the trace port is told when the turnstile is given ``inputs``."""
    trace = RunTrace()
    engine = machine.create_engine()
    engine.set_trace_port(trace.push)
    engine.initialize()
    for given in inputs:
        told = len(trace.steps)
        assert told >= given.after, (
            f"{name}: the machine took {told} steps, and the model {given.after} "
            f"before `{given.event}` came"
        )
        if given.kind == "arrive":
            engine.send_external_by_name(given.event)
            # The macrostep the event starts, run to its end.
            engine.advance_time(0)
        else:
            # None pending: the send came due with an earlier one of its
            # deadline, and the steps compared say whether it did.
            wait = engine.time_until_next_scheduled_ms()
            if wait is not None:
                engine.advance_time(wait)
    return trace


def _holds_to_the_recorded_run(path: Path, trace: RunTrace) -> None:
    """Hold ``trace`` to the recorded run at ``path``, or record it again when
    ``UPDATE_EXPECT`` is set."""
    text = trace.to_json()
    if os.environ.get("UPDATE_EXPECT"):
        path.write_bytes(text.encode("utf-8"))
        return
    assert path.is_file(), (
        f"{path} cannot be read: record it with `UPDATE_EXPECT=1 python -m pytest "
        "backends/python/tests/trace_port/test_a_scenario_of_the_turnstile_is_its_run.py`"
    )
    recorded = path.read_bytes().decode("utf-8")
    assert text == recorded, f"the run of {path} is not the recorded one the replay judge reads"


def test_each_scenario_of_the_turnstile_is_its_run_on_the_python_lane() -> None:
    names = _scenario_names()
    assert names, (
        f"{_SCENARIOS} holds no scenario: write them with `sce-codegen formal properties --scenarios`"
    )
    machine = _turnstile()
    for name in names:
        text = (_SCENARIOS / f"{name}{_SCENARIO_SUFFIX}").read_text(encoding="utf-8")
        trace = _replay(machine, name, _inputs(name, text))
        _holds_to_the_recorded_run(_SCENARIOS / f"{name}.python.json", trace)
