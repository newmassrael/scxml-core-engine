# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A test suite generated in Python (docs/SCE_TEST_PROFILE.md §11.6 to §11.8,
§12.2, §12.6 slice 6), run end to end by the profile's runner,
``sce_runtime.testprofile.run_suite``. The suite, its test cases and their
configurations are the tables sce-codegen writes from the fixtures under
``sce-build/tests/fixtures/test_profile/runtime/`` into a module each beside
this file, and the components are the machines it writes from the components
there, each the ``<machine>_sm`` module its configuration's table imports
(regen: ``scripts/regen_static_datamodel_python.sh``; the modules are not
committed). Only the system adapter, which plays the system under test, is
written here.

Each test is the twin of the Rust case of the same name in
``backends/rust/tests/tests/a_test_suite_runs_through_its_runner.rs``, named
``test_<case>`` after it, itself the twin of the C++ gtest in
``tests/integration/ATestSuiteRunsEndToEndTest.cpp``, of the Kotlin
``ATestSuiteRunsThroughItsRunnerTest.kt`` and of the Go
``TestATestSuiteRunsThroughItsRunner_<Case>`` in
``a_test_suite_runs_through_its_runner_test.go``, with the same suite, world,
verdicts and ``NOW``. What the generator writes for each table is held by
``sce-build/tests/a_test_suite_is_generated_in_python.rs``.
"""
from __future__ import annotations

import sys
from pathlib import Path
from typing import List, Optional, Tuple

_HERE = Path(__file__).resolve().parent
# The tests directory, so this package is importable by the name it has there.
sys.path.insert(0, str(_HERE.parents[1]))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))
sys.path.insert(0, str(_HERE.parents[2] / "forge-runtime"))

# The units are imported as modules and read through them: a test case's unit
# declares `test_case`, which pytest would collect as a test of this module were
# it imported by its own name.
from integration.static_datamodel import (  # noqa: E402 -- path inserted above
    answers_the_job,
    asking_configuration,
    job_reply,
    job_request,
    runtime_suite,
)
from sce_runtime import saved_values  # noqa: E402
from sce_runtime.testprofile import (  # noqa: E402
    CaseResult,
    Features,
    Instant,
    LogKind,
    Message,
    ModuleParameters,
    SystemAdapter,
    SystemInput,
    Verdict,
    features_of,
    run_suite,
)

#: A millisecond of ``NOW``, which counts microseconds.
MILLISECOND = 1000

#: A second of ``NOW``.
SECOND = 1000 * MILLISECOND

#: How long the system under test takes to answer a request.
ANSWER_DELAY = 500


class Responder:
    """The system under test on the system port ``sut``: it answers each
    request, ``ANSWER_DELAY`` after it, with a reply of ``code`` for the job the
    request names, and records each job it was asked for in ``requested``, which
    the test keeps across the test cases of a run while each run holds its own
    adapter."""

    def __init__(self, code: int, requested: List[int]) -> None:
        self._code = code
        self._requested = requested
        self._pending: List[Tuple[Instant, Message]] = []

    def send(self, system_port: str, message: Message, now: Instant) -> None:
        assert system_port == "sut", f"a request is sent on the system port sut, not {system_port}"
        request = message.value
        assert isinstance(request, job_request.JobRequestPayload), (
            f"a request travels as its record's value: {message}"
        )
        self._requested.append(request.job)
        reply = job_reply.JobReplyPayload(job=request.job, code=self._code)
        self._pending.append((now + ANSWER_DELAY, Message(event=job_reply.EVENT_NAME, value=reply)))

    def next_input(self) -> Optional[Instant]:
        return self._pending[0][0] if self._pending else None

    def take_inputs_due(self, now: Instant) -> List[SystemInput]:
        due: List[SystemInput] = []
        while self._pending and self._pending[0][0] <= now:
            _, message = self._pending.pop(0)
            due.append(SystemInput(system_port="sut", message=message))
        return due


def suite_run(
    world: Features,
    code: int,
    requested: List[int],
    overrides: Optional[ModuleParameters] = None,
) -> List[CaseResult]:
    """The suite's run in ``world``, answered with ``code`` by a fresh
    ``Responder`` for each test case, with ``overrides`` in place of the
    defaults of the module parameters they give; ``None`` overrides none."""

    def adapters(_test_case: str) -> Optional[SystemAdapter]:
        return Responder(code, requested)

    return run_suite(runtime_suite.suite(), world, adapters, overrides)


def names_of(results: List[CaseResult]) -> List[str]:
    """The names of the test cases ``results`` reports, in run order."""
    return [result.test_case for result in results]


def logs(result: CaseResult, kind: LogKind) -> bool:
    """Whether the test log of ``result`` holds a record of ``kind``."""
    return any(record.kind == kind for record in result.result.log)


def test_runs_every_test_case_of_a_world_with_the_verdict_it_expects() -> None:
    """P11, P12, P13, P14: in a world with every feature the suite runs every
    test case in its order, and each ends with the verdict it expects: a job
    answered in time passes, a PTC served passes, a removed PTC's inconclusive
    outweighs its MTC's pass, and a guard shorter than the answer ends in
    error."""
    requested: List[int] = []
    results = suite_run(features_of("ptcs", "removal", "guard"), 0, requested)
    assert names_of(results) == [
        "answers_the_job",
        "serves_a_ptc",
        "keeps_a_ptc",
        "answers_before_the_guard",
    ]
    outcome = [result.result for result in results]
    assert outcome[0].verdict == Verdict.PASS, outcome[0].log
    assert outcome[0].ended_at == ANSWER_DELAY, "the reply is taken in the round it arrives in"
    assert outcome[1].verdict == Verdict.PASS, outcome[1].log
    assert outcome[1].ended_at == 0, "a branch fired in every round, so time never moved"
    assert outcome[2].verdict == Verdict.INCONCLUSIVE, outcome[2].log
    assert outcome[2].ended_at == 15 * MILLISECOND
    assert outcome[3].verdict == Verdict.ERROR, outcome[3].log
    assert outcome[3].ended_at == 100, "the guard, 0.1ms, ends the case before the answer"
    assert logs(results[3], LogKind.ERROR), "the guard is logged as an error"
    assert requested == [7, 7], (
        "each asking case asks for the job the suite's module parameter gives"
    )


def test_fails_a_job_the_system_under_test_answers_with_a_failure() -> None:
    """P2, P7: a system under test that answers with a failure code fails the
    test case when its timer runs out. A world with no feature runs only the
    test case no selection guards."""
    requested: List[int] = []
    results = suite_run(features_of(), 3, requested)
    assert names_of(results) == ["answers_the_job"]
    assert results[0].result.verdict == Verdict.FAIL, results[0].result.log
    assert results[0].result.ended_at == SECOND, "time advances to the timer's instant"
    assert logs(results[0], LogKind.TIMEOUT), "the timeout is logged"


def test_runs_the_test_cases_a_world_selects() -> None:
    """§11.8: a world runs the test cases whose selection it satisfies, in the
    suite's order: ``ptcs`` alone selects the served PTC and not the removed
    one, and ``guard`` selects the guarded case."""
    requested: List[int] = []
    assert names_of(suite_run(features_of("ptcs"), 0, requested)) == [
        "answers_the_job",
        "serves_a_ptc",
    ]
    assert names_of(suite_run(features_of("guard"), 0, requested)) == [
        "answers_the_job",
        "answers_before_the_guard",
    ]


def test_asks_for_the_job_an_override_of_its_module_parameter_names() -> None:
    """-1 §8.2.1, §11.5: a value the caller gives the module parameter replaces
    the suite's default, and the component asks for the job it names."""
    requested: List[int] = []
    overrides = ModuleParameters()
    overrides.set("job", saved_values.saved_write_uint32(9))
    results = suite_run(features_of(), 0, requested, overrides)
    assert names_of(results) == ["answers_the_job"]
    assert results[0].result.verdict == Verdict.PASS, results[0].result.log
    assert requested == [9], "the component asks for the job the override names"


