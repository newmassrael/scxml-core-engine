#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/go/tests/integration/event_schema_native/statechart_minimal_sm.go
# from the canonical EventSchema native-lowering fixture at
# sce-build/tests/fixtures/event_schema/statechart_minimal.scxml.
#
# This is the Go compile+run gate for NL→IR Item C1 Path A (EventSchema
# native lowering) — the Go twin of the Rust `tests/event_schema_native.rs`
# and the C11 `c11_integration_event_schema_native` tests. It reuses the
# canonical fixture directly (rather than a copy under
# integration_resources/) so the receive-side schema sibling
# `schema_job_completed_minimal.scxml` resolves next to it — the same
# choice the Rust gate makes to avoid a third fixture copy.
#
# The committed SM compiles as part of `go test ./...` in sce-go-tests, so
# the generated payload struct, the type-erased `TypedPayload` carrier
# round-trip, and the per-event `RaiseJobCompleted` inject seam are really
# type-checked; the hand-authored `event_schema_native_test.go` drives the
# native typed guard. The guard `cond="_event.data.elapsed_ms === 0"` lowers
# to a tag-checked field comparison with NO script engine, so the policy is
# constructed without a `ScriptEngine` (the MCU-relevant property).
#
# Usage (from repo root):
#   scripts/regen_event_schema_native_go.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
FIXTURE="sce-build/tests/fixtures/event_schema/statechart_minimal.scxml"
INPUT_ROOT="sce-build/tests/fixtures/event_schema"
GENERATED_DIR="backends/go/tests/integration/event_schema_native"

# The bytes fixture (RFC rfc-eventschema-bytes-guard.md §bytesguard-6) lowers to a Go
# `string(p.pending….raw) == "ack"` guard (slice `==` is illegal in Go, so
# the conversion is the whole point). Its machine name differs, so the
# generated `package statechart_bytes` lives in its OWN directory (Go allows
# one package per dir) and rides the same compile+run gate.
BYTES_FIXTURE="sce-build/tests/fixtures/event_schema/statechart_bytes.scxml"
BYTES_DIR="backends/go/tests/integration/event_schema_bytes"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l go -o "$TMP/minimal/" --input-root "$INPUT_ROOT"
"$CODEGEN" generate "$BYTES_FIXTURE" -l go -o "$TMP/bytes/" --input-root "$INPUT_ROOT"

mkdir -p "$GENERATED_DIR" "$BYTES_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.go' -delete
find "$BYTES_DIR" -maxdepth 1 -name '*_sm.go' -delete
for src in "$TMP"/minimal/*_sm.go; do
    [[ -f "$src" ]] || continue
    sed -i "s|// From: ${TMP}/minimal/|// From: ${INPUT_ROOT}/|g" "$src"
    cp "$src" "$GENERATED_DIR/"
done
for src in "$TMP"/bytes/*_sm.go; do
    [[ -f "$src" ]] || continue
    sed -i "s|// From: ${TMP}/bytes/|// From: ${INPUT_ROOT}/|g" "$src"
    cp "$src" "$BYTES_DIR/"
done

echo "Regenerated: $GENERATED_DIR/ + $BYTES_DIR/ from $FIXTURE + $BYTES_FIXTURE"
