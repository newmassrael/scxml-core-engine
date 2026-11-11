# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The recorded input trace of a test case and its replay
(docs/SCE_TTCN3_LANE.md §10; docs/SCE_TEST_PROFILE.md §3.2, §3.3, §12.4): the
inputs of one run from the system under test, each at its logical instant,
``schemas/sce-test-trace.v1.schema.json``, and the system adapter that plays
them, ``TraceAdapter``.

A trace is one text of the JSON Canonicalization Scheme, so that it has one
spelling and two traces are equal exactly when their bytes are:
``{"inputs":[{"at","port","type","value"}, …],"schema","suite","test_case"}``.
``at`` is ``NOW`` in microseconds as the text of its digits, ``port`` a system
port of the test case's configuration, and ``type`` and ``value`` the record
type's name and the value in the canonical form a message record of the test
log writes (§12.5). The inputs stand in order of ``at``; the inputs of one
instant in the order they are appended to their ports' queues (§3.2 step 2).

The value of an input is read by its type: the reader of the test case's
configuration (``InputReader``, generated as ``input`` in the configuration's
unit) knows which records each system port takes and reads each with the reader
of its record (``from_canonical``). A trace that does not fit is refused whole
before the run starts, which then ends with ``error`` (P13).

The text is read by ``sce_runtime.saved_json.parse_json``, not by the ``json``
module, which reads ``NaN`` and ``Infinity``, holds a number past a double as a
``float`` and keeps the last of two members of one name: none of those is a
trace's one canonical spelling, and each is refused here, the first by the
reader and the others as a text that is not the trace's canonical text.

Cross-language siblings: ``sce_rust_runtime::test::trace`` (Rust),
``com.sce.runtime.test.Trace`` (Kotlin), ``SCE::Test::Trace`` (C++), which read
and refuse the same texts in the same words.
"""

from __future__ import annotations

from collections import deque
from dataclasses import dataclass, field
from typing import Any, Callable, Deque, List, Optional, Sequence, Tuple

from ..saved_json import JsonError, NotCanonical, parse_json, write_jcs
from .from_canonical import INT64, decimal
from .port import Instant, Message
from .run import SystemInput
from .trace_refusal import TraceRefusal

#: The name of the trace's shape, its ``schema``.
TEST_TRACE_SCHEMA = "sce-test-trace.v1"

#: The stability status of the trace's wire schema, pinned to the schema file's
#: own ``x-sce-schema-status`` by a test of this runtime and by
#: ``schema_file_declares_status`` of the Rust one. ``SCE_WIRE_CONTRACTS.md``
#: requires one commit to move both.
TEST_TRACE_SCHEMA_STATUS = "pre-release"

#: The reader of the inputs of a test case's configuration: the message an input
#: of the record type ``type_name`` on the system port ``system_port`` carries,
#: its value read from the canonical form, called as
#: ``read(system_port, type_name, value)``. Raises a ``TraceRefusal`` naming
#: why, where the input is not one of the configuration's or its value does not
#: belong to its type.
#:
#: Generated as ``input`` in the unit of a test configuration, which names the
#: records each of its system ports takes.
InputReader = Callable[[str, str, Any], Message]


def no_inputs(system_port: str, type_name: str, value: Any) -> Message:
    """The reader of a configuration with no system port: it refuses every
    input, naming its port."""
    raise TraceRefusal.not_an_input(system_port, type_name, [])


@dataclass(frozen=True)
class TraceInput:
    """One input of a trace."""

    #: ``NOW`` when it is due.
    at: Instant
    #: The system port it arrives on.
    port: str
    #: The name of its record type, as the test log names it.
    message_type: str
    #: Its value, in the canonical form of the test log.
    value: Any


def _members(value: Any, names: Sequence[str], what: str) -> List[Any]:
    """The members of ``value``, an object whose members are ``names`` exactly,
    each in the order of ``names``; refused, naming ``what``, otherwise."""
    if not isinstance(value, dict):
        raise TraceRefusal(f"{what} is not an object")
    stray = next((name for name in value if name not in names), None)
    if stray is not None:
        raise TraceRefusal(f"{what} has no member '{stray}'")
    found = []
    for name in names:
        if name not in value:
            raise TraceRefusal(f"{what} has no '{name}'")
        found.append(value[name])
    return found


def _name_of(value: Any, what: str) -> str:
    """The text ``value``, which is not empty, at ``what``."""
    if not isinstance(value, str) or not value:
        raise TraceRefusal(f"{what} is not a name")
    return value


def _instant_of(value: Any, what: str) -> Instant:
    """An instant: the text of the decimal digits of a count of microseconds
    from the start of the test case, with no leading zero, within ``NOW``'s
    range, a 64-bit integer's."""
    count = decimal(value) if isinstance(value, str) else None
    if count is None or count < 0:
        raise TraceRefusal(f"{what} is not an instant, the text of the digits of a count of microseconds")
    if not INT64.holds(count):
        raise TraceRefusal(f"{what} ({value}) is past the microseconds the clock counts to")
    return count


