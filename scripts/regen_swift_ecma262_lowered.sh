#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Write the Swift artifact the `ecma262-lowered-swift` gate runs:
# backends/swift/tests/Sources/Ecma262LoweredMachines/ecma262_lowered_sm.swift.
#
# The artifact is ONE machine that asks every case of the shared ECMA-262 table
# (`tests/ecmascript/ecma262_semantics.json`) as a `cond=` or an `expr=`, expanded from that
# table by `tools/generate_lowered_ecma262_fixture.py`, the expander the C++ and Kotlin lanes use
# too: one expander and one population, so a second copy of the document is not free to fall
# behind the table. Every expression reaches the machine's Lua session as the Lua the build-time
# frontend lowered, which is the only route Swift has, so what the machine answers is what
# `docs/SCE_LUA_TRANSLATION_SEAM.md` asks of a backend before it may stop passing source.
#
# The machine is not committed (as Go's W3C suite is not): a template edit would rewrite it, and
# the lane writes it before it runs it, so the lane judges the generator in the working tree and
# not an artifact somebody produced earlier. The expected answers are not in it either; the test
# reads them from the table.
#
# Usage (from repo root):
#   scripts/regen_swift_ecma262_lowered.sh
#
# Requires:
#   python3, and sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
source "$REPO_ROOT/scripts/lib/sce_generated_tree.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

# A gate whose whole subject is an artifact it cannot generate has nothing to say when the expander
# is missing, so this fails and does not skip.
command -v python3 > /dev/null 2>&1 || {
    echo "error: python3 is not on PATH; tools/generate_lowered_ecma262_fixture.py expands the document the artifact is generated from" >&2
    exit 1
}

OUTPUT_DIR="backends/swift/tests/Sources/Ecma262LoweredMachines"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

python3 tools/generate_lowered_ecma262_fixture.py \
    --cases tests/ecmascript/ecma262_semantics.json \
    -o "$TMP/ecma262_lowered.scxml"

# What an earlier run wrote is cleared by what each file says it is, so a machine the generator
# stopped writing does not stay behind.
sce_generated_tree_clear --recursive "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"
"$CODEGEN" generate "$TMP/ecma262_lowered.scxml" -l swift -o "$OUTPUT_DIR/" > /dev/null
echo "Swift ECMA-262 lowered artifact: written into $OUTPUT_DIR/"
