#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/swift/mesh-wss/Tests/SceMeshWssTests/Generated/ — the Swift twin of the
# peer-table half of backends/kotlin/mesh/generate.sh.
#
# The cross-language loopback test's peer table (SCE_MESH.md §mesh-19): the client machine of
# tests/mesh/wss_loopback/deploy.yaml, which LoopbackTests builds its router from. Generated with the
# fixture directory as the input root, which is what the drift check recomputes
# (sce-build/tests/b9_drift_detection.rs).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
source "$REPO_ROOT/scripts/lib/sce_codegen.sh"

if command -v cargo >/dev/null 2>&1; then
    (cd "$REPO_ROOT" && cargo build --bin sce-codegen --features cli -p sce-build)
fi
SCE_CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

LOOPBACK="$REPO_ROOT/tests/mesh/wss_loopback"
OUT_DIR="$SCRIPT_DIR/Tests/SceMeshWssTests/Generated"
find "$OUT_DIR" -mindepth 1 -exec rm -rf {} + 2>/dev/null || true
mkdir -p "$OUT_DIR"
"$SCE_CODEGEN" generate "$LOOPBACK/client.scxml" \
    --deploy "$LOOPBACK/deploy.yaml" \
    --language swift \
    --transport-only \
    --input-root "$LOOPBACK" \
    --output-dir "$OUT_DIR/" >/dev/null

echo "Generated the loopback client's peer table under $OUT_DIR"
