#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/kotlin/tests/src/main/kotlin/com/sce/integration/statechart_host_processor/
# from the canonical W3C SCXML 6.2.5 host-served-processor fixture at
# sce-build/tests/fixtures/host_processor/statechart_host_processor.scxml.
#
# This is the Kotlin compile+run gate for a `<send type>` the HOST serves —
# the Kotlin twin of the Rust `tests/host_processor.rs`, the Go
# `statechart_host_processor` package, the Python
# `test_host_processor_aot.py`, the C++ `HostProcessorAotTest` and the C11
# `c11_integration_host_processor` runner. The `--host-processor` declaration
# below is the whole point: the same fixture generated WITHOUT it compiles to
# a W3C SCXML 6.2 error.execution, and with it compiles to a dispatch into the
# registry the host registers a handler in.
#
# A per-stem script rather than the `generate-w3c` fan-out the Gradle build
# runs, for the reason `regen_event_schema_native_kotlin.sh` gives: that
# fan-out enumerates the W3C corpus and has no per-stem flags, and this
# fixture is deliberately not a stem under `integration_resources/` — a stem
# there is a seven-channel commitment.
#
# Usage (from repo root):
#   scripts/regen_host_processor_kotlin.sh
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
FIXTURE="sce-build/tests/fixtures/host_processor/statechart_host_processor.scxml"
INPUT_ROOT="sce-build/tests/fixtures/host_processor"
GENERATED_DIR="${SCE_KOTLIN_GENERATED_ROOT:-backends/kotlin/tests/src/main/kotlin}/com/sce/integration/statechart_host_processor"
PACKAGE_PREFIX="com.sce.integration"

# Kept in one place here and asserted from the runtime test, so the string the
# build was given and the string the host registers cannot drift into two.
HOST_PROCESSOR="x-sce-host"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l kotlin -o "$TMP/" \
    --input-root "$INPUT_ROOT" \
    --kotlin-package-prefix "$PACKAGE_PREFIX" \
    --host-processor "$HOST_PROCESSOR"

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*Sm.kt' -delete
for src in "$TMP"/*Sm.kt; do
    [[ -f "$src" ]] || continue
    sed -i "s|// Source: ${TMP}/|// Source: ${INPUT_ROOT}/|g" "$src"
    cp "$src" "$GENERATED_DIR/"
done

# The delay side of the same surface (W3C SCXML 6.2.4 + 6.3). A third document
# rather than a `delay` added to the one above, because that one is driven with
# `step()` and this one can only be driven with `tick()`: folding them together
# would put a clock under every assertion the first one makes, and a fixture
# measuring two things cannot say which of them broke. Its own directory
# because a Kotlin package is a directory, like the Go half of
# scripts/regen_host_processor.sh.
DELAYED_FIXTURE="sce-build/tests/fixtures/host_processor/statechart_delayed_host_send.scxml"
DELAYED_DIR="${SCE_KOTLIN_GENERATED_ROOT:-backends/kotlin/tests/src/main/kotlin}/com/sce/integration/statechart_delayed_host_send"
DELAYED_TMP="$(mktemp -d)"
trap 'rm -rf "$TMP" "$DELAYED_TMP"' EXIT

"$CODEGEN" generate "$DELAYED_FIXTURE" -l kotlin -o "$DELAYED_TMP/" \
    --input-root "$INPUT_ROOT" \
    --kotlin-package-prefix "$PACKAGE_PREFIX" \
    --host-processor "$HOST_PROCESSOR"

mkdir -p "$DELAYED_DIR"
find "$DELAYED_DIR" -maxdepth 1 -name '*Sm.kt' -delete
for src in "$DELAYED_TMP"/*Sm.kt; do
    [[ -f "$src" ]] || continue
    sed -i "s|// Source: ${DELAYED_TMP}/|// Source: ${INPUT_ROOT}/|g" "$src"
    cp "$src" "$DELAYED_DIR/"
done

echo "Regenerated: $GENERATED_DIR/ from $FIXTURE (--host-processor $HOST_PROCESSOR)"
echo "Regenerated: $DELAYED_DIR/ from $DELAYED_FIXTURE (--host-processor $HOST_PROCESSOR)"
