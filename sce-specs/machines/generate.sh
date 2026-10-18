#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate the machines of the specs crate from the documents that hold their rules:
#
#   sce-specs/machines/request_life/request_life.scxml  ->  sce-specs/src/request_life/generated/
#   sce-specs/machines/import_life/import_life.scxml    ->  sce-specs/src/import_life/generated/
#
# Each document is the one place the rules of its record are written (a generation request, ADR
# 0016; an import proposal, ADR 0037); `sce-specs/src/requests.rs` and `sce-specs/src/proposals.rs`
# ask the machine this produces what an event comes to. Generated from the file path with the
# repository as the source root, so the header says the same thing wherever the checkout lives,
# and formatted as the workspace's `cargo fmt --check` requires, like every committed Rust tree.
#
# One script for both, so the regeneration procedure keeps one entry and a machine added here is
# judged by `regen-reproduces` the day it lands.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
source "$REPO_ROOT/scripts/lib/sce_rustfmt.sh"

MACHINES=(request_life import_life)

if command -v cargo >/dev/null 2>&1; then
    (cd "$REPO_ROOT" && cargo build --bin sce-codegen --features cli -p sce-build)
fi
SCE_CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

for machine in "${MACHINES[@]}"; do
    OUT_DIR="$REPO_ROOT/sce-specs/src/$machine/generated"
    DOCUMENT="$SCRIPT_DIR/$machine/$machine.scxml"

    find "$OUT_DIR" -mindepth 1 -exec rm -rf {} + 2>/dev/null || true
    mkdir -p "$OUT_DIR"

    "$SCE_CODEGEN" generate "$DOCUMENT" \
        --language rust \
        --output-dir "$OUT_DIR/" \
        --source-root "$REPO_ROOT" >/dev/null

    # The sourcemap is a build-time companion the committed tree does not keep.
    rm -f "$OUT_DIR/sce_sourcemap.json"

    sce_rustfmt_dir "$OUT_DIR" "$REPO_ROOT"

    echo "Generated the $machine machine under $OUT_DIR"
done
