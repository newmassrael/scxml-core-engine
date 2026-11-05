# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""Both engines report through the one reporter, under each fixture's full ID.

Usage: expect_one_reporter_for_both_engines.py <w3c_test_cli> <test number> <variant id>...

The test number must be one whose fixtures are variants, and the variant ids
all of them (403 with 403a 403b 403c). This runs the CLI four ways, each in a
directory of its own, and reads the XML report each run writes there:

- the test number on both engines: every variant is listed once in the
  Interpreter suite and once in the AOT suite;
- the test number with --aot-only: every variant is listed once in the AOT
  suite, and there is no Interpreter suite;
- the test number with --interpreter-only: the converse;
- the first variant by its full ID with --aot-only: that variant alone, once,
  in the AOT suite.

The XML reporter used to hear of an AOT report only where the CLI handed it
one by hand, in the run of every test: a run of one test number with
--aot-only, or of one variant by its ID, wrote a report with no AOT test in
it, and an --interpreter-only run of a test number listed AOT runs the console
had dropped. The variant suffix is part of every name the report lists, so a
report that names a variant by its test number alone fails here too.

The exit status of each run is not read: whether the fixtures pass is the
conformance suite's question, and this one asks only what was reported.

It runs under whichever Python 3 CMake found for the build (a build host's may be
3.6), so it uses no interface newer than that.
"""

import os
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ElementTree
from typing import Dict, List

INTERPRETER_SUITE = "W3C_SCXML_Interpreter"
AOT_SUITE = "W3C_SCXML_AOT"


def reported(cli: str, arguments: List[str], problems: List[str]) -> Dict[str, List[str]]:
    """Run the CLI with these arguments; the test names its XML report lists, by suite."""
    with tempfile.TemporaryDirectory() as artifacts:
        # The CLI writes its XML report into the directory it runs in.
        run = subprocess.run(
            [cli] + arguments + ["--no-save-failed-logs"],
            cwd=artifacts,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=240,
            check=False,
        )
        report_path = os.path.join(artifacts, "w3c_test_results.xml")
        if not os.path.isfile(report_path):
            print(run.stdout.decode("utf-8", errors="replace"))
            problems.append(f"{' '.join(arguments)}: the run (exit {run.returncode}) wrote no XML report")
            return {}
        root = ElementTree.parse(report_path).getroot()

    suites = {}  # type: Dict[str, List[str]]
    for suite in root.findall("testsuite"):
        names = suites.setdefault(suite.get("name", ""), [])
        for case in suite.findall("testcase"):
            names.append(case.get("name", ""))
    return suites


def expect(arguments: List[str], suites: Dict[str, List[str]], suite: str, ids: List[str],
           problems: List[str]) -> None:
    """The suite lists each of these ids once, and nothing else."""
    wanted = sorted("Test_" + fixture_id for fixture_id in ids)
    listed = sorted(suites.get(suite, []))
    if listed != wanted:
        problems.append(f"{' '.join(arguments)}: suite {suite} lists {listed}, expected {wanted}")


def expect_absent(arguments: List[str], suites: Dict[str, List[str]], suite: str, problems: List[str]) -> None:
    """The run reported nothing on this engine."""
    if suite in suites:
        problems.append(f"{' '.join(arguments)}: suite {suite} is present ({sorted(suites[suite])}), "
                        "though that engine was not asked to run")


def main(argv: List[str]) -> int:
    if len(argv) < 4:
        print(__doc__, file=sys.stderr)
        return 2
    cli, number, variants = argv[1], argv[2], argv[3:]
    problems = []  # type: List[str]

    both = [number]
    suites = reported(cli, both, problems)
    expect(both, suites, INTERPRETER_SUITE, variants, problems)
    expect(both, suites, AOT_SUITE, variants, problems)

    aot_only = [number, "--aot-only"]
    suites = reported(cli, aot_only, problems)
    expect(aot_only, suites, AOT_SUITE, variants, problems)
    expect_absent(aot_only, suites, INTERPRETER_SUITE, problems)

    interpreter_only = [number, "--interpreter-only"]
    suites = reported(cli, interpreter_only, problems)
    expect(interpreter_only, suites, INTERPRETER_SUITE, variants, problems)
    expect_absent(interpreter_only, suites, AOT_SUITE, problems)

    one_variant = [variants[0], "--aot-only"]
    suites = reported(cli, one_variant, problems)
    expect(one_variant, suites, AOT_SUITE, [variants[0]], problems)
    expect_absent(one_variant, suites, INTERPRETER_SUITE, problems)

    if problems:
        print(f"The XML report of test {number} did not list what each engine ran:", file=sys.stderr)
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        return 1

    print(f"Test {number} ({', '.join(variants)}) was reported once per engine run, under its full ID.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
