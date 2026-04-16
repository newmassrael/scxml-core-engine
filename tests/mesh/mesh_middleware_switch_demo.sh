#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# SCE Mesh Phase 3 entry #5 — deploy.yaml-only middleware switch demo.
#
# Proves the central Phase 3 claim: the same SCXML compiles against any
# supported transport with zero source changes — only the deploy.yaml
# differs. Runs sce-codegen against one SCXML and N deploy.yaml variants,
# asserts:
#
#   (1) Every variant succeeds (exit 0).
#   (2) Every variant emits <machine>_transport.h.
#   (3) Every variant's generated header contains the expected transport
#       marker (ensures variant A's output did not silently reuse variant
#       B's include/field layout — a genuine dispatch happened).
#
# Usage:
#   mesh_middleware_switch_demo.sh <sce-codegen> <scxml> <out-dir> \
#       <label>:<deploy.yaml>:<marker> [<label>:<deploy.yaml>:<marker> ...]
#
# The marker is a substring that must appear in the generated header for
# that transport — for example, `vsomeip::application` for someip or
# `zenoh::Session` for zenoh. Pick markers that unambiguously identify
# the transport's code path.

set -u

if [[ $# -lt 4 ]]; then
    echo "usage: $0 <sce-codegen> <scxml> <out-dir> <label:deploy:marker> [...]" >&2
    exit 2
fi

SCE_CODEGEN=$1
SCXML=$2
OUT_ROOT=$3
shift 3

FAILED=0
TOTAL=0
MACHINE=$(basename "$SCXML" .scxml)

for spec in "$@"; do
    TOTAL=$((TOTAL + 1))
    IFS=':' read -r label deploy marker <<< "$spec"
    if [[ -z "$label" || -z "$deploy" || -z "$marker" ]]; then
        echo "FAIL: malformed spec '$spec' (expected label:deploy:marker)" >&2
        FAILED=$((FAILED + 1))
        continue
    fi

    out="$OUT_ROOT/$label"
    rm -rf "$out" && mkdir -p "$out"

    log="$out/codegen.log"
    if ! "$SCE_CODEGEN" generate "$SCXML" -l cpp -o "$out" --deploy "$deploy" > "$log" 2>&1; then
        echo "FAIL: $label — codegen exited non-zero" >&2
        cat "$log" >&2
        FAILED=$((FAILED + 1))
        continue
    fi

    header="$out/${MACHINE}_transport.h"
    if [[ ! -f "$header" ]]; then
        echo "FAIL: $label — expected $header but it was not generated" >&2
        ls "$out" >&2
        FAILED=$((FAILED + 1))
        continue
    fi

    if ! grep -q -- "$marker" "$header"; then
        echo "FAIL: $label — header does not contain transport marker '$marker'" >&2
        echo "--- header head ---" >&2
        head -40 "$header" >&2
        FAILED=$((FAILED + 1))
        continue
    fi

    echo "PASS: $label (marker: $marker)"
done

if [[ $FAILED -gt 0 ]]; then
    echo "middleware switch demo: $FAILED/$TOTAL variants failed" >&2
    exit 1
fi

echo "middleware switch demo: $TOTAL/$TOTAL transports generated from one SCXML, zero source changes"
exit 0
