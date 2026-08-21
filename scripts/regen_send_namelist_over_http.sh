#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/rust/tests/src/integration/send_namelist_over_http/ from
# the canonical fixture at
# integration_resources/send_namelist_over_http/send_namelist_over_http.scxml.
#
# Pipeline:
#   sce-codegen generate <fixture> -o $TMP → clear stale `_sm.rs` in the
#   integration tree → copy the new ones in → cargo fmt (pre-commit hook
#   gate) → stitch mod.rs.
#
# Usage (from repo root):
#   scripts/regen_send_namelist_over_http.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).
#   The fixture posts over BasicHTTP; RUNNING the generated test additionally
#   needs the W3C harness server (`node tests/w3c/standalone_http_server.js
#   8080 /test`), which the workspace-tests gate starts.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
FIXTURE="integration_resources/send_namelist_over_http/send_namelist_over_http.scxml"
GENERATED_DIR="backends/rust/tests/src/integration/send_namelist_over_http"
STEM="send_namelist_over_http"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l rust -o "$TMP/"

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.rs' -delete
cp "$TMP"/*.rs "$GENERATED_DIR/"

# Rebuild mod.rs from the actual `_sm.rs` filenames the regen produced.
# Sorted output keeps the diff stable across regens.
MODRS="$GENERATED_DIR/mod.rs"
{
    echo "// GENERATED -- DO NOT EDIT (scripts/regen_send_namelist_over_http.sh)"
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
