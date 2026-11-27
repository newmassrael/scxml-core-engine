# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The trace port's writer and recorder (docs/adr/0115, D4 and D8), asked of
``sce_runtime.trace_port`` directly.

The twins of the cases of the Rust runtime's ``trace_port`` module and of the
C11 ``c11_unit_trace_port``, with the same steps and the same bytes but the
lane, so the three writers are held to the one run trace format
(``tests/forge/conformance/run_trace.schema.json``) that ``sce-codegen formal
traces`` reads.

This directory is deliberately NOT a fixture stem: these cases hold the writer
to hand-worked answers over no document, and the runs beside them drive
documents other directories generate. ``scripts/gates/w3c-python.sh`` names it
explicitly for that reason.
"""
from __future__ import annotations

import sys
from pathlib import Path
from typing import List

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "runtime"))

from sce_runtime.trace_port import (  # noqa: E402
    LANE,
    TRACE_FORMAT_VERSION,
    Fired,
    Recorder,
    RunTrace,
    StepKind,
    TraceStep,
)


def _step(kind: StepKind) -> TraceStep:
    return TraceStep(
        kind=kind,
        event="go.on",
        fired=[Fired("s0", 2)],
        exited=["s01", "s0"],
        entered=["s1"],
        configuration=["s1"],
        outcome="pass",
    )


def test_the_lane_and_the_version_are_the_formats() -> None:
    assert LANE == "python"
    assert TRACE_FORMAT_VERSION == 1


def test_each_kind_writes_the_fields_the_format_asks_of_it() -> None:
    assert _step(StepKind.START).to_json() == (
        '{"kind": "start", "entered": ["s1"], "configuration": ["s1"]}'
    )
    assert _step(StepKind.EVENTLESS).to_json() == (
        '{"kind": "eventless", "fired": [{"source": "s0", "index": 2}], '
        '"exited": ["s01", "s0"], "entered": ["s1"], "configuration": ["s1"]}'
    )
    assert _step(StepKind.INTERNAL).to_json() == (
        '{"kind": "internal", "event": "go.on", "fired": [{"source": "s0", "index": 2}], '
        '"exited": ["s01", "s0"], "entered": ["s1"], "configuration": ["s1"]}'
    )
    assert _step(StepKind.EXTERNAL).to_json() == (
        '{"kind": "external", "event": "go.on", "fired": [{"source": "s0", "index": 2}], '
        '"exited": ["s01", "s0"], "entered": ["s1"], "configuration": ["s1"]}'
    )
    assert _step(StepKind.STOP).to_json() == (
        '{"kind": "stop", "exited": ["s01", "s0"], "outcome": "pass"}'
    )
    unended = _step(StepKind.STOP)
    unended.outcome = None
    assert unended.to_json() == '{"kind": "stop", "exited": ["s01", "s0"], "outcome": null}'


def test_a_step_that_takes_nothing_writes_empty_lists() -> None:
    nothing = TraceStep(StepKind.INTERNAL, "e", configuration=["s0"])
    assert nothing.to_json() == (
        '{"kind": "internal", "event": "e", "fired": [], "exited": [], "entered": [], '
        '"configuration": ["s0"]}'
    )


def test_a_trace_is_written_with_its_version_and_lane_one_step_to_a_line() -> None:
    trace = RunTrace()
    assert trace.to_json() == '{\n  "version": 1,\n  "lane": "python",\n  "steps": []\n}\n'
    trace.push(_step(StepKind.START))
    trace.push(_step(StepKind.STOP))
    assert trace.to_json() == (
        '{\n  "version": 1,\n  "lane": "python",\n  "steps": [\n    '
        '{"kind": "start", "entered": ["s1"], "configuration": ["s1"]},\n    '
        '{"kind": "stop", "exited": ["s01", "s0"], "outcome": "pass"}\n  ]\n}\n'
    )
    assert len(trace.steps) == 2


def test_a_name_is_written_as_a_json_string() -> None:
    quoted = TraceStep(StepKind.START, entered=['a"b\\c'])
    assert quoted.to_json() == '{"kind": "start", "entered": ["a\\"b\\\\c"], "configuration": []}'


def test_a_recorder_tells_its_port_each_step_once_it_ends() -> None:
    """A recorder tells its port each step once the step ends, with what was
    added between, and the configuration only of a step that leaves one."""
    told: List[TraceStep] = []
    recorder = Recorder(told.append)

    recorder.begin(StepKind.INTERNAL, "e")
    assert recorder.is_open
    recorder.fired("s0", 1)
    recorder.exited("s0")
    recorder.entered("s1")
    assert told == [], "told only once it ends"
    recorder.end(["s1"], "ignored")
    assert not recorder.is_open

    recorder.begin(StepKind.STOP)
    recorder.exited("s1")
    recorder.end(["ignored"], "pass")

    assert told == [
        TraceStep(
            kind=StepKind.INTERNAL,
            event="e",
            fired=[Fired("s0", 1)],
            exited=["s0"],
            entered=["s1"],
            configuration=["s1"],
            outcome=None,
        ),
        TraceStep(kind=StepKind.STOP, exited=["s1"], outcome="pass"),
    ]


def test_a_recorder_with_no_step_open_keeps_nothing() -> None:
    """What the engine adds while no step is open is kept nowhere, and an end
    with no step open tells the port nothing."""
    told: List[TraceStep] = []
    recorder = Recorder(told.append)
    recorder.fired("s0", 0)
    recorder.exited("s0")
    recorder.entered("s1")
    recorder.end(["s1"], None)
    assert not recorder.is_open
    assert told == []

    recorder.begin(StepKind.START)
    recorder.entered("s1")
    recorder.end(["s1"], None)
    assert told == [TraceStep(StepKind.START, entered=["s1"], configuration=["s1"])]


def test_each_step_told_is_the_ports_to_keep() -> None:
    """The recorder opens a new step for each, so a step a port kept does not
    change as the run goes on."""
    told: List[TraceStep] = []
    recorder = Recorder(told.append)
    recorder.begin(StepKind.START)
    recorder.entered("s0")
    recorder.end(["s0"], None)
    recorder.begin(StepKind.EVENTLESS)
    recorder.fired("s0", 0)
    recorder.exited("s0")
    recorder.entered("s1")
    recorder.end(["s1"], None)
    assert told[0] == TraceStep(StepKind.START, entered=["s0"], configuration=["s0"])
    assert told[0] is not told[1]
