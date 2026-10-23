#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/swift/tests/Sources/StaticMachines/ from
# sce-build/tests/fixtures/swift_statecharts/<machine>.scxml, for every statechart
# the Swift backend lowers.
#
# The Swift compile+run gate: the committed machines are one SwiftPM target of
# backends/swift/tests, so their states, events and tables really are Swift that
# compiles against the runtime (backends/swift/runtime), and the tests beside
# them drive each through its `Engine`.
#
# Usage (from repo root):
#   scripts/regen_swift_statecharts.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
INPUT_ROOT="sce-build/tests/fixtures/swift_statecharts"
GENERATED_DIR="backends/swift/tests/Sources/StaticMachines"

# Derived, not listed: every statechart the fixture directory holds. A refusal by
# name is exit status 7 (generate/unsupported-feature) and is a defect here,
# since a fixture of this directory exists to be lowered.
mapfile -t SOURCES < <(find "$INPUT_ROOT" -maxdepth 1 -name '*.scxml' | LC_ALL=C sort)
if [ "${#SOURCES[@]}" -lt 1 ]; then
    echo "error: no statechart found under $INPUT_ROOT" >&2
    exit 1
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

MACHINES=()
for source in "${SOURCES[@]}"; do
    machine="$(basename "$source" .scxml)"
    MACHINES+=("$machine")
    "$CODEGEN" generate "$source" -l swift -o "$TMP/$machine/"
done

mkdir -p "$GENERATED_DIR"
# Every file below is generated and the tests beside them are not: clearing by
# what each file says it is takes the first and leaves the second.
source "$REPO_ROOT/scripts/lib/sce_generated_tree.sh"
sce_generated_tree_clear --recursive "$GENERATED_DIR"
for machine in "${MACHINES[@]}"; do
    cp "$TMP/$machine"/*_sm.swift "$GENERATED_DIR/"
done

echo "Regenerated: ${MACHINES[*]} under $GENERATED_DIR/"
