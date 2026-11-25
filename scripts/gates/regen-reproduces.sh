#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: regen-reproduces.yml
#
# The regeneration procedure reproduces the committed trees.
#
# `scripts/regen_all_committed_trees.sh` states the property in its own
# header — "regenerate and expect no diff" — and nothing checked it. What that bought, measured: the Rust W3C tree carried
# three `__sce_synth_invoke__*.scxml` documents that no run produces. They
# were on-disk output of an older generator; the parser now synthesises an
# invoke child in memory ("disk emission is a codegen concern, not a parser
# side-effect") and 38 of the 41 sourcemap references to such a document
# already named a path that was not there. Three happened to be committed, so
# a regeneration left them behind and a reviewer diffing the tree saw files
# that no procedure could explain.
#
# An earlier attempt at this gate regenerated into an `--output-dir` scratch
# root from inside a Rust test, saw four files it could not account for, and
# was dropped rather than shipped unexplained. Two things were wrong with that
# shape: `--output-dir` is a different code path from the one the procedure
# uses, and a Rust test cannot run `cargo fmt` against a tree that is not a
# workspace member — while rustfmt IS part of the committed form. This runs
# the procedure itself, unmodified, in a throwaway worktree of HEAD, and
# compares with git.
#
# A worktree rather than the checkout in place: the procedure writes into the
# tracked trees, so running it here would leave a developer's uncommitted work
# mixed with regenerated output, and a failure would leave the tree dirty. The
# worktree shares the object store, so checking one out is cheap.
#
# `SCE_CODEGEN_BIN` is passed through so the worktree reuses this checkout's
# binary instead of rebuilding it. The binary is a product of the commit under
# test either way — the gate registry makes `codegen-build` a dependency.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

CODEGEN="$(sce_gate_codegen)"

WT="$(mktemp -d)"
sce_gate_on_exit "git worktree remove --force '$WT' 2>/dev/null || rm -rf '$WT'"

# HEAD, not the working tree: the procedure's claim is about committed
# content. An uncommitted edit is the developer's business, not this gate's.
git worktree add --detach --quiet "$WT" HEAD \
    || sce_gate_fail "could not create a worktree of HEAD"

# The submodules, at the commits HEAD pins, from this checkout's own copies:
# a linked worktree starts with each one an empty directory, and the procedure
# emits the embed manifest, which parses the public headers with QuickJS,
# cpp-httplib and pugixml on the include path. The URL is overridden for the
# one command, never written: the worktree shares this checkout's config, and
# a written URL would repoint the checkout's own submodules. Cloning from the
# local copy needs no network and is what makes a run here mean what CI's does.
while read -r key path; do
    name="${key#submodule.}"
    name="${name%.path}"
    [[ -e "$SCE_REPO_ROOT/$path/.git" ]] \
        || sce_gate_fail "submodule $path is not checked out here (git submodule update --init)"
    git -C "$WT" -c protocol.file.allow=always -c "submodule.${name}.url=$SCE_REPO_ROOT/$path" \
        submodule update --init --quiet -- "$path" \
        || sce_gate_fail "could not check out submodule $path in the worktree"
done < <(git config -f .gitmodules --get-regexp '^submodule\..*\.path$')

