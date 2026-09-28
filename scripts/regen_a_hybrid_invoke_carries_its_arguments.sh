#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/rust/tests/src/integration/a_hybrid_invoke_carries_its_arguments/
# from the canonical fixture at
# integration_resources/a_hybrid_invoke_carries_its_arguments/a_hybrid_invoke_carries_its_arguments.scxml.
#
# The children are the documents `sce:candidates` declares. Codegen stages
# them beside the parent's output, and each is generated as a child so the
# parent's reference to it resolves.
#
# Usage (from repo root):
#   scripts/regen_a_hybrid_invoke_carries_its_arguments.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
STEM="a_hybrid_invoke_carries_its_arguments"
FIXTURE_DIR="integration_resources/$STEM"
FIXTURE="$FIXTURE_DIR/$STEM.scxml"
GENERATED_DIR="backends/rust/tests/src/integration/$STEM"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l rust -o "$TMP/"
# ⚠ Generated from the TRACKED document, not from the copy the parent run
# staged beside itself, so the `// From:` line of a committed artefact names
# a path that is the same on every run (`regen-reproduces` reads a `$TMP`
# path there as the tree failing to regenerate).
for child in "$TMP"/*.scxml; do
    [ -e "$child" ] || continue
    base="$(basename "$child")"
    [ "$base" = "$STEM.scxml" ] && continue
    src="$FIXTURE_DIR/$base"
    [ -f "$src" ] || src="$child"
    "$CODEGEN" generate "$src" --as-child --parent-stem "$STEM" -l rust -o "$TMP/"
done

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.rs' -delete
cp "$TMP"/*.rs "$GENERATED_DIR/"

MODRS="$GENERATED_DIR/mod.rs"
{
    echo "// GENERATED -- DO NOT EDIT (scripts/regen_$STEM.sh)"
    echo ""
    echo "mod ${STEM}_sm;"
    echo "pub use ${STEM}_sm::*;"
    while IFS= read -r child_stem; do
        echo "mod ${child_stem};"
        echo "pub use ${child_stem}::*;"
    done < <(
        find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.rs' -printf '%f\n' \
            | sed 's/\.rs$//' | grep -v "^${STEM}_sm$" | sort
    )
} > "$MODRS"

source "$REPO_ROOT/scripts/lib/sce_rustfmt.sh"
sce_rustfmt_dir "$GENERATED_DIR" "$REPO_ROOT"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
