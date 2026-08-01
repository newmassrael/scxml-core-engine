#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/rust/tests/src/integration/autoforward_dequeue_point/ from
# the canonical fixture at
# integration_resources/autoforward_dequeue_point/autoforward_dequeue_point.scxml.
#
# Pipeline:
#   sce-codegen generate <fixture> -o $TMP → auto-emits parent + every
#   inline `<invoke><content>` child as `_sm.rs` (SCE Mesh §9.6.6 rule 2;
#   codegen emit is the single materialization point for synth children)
#   → clear stale `_sm.rs` in the integration tree → copy the new ones in
#   → cargo fmt (pre-commit hook gate) → stitch mod.rs.
#
# No tmp staging of the fixture, no `// From:` sed rewrite, no
# `--input-root` override: now that the parser captures inline children
# in-memory and codegen sets `model.scxml_source_path` directly from
# the canonical input path, the embedded `// From:` and the §synth-6.2.6
# `source-hash` already point at `integration_resources/...`.
#
# Usage (from repo root):
#   scripts/regen_autoforward_dequeue_point.sh
#
# Requires:
#   target/release/sce-codegen (auto-built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

CODEGEN="target/release/sce-codegen"
FIXTURE="integration_resources/autoforward_dequeue_point/autoforward_dequeue_point.scxml"
GENERATED_DIR="backends/rust/tests/src/integration/autoforward_dequeue_point"
STEM="autoforward_dequeue_point"

if [[ ! -x "$CODEGEN" ]]; then
    cargo build --bin sce-codegen --features cli --release -p sce-build
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l rust -o "$TMP/"

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.rs' -delete
cp "$TMP"/*.rs "$GENERATED_DIR/"

# Step 7 (was): rebuild mod.rs from the actual `_sm.rs` filenames the
# regen produced. Sorted output keeps the diff stable across regens.
MODRS="$GENERATED_DIR/mod.rs"
{
    echo "// GENERATED -- DO NOT EDIT (scripts/regen_autoforward_dequeue_point.sh)"
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

cargo fmt -p sce-rust-tests

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
