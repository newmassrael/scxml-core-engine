#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# Negative-path wrapper for mesh pattern capability validation.
# Runs sce-codegen and asserts two invariants simultaneously:
#
#   1. Exit code is non-zero (codegen refused to emit).
#   2. Output contains the "pattern capability violations" diagnostic.
#
# Without (2), a future regression where codegen happens to exit non-zero
# for an unrelated reason would masquerade as a passing negative test.
# The wrapper enforces that the *specific* pattern-validation code path fires.
#
# Usage:
#   expect_pattern_failure.sh <sce-codegen-path> <scxml> <deploy> <out-dir>

set -u

if [[ $# -ne 4 ]]; then
    echo "usage: $0 <sce-codegen> <scxml> <deploy.yaml> <out-dir>" >&2
    exit 2
fi

SCE_CODEGEN=$1
SCXML=$2
DEPLOY=$3
OUT_DIR=$4

mkdir -p "$OUT_DIR"

OUTPUT=$("$SCE_CODEGEN" generate "$SCXML" -l cpp -o "$OUT_DIR" --deploy "$DEPLOY" 2>&1)
EXIT_CODE=$?

if [[ $EXIT_CODE -eq 0 ]]; then
    echo "FAIL: expected non-zero exit but got 0" >&2
    echo "--- codegen output ---" >&2
    echo "$OUTPUT" >&2
    exit 1
fi

if ! grep -q "pattern capability violations" <<< "$OUTPUT"; then
    echo "FAIL: non-zero exit ($EXIT_CODE) but 'pattern capability violations' not in output" >&2
    echo "--- codegen output ---" >&2
    echo "$OUTPUT" >&2
    exit 1
fi

# Guard against the transport file being emitted anyway (silent drop
# of the validator would show as brake_transport.h appearing on disk).
if [[ -f "$OUT_DIR/brake_transport.h" ]]; then
    echo "FAIL: validator fired but transport file was still written to disk" >&2
    exit 1
fi

exit 0
