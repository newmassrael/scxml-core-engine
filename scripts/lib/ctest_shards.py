#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The shards of a ctest suite: which tests of a configured tree one job of a
# lane builds and runs. Five gates run their suite this way, each with its
# own selection of the tests its tree registers and its own variable:
#
#   gate            tree                selection    variable                  lane
#   cpp-suite       build               -LE c11      SCE_CTEST_SHARD           cpp-suite.yml
#   w3c-c11         build               -L c11       SCE_W3C_C11_SHARD         w3c-tests.yml
#   w3c-cpp         build               -L w3c_cpp   SCE_W3C_CPP_SHARD         w3c-tests.yml
#   w3c-wasm        build_wasm,         -L w3c_cpp   SCE_W3C_WASM_SHARD        w3c-tests.yml
#                   build_wasm_web      (one tree, SCE_W3C_WASM_TREE)
#   cpp-suite-wasm  build_wasm          (every)      SCE_CPP_SUITE_WASM_SHARD  cpp-suite.yml
#
# Unset, each gate builds its whole suite and runs it, which is what the build
# machines and a developer run. The plan's form and `verify` also serve the
# suites scripts/lib/workspace_shards.py spreads (w3c-rust's targets,
# regen-reproduces' steps and the targets tree-hygiene names): it writes its
# plans with `write_shard`, and each
# of those gates reports what it ran with `report`, so one verdict reads
# every sharded lane.
#
#   scripts/lib/ctest_shards.py plan --build <dir> [--label <label> |
#       --label-exclude <label>] --expect <count> --out <dir> <k>/<N>
#                                                  plan shard k of N into <dir>
#   scripts/lib/ctest_shards.py report --out <dir> --executed <n> --disabled <n>
#   scripts/lib/ctest_shards.py verify <dir>       hold the shards of one run together
#   scripts/lib/ctest_shards.py whole-tree-fixtures <build dir> < ctest JSON
#   scripts/lib/ctest_shards.py closure --build <dir> <test>...
#                                                 what running those tests needs built
#   scripts/lib/ctest_shards.py --self-test
#
# WHY SHARDS. Each suite's build outgrew one hosted job, or was most of one.
# The default target of the main tree was 75 minutes cold on the run of
# 12321e8233 (job 126014197137, 2026-11-03), and from 2026-11-21 every run of
# cpp-suite ended at its 140-minute ceiling inside that build (job
# 138540727626, fac3d022cf: the build started at 15:24 and was still compiling
# at 17:26). The default target is also more than the suite runs: it builds
# every example, the C11 half, which the w3c-c11 gate runs, and every program
# no non-c11 test names. The w3c-c11 job spent 13 of its 24 minutes building
# `sce_c11_tests` and 43 seconds running it (job 138891691071, 4c71fb062c).
# The emscripten tree built cold in 3328s before its tests ran at all (run
# 41956988917), and its job ran 4745s before it failed (job 137573089028). A
# shard builds what its own tests run and runs them, so each job pays for its
# part, and a red shard is reproduced by building and running that part alone.
#
# WHAT A SHARD MAY NOT DO is lose a test. The tests are read from ctest, not
# listed: `ctest --show-only=json-v1` with the gate's own selection names every
# test the unsharded gate runs, so a test registered anywhere is in some shard
# with no edit here. `plan` refuses unless the tests it spread are exactly the
# count the gate registered with the same selection (`--expect`), every shard
# computes the whole plan, and `verify` holds the plans of all the shards of a
# run to one digest and every shard to a report that its tests ran. One kind
# of test is in no shard, and it is named: the setup of a fixture whose command
# builds the whole tree (`sce_build_is_current`). A shard builds the closure of
# what it runs itself, so that setup would only rebuild everything the shard
# left out; the run excludes it with `--fixture-exclude-setup`, as the
# mutation harness does.
#
# WHAT A TEST NEEDS BUILT is what its command, its ENVIRONMENT and its
# REQUIRED_FILES name, where those are outputs of the build graph: an
# executable, a `-DNAME=<path>` handed to a script, a program a variable
# hands it to spawn. Ninja builds each such output with everything it is made
# from. A test that runs a program it does not name gets no program in a fresh
# shard and fails there, which is the loud way: name it in REQUIRED_FILES. A
# test that reads every file a directory of the build holds would fail the
# quiet way, reading whatever its shard happened to build, so a directory of
# the build tree named in REQUIRED_FILES needs every output under it built
# (`c11_headers_are_cxx_consumable`, backends/c/tests, names the generated
# trees it compiles every header of). Only REQUIRED_FILES is read that way: a
# variable naming the build tree's root would otherwise need all of it.
# Tests joined by a fixture (a setup, its cleanup and the tests that require
# it) are one unit and go to one shard, since ctest adds the setup to any run
# of a test that requires it.
#
# HOW THE UNITS ARE SPREAD. The cost of a shard is the number of compile and
# link steps in the union of its units' closures, read from ninja's own graph
# (`ninja -t graph`), so a library every test links is counted once per
# shard and two tests that share a program cost the program once together.
# Largest closure first, each unit to the shard whose cost after taking it is
# least (the longest-processing-time rule over unions); of equals, the shard
# holding the fewest tests, then the lowest-numbered. The same configure gives
# every job the same plan.

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
READER = REPO_ROOT / "scripts" / "lib" / "ctest_registered_commands.cmake"

