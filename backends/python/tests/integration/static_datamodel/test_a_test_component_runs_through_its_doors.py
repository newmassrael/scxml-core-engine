# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""Test components generated in Python (docs/SCE_TEST_PROFILE.md §12.3, §12.6
slice 6), run end to end by the profile's runtime,
``sce_runtime.testprofile.TestCaseRun``. The machines are the statecharts
sce-codegen writes from the fixtures under
``sce-build/tests/fixtures/test_profile/runtime/`` into a module each beside
this file, and the records and templates they name into modules of their own
beside them (regen: ``scripts/regen_static_datamodel_python.sh``; the modules
are not committed), driven only through the doors of §12.3; the
configuration, the factory of machines and the system adapter are written
here.

Each test is the twin of the Rust case of the same name in
``backends/rust/tests/tests/a_test_component_runs_through_its_doors.rs``, named
``test_<case>`` after it, itself the twin of the C++ gtest in
``tests/integration/ATestComponentRunsThroughItsDoorsTest.cpp``, of the Kotlin
``ATestComponentRunsThroughItsDoorsTest.kt`` and of the Go
``a_test_component_runs_through_its_doors_test.go``, with the same scenario,
verdict, ``NOW`` and log. What the generator writes for each component is held
by ``sce-build/tests/a_test_component_is_generated_in_python.rs``.
"""
from __future__ import annotations

import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable, Dict, List, Optional, Tuple

_HERE = Path(__file__).resolve().parent
# The tests directory, so this package is importable by the name it has there.
sys.path.insert(0, str(_HERE.parents[1]))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))
sys.path.insert(0, str(_HERE.parents[2] / "forge-runtime"))

from integration.static_datamodel import (  # noqa: E402 -- path inserted above
    answerer_sm,
    asker_sm,
    coordinator_sm,
    idler_sm,
    job_reply,
    job_request,
    keeper_sm,
)
from sce_runtime.testprofile import (  # noqa: E402
    MTC_NAME,
    ComponentMachine,
    Configuration,
    Connection,
    Endpoint,
    Instant,
    LogKind,
    Mapping,
    Message,
    SystemInput,
    TestCaseResult,
    TestCaseRun,
    Verdict,
)

#: A millisecond of ``NOW``, which counts microseconds.
MILLISECOND = 1000

#: A second of ``NOW``.
SECOND = 1000 * MILLISECOND

#: How long the system under test takes to answer a request.
ANSWER_DELAY = 500

#: The generated component of each component type a configuration here names.
COMPONENTS: Dict[str, Callable[[], ComponentMachine]] = {
    "asker": asker_sm.AskerComponent,
    "coordinator": coordinator_sm.CoordinatorComponent,
    "answerer": answerer_sm.AnswererComponent,
    "keeper": keeper_sm.KeeperComponent,
    "idler": idler_sm.IdlerComponent,
}


def machine_of(component_type: str) -> Optional[ComponentMachine]:
    """The generated machine of a component type a configuration here names,
    and none for another type, which ends the run with ``error``."""
    made = COMPONENTS.get(component_type)
    return made() if made is not None else None


@dataclass
class Seen:
    """What the system under test was handed: each send, as its system port and
    its event, and the job each request named."""

    sent: List[Tuple[str, str]] = field(default_factory=list)
    requested: List[int] = field(default_factory=list)


class Responder:
    """The system under test on the mapped port ``sut``: it answers each request,
    ``ANSWER_DELAY`` after it, with a reply of ``code`` for the requested job, or
    for ``job`` when one is given. It records what it was sent in ``seen``, which
    the test keeps while the run holds the adapter."""

    def __init__(self, seen: Seen, code: int, job: Optional[int] = None) -> None:
        self._seen = seen
        self._code = code
        self._job = job
        self._pending: List[Tuple[Instant, Message]] = []

    def send(self, system_port: str, message: Message, now: Instant) -> None:
        self._seen.sent.append((system_port, message.event))
        request = message.value
        if not isinstance(request, job_request.JobRequestPayload):
            return
        self._seen.requested.append(request.job)
        job = request.job if self._job is None else self._job
        reply = job_reply.JobReplyPayload(job=job, code=self._code)
        self._pending.append((now + ANSWER_DELAY, Message(event=job_reply.EVENT_NAME, value=reply)))

    def next_input(self) -> Optional[Instant]:
        return self._pending[0][0] if self._pending else None

    def take_inputs_due(self, now: Instant) -> List[SystemInput]:
        due: List[SystemInput] = []
        while self._pending and self._pending[0][0] <= now:
            _, message = self._pending.pop(0)
            due.append(SystemInput(system_port="sut", message=message))
        return due


class Silent:
    """A system under test nothing is mapped to."""

    def send(self, system_port: str, message: Message, now: Instant) -> None:
        raise AssertionError(
            "no port is mapped, so nothing reaches the system adapter, yet "
            f"{message.event} was sent on {system_port}"
        )

    def next_input(self) -> Optional[Instant]:
        return None

    def take_inputs_due(self, now: Instant) -> List[SystemInput]:
        return []


def first_record(result: TestCaseResult, component: str, kind: LogKind) -> Optional[int]:
    """The index of the first record of ``kind`` by ``component``, or ``None``."""
    for at, record in enumerate(result.log):
        if record.component == component and record.kind == kind:
            return at
    return None


def record_of(result: TestCaseResult, component: str, kind: LogKind) -> int:
    """The index of the first record of ``kind`` by ``component``, which the log
    holds."""
    at = first_record(result, component, kind)
    assert at is not None, f"{component} logs {kind}: {result.log}"
    return at


def run(configuration: Configuration, adapter) -> TestCaseResult:
    return TestCaseRun(configuration, machine_of, adapter).run()


def asker_configuration() -> Configuration:
    """The asker as the MTC, its port ``sut`` mapped to the system port ``sut``."""
    return Configuration(
        mtc_type="asker",
        ports_of_type={"asker": ["sut"]},
        timers_of_type={"asker": ["reply"]},
        mappings=[Mapping(port=Endpoint(component=MTC_NAME, port="sut"), system_port="sut")],
    )


def run_asker(code: int, job: Optional[int] = None) -> Tuple[TestCaseResult, Seen]:
    """The asker run against a responder that answers with ``code``, for the
    requested job or for ``job`` when one is given."""
    seen = Seen()
    return run(asker_configuration(), Responder(seen, code, job)), seen


def test_a_test_component_that_is_answered_with_a_matching_reply_passes() -> None:
    """P6, P2, P4: the send carries the template's value bound to the
    component's variable, a reply its receive branch's template matches is taken
    when it arrives, and the content reads the reply as ``_event.data``."""
    result, seen = run_asker(0)
    assert result.verdict == Verdict.PASS, result.log
    assert result.ended_at == ANSWER_DELAY, "the reply is taken in the round it arrives in"
    assert seen.sent == [("sut", job_request.EVENT_NAME)], (
        "a message travels under its template's record's event"
    )
    assert seen.requested == [7], "the template's value, given `job`"
    record_of(result, MTC_NAME, LogKind.RECEIVE)
    # P8: the `<log>` of the branch's content is in the test log, with the
    # instant and the component, its label and the job it read.
    logged = record_of(result, MTC_NAME, LogKind.LOG)
    assert result.log[logged].detail == "answered: 7"
    assert result.log[logged].at == ANSWER_DELAY


def test_a_test_component_that_is_answered_with_a_reply_its_template_does_not_match_fails_when_its_timer_fires() -> None:  # noqa: E501 -- the Rust case's name
    """P2, P7, §7: a reply the template does not match enables no branch and is
    not removed, and the timer's timeout branch fails the test case."""
    result, _ = run_asker(3)
    assert result.verdict == Verdict.FAIL, result.log
    assert result.ended_at == SECOND, "time advances to the timer's instant"
    assert first_record(result, MTC_NAME, LogKind.RECEIVE) is None, (
        f"a reply that does not match is never taken: {result.log}"
    )
    record_of(result, MTC_NAME, LogKind.TIMEOUT)


