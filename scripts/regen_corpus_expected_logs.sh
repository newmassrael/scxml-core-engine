#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Write the expected log of every trace of the corpus of the TTCN-3 lane
# (docs/SCE_TTCN3_LANE.md §12): each suite's Rust runner program, the
# authoritative one (§10.4), replays the suite's directory of traces in the
# world that selects every test case, and each log it writes becomes
# `expected_logs/<test case>.json`, the canonical log with one newline after it.
# Every backend's test of the corpus is held to these files byte for byte, so a
# file written here is reviewed against docs/SCE_TEST_PROFILE.md §12.5 before it
# is committed: the bytes are what the run did, not what the run should do.
#
# Usage (from repo root):
#   scripts/regen_corpus_expected_logs.sh           write every expected log
#   scripts/regen_corpus_expected_logs.sh --check   exit 1 where one differs
#
# Requires:
#   the generated Rust trees of the suites (scripts/regen_static_datamodel_rust.sh),
#   which the runner programs of sce-rust-tests are built from.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

CHECK=0
case "${1:-}" in
    "") ;;
    --check) CHECK=1 ;;
    *) echo "usage: $0 [--check]" >&2; exit 2 ;;
esac

PROFILE="sce-build/tests/fixtures/test_profile"
# Each suite: its runner program, the directory of its traces and expected logs,
# and the world that selects every test case. runtime_suite's runs stand beside
# its documents; every other suite's under runs/<suite>.
SUITES=(
    "runtime_suite_runner|$PROFILE/runtime|ptcs,removal,guard"
    "boundary_suite_runner|$PROFILE/runs/boundary_suite|"
)

cargo build -q -p sce-rust-tests --bins
TARGET_DIR="$(cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

differ=0
for entry in "${SUITES[@]}"; do
    IFS='|' read -r runner runs world <<< "$entry"
    logs="$TMP/$runner"
    "$TARGET_DIR/debug/$runner" --trace-dir "$runs/traces" --log-dir "$logs" --world "$world" > "$TMP/$runner.out"
    mkdir -p "$runs/expected_logs"
    for log in "$logs"/*.log.json; do
        case_name="$(basename "$log" .log.json)"
        expected="$runs/expected_logs/$case_name.json"
        if [ "$CHECK" -eq 1 ]; then
            if ! { cat "$log"; printf '\n'; } | cmp -s - "$expected"; then
                echo "differs: $expected" >&2
                differ=1
            fi
        else
            { cat "$log"; printf '\n'; } > "$expected"
        fi
    done
    echo "$runner:"
    cat "$TMP/$runner.out"
done

if [ "$CHECK" -eq 1 ] && [ "$differ" -ne 0 ]; then
    exit 1
fi
