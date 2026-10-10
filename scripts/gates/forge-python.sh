#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: forge-conformance.yml
#
# Python arm of the forge conformance suite (forge-conformance.yml).
#
# No build step, so this is the cheapest of the four language arms.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# The generated cases in numerical_reference.json are the model's output
# (tests/forge/conformance/gen_cases.py, E11): a file the generator would not
# write is either hand-edited in a generated line or stale against the model,
# and every arm below would then be held to cases nobody can reproduce. Run
# here because this arm has no build step and Python is all it needs.
python3 tests/forge/conformance/gen_cases.py --check \
    || sce_gate_fail "generated conformance cases are stale — run tests/forge/conformance/gen_cases.py"

( cd backends/python/forge-runtime && PYTHONPATH=tests python3 -m unittest \
    tests.test_numerical_conformance tests.test_default_round_trip ) \
    || sce_gate_fail "Python forge conformance"

# The queue kind's Python arm (SCE Protocol-Synthesis RFC §synth-5-P): the
# contract scenarios and the runtime's properties, the modules the generator
# writes for a queue, and the stress runs, which write their histories into
# the directory SCE_QUEUE_HISTORY_DIR names. The directory is under `target/`
# and emptied first: a file left from an earlier run would be judged in place
# of one this run did not write.
source "$SCE_REPO_ROOT/scripts/lib/sce_codegen.sh"
QUEUE_HISTORIES="$SCE_REPO_ROOT/target/queue-histories/python"
rm -rf "$QUEUE_HISTORIES"
mkdir -p "$QUEUE_HISTORIES"
( cd backends/python/forge-runtime && SCE_QUEUE_HISTORY_DIR="$QUEUE_HISTORIES" \
    PYTHONPATH=tests python3 -m unittest \
    tests.test_queue_runtime tests.test_queue_generated tests.test_queue_history ) \
    || sce_gate_fail "Python forge queue"
# 4 capacities recorded 3 times with one producer and one consumer, 6 shapes
# recorded 25 times and 5 segmented shapes recorded 25 times. A run that
# recorded fewer would pass the judgement below, so the count is held.
# Linearizability is judged by the command every backend's histories are judged
# by, not by a checker of the arm's own.
shopt -s nullglob
queue_histories=("$QUEUE_HISTORIES"/*.json)
shopt -u nullglob
(( ${#queue_histories[@]} >= 287 )) \
    || sce_gate_fail "Python queue histories: ${#queue_histories[@]} written, expected at least 287"
"$(sce_codegen_require "$SCE_REPO_ROOT")" check-queue-history "${queue_histories[@]}" \
    || sce_gate_fail "Python queue histories are not linearizable"
