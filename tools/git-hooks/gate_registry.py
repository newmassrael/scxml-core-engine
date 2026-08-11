#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The gate registry: what gates exist, what triggers each, what each needs
# built first, and what each costs.
#
# Gates are identified by a slug, never by position. The previous table
# keyed them by run order ("1", "1b", "2c", "8b"), which made the identifier
# and the schedule the same thing: reordering renamed gates, so the order
# could not be improved without breaking every reference to it, and each new
# gate had to be wedged in with a letter suffix instead of taking a place of
# its own. The evidence that this had stopped working was in the hook
# itself — three gates carried block comments naming a number two lower than
# their own label, left behind by an earlier renumbering, four gates
# mirroring one workflow were split across two number families, five gates
# were missing from the header list, and every label read "N/8" while
# seventeen gates existed.
#
# With slugs, order is derived rather than declared: `deps` says what must
# run first, `cost_s` says what is cheap, and the runner sorts by both. A
# gate added tomorrow lands in the right place without touching any other
# entry.
#
# Which gate a change needs is answered by the `paths:` filter on the CI
# workflow it mirrors, so those filters are read out of
# `.github/workflows/*.yml` rather than restated here — the hook cannot
# drift from CI the way a hand-copied table would, and a workflow that gains
# a path starts triggering its gate on the same commit that widens CI.
#
# Three rules keep the filtering from becoming a coverage hole, which is the
# failure mode a selective hook invites:
#
#   1. A changed path matching NO known glob — not a gate trigger and not
#      the inert list below — forces the FULL run. A new top-level directory
#      therefore gets more verification, never less.
#   2. Gates with no CI counterpart carry an explicit local trigger with the
#      reason written next to it, rather than being silently always-on or
#      silently never-on.
#   3. A gate whose trigger is the catch-all `**` is selected always and
#      counts as classifying nothing. Its workflow runs on every push, so a
#      match carries no information about whether a path is understood —
#      letting it count would mark every path in the repository known and
#      rule 1 would never fire again.

from __future__ import annotations

import argparse
import json
import pathlib
import re
import subprocess
import sys
from pathlib import Path

