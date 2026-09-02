#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The Kotlin conformance lane's COVERAGE verdict, in two halves that answer
# one question: which test classes the suite's own sources say must run
# (`derive`), and whether a row's JUnit reports account for exactly them
# (`verdict`).
#
# ── Why this is a program and not a heredoc in the gate ────────────
#
# Both halves lived inside `scripts/gates/w3c-kotlin.sh` until 2026-09-02, and
# what they decide is the only thing standing between an EMPTY RUN and a green
# row. Measured that day, by making one arm run nothing while Gradle still
# reported success (`--init-script` narrowing the test filter to a class that
# does not exist):
#
#     > Task :sce-kotlin-tests:test
#     BUILD SUCCESSFUL in 15s
#     ERROR gate[w3c-kotlin]: 251 test class(es) ... produced no JUnit report
#
# So Gradle greens a task that ran nothing — the premise the retired case-count
# floor was written on is TRUE — and the refusal that catches it today is the
# class comparison below, not a floor. That measurement had to be bought BY
# HAND, because nothing in the corpus can drive a gate's run-time logic
# (`no_shell_runner_reaches_a_gates_own_logic`): the inputs of that verdict —
# a JUnit report directory — do not exist until the gate has run Gradle.
#
# Split out here, they do exist: a report directory is a directory of files,
# and a test can build one. `sce-build/tests/kotlin_coverage_verdict.rs` does
# exactly that, so "an empty arm is refused" stops being a sentence a reader
# must trust and becomes a case that fails when it stops being true.
#
# ⚠ Refusals exit with REFUSED (3), never 1. A crash and a refusal are
# different verdicts, and a test that accepts any non-zero status cannot tell
# "the row was refused" from "the reader died before deciding" — the quiet-zero
# shape this repository keeps re-learning, in its exit-status alphabet.

import argparse
import pathlib
import re
import sys
import xml.etree.ElementTree as ET

# A refusal: this reader decided, and the answer is no.
REFUSED = 3

# ⚠ A floor under the DERIVATION, not under the report. The comparison the
# `verdict` half makes is an equality, and two empty sets are equal: a reader
# that parsed nothing would pass every row while asserting nothing at all. The
# report cannot be empty without failing that equality, so the floor belongs on
# this side of it. 251 classes were derived on 2026-08-31.
RUNNABLE_FLOOR = 200

BLOCK_COMMENT = re.compile(r"/\*.*?\*/", re.S)
LINE_COMMENT = re.compile(r"//[^\n]*")
EXECUTION_ANNOTATION = re.compile(
    r"@(Test|TestFactory|ParameterizedTest|RepeatedTest|TestTemplate)\b"
)
DECLARATION = re.compile(
    r"^((?:\w+[ \t]+)*)(class|object|interface)[ \t]+(\w+)([^\n{]*)", re.M
)


def refuse(message: str) -> int:
    print(f"{message}", file=sys.stderr)
    return REFUSED


def derive(root: pathlib.Path) -> list[str]:
    """Every concrete class the suite's sources say JUnit must execute.

    A FIXPOINT rather than a grep for `@Test`, because most of these classes
    do not carry one. A generated case is `class Test144 : W3CTestBase<...>()`
    and inherits every test it runs, so: a type declaring a JUnit execution
    annotation is a carrier, and a type extending a carrier is a carrier. What
    must have run is every carrier that is a concrete class -- 251 of them on
    2026-08-31, against 265 top-level declarations, the other 14 being sealed
    interfaces, private helpers, data classes and the two abstract bases. None
    of those 14 is named anywhere; they fall out of the fixpoint because
    nothing runs them.

    ⚠ Comments are stripped before anything is read. This repository has
    already watched a scanner read its own prose -- `reach_of` matched a gate
    script's COMMENT and demanded a tool that lane never installed -- and a
    `@Test` or a `class` named in a KDoc is exactly that defect here.
    """
    declarations: dict[str, dict] = {}
    for path in sorted(root.rglob("*.kt")):
        text = LINE_COMMENT.sub("", BLOCK_COMMENT.sub("", path.read_text()))
        package = re.search(r"^package\s+([\w.]+)", text, re.M)
        package = package.group(1) if package else ""
        found = list(DECLARATION.finditer(text))
        for index, match in enumerate(found):
            modifiers, kind, name, clause = match.groups()
            end = found[index + 1].start() if index + 1 < len(found) else len(text)
            supertypes = set(
                re.findall(r"\b([A-Z]\w*)\s*(?:<[^>]*>)?\s*\(", clause)
            ) | set(re.findall(r":\s*([A-Z]\w*)", clause))
            declarations[name] = {
                "qualified": f"{package}.{name}" if package else name,
                "kind": kind,
                "modifiers": modifiers.split(),
                "supertypes": supertypes,
                "declares_case": bool(EXECUTION_ANNOTATION.search(text[match.end() : end])),
            }

    carriers = {n for n, d in declarations.items() if d["declares_case"]}
    while True:
        grown = {
            n
            for n, d in declarations.items()
            if n not in carriers and d["supertypes"] & carriers
        }
        if not grown:
            break
        carriers |= grown

    return sorted(
        {
            declarations[name]["qualified"]
            for name in carriers
            if declarations[name]["kind"] == "class"
            and "abstract" not in declarations[name]["modifiers"]
        }
    )


