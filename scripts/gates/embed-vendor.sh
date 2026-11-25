#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: embed-vendor-smoke.yml
#
# embed/ vendor channel (mirror of embed-vendor-smoke.yml, in full).
#
# embed/MANIFEST.json is the tracked snapshot of the embeddable public API
# surface. A header edit under sce/include/ that forgets
# `scripts/package_embed.sh` lands a stale manifest.
# `embed-manifest-failfast` covers the emit fail-fast path; this covers the
# orthogonal DRIFT path — committed manifest against the current header
# surface — the gap that let the SetCurrentEventArgs surface change land
# stale.
#
# The workflow runs three checks and this gate ran only the first until
# 2026-08-04, which is the harder kind of gap to notice: it fires on the
# right changes and still misses two thirds of what CI will run. The other
# two cover different halves — verify_embed_payload.sh catches a manifest
# left stale by a sce/ edit (the upstream direction verify_embed_manifest.sh
# cannot see, since it re-emits from whatever embed/include/ is on disk),
# and smoke_embed_consumer.sh builds a consumer against a freshly packaged
# tree, catching assets that sce_codegen_assets.cmake lists but
# package_embed.sh never ships.
#
# All three work in `mktemp -d` scratch directories with `trap rm -rf`, so
# none can leave the working tree dirty, and none reads what another wrote.
#
# In parts (ADR 0120, D3). When SCE_EMBED_VENDOR_SHARD names `k/N`, as the
# lane's jobs and a landing's units do, the gate runs the k-th line of
# EMBED_VENDOR_PARTS below; N must be the number of lines. Unset, it runs every
# line, in this order. Whole, the gate ran 1142 s of a 1496 s job (job
# 138811665797, 2026-11-24): the drift check six minutes, the lag check six,
# and the consumers seven, each a packaging of its own, so each is a part.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

part_drift() {
    sce_gate_step "manifest drift against the current header surface"
    bash scripts/verify_embed_manifest.sh \
        || sce_gate_fail "embed/MANIFEST.json drift"
}

part_lag() {
    sce_gate_step "manifest lag behind sce/"
    bash scripts/verify_embed_payload.sh \
        || sce_gate_fail "embed/MANIFEST.json stale relative to sce/"
}

part_consumers() {
    sce_gate_step "consumer build over a freshly packaged tree"
    bash scripts/smoke_embed_consumer.sh \
        || sce_gate_fail "embed consumer smoke build"

    # The Bazel channels: the same freshly packaged tree as a Bzlmod module, and the
    # checkout through scripts/sce_repo.bzl, both at the pinned Bazel. Bazel 9
    # removed the native C++ rules, so a BUILD file that does not load them from
    # rules_cc fails here and not in a consumer's build.
    sce_gate_step "Bazel consumers through both channels"
    # The smoke ends 3 when no Bazel is on the machine: nothing was measured, which
    # is "could not run", never the tree's failure.
    local bazel_rc=0
    bash scripts/smoke_bazel_consumer.sh || bazel_rc=$?
    case "$bazel_rc" in
        0) ;;
        3) sce_gate_cannot_run "no Bazel on this machine (bx installs the pinned one: [platform.linux.tools.bazel] in bx.toml)" ;;
        *) sce_gate_fail "Bazel consumer smoke build" ;;
    esac
}

# The parts, one job of the lane each, each a `part_<name>` above;
# `hook_ci_parity` holds the lane's shard count to the number of lines.
EMBED_VENDOR_PARTS=(
    "drift"
    "lag"
    "consumers"
)

PARTS=("${EMBED_VENDOR_PARTS[@]}")
if [[ -n "${SCE_EMBED_VENDOR_SHARD:-}" ]]; then
    [[ "$SCE_EMBED_VENDOR_SHARD" =~ ^([0-9]+)/([0-9]+)$ ]] \
        || sce_gate_fail "SCE_EMBED_VENDOR_SHARD is k/N, got '$SCE_EMBED_VENDOR_SHARD'"
    shard="${BASH_REMATCH[1]}"
    count="${BASH_REMATCH[2]}"
    (( count == ${#EMBED_VENDOR_PARTS[@]} )) \
        || sce_gate_fail "SCE_EMBED_VENDOR_SHARD names $count shard(s), and the gate has ${#EMBED_VENDOR_PARTS[@]} parts"
    (( shard >= 1 && shard <= count )) \
        || sce_gate_fail "SCE_EMBED_VENDOR_SHARD names shard $shard of $count"
    read -r -a PARTS <<<"${EMBED_VENDOR_PARTS[shard - 1]}"
    sce_gate_step "shard $shard of $count: ${PARTS[*]}"
fi

for part in "${PARTS[@]}"; do
    "part_$part"
done
