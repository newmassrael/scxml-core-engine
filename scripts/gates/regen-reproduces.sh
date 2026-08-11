#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: regen-reproduces.yml
#
# The regeneration procedure reproduces the committed trees.
#
# `scripts/regen_all_committed_trees.sh` states the property in its own
# header — pin SOURCE_DATE_EPOCH, "regenerate and expect no diff" — and
# nothing checked it. What that bought, measured: the Rust W3C tree carried
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
sce_gate_step "regenerating every committed tree in a worktree of HEAD"

mkdir -p "$WT/target/debug"
cp "$CODEGEN" "$WT/target/debug/sce-codegen"

# Remove a sample of generated files first, so the run has to PRODUCE them
# rather than agree with what is already there.
#
# `write_if_changed` skips a file whose bytes already match, so a run over an
# intact tree rewrites nothing at all — measured, zero files — and the first
# version of this gate reported the strongest possible verdict for the
# weakest possible reason. Deleting first is what separates "reproduced" from
# "left alone", and it is the only shape that catches a file the procedure
# has stopped producing: the tree carried three `__sce_synth_invoke__`
# documents in exactly that state.
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
mapfile -t GENERATED < <(cd "$WT" && git grep -l "template-hash:" -- backends \
    | grep -v '/mod\.rs$' || true)
# Floor on the CANDIDATE set, so a scan that stops matching fails loudly
# instead of sampling nothing. 976 artifacts carried the header when this was
# set; the floor is below that because adding a fixture raises the count and
# removing the scan's target is what it exists to catch.
MINIMUM=800
if (( ${#GENERATED[@]} < MINIMUM )); then
    sce_gate_fail "only ${#GENERATED[@]} generated file(s) found, under the ${MINIMUM} floor — the scan is broken, not the tree"
fi
SAMPLE=()
for (( i = 0; i < ${#GENERATED[@]}; i += 47 )); do
    SAMPLE+=("${GENERATED[i]}")
done
sce_gate_step "removing ${#SAMPLE[@]} of ${#GENERATED[@]} generated file(s) so the run has to restore them"
( cd "$WT" && rm -f "${SAMPLE[@]}" )

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
if ! ( cd "$WT" && SOURCE_DATE_EPOCH=0 SCE_WORKSPACE_ROOT="$WT" \
        ./scripts/regen_all_committed_trees.sh ) >"$REGEN_LOG" 2>&1; then
    tail -40 "$REGEN_LOG" >&2
    sce_gate_fail "the regeneration procedure did not complete"
fi

CHANGED="$(cd "$WT" && git status --porcelain -- . | grep -v '^?? target/' || true)"
if [[ -n "$CHANGED" ]]; then
    printf '\n%s\n' "$CHANGED" >&2
    printf '\n  Diff of the first changed file:\n' >&2
    first="$(printf '%s\n' "$CHANGED" | head -1 | awk '{print $2}')"
    ( cd "$WT" && git --no-pager diff -- "$first" | head -40 ) >&2
    sce_gate_fail "regenerating the committed trees changed them — the procedure does not reproduce what is committed"
fi

sce_gate_step "every committed tree reproduced byte for byte"
