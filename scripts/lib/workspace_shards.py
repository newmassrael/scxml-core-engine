#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The shards of a suite that is not a ctest suite: which part of it one job of
# a lane runs. Three gates run their suite this way, each with its own units
# and its own variable:
#
#   gate              units                                variable              lane
#   workspace-tests   the workspace's test targets, `core`  SCE_WORKSPACE_SHARD   rust-workspace-tests.yml
#   w3c-rust          sce-rust-tests' test targets, `core`  SCE_W3C_RUST_SHARD    w3c-tests.yml
#   regen-reproduces  the regeneration's steps              SCE_REGEN_SHARD       regen-reproduces.yml
#
# Unset, each gate runs its whole suite, which is what the build machines and a
# developer run. (scripts/lib/ctest_shards.py is the same for a ctest suite.)
#
#   scripts/lib/workspace_shards.py --features <list> <k>/<N>   the units of shard k of N
#   scripts/lib/workspace_shards.py --features <list> --all     every unit
#   scripts/lib/workspace_shards.py --package <name> <k>/<N>    one package's suite
#   scripts/lib/workspace_shards.py --regen <k>/<N>             the regeneration's steps
#   scripts/lib/workspace_shards.py ... --out <dir> <k>/<N>     and write the plan there
#   scripts/lib/workspace_shards.py --measure <job log>         the costs a run measured
#   scripts/lib/workspace_shards.py --self-test
#
# A unit is a line. `test <name>` is the integration test target of that name,
# in every workspace package that has one (`cargo test --workspace --test
# <name>` runs them all), or in the one package `--package` names. `core` is
# everything else the suite runs: every package's library and binary unit
# tests, the doc tests, the examples it builds, and for the workspace the
# product-line builds the workspace-tests gate adds
# (scripts/gates/workspace-tests.sh says what each of them is). `step <name>`
# is a step of scripts/regen_all_committed_trees.sh with the trees it owns
# (scripts/lib/sce_generated_tree.sh, `sce_regen_steps`).
#
# WHY SHARDS. The workspace suite outgrew one job. Measured on the run of
# 293defb01c (Rust Workspace Tests, job 132834013098): 29 minutes of build and
# 132 of tests, and the job ended at its 170-minute ceiling before the last
# package had run. The Rust W3C arm took 1605 s (job 138891691369, 4c71fb062c),
# 7 minutes of it building the target `w3c` and 15 building every other target
# of sce-rust-tests, with about ten seconds of tests in all. The regeneration
# took 1666 s (job 138988228323, 1499ef6b36), 22 minutes of it regenerating.
# A job of its own for each shard builds what its shard needs and runs a part.
#
# WHAT A SHARD MAY NOT DO is lose a unit. The units are read, not listed: `cargo
# metadata` names every target of every workspace member, so a target added
# anywhere is a unit of some shard with no edit here, and the regeneration's
# own `--list` names every step. `hook_ci_parity` holds the shards' union to
# every unit the unsharded suite runs and each to exactly one shard. A target
# the unsharded suite does not run is not a unit either:
#   - `test = false` (the loom models, which need `--cfg loom`), and
#   - a target whose `required-features` the run does not enable, which cargo
#     leaves out of `cargo test --workspace` without a word and refuses by name
#     when it is asked for with `--test`. The features enabled are cargo's own
#     answer, the resolve of `cargo metadata --features <list>`.
# With `--out`, every job also writes the whole plan, in the form
# ctest_shards.py writes and reads, and the lane's last job holds the plans of
# all its shards to one digest and every shard to a report that its units ran
# (`ctest_shards.py verify`), as it does for a ctest suite.
#
# HOW THE UNITS ARE SPREAD. Longest first, each to the shard with the least
# work so far (the longest-processing-time rule), by the seconds the last
# measurement gave each unit; one it did not see is given the median of those it
# did. A stale cost moves a unit to another shard and changes nothing about which
# units run, so the tables below are a matter of balance only. Re-measure after a
# change of the suite's shape with `--measure` on a job log of the unsharded
# lane, or on the logs of every shard of a sharded one put together.

from __future__ import annotations

import argparse
import datetime
import hashlib
import json
import re
import statistics
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Callable

REPO_ROOT = Path(__file__).resolve().parents[2]

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

