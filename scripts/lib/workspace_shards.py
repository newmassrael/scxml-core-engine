#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The shards of the workspace suite: which part of `cargo test --workspace`
# one job of rust-workspace-tests.yml runs.
#
#   scripts/lib/workspace_shards.py --features <list> <k>/<N>   the units of shard k of N
#   scripts/lib/workspace_shards.py --features <list> --all     every unit
#   scripts/lib/workspace_shards.py --measure <job log>          the costs a run measured
#
# A unit is a line. `test <name>` is the integration test target of that name,
# in every workspace package that has one (`cargo test --workspace --test
# <name>` runs them all). `core` is everything else the suite runs: every
# package's library and binary unit tests, the doc tests, the examples it
# builds, and the product-line builds the workspace-tests gate adds
# (scripts/gates/workspace-tests.sh says what each of them is).
#
# WHY SHARDS. The suite outgrew one job. Measured on the run of 293defb01c
# (Rust Workspace Tests, job 132834013098): 29 minutes of build and 132 of tests,
# and the job ended at its 170-minute ceiling before the last package had run.
# A job of its own for each shard builds what its shard needs and runs a part.
#
# WHAT A SHARD MAY NOT DO is lose a target. The units are read from cargo, not
# listed: `cargo metadata` names every target of every workspace member, so a
# target added anywhere is a unit of some shard with no edit here, and
# `hook_ci_parity` holds the shards' union to every target the unsharded suite
# runs and each to exactly one shard. A target the unsharded suite does not run
# is not a unit either:
#   - `test = false` (the loom models, which need `--cfg loom`), and
#   - a target whose `required-features` the run does not enable, which cargo
#     leaves out of `cargo test --workspace` without a word and refuses by name
#     when it is asked for with `--test`. The features enabled are cargo's own
#     answer, the resolve of `cargo metadata --features <list>`.
#
# HOW THE UNITS ARE SPREAD. Longest first, each to the shard with the least
# work so far (the longest-processing-time rule), by the seconds the last
# measurement gave each unit; one it did not see is given the median of those it
# did. A stale cost moves a unit to another shard and changes nothing about which
# units run, so the table below is a matter of balance only. Re-measure after a
# change of the suite's shape with `--measure` on a job log of the unsharded
# lane, or on the logs of every shard of a sharded one put together.

from __future__ import annotations

import argparse
import datetime
import json
import re
import statistics
import subprocess
import sys

# Seconds per unit, from the run of 293defb01c (job 132834013098, 2026-11-15),
# the units of a minute or more. `core` is the library and binary unit tests of
# every package together.
MEASURED_SECONDS: dict[str, float] = {
    "a_cpp_kind_keeps_an_authors_names_apart_from_its_own": 1574,
    "a_kotlin_kind_keeps_an_authors_names_apart_from_its_own": 754,
    "codegen_depfile_content": 517,
    "cli_check": 507,
    "a_c_kind_keeps_an_authors_names_apart_from_its_own": 488,
    "core": 307,
    "diagnostic_fix_is_applicable": 289,
    "a_python_kind_keeps_an_authors_names_apart_from_its_own": 253,
    "a_go_kind_keeps_an_authors_names_apart_from_its_own": 252,
    "a_rust_kind_keeps_an_authors_names_apart_from_its_own": 217,
    "a_static_datamodel_holds_every_variable_to_a_type": 211,
    "diagnostic_corpus_schema": 191,
    "cli_lint_sweep": 189,
    "a_forge_declaration_is_spelled_the_way_the_collision_rule_says": 159,
    "a_declared_tree_reproduces_its_committed_tree": 143,
    "ecmascript_acceptance_parity": 142,
    "path_naming_parity": 124,
    "mutation_rounds_selection": 113,
    "forge_conformance": 98,
    "cli_expression_refusal": 93,
    "a_go_codec_keeps_an_authors_names_apart_from_its_own": 82,
    "a_python_codec_keeps_an_authors_names_apart_from_its_own": 74,
}

# The cost of a unit the table does not hold: the median of every unit the same
# run measured (`--measure` prints it), most of them a fraction of a second.
UNMEASURED_SECONDS = 0.1