# Exit status of a plan that could not be made on this machine (a tool is
# missing, the tree is not a Ninja tree), as against one refused for the tree.
CANNOT_PLAN = 3


class CannotPlan(Exception):
    """The machine cannot answer; nothing is claimed about the tree."""


class Refused(Exception):
    """The tree does not admit the plan the gate needs."""


# ── Reading ctest ───────────────────────────────────────────────────────────


def properties_of(test: dict) -> dict:
    return {entry.get("name"): entry.get("value") for entry in test.get("properties") or []}


def as_list(value) -> list[str]:
    """A test property that ctest may spell as a list or as one string."""
    if value is None or value is False:
        return []
    if isinstance(value, list):
        return [str(v) for v in value]
    return [part for part in str(value).split(";") if part]


def read_registered(text: str) -> dict[str, list[str]]:
    """The output of ctest_registered_commands.cmake: name to command, the
    first registration of a name winning, as ctest's own does."""
    registered: dict[str, list[str]] = {}
    for line in text.splitlines():
        fields = line.split("\t")
        if len(fields) > 1 and fields[0] not in registered:
            registered[fields[0]] = fields[1:]
    return registered


def command_of(test: dict, registered: dict[str, list[str]]) -> list[str]:
    return test.get("command") or registered.get(test.get("name", ""), [])


def whole_tree_build_fixtures(tests: list[dict], build_real: str,
                              registered: dict[str, list[str]] | None = None) -> list[str]:
    """The fixtures whose setup is `cmake --build <this tree>` with no target,
    in the order ctest lists them."""
    registered = registered or {}
    names: list[str] = []
    for test in tests:
        setup = as_list(properties_of(test).get("FIXTURES_SETUP"))
        command = command_of(test, registered)
        if not setup or not command or not os.path.basename(command[0]).startswith("cmake"):
            continue
        if "--build" not in command or "--target" in command or "-t" in command:
            continue
        at = command.index("--build") + 1
        if at >= len(command) or os.path.realpath(command[at]) != build_real:
            continue
        for name in setup:
            if name not in names:
                names.append(name)
    return names


def named_paths(test: dict, registered: dict[str, list[str]]) -> list[str]:
    """Every absolute path a test's command, ENVIRONMENT and REQUIRED_FILES
    name, a `NAME=<path>` argument's path included."""
    found: list[str] = []

    def add(candidate: str) -> None:
        if candidate and os.path.isabs(candidate) and candidate not in found:
            found.append(candidate)

    for token in command_of(test, registered):
        add(token)
        if "=" in token:
            add(token.split("=", 1)[1])
    props = properties_of(test)
    for entry in as_list(props.get("ENVIRONMENT")):
        _, sep, value = entry.partition("=")
        if sep:
            add(value)
            for piece in value.split(os.pathsep):
                add(piece)
    for required in as_list(props.get("REQUIRED_FILES")):
        add(required)
    return found


def build_outputs(paths: list[str], build_real: str, outputs: set[str]) -> list[str]:
    """Those of `paths` that are outputs of the build graph, spelled as ninja
    spells them: relative to the tree, or absolute for an output outside it."""
    named: list[str] = []
    for path in paths:
        real = os.path.realpath(path)
        relative = os.path.relpath(real, build_real)
        inside = relative != os.curdir and not (
            relative == os.pardir or relative.startswith(os.pardir + os.sep))
        for spelling in ([relative] if inside else []) + [real, os.path.normpath(path)]:
            if spelling in outputs:
                if spelling not in named:
                    named.append(spelling)
                break
    return named


def outputs_under(paths: list[str], build_real: str, outputs: set[str]) -> list[str]:
    """Every output of the build graph under those of `paths` that are a
    directory of the build tree: a path inside the tree that is no output
    itself and has outputs beneath it."""
    found: list[str] = []
    for path in paths:
        relative = os.path.relpath(os.path.realpath(path), build_real)
        if relative == os.curdir or relative == os.pardir \
                or relative.startswith(os.pardir + os.sep) or relative in outputs:
            continue
        prefix = relative.rstrip(os.sep) + "/"
        found.extend(o for o in sorted(outputs) if o.startswith(prefix) and o not in found)
    return found


def needed_outputs(test: dict, registered: dict[str, list[str]], build_real: str,
                   outputs: set[str]) -> list[str]:
    """What a test needs built: the outputs its command, ENVIRONMENT and
    REQUIRED_FILES name, and every output under a directory REQUIRED_FILES
    names."""
    named = build_outputs(named_paths(test, registered), build_real, outputs)
    required = as_list(properties_of(test).get("REQUIRED_FILES"))
    for output in outputs_under(required, build_real, outputs):
        if output not in named:
            named.append(output)
    return named


def parse_targets(text: str) -> set[str]:
    """`ninja -t targets all`: one `<output>: <rule>` per line."""
    return {line.rsplit(": ", 1)[0] for line in text.splitlines() if ": " in line}


# ── Reading ninja's graph ───────────────────────────────────────────────────

_ARROW = re.compile(r'^"([^"]+)" -> "([^"]+)"(?: \[(.*)\])?$')
_EDGE_NODE = re.compile(r'^"([^"]+)" \[label="([^"]*)", shape=ellipse\]$')
_NODE = re.compile(r'^"([^"]+)" \[label="(.*)"\]$')
_RULE_LABEL = re.compile(r'label="\s*([^"]*)"')