# Seconds per unit of one package's suite, by package. No run has measured a
# unit of sce-rust-tests alone: its lane builds every target before it runs
# any, and its gate prints the log after the run, so every `Running` line of
# the log carries one instant and `--measure` reads nothing from it. Its
# tests ran in about ten seconds in all; the time is the build, under the
# release profile's thin LTO and one codegen unit (job 138891691369). So these
# are estimates of each unit's build from that job, to be replaced when a run
# measures one. Every target but `w3c` was built in 912 s over the 132
# binaries the step linked, about 7 s a target, which is the cost of a target
# the table does not hold. Two units build more than one file's worth, and
# their figures are shares of the 436 s step that built the dependencies, the
# build script's trees, the library and `w3c`: `core`, whose library unit-test
# binary compiles every generated machine of the library a second time, under
# `cfg(test)`, and `w3c`, which compiles a module for each of the 202 W3C cases
# and links them with every machine.
PACKAGE_SECONDS: dict[str, dict[str, float]] = {
    "sce-rust-tests": {"core": 150, "w3c": 60},
}
PACKAGE_UNMEASURED_SECONDS: dict[str, float] = {"sce-rust-tests": 7}

# Seconds per step of the regeneration, from a run of every step as its own
# action on the build machines (`bx --actions`, 2026-11-23), less the time each
# spent resolving the generator: a job of regen-reproduces copies the generator
# its dependency built into the worktree it regenerates, where resolving it is
# one check of the binary's witness. A step the table does not hold is given
# the median of the table.
REGEN_SECONDS: dict[str, float] = {
    "runtime-rust": 0,
    "runtime-kotlin": 0,
    "runtime-go": 0,
    "runtime-python": 0,
    "runtime-swift": 0,
    "runtime-c": 1,
    "kotlin-tests": 63,
    "kotlin-method-size": 152,
    "go-w3c": 4,
    "go-integration": 34,
    "python-w3c": 4,
    "python-integration": 36,
    "swift-tests": 76,
    "go-round-trip": 101,
    "mesh-rust": 133,
    "mesh-cpp": 72,
    "mesh-kotlin": 81,
    "mesh-swift": 194,
    "mesh-swift-wss": 204,
    "specs": 73,
    "forge-goldens": 374,
    "embed-manifest": 122,
}

CORE = "core"

Cost = Callable[[str], float]


def unit_name(unit: str) -> str:
    """A unit less its kind: the target of `test <name>`, the step of
    `step <name>`, and `core` as it is."""
    kind, _, name = unit.partition(" ")
    return name if kind in ("test", "step") and name else unit


def cost_by(table: dict[str, float], unmeasured: float) -> Cost:
    return lambda unit: table.get(unit_name(unit), unmeasured)


def cargo_metadata(features: str | None) -> dict:
    """`cargo metadata` of the workspace, resolved with `features` enabled."""
    argv = ["cargo", "metadata", "--format-version", "1"]
    if features:
        argv += ["--features", features]
    out = subprocess.run(argv, check=True, capture_output=True, text=True)
    return json.loads(out.stdout)


def test_units(metadata: dict, package: str | None = None) -> list[str]:
    """Every integration test the unsharded suite runs, by name, sorted: of
    every workspace member, or of the one named `package`."""
    members = set(metadata["workspace_members"])
    enabled = {node["id"]: set(node["features"]) for node in metadata["resolve"]["nodes"]}
    names: set[str] = set()
    found = False
    for member in metadata["packages"]:
        if member["id"] not in members:
            continue
        if package is not None and member["name"] != package:
            continue
        found = True
        for target in member["targets"]:
            if target["kind"] != ["test"] or not target.get("test", True):
                continue
            required = target.get("required-features") or []
            for feature in required:
                if "/" in feature:
                    raise SystemExit(
                        f"workspace_shards: {member['name']}'s test `{target['name']}` "
                        f"requires `{feature}`, a feature of another package, which "
                        f"this reader does not resolve; teach it before sharding "
                        f"that target")
            if all(feature in enabled[member["id"]] for feature in required):
                names.add(target["name"])
    if package is not None and not found:
        raise SystemExit(f"workspace_shards: no workspace member is named {package!r}")
    return sorted(names)


def units(metadata: dict, package: str | None = None) -> list[str]:
    """Every unit, `core` first."""
    return [CORE] + [f"test {name}" for name in test_units(metadata, package)]


