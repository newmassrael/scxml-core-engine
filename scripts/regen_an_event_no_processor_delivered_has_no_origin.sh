#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/rust/tests/src/integration/an_event_no_processor_delivered_has_no_origin/
# from the canonical fixture at
# integration_resources/an_event_no_processor_delivered_has_no_origin/an_event_no_processor_delivered_has_no_origin.scxml.
#
# The document sends `host.ask` to a processor its HOST serves, so it is
# generated with `--host-processor x-sce-host`, the type the driver
# (backends/rust/tests/tests/an_event_no_processor_delivered_has_no_origin.rs)
# registers. Without it the send compiles to error.execution (W3C SCXML 6.2)
# and the reply the fixture asks about is never raised.
#
# Pipeline: sce-codegen generate <fixture> -o $TMP → clear stale `_sm.rs` in
# the integration tree → copy the new ones in → stitch mod.rs → cargo fmt
# (pre-commit hook gate).
#
# Usage (from repo root):
#   scripts/regen_an_event_no_processor_delivered_has_no_origin.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
FIXTURE="integration_resources/an_event_no_processor_delivered_has_no_origin/an_event_no_processor_delivered_has_no_origin.scxml"
GENERATED_DIR="backends/rust/tests/src/integration/an_event_no_processor_delivered_has_no_origin"
STEM="an_event_no_processor_delivered_has_no_origin"
HOST_PROCESSOR="x-sce-host"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l rust -o "$TMP/" --host-processor "$HOST_PROCESSOR"

mkdir -p "$GENERATED_DIR"
source "$REPO_ROOT/scripts/lib/sce_generated_tree.sh"
sce_generated_tree_clear "$GENERATED_DIR"
cp "$TMP"/*.rs "$GENERATED_DIR/"

MODRS="$GENERATED_DIR/mod.rs"
{
    echo "// GENERATED -- DO NOT EDIT (scripts/regen_an_event_no_processor_delivered_has_no_origin.sh)"
    echo ""
    parent_stem="${STEM}_sm"
    echo "mod ${parent_stem};"
    echo "pub use ${parent_stem}::*;"
} > "$MODRS"

source "$REPO_ROOT/scripts/lib/sce_rustfmt.sh"
sce_rustfmt_dir "$GENERATED_DIR" "$REPO_ROOT"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE (--host-processor $HOST_PROCESSOR)"
