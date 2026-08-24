#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/python/tests/integration/native_action/ from the
# canonical W3C SCXML G.7 `<sce:action>` fixture at
# sce-build/tests/fixtures/event_schema/statechart_native_action.scxml.
#
# This is the Python half of the six channels that drive that document — the
# twin of the Rust `tests/native_action.rs`, the Go
# `statechart_native_action` package, the Kotlin `NativeActionTest`, the C++
# `NativeActionAotTest` and the C11 `c11_integration_native_action` runner.
#
# A separate script from `regen_native_action.sh`, and the reason is the same
# one `regen_host_processor_python.sh` gives: `backends/python/tests/
# integration/*/*_sm.py` is GITIGNORED, so there is no committed artefact for
# `regen_all_committed_trees.sh` to keep current. What produces the module is
# the `w3c-python` gate, which calls this script — its test file is tracked
# while its module is not, so a checkout that ran only `generate-integration`
# has a test importing a module nothing produced.
#
# Usage (from repo root):
#   scripts/regen_native_action_python.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
FIXTURE="sce-build/tests/fixtures/event_schema/statechart_native_action.scxml"
INPUT_ROOT="sce-build/tests/fixtures/event_schema"
GENERATED_DIR="backends/python/tests/integration/native_action"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l python -o "$TMP/" --input-root "$INPUT_ROOT"

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.py' -delete
for src in "$TMP"/*_sm.py; do
    [[ -f "$src" ]] || continue
    sed -i "s|# From: ${TMP}/|# From: ${INPUT_ROOT}/|g" "$src"
    cp "$src" "$GENERATED_DIR/"
done

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