def read_reports(reports: pathlib.Path) -> tuple[dict[str, int], set[str], list[str]]:
    """Totals, the classes that reported, and the cases they skipped.

    ⚠ The class is read from each `<testcase classname=...>`, and the file
    name is CHECKED against it rather than trusted. The obvious-looking
    datum -- `<testsuite name=...>` -- is the DISPLAY name, measured
    2026-09-02 on this very suite:

        TEST-com.sce.w3c.Test453.xml
        <testsuite name="Test 453 -- W3C SCXML B.2" ...>
          <testcase name="testW3CConformance()"
                    classname="com.sce.w3c.Test453" .../>

    A reader taking the `name` sees 242 of 251 classes as never having
    reported and the same 242 as unaccounted for -- a suite that ran in full,
    refused twice over. Which is what happened when this program first tried
    it, and what the gate refused.

    Requiring the two to agree keeps the file name an encoding this reader
    checks instead of one it assumes: measured the same day, all 251 reports
    carry at least one case and every case's `classname` equals the class its
    file is named for.
    """
    totals = {"tests": 0, "failures": 0, "errors": 0, "skipped": 0}
    reported: set[str] = set()
    skipped: list[str] = []
    for path in sorted(reports.glob("TEST-*.xml")):
        suite = ET.parse(path).getroot()
        named = path.name[len("TEST-") : -len(".xml")]
        classes = {case.get("classname") for case in suite.iter("testcase")}
        if not classes:
            raise ValueError(
                f"{path} holds no <testcase>, so the class it reports for cannot "
                f"be read from anything but its file name"
            )
        if classes != {named}:
            raise ValueError(
                f"{path} is named for `{named}` and its cases name "
                f"{sorted(str(c) for c in classes)} — the file name is this "
                f"reader's index into the report and it does not describe the "
                f"content"
            )
        reported.add(named)
        for key in totals:
            totals[key] += int(suite.get(key, 0))
        for case in suite.iter("testcase"):
            if case.find("skipped") is not None:
                skipped.append(f"{case.get('classname')}.{case.get('name')}")
    return totals, reported, skipped


def names(values) -> str:
    listed = sorted(values)
    shown = "\n".join(f"  {value}" for value in listed[:20])
    if len(listed) > 20:
        shown += f"\n  ... and {len(listed) - 20} more"
    return shown


