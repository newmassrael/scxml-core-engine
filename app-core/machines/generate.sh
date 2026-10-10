#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate app-core/src/request_life/generated/ from the request's life
# (app-core/machines/request_life/request_life.scxml).
#
# The document is the one place the rules of a generation request are written;
# `app-core/src/requests.rs` asks the machine this produces what an event comes
# to. Generated from the file path with the repository as the source root, so the
# header says the same thing wherever the checkout lives, and formatted as the
# workspace's `cargo fmt --check` requires, like every committed Rust tree.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$REPO_ROOT/scripts/lib/sce_codegen.sh"

OUT_DIR="$REPO_ROOT/app-core/src/request_life/generated"
DOCUMENT="$SCRIPT_DIR/request_life/request_life.scxml"

if command -v cargo >/dev/null 2>&1; then
    (cd "$REPO_ROOT" && cargo build --bin sce-codegen --features cli -p sce-build)
fi
SCE_CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

find "$OUT_DIR" -mindepth 1 -exec rm -rf {} + 2>/dev/null || true
mkdir -p "$OUT_DIR"

"$SCE_CODEGEN" generate "$DOCUMENT" \
    --language rust \
    --output-dir "$OUT_DIR/" \
    --source-root "$REPO_ROOT" >/dev/null

# The sourcemap is a build-time companion the committed tree does not keep.
rm -f "$OUT_DIR/sce_sourcemap.json"

source "$REPO_ROOT/scripts/lib/sce_rustfmt.sh"
sce_rustfmt_dir "$OUT_DIR" "$REPO_ROOT"

echo "Generated the request machine under $OUT_DIR"
