#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: doc-content-gate.yml
#
# The sce-build library suite, run because a compiled-in INPUT changed
# rather than because its Rust did.
#
# `sce-build/src/` `include_str!`s documents, schemas, headers and Lua into
# the test binary, so editing one changes what the suite asserts without
# touching a `.rs`. `workspace-tests` runs the same assertions but its
# filter is narrow on purpose — it is 151s, half the push budget — so the
# wide filter lives on this lane instead and the expensive one keeps the
# filter it has.
#
# `--lib` rather than a list of test names: every input this lane exists
# for is read from `sce-build/src/`, so the library suite is exactly the
# set of assertions that see them. A named list would be a hand-kept list
# of the same shape as the one `include-str-coverage` exists to retire.
# `--features cli` because a target whose `required-features` are unmet is
# excluded silently, without building and without reporting a skip.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# ONE PART OF THE SUITE, when `SCE_DOC_CONTENT_SHARD=k/N` asks for one: the CI
# lane runs the library's tests as N jobs (doc-content-gate.yml, ADR 0120 D3).
# Whole, they ran 3730 tests in 463 s after a 283 s build (job 139036333453,
# 2026-11-25), and two tests set that length, one in `forge` and one in
# `conformance`. Every part builds the one test binary and asks it for its
# tests (`--list`); scripts/lib/workspace_shards.py (`--lib-list`) spreads them
# by top-level module and writes the part's exact names, which the part runs
# with `--exact`: libtest matches a filter anywhere in a name, so a module
# filter such as `tests::` would also run every nested `tests` module. The
# part then holds what libtest ran to the names it was given, every job writes
# the whole plan beside its part, and the lane's last job holds the plans to
# one digest and every part to a report that its modules ran (`ctest_shards.py
# verify`). Unset, the whole suite runs, which is what the build machines run.
if [[ -n "${SCE_DOC_CONTENT_SHARD:-}" ]]; then
    SHARD_PLAN="$SCE_REPO_ROOT/target/doc-content-shard"
    rm -rf "$SHARD_PLAN"
    mkdir -p "$SHARD_PLAN"
    LIST="$(mktemp)"
    LOG="$(mktemp)"
    sce_gate_on_exit "rm -f '$LIST' '$LOG'"
    sce_gate_step "building the library's test binary and listing its tests"
    cargo test -p sce-build --features cli --lib -- --list >"$LIST" 2>"$LOG" \
        || { tail -40 "$LOG" >&2; sce_gate_fail "the sce-build library's test binary did not build"; }
    shard_units="$(python3 scripts/lib/workspace_shards.py \
        --lib-list "$LIST" --out "$SHARD_PLAN" "$SCE_DOC_CONTENT_SHARD")" \
        || sce_gate_cannot_run "scripts/lib/workspace_shards.py could not name shard $SCE_DOC_CONTENT_SHARD"
    units=0
    while IFS= read -r unit; do
        case "$unit" in
            "module "*) units=$((units + 1)) ;;
            "") ;;
            *) sce_gate_cannot_run "scripts/lib/workspace_shards.py named a unit it does not define: $unit" ;;
        esac
    done <<<"$shard_units"
    mapfile -t names <"$SHARD_PLAN/libtest-names"
    sce_gate_step "shard $SCE_DOC_CONTENT_SHARD: $units module(s), ${#names[@]} test(s)"
    if (( ${#names[@]} == 0 )); then
        python3 scripts/lib/ctest_shards.py report --out "$SHARD_PLAN" --executed "$units" --disabled 0
        exit 0
    fi
    sce_gate_tested --log "$LOG" -- cargo test -p sce-build --features cli --lib -- --exact "${names[@]}" \
        || sce_gate_fail "sce-build library suite (compiled-in inputs), shard $SCE_DOC_CONTENT_SHARD"
    # Every test the part was given ran or was ignored, as it would have been in
    # the whole run, and none other: a name libtest did not match would leave
    # the part green over a test it never ran.
    result="$(grep -E '^test result: ' "$LOG" | tail -1)"
    passed="$(sed -nE 's/.* ([0-9]+) passed;.*/\1/p' <<<"$result")"
    ignored="$(sed -nE 's/.* ([0-9]+) ignored;.*/\1/p' <<<"$result")"
    [[ -n "$passed" && -n "$ignored" ]] \
        || sce_gate_fail "libtest printed no result line for shard $SCE_DOC_CONTENT_SHARD"
    (( passed + ignored == ${#names[@]} )) \
        || sce_gate_fail "shard $SCE_DOC_CONTENT_SHARD was given ${#names[@]} test(s) and libtest ran $passed and ignored $ignored"
    python3 scripts/lib/ctest_shards.py report --out "$SHARD_PLAN" --executed "$units" --disabled 0
    sce_gate_step "shard $SCE_DOC_CONTENT_SHARD: $passed test(s) passed, $ignored ignored"
    exit 0
fi

sce_gate_tested -- cargo test -p sce-build --features cli --lib \
    || sce_gate_fail "sce-build library suite (compiled-in inputs)"
