# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""docs/SCE_TTCN3_LANE.md §12, §13.1 S2: every trace of a suite of the corpus
replays, through the suite's runner (``run_runner``, the program of §4 run in
place), to its expected log, byte for byte. The suites are the ones sce-codegen
writes into a module each beside this file (regen:
``scripts/regen_static_datamodel_python.sh``; the modules are not committed);
runtime_suite's own traces are replayed by
``test_a_test_suite_runs_through_its_runner.py``, beside the rest of what that
suite is held to.

The expected logs are the files every backend is held to, the canonical logs a
reviewed run of the Rust runner wrote. Each test is the twin of the Rust case of
the same name in
``backends/rust/tests/tests/the_corpus_replays_to_its_expected_logs.rs``.
orders_suite and nested_suite are not replayed here: their mapped record holds a
union, which Python's test profile matches no value of
(``codegen_matrix::TEST_PROFILE_UNION_BACKENDS``).
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import List

_HERE = Path(__file__).resolve().parent
# The tests directory, so this package is importable by the name it has there.
sys.path.insert(0, str(_HERE.parents[1]))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))
sys.path.insert(0, str(_HERE.parents[2] / "forge-runtime"))

# The suite's unit is imported as a module and read through it, as the runtime
# suite's test reads its own.
from integration.static_datamodel import boundary_suite, job_suite  # noqa: E402 -- path inserted above
from sce_runtime.testprofile import ParameterReader, Suite, run_runner  # noqa: E402

#: The directory of the runs of the suites of the corpus but runtime_suite, a
#: directory ``<suite>`` each with its ``traces/`` and ``expected_logs/``.
RUNS_DIR = _HERE.parents[4] / "sce-build" / "tests" / "fixtures" / "test_profile" / "runs"


def replays(
    suite_name: str, suite: Suite, parameter: ParameterReader, world: str, verdicts: List[str], logs: Path
) -> None:
    """Runs the suite's runner over every trace of its directory, in the world
    that selects every test case, into ``logs``, and holds what it prints to
    the verdicts it expects, every log it writes to its expected log, and the
    set of logs to the set of expected logs."""
    runs = RUNS_DIR / suite_name
    out: List[str] = []
    err: List[str] = []
    status = run_runner(
        suite,
        parameter,
        ["--trace-dir", str(runs / "traces"), "--log-dir", str(logs), "--world", world],
        out.append,
        err.append,
    )
    assert status == 0, err
    assert out == verdicts, err
    expected_dir = runs / "expected_logs"
    expected = []
    for path in sorted(expected_dir.iterdir(), key=lambda p: p.name.encode("utf-8")):
        assert path.name.endswith(".json"), f"the expected log {path.name} is a .json file"
        expected.append(path.name[: -len(".json")])
    written = []
    for path in sorted(logs.iterdir(), key=lambda p: p.name.encode("utf-8")):
        assert path.name.endswith(".log.json"), f"the runner wrote {path.name}, which is no log of a case"
        case = path.name[: -len(".log.json")]
        written.append(case)
        want = (expected_dir / f"{case}.json").read_bytes()
        assert want.endswith(b"\n"), f"the expected log of {case} ends in one newline"
        assert path.read_bytes() == want[:-1], f"the log of {case} of {suite_name} is not its expected log"
    assert written == expected, "the runner wrote a log of each case that has an expected log, and of no other"


def test_the_boundary_suite_replays_to_its_expected_logs(tmp_path: Path) -> None:
    """docs/SCE_TTCN3_LANE.md §12.2: signed and 64-bit integers in a message,
    the verdicts ``fail`` and ``none``, the two ways to ``error`` a document
    leads a run to, a test case that can never end and a dynamic error, and a
    branch taken before the states it leaves log their exits
    (docs/SCE_TEST_PROFILE.md §4.4)."""
    replays(
        "boundary_suite",
        boundary_suite.suite(),
        boundary_suite.module_parameter,
        "",
        [
            "exchanges_signed_numbers pass",
            "answers_another_job fail",
            "sets_no_verdict none",
            "never_ends error",
            "sends_to_a_removed_ptc error",
            "takes_before_its_exits pass",
        ],
        tmp_path / "logs",
    )


def test_the_job_suite_replays_to_its_expected_logs(tmp_path: Path) -> None:
    """docs/SCE_TEST_PROFILE.md §11.8: a status every matcher of
    ``every_matcher`` accepts, at 1 ms, which the tester's branch reads as a
    composed message; the helper is told and passes, and the tester settles
    for the duration its start gives and passes at 501 ms."""
    replays(
        "job_suite",
        job_suite.suite(),
        job_suite.module_parameter,
        "job-timeout,job-retry",
        ["completes_in_time pass"],
        tmp_path / "logs",
    )