CORE = "core"


def cargo_metadata(features: str) -> dict:
    """`cargo metadata` of the workspace, resolved with `features` enabled."""
    out = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--features", features],
        check=True, capture_output=True, text=True,
    )
    return json.loads(out.stdout)


def test_units(metadata: dict) -> list[str]:
    """Every integration test the unsharded suite runs, by name, sorted."""
    members = set(metadata["workspace_members"])
    enabled = {node["id"]: set(node["features"]) for node in metadata["resolve"]["nodes"]}
    names: set[str] = set()
    for package in metadata["packages"]:
        if package["id"] not in members:
            continue
        for target in package["targets"]:
            if target["kind"] != ["test"] or not target.get("test", True):
                continue
            required = target.get("required-features") or []
            for feature in required:
                if "/" in feature:
                    raise SystemExit(
                        f"workspace_shards: {package['name']}'s test `{target['name']}` "
                        f"requires `{feature}`, a feature of another package, which "
                        f"this reader does not resolve; teach it before sharding "
                        f"that target")
            if all(feature in enabled[package["id"]] for feature in required):
                names.add(target["name"])
    return sorted(names)


def units(metadata: dict) -> list[str]:
    """Every unit, `core` first."""
    return [CORE] + [f"test {name}" for name in test_units(metadata)]


def cost(unit: str) -> float:
    name = unit[len("test "):] if unit.startswith("test ") else unit
    return MEASURED_SECONDS.get(name, UNMEASURED_SECONDS)


def shards(all_units: list[str], count: int) -> list[list[str]]:
    """`all_units` spread over `count` shards: longest first, each to the
    shard with the least work so far, the lowest-numbered of equals."""
    load = [0.0] * count
    spread: list[list[str]] = [[] for _ in range(count)]
    for unit in sorted(all_units, key=lambda u: (-cost(u), u)):
        least = min(range(count), key=lambda i: (load[i], i))
        load[least] += cost(unit)
        spread[least].append(unit)
    return [sorted(shard) for shard in spread]


def measure(log_path: str) -> dict[str, float]:
    """The seconds each unit took in a job log of the lane: the time from one
    test binary's start to the next one's, `core` for the unit-test binaries."""
    started = re.compile(r"^(\S+?)Z .*Running\S* (.+?) \(")
    marks: list[tuple[datetime.datetime, str]] = []
    with open(log_path, encoding="utf-8", errors="replace") as log:
        for line in log:
            match = started.match(line)
            if match:
                at = datetime.datetime.fromisoformat(match.group(1)[:26])
                what = match.group(2)
                unit = what[len("tests/"):-len(".rs")] \
                    if what.startswith("tests/") and what.endswith(".rs") else CORE
                marks.append((at, unit))
    seconds: dict[str, float] = {}
    for (at, unit), (after, _) in zip(marks, marks[1:]):
        seconds[unit] = seconds.get(unit, 0.0) + (after - at).total_seconds()
    return seconds


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--features", help="the features the suite enables")
    parser.add_argument("--all", action="store_true", help="every unit")
    parser.add_argument("--measure", metavar="LOG", help="the costs a job log measured")
    parser.add_argument("shard", nargs="?", help="k/N, the shard k of N")
    args = parser.parse_args()

    if args.measure:
        seconds = measure(args.measure)
        for unit, value in sorted(seconds.items(), key=lambda kv: -kv[1]):
            print(f"{value:8.0f}  {unit}")
        if seconds:
            print(f"median {statistics.median(seconds.values()):.1f}", file=sys.stderr)
        return 0
    if not args.features:
        parser.error("--features is required, the features the suite enables")
    every = units(cargo_metadata(args.features))
    if args.all:
        print("\n".join(every))
        return 0
    match = re.fullmatch(r"([0-9]+)/([0-9]+)", args.shard or "")
    if not match or not 1 <= int(match.group(1)) <= int(match.group(2)):
        parser.error(f"a shard is k/N with 1 <= k <= N, got {args.shard!r}")
    index, count = int(match.group(1)), int(match.group(2))
    print("\n".join(shards(every, count)[index - 1]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
