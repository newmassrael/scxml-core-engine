#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/kotlin/tests/src/main/kotlin/com/sce/integration/invoke_unsupported_type/
# from the canonical fixture at
# integration_resources/invoke_unsupported_type/invoke_unsupported_type.scxml.
#
# Mirrors scripts/regen_invoke_unsupported_type.sh (Rust), including its
# lack of a synth-invoke child loop: the fixture's `<invoke>` names a
# `type` no processor implements, so W3C SCXML 6.4.1 classification
# completes before any child document would be resolved and codegen emits
# no child. Nothing lands adjacent to the canonical fixture, so the
# TMP-staging dance the older integration regen scripts perform for
# SCE Mesh §9.6.6 rule 1 children has nothing to protect against here.
#
# The generated tree lives under `com/sce/integration/` instead of
# `com/sce/generated/` so the W3C IRP and integration package roots
# stay disjoint. `--kotlin-package-prefix com.sce.integration` flips
# the `package` header on the emitted file to match.
#
# Usage (from repo root):
#   scripts/regen_invoke_unsupported_type_kotlin.sh
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
GENERATED_DIR="backends/kotlin/tests/src/main/kotlin/com/sce/integration/invoke_unsupported_type"
PACKAGE_PREFIX="com.sce.integration"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l kotlin -o "$TMP/" \
    --kotlin-package-prefix "$PACKAGE_PREFIX"

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*Sm.kt' -delete
cp "$TMP"/*Sm.kt "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
