#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/rust/tests/src/integration/an_origin_is_the_address_its_sender_published/
# from the canonical fixture at
# integration_resources/an_origin_is_the_address_its_sender_published/an_origin_is_the_address_its_sender_published.scxml.
#
# Pipeline: sce-codegen generate <fixture> -o $TMP (the parent and its inline
# `<invoke><content>` child) → clear stale `_sm.rs` in the integration tree →
# copy the new ones in → stitch mod.rs → cargo fmt (pre-commit hook gate).
#
# Usage (from repo root):
#   scripts/regen_an_origin_is_the_address_its_sender_published.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
FIXTURE="integration_resources/an_origin_is_the_address_its_sender_published/an_origin_is_the_address_its_sender_published.scxml"
GENERATED_DIR="backends/rust/tests/src/integration/an_origin_is_the_address_its_sender_published"
STEM="an_origin_is_the_address_its_sender_published"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l rust -o "$TMP/"

mkdir -p "$GENERATED_DIR"
source "$REPO_ROOT/scripts/lib/sce_generated_tree.sh"
sce_generated_tree_clear "$GENERATED_DIR"
cp "$TMP"/*.rs "$GENERATED_DIR/"

# Rebuild mod.rs from the actual `_sm.rs` filenames the regen produced. Sorted
# output keeps the diff stable across regens.
MODRS="$GENERATED_DIR/mod.rs"
{
    echo "// GENERATED -- DO NOT EDIT (scripts/regen_an_origin_is_the_address_its_sender_published.sh)"
    echo ""
    parent_stem="${STEM}_sm"
    echo "mod ${parent_stem};"
    echo "pub use ${parent_stem}::*;"
    while IFS= read -r child_stem; do
        echo "mod ${child_stem};"
        echo "pub use ${child_stem}::*;"
    done < <(
        find "$GENERATED_DIR" -maxdepth 1 -name "${STEM}__sce_synth_invoke__*_sm.rs" \
            -printf '%f\n' | sed 's/\.rs$//' | sort
    )
} > "$MODRS"

source "$REPO_ROOT/scripts/lib/sce_rustfmt.sh"
sce_rustfmt_dir "$GENERATED_DIR" "$REPO_ROOT"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
