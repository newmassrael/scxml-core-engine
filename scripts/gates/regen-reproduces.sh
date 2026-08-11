#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: none
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

REGEN_LOG="$(mktemp)"
sce_gate_on_exit "rm -f '$REGEN_LOG'"
if ! ( cd "$WT" && SOURCE_DATE_EPOCH=0 ./scripts/regen_all_committed_trees.sh ) \
        >"$REGEN_LOG" 2>&1; then
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
