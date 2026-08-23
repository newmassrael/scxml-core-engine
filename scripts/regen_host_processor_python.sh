#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate backends/python/tests/integration/host_processor/statechart_host_processor_sm.py
# from the canonical W3C SCXML 6.2.5 host-served-processor fixture at
# sce-build/tests/fixtures/host_processor/statechart_host_processor.scxml.
#
# This is the Python compile+run gate for a `<send type>` the HOST serves —
# the Python twin of the Rust `tests/host_processor.rs`, the Go
# `statechart_host_processor` package, the C++ `HostProcessorAotTest` and the
# C11 `c11_integration_host_processor` runner. The `--host-processor`
# declaration below is the whole point: the same fixture generated WITHOUT it
# compiles to a W3C SCXML 6.2 error.execution, and with it compiles to a
# dispatch into the registry the host registers a handler in.
#
# A per-stem script rather than `generate-integration -l python`, for the
# reason `regen_event_schema_native_python.sh` gives: the fan-out enumerates
# `integration_resources/` and has no per-stem flags, and this fixture is
# deliberately not a stem there — a stem is a seven-channel commitment and two
# backends still refuse the declaration by name. The gate calls this script
# explicitly for the same reason it calls that one.
#
# Usage (from repo root):
#   scripts/regen_host_processor_python.sh
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
GENERATED_DIR="backends/python/tests/integration/host_processor"

# Kept in one place here and asserted from the runtime test, so the string the
# build was given and the string the host registers cannot drift into two.
HOST_PROCESSOR="x-sce-host"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$CODEGEN" generate "$FIXTURE" -l python -o "$TMP/" \
    --input-root "$INPUT_ROOT" --host-processor "$HOST_PROCESSOR"

# The delay side of the same surface (W3C SCXML 6.2.4 + 6.3). A third document
# rather than a `delay` added to the one above, because that one is driven with
# `step()` and this one can only be driven with `advance_time()`: folding them
# together would put a clock under every assertion the first one makes, and a
# fixture measuring two things cannot say which of them broke.
DELAYED_FIXTURE="sce-build/tests/fixtures/host_processor/statechart_delayed_host_send.scxml"
"$CODEGEN" generate "$DELAYED_FIXTURE" -l python -o "$TMP/" \
    --input-root "$INPUT_ROOT" --host-processor "$HOST_PROCESSOR"

mkdir -p "$GENERATED_DIR"
find "$GENERATED_DIR" -maxdepth 1 -name '*_sm.py' -delete
for src in "$TMP"/*_sm.py; do
    [[ -f "$src" ]] || continue
    sed -i "s|# From: ${TMP}/|# From: ${INPUT_ROOT}/|g" "$src"
done
cp "$TMP"/*_sm.py "$GENERATED_DIR/"

echo "Regenerated: $GENERATED_DIR/ from"
echo "  $FIXTURE (--host-processor $HOST_PROCESSOR)"
echo "  $DELAYED_FIXTURE (--host-processor $HOST_PROCESSOR)"
