#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/swift/tests/Sources/EcmaScriptMachines/ from
# sce-build/tests/fixtures/swift_ecmascript/<machine>.scxml, for every
# `datamodel="ecmascript"` statechart the Swift backend lowers.
#
# These machines run their expressions on the Lua 5.4 engine of the `SceLua` module
# (docs/SCE_LUA_TRANSLATION_SEAM.md), so they are a target of their own in
# backends/swift/tests: a machine that needs no engine does not link Lua. The W3C
# documents among the fixtures are links to `resources/`, not copies, so the oracle has
# one definition.
#
# Usage (from repo root):
#   scripts/regen_swift_ecmascript.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
INPUT_ROOT="sce-build/tests/fixtures/swift_ecmascript"
GENERATED_DIR="backends/swift/tests/Sources/EcmaScriptMachines"

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
