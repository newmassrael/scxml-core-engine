#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/kotlin/mesh/src/commonMain/kotlin/generated/ from the
# standard Mesh documents (stdlib/mesh/*.scxml) — the Kotlin twin of
# backends/rust/mesh/generate.sh.
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
#
# Each document lands in its own package, `com.sce.generated.<document>`,
# the name the same document takes in every Kotlin build — and the one the
# C++ runtime gives it (`SCE::Generated::<Document>`).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
source "$REPO_ROOT/scripts/lib/sce_codegen.sh"

OUT_DIR="$SCRIPT_DIR/src/commonMain/kotlin/generated"
STDLIB_MESH="$REPO_ROOT/stdlib/mesh"

if command -v cargo >/dev/null 2>&1; then
    (cd "$REPO_ROOT" && cargo build --bin sce-codegen --features cli -p sce-build)
fi
SCE_CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

find "$OUT_DIR" -mindepth 1 -exec rm -rf {} + 2>/dev/null || true
mkdir -p "$OUT_DIR"

count=0
for path in "$STDLIB_MESH"/*.scxml; do
    "$SCE_CODEGEN" generate "$path" \
        --language kotlin \
        --output-dir "$OUT_DIR/" \
        --source-root "$REPO_ROOT" >/dev/null
    count=$((count + 1))
done

echo "Generated $count Kotlin documents under $OUT_DIR"
