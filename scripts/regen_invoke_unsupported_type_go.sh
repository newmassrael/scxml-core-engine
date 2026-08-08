#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/go/tests/integration/invoke_unsupported_type/invoke_unsupported_type_sm.go
# from the canonical fixture at
# integration_resources/invoke_unsupported_type/invoke_unsupported_type.scxml.
#
# Mirrors scripts/regen_invoke_unsupported_type.sh (Rust), including its
# lack of a synth-invoke child loop: the fixture's `<invoke>` names a
# `type` no processor implements, so W3C SCXML 6.4.1 classification
# completes before any child document would be resolved and codegen emits
# no child.
#
# Only `*_sm.go` is copied back, so the hand-authored
# `invoke_unsupported_type_test.go` next to the generated file is never
# touched.
#
# The generated tree lives under `backends/go/tests/integration/<stem>/`
# rather than `backends/go/tests/<stem>/` so the W3C IRP and integration
# trees stay disjoint at the directory level.
#
# Usage (from repo root):
#   scripts/regen_invoke_unsupported_type_go.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).
#
# Idempotency: re-runs are byte-stable except for the embedded
# `generated-at: <unix-seconds>` header line that the codegen emits
# on every invocation. `source-hash` and `template-hash` stay
# deterministic for unchanged fixture + template inputs.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

FIXTURE="integration_resources/invoke_unsupported_type/invoke_unsupported_type.scxml"
GENERATED_DIR="backends/go/tests/integration/invoke_unsupported_type"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l go -o "$TMP/"

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.go' -delete
cp "$TMP"/*_sm.go "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
