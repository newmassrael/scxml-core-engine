#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: authoring-core.yml
#
# The authoring core turns a prose specification into a document, and it claims
# to know nothing about what the specification is about. That claim is the
# whole point of the package: the moment it knows one subject matter, it stops
# being usable for the next, and the way it stops being true is that somebody
# adds one helpful special case for the subject in front of them.
#
# So the claim is a test rather than a sentence in a README, and the test runs
# here. Two unlike guards, because a word list is only as wide as somebody's
# imagination: no source may spell a subject matter, and no source may name a
# location of its own (a hard-coded path is how a tool acquires a home).
#
# The refusal suite is on the same side of the push for a different reason. A
# guard that has never fired is a guard nobody has measured, and every refusal
# the core can make is built and asserted there -- each paired with the nearest
# situation that must NOT be refused, so a check that refuses everything fails.
#
# Python stdlib plus PyYAML and jsonschema. No engine toolchain.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# On the repository's Python environment where there is one (`sce_gate_python`): the run-driven
# cases run a statechart under the interpreter that runs the suite (`sys.executable`), through the
# AOT runtime and its Lua binding, which a distribution's python3-lupa 1.x cannot serve.
sce_gate_python
sce_gate_python_requires "authoring core" yaml jsonschema

# ONE PART OF THE SUITE, when `SCE_AUTHORING_CORE_SHARD=k/N` asks for one: the
# CI lane runs the suite with the generator built as N jobs (authoring-core.yml,
# ADR 0120 D3). Whole, it ran 2413 tests in 526 s after a 3.8-minute build of the
# generator (job 139000500233, 2026-11-25), and the run-driven cases, which
# spawn the generator, are most of that. A part is a set of the suite's test
# modules, read by scripts/lib/workspace_shards.py (`--unittest-modules`) the
# way discovery finds them, and held here to discovery itself: a module
# discovery loads that no plan names would run in no part, so the part refuses
# before it runs. Every job writes the whole plan beside its part, and the
# lane's last job holds the plans to one digest and every part to a report that
# its modules ran (`ctest_shards.py verify`). Unset, the whole suite runs by
# discovery, which is what a developer and the build machines run.
if [[ -n "${SCE_AUTHORING_CORE_SHARD:-}" ]]; then
    SHARD_PLAN="$SCE_REPO_ROOT/target/authoring-core-shard"
    rm -rf "$SHARD_PLAN"
    shard_units="$(python3 scripts/lib/workspace_shards.py \
        --unittest-modules tools/authoring/tests --out "$SHARD_PLAN" "$SCE_AUTHORING_CORE_SHARD")" \
        || sce_gate_cannot_run "scripts/lib/workspace_shards.py could not name shard $SCE_AUTHORING_CORE_SHARD"
    modules=()
    while IFS= read -r unit; do
        case "$unit" in
            "module "*) modules+=("${unit#module }") ;;
            "") ;;
            *) sce_gate_cannot_run "scripts/lib/workspace_shards.py named a unit it does not define: $unit" ;;
        esac
    done <<<"$shard_units"
    # Every module discovery loads is one the plan spread, read from the plan
    # every part wrote. A module that cannot be imported is named by discovery
    # as the test that failed to load it.
    discovered="$(cd tools/authoring && PYTHONPATH=. "$SCE_GATE_PYTHON" - <<'PY'
import unittest
import unittest.loader


def cases(suite):
    for item in suite:
        if isinstance(item, unittest.TestSuite):
            yield from cases(item)
        else:
            yield item


names = set()
for case in cases(unittest.defaultTestLoader.discover("tests", top_level_dir=".")):
    if isinstance(case, unittest.loader._FailedTest):
        names.add(case.id().split("unittest.loader._FailedTest.", 1)[-1])
    else:
        names.add(type(case).__module__)
print("\n".join(sorted(names)))
PY
)" || sce_gate_fail "unittest discovery of tools/authoring/tests did not run"
    planned="$(python3 -c 'import json, sys; plan = json.load(open(sys.argv[1])); print("\n".join(u.split(" ", 1)[1] for s in plan["shards"] for u in s["tests"]))' "$SHARD_PLAN/plan.json")"
    unplanned="$(comm -23 <(sort -u <<<"$discovered") <(sort -u <<<"$planned"))"
    [[ -z "$unplanned" ]] \
        || sce_gate_fail "discovery loads module(s) no part of the plan names, so they would run in no job: $unplanned"
    sce_gate_step "shard $SCE_AUTHORING_CORE_SHARD: ${#modules[@]} test module(s)"
    if (( ${#modules[@]} == 0 )); then
        python3 scripts/lib/ctest_shards.py report --out "$SHARD_PLAN" --executed 0 --disabled 0
        exit 0
    fi
    # `--durations 40` names the slowest tests of the part: the measurement that
    # replaces the per-module estimate the plan is spread by. unittest has it
    # from Python 3.12; an older interpreter runs the part without it.
    durations=()
    if "$SCE_GATE_PYTHON" -c 'import sys; sys.exit(sys.version_info < (3, 12))'; then
        durations=(--durations 40)
    fi
    ( cd tools/authoring && PYTHONPATH=. "$SCE_GATE_PYTHON" -m unittest ${durations[@]+"${durations[@]}"} "${modules[@]}" ) \
        || sce_gate_fail "authoring core regression (domain leak, or a refusal that no longer fires)"
    python3 scripts/lib/ctest_shards.py report --out "$SHARD_PLAN" --executed "${#modules[@]}" --disabled 0
    exit 0
fi

( cd tools/authoring && PYTHONPATH=. "$SCE_GATE_PYTHON" -m unittest discover -s tests -t . ) \
    || sce_gate_fail "authoring core regression (domain leak, or a refusal that no longer fires)"
