# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The trace port: what a run tells a test of each step Appendix D takes, so the
run can be judged a behaviour of the document's model (docs/adr/0115, D4 and
D8).

A step is the start of the interpreter, a microstep on eventless transitions,
the taking of an internal or an external event (with the microstep it selects,
or none), or the exit of the interpreter: the steps `docs/formal/AppendixD.tla`
names `Start`, `Eventless`, `Internal`, `External` and `Stop`. Each is told
once it is over, with what its record holds there: the transitions it took by
their rows, the states it exited in the order it exited them, the states it
entered in the order it entered them, and the configuration it left. The
macrosteps are the runs of steps from a start or an external event to the next
one.

What the port is told is written in the one run trace format every lane writes
(`RunTrace.to_json`, `tests/forge/conformance/run_trace.schema.json`, read by
`sce-build`'s `run_trace`), and `sce-codegen formal traces` judges the file
against the model it names. This module is the Python twin of the Rust
runtime's `trace_port` module and of the C11 runtime's `sce/trace_port.h` and
`sce/trace_json.h`, and writes the same bytes for the same run but its lane.

The port is empty unless a test sets one (`Engine.set_trace_port`): with none
set, the engine opens no step and keeps nothing, and each step asks one
attribute whether a port is set, as it asks of the observer. A port reads and is
never asked anything, so a machine with a port set takes the steps one without
takes.

An event from outside, and a delayed send that comes due, are steps of the
environment and not of the interpreter (AppendixD's `Arrive` and `Due`): the
engine cannot tell where an event on its external queue came from, and the
external step that takes it names it.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from enum import Enum
from typing import Callable, List, Optional, Sequence

# The version of the run trace format `RunTrace.to_json` writes, the
# `TRACE_FORMAT_VERSION` its reader in `sce-build` checks.
TRACE_FORMAT_VERSION = 1

# The lane this runtime is, as a run trace names it.
LANE = "python"


class StepKind(Enum):
    """Which step of Appendix D a `TraceStep` is; its value is the step's name
    in the run trace format."""

    # AppendixD's `Start`: the entry into the initial configuration.
    START = "start"
    # AppendixD's `Eventless`: a microstep on eventless transitions.
    EVENTLESS = "eventless"
    # AppendixD's `Internal`: an event taken off the internal queue, and the
    # microstep it selects, if it selects one.
    INTERNAL = "internal"
    # AppendixD's `External`: an event taken off the external queue, and the
    # microstep it selects, if it selects one.
    EXTERNAL = "external"
    # AppendixD's `Stop`: every state still active exited.
    STOP = "stop"


@dataclass(frozen=True)
class Fired:
    """A transition a step took, by the pair its row of the emitted transition
    table is named by (docs/adr/0089): the state it is written in, by its id,
    and its position among that state's own transitions in document order,
    from 0."""

    source: str
    index: int


@dataclass
class TraceStep:
    """One step a run took, as the port is told it.

    Each kind says what the run trace format asks of it and no more: a start
    what it entered and the configuration; a microstep, eventless or on an
    event, what it took, exited and entered and the configuration; an event
    step the event too; a stop what it exited and the top-level final state the
    run ended in. What a kind does not say is empty.

    `event` is the name `_event.name` holds while the event is taken: the whole
    name it was raised or sent under, which is not the name of the event of the
    document it matched when a door resolved a longer name, or one the document
    never writes, to it. `configuration` is in document order, and empty for a
    stop; `outcome` is the top-level `<final>` a stop ended the run in.
    """

    kind: StepKind
    event: Optional[str] = None
    fired: List[Fired] = field(default_factory=list)
    exited: List[str] = field(default_factory=list)
    entered: List[str] = field(default_factory=list)
    configuration: List[str] = field(default_factory=list)
    outcome: Optional[str] = None

    def to_json(self) -> str:
        """The step as one object of the run trace format: the fields its kind
        says, in the order the format lists them."""
        parts = ['{"kind": ' + _text(self.kind.value)]
        if self.kind is StepKind.START:
            parts.append(_names("entered", self.entered))
            parts.append(_names("configuration", self.configuration))
        elif self.kind is StepKind.STOP:
            parts.append(_names("exited", self.exited))
            outcome = "null" if self.outcome is None else _text(self.outcome)
            parts.append('"outcome": ' + outcome)
        else:
            if self.kind is not StepKind.EVENTLESS:
                parts.append('"event": ' + _text(self.event or ""))
            fired = ", ".join(
                '{"source": ' + _text(taken.source) + ', "index": ' + str(taken.index) + "}"
                for taken in self.fired
            )
            parts.append('"fired": [' + fired + "]")
            parts.append(_names("exited", self.exited))
            parts.append(_names("entered", self.entered))
            parts.append(_names("configuration", self.configuration))
        return ", ".join(parts) + "}"


def _text(text: str) -> str:
    """`text` as a JSON string, in the one form every engine writes
    (`tests/json_text/string_escape.json`, to which
    `backends/python/tests/json_text/test_string_escape.py` holds these
    arguments)."""
    return json.dumps(text, ensure_ascii=False)


def _names(name: str, names: Sequence[str]) -> str:
    return _text(name) + ": [" + ", ".join(_text(state) for state in names) + "]"


class RunTrace:
    """The steps of one run, kept as the port is told them, and written in the
    run trace format. Its `push` is a port."""

    def __init__(self) -> None:
        self._steps: List[TraceStep] = []

    def push(self, step: TraceStep) -> None:
        """Keep `step`, after the steps kept before it."""
        self._steps.append(step)

    @property
    def steps(self) -> Sequence[TraceStep]:
        """The steps kept, in the order the run took them."""
        return tuple(self._steps)

    def to_json(self) -> str:
        """The run in the run trace format: its version, its lane and its steps,
        one step to a line."""
        head = (
            "{\n"
            f'  "version": {TRACE_FORMAT_VERSION},\n'
            f'  "lane": {_text(LANE)},\n'
            '  "steps": ['
        )
        if not self._steps:
            return head + "]\n}\n"
        body = ",\n    ".join(step.to_json() for step in self._steps)
        return head + "\n    " + body + "\n  ]\n}\n"


# What the engine holds for a port a test set: any callable that takes a step.
TracePort = Callable[[TraceStep], None]


class Recorder:
    """The port a test set, and the step being told.

    A step is opened where Appendix D begins it and closed where it ends; the
    states exited and entered between are added to it as the engine exits and
    enters each. What is added while no step is open is kept nowhere. The step
    handed to the port is its own to keep: the recorder opens a new one for the
    next step.
    """

    __slots__ = ("_port", "_step")

    def __init__(self, port: TracePort) -> None:
        self._port = port
        self._step: Optional[TraceStep] = None

    @property
    def is_open(self) -> bool:
        """Whether a step is being told."""
        return self._step is not None

    def begin(self, kind: StepKind, event: Optional[str] = None) -> None:
        """Begin a step of `kind`, which took the event named `event` when it
        is an event step."""
        self._step = TraceStep(kind, event)

    def fired(self, source: str, index: int) -> None:
        """The step's microstep takes the transition written `index`-th in
        `source`."""
        if self._step is not None:
            self._step.fired.append(Fired(source, index))

    def exited(self, state: str) -> None:
        """The step exits `state`."""
        if self._step is not None:
            self._step.exited.append(state)

    def entered(self, state: str) -> None:
        """The step enters `state`."""
        if self._step is not None:
            self._step.entered.append(state)

    def end(self, configuration: List[str], outcome: Optional[str]) -> None:
        """The step is over: tell the port it, with the configuration it left,
        or, for a stop, the top-level `<final>` the run ended in."""
        step = self._step
        if step is None:
            return
        self._step = None
        if step.kind is StepKind.STOP:
            step.outcome = outcome
        else:
            step.configuration = configuration
        self._port(step)
