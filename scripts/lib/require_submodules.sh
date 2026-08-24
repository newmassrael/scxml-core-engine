#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Refuse to run a gate in a tree whose submodules are not checked out, and
# say which command fixes it.
#
# ── Why this exists ───────────────────────────────────────────────
#
# `git worktree add` does not populate submodules, and neither does a plain
# `git clone` without `--recurse-submodules`. Measured 2026-08-24: a push
# from a fresh worktree spent every gate ahead of `w3c-kotlin` and then died
# 27 seconds into a Gradle task with
#
#     Execution failed for task ':sce-kotlin-quickjs:cmakeConfigure'
#     CMake Generate step failed.  Build files cannot be regenerated correctly.
#
# Nothing in that names `third_party/quickjs`, nothing names the repair, and
# the gates ahead of it had to run again from the start on the next push. The
# repair itself took ten seconds once it was known. This turns that into one
# line, before anything is built.
#
# ── Why it is per-gate, and why the list is not in this file ──────
#
# "A tree without submodules is broken" is FALSE here: `tree-hygiene.yml`
# checks out WITHOUT them on purpose and says so in a comment — that gate
# skips `third_party/` and `vendor/`. A blanket refusal would break the one
# workflow that made the opposite choice deliberately.
#
# So the requirement is per-gate, and CI already records it: a workflow
# either says `submodules: recursive` or it does not. That is the SSOT, and
# this reads it through the registry's `workflows` field. A list of gate
# names here would be a second copy of a declaration that already exists,
# free to drift the moment a workflow changes its mind — the same rule
# `mutation-cases` and `mutation-rounds` follow for their own triggers.
#
# ── The declaration is per JOB, not per file ──────────────────────
#
# `actions/checkout` is a STEP, so a workflow with several jobs answers this
# question several times. `w3c-tests.yml` has seven: `test-cpp`,
# `test-kotlin`, `test-python-bindings` and `test-c11` ask for submodules
# and `test-rust`, `test-python` and `test-go` explicitly do not
# (`submodules: false`). A file-level scan puts every gate that file mirrors
# on the side its LOUDEST job took.
#
# Measured 2026-08-24, the round after this preflight landed: it refused
# `w3c-go`, `w3c-python`, `forge-go`, `forge-python`, `forge-rust` and
# `embed-manifest-failfast` INSIDE CI — the very jobs that had been running
# them green without submodules for as long as they had existed. Five red
# lanes, all saying "4 submodule(s) are recorded but not checked out" in a
# checkout that was correct.
#
# So the job that RUNS the slug is the one whose checkout decides. A slug no
# job names falls back to the file's answer, because "cannot tell" should
# refuse rather than let a build die deep with a message naming neither the
# submodule nor the repair — which is the defect this whole file exists for.
#
# ── Usage ─────────────────────────────────────────────────────────
#
#   scripts/lib/require_submodules.sh <slug>...
#       Refuse (exit 1) if any named gate needs submodules and any submodule
#       is not checked out. Silent otherwise.
#
#   scripts/lib/require_submodules.sh --which <slug>...
#       Print, one per line, those of the named gates that need submodules.
#       This is what `hook_ci_parity` asserts on, so the derivation itself
#       has a witness rather than only its effect.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
REGISTRY="$REPO_ROOT/tools/git-hooks/gate_registry.py"
WORKFLOW_DIR="$REPO_ROOT/.github/workflows"

MODE="refuse"
if [[ "${1:-}" == "--which" ]]; then
    MODE="which"
    shift
fi

(( $# )) || exit 0

# Which of the named gates mirror a workflow that asks CI for submodules.
mapfile -t NEEDING < <(python3 - "$REGISTRY" "$WORKFLOW_DIR" "$@" <<'PYEOF'
import runpy
import sys
from pathlib import Path

import re

mod = runpy.run_path(sys.argv[1])
workflow_dir = Path(sys.argv[2])


def declares_submodules(text):
    """True if this YAML really asks `actions/checkout` for submodules.

    Comment lines are stripped FIRST. `tree-hygiene.yml` explains its own
    exemption in the words it is exempt from — "No `submodules: recursive`:
    the gate skips third_party/" — and a substring search reads that comment
    as the opposite of what it says. Measured 2026-08-24: it put both gates
    mirroring that workflow on the wrong side of this answer.
    """
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("#"):
            continue
        code = stripped.split("#", 1)[0]
        if code.startswith("submodules:") and "recursive" in code:
            return True
    return False


def jobs_of(text):
    """`(job name, job body)` for each job in a workflow.

    A job is a two-space key under `jobs:`, which is the whole grammar this
    needs — the answer being derived is one step's `with:` value, and a YAML
    parser is not in the standard library. A file whose shape this cannot
    read yields no jobs, and the caller falls back to the file's answer.
    """
    out, name, body = [], None, []
    in_jobs = False
    for line in text.splitlines():
        if line.startswith("jobs:"):
            in_jobs = True
            continue
        if not in_jobs:
            continue
        header = re.match(r"^  ([A-Za-z0-9_-]+):\s*$", line)
        if header:
            if name is not None:
                out.append((name, "\n".join(body)))
            name, body = header.group(1), []
            continue
        body.append(line)
    if name is not None:
        out.append((name, "\n".join(body)))
    return out


def runs_slug(body, slug):
    """Does this job body invoke `scripts/gate <slug>`?

    The trailing boundary is load-bearing: without it `w3c-python` matches
    the job running `scripts/gate w3c-python-bindings`, and the two jobs
    check out differently.
    """
    return re.search(r"\bgate\s+" + re.escape(slug) + r"(?![\w.-])", body) is not None


for slug in sys.argv[3:]:
    entry = mod["GATES"].get(slug)
    if not entry:
        continue
    texts = []
    for workflow in entry.get("workflows", []):
        path = workflow_dir / workflow
        if path.is_file():
            texts.append(path.read_text(encoding="utf-8"))

    running = [body for text in texts for _, body in jobs_of(text) if runs_slug(body, slug)]
    if running:
        # The job that runs it is the one whose checkout it has to mirror.
        needs = any(declares_submodules(body) for body in running)
    else:
        # No job names it — fall back to the file, which refuses rather than
        # guessing that a gate it cannot place needs nothing.
        needs = any(declares_submodules(text) for text in texts)
    if needs:
        print(slug)
PYEOF
)

if [[ "$MODE" == "which" ]]; then
    (( ${#NEEDING[@]} )) && printf '%s\n' "${NEEDING[@]}"
    exit 0
fi

(( ${#NEEDING[@]} )) || exit 0

# `-<sha> <path>` is git's spelling for "recorded, not checked out". An
# initialised submodule leads with a space or a `+`, so this collects exactly
# the ones that would fail a build.
mapfile -t MISSING < <(git -C "$REPO_ROOT" submodule status | awk '/^-/ { print $2 }')

(( ${#MISSING[@]} )) || exit 0

{
    printf '\nERROR gate: %d submodule(s) are recorded but not checked out, so nothing can be claimed:\n' \
        "${#MISSING[@]}"
    printf '  %s\n' "${MISSING[@]}"
    printf '\nThese gate(s) build against them: %s\n' "${NEEDING[*]}"
    printf 'A new `git worktree` and a plain `git clone` both start this way.\n'
    printf '\nRepair, from %s:\n' "$REPO_ROOT"
    printf '  git submodule update --init --recursive\n\n'
} >&2

exit 1
