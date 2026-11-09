# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The test log of a test case run (docs/SCE_TEST_PROFILE.md §12.5;
docs/adr/0091, C3 and answer 8): the records of TCI's test logging interface
(ETSI ES 201 873-6), each written where the profile's semantics places its
event, in the order the round scheduler produces them, and the one canonical
text of a run's log, ``schemas/sce-test-log.v1.schema.json``.

The text is the JSON Canonicalization Scheme of the log
(``sce_runtime.saved_json.write_jcs``), and it holds no number: an instant, a
duration and every integer of a value are texts of their digits
(``canonical``). So two runs agree, under C3, exactly when their logs are the
same bytes, whichever runtime wrote each.

The module is ``testlog`` where the other runtimes name theirs ``test_log``, for
the reason the package is ``testprofile``: pytest collects every module whose
file is named ``test_*.py``. The classes whose names begin with ``Test`` say
``__test__ = False``, so a test that imports one is not asked to collect it.

Cross-language siblings: ``sce_rust_runtime::test::TestLogRecord`` and
``TestCaseResult::canonical_log`` (Rust), whose text this one writes byte for
byte; ``com.sce.runtime.test.TestLogRecord`` (Kotlin).
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import ClassVar, Dict, List, Optional, Sequence, Union

from ..saved_json import NotCanonical, write_jcs
from . import canonical
from .canonical import CanonicalValue
from .port import Duration, Instant
from .verdict import Verdict

#: The name of the test log's shape, its ``schema``.
TEST_LOG_SCHEMA = "sce-test-log.v1"

#: The members of a record beside ``at`` and ``kind``, by their names.
Members = Dict[str, CanonicalValue]


@dataclass(frozen=True)
class TestCaseStarted:
    """The test case started, at ``NOW = 0`` (P11)."""

    # Not a class of tests, though its name begins as pytest's do.
    __test__ = False

    kind: ClassVar[str] = "test-case-started"

    def members(self) -> Members:
        return {}


@dataclass(frozen=True)
class TestCaseEnded:
    """The test case ended with its verdict (P14)."""

    # Not a class of tests, though its name begins as pytest's do.
    __test__ = False

    kind: ClassVar[str] = "test-case-ended"

    #: The test case verdict.
    verdict: Verdict

    def members(self) -> Members:
        return {"verdict": str(self.verdict)}


@dataclass(frozen=True)
class ComponentCreated:
    """A component was created: the MTC as the test case starts, a PTC by the
    invocation that creates it (P9)."""

    kind: ClassVar[str] = "component-created"

    #: The component.
    component: str
    #: Its component type.
    component_type: str

    def members(self) -> Members:
        return {"component": self.component, "type": self.component_type}


@dataclass(frozen=True)
class ComponentDone:
    """A component terminated, with its local verdict then (P10)."""

    kind: ClassVar[str] = "component-done"

    #: The component.
    component: str
    #: Its local verdict when it terminated.
    verdict: Verdict

    def members(self) -> Members:
        return {"component": self.component, "verdict": str(self.verdict)}


@dataclass(frozen=True)
class MessageSent:
    """A component sent a message on a port (P6)."""

    kind: ClassVar[str] = "message-sent"

    #: The sender.
    component: str
    #: The port it sent on.
    port: str
    #: The name of the message's record type (``TestValue.type_name``).
    message_type: str
    #: The message's value in canonical form.
    value: CanonicalValue

    def members(self) -> Members:
        return _message_members(self.component, self.port, self.message_type, self.value)


@dataclass(frozen=True)
class MessageReceived:
    """A receive branch took the head of a port's queue (P4 step 1): the message
    was received and matched."""

    kind: ClassVar[str] = "message-received"

    #: The receiver.
    component: str
    #: The port it took the message from.
    port: str
    #: The name of the message's record type (``TestValue.type_name``).
    message_type: str
    #: The message's value in canonical form.
    value: CanonicalValue

    def members(self) -> Members:
        return _message_members(self.component, self.port, self.message_type, self.value)


@dataclass(frozen=True)
class TimerStarted:
    """A component started a timer, for a duration in microseconds (P7)."""

    kind: ClassVar[str] = "timer-started"

    #: The component.
    component: str
    #: The timer.
    timer: str
    #: The duration, in microseconds.
    duration: Duration

    def members(self) -> Members:
        return {
            "component": self.component,
            "timer": self.timer,
            "duration_us": canonical.integer(self.duration),
        }


