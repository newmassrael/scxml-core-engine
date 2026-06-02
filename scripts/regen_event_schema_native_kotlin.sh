#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate sce-kotlin-tests/src/main/kotlin/com/sce/integration/statechart_minimal/
# from the canonical EventSchema native-lowering fixture at
# sce-build/tests/fixtures/event_schema/statechart_minimal.scxml.
#
# This is the Kotlin compile+run gate for NL→IR Item C1 Path A (EventSchema
# native lowering) — the Kotlin twin of the Rust `tests/event_schema_native.rs`,
# the Go `event_schema_native` package, and the C11
# `c11_integration_event_schema_native` tests. It reuses the canonical fixture
# directly (rather than a copy under integration_resources/) so the receive-side
# schema sibling `schema_job_completed_minimal.scxml` resolves next to it — the
# same choice the Rust and Go gates make to avoid a third fixture copy.
#
# The committed SM compiles as part of `:sce-kotlin-tests` in the JVM build, so
# the generated payload data class, the type-erased `EventMetadata.typedPayload`
# carrier round-trip, and the per-event `raiseJobCompleted` inject seam are
# really type-checked; the hand-authored `EventSchemaNativeTest.kt` drives the
# native typed guard. The guard `cond="_event.data.elapsed_ms === 0"` lowers to
# a `pendingJobCompletedPayload != null && (…)` field comparison with NO script
# engine, so the machine is constructed WITHOUT a `ScxmlScriptEngine` (the
# MCU-relevant property: a typed-guard machine needs no JS/Lua engine).
#
# The generated tree lives under `com/sce/integration/` instead of
# `com/sce/generated/` so the W3C IRP and integration package roots stay
# disjoint. `--kotlin-package-prefix com.sce.integration` flips the `package`
# header on every emitted file to match.
#
# Usage (from repo root):
#   scripts/regen_event_schema_native_kotlin.sh
#
# Requires:
#   target/release/sce-codegen (auto-built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

CODEGEN="target/release/sce-codegen"
FIXTURE="sce-build/tests/fixtures/event_schema/statechart_minimal.scxml"
INPUT_ROOT="sce-build/tests/fixtures/event_schema"
GENERATED_DIR="sce-kotlin-tests/src/main/kotlin/com/sce/integration/statechart_minimal"
PACKAGE_PREFIX="com.sce.integration"

if [[ ! -x "$CODEGEN" ]]; then
    cargo build --bin sce-codegen --features cli --release -p sce-build
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l kotlin -o "$TMP/" \
    --input-root "$INPUT_ROOT" \
    --kotlin-package-prefix "$PACKAGE_PREFIX"

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*Sm.kt' -delete
for src in "$TMP"/*Sm.kt; do
    [[ -f "$src" ]] || continue
    sed -i "s|// Source: ${TMP}/|// Source: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*Sm.kt "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE"