def test_ends_in_error_when_a_module_parameter_is_not_of_its_type() -> None:
    """P13: a value of the module parameter that is not of the type the
    component declares ends the test case in error before the component
    starts."""
    requested: List[int] = []
    overrides = ModuleParameters()
    overrides.set("job", saved_values.saved_write_string("seven"))
    results = suite_run(features_of(), 0, requested, overrides)
    assert names_of(results) == ["answers_the_job"]
    assert results[0].result.verdict == Verdict.ERROR, results[0].result.log
    assert logs(results[0], LogKind.ERROR), "the refused value is logged as an error"
    assert requested == [], "the component did not start, so it sent nothing"


def test_reads_the_module_parameters_its_configuration_names() -> None:
    """§11.6, §11.7: the configuration's tables name the module parameters its
    components read, which a suite that runs it must give, and the test case
    names itself and its configuration."""
    assert asking_configuration.module_parameters() == ["job"]
    assert asking_configuration.configuration().mtc_type == "param_asker"
    assert asking_configuration.machine("nobody") is None, (
        "a type the configuration does not name has no machine"
    )
    assert asking_configuration.machine("param_asker") is not None, "the MTC's type has its machine"
    assert answers_the_job.NAME == "answers_the_job"
    case = answers_the_job.test_case()
    assert case.name == answers_the_job.NAME, "the test case is named by its unit's name"
    assert case.configuration == asking_configuration.configuration()
    assert case.module_parameters == ["job"]
