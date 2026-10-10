#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: forge-conformance.yml
#
# Go arm of the forge conformance suite (forge-conformance.yml).
#
# generate.sh already runs `go build ./conformance/...` as a smoke check, so
# a codegen drift that produces uncompilable Go (the package-alias bug this
# gate was born to catch) surfaces right here.
#
# The regenerate is the expensive half; running the suite it just produced
# costs milliseconds. Building an artifact and then not looking at the
# result is how forge-conformance.yml came to be mirrored in name only.
#
# ./round_trip/ is the committed tree, orthogonal to the header-hash drift
# gate: that one checks parity, this checks the tree still functions.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

( cd backends/go/forge-runtime/conformance && bash generate.sh ) \
    || sce_gate_fail "Go forge conformance generate"
( cd backends/go/forge-runtime && go test ./conformance/ -count=1 ) \
    || sce_gate_fail "Go forge conformance"
( cd backends/go/forge-runtime && go test ./round_trip/ -count=1 ) \
    || sce_gate_fail "Go forge round-trip"

# The queue kind's Go arm (SCE Protocol-Synthesis RFC §synth-5-P). Go has no
# tool for the memory-model layer, so its evidence is the race detector over
# the runtime's tests (layer 5) and the histories its stress runs write
# (layer 2), which are judged below by the command every backend's histories
# are judged by, not by a checker of the arm's own.
#
# The histories go under `target/`, emptied first: a file left from an earlier
# run would be judged in place of one this run did not write.
source "$SCE_REPO_ROOT/scripts/lib/sce_codegen.sh"
QUEUE_HISTORIES="$SCE_REPO_ROOT/target/queue-histories/go"
rm -rf "$QUEUE_HISTORIES"
mkdir -p "$QUEUE_HISTORIES"
( cd backends/go/forge-runtime \
    && SCE_QUEUE_HISTORY_DIR="$QUEUE_HISTORIES" go test -race -count=1 ./queue/ ) \
    || sce_gate_fail "Go forge queue runtime under the race detector"
# 4 Lamport capacities recorded 3 times, 6 SCQ shapes recorded 25 times, and 5
# segmented shapes (the linked Lamport rings and four of the list of SCQ rings)
# recorded 25 times. A run that recorded fewer would pass the judgement below,
# so the count is held.
shopt -s nullglob
queue_histories=("$QUEUE_HISTORIES"/*.json)
shopt -u nullglob
(( ${#queue_histories[@]} >= 287 )) \
    || sce_gate_fail "Go queue histories: ${#queue_histories[@]} written, expected at least 287"
"$(sce_codegen_require "$SCE_REPO_ROOT")" check-queue-history "${queue_histories[@]}" \
    || sce_gate_fail "Go queue histories are not linearizable"
# The packages the generator writes for a queue, under the race detector too.
( cd backends/go/forge-runtime && go test -race -count=1 -run 'Lamport|Scq|Lscq' ./conformance/ ) \
    || sce_gate_fail "Go forge queue generated packages under the race detector"
