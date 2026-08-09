#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
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
# none can leave the working tree dirty.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

sce_gate_step "manifest drift against the current header surface"
bash scripts/verify_embed_manifest.sh \
    || sce_gate_fail "embed/MANIFEST.json drift"

sce_gate_step "manifest lag behind sce/"
bash scripts/verify_embed_payload.sh \
    || sce_gate_fail "embed/MANIFEST.json stale relative to sce/"

sce_gate_step "consumer build over a freshly packaged tree"
bash scripts/smoke_embed_consumer.sh \
    || sce_gate_fail "embed consumer smoke build"