class Graph:
    """What `ninja -t graph` prints, as the edges that make each node.

    An edge of one input and one output is printed as an arrow from the input
    to the output labelled with its rule; any other edge as a node of its own
    (an ellipse labelled with its rule), an arrow from it to each output and
    an arrow with no head from each input to it."""

    def __init__(self, text: str) -> None:
        self.path_of: dict[str, str] = {}
        self.node_of: dict[str, str] = {}
        self.rule: dict[str, str] = {}
        self.made_by: dict[str, str] = {}
        self.inputs: dict[str, list[str]] = {}
        self._closures: dict[str, frozenset[str]] = {}
        for raw in text.splitlines():
            line = raw.strip()
            arrow = _ARROW.match(line)
            if arrow:
                source, target, attrs = arrow.group(1), arrow.group(2), arrow.group(3) or ""
                if "arrowhead=none" in attrs:
                    self.inputs.setdefault("e:" + target, []).append(source)
                elif "label=" in attrs:
                    edge = "s:" + target
                    label = _RULE_LABEL.search(attrs)
                    self.rule[edge] = label.group(1).strip() if label else ""
                    self.made_by[target] = edge
                    self.inputs.setdefault(edge, []).append(source)
                else:
                    self.made_by[target] = "e:" + source
                continue
            edge_node = _EDGE_NODE.match(line)
            if edge_node:
                self.rule["e:" + edge_node.group(1)] = edge_node.group(2)
                continue
            node = _NODE.match(line)
            if node:
                self.path_of[node.group(1)] = node.group(2)
                self.node_of[node.group(2)] = node.group(1)

    def closure(self, path: str) -> frozenset[str]:
        """The compile and link steps building `path` takes, by edge."""
        if path not in self._closures:
            self._closures[path] = self.steps(path, costs)
        return self._closures[path]

    def steps(self, path: str, keep) -> frozenset[str]:
        """The edges building `path` takes whose rule `keep` accepts, by edge.
        `closure` asks for the compiles and links; the interface-standard
        gate asks for the custom commands that generate its headers
        (tools/interface-standard/interface_standard.py, `shard-plan`)."""
        start = self.node_of.get(path)
        if start is None:
            return frozenset()
        seen_edges: set[str] = set()
        seen_nodes = {start}
        stack = [start]
        while stack:
            edge = self.made_by.get(stack.pop())
            if edge is None or edge in seen_edges:
                continue
            seen_edges.add(edge)
            for node in self.inputs.get(edge, []):
                if node not in seen_nodes:
                    seen_nodes.add(node)
                    stack.append(node)
        return frozenset(e for e in seen_edges if keep(self.rule.get(e, "")))


def costs(rule: str) -> bool:
    """A compile or a link: the steps a shard's build spends its time on. CMake
    names them `<LANG>_COMPILER__<target>...` and `<LANG>_<KIND>_LINKER__...`."""
    return rule.startswith(("C_COMPILER", "CXX_COMPILER")) or "_LINKER__" in rule


# ── The plan ────────────────────────────────────────────────────────────────


class Unit:
    """Tests that go to one shard together: one test, or every test one
    fixture joins."""

    def __init__(self, tests: list[str]) -> None:
        self.tests = sorted(tests)
        self.name = self.tests[0]
        self.outputs: list[str] = []
        self.closure: frozenset[str] = frozenset()


def units_of(tests: list[dict], excluded: set[str]) -> tuple[list[Unit], list[str]]:
    """The units of `tests`, and the tests left out of every unit: the setup
    and cleanup tests of the fixtures in `excluded`."""
    parent: dict[str, str] = {}

    def find(name: str) -> str:
        while parent[name] != name:
            parent[name] = parent[parent[name]]
            name = parent[name]
        return name

    def join(a: str, b: str) -> None:
        ra, rb = find(a), find(b)
        if ra != rb:
            parent[max(ra, rb)] = min(ra, rb)

    left_out: list[str] = []
    kept: list[str] = []
    for test in tests:
        name = test["name"]
        props = properties_of(test)
        provides = as_list(props.get("FIXTURES_SETUP")) + as_list(props.get("FIXTURES_CLEANUP"))
        if any(fixture in excluded for fixture in provides):
            left_out.append(name)
            continue
        parent[name] = name
        kept.append(name)
        for fixture in provides + as_list(props.get("FIXTURES_REQUIRED")):
            if fixture in excluded:
                continue
            key = "fixture:" + fixture
            if key not in parent:
                parent[key] = key
            join(name, key)
    groups: dict[str, list[str]] = {}
    for name in kept:
        groups.setdefault(find(name), []).append(name)
    return [Unit(members) for members in groups.values()], sorted(left_out)


def assign(units: list[Unit], count: int) -> list[list[Unit]]:
    """Largest closure first, each to the shard whose compile and link steps
    after taking it are fewest; of equals, the one holding the fewest tests
    (so units that build nothing, a disabled test or a script, still spread),
    then the lowest-numbered."""
    loads: list[set[str]] = [set() for _ in range(count)]
    shards: list[list[Unit]] = [[] for _ in range(count)]
    held = [0] * count
    for unit in sorted(units, key=lambda u: (-len(u.closure), u.name)):
        best = min(range(count),
                   key=lambda s: (len(loads[s]) + len(unit.closure - loads[s]), held[s], s))
        held[best] += len(unit.tests)
        loads[best] |= unit.closure
        shards[best].append(unit)
    return shards