@dataclass(frozen=True)
class Trace:
    """A recorded input trace of one test case of one suite."""

    #: The suite's name.
    suite: str
    #: The test case's name.
    test_case: str
    #: The inputs, in order of ``at``, those of one instant in the order they
    #: are appended to their ports.
    inputs: List[TraceInput] = field(default_factory=list)

    @classmethod
    def parse(cls, text: str) -> "Trace":
        """The trace ``text`` holds: the JSON Canonicalization Scheme of one
        trace, which a file may follow with one line feed, as a text file ends.

        Raises a ``TraceRefusal`` naming why, where the text is not JSON, the
        shape is not ``sce-test-trace.v1``, the inputs are out of the order of
        their instants, or the text is not the trace's one canonical spelling.
        Whether each input fits the suite is the configuration's to read
        (``TraceAdapter.replaying``).
        """
        body = text[:-1] if text.endswith("\n") else text
        try:
            value = parse_json(body)
        except JsonError as refused:
            raise TraceRefusal(f"the trace is {refused}") from None
        except RecursionError:
            # The reader descends once per array or object it opens, so a text
            # that nests past the interpreter's stack is refused by name rather
            # than ending the runner.
            raise TraceRefusal("the trace is not JSON (it nests deeper than the reader reads)") from None
        schema, suite, test_case, inputs = _members(value, ("schema", "suite", "test_case", "inputs"), "the trace")
        if schema != TEST_TRACE_SCHEMA:
            raise TraceRefusal(f"the trace's schema is not {TEST_TRACE_SCHEMA}")
        if not isinstance(inputs, list):
            raise TraceRefusal("the trace's inputs are not an array")
        read: List[TraceInput] = []
        for index, item in enumerate(inputs):
            what = f"input {index}"
            at, port, message_type, held = _members(item, ("at", "port", "type", "value"), what)
            instant = _instant_of(at, f"the instant of {what}")
            if read and read[-1].at > instant:
                raise TraceRefusal(
                    f"{what}, at {instant}, comes after an input at {read[-1].at}: the inputs stand in order of "
                    "their instants"
                )
            read.append(
                TraceInput(
                    at=instant,
                    port=_name_of(port, f"the port of {what}"),
                    message_type=_name_of(message_type, f"the type of {what}"),
                    value=held,
                )
            )
        trace = cls(
            suite=_name_of(suite, "the trace's suite"),
            test_case=_name_of(test_case, "the trace's test case"),
            inputs=read,
        )
        # A value with no canonical text, a number past the range of a double
        # among them, is a trace not written in it either: it is refused here
        # as one, never handed to `canonical_text`, which holds that a trace has
        # that text.
        try:
            canonical: Optional[str] = write_jcs(trace._canonical_value())
        except NotCanonical:
            canonical = None
        if canonical != body:
            raise TraceRefusal(
                "the trace is not written in its one canonical text, the JSON Canonicalization Scheme"
            )
        return trace

    def canonical_text(self) -> str:
        """The trace in its one canonical text, the JSON Canonicalization
        Scheme, with no line feed after it.

        Raises ``NotCanonical`` when a value holds a number past the range of a
        double, or a value that is no JSON value, which no canonical value holds
        and ``parse`` refuses.
        """
        return write_jcs(self._canonical_value())

    def _canonical_value(self) -> Any:
        """The trace as the one JSON value its canonical text writes."""
        return {
            "schema": TEST_TRACE_SCHEMA,
            "suite": self.suite,
            "test_case": self.test_case,
            "inputs": [
                {
                    "at": str(given.at),
                    "port": given.port,
                    "type": given.message_type,
                    "value": given.value,
                }
                for given in self.inputs
            ],
        }