# ── Gate table ────────────────────────────────────────────────────
#
# `workflows` names the CI workflow(s) a gate mirrors; its `paths:` filter
# is the trigger. `extra` adds locally-known triggers for what the
# workflow's filter cannot express, and `local` is for gates CI has no
# counterpart for. `deps` names gates whose output this one executes.
#
# `cost_s` is wall-clock seconds, MEASURED by `scripts/gate --measure --all`
# on a warm tree, not estimated. Warm is the right basis: a push happens on
# a tree the developer has just built, and a cold first build is paid once
# in whatever order the gates run. Two limits are worth stating rather than
# discovering later. The numbers come from a full run, so a gate that
# benefits from an earlier gate having warmed the cargo cache reads cheaper
# here than it would in isolation — which is the number that matters for
# ordering a full run, and the wrong one for judging a single gate. And the
# clock is whole seconds, so everything under a second reads 0.
#
# Re-measure after any gate's work changes materially; a stale cost is a
# wrong order, not a wrong comment. The value that motivated all of this
# was `ledger-citations`: the hook's prose called it "sub-second per
# workspace" and it sat last of seventeen, while a measurement puts the
# whole gate at 145s — still nowhere near last, and the prose was
# describing one step of it.
GATES: dict[str, dict] = {
    # Prerequisite, not a gate: several gates execute target/debug/sce-codegen.
    # Triggered by its own sources, and forced on by its dependents.
    "codegen-build": {
        "local": ["sce-build/**", "Cargo.toml", "Cargo.lock"],
        "no_ci_reason": "not a check — it produces the binary other gates "
                        "run. CI builds it per workflow that needs it "
                        "(`debug_only_codegen_builds_drop_the_stale_release_binary` "
                        "counts those steps), so there is no verdict here "
                        "for a workflow to mirror.",
        "cost_s": 1,
        "summary": "build target/debug/sce-codegen",
    },
    # Structural check over the Rust module tree — only a .rs add/remove can
    # break it, so the Rust trigger set is exactly right.
    "rust-modrs-drift": {
        "local": ["**/*.rs"],
        "no_ci_reason": "CI catches the same defect as a compile error in "
                        "rust-workspace-tests.yml — an aggregator naming a "
                        "missing subdir is E0432, and an unnamed subdir is "
                        "dead code its importers fail on. This gate exists "
                        "to say so in under a second instead of after 30s "
                        "of compilation, so a bypass loses speed, not "
                        "coverage.",
        "cost_s": 0,
        "summary": "mod.rs <-> subdirectory drift",
    },
    # Guards the clang degraded-AST regression in the manifest emitter.
    # Its own lane in the embed workflow rather than a step in the smoke
    # job: it needs clang and a second, and the smoke build takes seven
    # minutes. Until that lane existed the emitter was outside every CI
    # trigger — `embed-vendor-smoke.yml` filtered on package/verify but
    # not on `emit_embed_manifest.sh` — so this ran at push time and
    # nowhere else, which `--no-verify` skips.
    "embed-manifest-failfast": {
        "workflows": ["embed-vendor-smoke.yml"],
        "runner_workflow": True,
        "cost_s": 0,
        "summary": "emit_embed_manifest fail-fast case",
    },
    # `roadmap_marker_gate` reads every tracked file and
    # `workflow_trigger_coverage` reads every workflow, so tree-hygiene.yml
    # declares no `paths:` filter — no glob list is both correct and
    # narrower than the tree. That missing filter is what makes this gate
    # unconditional, derived the same way every other workflow-backed
    # trigger is rather than hand-set to "always".
    "tree-hygiene": {
        "workflows": ["tree-hygiene.yml"],
        "runner_workflow": True,
        "cost_s": 2,
        "summary": "tree-wide marker + trigger + parity gates",
    },
    "clippy": {
        "workflows": ["clippy-check.yml"],
        "runner_workflow": True,
        "cost_s": 0,
        "summary": "cargo clippy --workspace --all-targets",
    },
    "nostd-mcu": {
        "workflows": ["sce-rust-runtime-no-std.yml"],
        "runner_workflow": True,
        "deps": ["codegen-build"],
        "cost_s": 1,
        "summary": "no_std MCU build + clippy + probes",
    },
    # Mirrors doc-check.yml. The numbered table mapped this to
    # sce-rust-runtime-no-std.yml while the gate's own comment named
    # doc-check.yml; the two share the `backends/rust/runtime/**` trigger,
    # so the mismatch never showed as a missed run except for edits to
    # doc-check.yml itself.
    "rustdoc-links": {
        "workflows": ["doc-check.yml"],
        "runner_workflow": True,
        "cost_s": 4,
        "summary": "cargo doc broken intra-doc links, both profiles",
    },
    "embed-vendor": {
        "workflows": ["embed-vendor-smoke.yml"],
        # Not delegated: the lane builds sce-codegen first and calls a
        # sibling gate in the same workflow, so its steps are a
        # superset of this gate rather than a copy of it.
        "extra": ["sce/include/**", "embed/MANIFEST.json"],
        # The most expensive gate in the set: three scratch-directory builds,
        # one of which packages embed/ from scratch and builds a consumer
        # against it.
        "cost_s": 399,
        "summary": "embed manifest drift + payload lag + consumer smoke",
    },
    # Mirrors the Rust workspace suite only. `w3c-tests.yml` is deliberately
    # NOT listed: it has no `paths:` filter (it runs on every CI push) and
    # it drives the C++ ctest suite, which this hook does not mirror at all.
    # Mapping it here would make its catch-all the trigger for the hook's
    # longest gate and, worse, would classify every path — defeating the
    # unclassified-forces-FULL rule that keeps this file honest.
    # `extra` covers inputs the workflow filter misses because they are
    # `include_str!`-ed into the suite rather than compiled: the acceptance
    # doc, the wire schemas, and the forge-AST export schema.
    "workspace-tests": {
        "workflows": ["rust-workspace-tests.yml"],
        # Not delegated: the lane adds `--no-fail-fast` so one CI run
        # reports every failure, while the hook stops at the first —
        # a reporting choice, not a different verification.
        "extra": ["docs/SCE_ACCEPTED_SUBSET.md", "schemas/**", "apis/**"],
        "cost_s": 138,
        "summary": "cargo test --workspace --features cli",
    },
    "drift-suites": {
        "workflows": ["drift-verify.yml"],
        "runner_workflow": True,
        "cost_s": 1,
        "summary": "committed-tree drift + sourcemap, serial",
    },
    # forge-conformance.yml verifies the language arms in parallel jobs, and
    # its path filter is workflow-wide: a change under
    # backends/*/forge-runtime/** starts all of them in CI. These gates
    # therefore fire together too, which is the parity we want. The split is
    # for attribution — a failure names its arm — and each `extra` widens
    # the trigger past the workflow filter rather than narrowing it, so a
    # change outside forge-runtime still reaches the arm it could break.
    "forge-go": {
        "workflows": ["forge-conformance.yml"],
        "runner_workflow": True,
        "extra": ["backends/go/**"],
        "deps": ["codegen-build"],
        "cost_s": 11,
        "summary": "Go forge conformance regenerate + test",
    },
    "forge-rust": {
        "workflows": ["forge-conformance.yml"],
        "runner_workflow": True,
        "extra": ["backends/rust/**"],
        # Warm reads as 0; the release profile it needs is a separate build
        # tree from every other gate, so a cold run pays that once.
        "cost_s": 0,
        "summary": "Rust forge conformance (numerical, release)",
    },
    "forge-python": {
        "workflows": ["forge-conformance.yml"],
        "runner_workflow": True,
        "extra": ["backends/python/**"],
        "cost_s": 11,
        "summary": "Python forge conformance (numerical)",
    },
    "forge-cpp": {
        "workflows": ["forge-conformance.yml"],
        # Not delegated: the lane tees each arm's output to a log file
        # it uploads as a build artifact; the gate builds in a temp dir
        # and keeps nothing. Delegating would drop the artifact.
        "extra": ["backends/cpp/**", "sce/**"],
        "deps": ["codegen-build"],
        "cost_s": 6,
        "summary": "C++ forge conformance build + test",
    },
    # Catches codegen breakage in the example documents (the namespace
    # migration that broke them shipped green otherwise) and lints every
    # document this repository authors. The trigger comes from the
    # workflow's `paths:` like every other mirrored gate; it used to be a
    # `local` list, and that list named `examples/**` while the gate's own
    # sweep also read `integration_resources/*/*.scxml` — so editing an
    # authored document there ran the lint never. The self-test now
    # compares a gate's sweep against its trigger.
    "example-codegen": {
        "workflows": ["example-codegen.yml"],
        # The workflow runs `scripts/gate example-codegen` instead of
        # restating this gate's commands, so the check has one spelling
        # for both callers. Every other mirrored gate still carries a
        # second copy of its commands in its workflow; this flag is what
        # makes the difference measurable per gate rather than a claim
        # about the set, and the self-test below holds the delegation.
        "runner_workflow": True,
        "deps": ["codegen-build"],
        "cost_s": 1,
        "summary": "example SCXML codegen smoke + authored-document lint",
    },
    "ledger-citations": {
        "workflows": ["spec-citations.yml"],
        "runner_workflow": True,
        # The mnemosyne validators are the sub-second part the hook's prose
        # described; the ledger-existence sweep over five source trees that
        # follows them is the rest of the 145s.
        "cost_s": 145,
        "summary": "spec-citation ledgers, 5 workspaces",
    },
    # The gate whose absence let a stale verifies-catalog reach CI red:
    # `ledger-citations` runs mnemosyne-cli, this workflow runs a separate
    # python generator, and the hook mirrored only the first.
    "spec-snapshot": {
        "workflows": ["spec-snapshot-drift.yml"],
        "runner_workflow": True,
        "cost_s": 1,
        "summary": "spec snapshot integrity + verifies-catalog drift",
    },
}

