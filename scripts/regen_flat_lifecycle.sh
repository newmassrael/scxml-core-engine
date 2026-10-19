#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/rust/tests/src/integration/flat_lifecycle/
# from the canonical fixture at
# integration_resources/flat_lifecycle/flat_lifecycle.scxml.
#
# The document declares its transition function (`sce:transition-function="emit"`,
# docs/SCE_TRANSITION_FUNCTION.md), so generation writes two files: the machine,
# `flat_lifecycle_sm.rs`, and the function, `flat_lifecycle_transition_function.rs`.
# Both are copied in, and mod.rs re-exports the machine and declares the function
# as a public module of its own, the way a crate holds the two side by side.
#
# Usage (from repo root):
#   scripts/regen_flat_lifecycle.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
FIXTURE="integration_resources/flat_lifecycle/flat_lifecycle.scxml"
GENERATED_DIR="backends/rust/tests/src/integration/flat_lifecycle"
STEM="flat_lifecycle"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l rust -o "$TMP/"

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 \( -name '*_sm.rs' -o -name '*_transition_function.rs' \) -delete
cp "$TMP"/*.rs "$GENERATED_DIR/"

MODRS="$GENERATED_DIR/mod.rs"
{
    echo "// GENERATED -- DO NOT EDIT (scripts/regen_${STEM}.sh)"
    echo ""
    echo "mod ${STEM}_sm;"
    echo "pub use ${STEM}_sm::*;"
    echo "pub mod ${STEM}_transition_function;"
} > "$MODRS"

source "$REPO_ROOT/scripts/lib/sce_rustfmt.sh"
sce_rustfmt_dir "$GENERATED_DIR" "$REPO_ROOT"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
