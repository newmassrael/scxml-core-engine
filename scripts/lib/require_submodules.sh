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

mod = runpy.run_path(sys.argv[1])
workflow_dir = Path(sys.argv[2])


def declares_submodules(path):
    """True if the workflow really asks `actions/checkout` for submodules.

    Comment lines are stripped FIRST. `tree-hygiene.yml` explains its own
    exemption in the words it is exempt from — "No `submodules: recursive`:
    the gate skips third_party/" — and a substring search reads that comment
    as the opposite of what it says. Measured 2026-08-24: it put both gates
    mirroring that workflow on the wrong side of this answer.
    """
    for line in path.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if stripped.startswith("#"):
            continue
        code = stripped.split("#", 1)[0]
        if code.startswith("submodules:") and "recursive" in code:
            return True
    return False


for slug in sys.argv[3:]:
    entry = mod["GATES"].get(slug)
    if not entry:
        continue
    for workflow in entry.get("workflows", []):
        path = workflow_dir / workflow
        if path.is_file() and declares_submodules(path):
            print(slug)
            break
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
