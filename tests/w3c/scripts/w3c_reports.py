#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The JUnit reports of the C++ W3C programs, joined and held to the registry.

Every fixture of the conformance registry (tests/w3c/conformance/fixtures.json)
runs in the program of its test number, `w3c_cpp_test_<number>`, registered as
the ctest `w3c_cpp_<number>` (tests/w3c/W3CPrograms.cmake), and each test writes
the runner's JUnit report into a run directory of its own,
`<runs>/<number>/w3c_test_results.xml`. A lane runs the tests in shards, so the
report the lanes publish (the C++ check and the Unified Report) is made here,
and so is the account that no fixture fell out on the way:

  w3c_reports.py collect --registry <fixtures.json> --runs <dir> --tests <file>
                         --out <xml> [--whole]
      Join the reports of the tests named in <file> (one ctest name a line; a
      name that is not `w3c_cpp_<number>` is no W3C program and is passed
      over). Each report must list every fixture of its test number once on
      each engine and nothing else; with --whole, the tests together must
      cover every fixture of the registry. Prints the number of cases joined.

  w3c_reports.py join --registry <fixtures.json> --out <xml> <report>...
      Join reports `collect` wrote (one per shard). Together they must list
      every fixture of the registry once on each engine, and nothing else.

  w3c_reports.py interactive --xml <gtest.xml> --floor <n>
      Read GoogleTest's XML report of the interactive suite: it must record no
      failure and at least <n> passed cases. Prints the number that passed.

  w3c_reports.py --self-test

Either joining command writes the joined report even when it finds a fault, so
the lane still publishes what did run; the exit status is 1 when it found one.

The joined report has the runner's own shape (W3CTestRunner.cpp, the XML
reporter): a `<testsuites>` header with the totals, then one `<testsuite>` per
engine, `W3C_SCXML_Interpreter` and `W3C_SCXML_AOT`, each testcase on a line of
its own, so the lane's summary can count them a line at a time. The totals are
tallied from the testcases, by the rule TestSummaryHelper applies: FAIL is a
failure, SKIPPED a skip, PASS a pass, and any other result an error.

