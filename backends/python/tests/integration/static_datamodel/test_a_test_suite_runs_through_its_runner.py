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

The suite is also replayed from the committed trace of ``answers_the_job``
(docs/SCE_TTCN3_LANE.md §10.5) through ``run_suite_with_traces``, and from
every committed trace, each to its expected log, through the runner in place
(``run_runner``) and through the suite's unit run as its own program (§4), as
the Rust, Kotlin and C++ suites are.
"""
from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path
from typing import List, Optional, Tuple

import pytest

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
    reply_ok,
    runtime_suite,
)
from sce_runtime import saved_values  # noqa: E402
from sce_runtime.saved_json import parse_json, write_jcs  # noqa: E402
from sce_runtime.testprofile import (  # noqa: E402
    RUNNER_USAGE,
    CaseResult,
    Features,
    Instant,
    LogKind,
    Message,
    ModuleParameters,
    SystemAdapter,
    SystemInput,
    Trace,
    TraceRefusal,
    Verdict,
    features_of,
    run_runner,
    run_suite,
    run_suite_with_traces,
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
        # The reply's canonical form in the test log is the one the unit of a
        # template over its record writes.
        self._pending.append(
            (now + ANSWER_DELAY, Message.of(job_reply.EVENT_NAME, reply, reply_ok.TYPE, reply_ok.canonical))
        )

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


#: The test log every runtime writes of ``answers_the_job`` in a world with no
#: feature, answered with success: one file the Rust, C++, Kotlin and Python runs
#: are held to (``a_generated_test_case_writes_its_test_log`` in
#: ``backends/rust/tests/tests/a_test_suite_runs_through_its_runner.rs``), so the
#: agreement of the backends is one set of bytes, named by the path every other
#: reader uses.
ANSWERS_THE_JOB_LOG = (
    _HERE.parents[4]
    / "sce-build"
    / "tests"
    / "fixtures"
    / "test_profile"
    / "runtime"
    / "expected_logs"
    / "answers_the_job.json"
)


def answers_the_job_log() -> str:
    """The shared log, without the one newline the file ends in, which no
    canonical text holds."""
    assert ANSWERS_THE_JOB_LOG.is_file(), f"cannot read the shared test log at {ANSWERS_THE_JOB_LOG}"
    text = ANSWERS_THE_JOB_LOG.read_text(encoding="utf-8")
    assert text.endswith("\n") and not text.endswith("\n\n"), "the shared log ends in one newline"
    return text[:-1]


def test_a_generated_test_case_writes_its_test_log() -> None:
    """docs/SCE_TEST_PROFILE.md §12.5: the test log of a generated test case, in
    its one canonical text, the bytes of the shared file. The request the
    component sends carries the value its template's unit writes, the job of the
    suite's module parameter; the reply arrives at 500 and is taken in that
    round, which stops the timer, passes and ends the MTC."""
    requested: List[int] = []
    results = suite_run(features_of(), 0, requested)
    assert names_of(results) == ["answers_the_job"]
    assert results[0].canonical_log("runtime_suite") == answers_the_job_log()


#: The directory of the suite's committed traces (docs/SCE_TTCN3_LANE.md §10.2).
TRACES_DIR = ANSWERS_THE_JOB_LOG.parents[1] / "traces"


def test_a_recorded_trace_replays_to_the_test_log_every_runtime_writes() -> None:
    """docs/SCE_TTCN3_LANE.md §10.5: the generated suite replayed from the
    committed trace of ``answers_the_job`` writes the test log every runtime
    writes of it, byte for byte: the configuration's unit reads the reply from
    its canonical form and writes it back the same, and the clock advances to
    the trace's instant as it did to the Responder's. The twin of the Rust case
    of the same name."""
    text = (TRACES_DIR / "answers_the_job.json").read_bytes().decode("utf-8")
    results = run_suite_with_traces(runtime_suite.suite(), features_of(), lambda _: Trace.parse(text))
    assert names_of(results) == ["answers_the_job"]
    assert results[0].result.verdict == Verdict.PASS, results[0].result.log
    assert results[0].canonical_log("runtime_suite") == answers_the_job_log()


def test_the_configuration_reads_the_inputs_of_a_recorded_trace() -> None:
    """§10.2, §10.5: the configuration's unit reads the reply from its canonical
    form into the record its components take, and writes it back the same; an
    input on a port it does not have, of a type the port does not take, or of a
    value that does not belong to the type is refused by name."""
    reply = asking_configuration.read_job_reply(parse_json('{"code":"0","job":"7"}'))
    assert reply == job_reply.JobReplyPayload(job=7, code=0)
    assert write_jcs(asking_configuration.canonical_job_reply(reply)) == '{"code":"0","job":"7"}'
    message = asking_configuration.input("sut", "job_reply", asking_configuration.canonical_job_reply(reply))
    assert message.event == job_reply.EVENT_NAME
    assert message.type_name == "job_reply"
    assert message.value == reply
    for port, type_name, value, why in [
        ("sut", "job_request", None, "'job_request' is not on the `in` list of the type of the system port 'sut'"),
        ("oracle", "job_reply", None, "'oracle' is not a system port of the test case's configuration"),
        (
            "sut",
            "job_reply",
            parse_json('{"code":"0","job":"07"}'),
            "job_reply.job: '07' is not the text of an integer's decimal digits",
        ),
    ]:
        with pytest.raises(TraceRefusal) as refusal:
            asking_configuration.input(port, type_name, value)
        assert refusal.value.reason == why
    assert answers_the_job.test_case().inputs is asking_configuration.input, (
        "the test case hands the runner its configuration's reader"
    )


class ProgramRun:
    """What one run of the runner program printed and the status it exited
    with."""

    def __init__(self, status: int, out: List[str], err: List[str]) -> None:
        self.status = status
        self.out = out
        self.err = err


def run_in_place(logs: Path, *given: str, traces: Path = TRACES_DIR) -> ProgramRun:
    """The suite's runner program run in place (``run_runner``), given
    ``given`` after the directories of the traces, the committed ones unless
    ``traces`` names others, and of the logs."""
    out: List[str] = []
    err: List[str] = []
    status = run_runner(
        runtime_suite.suite(),
        runtime_suite.module_parameter,
        ["--trace-dir", str(traces), "--log-dir", str(logs), *given],
        out.append,
        err.append,
    )
    return ProgramRun(status, out, err)


#: The directory of the expected log of each committed trace, ``<test
#: case>.json`` each: the canonical log a reviewed run wrote of it, with one
#: newline after it.
EXPECTED_LOGS_DIR = ANSWERS_THE_JOB_LOG.parent

#: The world that selects every test case of the suite, so that the runner
#: replays every trace of the directory.
EVERY_FEATURE = "ptcs,removal,guard"

#: The lines the runner prints in the world of every feature: each case of the
#: suite, in its order, with the verdict it ends with.
EVERY_VERDICT = [
    "answers_the_job pass",
    "serves_a_ptc pass",
    "keeps_a_ptc inconc",
    "answers_before_the_guard error",
]


def assert_every_log_is_its_expected_log(written_dir: Path) -> None:
    """docs/SCE_TTCN3_LANE.md §12, §13.1 S2: every log the runner wrote into
    ``written_dir``, ``<case>.log.json``, is the expected log of its case byte
    for byte, and the runner wrote a log for every expected log and nothing
    else. The twin of the Rust ``assert_every_log_is_its_expected_log``."""
    expected: List[str] = []
    for name in sorted(path.name for path in EXPECTED_LOGS_DIR.iterdir()):
        assert name.endswith(".json"), f"the expected log {name} is a .json file"
        expected.append(name[: -len(".json")])
    written: List[str] = []
    for name in sorted(path.name for path in written_dir.iterdir()):
        assert name.endswith(".log.json"), f"the runner wrote {name}, which is no log of a case"
        test_case = name[: -len(".log.json")]
        written.append(test_case)
        want = EXPECTED_LOGS_DIR / f"{test_case}.json"
        assert want.is_file(), f"the case {test_case} has no expected log {want}"
        text = want.read_bytes()
        assert text.endswith(b"\n") and not text.endswith(b"\n\n"), (
            f"the expected log of {test_case} ends in one newline"
        )
        assert (written_dir / name).read_bytes() == text[:-1], f"the log of {test_case} is not its expected log"
    assert written == expected, "the runner wrote a log of each case that has an expected log, and of no other"


def test_the_suites_runner_program_writes_the_log_of_each_trace(tmp_path: Path) -> None:
    """docs/SCE_TTCN3_LANE.md §4: the suite's runner program runs each test case
    its world selects against its trace and writes ``<case>.log.json``, the
    canonical log with nothing after it, and prints each case's verdict. In the
    world of every feature it replays every trace of the directory, and each
    log it writes is its case's expected log (§12, §13.1 S2). A module
    parameter's value on its command line replaces its default, and one the
    suite does not declare, or that is no value of its type, is refused with
    exit status 2 before any case runs. The twin of the Rust case of the same
    name."""
    logs = tmp_path / "every"
    run = run_in_place(logs, "--world", EVERY_FEATURE)
    assert run.status == 0, run.err
    assert run.out == EVERY_VERDICT
    assert_every_log_is_its_expected_log(logs)

    overridden = tmp_path / "overridden"
    given = run_in_place(overridden, "--world", "", "--module-parameter", "job=9")
    assert given.status == 0, given.err
    log = (overridden / "answers_the_job.log.json").read_text(encoding="utf-8")
    assert '"kind":"message-sent","port":"sut","type":"job_request","value":{"job":"9"}' in log, (
        "the request names the job of the value given"
    )

    for literal, why in [
        ("limit=1", "the suite declares no module parameter 'limit'"),
        ("job=seven", "--module-parameter job=seven: job: 'seven' is not the text of an integer's decimal digits"),
    ]:
        refused = tmp_path / literal
        attempt = run_in_place(refused, "--module-parameter", literal)
        assert attempt.status == 2, literal
        assert any(why in line for line in attempt.err), attempt.err
        assert attempt.err[-1] == RUNNER_USAGE
        assert not refused.exists(), "no case ran, so no log was written"


def test_a_test_case_whose_trace_is_missing_ends_in_error_and_its_log_is_written(tmp_path: Path) -> None:
    """P13, §4: a test case whose trace is missing ends with ``error`` and its
    log is written, the reason on the standard error, and the program exits 0,
    since the verdict is the log's. The directory of the traces holds the
    committed trace of ``answers_the_job`` alone, so ``serves_a_ptc``'s is
    missing."""
    traces = tmp_path / "traces"
    traces.mkdir()
    shutil.copyfile(TRACES_DIR / "answers_the_job.json", traces / "answers_the_job.json")
    logs = tmp_path / "logs"
    run = run_in_place(logs, "--world", "ptcs", traces=traces)
    assert run.status == 0, run.err
    assert run.out == ["answers_the_job pass", "serves_a_ptc error"]
    assert (logs / "serves_a_ptc.log.json").read_text(encoding="utf-8") == (
        '{"records":[{"at":"0","kind":"test-case-started"},{"at":"0","kind":"test-case-ended",'
        '"verdict":"error"}],"schema":"sce-test-log.v1","suite":"runtime_suite",'
        '"test_case":"serves_a_ptc","verdict":"error"}'
    )
    assert any(
        line.startswith("serves_a_ptc: mtc: the trace is refused: ") and "is not read" in line for line in run.err
    ), run.err


def test_the_suites_unit_is_the_runner_program(tmp_path: Path) -> None:
    """§4: the suite's runner is a program: the suite's unit run as a module of
    its package (``python -m integration.static_datamodel.runtime_suite``), as
    its own process, prints the verdict of each case and nothing else on its
    standard output, writes its log and exits with the status the command line
    asks for. The twin of the Kotlin and C++ program cases."""
    tests_dir = _HERE.parents[1]
    path = [str(tests_dir), str(_HERE.parents[2] / "runtime"), str(_HERE.parents[2] / "forge-runtime")]
    inherited = os.environ.get("PYTHONPATH")
    env = dict(os.environ, PYTHONPATH=os.pathsep.join(path + ([inherited] if inherited else [])))

    def program(name: str, *given: str) -> "subprocess.CompletedProcess[str]":
        return subprocess.run(
            [
                sys.executable,
                "-m",
                "integration.static_datamodel.runtime_suite",
                "--trace-dir",
                str(TRACES_DIR),
                "--log-dir",
                str(tmp_path / f"{name}-logs"),
                *given,
            ],
            cwd=tests_dir,
            env=env,
            capture_output=True,
            text=True,
            timeout=120,
            check=False,
        )

    run = program("every", "--world", EVERY_FEATURE)
    assert run.returncode == 0, run.stderr
    assert run.stdout == "".join(f"{line}\n" for line in EVERY_VERDICT), (
        "the standard output is the contract's lines alone"
    )
    assert_every_log_is_its_expected_log(tmp_path / "every-logs")

    given = program("given", "--module-parameter", "job=9")
    assert given.returncode == 0, given.stderr
    assert '"value":{"job":"9"}' in (tmp_path / "given-logs" / "answers_the_job.log.json").read_text(
        encoding="utf-8"
    )

    refused = program("undeclared", "--module-parameter", "limit=1")
    assert refused.returncode == 2
    assert "the suite declares no module parameter 'limit'" in refused.stderr
    assert refused.stdout == ""
    assert not (tmp_path / "undeclared-logs").exists(), "no case ran, so no log was written"


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