def regex_of(names: list[str]) -> str:
    """A ctest `-R` expression matching exactly `names`."""
    return "^(" + "|".join(re.escape(name) for name in names) + ")$" if names else ""


def plan_of(tests: list[dict], registered: dict[str, list[str]], build_real: str,
            outputs: set[str], graph: Graph | None, count: int) -> dict:
    """The whole plan, every shard of it, as one JSON-ready object."""
    if count < 1:
        raise Refused(f"a plan of {count} shard(s)")
    names = [t.get("name", "") for t in tests]
    doubled = sorted({n for n in names if names.count(n) > 1})
    if doubled:
        raise Refused(f"ctest names more than one test {doubled}; a shard selects by name")
    fixtures = whole_tree_build_fixtures(tests, build_real, registered)
    units, left_out = units_of(tests, set(fixtures))
    by_name = {t["name"]: t for t in tests}
    for unit in units:
        closure: set[str] = set()
        for name in unit.tests:
            test = by_name[name]
            if properties_of(test).get("DISABLED"):
                continue
            for output in needed_outputs(test, registered, build_real, outputs):
                if output not in unit.outputs:
                    unit.outputs.append(output)
                if graph is not None:
                    closure |= graph.closure(output)
        unit.outputs.sort()
        unit.closure = frozenset(closure)
    shards = assign(units, count)
    spread = []
    for shard in shards:
        shard_tests = sorted(n for u in shard for n in u.tests)
        shard_outputs = sorted({o for u in shard for o in u.outputs})
        cost = len(frozenset().union(*(u.closure for u in shard))) if shard else 0
        spread.append({"tests": shard_tests, "targets": shard_outputs, "steps": cost})
    assigned = [n for s in spread for n in s["tests"]]
    if sorted(assigned + left_out) != sorted(names):
        raise Refused("the shards and the tests left out are not every test, each once")
    shape = {"count": count, "shards": [s["tests"] for s in spread],
             "left_out": left_out, "fixtures": fixtures}
    digest = hashlib.sha256(json.dumps(shape, sort_keys=True).encode()).hexdigest()
    return {"count": count, "digest": digest, "registered": len(names),
            "fixtures": fixtures, "left_out": left_out, "shards": spread}


# ── What a run of some tests needs built ────────────────────────────────────
#
# The smallest rerun of a test that failed (scripts/lib/repro.py) builds what
# that test runs and nothing else, which is a shard of one: the same reading
# of the test's command, environment and required files, the setups and
# cleanups of the fixtures it requires, which ctest adds to the run, and the
# whole-tree build fixtures left out, whose setup would build everything.


def closure_of(tests: list[dict], registered: dict[str, list[str]], build_real: str,
               outputs: set[str], names: list[str]) -> dict:
    """The outputs a run of exactly `names` needs built, the tests that run
    with them, and the whole-tree build fixtures the run must leave out."""
    by_name = {t.get("name"): t for t in tests}
    missing = [n for n in names if n not in by_name]
    if missing:
        raise Refused(f"ctest registers no test named {missing}")
    fixtures = whole_tree_build_fixtures(tests, build_real, registered)
    excluded = set(fixtures)
    run = list(names)
    required: set[str] = set()
    grew = True
    while grew:
        grew = False
        for name in list(run):
            for fixture in as_list(properties_of(by_name[name]).get("FIXTURES_REQUIRED")):
                if fixture not in excluded and fixture not in required:
                    required.add(fixture)
                    grew = True
        for test in tests:
            props = properties_of(test)
            provides = set(as_list(props.get("FIXTURES_SETUP"))) | set(
                as_list(props.get("FIXTURES_CLEANUP")))
            if provides & required and test["name"] not in run:
                run.append(test["name"])
                grew = True
    targets: list[str] = []
    for name in run:
        test = by_name[name]
        if properties_of(test).get("DISABLED"):
            continue
        # The same reading a shard's plan makes (`needed_outputs`), so a rerun of
        # one test builds what the shard that ran it built.
        for output in needed_outputs(test, registered, build_real, outputs):
            if output not in targets:
                targets.append(output)
    return {"targets": sorted(targets), "fixtures": fixtures, "run": run}


class ClosureReader:
    """Each build tree read once, and the closure of any tests of it."""

    def __init__(self) -> None:
        self._trees: dict[str, tuple] = {}

    def closure(self, build: str, names: list[str]) -> dict:
        build_real = os.path.realpath(build)
        if build_real not in self._trees:
            self._trees[build_real] = read_tests(build)
        tests, registered, outputs = self._trees[build_real]
        return closure_of(tests, registered, build_real, outputs, names)


# ── Running the tools ───────────────────────────────────────────────────────


def run(argv: list[str], what: str, stream: str = "stdout") -> str:
    try:
        done = subprocess.run(argv, capture_output=True, text=True, check=False)
    except FileNotFoundError as error:
        raise CannotPlan(f"{what}: {argv[0]} is not installed ({error})") from error
    if done.returncode != 0:
        raise CannotPlan(f"{what} failed ({done.returncode}): {done.stderr.strip()[-2000:]}")
    return done.stdout if stream == "stdout" else done.stderr


def selection_args(label: str | None, label_exclude: str | None) -> list[str]:
    """The ctest arguments that select a gate's tests: its label, the
    complement of a label, or every test the tree registers."""
    if label and label_exclude:
        raise Refused(f"a selection is one label or one excluded label, not both "
                      f"({label!r} and {label_exclude!r})")
    if label:
        return ["-L", label]
    if label_exclude:
        return ["-LE", label_exclude]
    return []


