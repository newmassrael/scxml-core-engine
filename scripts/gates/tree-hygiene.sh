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
# `mutation_corpus_fits_its_lane` and `mutation_round_survives_the_next_push`
# are here for the 2026-08-04 reason in its purest form: both judge
# `.github/workflows/mutation-rounds.yml` — the first its job ceiling
# against the slice size this repository keeps in
# `scripts/gates/mutation-rounds.sh`, the second its concurrency group —
# and the only workflow that ran either of them was
# rust-workspace-tests.yml, whose `paths:` filter names its own workflow
# file and not the lane it judges. So an edit to that ceiling or that group
# was held to both rules by a suite the edit could not start. The first
# also drives the mutation-rounds gate over the whole corpus, which reaches
# every casefile `git ls-files` finds, so it is tree-wide by the ordinary
# measure as well.
#
# tree-hygiene.yml declares no `paths:` filter, so the registry derives
# "always" for this gate the same way it derives every other
# workflow-backed trigger. Every target is named explicitly rather than
# swept, so this costs one cached build and sub-second tests instead of the
# full workspace suite.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# `--features cli` because `committed_sourcemap_drift`,
# `diagnostic_corpus_schema`, `diagnostic_fix_is_applicable` and
# `mesh_rpc_backend_contract` drive the real `sce-codegen` binary; the
# other targets do not need it and are unaffected by its presence.
#
# `script_engine_language_parity` is here because its fixture lives under
# `integration_resources/`, which no workflow's `paths:` filter names — the
# workspace lane starts for `sce-build/**` and `schemas/**` but not for the
# document the gate generates. An edit to that document would otherwise change
# what the gate measures without starting the lane that measures it.
#
# `ecma262_scoreboard_contract` is here for the same reason and reads a wider
# set still: `ARCHITECTURE.md`'s engine matrix and the two JSON files under
# `tests/ecmascript/`. None of the three is in any workflow's `paths:` filter,
# and the numbers it derives are exactly the kind that rot in prose — the
# column it holds had been scored out of 58 for weeks after the table grew
# to 98.
#
# `mesh_rpc_backend_contract` is here rather than left to the workspace
# suite for the reason the whole list exists: what it reads is `SCE_MESH.md`
# §9.5, `tools/codegen/templates/mesh/**` and a fixture under `tests/mesh/`,
# and rust-workspace-tests.yml's `paths:` filter names none of the three. An
# edit to the mesh roster would otherwise be held to the contract only by a
# lane that edit cannot start. This workflow declares no filter, so it runs
# on exactly the pushes such an edit arrives in.
cargo test -p sce-build --features cli \
    --test roadmap_marker_gate \
    --test scope_terminology \
    --test workflow_trigger_coverage \
    --test hook_ci_parity \
    --test build_jobs_has_one_owner \
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
    --test ecmascript_semantics \
    --test ecmascript_acceptance_parity \
    --test cli_expression_refusal \
    --test cli_guard_emission \
    --test mutation_rounds_selection \
    --test mutation_corpus_fits_its_lane \
    --test mutation_round_survives_the_next_push \
    --test mesh_rpc_backend_contract \
    --test ecma262_scoreboard_contract \
    --test script_engine_language_parity \
    || sce_gate_fail "tree-wide hygiene gates"
