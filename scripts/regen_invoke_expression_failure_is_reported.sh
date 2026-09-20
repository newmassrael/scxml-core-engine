#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/rust/tests/src/integration/invoke_expression_failure_is_reported/
# from the canonical fixture at
# integration_resources/invoke_expression_failure_is_reported/invoke_expression_failure_is_reported.scxml.
#
# Pipeline:
#   sce-codegen generate <fixture> -o $TMP → generate each hybrid child
#   stub codegen wrote beside it → clear stale `_sm.rs` → copy in →
#   cargo fmt (pre-commit hook gate) → stitch mod.rs.
#
# The child loop is over `*_hybrid*.scxml` rather than the synth-invoke
# spelling the inline-`<content>` fixtures use. A hybrid `<invoke>`
# resolves its target expression at runtime, so codegen writes an
# immediate-`<final>` stub for it (docs/SCE_ACCEPTED_SUBSET.md §2.13) and
# the parent instantiates that stub by name — the file has to exist and be
# generated, or the parent does not compile.
#
# Usage (from repo root):
#   scripts/regen_invoke_expression_failure_is_reported.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
STEM="invoke_expression_failure_is_reported"
FIXTURE="integration_resources/$STEM/$STEM.scxml"
GENERATED_DIR="backends/rust/tests/src/integration/$STEM"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l rust -o "$TMP/"
for child in "$TMP"/*_hybrid*.scxml; do
    [ -e "$child" ] || continue
    "$CODEGEN" generate "$child" -l rust -o "$TMP/"
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
        find "$GENERATED_DIR" -maxdepth 1 -name "${STEM}_hybrid*_sm.rs" \
            -printf '%f\n' | sed 's/\.rs$//' | sort
    )
} > "$MODRS"

source "$REPO_ROOT/scripts/lib/sce_rustfmt.sh"
sce_rustfmt_dir "$GENERATED_DIR" "$REPO_ROOT"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