# Paths that drive no path-scoped gate. Anything here is classified — and
# therefore may skip the full run — so an entry is a claim that no such gate
# reads it. Rule 3 gates are outside that claim by construction: they run
# for every change, so an inert path is still judged by them. Everything NOT
# matched here and NOT matched by a gate trigger forces FULL, which is what
# keeps this list from being load-bearing in the unsafe direction.
INERT = [
    ".claude/**",
    ".gitignore",
    ".gitattributes",
    "*.md",
    "docs/adr/**",
    "LICENSE*",
    # Local tooling. The one gate that reads these, `roadmap_marker_gate`,
    # reaches every change through `tree-hygiene`, and this file's own
    # cases run unconditionally before any gate is chosen.
    #
    # Narrower than it reads: a gate whose workflow delegates to the
    # runner (`runner_workflow`) names its own script, `scripts/gate` and
    # `scripts/gates/lib.sh` in that workflow's `paths:`, and a gate
    # trigger is consulted before this list — so editing one of those
    # selects its gate rather than falling through to here. What stays
    # inert is the scripts of gates CI still mirrors by restating their
    # commands, where the script is not an input to any workflow.
    "tools/git-hooks/**",
    "scripts/gates/**",
    "scripts/gate",
]

# Entries here list only TRACKED paths, because a changed path comes from
# `git diff --name-only` and a gitignored one can never appear there. An
# ignored directory in this list would be dead config that reads as a
# decision.


def gate_script(repo_root: Path, slug: str) -> Path:
    return repo_root / "scripts" / "gates" / f"{slug}.sh"


def glob_to_regex(glob: str) -> re.Pattern:
    """Translate a GitHub Actions path glob into an anchored regex.

    Handles the three shapes the workflows actually use — `dir/**`,
    `**/*.ext`, and literal paths — plus the single-segment `*` that
    `backends/*/forge-runtime/**` needs. `**` crosses separators, `*` does
    not, matching Actions' own semantics.
    """
    out = ["^"]
    i = 0
    while i < len(glob):
        c = glob[i]
        if glob.startswith("**/", i):
            out.append("(.*/)?")
            i += 3
        elif glob.startswith("**", i):
            out.append(".*")
            i += 2
        elif c == "*":
            out.append("[^/]*")
            i += 1
        else:
            out.append(re.escape(c))
            i += 1
    out.append("$")
    return re.compile("".join(out))


def workflow_paths(repo_root: Path, name: str) -> list[str]:
    """The deduped `paths:` globs a workflow declares under `on:`.

    A workflow with no `paths:` filter runs on every push, so it returns the
    catch-all — the honest translation of "CI always runs this".
    """
    wf = repo_root / ".github" / "workflows" / name
    if not wf.is_file():
        # Missing workflow: refuse to guess. The caller turns an empty
        # trigger set into "always run", which is the safe direction.
        return ["**"]
    text = wf.read_text(encoding="utf-8")
    on_block = re.search(r"^on:\n(.*?)(?=^\S)", text, re.S | re.M)
    if not on_block:
        return ["**"]
    globs: list[str] = []
    for block in re.finditer(r"^(\s+)paths:\s*\n((?:\1\s+-.*\n|\s*#.*\n)*)",
                             on_block.group(1), re.M):
        for line in block.group(2).splitlines():
            line = line.strip()
            if line.startswith("- "):
                globs.append(line[2:].strip().strip("'\""))
    if not globs:
        return ["**"]
    return list(dict.fromkeys(globs))


