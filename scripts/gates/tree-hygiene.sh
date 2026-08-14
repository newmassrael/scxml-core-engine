#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: tree-hygiene.yml
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
# `cmake_option_guard_scope` reads every tracked CMake file, for the same
# reason in a narrower alphabet: an option can default to false, so a
# block appended past its `endif()` configures cleanly and fails at build
# time, and a directory can acquire a CMakeLists.txt anywhere.
#
# tree-hygiene.yml declares no `paths:` filter, so the registry derives
# "always" for this gate the same way it derives every other
# workflow-backed trigger. Every target is named explicitly rather than
# swept, so this costs one cached build and sub-second tests instead of the
# full workspace suite.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# `--features cli` because `committed_sourcemap_drift`,
# `diagnostic_corpus_schema` and `diagnostic_fix_is_applicable` drive the
# real `sce-codegen` binary; the other targets do not need it and are
# unaffected by its presence.
cargo test -p sce-build --features cli \
    --test roadmap_marker_gate \
    --test scope_terminology \
    --test workflow_trigger_coverage \
    --test hook_ci_parity \
    --test codegen_binary_resolution \
    --test cmake_option_guard_scope \
    --test gate_registry_contract \
    --test committed_sourcemap_drift \
    --test diagnostic_corpus_schema \
    --test diagnostic_fix_is_applicable \
    --test sourced_scripts_are_tracked \
    --test sourcemap_symbol_markers \
    --test forge_document_name_is_the_stem \
    --test test_result_gating \
    --test ledger_symbol_axis_reach \
    --test integration_stem_registration \
    --test datamodel_read_accessor \
    || sce_gate_fail "tree-wide hygiene gates"