def require_ninja(build: str) -> str:
    """The tree's real path, or CannotPlan when ninja does not build it."""
    build_real = os.path.realpath(build)
    cache = Path(build_real, "CMakeCache.txt")
    generator = ""
    if cache.is_file():
        for line in cache.read_text(encoding="utf-8", errors="replace").splitlines():
            if line.startswith("CMAKE_GENERATOR:INTERNAL="):
                generator = line.split("=", 1)[1]
    if generator != "Ninja":
        raise CannotPlan(f"{build} is built by '{generator or 'no generator'}'; a shard "
                         f"reads its build closure from ninja")
    return build_real


def read_tests(build: str) -> tuple[list[dict], dict[str, list[str]], set[str]]:
    """Every test the tree registers, the commands its configure step names,
    and the outputs of its build graph."""
    build_real = require_ninja(build)
    listed = json.loads(run(["ctest", "--test-dir", build_real, "--show-only=json-v1"],
                            "ctest --show-only=json-v1"))
    registered = read_registered(run(["cmake", f"-DSCE_CTEST_TREE={build_real}", "-P",
                                      str(READER)], "ctest_registered_commands.cmake",
                                     stream="stderr"))
    outputs = parse_targets(run(["ninja", "-C", build_real, "-t", "targets", "all"],
                                "ninja -t targets all"))
    return listed.get("tests", []), registered, outputs


def read_tree(build: str, selection: list[str]) -> tuple[list[dict], dict[str, list[str]], str, set[str], Graph]:
    build_real = require_ninja(build)
    listed = json.loads(run(["ctest", "--test-dir", build_real, *selection,
                             "--show-only=json-v1"], "ctest --show-only=json-v1"))
    tests = listed.get("tests", [])
    registered = read_registered(run(["cmake", f"-DSCE_CTEST_TREE={build_real}", "-P",
                                      str(READER)], "ctest_registered_commands.cmake",
                                     stream="stderr"))
    outputs = parse_targets(run(["ninja", "-C", build_real, "-t", "targets", "all"],
                                "ninja -t targets all"))
    named = sorted({o for t in tests
                    for o in needed_outputs(t, registered, build_real, outputs)})
    graph = Graph(run(["ninja", "-C", build_real, "-t", "graph", *named], "ninja -t graph")
                  if named else "")
    return tests, registered, build_real, outputs, graph


def shard_arg(text: str) -> tuple[int, int]:
    match = re.fullmatch(r"([0-9]+)/([0-9]+)", text or "")
    if not match or not 1 <= int(match.group(1)) <= int(match.group(2)):
        raise argparse.ArgumentTypeError(f"a shard is k/N with 1 <= k <= N, got {text!r}")
    return int(match.group(1)), int(match.group(2))


def write_shard(out: Path, plan: dict, index: int) -> None:
    """The plan, and the four files the gate reads for shard `index` (1-based)."""
    out.mkdir(parents=True, exist_ok=True)
    shard = plan["shards"][index - 1]
    (out / "plan.json").write_text(json.dumps({**plan, "shard": index}, indent=1) + "\n",
                                   encoding="utf-8")
    (out / "tests").write_text("".join(f"{n}\n" for n in shard["tests"]), encoding="utf-8")
    (out / "targets").write_text("".join(f"{o}\n" for o in shard["targets"]), encoding="utf-8")
    (out / "run-regex").write_text(regex_of(shard["tests"]) + "\n", encoding="utf-8")
    (out / "fixture-regex").write_text(regex_of(plan["fixtures"]) + "\n", encoding="utf-8")


def describe(plan: dict, index: int) -> str:
    lines = [f"plan {plan['digest'][:16]}: {plan['registered']} test(s) in "
             f"{plan['count']} shard(s)"]
    for i, shard in enumerate(plan["shards"], 1):
        mark = "  <- this job" if i == index else ""
        lines.append(f"  shard {i}: {len(shard['tests'])} test(s), {len(shard['targets'])} "
                     f"output(s), {shard['steps']} compile and link step(s){mark}")
    if plan["left_out"]:
        lines.append(f"  in no shard, the setup of a whole-tree build fixture: "
                     f"{', '.join(plan['left_out'])}")
    return "\n".join(lines)


def verify(directory: Path) -> list[str]:
    """What is wrong with the shards of one run, read from the plan and the
    report each shard left; nothing when they are the whole suite, each test
    once, every one of them run."""
    plans: dict[int, dict] = {}
    reports: dict[int, dict] = {}
    faults: list[str] = []
    for plan_path in sorted(directory.rglob("plan.json")):
        plan = json.loads(plan_path.read_text(encoding="utf-8"))
        index = plan.get("shard")
        if index in plans:
            faults.append(f"shard {index} left two plans ({plan_path})")
        plans[index] = plan
        report_path = plan_path.with_name("report.json")
        if report_path.is_file():
            reports[index] = json.loads(report_path.read_text(encoding="utf-8"))
    if not plans:
        return [f"no shard left a plan under {directory}"]
    digests = {p["digest"] for p in plans.values()}
    if len(digests) != 1:
        faults.append("the shards planned differently, so a test can fall between "
                      f"them: {sorted((i, p['digest'][:16]) for i, p in plans.items())}")
    count = next(iter(plans.values()))["count"]
    for index in range(1, count + 1):
        if index not in plans:
            faults.append(f"shard {index} of {count} left no plan")
            continue
        plan = plans[index]
        report = reports.get(index)
        expected = len(plan["shards"][index - 1]["tests"])
        if report is None:
            faults.append(f"shard {index} of {count} left no report: its tests did not "
                          f"all run")
        elif report.get("executed", 0) + report.get("disabled", 0) != expected:
            faults.append(f"shard {index} ran {report.get('executed')} and skipped "
                          f"{report.get('disabled')} disabled of its {expected} test(s)")
    extra = sorted(i for i in plans if not isinstance(i, int) or not 1 <= i <= count)
    if extra:
        faults.append(f"plans for shards outside 1..{count}: {extra}")
    return faults