# Assembled rather than spelled so this file does not match itself when a
# reader greps for scripts that enumerate the tree.
_SWEEP_CALL = re.compile("ls-" + r"files((?:\s+'[^']*')+)")


def swept_globs(src: str) -> list[str]:
    """Globs a gate script hands to git's tracked-file enumeration.

    A sweep is what makes a gate's input set wider than its trigger. The
    trigger below decides WHEN a gate runs, from a path list written
    here; a sweep inside the script decides WHAT it judges, from the tree
    at run time. The two are separate statements about the same gate and
    nothing kept them in step.
    """
    out: list[str] = []
    for match in _SWEEP_CALL.finditer(src):
        out.extend(re.findall(r"'([^']*)'", match.group(1)))
    return out


def tracked_matching(repo_root: Path, glob: str) -> list[str]:
    """Tracked paths a sweep glob actually yields, asked of git itself.

    Comparing the sweep glob to a trigger glob as text would need a
    containment rule between two glob dialects. The tree answers the
    question directly and without a rule to get wrong.
    """
    proc = subprocess.run(
        ["git", "-C", str(repo_root), "ls-" + "files", glob],
        capture_output=True, text=True, check=False,
    )
    return [line for line in proc.stdout.splitlines() if line]


def gate_triggers(repo_root: Path) -> dict[str, list[str]]:
    triggers: dict[str, list[str]] = {}
    for slug, spec in GATES.items():
        globs: list[str] = list(spec.get("local", []))
        for wf in spec.get("workflows", []):
            globs.extend(workflow_paths(repo_root, wf))
        globs.extend(spec.get("extra", []))
        triggers[slug] = list(dict.fromkeys(globs))
    return triggers


def run_order(slugs, table=None) -> list[str]:
    """Order the given gates: dependencies first, then cheapest first.

    Two properties, in that priority. A gate never runs before something it
    executes the output of, and among gates that are free to move, the one
    that can fail soonest goes first. Nothing about the order is written
    down as an order — it falls out of `deps` and `cost_s`, so adding a gate
    cannot silently push a cheap check behind an expensive one the way the
    hand-placed sequence did.

    A gate with no measured cost sorts last among the gates available at
    that moment rather than being guessed at: an unmeasured gate should not
    claim a cheap slot it has not earned.

    One gate is taken at a time, not one dependency level at a time. Level
    order looks equivalent and is not: it puts every dependent behind every
    independent gate, so a 1s gate that only needs the 1s generator build
    would run after the 399s vendor smoke. Taking the cheapest currently
    available gate lets a dependency unlock cheap work that then goes
    straight to the front.
    """
    table = GATES if table is None else table
    remaining = set(slugs)
    ordered: list[str] = []
    while remaining:
        ready = [s for s in remaining
                 if all(d not in remaining for d in table[s].get("deps", []))]
        if not ready:
            # A dependency cycle is a registry bug, not a runtime condition.
            raise ValueError(f"dependency cycle among {sorted(remaining)}")
        pick = min(ready, key=lambda s: (table[s].get("cost_s") is None,
                                         table[s].get("cost_s") or 0.0,
                                         s))
        ordered.append(pick)
        remaining.discard(pick)
    return ordered


# Measurement noise floor for the drift report below. `scripts/gate` times
# with bash's `SECONDS`, whose resolution is one second, and a gate's reading
# moves by a second or two between runs for reasons that say nothing about the
# declaration — a warm cache, a busy machine. A declared cost is treated as
# still true unless the run disagrees by more than this.
COST_NOISE_S = 2.0
COST_NOISE_FRACTION = 0.25


def cost_is_stale(declared, measured) -> bool:
    """Whether a run's timing genuinely disagrees with the declared cost."""
    if declared is None:
        return True  # an unmeasured gate has just been measured
    return abs(measured - declared) > max(COST_NOISE_S, declared * COST_NOISE_FRACTION)


def order_drift(measured: dict, table=None):
    """Report whether this run's timings would have ordered the gates differently.

    `cost_s` is hand-updated from `scripts/gate --measure --all`, and nothing
    noticed when a value went stale: every gate still passed, and the run order
    — the only thing the number decides — was quietly wrong. The registry's own
    order case cannot see it either, and says so: it derives the expected order
    from `cost_s` and then checks the order against `cost_s`, so editing a cost
    moves the gate and the property still holds.

    The question that is not a tautology is whether the TRUTH would have
    ordered them differently. This substitutes the run's own measurements for
    the declarations that have genuinely moved and re-derives the order; a
    difference means the declared costs are steering the run wrongly, and the
    caller says so. It is a report, not a verdict: a slow machine is not a
    reason to refuse a push.

    Returns None when the order stands, otherwise the two orders and the
    entries whose cost moved.
    """
    table = GATES if table is None else table
    slugs = [s for s in measured if s in table]
    if len(slugs) < 2:
        return None  # one gate cannot be out of order with itself
    declared_order = run_order(slugs, table)

    moved, fresh = [], {}
    for s in slugs:
        declared = table[s].get("cost_s")
        if cost_is_stale(declared, measured[s]):
            moved.append((s, declared, measured[s]))
            fresh[s] = dict(table[s], cost_s=measured[s])
        else:
            fresh[s] = table[s]
    if not moved:
        return None
    measured_order = run_order(slugs, fresh)
    if measured_order == declared_order:
        return None
    return {"declared": declared_order, "measured": measured_order, "moved": moved}