def regen_steps() -> list[str]:
    """The steps of the regeneration, in the order a run of every step takes
    them, from the one list that names them (`sce_regen_steps`)."""
    out = subprocess.run(
        ["bash", "-c", "source scripts/lib/sce_generated_tree.sh && sce_regen_steps"],
        cwd=REPO_ROOT, check=True, capture_output=True, text=True,
    )
    steps = [line.strip() for line in out.stdout.splitlines() if line.strip()]
    if not steps:
        raise SystemExit("workspace_shards: sce_regen_steps named no step")
    return [f"step {step}" for step in steps]


def regen_outputs(steps: list[str], bx_toml: Path | None = None) -> list[str]:
    """What the steps write, as the globs each one's action declares as its
    outputs (`[actions.regen-<step>]` in bx.toml): the files a run of those
    steps alone may take from the tree and has to put back."""
    with open(bx_toml or REPO_ROOT / "bx.toml", "rb") as declared:
        actions = tomllib.load(declared).get("actions", {})
    globs: list[str] = []
    for unit in steps:
        step = unit_name(unit)
        action = actions.get(f"regen-{step}")
        if action is None:
            raise SystemExit(f"workspace_shards: bx.toml declares no action regen-{step}, "
                             f"so nothing says which trees the step {step} writes")
        for glob in action.get("outputs", []):
            if glob not in globs:
                globs.append(glob)
    return globs


def shards(all_units: list[str], count: int, cost: Cost) -> list[list[str]]:
    """`all_units` spread over `count` shards: longest first, each to the
    shard with the least work so far, the lowest-numbered of equals. Each
    shard keeps its units in the order `all_units` gives them, which for the
    regeneration is the order its steps run in."""
    load = [0.0] * count
    spread: list[set[str]] = [set() for _ in range(count)]
    for unit in sorted(all_units, key=lambda u: (-cost(u), u)):
        least = min(range(count), key=lambda i: (load[i], i))
        load[least] += cost(unit)
        spread[least].add(unit)
    return [[u for u in all_units if u in shard] for shard in spread]


def plan_of(all_units: list[str], count: int, cost: Cost) -> dict:
    """The whole plan, every shard of it, in the form ctest_shards.py writes
    and `verify` reads; refused unless it holds every unit, each once."""
    if count < 1:
        raise SystemExit(f"workspace_shards: a plan of {count} shard(s)")
    doubled = sorted({u for u in all_units if all_units.count(u) > 1})
    if doubled:
        raise SystemExit(f"workspace_shards: the suite names a unit twice: {doubled}")
    spread = shards(all_units, count, cost)
    assigned = [u for shard in spread for u in shard]
    if sorted(assigned) != sorted(all_units):
        raise SystemExit("workspace_shards: the shards are not every unit, each once")
    shape = {"count": count, "shards": spread, "left_out": [], "fixtures": []}
    digest = hashlib.sha256(json.dumps(shape, sort_keys=True).encode()).hexdigest()
    return {"count": count, "digest": digest, "registered": len(all_units),
            "fixtures": [], "left_out": [],
            "shards": [{"tests": shard, "targets": [],
                        "seconds": round(sum(cost(u) for u in shard))}
                       for shard in spread]}


def write_plan(out: Path, plan: dict, index: int, outputs: list[str] | None = None) -> None:
    """The plan and this shard's part of it, written by ctest_shards.py's own
    writer, so the lane's last job reads them as it reads a ctest plan; and,
    for the regeneration, the outputs of this shard's steps."""
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from ctest_shards import write_shard  # noqa: E402  (the sibling helper)

    write_shard(out, plan, index)
    if outputs is not None:
        (out / "outputs").write_text("".join(f"{g}\n" for g in outputs), encoding="utf-8")