It runs under whichever Python 3 the gate finds (a build host's may be 3.6), so
it uses no interface newer than that.
"""

import argparse
import json
import os
import re
import sys
import tempfile
import xml.etree.ElementTree as ElementTree
from typing import Dict, List, Tuple

# The runner's suites, by engine, in the order it writes them.
SUITES = [("W3C_SCXML_Interpreter", "interpreter"), ("W3C_SCXML_AOT", "aot")]
SUITE_OF = dict(SUITES)

# A ctest name that is the run of a W3C program.
PROGRAM_TEST = re.compile(r"^w3c_cpp_([0-9]+)$")

# The report each W3C test writes into its run directory.
REPORT_NAME = "w3c_test_results.xml"

# A case: (engine, fixture id) and its testcase element.
Case = Tuple[Tuple[str, str], ElementTree.Element]


def number_of(fixture_id: str) -> str:
    """The test number a fixture id begins with (403 for 403a)."""
    match = re.match(r"^([0-9]+)[a-z]?$", fixture_id)
    if match is None:
        raise ValueError("{!r} is no W3C fixture id".format(fixture_id))
    return match.group(1)


def registry_fixtures(path: str) -> List[str]:
    """Every fixture id the registry lists, in registry order."""
    with open(path, encoding="utf-8") as text:
        registry = json.load(text)
    fixtures = registry.get("fixtures")
    if not isinstance(fixtures, list) or not fixtures:
        raise ValueError("{} declares no `fixtures` array".format(path))
    ids = [str(entry["id"]) for entry in fixtures]
    for fixture_id in ids:
        number_of(fixture_id)
    return ids


def read_cases(path: str, faults: List[str]) -> List[Case]:
    """The testcases of one runner report, each with its engine and fixture."""
    try:
        root = ElementTree.parse(path).getroot()
    except (OSError, ElementTree.ParseError) as error:
        faults.append("{}: unreadable ({})".format(path, error))
        return []
    cases = []  # type: List[Case]
    for suite in root.findall("testsuite"):
        name = suite.get("name", "")
        engine = SUITE_OF.get(name)
        if engine is None:
            faults.append("{}: a suite named {!r}, which is no engine's".format(path, name))
            continue
        for case in suite.findall("testcase"):
            cases.append(((engine, case.get("id", "")), case))
    return cases


def held_to(cases: List[Case], expected: List[str], where: str, faults: List[str]) -> None:
    """Every fixture in `expected` once on each engine, and no other case."""
    seen = {}  # type: Dict[Tuple[str, str], int]
    for key, _ in cases:
        seen[key] = seen.get(key, 0) + 1
    wanted = set(expected)
    for _, engine in SUITES:
        for fixture_id in expected:
            count = seen.get((engine, fixture_id), 0)
            if count == 0:
                faults.append("{}: no {} case of fixture {}".format(where, engine, fixture_id))
            elif count > 1:
                faults.append("{}: {} {} cases of fixture {}, where one was run".format(
                    where, count, engine, fixture_id))
    for (engine, fixture_id), count in sorted(seen.items()):
        if fixture_id not in wanted:
            faults.append("{}: {} {} case(s) of {!r}, which is not a fixture it runs".format(
                where, count, engine, fixture_id))


def tally(cases: List[Case]) -> Dict[str, str]:
    """The header's counts, by the rule TestSummaryHelper applies."""
    counts = {"tests": 0, "failures": 0, "errors": 0, "skipped": 0}
    seconds = 0.0
    for _, case in cases:
        counts["tests"] += 1
        result = case.get("result", "")
        if result == "FAIL":
            counts["failures"] += 1
        elif result == "SKIPPED":
            counts["skipped"] += 1
        elif result != "PASS":
            counts["errors"] += 1
        try:
            seconds += float(case.get("time", "0"))
        except ValueError:
            pass
    totals = {key: str(value) for key, value in counts.items()}
    totals["time"] = "{:g}".format(seconds)
    return totals


def attributes(element: ElementTree.Element) -> str:
    """An element's attributes, in the order its report wrote them, escaped again."""
    return "".join(' {}="{}"'.format(key, escape(value)) for key, value in element.attrib.items())


def escape(value: str) -> str:
    return (value.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
            .replace('"', "&quot;").replace("'", "&apos;"))


def write_report(path: str, cases: List[Case]) -> None:
    """The joined report, in the runner's shape: totals, then one suite per engine."""
    lines = ['<?xml version="1.0" encoding="UTF-8"?>']
    header = tally(cases)
    lines.append("<testsuites{}>".format(
        "".join(' {}="{}"'.format(key, header[key])
                for key in ("tests", "failures", "errors", "skipped", "time"))))
    for suite, engine in SUITES:
        members = [entry for entry in cases if entry[0][0] == engine]
        if not members:
            continue
        totals = tally(members)
        lines.append('  <testsuite name="{}"{}>'.format(suite, "".join(
            ' {}="{}"'.format(key, totals[key])
            for key in ("tests", "failures", "errors", "skipped", "time"))))
        for _, case in members:
            children = list(case)
            if not children:
                lines.append("    <testcase{}/>".format(attributes(case)))
                continue
            lines.append("    <testcase{}>".format(attributes(case)))
            for child in children:
                lines.append("      <{}{}/>".format(child.tag, attributes(child)))
            lines.append("    </testcase>")
        lines.append("  </testsuite>")
    lines.append("</testsuites>")
    directory = os.path.dirname(os.path.abspath(path))
    os.makedirs(directory, exist_ok=True)
    with open(path, "w", encoding="utf-8") as out:
        out.write("\n".join(lines) + "\n")


def collect(registry: str, runs: str, tests_file: str, out: str, whole: bool,
            faults: List[str]) -> int:
    """Join the reports of the W3C tests named in `tests_file`; the number of cases."""
    ids = registry_fixtures(registry)
    by_number = {}  # type: Dict[str, List[str]]
    for fixture_id in ids:
        by_number.setdefault(number_of(fixture_id), []).append(fixture_id)
    with open(tests_file, encoding="utf-8") as text:
        names = [line.strip() for line in text if line.strip()]
    cases = []  # type: List[Case]
    numbers = []  # type: List[str]
    for name in names:
        match = PROGRAM_TEST.match(name)
        if match is None:
            continue
        number = match.group(1)
        numbers.append(number)
        expected = by_number.get(number)
        if expected is None:
            faults.append("{} runs test number {}, of which the registry lists no fixture".format(
                name, number))
            continue
        report = os.path.join(runs, number, REPORT_NAME)
        if not os.path.isfile(report):
            faults.append("{} wrote no report ({})".format(name, report))
            continue
        found = read_cases(report, faults)
        held_to(found, expected, name, faults)
        cases.extend(found)
    if whole:
        missing = sorted(set(by_number) - set(numbers), key=int)
        if missing:
            faults.append("no test runs test number(s) {} of the registry".format(" ".join(missing)))
    write_report(out, cases)
    return len(cases)


def join(registry: str, reports: List[str], out: str, faults: List[str]) -> int:
    """Join the shards' reports; together they are every fixture once on each engine."""
    ids = registry_fixtures(registry)
    if not reports:
        faults.append("no report to join: no shard left one")
    cases = []  # type: List[Case]
    for report in reports:
        cases.extend(read_cases(report, faults))
    held_to(cases, ids, "the joined report", faults)
    write_report(out, cases)
    return len(cases)


def interactive(path: str, floor: int, faults: List[str]) -> int:
    """How many cases of the interactive suite passed, by its GoogleTest report."""
    try:
        root = ElementTree.parse(path).getroot()
    except (OSError, ElementTree.ParseError) as error:
        faults.append("{}: unreadable ({})".format(path, error))
        return 0
    passed = failed = 0
    for case in root.iter("testcase"):
        if case.get("status", "run") != "run":
            continue
        if case.find("failure") is not None or case.find("error") is not None:
            failed += 1
        elif case.find("skipped") is None and case.get("result", "completed") != "skipped":
            passed += 1
    if failed:
        faults.append("{}: {} case(s) of the interactive suite failed".format(path, failed))
    if passed < floor:
        faults.append("{}: {} case(s) of the interactive suite passed, expected at least {}: "
                      "it covered less than its name claims".format(path, passed, floor))
    return passed


# ── Self-test ────────────────────────────────────────────────────────────────


def self_test() -> int:
    checks = [0]

    def check(condition: bool, what: str) -> None:
        checks[0] += 1
        if not condition:
            raise AssertionError(what)

    def runner_report(path: str, cases: List[Tuple[str, str, str]]) -> None:
        """A report in the runner's shape: (engine, id, result) per case."""
        lines = ['<?xml version="1.0" encoding="UTF-8"?>', '<testsuites tests="0">']
        for suite, engine in SUITES:
            lines.append('  <testsuite name="{}">'.format(suite))
            for case_engine, fixture_id, result in cases:
                if case_engine != engine:
                    continue
                classname = "W3C_Interpreter" if engine == "interpreter" else "W3C_AOT"
                head = ('    <testcase classname="{}" name="Test_{}" id="{}" iteration="1" '
                        'time="0.5" type="pure_static" result="{}" description="d &amp; e"'
                        .format(classname, fixture_id, fixture_id, result))
                if result == "PASS":
                    lines.append(head + "/>")
                else:
                    tag = "skipped" if result == "SKIPPED" else "failure"
                    lines.append(head + ">")
                    lines.append('      <{} message="why"/>'.format(tag))
                    lines.append("    </testcase>")
            lines.append("  </testsuite>")
        lines.append("</testsuites>")
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as out:
            out.write("\n".join(lines) + "\n")

    def both(fixture_id: str, result: str = "PASS") -> List[Tuple[str, str, str]]:
        return [("interpreter", fixture_id, result), ("aot", fixture_id, result)]

    with tempfile.TemporaryDirectory() as scratch:
        registry = os.path.join(scratch, "fixtures.json")
        with open(registry, "w", encoding="utf-8") as out:
            json.dump({"fixtures": [{"id": i, "harness": "simple"}
                                    for i in ("144", "147", "403a", "403b")]}, out)
        runs = os.path.join(scratch, "runs")
        runner_report(os.path.join(runs, "144", REPORT_NAME), both("144"))
        runner_report(os.path.join(runs, "147", REPORT_NAME), both("147", "FAIL"))
        runner_report(os.path.join(runs, "403", REPORT_NAME), both("403a") + both("403b", "SKIPPED"))
        tests = os.path.join(scratch, "tests")

        def tests_are(*names: str) -> None:
            with open(tests, "w", encoding="utf-8") as out:
                out.write("".join(name + "\n" for name in names))

        def collected(whole: bool = False) -> Tuple[int, List[str], ElementTree.Element]:
            faults = []  # type: List[str]
            out = os.path.join(scratch, "joined.xml")
            count = collect(registry, runs, tests, out, whole, faults)
            return count, faults, ElementTree.parse(out).getroot()

        tests_are("w3c_cpp_144", "w3c_cpp_403", "ComprehensiveInteractiveTests")
        count, faults, root = collected()
        check(faults == [], "a shard whose reports hold its fixtures collects: {}".format(faults))
        check(count == 6, "two fixtures of 403 and one of 144, on two engines: {}".format(count))
        check(root.get("tests") == "6" and root.get("skipped") == "2"
              and root.get("failures") == "0" and root.get("errors") == "0",
              "the header is tallied from the cases: {}".format(root.attrib))
        check([s.get("name") for s in root.findall("testsuite")]
              == ["W3C_SCXML_Interpreter", "W3C_SCXML_AOT"], "one suite per engine, in order")
        with open(os.path.join(scratch, "joined.xml"), encoding="utf-8") as text:
            lines = text.read().splitlines()
        check(sum(1 for l in lines if 'classname="W3C_AOT"' in l and 'result="PASS"' in l) == 2,
              "each testcase is a line of its own, classname and result on it")
        check(any("d &amp; e" in l for l in lines), "an attribute is written escaped")

        count, faults, _ = collected(whole=True)
        check(any("147" in f and "no test runs" in f for f in faults),
              "a whole run that leaves a test number out is refused: {}".format(faults))

        tests_are("w3c_cpp_144", "w3c_cpp_147", "w3c_cpp_403")
        count, faults, root = collected(whole=True)
        check(faults == [], "a whole run collects: {}".format(faults))
        check(root.get("failures") == "2", "a failing case is counted, not refused: {}".format(
            root.attrib))

        tests_are("w3c_cpp_580")
        _, faults, _ = collected()
        check(any("580" in f and "no fixture" in f for f in faults),
              "a test number the registry does not list is refused")

        os.remove(os.path.join(runs, "144", REPORT_NAME))
        tests_are("w3c_cpp_144")
        _, faults, _ = collected()
        check(any("wrote no report" in f for f in faults), "a test that wrote no report is named")

        runner_report(os.path.join(runs, "144", REPORT_NAME), [("interpreter", "144", "PASS")])
        _, faults, _ = collected()
        check(any("no aot case of fixture 144" in f for f in faults),
              "an engine's missing case is named: {}".format(faults))

        runner_report(os.path.join(runs, "144", REPORT_NAME), both("144") + both("144") + both("999"))
        _, faults, _ = collected()
        check(any("2 interpreter cases of fixture 144" in f for f in faults),
              "a case listed twice is named: {}".format(faults))
        check(any("'999'" in f for f in faults), "a case of another fixture is named")

        shard_a = os.path.join(scratch, "a.xml")
        shard_b = os.path.join(scratch, "b.xml")
        runner_report(shard_a, both("144") + both("403a"))
        runner_report(shard_b, both("147") + both("403b"))
        joined = os.path.join(scratch, "all.xml")
        faults = []
        check(join(registry, [shard_a, shard_b], joined, faults) == 8 and faults == [],
              "the shards' reports join to the registry: {}".format(faults))
        faults = []
        join(registry, [shard_a], joined, faults)
        check(any("fixture 147" in f for f in faults), "a fixture no shard ran is named")
        faults = []
        join(registry, [shard_a, shard_b, shard_a], joined, faults)
        check(any("2 aot cases of fixture 144" in f for f in faults),
              "a fixture two shards ran is named: {}".format(faults))
        faults = []
        join(registry, [], joined, faults)
        check(any("no report to join" in f for f in faults), "no report at all is not a pass")

        gtest = os.path.join(scratch, "gtest.xml")
        with open(gtest, "w", encoding="utf-8") as out:
            out.write('<?xml version="1.0"?>\n<testsuites tests="4">\n'
                      '  <testsuite name="AllW3CTests/T">\n'
                      '    <testcase name="a" status="run" result="completed"/>\n'
                      '    <testcase name="b" status="run" result="completed"/>\n'
                      '    <testcase name="c" status="run" result="skipped">'
                      '<skipped message="no fixture"/></testcase>\n'
                      '    <testcase name="d" status="notrun" result="suppressed"/>\n'
                      '  </testsuite>\n</testsuites>\n')
        faults = []
        check(interactive(gtest, 2, faults) == 2 and faults == [],
              "a skipped or disabled case is not a pass: {}".format(faults))
        faults = []
        interactive(gtest, 3, faults)
        check(any("expected at least 3" in f for f in faults), "a suite under its floor is refused")
        with open(gtest, "w", encoding="utf-8") as out:
            out.write('<testsuites><testsuite><testcase name="a" status="run">'
                      '<failure message="x"/></testcase></testsuite></testsuites>\n')
        faults = []
        interactive(gtest, 0, faults)
        check(any("failed" in f for f in faults), "a failing case is refused")
        faults = []
        interactive(os.path.join(scratch, "missing.xml"), 0, faults)
        check(any("unreadable" in f for f in faults), "a missing report is refused")

    print("w3c_reports self-test: {} checks passed".format(checks[0]))
    return 0


# ── Command line ─────────────────────────────────────────────────────────────


def report_faults(faults: List[str]) -> int:
    for fault in faults:
        print("w3c_reports: {}".format(fault), file=sys.stderr)
    return 1 if faults else 0


def main(argv: List[str]) -> int:
    if argv[1:] == ["--self-test"]:
        return self_test()
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command")
    collecting = sub.add_parser("collect", help="join the reports of the W3C tests one run ran")
    collecting.add_argument("--registry", required=True)
    collecting.add_argument("--runs", required=True)
    collecting.add_argument("--tests", required=True)
    collecting.add_argument("--out", required=True)
    collecting.add_argument("--whole", action="store_true")
    joining = sub.add_parser("join", help="join the shards' reports")
    joining.add_argument("--registry", required=True)
    joining.add_argument("--out", required=True)
    joining.add_argument("reports", nargs="*")
    reading = sub.add_parser("interactive", help="read the interactive suite's report")
    reading.add_argument("--xml", required=True)
    reading.add_argument("--floor", type=int, required=True)
    args = parser.parse_args(argv[1:])

    faults = []  # type: List[str]
    try:
        if args.command == "collect":
            print(collect(args.registry, args.runs, args.tests, args.out, args.whole, faults))
        elif args.command == "join":
            print(join(args.registry, args.reports, args.out, faults))
        elif args.command == "interactive":
            print(interactive(args.xml, args.floor, faults))
        else:
            parser.print_help(sys.stderr)
            return 2
    except (OSError, ValueError, KeyError) as error:
        faults.append(str(error))
    return report_faults(faults)


if __name__ == "__main__":
    sys.exit(main(sys.argv))
