#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Decide which pre-push stages a given set of changed paths requires.
#
# The hook mirrors CI, so the question "does this change need that gate?" is
# already answered — by the `paths:` filter on the workflow the stage mirrors.
# This reads those filters out of `.github/workflows/*.yml` rather than
# restating them, so the hook cannot drift from CI the way a hand-copied table
# would. A stage whose workflow gains a path starts running here on the same
# commit that widens CI.
#
# Two rules keep the filtering from becoming a coverage hole, which is the
# failure mode a selective hook invites:
#
#   1. A changed path matching NO known glob — not a stage trigger and not the
#      inert list below — forces the FULL run. A new top-level directory
#      therefore gets more verification, never less.
#   2. Stages with no CI counterpart (the sce-codegen build, the mod.rs drift
#      check, the example smoke) carry an explicit local trigger with the
#      reason written next to it, rather than being silently always-on or
#      silently never-on.
#   3. A stage whose trigger is the catch-all `**` is selected always and
#      counts as classifying nothing. Its workflow runs on every push, so a
#      match carries no information about whether a path is understood —
#      letting it count would mark every path in the repository known and
#      rule 1 would never fire again.
#
# Output: one stage key per line on stdout, or the single line `FULL`.
# Exit status is 0 in both cases; a usage error exits 2.

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

# ── Stage table ───────────────────────────────────────────────────
#
# `workflows` names the CI workflow(s) a stage mirrors; its `paths:` filter is
# the trigger. `extra` adds locally-known triggers for what the workflow's
# filter cannot express, and `local` is for stages CI has no counterpart for.
#
# Keep the keys in sync with the `log_step "Stage ..."` labels in `pre-push`;
# `--self-test` checks the table is well-formed but cannot see the hook.
STAGES: dict[str, dict] = {
    # Prerequisite, not a gate: stages 2b/5/6/7 execute target/release/sce-codegen.
    # Triggered by its own sources, and forced on by dependents (see DEPENDS_ON).
    "1": {"local": ["sce-build/**", "Cargo.toml", "Cargo.lock"]},
    # Structural check over the Rust module tree — only a .rs add/remove can
    # break it, so the Rust trigger set is exactly right.
    "1b": {"local": ["**/*.rs"]},
    # Guards the clang degraded-AST regression in the manifest emitter.
    "1c": {"local": ["scripts/emit_embed_manifest.sh", "scripts/package_embed.sh",
                     "sce/include/**"]},
    # Tree-wide hygiene gates. `roadmap_marker_gate` reads every tracked file
    # and `workflow_trigger_coverage` reads every workflow, so tree-hygiene.yml
    # declares no `paths:` filter — no glob list is both correct and narrower
    # than the tree. That missing filter is what makes this stage
    # unconditional here, derived the same way every other workflow-backed
    # trigger is rather than hand-set to "always".
    "1d": {"workflows": ["tree-hygiene.yml"]},
    "2": {"workflows": ["clippy-check.yml"]},
    "2b": {"workflows": ["sce-rust-runtime-no-std.yml"]},
    "2c": {"workflows": ["sce-rust-runtime-no-std.yml"]},
    "3": {"workflows": ["embed-vendor-smoke.yml"],
          "extra": ["sce/include/**", "embed/MANIFEST.json"]},
    # Mirrors the Rust workspace suite only. `w3c-tests.yml` is deliberately
    # NOT listed: it has no `paths:` filter (it runs on every CI push) and it
    # drives the C++ ctest suite, which this hook does not mirror at all.
    # Mapping it here would make its catch-all the trigger for the hook's
    # 20-minute bottleneck and, worse, would classify every path — defeating
    # the unclassified-forces-FULL rule that keeps this file honest.
    # `extra` covers inputs the workflow filter misses because they are
    # `include_str!`-ed into the suite rather than compiled: the acceptance
    # doc (`acceptance_doc_covers_every_code`), the wire schemas
    # (`json_schema_enums_match_rust_source_of_truth`, the sourcemap and XSD
    # byte pins), and the forge-AST export schema.
    # `tools/git-hooks/**` used to be listed here because the workspace sweep
    # carries `roadmap_marker_gate`, which reads this very file. That was a
    # patch over one path: the gate's scan set is the whole tree, so every
    # unlisted path had the same hole. Stage 1d now runs it unconditionally
    # from a workflow with no filter, which closes the class rather than the
    # instance.
    "4": {"workflows": ["rust-workspace-tests.yml"],
          "extra": ["docs/SCE_ACCEPTED_SUBSET.md", "schemas/**", "apis/**"]},
    "4b": {"workflows": ["drift-verify.yml"]},
    "5": {"workflows": ["forge-conformance.yml"], "extra": ["backends/go/**"]},
    "6": {"workflows": ["forge-conformance.yml"], "extra": ["backends/cpp/**", "sce/**"]},
    # No CI counterpart: catches codegen breakage in the example documents
    # (the namespace migration that broke them shipped green otherwise).
    "7": {"local": ["examples/**", "tools/codegen/templates/**", "sce-build/**"]},
    "8": {"workflows": ["spec-citations.yml"]},
    # The gate whose absence let a stale verifies-catalog reach CI red on
    # 2026-08-04: Stage 8 runs mnemosyne-cli, this workflow runs a separate
    # python generator, and the hook mirrored only the first.
    "8b": {"workflows": ["spec-snapshot-drift.yml"]},
}