def transitive_deps(slug: str, table=None) -> set[str]:
    """Every gate `slug` needs, directly or through another gate."""
    table = GATES if table is None else table
    seen: set[str] = set()
    stack = list(table[slug].get("deps", []))
    while stack:
        d = stack.pop()
        if d in seen:
            continue
        seen.add(d)
        stack.extend(table[d].get("deps", []))
    return seen


def select(repo_root: Path, changed: list[str]) -> tuple[list[str], str]:
    """Return (gates to run in order, reason). Empty list means nothing to do."""
    if not changed:
        return ([], "no changed paths — nothing to verify")

    triggers = gate_triggers(repo_root)
    # Rule 3. Held out of `compiled` so they cannot classify a path, and
    # seeded into `selected` so they still run. `workflow_paths` also
    # returns the catch-all for a workflow it cannot find, which lands here
    # as "run it, learn nothing" — the safe reading of a missing file.
    always_on = {k for k, globs in triggers.items() if "**" in globs}
    compiled = {k: [glob_to_regex(g) for g in v]
                for k, v in triggers.items() if k not in always_on}
    inert = [glob_to_regex(g) for g in INERT]

    selected: set[str] = set(always_on)
    for path in changed:
        hit = False
        for slug, pats in compiled.items():
            if any(p.match(path) for p in pats):
                selected.add(slug)
                hit = True
        if hit:
            continue
        if any(p.match(path) for p in inert):
            continue
        # Rule 1: an unclassified path buys the full run rather than silence.
        return (run_order(GATES.keys()),
                f"unclassified path '{path}' — running every gate")

    # Dependency closure, applied until it stops growing: a dep may itself
    # have deps.
    changed_set = True
    while changed_set:
        changed_set = False
        for slug in list(selected):
            for dep in GATES[slug].get("deps", []):
                if dep not in selected:
                    selected.add(dep)
                    changed_set = True

    return (run_order(selected), "path-scoped selection")


