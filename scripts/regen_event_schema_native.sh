#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/rust/tests/src/integration/event_schema_native/ from the
# canonical EventSchema native-lowering fixture at
# sce-build/tests/fixtures/event_schema/statechart_minimal.scxml.
#
# This is the Rust compile+run gate for NL→IR Item C1 Path A (EventSchema
# MCU native lowering, RFC §10.4 step 5) — the twin of the C11 integration
# test `c11_integration_event_schema_native`. It is a Rust-ONLY committed
# tree (NOT driven by `generate-integration`, which fans out to
# kotlin/go/python): those backends still fail-fast on a typed `_event.data`
# guard, so the fixture cannot live under the shared `integration_resources/`
# tree until they grow native lowering of their own.
#
# `syn::parse_file` in the smoke gate is syntax-only and cannot catch a
# Rust semantic error such as the orphan rule (an inherent `impl Engine<P>`
# is E0116); this committed tree compiles as part of
# `cargo test -p sce-rust-tests`, so the per-event inject extension trait is
# really type-checked, and the runtime test (`tests/event_schema_native.rs`)
# drives the typed guard.
#
# Usage (from repo root):
#   scripts/regen_event_schema_native.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
FIXTURE="sce-build/tests/fixtures/event_schema/statechart_minimal.scxml"
GENERATED_DIR="backends/rust/tests/src/integration/event_schema_native"

# The bytes fixture (RFC rfc-eventschema-bytes-guard.md §bytesguard-6) rides the same
# committed-tree gate so the bytes-equality guard is REALLY compiled + run,
# not only form-asserted in the sce-build smoke layer.
BYTES_FIXTURE="sce-build/tests/fixtures/event_schema/statechart_bytes.scxml"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l rust -o "$TMP/"
"$CODEGEN" generate "$BYTES_FIXTURE" -l rust -o "$TMP/"

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.rs' -delete
cp "$TMP"/*.rs "$GENERATED_DIR/"

MODRS="$GENERATED_DIR/mod.rs"
{
    echo "// GENERATED -- DO NOT EDIT (scripts/regen_event_schema_native.sh)"
    echo ""
    echo "mod statechart_minimal_sm;"
    echo "pub use statechart_minimal_sm::*;"
    echo ""
    echo "mod statechart_bytes_sm;"
    echo "pub use statechart_bytes_sm::*;"
} > "$MODRS"

source "$REPO_ROOT/scripts/lib/sce_rustfmt.sh"
sce_rustfmt_dir "$GENERATED_DIR" "$REPO_ROOT"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