def measure(log_path: str) -> dict[str, float]:
    """The seconds each unit took in a job log of a lane. For a cargo suite,
    the time from one test binary's start to the next one's, `core` for the
    unit-test binaries; for the regeneration, the stages each step reports
    (`=== step <name>`, then `    <stage>: <n>s` per stage)."""
    started = re.compile(r"^(\S+?)Z .*Running\S* (.+?) \(")
    step_line = re.compile(r"^(?:\S+Z )?=== step (\S+)\s*$")
    stage_line = re.compile(r"^(?:\S+Z )?    (.+): ([0-9]+)s\s*$")
    marks: list[tuple[datetime.datetime, str]] = []
    seconds: dict[str, float] = {}
    step = ""
    with open(log_path, encoding="utf-8", errors="replace") as log:
        for raw in log:
            line = raw.rstrip("\n")
            stepped = step_line.match(line)
            if stepped:
                step = stepped.group(1)
                continue
            staged = stage_line.match(line)
            if staged and step:
                seconds[step] = seconds.get(step, 0.0) + int(staged.group(2))
                continue
            match = started.match(line)
            if match:
                at = datetime.datetime.fromisoformat(match.group(1)[:26])
                what = match.group(2)
                unit = what[len("tests/"):-len(".rs")] \
                    if what.startswith("tests/") and what.endswith(".rs") else CORE
                marks.append((at, unit))
    for (at, unit), (after, _) in zip(marks, marks[1:]):
        seconds[unit] = seconds.get(unit, 0.0) + (after - at).total_seconds()
    return seconds


def self_test() -> int:
    import tempfile

    checks = 0

    def check(condition: bool, what: str) -> None:
        nonlocal checks
        checks += 1
        if not condition:
            raise AssertionError(what)

    workspace = [CORE] + [f"test t{i:02d}" for i in range(23)]
    regen = [f"step {s}" for s in REGEN_SECONDS] + ["step not-yet-measured"]
    for every, cost in ((workspace, cost_by(MEASURED_SECONDS, UNMEASURED_SECONDS)),
                        (regen, cost_by(REGEN_SECONDS,
                                        statistics.median(REGEN_SECONDS.values())))):
        for count in (1, 2, 3, 4, 7, len(every), len(every) + 3):
            spread = shards(every, count, cost)
            assigned = [u for s in spread for u in s]
            check(len(spread) == count, f"{count}: one list per shard")
            check(len(assigned) == len(set(assigned)), f"{count}: a unit in two shards")
            check(sorted(assigned) == sorted(every), f"{count}: the shards are every unit")
            for shard in spread:
                check(shard == [u for u in every if u in shard],
                      f"{count}: a shard keeps the order the suite gives its units")
            plan = plan_of(every, count, cost)
            check(plan == plan_of(list(every), count, cost),
                  f"{count}: the same suite plans the same way")
            check([s["tests"] for s in plan["shards"]] == spread,
                  f"{count}: the plan holds the shards")
        check(plan_of(every, 2, cost)["digest"] != plan_of(every, 3, cost)["digest"],
              "a different count is a different plan")

    regen_cost = cost_by(REGEN_SECONDS, statistics.median(REGEN_SECONDS.values()))
    check(regen_cost("step not-yet-measured") == statistics.median(REGEN_SECONDS.values()),
          "a step the table does not hold costs the median")
    spread = shards(regen, 3, regen_cost)
    check(sum(1 for s in spread if "step forge-goldens" in s) == 1
          and "step runtime-rust" == next(s for s in spread if "step runtime-rust" in s)[0],
          "the step the generator is compiled from comes first in its shard")
    package = cost_by(PACKAGE_SECONDS["sce-rust-tests"],
                      PACKAGE_UNMEASURED_SECONDS["sce-rust-tests"])
    suite = [CORE] + [f"test t{i:03d}" for i in range(120)] + ["test w3c"]
    spread = shards(suite, 4, package)
    check(not any(CORE in s and "test w3c" in s for s in spread),
          "the two largest units of sce-rust-tests go to two shards")
    check(unit_name("test w3c") == "w3c" and unit_name("step specs") == "specs"
          and unit_name(CORE) == CORE, "a unit's name is read past its kind")

    try:
        plan_of(["core", "core"], 2, package)
        check(False, "a unit named twice is refused")
    except SystemExit:
        check(True, "refused")

    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch)
        bx = root / "bx.toml"
        bx.write_text('[actions.regen-a]\noutputs = ["x/**", "y/f.txt"]\n'
                      '[actions.regen-b]\noutputs = ["x/**", "z/**"]\n'
                      '[actions.regen-c]\nrun = "true"\n', encoding="utf-8")
        check(regen_outputs(["step a", "step b"], bx) == ["x/**", "y/f.txt", "z/**"],
              "the outputs of two steps, each glob once, in order")
        check(regen_outputs(["step c"], bx) == [], "an action that writes nothing")
        try:
            regen_outputs(["step missing"], bx)
            check(False, "a step with no action is refused")
        except SystemExit:
            check(True, "refused")

        sys.path.insert(0, str(Path(__file__).resolve().parent))
        from ctest_shards import verify  # noqa: E402  (the sibling helper)

        plan = plan_of(regen, 3, regen_cost)
        for index in (1, 2, 3):
            out = root / "shards" / f"shard-{index}"
            write_plan(out, plan, index, regen_outputs([], bx))
            ran = len(plan["shards"][index - 1]["tests"])
            (out / "report.json").write_text(json.dumps(
                {"shard": index, "executed": ran, "disabled": 0}), encoding="utf-8")
            check((out / "tests").read_text(encoding="utf-8").split()[::2] ==
                  ["step"] * ran, "the part a shard runs is written beside the plan")
            check((out / "outputs").is_file(), "a regeneration shard writes its outputs")
        check(verify(root / "shards") == [], f"a whole run verifies: {verify(root / 'shards')}")
        (root / "shards" / "shard-3" / "report.json").unlink()
        check(any("left no report" in f for f in verify(root / "shards")),
              "a shard whose units did not all run is named")

        log = root / "job.log"
        log.write_text("\n".join([
            "2026-11-25T05:13:40.1000000Z === step runtime-kotlin",
            "2026-11-25T05:13:41.1000000Z     Setting aside the generated files: 1s",
            "2026-11-25T05:13:42.1000000Z     Kotlin runtime's standard engine tree: 3s",
            "=== step forge-goldens",
            "    Regenerating the forge conformance goldens: 374s",
            "2026-11-25T01:26:36.1000000Z      Running unittests src/lib.rs (target/x)",
            "2026-11-25T01:26:46.1000000Z      Running tests/w3c.rs (target/y)",
            "2026-11-25T01:26:50.1000000Z      Running tests/zz.rs (target/z)",
        ]) + "\n", encoding="utf-8")
        seconds = measure(str(log))
        check(seconds.get("runtime-kotlin") == 4 and seconds.get("forge-goldens") == 374,
              f"the stages of each step are summed: {seconds}")
        check(seconds.get(CORE) == 10 and seconds.get("w3c") == 4,
              f"a binary runs until the next one starts: {seconds}")

    print(f"workspace_shards self-test: {checks} checks passed")
    return 0


