#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate sce-python-tests/integration/event_schema_native/statechart_minimal_sm.py
# from the canonical EventSchema native-lowering fixture at
# sce-build/tests/fixtures/event_schema/statechart_minimal.scxml.
#
# This is the Python compile+run gate for NL→IR Item C1 Path A (EventSchema
# native lowering) — the Python twin of the Rust `tests/event_schema_native.rs`,
# the Go `event_schema_native` package, the Kotlin `EventSchemaNativeTest`, and
# the C11 `c11_integration_event_schema_native` tests. It reuses the canonical
# fixture directly (rather than a copy under integration_resources/) so the
# receive-side schema sibling `schema_job_completed_minimal.scxml` resolves next
# to it — the same choice the Rust / Go / Kotlin gates make to avoid a third
# fixture copy.
#
# The committed SM is imported + run by `test_event_schema_native_aot.py`, so the
# generated payload dataclass, the type-erased `EventMetadata.typed_payload`
# carrier round-trip, and the per-event `raise_job_completed` inject seam are
# really exercised. The guard `cond="_event.data.elapsed_ms === 0"` lowers to a
# `self._pending_job_completed_payload is not None and (…)` comparison that never
# calls `self._guard(...)` — the test pins this with a script engine whose
# `evaluate_expression` raises (a native-lowered guard must not reach it).
#
# Usage (from repo root):
#   scripts/regen_event_schema_native_python.sh
#
# Requires:
#   target/release/sce-codegen (auto-built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

CODEGEN="target/release/sce-codegen"
FIXTURE="sce-build/tests/fixtures/event_schema/statechart_minimal.scxml"
INPUT_ROOT="sce-build/tests/fixtures/event_schema"
GENERATED_DIR="sce-python-tests/integration/event_schema_native"

if [[ ! -x "$CODEGEN" ]]; then
    cargo build --bin sce-codegen --features cli --release -p sce-build
fi

# The bytes fixture (RFC rfc-eventschema-bytes-guard.md §6) rides the same
# compile+run gate so the Python `bytes == b"ack"` guard is REALLY run — a
# `bytes == str` regression silently evaluates False and only a runtime
# transition check catches it.
BYTES_FIXTURE="sce-build/tests/fixtures/event_schema/statechart_bytes.scxml"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l python -o "$TMP/" --input-root "$INPUT_ROOT"
"$CODEGEN" generate "$BYTES_FIXTURE" -l python -o "$TMP/" --input-root "$INPUT_ROOT"

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.py' -delete
for src in "$TMP"/*_sm.py; do
    [[ -f "$src" ]] || continue
    sed -i "s|# From: ${TMP}/|# From: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*_sm.py "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
