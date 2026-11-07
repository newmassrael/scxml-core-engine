#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# What heap a JVM the Kotlin lane started used during one row, read from the
# JVM's own garbage-collection log (`-Xlog:gc`), against the `-Xmx` it was
# given.
#
# ── Why this exists ────────────────────────────────────────────────
#
# From 2026-11-05 every Kotlin W3C run on main stopped at the same place: the
# `lua:lua` row's `:sce-kotlin-tests:compileKotlin` died inside the Kotlin
# compile daemon with `java.lang.OutOfMemoryError: GC overhead limit exceeded`,
# and Gradle answered "Not enough memory to run compilation. Try to increase it
# via 'gradle.properties': kotlin.daemon.jvmargs=-Xmx<size>". Nothing on a build
# host reproduced it: the same compiles, in the same order, in one daemon of the
# same 2 GB, all succeeded (measured 2026-11-07). So the heap the failing run
# used was known only as "more than it had", the one number a fix has to be
# sized from was missing, and the only place it can be read is the run that
# fails.
#
# This program reads it there. The gate starts both daemons with a GC log, notes
# where each log ended before a row, and asks this program about what the row
# added. It reports; it does not fail a row and it does not warn.
#
# ⚠⚠ Headroom cannot be read from this log, so this program does not try. On
# 2026-11-07 the Lua tree's `:sce-kotlin-tests:compileKotlin` was run on a
# build host at one -Xmx after another. It failed at 1,792 MB and every smaller
# size, and at every one of those sizes the last full collection left about
# 78% of the heap live. At 2,048 MB it compiled, with about 50% live. A
# parallel collector gives up ("GC overhead limit exceeded") before the live
# set reaches the whole heap, so the live share of a JVM that is about to fail
# and of one that has room stay close together, and no threshold on that share
# separates them. The earlier warning at 80% of the budget could therefore not
# fire before the failure. Headroom is measured where it can be: the gate
# compiles the same tree a second time in a daemon given a smaller heap than
# the budget, and that compile failing is the signal (`w3c-kotlin.sh`, the
# headroom probe).
#
# ── What is reported, and why those numbers ────────────────────────
#
#   peak       the most heap in use when any collection began: what the row
#              ever held, garbage included.
#   live       the most heap still in use after a FULL collection. A full
#              collection keeps only what is reachable at that moment, so this
#              is a LOWER bound on what the row needs: the moment the
#              compiler held the most is rarely the moment a collection ran.
#              Two fresh daemons doing comparable compiles read 1,033 and
#              1,425 MB (2026-11-07). A young collection's "after" also keeps
#              the old generation's garbage, so it is an upper bound on the
#              live set at that moment only, and it is reported as that when no
#              full collection happened.
#   gc share   the pause time over the wall time the row's collections cover,
#              from where the first pause began to where the last ended. The
#              JVM's "GC overhead limit exceeded" means this share went past
#              98%: a row whose share climbs is heading for it before any
#              `OutOfMemoryError` says so. One collection covers only its own
#              pause, so its share would be 100% by construction. Run
#              42353262993 printed "100.0% of 0.0s in pauses" for a daemon that
#              collected once in a row, which reads as a JVM that did nothing
#              but collect. Such a row is reported as having no share.
#
# The budget is printed beside these numbers and no ratio is drawn from it,
# for the reason above.
#
# ── What it refuses ────────────────────────────────────────────────
#
# A log that must exist and does not, and arguments with no `-Xmx`. Both mean
# the observation did not happen, and a report of zero collections from a log
# nobody wrote would read as "this row used no heap" (the lesson of a quiet
# zero: it is a reading that never happened, not an answer).
#
# Usage:
#   jvm_heap_report.py report --log FILE --from-byte N --jvmargs="ARGS"
#                             --label TEXT [--must-exist]
#   jvm_heap_report.py budget --jvmargs="ARGS"
#       prints the -Xmx of ARGS in MB (the last one, as the JVM takes it), so
#       the gate sizes its headroom probe from the same reading as the report
#   jvm_heap_report.py pid --log FILE
#       prints the process id of the JVM that wrote FILE, from the `pid`
#       decoration (`-Xlog:gc:file=FILE:uptime,pid,level,tags`), so the gate
#       stops the probe's daemon by a recorded id rather than by matching
#       command lines
#
# `--jvmargs=` is joined with `=` on purpose: JVM arguments open with `-`, and
# argparse reads a separate value with no space in it (`-XX:+UseParallelGC`)
# as an option rather than as the value.

import argparse
import os
import re
import sys

# `[12.345s][info][gc] GC(7) Pause Full (Ergonomics) 1900M->1850M(1964M) 2345.678ms`
# The decorations are the ones the gate asks for (`uptime,level,tags`); the
# collection fields are what every collector the JDK ships writes on `gc`.
_LINE = re.compile(
    r"^\[(?P<uptime>[0-9.]+)s\].*?\bGC\((?P<id>\d+)\)\s+(?P<what>Pause [A-Za-z]+)"
    r".*?\s(?P<before>\d+)(?P<bu>[KMG])->(?P<after>\d+)(?P<au>[KMG])\((?P<cap>\d+)(?P<cu>[KMG])\)"
    r"\s+(?P<ms>[0-9.]+)ms\s*$"
)
_XMX = re.compile(r"(?:^|\s)-Xmx(\d+)([kKmMgG]?)(?:\s|$)")
# `[0.002s][12345][info][gc] Using Parallel`: with `pid` among the decorations
# the JDK writes it after the uptime, whatever order the flag names them in.
_PID = re.compile(r"^\[[0-9.]+s\]\[(?P<pid>\d+)\]\[")
# A decision this reader made, and the answer is no: distinct from argparse's
# usage error (2) and from a crash, as `kotlin_coverage.py` keeps it.
REFUSED = 3
_UNIT_MB = {"K": 1.0 / 1024.0, "M": 1.0, "G": 1024.0}


