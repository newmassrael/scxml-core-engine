# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A manual fixture is judged, a skip is reported as a skip, and a repeated run keeps every iteration.

Usage: expect_skips_and_repeats_reported.py <w3c_test_cli> <resources dir> <variant number> <fixture id>...

The fixture ids are every fixture the build registers; the variant number is a
test number whose fixtures are variants (403 for 403a 403b 403c). Each run is
made in a directory of its own, and the XML report it writes there is read.

Manual verdicts. A fixture whose metadata says `manual: True` names an
observable its final state cannot carry, and the runner judges it by the
fixture's verifier (W3CTestRunner::runManualFixture, ManualVerification.cpp).
Every registered manual fixture is run on the Interpreter in one invocation,
and each must be listed once, with result="PASS" and type="manual" (judged by
its verifier, not passed on its final state and not skipped), the header must
count no failure, error or skip, and the run must exit 0.

Skips. A test that is not run used to be reported as a PASS whose reason said
"Test skipped", so the summary, the XML report and the console counted a test
nobody ran as passing. Two kinds are still not run: a manual fixture with no
verifier, and a BasicHTTP fixture under ThreadSanitizer. The suite registers no
manual fixture without a verifier, so this writes one, in a resources
directory of its own (`--resources`), numbered below the suite's first test
(144) so that no verifier holds it, and runs it alone on the Interpreter. The
run must say it skipped it: the testcase carries result="SKIPPED" and a
<skipped/> element and no <failure/>, the header counts it under `skipped`
and in no other bucket, the console's Total row counts it as skipped and not
as passed, and the run exits 0 (a skip is not a failure).

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

# The ID of the manual fixture this writes: below the W3C suite's first test
# (144), so no verifier in ManualVerification.cpp holds it
UNVERIFIED_MANUAL_FIXTURE = "1"


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


def manual_fixtures(resources: str, fixture_ids: List[str]) -> List[str]:
    """Every registered fixture whose metadata says it is manual, in registration order."""
    manual = []  # type: List[str]
    for fixture_id in fixture_ids:
        if not fixture_id.isdigit():
            continue
        metadata = os.path.join(resources, fixture_id, "metadata.txt")
        if not os.path.isfile(metadata):
            continue
        with open(metadata, encoding="utf-8", errors="replace") as text:
            if re.search(r"^\s*manual:\s*true\s*$", text.read(), re.M | re.I):
                manual.append(fixture_id)
    return manual


def check_manual_verdicts(cli: str, fixture_ids: List[str], problems: List[str]) -> None:
    arguments = fixture_ids + ["--interpreter-only"]
    result = run(cli, arguments, 300, problems)
    if result.root is None:
        return
    where = " ".join(arguments)

    cases = result.cases(INTERPRETER_SUITE)
    listed = sorted(case.get("id", "") for case in cases)
    if listed != sorted(fixture_ids):
        problems.append(f"{where}: the Interpreter suite lists {listed}, expected each of {sorted(fixture_ids)} once")
    for case in cases:
        name = case.get("name")
        if case.get("result") != "PASS":
            problems.append(f"{where}: {name} has result={case.get('result')!r}, expected 'PASS' "
                            f"(its verifier said: {case.get('description')!r})")
        if case.get("type") != "manual":
            problems.append(f"{where}: {name} has type={case.get('type')!r}, expected 'manual': "
                            "a manual fixture is judged by its verifier, not by its final state")
        if case.find("skipped") is not None or case.find("failure") is not None:
            problems.append(f"{where}: {name} carries a <skipped/> or <failure/> element")

    header = {key: result.root.get(key) for key in ("tests", "failures", "errors", "skipped")}
    expected = {"tests": str(len(fixture_ids)), "failures": "0", "errors": "0", "skipped": "0"}
    if header != expected:
        problems.append(f"{where}: the report's header counts {header}, expected {expected}")

    if result.returncode != 0:
        problems.append(f"{where}: exited {result.returncode}, expected 0")


def write_unverified_manual_fixture(resources: str) -> None:
    """A manual fixture no verifier holds: its metadata, and a document that would pass were it run."""
    directory = os.path.join(resources, UNVERIFIED_MANUAL_FIXTURE)
    os.makedirs(directory)
    with open(os.path.join(directory, "metadata.txt"), "w", encoding="utf-8") as metadata:
        metadata.write(f"id: {UNVERIFIED_MANUAL_FIXTURE}\n"
                       "specnum: none\n"
                       "conformance: mandatory\n"
                       "manual: True\n"
                       "description: A manual fixture no verifier judges\n"
                       "variants: 1\n")
    with open(os.path.join(directory, f"test{UNVERIFIED_MANUAL_FIXTURE}.txml"), "w", encoding="utf-8") as document:
        document.write('<?xml version="1.0"?>\n'
                       '<scxml xmlns="http://www.w3.org/2005/07/scxml" '
                       'xmlns:conf="http://www.w3.org/2005/scxml-conformance" version="1.0" initial="pass">\n'
                       '  <conf:pass/>\n'
                       '</scxml>\n')


def check_skip(cli: str, problems: List[str]) -> None:
    with tempfile.TemporaryDirectory() as scratch:
        resources = os.path.join(scratch, "resources")
        write_unverified_manual_fixture(resources)
        arguments = ["--resources", resources, UNVERIFIED_MANUAL_FIXTURE, "--interpreter-only"]
        result = run(cli, arguments, 240, problems)
    if result.root is None:
        return
    where = " ".join(arguments)

    cases = result.cases(INTERPRETER_SUITE)
    if len(cases) != 1:
        problems.append(f"{where}: the Interpreter suite lists {len(cases)} cases, expected 1")
    for case in cases:
        if case.get("id") != UNVERIFIED_MANUAL_FIXTURE:
            problems.append(f"{where}: the case is fixture {case.get('id')!r}, expected {UNVERIFIED_MANUAL_FIXTURE!r}")
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

    manual = manual_fixtures(resources, fixture_ids)
    if not manual:
        problems.append("no registered fixture says `manual: True`, so no manual verdict can be checked")
    else:
        check_manual_verdicts(cli, manual, problems)

    check_skip(cli, problems)

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
        print("The W3C runner did not judge a manual fixture, report what it skipped, or keep what each iteration "
              "ran:", file=sys.stderr)
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        return 1

    print(f"Manual fixtures {', '.join(manual)} were each judged a pass by their verifier, a manual fixture with "
          "no verifier was reported skipped, and both iterations of each repeated run were kept.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
