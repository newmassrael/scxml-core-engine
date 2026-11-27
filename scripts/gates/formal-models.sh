#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: formal-models.yml
#
# The TLA+ models of docs/formal under TLC, as docs/formal/models.json declares
# them (docs/adr/0115, D5): every configuration explored whole, its count of
# distinct states as recorded, and every part of a protocol taken out in turn,
# where TLC must find the violation the manifest names.
#
# The judge is `sce-codegen formal models` (docs/adr/0146), the command a
# consumer runs on its own manifest; this repository runs it the same way, and
# also on the fixture shaped like a consumer
# (sce-build/tests/fixtures/formal_consumer), so a path the judge assumes of this
# repository fails here first (D6). The judge's own checks are its unit tests,
# which the Rust lanes run.
#
# Runs in the host image. The pinned Java runtime, TLC and Apalache are fetched
# by `sce-codegen formal install` below and admitted by their digests, or found
# in the image's home; the judge itself never fetches, and never runs the JDK the
# image carries for the Kotlin lanes (Apalache is built for Java 21). Apalache
# checks the inductive invariants a manifest states (docs/adr/0115, D3), and
# gives TLC the standard module `Apalache` a model extends. Without the pinned
# runtime, TLC or Apalache the judge exits 15 (`formal/tool-unavailable`), which
# is "could not run" and no verdict about the models.
#
# The first part also judges docs/formal/quantities.json with
# `sce-codegen formal quantities` (docs/adr/0119, D4): each JANI model of a
# document, the second encoding Storm reads, held to the TLA+ module of the same
# document, its state graph to TLC's and every recorded run of the module to a
# path of it. Storm is installed as the others are; the system libraries its
# package asks for are the host image's.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# `target/debug/sce-codegen` is built by `codegen-build`, which the registry
# names as this gate's dependency.
# In parts (ADR 0120, D3): when SCE_FORMAL_MODELS_SHARD names `k/N`, as the
# lane's jobs and a landing's units do, the k-th part of this repository's runs,
# spread by their recorded weight (`shard_runs` in sce-build/src/formal/models.rs,
# whose unit test holds the parts to every run once). Unset, every run. Whole,
# the runs took 1042 s on a hosted runner (job 138489726305, 2026-11-24). The
# consumer fixture is four states and is judged by the first part alone.
SHARD_ARGS=()
FIRST_PART=1
if [[ -n "${SCE_FORMAL_MODELS_SHARD:-}" ]]; then
    SHARD_ARGS=(--shard "$SCE_FORMAL_MODELS_SHARD")
    [[ "$SCE_FORMAL_MODELS_SHARD" == 1/* ]] || FIRST_PART=0
fi

for tool in java tlc apalache storm; do
    install=0
    target/debug/sce-codegen formal install --tool "$tool" || install=$?
    case "$install" in
        0) ;;
        15) sce_gate_cannot_run "the pinned $tool could not be fetched or admitted" ;;
        *) sce_gate_fail "sce-codegen formal install --tool $tool ended $install" ;;
    esac
done

# The processors this unit was given (`sce_build_jobs`: a build machine's
# grant, `BX_GRANT_CORES`, before anything the machine reads of itself), each
# run of TLC given them one after another. Never TLC's `auto`, which is every
# processor of the machine whatever the unit was granted: a unit granted one
# core ran so on 2026-11-27 and did not end within the hour.
WORKERS="$(sce_build_jobs)"

judge() {
    local status=0
    target/debug/sce-codegen formal models --workers "$WORKERS" --manifest "$@" || status=$?
    case "$status" in
        0) ;;
        15) sce_gate_cannot_run "not the pinned Java runtime, TLC or Apalache: sce-codegen formal install" ;;
        *) sce_gate_fail "a model is not as $1 declares it" ;;
    esac
}

# The properties judge (docs/adr/0115, D15) on the fixture of one machine: the
# properties that hold, judged and written with the digests of what was judged,
# then read back from that record; a guarantee in each form broken on purpose,
# which must be found; a vacuous guarantee and an assumption about the machine
# itself, which must be refused by name. `$1` the properties, `$2` the
# exploration, `$3` the status the judge must end with, and each further
# argument a line its report must hold; flags for the judge are
# PROPERTIES_FLAGS.
PROPERTIES=sce-build/tests/fixtures/formal_properties
PROPERTIES_FLAGS=()
judge_properties() {
    local properties="$PROPERTIES/$1" exploration="$PROPERTIES/$2" expected="$3"
    local status=0 report line
    shift 3
    report="$(target/debug/sce-codegen formal properties --workers "$WORKERS" \
        --properties "$properties" --exploration "$exploration" \
        ${PROPERTIES_FLAGS[@]+"${PROPERTIES_FLAGS[@]}"})" || status=$?
    printf '%s\n' "$report"
    case "$status" in
        "$expected") ;;
        15) sce_gate_cannot_run "not the pinned Java runtime or TLC: sce-codegen formal install" ;;
        *) sce_gate_fail "$properties ended $status, and must end $expected" ;;
    esac
    for line in "$@"; do
        grep -qxF -- "$line" <<<"$report" \
            || sce_gate_fail "$properties: the report does not say: $line"
    done
}

judge_turnstile() {
    local within="holds within: the most events from outside (4) reached"
    judge_properties turnstile_properties.scxml turnstile_exploration.scxml 0 \
        "every macrostep ends: $within" \
        "assumed and not discharged: ENV-1" \
        "TURN-1 (<sce:always>): $within" \
        "TURN-2 (<sce:step>): $within" \
        "TURN-3 (<sce:leads-to>): $within" \
        "TURN-4 (<sce:recurs>): $within" \
        "TURN-5 (<sce:possible>): $within" \
        "every property holds as judged"
}

quantities() {
    local status=0
    target/debug/sce-codegen formal quantities --workers "$WORKERS" --manifest "$1" || status=$?
    case "$status" in
        0) ;;
        15) sce_gate_cannot_run "not the pinned Java runtime, TLC or Storm: sce-codegen formal install" ;;
        *) sce_gate_fail "a JANI model is not held to its TLA+ model as $1 declares it" ;;
    esac
}

judge docs/formal/models.json ${SHARD_ARGS[@]+"${SHARD_ARGS[@]}"}
if [[ "$FIRST_PART" == 1 ]]; then
    judge sce-build/tests/fixtures/formal_consumer/formal/models.json
    RECORDED="$(mktemp)"
    PROPERTIES_FLAGS=(--record "$RECORDED")
    judge_turnstile
    PROPERTIES_FLAGS=(--recorded "$RECORDED")
    judge_turnstile
    rm -f "$RECORDED"
    PROPERTIES_FLAGS=()
    judge_properties turnstile_broken_properties.scxml turnstile_exploration.scxml 1 \
        "BROKEN-1 (<sce:always>): violated" \
        "BROKEN-2 (<sce:step>): violated" \
        "BROKEN-3 (<sce:recurs>): violated" \
        "BROKEN-4 (<sce:leads-to>): violated"
    judge_properties turnstile_unreachable_properties.scxml turnstile_quiet_exploration.scxml 1 \
        "every macrostep ends: holds of the whole space" \
        "UNREACH-1 (<sce:possible>): violated"
    judge_properties turnstile_vacuous_properties.scxml turnstile_exploration.scxml 1 \
        "VAC-1 (<sce:leads-to>): vacuous: it holds though \`In('unlocked')\` is replaced by true, so it passes for nothing in that part"
    judge_properties turnstile_assumes_itself_properties.scxml turnstile_exploration.scxml 1
    quantities docs/formal/quantities.json
fi