# ONE SHARD OF THE STEPS, when `SCE_REGEN_SHARD=k/N` asks for one: the CI lane
# runs the regeneration as N jobs (regen-reproduces.yml), since it took 22 of
# the job's 28 minutes (job 138988228323). A step owns its trees and reads
# nothing another step writes (scripts/regen_all_committed_trees.sh), so a run
# of some steps alone reproduces their trees as the whole run does.
# scripts/lib/workspace_shards.py reads the steps from the one list that names
# them and spreads them; it plans from the worktree, so the steps are HEAD's,
# and writes the whole plan beside this shard's part, with the outputs each of
# its steps declares (`[actions.regen-<step>]` in bx.toml). The lane's last job
# holds the plans of all the shards to one digest and every shard to a report
# that its steps ran (`ctest_shards.py verify`), which is what makes the
# shards' union every tree this gate checks. Unset, the gate runs every step,
# which is what a push and the build machines run.
STEPS=()
SHARD_PLAN=""
SHARD_OUTPUTS=()
if [[ -n "${SCE_REGEN_SHARD:-}" ]]; then
    SHARD_PLAN="$SCE_REPO_ROOT/target/regen-shard"
    rm -rf "$SHARD_PLAN"
    shard_steps="$(python3 "$WT/scripts/lib/workspace_shards.py" --regen \
        --out "$SHARD_PLAN" "$SCE_REGEN_SHARD")" \
        || sce_gate_cannot_run "scripts/lib/workspace_shards.py could not plan shard $SCE_REGEN_SHARD of the regeneration's steps"
    while IFS= read -r unit; do
        case "$unit" in
            "step "*) STEPS+=("${unit#step }") ;;
            "") ;;
            *) sce_gate_cannot_run "scripts/lib/workspace_shards.py named a unit it does not define: $unit" ;;
        esac
    done <<<"$shard_steps"
    if (( ${#STEPS[@]} == 0 )); then
        python3 "$WT/scripts/lib/ctest_shards.py" report --out "$SHARD_PLAN" --executed 0 --disabled 0
        sce_gate_step "shard $SCE_REGEN_SHARD holds no step; the other shards regenerate every tree"
        exit 0
    fi
    mapfile -t SHARD_OUTPUTS < "$SHARD_PLAN/outputs"
    sce_gate_step "shard $SCE_REGEN_SHARD: regenerating the trees of ${STEPS[*]} in a worktree of HEAD"
else
    sce_gate_step "regenerating every committed tree in a worktree of HEAD"
fi

mkdir -p "$WT/target/debug"
cp "$CODEGEN" "$WT/target/debug/sce-codegen"

# Remove a sample of generated files first, so the run has to PRODUCE them
# rather than agree with what is already there.
#
# `write_if_changed` skips a file whose bytes already match, so a run over an
# intact tree rewrites nothing at all — measured, zero files — and the first
# version of this gate reported the strongest possible verdict for the
# weakest possible reason. Deleting first is what separates "reproduced" from
# "left alone": the tree carried three `__sce_synth_invoke__` documents the
# procedure had stopped producing. The procedure now sets aside every generated
# file of the trees it owns and removes each one no stage writes again
# (scripts/lib/sce_generated_tree.sh), which the check after the run names; the
# sample still reaches a tree outside that set.
#
# A SAMPLE rather than every generated file, because one file the procedure
# cannot rebuild is enough to stop it: `mod.rs` is an aggregator the arms
# read before they write, so emptying a tree fails for a reason that is not
# the property under test. The sample covers the W3C and integration trees
# alike — it did not always: while each regen script ended in a
# WHOLE-PACKAGE `cargo fmt`, a file missing in one tree aborted the format
# step of a script owning another, so the script that would have restored it
# never ran. Each script now formats only what it wrote, and the integration
# trees are inside this gate rather than excluded from it.
#
# The candidate set is derived, not listed: every tracked file carrying the
# generated-source drift header is generator output by construction, which is
# what keeps a new backend inside the gate without an edit here. (Prose, not a
# §-token: this comment describes the header, it does not implement the clause
# that defines it, and a citation here would be a claim of the second kind.)
mapfile -t GENERATED < <(cd "$WT" && git grep -l "source-hash:" -- backends \
    | grep -v '/mod\.rs$' || true)
# Floor on the CANDIDATE set, so a scan that stops matching fails loudly
# instead of sampling nothing. 976 artifacts carried the header when this was
# set; the floor is below that because adding a fixture raises the count and
# removing the scan's target is what it exists to catch.
MINIMUM=800
if (( ${#GENERATED[@]} < MINIMUM )); then
    sce_gate_fail "only ${#GENERATED[@]} generated file(s) found, under the ${MINIMUM} floor — the scan is broken, not the tree"
fi
# A shard samples only what its own steps write, by the outputs each declares:
# a file another shard's step writes would stay removed here, and read as a
# file the procedure no longer produces.
CANDIDATES=()
if [[ -n "$SHARD_PLAN" ]]; then
    for file in "${GENERATED[@]}"; do
        for glob in ${SHARD_OUTPUTS[@]+"${SHARD_OUTPUTS[@]}"}; do
            # shellcheck disable=SC2053  # the right side is the declared glob
            if [[ "$file" == $glob ]]; then
                CANDIDATES+=("$file")
                break
            fi
        done
    done
else
    CANDIDATES=("${GENERATED[@]}")
fi
SAMPLE=()
for (( i = 0; i < ${#CANDIDATES[@]}; i += 47 )); do
    SAMPLE+=("${CANDIDATES[i]}")
done
sce_gate_step "removing ${#SAMPLE[@]} of ${#CANDIDATES[@]} generated file(s) so the run has to restore them"
if (( ${#SAMPLE[@]} > 0 )); then
    ( cd "$WT" && rm -f "${SAMPLE[@]}" )
fi

REGEN_LOG="$(mktemp)"
sce_gate_on_exit "rm -f '$REGEN_LOG'"
# `SCE_WORKSPACE_ROOT` is not decoration. `locate_workspace_root` prefers the
# compile-time `CARGO_MANIFEST_DIR` parent over a walk up from the current
# directory — deliberately, so a vendored binary finds the SCE tree it
# shipped with — and a linked worktree's `.git` is a file, so nothing else
# corrects it. Measured without this line: the run wrote nine files into the
# ORIGINAL checkout while reporting success in the worktree, which is also
# the likeliest explanation for the four files an earlier attempt at this
# gate could not account for from inside a Rust test.
if ! ( cd "$WT" && SCE_WORKSPACE_ROOT="$WT" \
        ./scripts/regen_all_committed_trees.sh ${STEPS[@]+"${STEPS[@]}"} ) >"$REGEN_LOG" 2>&1; then
    tail -40 "$REGEN_LOG" >&2
    sce_gate_fail "the regeneration procedure did not complete"
fi

# Where the time went, step by step: each step names itself and says how long
# each of its stages took, and `workspace_shards.py --measure` reads these
# lines back into the table that spreads the steps over the lane's shards.
grep -E '^(=== step |    .+: [0-9]+s$)' "$REGEN_LOG" || true

# A committed generated file the procedure no longer produces: the procedure
# removed it, so it is a tracked file the worktree lacks. Named on its own and
# first, because comparing the files a run writes could never see it -- the
# measured case is two `test192__sce_synth_invoke__invokedChild` files left
# committed for days beside the `0zinvokedzuchild` ones that replaced them.
source "$(dirname "${BASH_SOURCE[0]}")/../lib/sce_generated_tree.sh"
mapfile -t UNPRODUCED < <(sce_generated_tree_unproduced "$WT")
if (( ${#UNPRODUCED[@]} > 0 )); then
    printf '\n  Committed, and no longer produced by the regeneration:\n' >&2
    printf '    %s\n' "${UNPRODUCED[@]}" >&2
    regen_changed=()
    for path in "${UNPRODUCED[@]}"; do regen_changed+=(--changed "$path"); done
    sce_gate_repro_from /dev/null "${regen_changed[@]}" -- scripts/regen_all_committed_trees.sh
    sce_gate_fail "${#UNPRODUCED[@]} committed generated file(s) are not produced by the regeneration procedure — commit their removal, or restore what produced them"
fi

CHANGED="$(cd "$WT" && git status --porcelain -- . | grep -v '^?? target/' || true)"
if [[ -n "$CHANGED" ]]; then
    printf '\n%s\n' "$CHANGED" >&2
    # What reproduces it: the declared tree each changed file is in,
    # materialized in a checkout of HEAD, then what git sees of the files.
    regen_changed=()
    first=""
    while IFS= read -r line; do
        path="${line:3}"
        path="${path##* -> }"
        if [[ -n "$path" ]]; then
            regen_changed+=(--changed "$path")
            first="${first:-$path}"
        fi
    done <<<"$CHANGED"
    sce_gate_repro_from /dev/null ${regen_changed[@]+"${regen_changed[@]}"} \
        -- scripts/regen_all_committed_trees.sh
    printf '\n  Diff of the first changed file:\n' >&2
    git -C "$WT" --no-pager diff -- "$first" | sed -n '1,40p' >&2
    sce_gate_fail "regenerating the committed trees changed them — the procedure does not reproduce what is committed"
fi

# What this shard ran, for the lane's last job to hold beside the other shards'
# (`ctest_shards.py verify`): a step counts when the procedure started it, and
# the procedure completed, so each step it started also finished.
if [[ -n "$SHARD_PLAN" ]]; then
    ran=0
    for step in "${STEPS[@]}"; do
        if grep -qxF "=== step $step" "$REGEN_LOG"; then
            ran=$((ran + 1))
        fi
    done
    if (( ran != ${#STEPS[@]} )); then
        sce_gate_fail "the procedure ran $ran of the ${#STEPS[@]} step(s) of shard $SCE_REGEN_SHARD"
    fi
    python3 "$WT/scripts/lib/ctest_shards.py" report --out "$SHARD_PLAN" --executed "$ran" --disabled 0
    sce_gate_step "the trees of shard $SCE_REGEN_SHARD (${STEPS[*]}) reproduced byte for byte"
    exit 0
fi

sce_gate_step "every committed tree reproduced byte for byte"