@dataclass(frozen=True)
class SystemOutput:
    """What a component sent to a system port, as a ``TraceAdapter`` records
    it."""

    #: ``NOW`` when it was sent.
    at: Instant
    #: The system port it was sent to.
    system_port: str
    #: The message.
    message: Message


class TraceAdapter:
    """A system adapter that plays a trace (docs/SCE_TTCN3_LANE.md §10.5): at
    each instant it delivers the inputs due then, in the trace's order, to their
    system ports (docs/SCE_TEST_PROFILE.md §3.2 step 2), and it records what the
    components send to the system ports.

    It announces the instant of its next input (``next_input``), so when no
    branch can fire the run's clock advances to the earlier of that instant and
    the next timer expiry or the guard (§3.3, §12.4 step 4), and the run is the
    same whenever it is played. It is a ``SystemAdapter``.
    """

    __slots__ = ("_inputs", "_sent")

    def __init__(self, inputs: Sequence[Tuple[Instant, str, Message]]) -> None:
        """The player of ``inputs``, each the instant it is due, its system port
        and its message, already read: ``replaying`` reads them from a
        trace."""
        self._inputs: Deque[Tuple[Instant, str, Message]] = deque(inputs)
        self._sent: List[SystemOutput] = []

    @classmethod
    def replaying(cls, trace: Trace, read: InputReader) -> "TraceAdapter":
        """The player of ``trace``, each input's value read by ``read``, the
        reader of the test case's configuration.

        Raises a ``TraceRefusal`` naming the input, where one does not fit the
        configuration: its port is no system port of it, its type is not on the
        ``in`` list of that port's type, or its value does not belong to its
        type (docs/SCE_TTCN3_LANE.md §10.2). The whole trace is refused.
        """
        inputs: List[Tuple[Instant, str, Message]] = []
        for index, given in enumerate(trace.inputs):
            try:
                message = read(given.port, given.message_type, given.value)
            except TraceRefusal as refused:
                raise TraceRefusal(f"input {index}, at {given.at}: {refused.reason}") from None
            inputs.append((given.at, given.port, message))
        return cls(inputs)

    @property
    def sent(self) -> List[SystemOutput]:
        """What the components have sent to the system ports, in the order they
        sent it. The caller keeps the adapter, which the run only holds, so it
        reads this after the run."""
        return list(self._sent)

    def send(self, system_port: str, message: Message, now: Instant) -> None:
        self._sent.append(SystemOutput(at=now, system_port=system_port, message=message))

    def next_input(self) -> Optional[Instant]:
        return self._inputs[0][0] if self._inputs else None

    def take_inputs_due(self, now: Instant) -> List[SystemInput]:
        due: List[SystemInput] = []
        while self._inputs and self._inputs[0][0] <= now:
            _, port, message = self._inputs.popleft()
            due.append(SystemInput(system_port=port, message=message))
        return due