@dataclass(frozen=True)
class TimerStopped:
    """A component stopped a running timer (P7). A stop of a timer that is not
    running changes nothing, and is not recorded."""

    kind: ClassVar[str] = "timer-stopped"

    #: The component.
    component: str
    #: The timer.
    timer: str

    def members(self) -> Members:
        return {"component": self.component, "timer": self.timer}


@dataclass(frozen=True)
class TimerTimedOut:
    """A timeout branch took a timer that had timed out (P4 step 1)."""

    kind: ClassVar[str] = "timer-timed-out"

    #: The component.
    component: str
    #: The timer.
    timer: str

    def members(self) -> Members:
        return {"component": self.component, "timer": self.timer}


@dataclass(frozen=True)
class VerdictSet:
    """A component's local verdict was set: by ``<sce:setverdict>`` (P12), or to
    ``error`` by the runtime (P13). The reason is the document's, and none is
    recorded for an empty one or for the runtime's ``error``, whose cause is the
    runtime's own prose, which another runtime would not write alike."""

    kind: ClassVar[str] = "verdict-set"

    #: The component.
    component: str
    #: The verdict it was given.
    verdict: Verdict
    #: The reason the document gave, if any.
    reason: Optional[str] = None

    def members(self) -> Members:
        members: Members = {"component": self.component, "verdict": str(self.verdict)}
        if self.reason is not None:
            members["reason"] = self.reason
        return members


@dataclass(frozen=True)
class Log:
    """A ``<log>`` (P8)."""

    kind: ClassVar[str] = "log"

    #: The component.
    component: str
    #: The text.
    text: str

    def members(self) -> Members:
        return {"component": self.component, "text": self.text}


#: An event of TCI's test logging interface (ES 201 873-6), as the profile's
#: semantics places it (docs/SCE_TEST_PROFILE.md §12.5). A component is named
#: ``mtc`` or by the PTC's name in its configuration.
TestLogEvent = Union[
    TestCaseStarted,
    TestCaseEnded,
    ComponentCreated,
    ComponentDone,
    MessageSent,
    MessageReceived,
    TimerStarted,
    TimerStopped,
    TimerTimedOut,
    VerdictSet,
    Log,
]


def _message_members(component: str, port: str, message_type: str, value: CanonicalValue) -> Members:
    """The members a message sent and a message received both write."""
    return {"component": component, "port": port, "type": message_type, "value": value}


@dataclass(frozen=True)
class TestLogRecord:
    """One record of the test log: an event and ``NOW`` when it happened."""

    # Not a class of tests, though its name begins as pytest's do.
    __test__ = False

    #: ``NOW`` when it was recorded.
    at: Instant
    #: What happened.
    event: TestLogEvent

    def to_value(self) -> Members:
        """The record as the test log writes it: ``at``, ``kind``, and the
        members of its kind."""
        value: Members = {"at": canonical.integer(self.at), "kind": self.event.kind}
        value.update(self.event.members())
        return value


def write_test_log(suite: str, test_case: str, verdict: Verdict, records: Sequence[TestLogRecord]) -> str:
    """The test log of the run of ``test_case`` of the suite ``suite``, which
    ended with ``verdict`` and wrote ``records``, in its one canonical text: the
    JSON Canonicalization Scheme of ``schema``, ``suite``, ``test_case``,
    ``verdict`` and ``records``, each record in the order the run produced it.

    Raises ``ValueError`` when a message's value is not in the canonical form
    (``TestValue.canonical``) but holds a number, or a value of a type that is
    no JSON value, which no writer of ``canonical`` builds.
    """
    records_written: List[CanonicalValue] = [record.to_value() for record in records]
    log: Members = {
        "schema": TEST_LOG_SCHEMA,
        "suite": suite,
        "test_case": test_case,
        "verdict": str(verdict),
        "records": records_written,
    }
    try:
        return write_jcs(log)
    except NotCanonical as refused:
        raise ValueError(
            f"the test log of {suite} {test_case} is not canonical: {refused.reason}"
        ) from refused
