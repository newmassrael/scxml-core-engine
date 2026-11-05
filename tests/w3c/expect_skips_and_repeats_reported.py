# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A skip is reported as a skip, and a repeated run keeps every iteration.

Usage: expect_skips_and_repeats_reported.py <w3c_test_cli> <resources dir> <variant number> <fixture id>...

The fixture ids are every fixture the build registers; the variant number is a
test number whose fixtures are variants (403 for 403a 403b 403c). Each run is
made in a directory of its own, and the XML report it writes there is read.

Skips. A manual fixture is not run on the Interpreter. It used to be reported
as a PASS whose reason said "Test skipped", so the summary, the XML report and
the console counted a test nobody ran as passing, and a BasicHTTP fixture
skipped under ThreadSanitizer the same way. One manual fixture is run alone on
the Interpreter, and the run must say it skipped it: the testcase carries
result="SKIPPED" and a <skipped/> element and no <failure/>, the header counts
it under `skipped` and in no other bucket, the console's Total row counts it
as skipped and not as passed, and the run exits 0 (a skip is not a failure).
The manual fixture is the first the build registers whose metadata says
`manual: True`, other than 178: the runner verifies 178 itself
(W3CTestRunner::runManualTest178), so it is run, not skipped.

Repeats. `--repeat 2` runs every test twice in one invocation, which is one
test run for the reporter: both iterations are in the report, each testcase
named "Test_<id>@<iteration>" and carrying `id` and `iteration`, so none of
them shadows another. Checked twice:
- every fixture on the Interpreter (`--interpreter-only --repeat 2`, the mode
  whose second iteration used to begin a new test run and drop the first),
  where each iteration must list every fixture once;
- the variant number on both engines (`<number> --repeat 2`), where each
  engine must list each variant once per iteration.

