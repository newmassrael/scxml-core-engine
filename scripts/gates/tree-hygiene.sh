#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Tree-wide hygiene gates (mirror of tree-hygiene.yml).
#
# Four gates whose inputs are wider than any `paths:` list.
# `roadmap_marker_gate` reads every tracked file; when it rode the workspace
# suite, a violation under a path outside that suite's filter never started
# CI and reached main green. `workflow_trigger_coverage` reads every
# workflow and is what keeps the arrangement from rotting: it fails if a
# gate like that is ever run only by a filtered workflow again.
# `hook_ci_parity` covers the neighbouring failure — a gate that fires
# correctly but runs a weaker command than the workflow it mirrors.
# `codegen_binary_resolution` reads every tracked file too: it holds that no
# harness names a cargo build profile when reaching for the sce-codegen
# binary, and those sites span five languages with no common path prefix.
#
# tree-hygiene.yml declares no `paths:` filter, so the registry derives
# "always" for this gate the same way it derives every other
# workflow-backed trigger. Every target is named explicitly rather than
# swept, so this costs one cached build and sub-second tests instead of the
# full workspace suite.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

cargo test -p sce-build \
    --test roadmap_marker_gate \
    --test workflow_trigger_coverage \
    --test hook_ci_parity \
    --test codegen_binary_resolution \
    --test gate_registry_contract \
    || sce_gate_fail "tree-wide hygiene gates"
