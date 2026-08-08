#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/python/tests/integration/invoke_unsupported_type/*_sm.py
# from the canonical fixture at
# integration_resources/invoke_unsupported_type/invoke_unsupported_type.scxml.
#
# Mirrors scripts/regen_invoke_unsupported_type.sh (Rust), including its
# lack of a synth-invoke child loop: the fixture's `<invoke>` names a
# `type` no processor implements, so W3C SCXML 6.4.1 classification
# completes before any child document would be resolved and codegen emits
# no child.
#
# Unlike the Rust / Kotlin / Go regen scripts, the Python generated
# tree is `.gitignored` (mirroring the W3C IRP Python pattern under
# `backends/python/tests/generated/`) — CI runs this script before pytest
# so the committed source tree never carries the SCE-GENERATED Python
# files.
#
# Usage (from repo root):
#   scripts/regen_invoke_unsupported_type_python.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

FIXTURE="integration_resources/invoke_unsupported_type/invoke_unsupported_type.scxml"
GENERATED_DIR="backends/python/tests/integration/invoke_unsupported_type"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l python -o "$TMP/"

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.py' -delete
cp "$TMP"/*_sm.py "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
