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
# Runs in the host image, which carries the JDK. The pinned TLC and Apalache are
# fetched by `sce-codegen formal install` below and admitted by their digests, or
# found in the image's home; the judge itself never fetches. Apalache checks the
# inductive invariants a manifest states (docs/adr/0115, D3), and gives TLC the
# standard module `Apalache` a model extends. Without Java, TLC or Apalache the
# judge exits 15 (`formal/tool-unavailable`), which is "could not run" and no
# verdict about the models.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# What `pre-push` judges the commits of a push by (docs/adr/0115, D11); its tests run here.
python3 tools/git-hooks/spec_isolation.py --self-test \
    || sce_gate_fail "the judge of what changes a check is wrong (tools/git-hooks/spec_isolation.py --self-test)"

# `target/debug/sce-codegen` is built by `codegen-build`, which the registry
# names as this gate's dependency.
# In parts (ADR 0120, D3): when SCE_FORMAL_MODELS_SHARD names `k/N`, as the
# lane's jobs and a landing's units do, the k-th part of this repository's runs,
# spread by their recorded weight (`shard_runs` in sce-build/src/formal/models.rs,
# whose unit test holds the parts to every run once). Unset, every run. Whole,
# the runs took 1042 s on a hosted runner (job 138489726305, 2026-11-24). The
# consumer fixture is four states and is judged by the first part alone.
SHARD_ARGS=()
CONSUMER=1
if [[ -n "${SCE_FORMAL_MODELS_SHARD:-}" ]]; then
    SHARD_ARGS=(--shard "$SCE_FORMAL_MODELS_SHARD")
    [[ "$SCE_FORMAL_MODELS_SHARD" == 1/* ]] || CONSUMER=0
fi

for tool in tlc apalache; do
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
        15) sce_gate_cannot_run "no Java, or not the pinned TLC or Apalache: sce-codegen formal install" ;;
        *) sce_gate_fail "a model is not as $1 declares it" ;;
    esac
}

judge docs/formal/models.json ${SHARD_ARGS[@]+"${SHARD_ARGS[@]}"}
if [[ "$CONSUMER" == 1 ]]; then
    judge sce-build/tests/fixtures/formal_consumer/formal/models.json
fi
