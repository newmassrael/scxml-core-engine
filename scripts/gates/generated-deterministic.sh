#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: none
#
# A generated tree is a function of its declared inputs on every machine
# (docs/adr/0097, section 5). Every action of every declared tree runs here
# from an empty cache, and each result is held against the fleet's entry for
# the same action key: an entry with other files or other bytes is an action
# that is not a function of what its key names, or a fleet holding a wrong
# entry, and either would reach every machine that takes it.
#
# The fleet is the REAPI instance `SCE_FLEET_CACHE` and `SCE_FLEET_INSTANCE`
# name (the forward `bx-fleet site-tunnel <site>` opens, or a site's worker
# port). Until bx's fleet cache exists no machine names one, and the gate
# says it was skipped and passes: what it would compare does not exist yet.
# The half that needs no fleet, every declared tree materialized twice from
# empty caches into two paths and compared, is
# `a_declared_tree_reproduces_its_committed_tree` in the workspace suite.
#
# Two programs: the generator as every action key names it (default features
# and `cli`), which runs the actions, and the comparer, built with
# `remote-cache` besides into a target directory of its own so that it never
# replaces the generator other gates run.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

if [[ -z "${SCE_FLEET_CACHE:-}" ]]; then
    sce_gate_step "skipped: SCE_FLEET_CACHE names no fleet cache, so there is no other machine's entry to compare with (docs/adr/0097, amendment 16)"
    exit 0
fi

cargo build --bin sce-codegen --features cli -p sce-build \
    || sce_gate_fail "sce-codegen build"
cargo build --bin sce-codegen --features cli,remote-cache -p sce-build \
    --target-dir target/fleet-compare \
    || sce_gate_fail "sce-codegen build with the fleet's client"

trees=()
for manifest in tools/codegen/trees/*.toml; do
    trees+=("$(basename "$manifest" .toml)")
done
[[ ${#trees[@]} -gt 0 ]] || sce_gate_cannot_run "no tree manifest under tools/codegen/trees"

target/fleet-compare/debug/sce-codegen fleet-compare "${trees[@]}" \
    --generator target/debug/sce-codegen \
    || sce_gate_fail "an action's result differs from the fleet's entry of the same key"