# A stage that cannot run without another one's output. Selecting the key
# forces its dependencies on, so `--changed examples/x.scxml` still builds
# sce-codegen even though no Rust file changed.
DEPENDS_ON: dict[str, list[str]] = {
    "2b": ["1"],
    "5": ["1"],
    "6": ["1"],
    "7": ["1"],
}

# Paths that drive no path-scoped stage. Anything here is classified — and
# therefore may skip the full run — so an entry is a claim that no such gate
# reads it. Rule 3 stages are outside that claim by construction: they run for
# every change, so an inert path is still judged by them. Everything NOT
# matched here and NOT matched by a stage trigger forces FULL, which is what
# keeps this list from being load-bearing in the unsafe direction.
INERT = [
    ".claude/**",
    ".gitignore",
    ".gitattributes",
    "*.md",
    "docs/adr/**",
    "LICENSE*",
    # Local tooling — no CI workflow consumes the hooks, so no mirrored stage
    # has them as an input. The one gate that does read them,
    # `roadmap_marker_gate`, reaches every change through stage 1d, and the
    # selector's own cases run unconditionally before any stage is chosen.
    "tools/git-hooks/**",
]

# Entries here list only TRACKED paths, because a changed path comes from
# `git diff --name-only` and a gitignored one can never appear there. An
# ignored directory in this list would be dead config that reads as a
# decision.


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
        # Missing workflow: refuse to guess. The caller turns an empty trigger
        # set into "always run", which is the safe direction.
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


def stage_triggers(repo_root: Path) -> dict[str, list[str]]:
    triggers: dict[str, list[str]] = {}
    for key, spec in STAGES.items():
        globs: list[str] = list(spec.get("local", []))
        for wf in spec.get("workflows", []):
            globs.extend(workflow_paths(repo_root, wf))
        globs.extend(spec.get("extra", []))
        triggers[key] = list(dict.fromkeys(globs))
    return triggers


def select(repo_root: Path, changed: list[str]) -> tuple[list[str], str]:
    """Return (stage keys to run, reason). An empty list means nothing to do."""
    if not changed:
        return ([], "no changed paths — nothing to verify")

    triggers = stage_triggers(repo_root)
    # Rule 3. Held out of `compiled` so they cannot classify a path, and
    # seeded into `selected` so they still run. `workflow_paths` also returns
    # the catch-all for a workflow it cannot find, which lands here as
    # "run it, learn nothing" — the safe reading of a missing file.
    always_on = {k for k, globs in triggers.items() if "**" in globs}
    compiled = {k: [glob_to_regex(g) for g in v]
                for k, v in triggers.items() if k not in always_on}
    inert = [glob_to_regex(g) for g in INERT]

    selected: set[str] = set(always_on)
    for path in changed:
        hit = False
        for key, pats in compiled.items():
            if any(p.match(path) for p in pats):
                selected.add(key)
                hit = True
        if hit:
            continue
        if any(p.match(path) for p in inert):
            continue
        # Rule 1: an unclassified path buys the full run rather than silence.
        return (sorted(STAGES, key=stage_sort_key),
                f"unclassified path '{path}' — running every stage")

    for key in list(selected):
        for dep in DEPENDS_ON.get(key, []):
            selected.add(dep)

    return (sorted(selected, key=stage_sort_key), "path-scoped selection")


