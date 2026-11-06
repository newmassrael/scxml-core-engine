#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/rust/tests/src/integration/a_host_names_its_origin_in_its_own_terms/
# from the canonical fixture at
# integration_resources/a_host_names_its_origin_in_its_own_terms/a_host_names_its_origin_in_its_own_terms.scxml.
#
# The document's `<invoke type="x-sce-host">` is run by its HOST, so it is
# generated with `--host-invoker x-sce-host`, the type the driver
# (backends/rust/tests/tests/a_host_names_its_origin_in_its_own_terms.rs)
# registers. Without it the invoke compiles to the refusal of an unsupported
# type (W3C SCXML 6.4.1) and no host is ever asked to run it.
#
# Pipeline: sce-codegen generate <fixture> -o $TMP → clear stale `_sm.rs` in
# the integration tree → copy the new ones in → stitch mod.rs → cargo fmt
# (pre-commit hook gate).
#
# Usage (from repo root):
#   scripts/regen_a_host_names_its_origin_in_its_own_terms.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
FIXTURE="integration_resources/a_host_names_its_origin_in_its_own_terms/a_host_names_its_origin_in_its_own_terms.scxml"
GENERATED_DIR="backends/rust/tests/src/integration/a_host_names_its_origin_in_its_own_terms"
STEM="a_host_names_its_origin_in_its_own_terms"
HOST_INVOKER="x-sce-host"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l rust -o "$TMP/" --host-invoker "$HOST_INVOKER"

mkdir -p "$GENERATED_DIR"
source "$REPO_ROOT/scripts/lib/sce_generated_tree.sh"
sce_generated_tree_clear "$GENERATED_DIR"
cp "$TMP"/*.rs "$GENERATED_DIR/"

MODRS="$GENERATED_DIR/mod.rs"
{
    echo "// GENERATED -- DO NOT EDIT (scripts/regen_a_host_names_its_origin_in_its_own_terms.sh)"
    echo ""
    parent_stem="${STEM}_sm"
    echo "mod ${parent_stem};"
    echo "pub use ${parent_stem}::*;"
} > "$MODRS"

source "$REPO_ROOT/scripts/lib/sce_rustfmt.sh"
sce_rustfmt_dir "$GENERATED_DIR" "$REPO_ROOT"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE (--host-invoker $HOST_INVOKER)"
