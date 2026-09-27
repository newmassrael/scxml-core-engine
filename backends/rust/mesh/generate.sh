#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/rust/mesh/src/generated/ from the standard Mesh
# documents (stdlib/mesh/*.scxml), and the module list that declares them.
#
# The set is read from the directory rather than listed here: every document
# under sce:std/mesh is part of the envelope or of a delivery rule the router
# applies, and a list written in this file could fall behind the directory
# without anything noticing — `sce-codegen verify` checks the files a tree
# holds, not the files it should hold.
#
# Generated from file paths, not `sce:std/...` names, so the headers match
# what `verify --input-root stdlib/mesh` recomputes
# (sce-build/tests/b9_drift_detection.rs).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
source "$REPO_ROOT/scripts/lib/sce_codegen.sh"

OUT_DIR="$SCRIPT_DIR/src/generated"
MODULE_LIST="$SCRIPT_DIR/src/generated.rs"
STDLIB_MESH="$REPO_ROOT/stdlib/mesh"

if command -v cargo >/dev/null 2>&1; then
    (cd "$REPO_ROOT" && cargo build --bin sce-codegen --features cli -p sce-build)
fi
SCE_CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

find "$OUT_DIR" -mindepth 1 -exec rm -rf {} + 2>/dev/null || true
mkdir -p "$OUT_DIR"

documents=()
for path in "$STDLIB_MESH"/*.scxml; do
    documents+=("$(basename "$path" .scxml)")
done

for document in "${documents[@]}"; do
    "$SCE_CODEGEN" generate "$STDLIB_MESH/$document.scxml" \
        --language rust \
        --output-dir "$OUT_DIR/" \
        --source-root "$REPO_ROOT" >/dev/null
done

{
    echo "// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial"
    echo "// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael"
    echo ""
    echo "//! The standard Mesh documents (\`sce:std/mesh\`), generated."
    echo "//!"
    echo "//! GENERATED -- DO NOT EDIT (backends/rust/mesh/generate.sh)."
    echo "//! Each module is one document; they name one another as siblings"
    echo "//! (\`super::envelope_id\`), which is why they share this parent."
    echo ""
    for document in "${documents[@]}"; do
        echo "pub mod ${document};"
    done
} > "$MODULE_LIST"

# The workspace's `cargo fmt --check` covers a committed tree like any other
# source, so it is committed formatted — the same step every committed Rust
# tree's regeneration ends with.
source "$REPO_ROOT/scripts/lib/sce_rustfmt.sh"
sce_rustfmt_dir "$OUT_DIR" "$REPO_ROOT"

echo "Generated ${#documents[@]} Rust modules under $OUT_DIR"