def stage_sort_key(key: str) -> tuple:
    m = re.match(r"(\d+)([a-z]*)", key)
    return (int(m.group(1)), m.group(2)) if m else (99, key)


def self_test(repo_root: Path) -> int:
    """Cases that pin the two rules the filtering must not break."""
    failures = []
    cases = 0

    def check(label, changed, want_full=None, want_has=(), want_lacks=()):
        nonlocal cases
        cases += 1
        keys, reason = select(repo_root, changed)
        full = len(keys) == len(STAGES)
        if want_full is not None and full != want_full:
            failures.append(f"{label}: full={full}, wanted {want_full} ({reason})")
        for k in want_has:
            if k not in keys:
                failures.append(f"{label}: stage {k} missing from {keys}")
        for k in want_lacks:
            if k in keys:
                failures.append(f"{label}: stage {k} unexpectedly selected")

    # Rule 1 — the safety property. A path nobody classified runs everything.
    check("unclassified", ["some_new_top_level_dir/thing.txt"], want_full=True)
    # Rule 3 — an always-on stage must not classify. Were the catch-all
    # allowed to count, this path would read as known and rule 1 would never
    # fire again for anything.
    check("catch-all-does-not-classify", ["brand_new_dir/file.xyz"],
          want_full=True, want_has=["1d"])
    # Every other workflow classifies its own edits by naming itself in its
    # `paths:`. The unfiltered one has no such list to name itself in, so
    # editing it is unclassified and buys the full run. Rare, and rule 1's
    # direction.
    check("unfiltered-workflow-self", [".github/workflows/tree-hygiene.yml"],
          want_full=True)
    # Docs-only runs the tree-wide gates and nothing else: prose is still
    # tracked source as far as the marker gate is concerned.
    check("inert", ["README.md", ".claude/settings.json"],
          want_full=False, want_has=["1d"], want_lacks=["4"])
    # The failure of 2026-08-04: a SCE-VERIFIES marker must reach the catalog gate.
    check("verifies-marker", ["tests/mesh/CustomTcpSocketOptionsTest.cpp"],
          want_has=["8b"])
    # The hook's own sources are read by the tree-wide gates. They reach the
    # change through stage 1d's unfiltered workflow now, so the whole
    # workspace sweep no longer has to run to judge a hook edit.
    check("hook-self", ["tools/git-hooks/select_stages.py"],
          want_full=False, want_has=["1d"], want_lacks=["4"])
    # A ledger-only edit needs the citation gates, not the C++ build.
    check("ledger-only", ["docs/sce-ledger/mesh/.atomic/workspace.atomic.json"],
          want_has=["8"], want_lacks=["2b", "6"])
    # Rust source pulls in clippy and the workspace suite.
    check("rust", ["sce-build/src/mesh/deploy.rs"], want_has=["1", "1b", "2", "4"])
    # Dependency closure: an example-only change still builds sce-codegen.
    check("example-dep", ["examples/smart_light/smart_light.scxml"],
          want_has=["1", "7"])
    # A template edit must reach the committed-tree drift gate.
    check("template", ["tools/codegen/templates/mesh/cpp/mesh_transport.h.jinja2"],
          want_has=["4b"])

    if failures:
        for f in failures:
            print(f"SELF-TEST FAIL: {f}", file=sys.stderr)
        return 1
    print(f"select_stages self-test: {cases} cases OK", file=sys.stderr)
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--repo-root", default=".", type=Path)
    ap.add_argument("--changed-from", type=Path,
                    help="file with one changed path per line ('-' for stdin)")
    ap.add_argument("--changed", nargs="*", default=None)
    ap.add_argument("--self-test", action="store_true")
    ap.add_argument("--explain", action="store_true",
                    help="also print the selection reason to stderr")
    args = ap.parse_args()

    repo_root = args.repo_root.resolve()

    if args.self_test:
        return self_test(repo_root)

    if args.changed is not None:
        changed = list(args.changed)
    elif args.changed_from:
        text = sys.stdin.read() if str(args.changed_from) == "-" \
            else args.changed_from.read_text(encoding="utf-8")
        changed = [ln.strip() for ln in text.splitlines() if ln.strip()]
    else:
        ap.error("one of --changed / --changed-from / --self-test is required")
        return 2

    keys, reason = select(repo_root, changed)
    if args.explain:
        print(f"  selection: {reason}", file=sys.stderr)
    for k in keys:
        print(k)
    return 0


if __name__ == "__main__":
    sys.exit(main())
