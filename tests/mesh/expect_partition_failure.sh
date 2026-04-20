#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Negative-path wrapper for SCE_MESH.md §14 partition validation. The
# deploy.rs (rules 6, 7, 8, 9, 10) and partitions.rs (rules 1, 2, 11)
# unit suites already prove each rule's Rust-level semantics; this
# wrapper lifts that coverage to the CLI + filesystem layer so a future
# regression that silently skips a validator — or routes the diagnostic
# through the wrong exit path — shows up at ctest time.
#
# Runs sce-codegen with `--error-format json` and asserts three
# invariants simultaneously:
#
#   1. Non-zero exit (codegen refused to emit).
#   2. NDJSON stderr contains "code":"<expected-code>" — pinpoints
#      the specific validator instead of accepting any non-zero exit.
#   3. No `*_transport.*` file was written to the output directory.
#      The transport header is the artefact that deploy.yaml validation
#      gates; its appearance would mean the validator fired but codegen
#      still flushed its output. Rules 1/2/11 fire AFTER the state-
#      machine header (sm.h/sm.inl) is emitted — the sm.h is allowed
#      because it depends on the positional SCXML, not on deploy.yaml.
#      Only the transport file is gated by mesh validation, so only the
#      transport file is a red flag.
#
# The wrapper intentionally does NOT share source with
# expect_coverage_failure.sh / expect_pattern_failure.sh: those wrappers
# grep free-form diagnostic prose (text content, not codes) so their
# single-source-of-truth is the message string. This wrapper grep's
# the structured `code` field, which is authoritative in
# sce-build/src/forge/diagnostic.rs `as_str` and wire-protected by
# schemas/sce-diagnostic.v1.schema.json.
#
# Usage:
#   expect_partition_failure.sh \
#       <sce-codegen-path> <scxml> <deploy.yaml> <out-dir> <expected-code>

set -u

if [[ $# -ne 5 ]]; then
    echo "usage: $0 <sce-codegen> <scxml> <deploy.yaml> <out-dir> <expected-code>" >&2
    exit 2
fi

SCE_CODEGEN=$1
SCXML=$2
DEPLOY=$3
OUT_DIR=$4
EXPECTED_CODE=$5

rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR"

OUTPUT=$("$SCE_CODEGEN" --error-format json generate \
    "$SCXML" -l cpp -o "$OUT_DIR" --deploy "$DEPLOY" 2>&1)
EXIT_CODE=$?

if [[ $EXIT_CODE -eq 0 ]]; then
    echo "FAIL: expected non-zero exit for code '$EXPECTED_CODE' but got 0" >&2
    echo "--- codegen output ---" >&2
    echo "$OUTPUT" >&2
    exit 1
fi

if ! grep -q "\"code\":\"${EXPECTED_CODE}\"" <<< "$OUTPUT"; then
    echo "FAIL: non-zero exit ($EXIT_CODE) but \"code\":\"${EXPECTED_CODE}\" not in NDJSON output" >&2
    echo "--- codegen output ---" >&2
    echo "$OUTPUT" >&2
    exit 1
fi

# A mesh validator that fires after the transport header is flushed
# would show up as `<stem>_transport.<ext>` on disk. Glob-match so the
# wrapper stays parametric across SCXML stems; sm.h / sm.inl emitted
# before the mesh pipeline runs are allowed by design.
TRANSPORT_HITS=$(find "$OUT_DIR" -type f -name '*_transport.*' 2>/dev/null)
if [[ -n "$TRANSPORT_HITS" ]]; then
    echo "FAIL: validator fired ('$EXPECTED_CODE') but transport file(s) were still written:" >&2
    echo "$TRANSPORT_HITS" >&2
    exit 1
fi

exit 0