def self_test(repo_root: Path) -> int:
    """Cases that pin the rules the registry must not break."""
    failures = []
    cases = 0

    def check(label, changed, want_full=None, want_has=(), want_lacks=()):
        nonlocal cases
        cases += 1
        keys, reason = select(repo_root, changed)
        full = len(keys) == len(GATES)
        if want_full is not None and full != want_full:
            failures.append(f"{label}: full={full}, wanted {want_full} ({reason})")
        for k in want_has:
            if k not in keys:
                failures.append(f"{label}: gate {k} missing from {keys}")
        for k in want_lacks:
            if k in keys:
                failures.append(f"{label}: gate {k} unexpectedly selected")

    # Rule 1 — the safety property. A path nobody classified runs everything.
    check("unclassified", ["some_new_top_level_dir/thing.txt"], want_full=True)
    # Rule 3 — an always-on gate must not classify. Were the catch-all
    # allowed to count, this path would read as known and rule 1 would never
    # fire again for anything.
    check("catch-all-does-not-classify", ["brand_new_dir/file.xyz"],
          want_full=True, want_has=["tree-hygiene"])
    # Every other workflow classifies its own edits by naming itself in its
    # `paths:`. The unfiltered one has no such list to name itself in, so
    # editing it is unclassified and buys the full run.
    check("unfiltered-workflow-self", [".github/workflows/tree-hygiene.yml"],
          want_full=True)
    # Docs-only runs the tree-wide gates and nothing else: prose is still
    # tracked source as far as the marker gate is concerned.
    check("inert", ["README.md", ".claude/settings.json"],
          want_full=False, want_has=["tree-hygiene"], want_lacks=["workspace-tests"])
    # A SCE-VERIFIES marker must reach the catalog gate.
    check("verifies-marker", ["tests/mesh/CustomTcpSocketOptionsTest.cpp"],
          want_has=["spec-snapshot"])
    # The hook's own sources are read by the tree-wide gates. They reach the
    # change through the unfiltered workflow, so the whole workspace sweep
    # no longer has to run to judge a hook edit.
    check("hook-self", ["tools/git-hooks/gate_registry.py"],
          want_full=False, want_has=["tree-hygiene"], want_lacks=["workspace-tests"])
    # A gate script edit is judged the same way, for the same reason.
    check("gate-script-self", ["scripts/gates/clippy.sh"],
          want_full=False, want_has=["tree-hygiene"], want_lacks=["workspace-tests"])
    # A ledger-only edit needs the citation gates, not the C++ build.
    check("ledger-only", ["docs/sce-ledger/mesh/.atomic/workspace.atomic.json"],
          want_has=["ledger-citations"], want_lacks=["nostd-mcu", "forge-cpp"])
    # Rust source pulls in clippy and the workspace suite.
    check("rust", ["sce-build/src/mesh/deploy.rs"],
          want_has=["codegen-build", "rust-modrs-drift", "clippy", "workspace-tests"])
    # Dependency closure: an example-only change still builds sce-codegen.
    check("example-dep", ["examples/smart_light/smart_light.scxml"],
          want_has=["codegen-build", "example-codegen"])
    # A template edit must reach the committed-tree drift gate.
    check("template", ["tools/codegen/templates/mesh/cpp/mesh_transport.h.jinja2"],
          want_has=["drift-suites"])
    # doc-check.yml edits must reach the gate that mirrors it. Under the
    # numbered table this gate was mapped to the no_std workflow, so an edit
    # to doc-check.yml alone did not select it.
    cases += 1
    keys, _ = select(repo_root, [".github/workflows/doc-check.yml"])
    if "rustdoc-links" not in keys:
        failures.append("doc-check-self: rustdoc-links missing from " + str(keys))

    # A gate that enumerates the tree must be triggered by everything it
    # enumerates. The two statements about a gate — when it runs and what
    # it judges — were written in different places and nothing compared
    # them, so they had drifted: `example-codegen` lints every authored
    # document under `integration_resources/` and its trigger named only
    # `examples/**`. Editing one of those documents ran the lint never.
    #
    # Worse than never-selected. Another gate's trigger matched the path,
    # so it counted as classified and rule 1 did not fire either — the
    # safety net that catches an unknown path cannot catch a known one
    # routed to the wrong gate.
    cases += 1
    triggers = gate_triggers(repo_root)
    sweeps_seen = 0
    for slug in sorted(GATES):
        script = gate_script(repo_root, slug)
        if not script.is_file():
            continue
        pats = [glob_to_regex(g) for g in triggers[slug]]
        for swept in swept_globs(script.read_text(encoding="utf-8")):
            sweeps_seen += 1
            for path in tracked_matching(repo_root, swept):
                if not any(p.match(path) for p in pats):
                    failures.append(
                        f"trigger: {slug} sweeps '{swept}' and judges "
                        f"'{path}', which no trigger of its own selects — "
                        f"the gate reads a path that never starts it")
                    break
    # Lower bound: a sweep extractor that stops matching would leave this
    # case reading nothing and still passing.
    if sweeps_seen < 1:
        failures.append(
            "trigger: no gate script was seen sweeping tracked files — "
            "the extractor is broken, not the gates")

    # Rule 2, held rather than described. A gate with no CI counterpart
    # states why in a field, not in a comment a reader has to find and a
    # test cannot read. Checked both ways: a gate that maps a workflow
    # must not also claim to have none, or the claim is stale the moment
    # the counterpart lands.
    cases += 1
    for slug, spec in sorted(GATES.items()):
        has_workflow = bool(spec.get("workflows"))
        reason = spec.get("no_ci_reason")
        if has_workflow and reason:
            failures.append(
                f"no-ci: {slug} maps {spec['workflows']} and also carries "
                f"no_ci_reason — drop the reason, it now has a counterpart")
        if not has_workflow and not reason:
            failures.append(
                f"no-ci: {slug} has no CI counterpart and no no_ci_reason. "
                f"A gate that runs only at push time is skipped by "
                f"`--no-verify`; say why that is acceptable here, or give "
                f"it a workflow")

    # A gate whose workflow delegates to the runner must actually call it.
    #
    # Mapping a workflow proves only that a file exists with the right
    # `paths:`. A workflow that stopped running its gate would keep the
    # trigger derivation working, keep every existing parity check green,
    # and verify nothing — the shape a push-time-only gate takes when it
    # is given a CI counterpart in name.
    cases += 1
    delegating = 0
    for slug, spec in sorted(GATES.items()):
        if not spec.get("runner_workflow"):
            continue
        delegating += 1
        for name in spec.get("workflows", []):
            wf = repo_root / ".github" / "workflows" / name
            if not wf.is_file():
                continue  # `every_workflow_the_registry_maps_exists` owns this
            if f"scripts/gate {slug}" not in wf.read_text(encoding="utf-8"):
                failures.append(
                    f"delegation: {name} is declared as running {slug} "
                    f"through the runner but never calls "
                    f"`scripts/gate {slug}` — either restore the call or "
                    f"drop `runner_workflow` and mirror the commands")
    if delegating < 1:
        failures.append(
            "delegation: no gate declares runner_workflow — the flag that "
            "tracks CI-calls-the-runner progress reads nothing")

    # ...and must not keep a second copy of what it delegates.
    #
    # Calling the runner while the old block sits beside it is the worst
    # of both: the commands still live in two files, and the flag now
    # says they do not. The check is on command lines rather than on
    # prose, so a workflow may still explain what the gate does.
    cases += 1
    verbs = ("cargo ", "ctest", "go test", "./gradlew", "python3 ", "bash scripts/")
    for slug, spec in sorted(GATES.items()):
        if not spec.get("runner_workflow"):
            continue
        script = gate_script(repo_root, slug)
        if not script.is_file():
            continue
        # Continuation backslashes and run-step indentation are
        # spelling, not command: a gate that wraps its arguments and a
        # workflow that does not were the same command, and comparing
        # the raw lines matched neither. Normalising is what lets the
        # duplicate be seen.
        def normalise(line: str) -> str:
            line = line.strip()
            if line.startswith("run:"):
                line = line[4:].strip()
            return " ".join(line.rstrip("\\").split())

        gate_cmds = {
            normalise(line)
            for line in script.read_text(encoding="utf-8").splitlines()
            if normalise(line).startswith(verbs)
        }
        for name in spec.get("workflows", []):
            wf = repo_root / ".github" / "workflows" / name
            if not wf.is_file():
                continue
            body = wf.read_text(encoding="utf-8")
            wf_cmds = {
                normalise(line)
                for line in body.splitlines()
                if normalise(line).startswith(verbs)
            }
            for dup in sorted(gate_cmds & wf_cmds):
                failures.append(
                    f"delegation: {name} calls `scripts/gate {slug}` and "
                    f"still restates its command `{dup}` — one of the two "
                    f"spellings is the one that will be edited")

    # A gate whose workflow does not delegate says why, next to itself.
    #
    # The count of delegating gates is a progress measure, and a
    # progress measure with no reason attached to the remainder reads
    # as "not done yet" forever. Each non-delegating entry carries a
    # `# Not delegated:` note stating the measured difference — the
    # lane adds a reporting flag, uploads an artifact, installs a tool.
    # Reading the registry's own source for the note is what keeps the
    # note from being optional; the same shape the `local`-only gates'
    # reasons take.
    cases += 1
    registry_src = pathlib.Path(__file__).read_text(encoding="utf-8").splitlines()
    for slug, spec in sorted(GATES.items()):
        if not spec.get("workflows") or spec.get("runner_workflow"):
            continue
        entry = next(
            (i for i, line in enumerate(registry_src) if line.strip().startswith(f'"{slug}": {{')),
            None,
        )
        if entry is None:
            failures.append(f"delegation: {slug} is not declared in this file's source")
            continue
        window = "\n".join(registry_src[entry : entry + 12])
        if "Not delegated:" not in window:
            failures.append(
                f"delegation: {slug} still restates its gate's commands in CI and "
                f"says nothing about why — add a `# Not delegated:` note with the "
                f"measured difference, or convert the workflow to `scripts/gate {slug}`")

    # Every gate in the table has a script, and every script is in the table.
    cases += 1
    for slug in GATES:
        if not gate_script(repo_root, slug).is_file():
            failures.append(f"registry: {slug} has no scripts/gates/{slug}.sh")
    script_dir = repo_root / "scripts" / "gates"
    if script_dir.is_dir():
        for path in sorted(script_dir.glob("*.sh")):
            if path.name == "lib.sh":
                continue
            if path.stem not in GATES:
                failures.append(f"registry: scripts/gates/{path.name} is not registered")

    # Order is derived, not declared: dependencies first, then cheapest.
    cases += 1
    order = run_order(GATES.keys())
    pos = {s: i for i, s in enumerate(order)}
    for slug, spec in GATES.items():
        for dep in spec.get("deps", []):
            if pos[dep] > pos[slug]:
                failures.append(f"order: {slug} runs before its dependency {dep}")

    # Nothing expensive runs ahead of something cheap unless the cheap one
    # was waiting on it. Note what this does and does not catch: because the
    # order is computed from `cost_s`, editing a cost moves the gate and the
    # property still holds — so this cannot detect a stale measurement. What
    # it does detect is an ordering that stops consulting cost at all.
    cases += 1
    for i, earlier in enumerate(order):
        for later in order[i + 1:]:
            ce = GATES[earlier].get("cost_s")
            cl = GATES[later].get("cost_s")
            if ce is None or cl is None:
                continue
            if ce > cl and earlier not in transitive_deps(later):
                failures.append(
                    f"order: {earlier} ({ce}s) runs before the cheaper "
                    f"{later} ({cl}s) without being its dependency")

    # The distinction the check above is blind to, on a fixture that pins it.
    # Taking one dependency LEVEL at a time satisfies every property stated
    # so far and still puts a 1s gate behind a 999s one, because the cheap
    # gate happens to have a dependency. Only taking the cheapest currently
    # available gate gets this right, and only a synthetic table can tell the
    # two apart — the real one would have to be reshaped to expose it.
    cases += 1
    fixture = {
        "prereq": {"cost_s": 1},
        "cheap-dependent": {"deps": ["prereq"], "cost_s": 1},
        "expensive-independent": {"cost_s": 999},
    }
    got = run_order(fixture.keys(), table=fixture)
    want = ["prereq", "cheap-dependent", "expensive-independent"]
    if got != want:
        failures.append(f"order: level-at-a-time regression — got {got}, want {want}")

    # Drift report — the answer to what the order case above is blind to. Each
    # case runs against a synthetic table, because the point is the rule and
    # not this week's numbers.
    drift_table = {
        "prereq": {"cost_s": 1},
        "cheap": {"cost_s": 2},
        "dear": {"cost_s": 100},
        "dependent": {"deps": ["prereq"], "cost_s": 3},
    }
    cases += 1
    if order_drift({s: drift_table[s]["cost_s"] for s in drift_table},
                   table=drift_table) is not None:
        failures.append("drift: a run that matches the declarations reported drift")

    cases += 1
    swap = order_drift({"prereq": 1, "cheap": 200, "dear": 100, "dependent": 3},
                       table=drift_table)
    if swap is None:
        failures.append("drift: a gate measured 100x its declared cost went unreported")
    elif swap["measured"].index("dear") > swap["measured"].index("cheap"):
        failures.append(f"drift: measured order did not reseat the pair: {swap}")

    cases += 1
    # Within the noise floor: `SECONDS` granularity must not nag.
    if order_drift({"prereq": 2, "cheap": 3, "dear": 101, "dependent": 4},
                   table=drift_table) is not None:
        failures.append("drift: reported a difference inside the measurement noise floor")

    cases += 1
    # A dependency still binds: measuring the dependent as free cannot lift it
    # above the gate whose output it consumes.
    dep = order_drift({"prereq": 50, "cheap": 2, "dear": 100, "dependent": 0},
                      table=drift_table)
    if dep is not None and dep["measured"].index("prereq") > dep["measured"].index("dependent"):
        failures.append(f"drift: measured order broke a dependency: {dep}")

    cases += 1
    # A gate with no declared cost is reported the first time it is timed —
    # `run_order` sorts it last, which is a guess the measurement can correct.
    if order_drift({"prereq": 1, "cheap": 2, "unmeasured": 0},
                   table=dict(drift_table, unmeasured={})) is None:
        failures.append("drift: an unmeasured gate's first timing went unreported")

    if failures:
        for f in failures:
            print(f"SELF-TEST FAIL: {f}", file=sys.stderr)
        return 1
    print(f"gate_registry self-test: {cases} cases OK", file=sys.stderr)
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--repo-root", default=".", type=Path)
    ap.add_argument("--changed-from", type=Path,
                    help="file with one changed path per line ('-' for stdin)")
    ap.add_argument("--changed", nargs="*", default=None)
    ap.add_argument("--self-test", action="store_true")
    ap.add_argument("--list", action="store_true",
                    help="print every gate in run order")
    ap.add_argument("--explain", action="store_true",
                    help="also print the selection reason to stderr")
    ap.add_argument("--mapping", action="store_true",
                    help="print the table as JSON, so a consumer reads the "
                         "registry instead of parsing this file's source")
    ap.add_argument("--order-drift", nargs="*", metavar="SLUG=SECONDS",
                    help="compare a run's timings against the declared costs "
                         "and report when they would have ordered the gates "
                         "differently")
    args = ap.parse_args()

    repo_root = args.repo_root.resolve()

    if args.self_test:
        return self_test(repo_root)

    if args.mapping:
        # A reader that scrapes this file's source picks up the self-test's
        # synthetic tables along with the real one — measured, three fixture
        # gates and four more. Emitting the table is the answer.
        json.dump(GATES, sys.stdout, indent=1, sort_keys=True, default=str)
        sys.stdout.write("\n")
        return 0

    if args.order_drift is not None:
        measured = {}
        for pair in args.order_drift:
            slug, _, secs = pair.partition("=")
            if slug in GATES:
                measured[slug] = float(secs)
        drift = order_drift(measured)
        if drift is None:
            return 0
        print("\ngate: the declared costs no longer order this run correctly.",
              file=sys.stderr)
        for slug, declared, actual in drift["moved"]:
            was = "unmeasured" if declared is None else f"{declared}s"
            print(f"    {slug:<26} declared {was:>12}  ran {actual:g}s", file=sys.stderr)
        print(f"  ran in:   {' '.join(drift['declared'])}", file=sys.stderr)
        print(f"  would be: {' '.join(drift['measured'])}", file=sys.stderr)
        print("  Re-measure with `scripts/gate --measure --all` and update "
              "cost_s.\n", file=sys.stderr)
        return 0

    if args.list:
        for slug in run_order(GATES.keys()):
            cost = GATES[slug].get("cost_s")
            cost_s = f"{cost:>7.1f}s" if cost is not None else "      ?"
            print(f"{cost_s}  {slug:<26} {GATES[slug].get('summary', '')}")
        return 0

    if args.changed is not None:
        changed = list(args.changed)
    elif args.changed_from:
        text = sys.stdin.read() if str(args.changed_from) == "-" \
            else args.changed_from.read_text(encoding="utf-8")
        changed = [ln.strip() for ln in text.splitlines() if ln.strip()]
    else:
        ap.error("one of --changed / --changed-from / --self-test / --list is required")
        return 2

    keys, reason = select(repo_root, changed)
    if args.explain:
        print(f"  selection: {reason}", file=sys.stderr)
    for k in keys:
        print(k)
    return 0


if __name__ == "__main__":
    sys.exit(main())