def verdict(reports: pathlib.Path, runnable_file: pathlib.Path, label: str) -> int:
    runnable = {line for line in runnable_file.read_text().split("\n") if line.strip()}

    # The vacuity guard, on the side the comparison cannot defend. Two empty
    # sets are equal, so an empty derivation would accept every row -- and the
    # row that would expose it is the one that ran nothing, which is precisely
    # the row this verdict exists to refuse.
    if not runnable:
        return refuse(
            f"the runnable class set handed to this verdict for {label} is empty. "
            f"Every check below is a comparison against it, and an empty set "
            f"accepts an empty report -- so a derivation that parsed nothing "
            f"would report every row as complete"
        )

    if not reports.is_dir():
        return refuse(
            f"no JUnit results under {reports} -- the {label} run reported "
            f"success without producing a result file"
        )

    # A report this reader cannot parse is refused, never skipped. The
    # alternative — ignore the file and judge the rest — turns a corrupted or
    # renamed report into a class that "never reported", which is a true
    # sentence about the wrong defect.
    try:
        totals, reported, skipped = read_reports(reports)
    except (ET.ParseError, ValueError) as unreadable:
        return refuse(
            f"a JUnit report under {reports} could not be read for {label}: "
            f"{unreadable}"
        )

    if totals["failures"] or totals["errors"]:
        return refuse(
            f"Kotlin conformance on {label}: {totals['failures']} failure(s), "
            f"{totals['errors']} error(s) across {totals['tests']} case(s)"
        )

    # ⚠ A skipped case is not a passing one, and the class comparison below
    # cannot see it -- an `@Disabled` on a method leaves the class reporting.
    # So this is the method-level half of the same question, and it is asked by
    # NAME because a count would say how many stopped being measured without
    # saying which.
    if skipped:
        return refuse(
            "cases skipped on {label}:\n{listed}\n{count} Kotlin case(s) were "
            "SKIPPED on {label}. A skipped case is measured by nothing -- a "
            "conformance claim this row did not make, reported from inside a "
            "green run".format(label=label, listed=names(skipped), count=len(skipped))
        )

    # ⚠ The two directions fail on DIFFERENT defects, which is why neither is
    # dropped. A derived class missing from the report is a case that stopped
    # running -- a filter, an `@Disabled`, a registration deleted. A reported
    # class missing from the derivation is this reader going blind: a JUnit
    # annotation it does not know, a supertype clause it could not parse.
    # Checking only the first direction would let a blind reader report a
    # shrinking suite as whole.
    silent = runnable - reported
    if silent:
        return refuse(
            "test classes this row never reported:\n{listed}\n{count} test "
            "class(es) derived from the suite's sources produced no JUnit "
            "report on {label}. Each is a conformance claim this lane makes "
            "and did not run, and a total held over a floor cannot see one -- "
            "which is how `SendParamPayloadTest` and `XmlDataIsADomTreeTest` "
            "came to be named in a comment rather than measured".format(
                listed=names(silent), count=len(silent), label=label
            )
        )

    unknown = reported - runnable
    if unknown:
        return refuse(
            "reported classes the derivation does not account for:\n{listed}\n"
            "{count} class(es) reported on {label} are outside the set derived "
            "from the suite's sources. That set is this gate's only account of "
            "what must run, so a class it cannot see is a blind spot in the "
            "reader rather than a bonus: teach the fixpoint the annotation or "
            "supertype it missed. Unclassified is RED".format(
                listed=names(unknown), count=len(unknown), label=label
            )
        )

    print(totals["tests"])
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    derived = sub.add_parser(
        "derive", help="print every test class the suite's sources say must run"
    )
    derived.add_argument("sources", type=pathlib.Path)

    judged = sub.add_parser(
        "verdict", help="judge one row's JUnit reports against that set"
    )
    judged.add_argument("--reports", type=pathlib.Path, required=True)
    judged.add_argument("--runnable", type=pathlib.Path, required=True)
    judged.add_argument("--label", required=True)

    args = parser.parse_args()

    if args.command == "derive":
        if not args.sources.is_dir():
            return refuse(
                f"{args.sources} is not a directory, so no runnable test class "
                f"can be derived from it"
            )
        classes = derive(args.sources)
        if len(classes) < RUNNABLE_FLOOR:
            return refuse(
                f"only {len(classes)} runnable test class(es) were derived from "
                f"{args.sources}, under the floor of {RUNNABLE_FLOOR}. Every row "
                f"compares its report against this set, and a set this small "
                f"means the derivation failed rather than that the suite shrank "
                f"-- a derivation that failed reports every row as complete"
            )
        print("\n".join(classes))
        return 0

    return verdict(args.reports, args.runnable, args.label)


if __name__ == "__main__":
    sys.exit(main())
