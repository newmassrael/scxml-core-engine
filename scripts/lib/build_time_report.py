#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Where a ninja build's time went, read from the `.ninja_log` it leaves in the
# tree it built.
#
#   scripts/lib/build_time_report.py <build dir> [--top N]
#   scripts/lib/build_time_report.py --self-test
#
# WHY. From 2026-11-21 every run of the C++ suite's lane ended at its ceiling
# with nothing in its log between "building the main tree" and the cancel
# (job 138540727626: 15:24 to 17:26), so two hours of a hosted runner said
# nothing about which targets took them. Ninja records every step it finishes
# in `.ninja_log`, start and end in milliseconds from the start of its run, as
# the step ends, so the steps a cancelled build did finish are on disk. This
# reads the last run in the log and prints its wall time, its steps, the
# slowest of them and the CMake targets whose steps took longest together.
#
# THE LAST RUN. A run appends its steps in the order they end, so their end
# times never decrease; the next run's times start again near zero. The last
# run is the tail of the log over which the end times do not decrease.

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

_TARGET = re.compile(r"CMakeFiles/([^/]+)\.dir/")


def entries_of(text: str) -> list[tuple[int, int, str]]:
    """`(start ms, end ms, output)` for every step line of a `.ninja_log`."""
    found = []
    for line in text.splitlines():
        if line.startswith("#"):
            continue
        fields = line.split("\t")
        if len(fields) < 4:
            continue
        try:
            found.append((int(fields[0]), int(fields[1]), fields[3]))
        except ValueError:
            continue
    return found


def last_run(entries: list[tuple[int, int, str]]) -> list[tuple[int, int, str]]:
    start = len(entries)
    while start > 0 and (start == len(entries) or entries[start - 1][1] <= entries[start][1]):
        start -= 1
    return entries[start:]


def steps_of(run: list[tuple[int, int, str]]) -> list[tuple[int, int, list[str]]]:
    """One step per `(start, end)`: an edge with several outputs (an object and
    its `.dwo`) is logged once per output."""
    steps: dict[tuple[int, int], list[str]] = {}
    for start, end, output in run:
        steps.setdefault((start, end), []).append(output)
    return [(start, end, outputs) for (start, end), outputs in steps.items()]


def target_of(output: str) -> str:
    match = _TARGET.search(output)
    return match.group(1) if match else output


def report(text: str, top: int) -> str:
    run = last_run(entries_of(text))
    if not run:
        return "build time: the log holds no step"
    steps = steps_of(run)
    wall = max(end for _, end, _ in steps) - min(start for start, _, _ in steps)
    busy = sum(end - start for start, end, _ in steps)
    lines = [f"build time: {len(steps)} step(s) finished in {wall / 60000:.1f} min of wall "
             f"time, {busy / 60000:.1f} min of step time (about {busy / max(wall, 1):.1f} "
             f"at once)"]
    lines.append(f"slowest {min(top, len(steps))} step(s):")
    for start, end, outputs in sorted(steps, key=lambda s: (s[0] - s[1], s[2]))[:top]:
        lines.append(f"  {(end - start) / 1000:8.1f}s  {outputs[0]}")
    by_target: dict[str, list[int]] = {}
    for start, end, outputs in steps:
        spent = by_target.setdefault(target_of(outputs[0]), [0, 0])
        spent[0] += end - start
        spent[1] += 1
    lines.append(f"slowest {min(top, len(by_target))} target(s), their steps' time summed:")
    for name, (spent, count) in sorted(by_target.items(),
                                       key=lambda kv: (-kv[1][0], kv[0]))[:top]:
        lines.append(f"  {spent / 1000:8.1f}s  {name} ({count} step(s))")
    return "\n".join(lines)


def self_test() -> int:
    log = "\n".join([
        "# ninja log v5",
        "0\t9000\t0\told/a.o\t1",
        "100\t9500\t0\told/b.o\t2",
        "0\t4000\t0\ttests/CMakeFiles/slow.dir/s.cpp.o\t3",
        "0\t4000\t0\ttests/CMakeFiles/slow.dir/s.cpp.dwo\t3",
        "10\t5000\t0\ttests/CMakeFiles/fast.dir/f.cpp.o\t4",
        "4000\t7000\t0\ttests/CMakeFiles/slow.dir/t.cpp.o\t5",
        "7000\t7500\t0\ttests/slow\t6",
    ])
    run = last_run(entries_of(log))
    assert [o for _, _, o in run][0] == "tests/CMakeFiles/slow.dir/s.cpp.o", run
    assert len(run) == 5, run
    steps = steps_of(run)
    assert len(steps) == 4, steps
    text = report(log, 2)
    assert "4 step(s)" in text and "7.5 min" not in text, text
    lines = text.splitlines()
    assert lines[2].strip().endswith("fast.dir/f.cpp.o"), lines
    assert any(line.strip().startswith("7.0s  slow (2 step(s))") for line in lines), lines
    assert report("# ninja log v5\n", 3) == "build time: the log holds no step"
    assert last_run([]) == []
    print("build_time_report self-test: 8 checks passed")
    return 0


def main() -> int:
    if sys.argv[1:] == ["--self-test"]:
        return self_test()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("build")
    parser.add_argument("--top", type=int, default=15)
    args = parser.parse_args()
    log = Path(args.build, ".ninja_log")
    if not log.is_file():
        print(f"build time: {log} does not exist; nothing was built by ninja here")
        return 0
    print(report(log.read_text(encoding="utf-8", errors="replace"), args.top))
    return 0


if __name__ == "__main__":
    sys.exit(main())