def test_a_test_component_reads_the_reply_it_took_as_the_events_data() -> None:
    """P4: the content reads the message the branch took: a reply of success for
    another job matches the template, which takes any job, and the content's
    comparison of the job it read fails the test case."""
    result, _ = run_asker(0, job=8)
    assert result.verdict == Verdict.FAIL, result.log
    assert result.ended_at == ANSWER_DELAY
    record_of(result, MTC_NAME, LogKind.RECEIVE)


def test_a_test_component_creates_a_ptc_exchanges_a_message_with_it_and_reads_its_done() -> None:
    """P9, P6, P2: the MTC creates a PTC connected to it, which asks it for job 42
    as it starts; the MTC answers with the job it read, the PTC passes on the
    reply and terminates, and the MTC's done branch passes on the next snapshot.
    Leaving the invoking state kills the PTC, before the done branch's content
    runs."""
    configuration = Configuration(
        mtc_type="coordinator",
        ptc_types={"answerer": "answerer"},
        ports_of_type={"coordinator": ["peer"], "answerer": ["peer"]},
        connections=[
            Connection(
                a=Endpoint(component=MTC_NAME, port="peer"),
                b=Endpoint(component="answerer", port="peer"),
            )
        ],
    )
    result = run(configuration, Silent())

    assert result.verdict == Verdict.PASS, result.log
    assert result.ended_at == 0, "a branch fired in every round, so time never moved"
    created = record_of(result, "answerer", LogKind.CREATE)
    asked = record_of(result, "answerer", LogKind.SEND)
    answered = record_of(result, MTC_NAME, LogKind.SEND)
    done = record_of(result, "answerer", LogKind.DONE)
    killed = record_of(result, "answerer", LogKind.KILL)
    passed = record_of(result, MTC_NAME, LogKind.VERDICT)
    assert created < asked, (
        f"the PTC is created when the macrostep that entered its state ends: {result.log}"
    )
    # P8: a `<log>` that writes only a label is recorded, by the PTC.
    noted = record_of(result, "answerer", LogKind.LOG)
    assert result.log[noted].detail == "asked"
    assert asked < noted, f"the log runs in its place, after the send before it: {result.log}"
    assert result.log[asked].detail == "peer job.request"
    assert result.log[answered].detail == "peer job.reply"
    assert answered < done, f"the MTC answers before the PTC is done: {result.log}"
    assert done < killed, (
        f"a done branch fires on a snapshot taken after the PTC terminated: {result.log}"
    )
    assert killed < passed, (
        f"the invoking state is exited before the done branch's content runs: {result.log}"
    )


def test_a_test_component_kills_a_ptc_on_leaving_the_state_that_invoked_it() -> None:
    """P9, P14: leaving the state that invoked a PTC removes it while it still
    runs, at that instant, and its local verdict still counts."""
    configuration = Configuration(
        mtc_type="keeper",
        ptc_types={"idler": "idler"},
        ports_of_type={"idler": ["peer"]},
        timers_of_type={"keeper": ["patience", "linger"]},
    )
    result = run(configuration, Silent())

    killed = record_of(result, "idler", LogKind.KILL)
    assert result.log[killed].at == 10 * MILLISECOND, "the PTC is removed when `patience` runs out"
    assert first_record(result, "idler", LogKind.DONE) is None, (
        f"the idler never terminates: {result.log}"
    )
    assert result.ended_at == 15 * MILLISECOND
    assert result.verdict == Verdict.INCONCLUSIVE, (
        "the removed PTC's inconclusive outweighs the MTC's pass"
    )