# ── Self-test ───────────────────────────────────────────────────────────────


def self_test() -> int:
    checks = 0

    def check(condition: bool, what: str) -> None:
        nonlocal checks
        checks += 1
        if not condition:
            raise AssertionError(what)

    build = "/b"
    graph_text = "\n".join([
        "digraph ninja {",
        'rankdir="LR"',
        "node [fontsize=10, shape=box, height=0.25]",
        '"0x1" [label="tests/a"]',
        '"0xe1" [label="CXX_EXECUTABLE_LINKER__a_RelWithDebInfo", shape=ellipse]',
        '"0xe1" -> "0x1"',
        '"0x2" -> "0xe1" [arrowhead=none]',
        '"0x3" -> "0xe1" [arrowhead=none]',
        '"0x2" [label="tests/CMakeFiles/a.dir/a.cpp.o"]',
        '"0xe2" [label="CXX_COMPILER__a_unscanned_RelWithDebInfo", shape=ellipse]',
        '"0xe2" -> "0x2"',
        '"0x4" -> "0xe2" [arrowhead=none]',
        '"0x5" -> "0xe2" [arrowhead=none style=dotted]',
        '"0x4" [label="/src/a.cpp"]',
        '"0x5" [label="cmake_object_order_depends_target_a"]',
        '"0xe5" [label="phony", shape=ellipse]',
        '"0xe5" -> "0x5"',
        '"0x6" -> "0xe5" [arrowhead=none style=dotted]',
        '"0x6" [label="gen/a_sm.h"]',
        '"0x7" -> "0x6" [label=" CUSTOM_COMMAND"]',
        '"0x7" [label="/src/a.scxml"]',
        '"0x3" [label="sce/libcore.a"]',
        '"0x8" -> "0x3" [label=" CXX_STATIC_LIBRARY_LINKER__core_RelWithDebInfo"]',
        '"0x8" [label="sce/CMakeFiles/core.dir/core.cpp.o"]',
        '"0x9" -> "0x8" [label=" CXX_COMPILER__core_RelWithDebInfo"]',
        '"0x9" [label="/src/core.cpp"]',
        '"0x10" [label="tests/b"]',
        '"0x11" -> "0x10" [label=" CXX_EXECUTABLE_LINKER__b_RelWithDebInfo"]',
        '"0x11" [label="sce/libcore.a"]',
        "}",
    ])
    graph = Graph(graph_text)
    a = graph.closure("tests/a")
    check(len(a) == 4, f"tests/a takes a link, a compile, the library's link and its "
                       f"compile, not the custom command or the phony: {sorted(a)}")
    check(graph.closure("tests/missing") == frozenset(), "an unknown output costs nothing")
    generation = graph.steps("tests/a", lambda rule: rule == "CUSTOM_COMMAND")
    check(generation == frozenset({"s:0x6"}),
          f"tests/a takes the one custom command that writes gen/a_sm.h, through the "
          f"order-only phony, and no compile or link: {sorted(generation)}")

    targets = parse_targets("tests/a: CXX_EXECUTABLE_LINKER__a\ntests/b: CXX_EXECUTABLE_"
                            "LINKER__b\n/out/side.bin: CUSTOM_COMMAND\nall: phony\n")
    check(targets == {"tests/a", "tests/b", "/out/side.bin", "all"}, f"targets: {targets}")

    def test(name, command=None, **props):
        entry = {"name": name, "properties": [{"name": k, "value": v} for k, v in props.items()]}
        if command is not None:
            entry["command"] = command
        return entry

    registered = read_registered("t_script\t/usr/bin/python3\t/src/x.py\t/b/tests/a\n"
                                 "t_script\t/ignored\n")
    check(registered["t_script"][-1] == "/b/tests/a", "the first registration wins")

    named = named_paths(test("p", ["/usr/bin/cmake", "-DEXE=/b/tests/b", "-P", "x.cmake"],
                             ENVIRONMENT=["SCE_BIN=/b/tests/a", "LD_LIBRARY_PATH=/l:/b/lib"],
                             REQUIRED_FILES=["/out/side.bin"]), {})
    for path in ("/b/tests/b", "/b/tests/a", "/b/lib", "/out/side.bin"):
        check(path in named, f"{path} is named by the command, environment or required "
                             f"files: {named}")
    check("x.cmake" not in named, "a relative token names nothing")
    outputs = build_outputs(named, build, targets)
    check(set(outputs) == {"tests/a", "tests/b", "/out/side.bin"},
          f"the named paths that are build outputs: {outputs}")

    generated = targets | {"gen/x_sm.h", "gen/deep/y_sm.h", "general/z.h"}
    scanner = test("scan", ["/usr/bin/cmake", "-DROOTS=/b/gen|/b/other", "-P", "s.cmake"],
                   REQUIRED_FILES=["/b/gen", "/b", "/b/tests/a"],
                   ENVIRONMENT=["SCE_TREE=/b/tests"])
    needed = needed_outputs(scanner, {}, build, generated)
    check(set(needed) == {"gen/x_sm.h", "gen/deep/y_sm.h", "tests/a"},
          f"a directory REQUIRED_FILES names needs every output under it, and no "
          f"sibling that shares its prefix, the tree's root or a directory a "
          f"variable names: {needed}")
    check(outputs_under(["/elsewhere/gen", "/b/missing"], build, generated) == [],
          "a directory outside the tree, or one with no output under it, needs nothing")

    whole = test("sce_build_is_current", ["/usr/bin/cmake", "--build", "/b", "--parallel"],
                 FIXTURES_SETUP=["sce_built"])
    partial = test("builds_one", ["/usr/bin/cmake", "--build", "/b", "--target", "x"],
                   FIXTURES_SETUP=["one_built"])
    check(whole_tree_build_fixtures([whole, partial], build) == ["sce_built"],
          "only the untargeted build of this tree is a whole-tree build fixture")

    tests = [
        whole,
        test("ta", ["/b/tests/a"], FIXTURES_REQUIRED=["sce_built"]),
        test("tb", ["/b/tests/b"], FIXTURES_REQUIRED=["sce_built"]),
        test("server_up", ["/b/tests/a", "--serve"], FIXTURES_SETUP=["server"]),
        test("uses_server", ["/b/tests/b"], FIXTURES_REQUIRED=["server", "sce_built"]),
        test("server_down", ["/b/tests/a", "--stop"], FIXTURES_CLEANUP=["server"]),
        test("t_script"),
        test("off", ["/b/tests/b"], DISABLED=True),
        test("pure", ["/usr/bin/sh", "/src/check.sh"]),
    ]
    units, left_out = units_of(tests, {"sce_built"})
    check(left_out == ["sce_build_is_current"], f"left out: {left_out}")
    joined = [u.tests for u in units if "uses_server" in u.tests]
    check(joined == [["server_down", "server_up", "uses_server"]],
          f"a fixture joins its setup, cleanup and users: {joined}")

    for count in (1, 2, 3, 4, 9):
        plan = plan_of(tests, registered, build, targets, graph, count)
        assigned = [n for s in plan["shards"] for n in s["tests"]]
        check(len(assigned) == len(set(assigned)), f"{count}: a test in two shards")
        check(sorted(assigned + plan["left_out"]) == sorted(t["name"] for t in tests),
              f"{count}: the shards and the left-out setup are every test")
        check(plan == plan_of(list(tests), registered, build, targets, Graph(graph_text), count),
              f"{count}: the same tree plans the same way")
        check(len(plan["shards"]) == count, f"{count}: one entry per shard")
        for shard in plan["shards"]:
            if "off" in shard["tests"] and len(shard["tests"]) == 1:
                check(shard["targets"] == [], "a disabled test asks for nothing to be built")
            if "t_script" in shard["tests"]:
                check("tests/a" in shard["targets"],
                      "a command read from the configure step names its outputs")
        server = [i for i, s in enumerate(plan["shards"]) if "uses_server" in s["tests"]]
        check(all("server_up" in plan["shards"][i]["tests"] for i in server),
              f"{count}: a fixture's users run where its setup does")
    check(plan_of(tests, registered, build, targets, graph, 2)["digest"]
          != plan_of(tests, registered, build, targets, graph, 3)["digest"],
          "a different count is a different plan")

    joined_run = closure_of(tests, registered, build, targets, ["uses_server"])
    check(joined_run["fixtures"] == ["sce_built"], f"the whole-tree fixture is left out: {joined_run}")
    check(sorted(joined_run["run"]) == ["server_down", "server_up", "uses_server"],
          f"a test runs with its fixture's setup and cleanup, not its other users: {joined_run}")
    check(joined_run["targets"] == ["tests/a", "tests/b"],
          f"and needs what all three run built: {joined_run}")
    alone = closure_of(tests, registered, build, targets, ["ta"])
    check(alone["run"] == ["ta"] and alone["targets"] == ["tests/a"],
          f"a test whose one fixture is the whole-tree build needs only its program: {alone}")
    check(closure_of(tests, registered, build, targets, ["off"])["targets"] == [],
          "a disabled test needs nothing built")
    try:
        closure_of(tests, registered, build, targets, ["no_such_test"])
        check(False, "a test the tree does not register is refused")
    except Refused:
        check(True, "refused")

    two = assign(list(units), 2)
    check(sum(1 for s in two if s) == 2,
          "units that build nothing still spread over the shards")

    shared = [Unit(["x1"]), Unit(["x2"]), Unit(["y"])]
    shared[0].closure = frozenset({"lib", "x"})
    shared[1].closure = frozenset({"lib", "x"})
    shared[2].closure = frozenset({"lib", "y", "z"})
    spread = assign(shared, 2)
    together = [sorted(u.name for u in s) for s in spread]
    check(["x1", "x2"] in together, f"two units of one closure cost it once together: {together}")

    try:
        plan_of([test("d", ["/b/tests/a"]), test("d", ["/b/tests/b"])], {}, build, targets,
                graph, 2)
        check(False, "a name ctest gives twice is refused")
    except Refused:
        check(True, "refused")

    names = ["a.b", "x+y", "plain_name", "w3c[1]"]
    expression = re.compile(regex_of(names))
    check(all(expression.fullmatch(n) for n in names), "the run expression matches its tests")
    check(not expression.fullmatch("aXb") and not expression.fullmatch("plain_name2"),
          "and nothing else")
    check(regex_of([]) == "", "no tests, no expression")

    check(selection_args("c11", None) == ["-L", "c11"], "a label selects its tests")
    check(selection_args(None, "c11") == ["-LE", "c11"], "an excluded label selects the rest")
    check(selection_args(None, None) == [], "no label selects every test the tree registers")
    try:
        selection_args("c11", "c11")
        check(False, "a label and its exclusion at once are refused")
    except Refused:
        check(True, "refused")

    import tempfile
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch)
        plan = plan_of(tests, registered, build, targets, graph, 2)
        for index in (1, 2):
            write_shard(root / f"shard-{index}", plan, index)
            expected = len(plan["shards"][index - 1]["tests"])
            disabled = 1 if "off" in plan["shards"][index - 1]["tests"] else 0
            (root / f"shard-{index}" / "report.json").write_text(json.dumps(
                {"shard": index, "executed": expected - disabled, "disabled": disabled}))
        check(verify(root) == [], f"a whole run verifies: {verify(root)}")
        (root / "shard-2" / "report.json").unlink()
        check(any("left no report" in f for f in verify(root)), "a shard that did not "
                                                                "finish is named")
        other = plan_of(tests, registered, build, targets, graph, 2)
        other["digest"] = "0" * 64
        write_shard(root / "shard-2", other, 2)
        check(any("planned differently" in f for f in verify(root)),
              "shards that planned differently are refused")
        (root / "shard-2" / "plan.json").unlink()
        check(any("shard 2 of 2 left no plan" in f for f in verify(root)),
              "a shard that left no plan is named")
        check(verify(root / "nothing") != [], "no plans at all is not a pass")
        check((root / "shard-1" / "run-regex").read_text().startswith("^("),
              "the gate is handed a run expression")

    print(f"ctest_shards self-test: {checks} checks passed")
    return 0


