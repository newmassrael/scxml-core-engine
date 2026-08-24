#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/go/tests/integration/parallel_region_root_external_domain/
# from tests/integration/parallel_region_root_external_domain.scxml.
#
# The Go half of the transition-domain witness; the Rust half is
# `scripts/regen_parallel_region_root_external_domain.sh` and the two C++
# drivers build from the same document through CMake.
#
# The source is NOT under `integration_resources/`, and that is deliberate.
# A stem there is a seven-channel contract that `integration_stem_registration.rs`
# enforces, and the Python and C11 engines still resolve a region root's
# external transition to the enclosing `<parallel>` rather than to the document
# root. Registering the contract before those engines meet it would claim
# coverage this repository does not have. The document lives beside its first
# driver until they are repaired.
#
# No `--input-root` and no tmp staging of the fixture: this document declares
# no `<invoke>`, so codegen emits exactly one `_sm.go` and sets the drift
# header's source path from the canonical input path directly.
#
# Usage (from repo root):
#   scripts/regen_parallel_region_root_external_domain_go.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
FIXTURE="tests/integration/parallel_region_root_external_domain.scxml"
GENERATED_DIR="backends/go/tests/integration/parallel_region_root_external_domain"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l go -o "$TMP/"

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.go' -delete
cp "$TMP"/*.go "$GENERATED_DIR/"

gofmt -w "$GENERATED_DIR"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
