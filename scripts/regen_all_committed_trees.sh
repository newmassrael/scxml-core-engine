#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Master regeneration for every committed §6.2.6 generated tree
# tracked by the SCE workspace. Runs in three groups:
#
#   1. W3C IRP committed trees (Rust + Kotlin)
#   2. Integration committed trees (Rust + Kotlin + Go) via the
#      new `generate-integration` CLI surface (RFC
#      `claudedocs/rfc-donedata-5-backend-layout.md` Q-6 + Q-7).
#   3. Forge round-trip Go codec tree.
#
# Build-time backends (cpp / c11 / Python) are intentionally absent
# — their generated trees materialise at CMake / CI time without a
# committed §6.2.6 header to refresh.
#
# Use when a `tools/codegen/templates/` edit or a `Cargo.lock` bump
# invalidates the embedded `template-hash` on every committed tree
# (Q-§6.2.6-3 lock-in). Running this script + `git add -A` produces
# a single coherent commit that re-greens the drift-verify gate
# across every tracked context in one shot.
#
# Usage (from repo root):
#   scripts/regen_all_committed_trees.sh

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

CODEGEN="target/release/sce-codegen"

if [[ ! -x "$CODEGEN" ]]; then
    cargo build --bin sce-codegen --features cli --release -p sce-build
fi

echo "==> W3C committed trees"
"$CODEGEN" generate-w3c -l rust
"$CODEGEN" generate-w3c -l kotlin

echo "==> Integration trees (Rust / Kotlin / Go committed; Python gitignored)"
"$CODEGEN" generate-integration -l rust
"$CODEGEN" generate-integration -l kotlin
"$CODEGEN" generate-integration -l go
"$CODEGEN" generate-integration -l python

echo "==> Forge round-trip Go codec tree"
sce-forge-runtime/go/round_trip/generate.sh

echo "All committed §6.2.6 trees regenerated."
