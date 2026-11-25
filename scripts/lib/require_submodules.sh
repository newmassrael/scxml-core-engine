#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
# this reads it through the `workflows` field of each gate's entry. A list
# of gate names here would be a second copy of a declaration that already
# exists, free to drift the moment a workflow changes its mind — the same
# rule `mutation-cases` and `mutation-rounds` follow for their own triggers.
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
# ── A narrowed gate needs only the submodules its inputs reach ────
#
# A gate whose registry entry declares `inputs` runs, as a unit of a plan, in
# a copy bx makes of those inputs alone, and the copy holds every submodule
# the inputs do not reach as a gitlink that is not checked out (bx RFC 0026,
# section 4.1, 2026-11-23). `license-verify.yml` checks submodules out for
# `license-ssot`, which reads them; `web-vendor-licenses` runs in the same
# job and reads none. Asked of every submodule, the preflight refused that
# gate in every copy (measured 2026-11-24, bx 340e9f9e). So for a narrowed
# gate the refusal counts only the submodules its inputs reach
# (`submodules_reached` in the registry), and a gate that reads none is
# refused for none of them. `--which` still answers the workflow's question.
#
# ── Usage ─────────────────────────────────────────────────────────
#
#   scripts/lib/require_submodules.sh <slug>...
#       Refuse (exit 1) if any named gate needs submodules and a submodule it
#       needs is not checked out. Silent otherwise.
#
#   scripts/lib/require_submodules.sh --which <slug>...
#       Print, one per line, those of the named gates that need submodules.
#       This is what `hook_ci_parity` asserts on, so the derivation itself
#       has a witness rather than only its effect.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
TABLE="$REPO_ROOT/tools/git-hooks/gate_table.py"

MODE="refuse"
if [[ "${1:-}" == "--which" ]]; then
    MODE="which"
    shift
fi

(( $# )) || exit 0

# Which of the named gates mirror a workflow that asks CI for submodules.
# In --which mode one slug a line; otherwise `slug<TAB>scope`, the scope being
# `*` (every submodule) or the paths of those the gate's inputs reach.
# Answered from the gates' own entries (`submodule_needs` in gate_table.py),
# so a unit's run reads those and not the registry.
WHICH=()
if [[ "$MODE" == "which" ]]; then
    WHICH=(--which)
fi
mapfile -t NEEDING < <(python3 -I "$TABLE" submodules ${WHICH[@]+"${WHICH[@]}"} "$@")

if [[ "$MODE" == "which" ]]; then
    (( ${#NEEDING[@]} )) && printf '%s\n' "${NEEDING[@]}"
    exit 0
fi

(( ${#NEEDING[@]} )) || exit 0

SLUGS=()
EVERY=0
declare -A WANTED=()
for line in "${NEEDING[@]}"; do
    SLUGS+=("${line%%$'\t'*}")
    scope="${line#*$'\t'}"
    if [[ "$scope" == "*" ]]; then
        EVERY=1
    else
        for path in $scope; do
            WANTED["$path"]=1
        done
    fi
done

# `-<sha> <path>` is git's spelling for "recorded, not checked out". An
# initialised submodule leads with a space or a `+`, so this collects exactly
# the ones that would fail a build, of those the named gates need.
MISSING=()
while read -r path; do
    if (( EVERY )) || [[ -n "${WANTED[$path]:-}" ]]; then
        MISSING+=("$path")
    fi
done < <(git -C "$REPO_ROOT" submodule status | awk '/^-/ { print $2 }')

(( ${#MISSING[@]} )) || exit 0

{
    printf '\nERROR gate: %d submodule(s) are recorded but not checked out, so nothing can be claimed:\n' \
        "${#MISSING[@]}"
    printf '  %s\n' "${MISSING[@]}"
    printf '\nThese gate(s) build against them: %s\n' "${SLUGS[*]}"
    printf 'A new `git worktree` and a plain `git clone` both start this way.\n'
    printf '\nRepair, from %s:\n' "$REPO_ROOT"
    printf '  git submodule update --init --recursive\n\n'
} >&2

exit 1
