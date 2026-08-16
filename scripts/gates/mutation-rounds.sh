#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: mutation-rounds.yml
#
# The half of a mutation round that `mutation-cases` cannot answer.
#
# `mutation-cases` asks whether every case still finds an unambiguous anchor
# and still changes the tree. It says so in its own header: it does not build
# and does not run a test, so it reports "applies" and never "CAUGHT". Two
# ways a case stops being evidence are invisible to it, and both need a
# build:
#
#   * the mutated tree no longer COMPILES, so the round is INCONCLUSIVE and
#     the case proves nothing while reading as present; and
#   * the suite no longer turns red, so the mutation SURVIVES and the clause
#     it was written to defend is defended by nothing.
#
# `parallel_microstep_owns_exit_and_entry.cases` was in the first class for a
# day, recorded in the debt registry as CAUGHT 1/1, and surfaced only because
# somebody went back and ran it by hand. Between two commits, nobody does.
#
# ── What it runs, and why not everything ──────────────────────────
#
# A round is a rebuild and a test run per case, so the corpus cannot be run
# on every push. What decides the subset is the casefiles' own
# `mutation_targets` declaration, read through `scripts/mutate --targets` —
# never a second list here. That is the same rule `mutation-cases` follows
# for its trigger, and for the same reason: a list restating the union would
# be free to drift the moment a case names a new file. A casefile whose
# declared target is in the push's change set gets a full round; the rest are
# left to `mutation-cases`.
#
# Measured on a warm tree (2026-08-15, this machine):
#
#     datamodel_type_is_read_from_the_value.cases  5 cargo cases   139s
#     c11_datamodel_reader.cases                   3 ctest cases   237s
#
# Warm is the honest basis here: this gate depends on `workspace-tests` and
# `cpp-suite`, so the baseline build a round would otherwise pay for has
# already happened by the time it runs.
#
# ── Without a change set ──────────────────────────────────────────
#
# `scripts/gate --all` and a by-hand invocation pass no change set, so this
# derives one the way the push hook derives its own: the branch against
# `origin/HEAD`, plus whatever is uncommitted. A gate whose by-hand result
# can differ from its in-hook result is not one you can check with before
# pushing, and that is the property `scripts/gate` already states about the
# toolchain it pins.
#
# Only when no range resolves at all does the corpus run whole — the same
# answer `tools/git-hooks/pre-push` gives for the same condition, and for the
# same reason: an unknown range is not an empty one. `SCE_MUTATION_ROUNDS`
# names an explicit subset, or `all` for the whole corpus deliberately.
#
# Running the corpus unconditionally was the first shape of this gate, and it
# was wrong in a way worth recording: it made the one number the registry
# keeps about a gate — `cost_s`, which decides run order — describe a mode
# almost nobody runs, and it put an hour into the path a developer reaches
# for when a push range is merely unusual. A gate expensive enough to be
# turned off protects nothing.
#
# What this gate does NOT catch, stated rather than discovered later: a case
# can stop being evidence because the TEST that caught it was weakened, with
# its target untouched. The change set cannot see that — a casefile declares
# a test selector, not the sources behind it — so such a case keeps its last
# verdict until the corpus is run whole.
#
# This gate runs in CI, not in the push hook (`ci_only` in
# `gate_registry.py`). It was in the hook first, and the number is why it
# moved: 877s on the first push whose change set reached its casefiles,
# against 4s on a push that reached none. A push whose length swings by a
# quarter of an hour on what the author happened to edit is one that gets
# bypassed. The cost of that trade, stated rather than discovered: a red
# arrives one round later, and `--no-verify` no longer has anything to skip
# here because there is nothing here to skip.
#
# `scripts/gate mutation-rounds` still runs it by hand, and the lane calls
# exactly that, so both judge the corpus by one set of rules.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

CORPUS="sce-build/tests/mutations"

