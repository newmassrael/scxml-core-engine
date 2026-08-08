#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/rust/tests/src/integration/invoke_unsupported_type/ from
# the canonical fixture at
# integration_resources/invoke_unsupported_type/invoke_unsupported_type.scxml.
#
# Pipeline:
#   sce-codegen generate <fixture> -o $TMP → clear stale `_sm.rs` in the
#   integration tree → copy the new one in → cargo fmt (pre-commit hook
#   gate) → stitch mod.rs.
#
# No synth-invoke child loop: the fixture's `<invoke>` names a `type` no
# processor implements, so W3C SCXML 6.4.1 classification completes before
# any child document would be resolved and codegen emits no child at all.
# The mod.rs stitch below therefore has a single member.
#
# No tmp staging of the fixture, no `// From:` sed rewrite, no
# `--input-root` override: codegen sets `model.scxml_source_path` directly
# from the canonical input path, so the embedded `// From:` and the
# §synth-6.2.6 `source-hash` already point at `integration_resources/...`.
#
# Usage (from repo root):
#   scripts/regen_invoke_unsupported_type.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
FIXTURE="integration_resources/invoke_unsupported_type/invoke_unsupported_type.scxml"
GENERATED_DIR="backends/rust/tests/src/integration/invoke_unsupported_type"
STEM="invoke_unsupported_type"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l rust -o "$TMP/"

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.rs' -delete
cp "$TMP"/*.rs "$GENERATED_DIR/"

MODRS="$GENERATED_DIR/mod.rs"
{
    echo "// GENERATED -- DO NOT EDIT (scripts/regen_invoke_unsupported_type.sh)"
    echo ""
    echo "mod ${STEM}_sm;"
    echo "pub use ${STEM}_sm::*;"
} > "$MODRS"

cargo fmt -p sce-rust-tests

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