def _mb(value, unit):
    return int(value) * _UNIT_MB[unit]


def budget_mb(jvmargs):
    """The `-Xmx` of `jvmargs` in MB, the last one when several are given (as
    the JVM itself takes it); None when there is none."""
    found = _XMX.findall(jvmargs)
    if not found:
        return None
    value, unit = found[-1]
    unit = unit.upper() or "B"
    if unit == "B":
        return int(value) / (1024.0 * 1024.0)
    return _mb(value, unit)


def collections(text):
    """Every collection line of `text`, as dicts of floats; lines that are not
    a collection (the JVM's own start-up lines, a torn last line) are left."""
    found = []
    for line in text.splitlines():
        match = _LINE.match(line)
        if match is None:
            continue
        found.append(
            {
                "uptime": float(match["uptime"]),
                "full": match["what"] == "Pause Full",
                "before": _mb(match["before"], match["bu"]),
                "after": _mb(match["after"], match["au"]),
                "ms": float(match["ms"]),
            }
        )
    return found


def summarize(found, budget, label):
    """The report line for one row."""
    if not found:
        return f"{label}: no collection in this row (the JVM did not collect while the row ran)"
    fulls = [entry for entry in found if entry["full"]]
    peak = max(entry["before"] for entry in found)
    # A line is written when its pause ENDS, so the window the row's collections
    # cover opens where the first pause began, not at its line. One collection
    # covers only its own pause, so it has no share to report.
    share = None
    if len(found) >= 2:
        first = min(found, key=lambda entry: entry["uptime"])
        span = max(entry["uptime"] for entry in found) - (first["uptime"] - first["ms"] / 1000.0)
        pause_s = sum(entry["ms"] for entry in found) / 1000.0
        share = (pause_s / span) if span > 0 else None
    if fulls:
        live = max(entry["after"] for entry in fulls)
        live_text = f"live {live:.0f} MB after the largest of {len(fulls)} full collection(s)"
    else:
        bound = max(entry["after"] for entry in found)
        live_text = f"no full collection, so the live set is at most {bound:.0f} MB"
    share_text = (
        f"{share * 100:.1f}% of {span:.1f}s in pauses"
        if share is not None
        else "no pause share (one collection covers only its own pause)"
    )
    return (
        f"{label}: {len(found)} collection(s); peak {peak:.0f} MB; {live_text}; "
        f"{share_text}; budget -Xmx {budget:.0f} MB"
    )


def print_writer_pid(log):
    """Print the process id of the JVM that wrote `log`, read from the `pid`
    decoration of its first line. Every line of a log carries the same one, so
    the first is enough, and it is written as the JVM starts."""
    try:
        with open(log, encoding="utf-8", errors="replace") as handle:
            first = handle.readline()
    except OSError as error:
        print(f"{log}: {error.strerror}", file=sys.stderr)
        return REFUSED
    match = _PID.match(first)
    if match is None:
        print(
            f"{log}: its first line carries no pid decoration ({first.strip()!r}), so the JVM "
            "that wrote it cannot be named",
            file=sys.stderr,
        )
        return REFUSED
    print(match["pid"])
    return 0


def main(argv):
    parser = argparse.ArgumentParser(prog="jvm_heap_report.py")
    commands = parser.add_subparsers(dest="command", required=True)
    report = commands.add_parser("report")
    report.add_argument("--log", required=True)
    report.add_argument("--from-byte", type=int, default=0)
    report.add_argument("--jvmargs", required=True)
    report.add_argument("--label", required=True)
    report.add_argument("--must-exist", action="store_true")
    budget_of = commands.add_parser("budget")
    budget_of.add_argument("--jvmargs", required=True)
    pid_of = commands.add_parser("pid")
    pid_of.add_argument("--log", required=True)
    options = parser.parse_args(argv)

    if options.command == "pid":
        return print_writer_pid(options.log)

    budget = budget_mb(options.jvmargs)
    if options.command == "budget":
        if budget is None:
            print(f"the JVM arguments carry no -Xmx ({options.jvmargs!r})", file=sys.stderr)
            return REFUSED
        print(f"{budget:.0f}")
        return 0
    if budget is None:
        print(
            f"{options.label}: the JVM arguments carry no -Xmx ({options.jvmargs!r}), "
            "so there is no budget to read this row's heap against",
            file=sys.stderr,
        )
        return REFUSED

    if not os.path.isfile(options.log):
        if options.must_exist:
            print(
                f"{options.label}: no GC log at {options.log}. The JVM was started without the "
                "-Xlog:gc this row asked for, so its heap was not observed",
                file=sys.stderr,
            )
            return REFUSED
        print(
            f"{options.label}: no GC log at {options.log}: this JVM did not start during the row, "
            "so there is nothing of this row to read"
        )
        return 0

    with open(options.log, "rb") as handle:
        handle.seek(options.from_byte)
        text = handle.read().decode("utf-8", errors="replace")

    print(summarize(collections(text), budget, options.label))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