mapfile -t CASEFILES < <(git ls-files "$CORPUS/*.cases")
if (( ${#CASEFILES[@]} == 0 )); then
    sce_gate_fail "found no casefile under $CORPUS — the sweep is not reaching the corpus"
fi

# Every casefile's declaration, read once through the harness that owns the
# casefile vocabulary. `runner` says whether a round needs a configured CMake
# tree; `target` lines are what the change set is intersected against.
declare -A RUNNER_OF=()
declare -A TARGETS_OF=()
for casefile in "${CASEFILES[@]}"; do
    if ! declared="$(scripts/mutate --declares "$casefile" 2>&1)"; then
        sce_gate_fail "could not read the declaration of $casefile:
$declared
A casefile whose declaration cannot be read is one this gate cannot decide
about, and passing over it would be the silent coverage loss the gate exists
to remove."
    fi
    while IFS=$'\t' read -r key value; do
        case "$key" in
            runner) RUNNER_OF["$casefile"]="$value" ;;
            target) TARGETS_OF["$casefile"]+="$value"$'\n' ;;
        esac
    done <<<"$declared"
done

# ── Choose the casefiles to run ───────────────────────────────────
SELECTED=()
reason=""

if [[ "${SCE_MUTATION_ROUNDS:-}" == "all" ]]; then
    reason="SCE_MUTATION_ROUNDS=all — the whole corpus"
    SELECTED=("${CASEFILES[@]}")
elif [[ -n "${SCE_MUTATION_ROUNDS:-}" ]]; then
    reason="named by SCE_MUTATION_ROUNDS"
    IFS=',' read -r -a SELECTED <<<"$SCE_MUTATION_ROUNDS"
    for casefile in "${SELECTED[@]}"; do
        [[ -n "${RUNNER_OF[$casefile]:-}" ]] \
            || sce_gate_fail "SCE_MUTATION_ROUNDS names $casefile, which is not a tracked casefile under $CORPUS"
    done
else
    CHANGED=()
    if [[ -n "${SCE_GATE_CHANGED_FILE:-}" && -f "${SCE_GATE_CHANGED_FILE}" ]]; then
        reason="declared target in the push's change set"
        mapfile -t CHANGED < <(sort -u "$SCE_GATE_CHANGED_FILE")
    # `@{upstream}` first and `origin/HEAD` only as a fallback, because the
    # tracking ref is the one that is actually configured: measured on this
    # repository, `origin/HEAD` is not a symbolic ref at all, so a derivation
    # that asked for it first fell straight through to "no resolvable range"
    # and ran the whole corpus — the expensive answer, reached by a lookup
    # that was simply wrong.
    elif base="$(git merge-base HEAD '@{upstream}' 2>/dev/null || git merge-base HEAD origin/HEAD 2>/dev/null)" \
         && [[ -n "$base" ]]; then
        reason="declared target changed since $(git rev-parse --short "$base")"
        mapfile -t CHANGED < <(
            { git diff --name-only "$base" HEAD; git status --porcelain --short | cut -c4-; } | sort -u
        )
    else
        # No range resolves — pre-push answers this condition by running
        # every gate, and the honest answer here is the same shape: an
        # unknown range is not an empty one.
        reason="no resolvable range — the whole corpus"
        SELECTED=("${CASEFILES[@]}")
    fi

    if (( ${#SELECTED[@]} == 0 )); then
        # The change set is paths and a casefile's declaration is paths, so
        # the intersection is exact rather than glob-matched: both sides are
        # repository-relative and written by the same tooling.
        for casefile in "${CASEFILES[@]}"; do
            while IFS= read -r target; do
                [[ -n "$target" ]] || continue
                for changed in ${CHANGED[@]+"${CHANGED[@]}"}; do
                    if [[ "$changed" == "$target" ]]; then
                        SELECTED+=("$casefile")
                        break 2
                    fi
                done
            done <<<"${TARGETS_OF[$casefile]:-}"
        done
    fi
fi

# The count is printed whatever it is, including zero. A gate that says
# nothing when it did nothing is indistinguishable from one that passed, and
# this one legitimately has nothing to do on most pushes.
sce_gate_step "${#SELECTED[@]} of ${#CASEFILES[@]} casefile(s) to run — $reason"

# `SCE_MUTATION_ROUNDS_DRY_RUN=1` prints the selection and stops.
#
# It exists so the choosing can be tested without the running: what this gate
# decides is which casefiles a change set reaches, and a test that had to
# execute the rounds to observe that decision would take an hour to assert a
# property that is settled in milliseconds. The rounds themselves are
# `scripts/mutate`'s, and `mutation_case_liveness.rs` already covers its half.
#
# Ahead of the ctest precondition below, because selecting is not running.
# The workflow asks this dry run which casefiles a push reaches and configures
# CMake when one of them needs it — so a dry run that refused for the want of
# that tree refused precisely when the answer was "yes, configure one", and
# the job went red for having been asked. Measured on the first push to touch
# a C11 template: `c11_datamodel_reader.cases` was selected and the selection
# step exited 3 before printing it.
if [[ -n "${SCE_MUTATION_ROUNDS_DRY_RUN:-}" ]]; then
    printf '%s\n' ${SELECTED[@]+"${SELECTED[@]}"}
    exit 0
fi

# ── Refuse rather than skip what this machine cannot run ──────────
#
# A ctest round selects from a configured CMake tree. Without one there is
# nothing to select and nothing to build, and the two available answers are
# not equivalent: passing over those casefiles would report green for having
# examined nothing, while failing would blame the author's tree for a
# missing tool. Exit 3 is the third answer this repository already has a name
# for — the gate's own inputs are absent, so nothing is claimed.
#
# `cpp-suite` learned the same thing on a build machine without the mesh
# transport SDKs, where it reported FAILED for what was never its tree's
# fault.
if [[ ! -f build/CMakeCache.txt ]]; then
    needs_ctest=()
    for casefile in ${SELECTED[@]+"${SELECTED[@]}"}; do
        [[ "${RUNNER_OF[$casefile]}" == "ctest" ]] && needs_ctest+=("$casefile")
    done
    if (( ${#needs_ctest[@]} > 0 )); then
        sce_gate_cannot_run "no configured CMake tree at build/, and ${#needs_ctest[@]} selected casefile(s) declare a ctest selector: ${needs_ctest[*]}
Configure and build first (the cpp-suite and w3c-c11 gates do), or run only
the cargo casefiles with SCE_MUTATION_ROUNDS."
    fi
fi

if (( ${#SELECTED[@]} == 0 )); then
    sce_gate_step "no declared mutation target changed; the anchors are checked by mutation-cases"
    exit 0
fi

# ── Run them ──────────────────────────────────────────────────────
#
# `scripts/mutate` exits 0 only when every case was CAUGHT, and owns its own
# restore through an EXIT trap, so a failing round leaves the tree as it
# found it. Every round is run before the verdict rather than stopping at the
# first failure: a gate that stops early has not measured the rest, and the
# author would fix one and rediscover the next on the following push.
failed=()
for casefile in "${SELECTED[@]}"; do
    sce_gate_step "round: $casefile"
    if ! scripts/mutate "$casefile"; then
        failed+=("$casefile")
    fi
done

if (( ${#failed[@]} > 0 )); then
    sce_gate_fail "${#failed[@]} casefile(s) no longer prove what they claim: ${failed[*]}
Read the verdict above for which case and which kind. INCONCLUSIVE means the
mutated tree stopped compiling or the mutation reached nothing the tests
build — re-aim the case at a site that is still compiled into them. SURVIVED
means the suite no longer turns red, which is a finding about the suite, not
about the case: the clause that case defends is currently defended by nothing."
fi

sce_gate_step "${#SELECTED[@]} casefile(s) still turn their suites red"
