#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: formal-isolation.yml
#
# Every commit of the change set changes what a formal check says by itself, with
# nothing beside it but documents, and names its reason in a bullet
# `- Spec: <why>` (docs/adr/0115, D11), so that no check is made to pass by
# changing the check.
#
# The judge is `sce-codegen formal isolation` (docs/adr/0146, order step 5), the
# command a consumer runs over its own commits. Its own checks are its unit
# tests, which the Rust lanes run.
#
# The unit is the commit, so the gate reads commits and not the tree: the ranges
# it judges are the ones the runner hands it (`SCE_GATE_CHANGE_RANGES`), which
# a push's plan carries to its unit (`--change-set`, `commits: ["CHANGES"]` in
# the registry) and the lane resolves from the last run that reached a verdict.
# The push hook and the lane therefore judge the same commits by one command.
# Handed no range (`--all`, or slugs alone), there is no commit to judge: the
# gate says so and skips, which a landing counts as could not run.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

if [[ -z "${SCE_GATE_CHANGE_RANGES:-}" ]]; then
    sce_gate_skip "handed no commits to judge: pass --change-set <base>..<head>, as a push's plan and the lane do."
fi
read -r -a RANGES <<<"$SCE_GATE_CHANGE_RANGES"

# `target/debug/sce-codegen` is built by `codegen-build`, which the registry
# names as this gate's dependency.
status=0
target/debug/sce-codegen formal isolation "${RANGES[@]}" || status=$?
case "$status" in
    0) ;;
    20) sce_gate_cannot_run "the commits of ${RANGES[*]} could not be read: is the history there?" ;;
    *) sce_gate_fail "a commit changes what a check says beside what it judges, or gives no reason (docs/adr/0115, D11)" ;;
esac