It runs under whichever Python 3 CMake found for the build (a build host's may be
3.6), so it uses no interface newer than that.
"""

import os
import re
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ElementTree
from typing import Dict, List, Optional, Tuple

INTERPRETER_SUITE = "W3C_SCXML_Interpreter"
AOT_SUITE = "W3C_SCXML_AOT"

# A fixture the runner verifies itself although its metadata says manual
RUNNER_VERIFIED_MANUAL = "178"


class Run:
    """One CLI run: its exit status, its console output and its XML report."""

    def __init__(self, returncode: int, output: str, root: Optional[ElementTree.Element]) -> None:
        self.returncode = returncode
        self.output = output
        self.root = root

    def cases(self, suite: str) -> List[ElementTree.Element]:
        if self.root is None:
            return []
        for element in self.root.findall("testsuite"):
            if element.get("name") == suite:
                return element.findall("testcase")
        return []

    def has_suite(self, suite: str) -> bool:
        return self.root is not None and any(e.get("name") == suite for e in self.root.findall("testsuite"))


def run(cli: str, arguments: List[str], timeout: int, problems: List[str]) -> Run:
    with tempfile.TemporaryDirectory() as artifacts:
        # The CLI writes its XML report into the directory it runs in.
        completed = subprocess.run(
            [cli] + arguments + ["--no-save-failed-logs"],
            cwd=artifacts,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=timeout,
            check=False,
        )
        output = completed.stdout.decode("utf-8", errors="replace")
        report_path = os.path.join(artifacts, "w3c_test_results.xml")
        if not os.path.isfile(report_path):
            print(output)
            problems.append(f"{' '.join(arguments)}: the run (exit {completed.returncode}) wrote no XML report")
            return Run(completed.returncode, output, None)
        return Run(completed.returncode, output, ElementTree.parse(report_path).getroot())


def manual_fixture(resources: str, fixture_ids: List[str]) -> Optional[str]:
    """The first registered fixture whose metadata says it is manual, 178 aside."""
    for fixture_id in fixture_ids:
        if fixture_id == RUNNER_VERIFIED_MANUAL or not fixture_id.isdigit():
            continue
        metadata = os.path.join(resources, fixture_id, "metadata.txt")
        if not os.path.isfile(metadata):
            continue
        with open(metadata, encoding="utf-8", errors="replace") as text:
            if re.search(r"^\s*manual:\s*true\s*$", text.read(), re.M | re.I):
                return fixture_id
    return None


def check_skip(cli: str, fixture_id: str, problems: List[str]) -> None:
    arguments = [fixture_id, "--interpreter-only"]
    result = run(cli, arguments, 240, problems)
    if result.root is None:
        return
    where = " ".join(arguments)

    cases = result.cases(INTERPRETER_SUITE)
    if len(cases) != 1:
        problems.append(f"{where}: the Interpreter suite lists {len(cases)} cases, expected 1")
    for case in cases:
        if case.get("result") != "SKIPPED":
            problems.append(f"{where}: {case.get('name')} has result={case.get('result')!r}, expected 'SKIPPED'")
        if case.find("skipped") is None:
            problems.append(f"{where}: {case.get('name')} carries no <skipped/> element")
        if case.find("failure") is not None:
            problems.append(f"{where}: {case.get('name')} carries a <failure/> element")

    header = {key: result.root.get(key) for key in ("tests", "failures", "errors", "skipped")}
    if header != {"tests": "1", "failures": "0", "errors": "0", "skipped": "1"}:
        problems.append(f"{where}: the report's header counts {header}, expected one test, skipped")

    # The console's Total row: Total, Passed, Failed, Errors, Skipped
    total_row = re.search(r"Total\s*\S\s*(\d+)\s*\S\s*(\d+)\s*\S\s*(\d+)\s*\S\s*(\d+)\s*\S\s*(\d+)", result.output)
    if total_row is None:
        problems.append(f"{where}: the console printed no Total row")
    elif total_row.groups() != ("1", "0", "0", "0", "1"):
        problems.append(f"{where}: the console's Total row reads {total_row.groups()} "
                        "(total, passed, failed, errors, skipped), expected one skipped and none passed")

    if result.returncode != 0:
        problems.append(f"{where}: exited {result.returncode}; a skip is not a failure, expected 0")


def iterations_of(cases: List[ElementTree.Element], where: str, problems: List[str]) -> Dict[str, List[str]]:
    """The test ids each iteration lists, checking each name is Test_<id>@<iteration>."""
    by_iteration = {}  # type: Dict[str, List[str]]
    for case in cases:
        test_id, iteration, name = case.get("id", ""), case.get("iteration", ""), case.get("name", "")
        if name != f"Test_{test_id}@{iteration}":
            problems.append(f"{where}: a repeated case is named {name!r}, expected 'Test_{test_id}@{iteration}'")
        by_iteration.setdefault(iteration, []).append(test_id)
    return by_iteration


def check_repeat(cli: str, arguments: List[str], suites: List[Tuple[str, Optional[List[str]]]], absent: List[str],
                 timeout: int, problems: List[str]) -> None:
    """Each suite lists the same tests, once each, in iteration 1 and in iteration 2.

    A suite given its ids must list exactly those in each iteration; a suite
    given None must list a non-empty set, the same in both.
    """
    result = run(cli, arguments, timeout, problems)
    if result.root is None:
        return
    where = " ".join(arguments)

    expected_total = 0
    for suite, ids in suites:
        cases = result.cases(suite)
        names = [case.get("name") for case in cases]
        if len(names) != len(set(names)):
            problems.append(f"{where}: suite {suite} lists one name more than once")
        by_iteration = iterations_of(cases, f"{where} ({suite})", problems)
        if sorted(by_iteration) != ["1", "2"]:
            problems.append(f"{where}: suite {suite} lists iterations {sorted(by_iteration)}, expected ['1', '2']")
            continue
        first, second = sorted(by_iteration["1"]), sorted(by_iteration["2"])
        wanted = sorted(ids) if ids is not None else second
        if not wanted:
            problems.append(f"{where}: suite {suite} lists no case")
        for iteration, listed in (("1", first), ("2", second)):
            if listed != wanted:
                problems.append(f"{where}: suite {suite}, iteration {iteration}, lists {len(listed)} cases "
                                f"where {len(wanted)} were run")
        expected_total += 2 * len(wanted)
    for suite in absent:
        if result.has_suite(suite):
            problems.append(f"{where}: suite {suite} is present, though that engine was not asked to run")

    if result.root.get("tests") != str(expected_total):
        problems.append(f"{where}: the report's header counts tests={result.root.get('tests')}, "
                        f"expected {expected_total}")


def main(argv: List[str]) -> int:
    if len(argv) < 5:
        print(__doc__, file=sys.stderr)
        return 2
    cli, resources, number, fixture_ids = argv[1], argv[2], argv[3], argv[4:]
    variants = [fixture_id for fixture_id in fixture_ids if re.fullmatch(re.escape(number) + "[a-z]", fixture_id)]
    problems = []  # type: List[str]

    manual = manual_fixture(resources, fixture_ids)
    if manual is None:
        problems.append("no registered fixture other than 178 says `manual: True`, so no skip path can be run")
    else:
        check_skip(cli, manual, problems)

    if not variants:
        problems.append(f"no registered fixture is a variant of {number}")
    else:
        check_repeat(cli, [number, "--repeat", "2"], [(INTERPRETER_SUITE, variants), (AOT_SUITE, variants)], [],
                     240, problems)

    # Every fixture the Interpreter discovers: the second iteration must list
    # the same set as the first, and the first must still be there.
    check_repeat(cli, ["--interpreter-only", "--repeat", "2"], [(INTERPRETER_SUITE, None)], [AOT_SUITE], 600,
                 problems)

    if problems:
        print("The W3C runner did not report what it skipped, or what each iteration ran:", file=sys.stderr)
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        return 1

    print(f"Fixture {manual} was reported skipped, and both iterations of each repeated run were kept.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