def main() -> int:
    if sys.argv[1:] == ["--self-test"]:
        return self_test()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--features", help="the features the suite enables")
    parser.add_argument("--package", help="one package's suite, not the workspace's")
    parser.add_argument("--regen", action="store_true",
                        help="the regeneration's steps, not a cargo suite")
    parser.add_argument("--all", action="store_true", help="every unit")
    parser.add_argument("--out", metavar="DIR", help="write the whole plan and this "
                                                     "shard's part of it into DIR")
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
    if args.regen and (args.package or args.features):
        parser.error("--regen spreads the regeneration's steps; it takes no "
                     "--package or --features")
    if args.regen:
        every = regen_steps()
        cost = cost_by(REGEN_SECONDS, statistics.median(REGEN_SECONDS.values()))
    elif args.package:
        every = units(cargo_metadata(args.features), args.package)
        cost = cost_by(PACKAGE_SECONDS.get(args.package, {}),
                       PACKAGE_UNMEASURED_SECONDS.get(args.package, UNMEASURED_SECONDS))
    elif args.features:
        every = units(cargo_metadata(args.features))
        cost = cost_by(MEASURED_SECONDS, UNMEASURED_SECONDS)
    else:
        parser.error("--features is required, the features the suite enables "
                     "(or --package, or --regen)")
    if args.all:
        print("\n".join(every))
        return 0
    match = re.fullmatch(r"([0-9]+)/([0-9]+)", args.shard or "")
    if not match or not 1 <= int(match.group(1)) <= int(match.group(2)):
        parser.error(f"a shard is k/N with 1 <= k <= N, got {args.shard!r}")
    index, count = int(match.group(1)), int(match.group(2))
    plan = plan_of(every, count, cost)
    part = plan["shards"][index - 1]["tests"]
    if args.out:
        write_plan(Path(args.out), plan, index, regen_outputs(part) if args.regen else None)
    print("\n".join(part))
    return 0


if __name__ == "__main__":
    sys.exit(main())
