#!/bin/bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate the Mesh envelope codec the C++ runtime is built on
# (sce/src/mesh/generated/): the standard document sce:std/mesh/envelope.scxml
# and the three enums it imports, generated for C++. MeshEnvelopeCodec.cpp
# converts the runtime's MeshEnvelope to and from this codec's value, so the
# envelope's CBOR is written and read by the same generated code every other
# backend runs (SCE_FORGE.md §4.6.1).
#
# The tree is committed rather than generated at build time so a C++
# consumer of the runtime needs no Rust toolchain; the drift test
# (sce-build/tests/b9_drift_detection.rs) holds it to the standard library.
# Run after editing stdlib/mesh/*.scxml or the cpp codec / enum templates:
#
#   sce/src/mesh/generate_envelope.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
OUT_DIR="$SCRIPT_DIR/generated"

if command -v cargo >/dev/null 2>&1; then
    (cd "$REPO_ROOT" && cargo build --bin sce-codegen --features cli -p sce-build)
fi

SCE_CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

find "$OUT_DIR" -mindepth 1 -exec rm -rf {} + 2>/dev/null || true
mkdir -p "$OUT_DIR"

# Named by path under stdlib/mesh/, not by `sce:std/...` name: a document
# generated from a path embeds the source hash of its own directory, which
# is the root the drift test verifies this tree against (stdlib/mesh/).
STDLIB_MESH="$REPO_ROOT/stdlib/mesh"
DOCUMENTS=(
    envelope
    pattern_kind
    payload_codec
    rpc_status
)

for document in "${DOCUMENTS[@]}"; do
    "$SCE_CODEGEN" generate "$STDLIB_MESH/$document.scxml" \
        --language cpp \
        --output-dir "$OUT_DIR/" \
        --source-root "$REPO_ROOT" >/dev/null
done

echo "Generated ${#DOCUMENTS[@]} C++ headers under $OUT_DIR"