# ── Command line ────────────────────────────────────────────────────────────


def main() -> int:
    if sys.argv[1:] == ["--self-test"]:
        return self_test()
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    planning = sub.add_parser("plan", help="plan shard k of N into --out")
    planning.add_argument("--build", required=True)
    planning.add_argument("--label", help="run only the tests carrying this label")
    planning.add_argument("--label-exclude", help="run every test but those carrying "
                                                  "this label")
    planning.add_argument("--expect", type=int, required=True,
                          help="the tests the gate counted with the same selector")
    planning.add_argument("--out", required=True)
    planning.add_argument("shard", type=shard_arg)
    reporting = sub.add_parser("report", help="record what this shard ran")
    reporting.add_argument("--out", required=True)
    reporting.add_argument("--executed", type=int, required=True)
    reporting.add_argument("--disabled", type=int, required=True)
    verifying = sub.add_parser("verify", help="hold the shards of one run together")
    verifying.add_argument("directory")
    fixtures = sub.add_parser("whole-tree-fixtures",
                              help="the whole-tree build fixtures, from ctest JSON on stdin")
    fixtures.add_argument("build")
    closing = sub.add_parser("closure", help="what running exactly these tests needs built")
    closing.add_argument("--build", required=True)
    closing.add_argument("names", nargs="+")
    args = parser.parse_args()

    if args.command == "closure":
        try:
            print(json.dumps(ClosureReader().closure(args.build, args.names), indent=1))
        except CannotPlan as error:
            print(f"ctest_shards: {error}", file=sys.stderr)
            return CANNOT_PLAN
        except Refused as error:
            print(f"ctest_shards: {error}", file=sys.stderr)
            return 1
        return 0

    if args.command == "whole-tree-fixtures":
        tests = json.load(sys.stdin).get("tests", [])
        names = whole_tree_build_fixtures(tests, os.path.realpath(args.build))
        print("|".join(re.escape(name) for name in names))
        return 0
    if args.command == "verify":
        faults = verify(Path(args.directory))
        for fault in faults:
            print(f"ctest_shards: {fault}", file=sys.stderr)
        if not faults:
            print("ctest_shards: every shard planned alike and ran its tests")
        return 1 if faults else 0
    if args.command == "report":
        out = Path(args.out)
        plan = json.loads((out / "plan.json").read_text(encoding="utf-8"))
        (out / "report.json").write_text(json.dumps(
            {"shard": plan["shard"], "executed": args.executed, "disabled": args.disabled})
            + "\n", encoding="utf-8")
        return 0

    index, count = args.shard
    try:
        selection = selection_args(args.label, args.label_exclude)
        tests, registered, build_real, outputs, graph = read_tree(args.build, selection)
        if len(tests) != args.expect:
            raise Refused(f"ctest's JSON names {len(tests)} test(s) where the gate counted "
                          f"{args.expect} with the same selector")
        plan = plan_of(tests, registered, build_real, outputs, graph, count)
    except CannotPlan as error:
        print(f"ctest_shards: {error}", file=sys.stderr)
        return CANNOT_PLAN
    except Refused as error:
        print(f"ctest_shards: {error}", file=sys.stderr)
        return 1
    write_shard(Path(args.out), plan, index)
    print(describe(plan, index), file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
