#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/rust/tests/src/integration/parallel_region_root_external_domain/
# from tests/integration/parallel_region_root_external_domain.scxml.
#
# The source is NOT under `integration_resources/`, and that is deliberate.
# A stem there is a seven-channel contract — C++ Interpreter, C++ AOT, Rust,
# Go, Python, Kotlin, C11 — that `integration_stem_registration.rs` enforces,
# and the Go engine still resolves a region root's external transition to the
# enclosing `<parallel>` rather than to the document root. Registering the
# contract before the engine meets it would claim coverage this repository
# does not have. The document lives beside its first driver until Go is
# repaired, on the same footing as `ai_loop`, whose machine is generated from
# `examples/`.
#
# The clause under test cannot be asked of `examples/ai_loop/ai_loop.scxml`
# either: that document is where the divergence was found, and it was repaired
# by spelling `type="internal"`, which is what its region-root transitions
# meant. The repair left no committed document reaching the external form.
#
# Usage (from repo root):
#   scripts/regen_parallel_region_root_external_domain.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
FIXTURE="tests/integration/parallel_region_root_external_domain.scxml"
GENERATED_DIR="backends/rust/tests/src/integration/parallel_region_root_external_domain"
STEM="parallel_region_root_external_domain"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l rust -o "$TMP/"

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.rs' -delete
cp "$TMP"/*.rs "$GENERATED_DIR/"

MODRS="$GENERATED_DIR/mod.rs"
{
    echo "// GENERATED -- DO NOT EDIT (scripts/regen_parallel_region_root_external_domain.sh)"
    echo ""
    echo "mod ${STEM}_sm;"
    echo "pub use ${STEM}_sm::*;"
} > "$MODRS"

source "$REPO_ROOT/scripts/lib/sce_rustfmt.sh"
sce_rustfmt_dir "$GENERATED_DIR" "$REPO_ROOT"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
