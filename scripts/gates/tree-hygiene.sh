#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
# `the_conformance_cmake_route_declares_what_the_generator_plans` reads every
# tracked CMake file too, for that last reason: a plan of a generation that no
# request asks ahead is sent to the generator alone when the tree is
# configured, and the file that writes one next can be anywhere.
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
#
# `lowering_decision_ledger` is here for the sharper half of the same reason.
# It reads every tracked CMake file, every C++ source under
# `sce/src/scripting/` and the prose of `docs/SCE_LUA_TRANSLATION_SEAM.md` to
# hold the D1 rows to the tree, so what selects it is an edit anywhere those
# three shapes live. The middle one was one named file until the seam's call
# moved translation unit, which is the second time a filter written over the
# tree of the day would have named the answer instead of the case. It was here
# first for a premise — that nothing linked a Rust artifact yet — and that
# premise was made false on 2026-08-29 by the link the ledger now records,
# which is exactly the case a filter written over the tree of the day would
# have missed: it names the answer and not the case.
#
# `scope_obligation` lowers every expression in every committed *.scxml once
# per scope stage, so a document added anywhere changes what it reads. Its
# reason for being here is sharper than the sweep, though: the census it
# prints is what says whether a run-time lowering surface needs a scope
# handle, and a zero is an answer. A document that crosses a stage boundary
# is what keeps that census honest, so it is precisely a NEW document — the
# thing a `paths:` filter over today's trees cannot name — that this gate
# must not miss.
#
# `a_standard_named_in_code_is_one_sce_implements` lexes every tracked
# source file in six languages and refuses a standards designation that
# reaches a position which RUNS, while leaving the same name in a comment
# alone — a citation is the evidence a rule was derived from. Its reason for
# being here is the registry's own: the case it exists for is a
# specification SCE has never seen, named for the first time in a file that
# does not exist today, so a filter written over the tree as it stands names
# the answers and not the case.
#
# `an_identifier_is_checked_against_the_grammar_w3c_gives_it` sweeps every
# committed `*.scxml` and checks each identifier-bearing attribute against
# the grammar W3C gives it, so a document added anywhere changes what it
# reads. It ran only under the workspace suite until 2026-09-14, and that
# lane's `paths:` filter names today's fixture trees — so a hostile `id`
# arriving in a directory the filter does not name was judged by a lane the
# commit never started. That is precisely the case the gate exists for,
# which is why it runs from this unfiltered workflow instead.
#
# `a_name_the_generated_code_spells_is_a_code_identifier` is that sweep for
# the names SCE owns — `SCE_ACCEPTED_SUBSET.md` §2.14 — and is here for the
# same reason: it proves each row of its table reachable from whichever
# committed document carries that attribute, and the carrier of the next row
# can arrive in any directory.
#
# `a_forge_document_never_declares_one_name_twice` parses every committed
# `*.scxml` and fails if the rule against two names that a backend spells as
# one refuses any of them (`SCE_ACCEPTED_SUBSET.md` §2.14). It is here for the
# same reason: a pair that folds can arrive in a document in any directory, and
# the lane that would judge it must start on that commit.
#
# `an_sce_attribute_nothing_reads_is_refused_not_ignored` parses every
# committed `*.scxml` against the rule that an `sce:` attribute on a W3C
# element must have a reader, and writes an invented one on each W3C element
# name the documents use. The document that carries an unread attribute next
# can be written in any directory, which is this workflow's reason.
#
# `an_scxml_attribute_outside_the_standard_is_refused` parses every committed
# `*.scxml` against the Recommendation's attribute table and refuses an
# unqualified attribute outside it, and holds that table to the specification
# text. The document that carries such an attribute next can be written in any
# directory, which is this workflow's reason.
#
# `non_latin_prose_declares_why_it_is_here` decodes every tracked file and
# refuses a line of prose written in a script this tree does not write prose
# in, unless the file argues in one line that the characters are what it
# measures. It is here for the registry's plainest reason: the six files it
# was written for sat in the C AOT suite, a Gradle properties file and the
# visualizer — no prefix joins them — and the case it exists for is the
# SEVENTH, in a file that does not exist today. A filter written over the
# places already cleaned names the answers and not the case.
#
# `the_licence_expression_names_its_exception_as_an_addition` reads the SPDX
# expression on every tracked file, and the licence field of every Cargo and
# Python manifest, against the expression LICENSE-EXCEPTION.md gives. Every
# source file carries one, so the next one spelled wrongly is in a file that
# does not exist today, in any directory.
#
# `spec_surface_registration` is here for the registry's own reason, and it
# is the sharpest instance of it. What it reads is every tracked path, and
# what it judges is whether `SCE_WIRE_CONTRACTS.md` names the spec-bearing
# surfaces the tree holds — stores under `docs/spec/**` and
# `docs/sce-ledger/**`, conformance registries under `tests/**`, the embed
# manifest and the vendored W3C snapshot under `tools/**`.
# rust-workspace-tests.yml's `paths:` filter names none of those roots, so
# the case the gate exists for — a NEW surface, in a directory
# that does not exist today — is precisely the commit that would not start
# the lane holding it. This workflow declares no filter, so it runs on
# exactly those pushes.
#
# `a_regeneration_removes_what_it_no_longer_writes` is here because what it
# holds together lives in no workflow's `paths:`: the regeneration helper under
# `scripts/lib/`, the `regen-<step>` actions of bx.toml, and every
# committed file of the generated trees, which it reads to prove each one is
# recognised as generated or written by hand. The file whose kind the helper
# does not know is a NEW kind, in a tree a filter over today's files would not
# name.
#
# `a_documents_tables_are_its_tree_in_document_order` parses every committed
# `*.scxml` the generator accepts and holds its tables (ADR 0064) to the
# numbering the Rust runtime's Appendix D core relies on: states in document
# order, each subtree one contiguous run, every row agreeing with the model the
# generated structural methods are emitted from. The document whose tree breaks
# that is one an author writes next, in any directory.
#
# `no_file_keeps_a_copy_of_the_w3c_registry` reads every tracked file and
# refuses a list of W3C test numbers a file keeps instead of asking the
# conformance registry. The four copies it was written after (a C++ runner, a
# WASM pre-js, an interactive test, a Python test) shared no prefix, and the
# next is written in a file that does not exist today.
#
# `ffi` is here so that SOMETHING in the Rust lanes compiles the C
# surface. It is off by default and `clippy-check.yml` runs `--workspace
# --all-targets` without `--all-features`, so without this no Rust lane
# touches it — and a surface nothing compiles is one that rots. The C++
# build does compile it now (`cmake/SCEBuildLowering.cmake`), but a
# breakage found there is found by a 30-minute lane; found here it is
# found in seconds.
# ONE PART OF THE SUITE, when `SCE_TREE_HYGIENE_SHARD=k/N` asks for one: the
# CI lane runs these targets as N jobs (tree-hygiene.yml, ADR 0120 D3). Run
# whole on a hosted runner they took 1530 s of tests after a 385 s build (job
# 138916754622, 2026-11-25), the longest target 279 s. The units are the
# targets the `cargo test` command below names, read from this file by
# scripts/lib/workspace_shards.py (`--gate-targets`), so the list stays
# written once, in the command; `hygiene_part` runs that command with the
# targets of this part and no other. Every job writes the whole plan beside
# its part, and the lane's last job holds the plans of all the parts to one
# digest and every part to a report that each of its targets ran
# (`ctest_shards.py verify`); `hook_ci_parity` holds the parts to every target
# the command names, each in one part. The two hook tests at the end build
# nothing and take seconds; the first part runs them. Unset, the gate runs the
# whole suite, which is what a developer and the build machines run.
SHARD_PLAN=""
SHARD_UNITS=()
SHARD_INDEX=1
HYGIENE_LOG="$(mktemp)"
sce_gate_on_exit "rm -f '$HYGIENE_LOG'"
if [[ -n "${SCE_TREE_HYGIENE_SHARD:-}" ]]; then
    SHARD_PLAN="$SCE_REPO_ROOT/target/tree-hygiene-shard"
    rm -rf "$SHARD_PLAN"
    shard_units="$(python3 scripts/lib/workspace_shards.py \
        --gate-targets scripts/gates/tree-hygiene.sh --out "$SHARD_PLAN" "$SCE_TREE_HYGIENE_SHARD")" \
        || sce_gate_cannot_run "scripts/lib/workspace_shards.py could not name shard $SCE_TREE_HYGIENE_SHARD"
    while IFS= read -r unit; do
        case "$unit" in
            "test "*) SHARD_UNITS+=("$unit") ;;
            "") ;;
            *) sce_gate_cannot_run "scripts/lib/workspace_shards.py named a unit it does not define: $unit" ;;
        esac
    done <<<"$shard_units"
    SHARD_INDEX="${SCE_TREE_HYGIENE_SHARD%%/*}"
    sce_gate_step "shard $SCE_TREE_HYGIENE_SHARD: ${#SHARD_UNITS[@]} test target(s)"
fi

# The command it is given, with each `--test <name>` kept when that target is
# in this part and dropped when it is not; unsharded, the command as written.
# A part whose targets the command does not all name is refused: the plan and
# the command would then be reading two lists.
hygiene_part() {
    if [[ -z "$SHARD_PLAN" ]]; then
        sce_gate_tested --log "$HYGIENE_LOG" -- "$@"
        return
    fi
    local -a command=()
    local kept=0 unit
    while (( $# )); do
        if [[ "$1" == --test ]]; then
            for unit in ${SHARD_UNITS[@]+"${SHARD_UNITS[@]}"}; do
                if [[ "$unit" == "test $2" ]]; then
                    command+=(--test "$2")
                    kept=$((kept + 1))
                    break
                fi
            done
            shift 2
            continue
        fi
        command+=("$1")
        shift
    done
    (( kept == ${#SHARD_UNITS[@]} )) \
        || sce_gate_fail "shard $SCE_TREE_HYGIENE_SHARD holds ${#SHARD_UNITS[@]} target(s) and the command names $kept of them"
    (( kept > 0 )) || return 0
    sce_gate_tested --log "$HYGIENE_LOG" -- "${command[@]}"
}

hygiene_part cargo test -p sce-build --features cli,ffi \
    --test a_standard_named_in_code_is_one_sce_implements \
    --test a_declared_attribute_must_reach_the_ir \
    --test a_deployment_annotation_is_not_the_document \
    --test a_line_that_opens_a_block_carries_its_word \
    --test a_page_in_any_shape_normalises_to_the_canonical_one \
    --test a_rendering_reads_back_as_the_same_document \
    --test a_rendering_shows_the_literal_the_author_wrote \
    --test an_action_uses_only_the_fields_its_tag_declares \
    --test the_analyzer_declares_which_fields_it_writes \
    --test transition_action_annotations \
    --test b9_drift_detection \
    --test a_value_written_into_a_string_literal_is_escaped \
    --test codegen_depfile_coverage \
    --test spec_citations_carry_no_line_numbers \
    --test an_identifier_is_checked_against_the_grammar_w3c_gives_it \
    --test a_name_the_generated_code_spells_is_a_code_identifier \
    --test a_forge_document_never_declares_one_name_twice \
    --test an_sce_attribute_nothing_reads_is_refused_not_ignored \
    --test an_scxml_attribute_outside_the_standard_is_refused \
    --test non_latin_prose_declares_why_it_is_here \
    --test the_licence_expression_names_its_exception_as_an_addition \
    --test ffi_header_parity \
    --test roadmap_marker_gate \
    --test scope_terminology \
    --test spec_surface_registration \
    --test workflow_trigger_coverage \
    --test a_transitions_enabling_events_are_the_rows_its_event_enum_numbers \
    --test the_class_walker_runs_as_the_general_interpreter_does \
    --test hook_ci_parity \
    --test the_restricted_term_gate_runs_each_pattern_once \
    --test build_jobs_has_one_owner \
    --test codegen_binary_resolution \
    --test cmake_option_guard_scope \
    --test the_conformance_cmake_route_declares_what_the_generator_plans \
    --test no_file_keeps_a_copy_of_the_w3c_registry \
    --test configuration_rules_cite_their_clause \
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
    --test scope_obligation \
    --test lowering_decision_ledger \
    --test cli_expression_refusal \
    --test cli_feature_gating \
    --test cli_guard_emission \
    --test cli_lint_sweep \
    --test mutation_rounds_selection \
    --test mutation_corpus_fits_its_lane \
    --test mutation_round_survives_the_next_push \
    --test ci_supersession_policy \
    --test mesh_rpc_backend_contract \
    --test ecma262_scoreboard_contract \
    --test script_engine_language_parity \
    --test the_pin_is_read_from_one_place \
    --test every_annotated_document_is_accounted_for \
    --test script_engine_census \
    --test hybrid_stubs_are_written_only_by_the_generator \
    --test a_regeneration_removes_what_it_no_longer_writes \
    --test a_cpp_test_unit_compiles_a_bounded_number_of_machines \
    --test a_documents_tables_are_its_tree_in_document_order \
    --test a_workflow_names_the_image_it_runs_on \
    --test a_pipeline_has_no_reader_that_leaves_early \
    --test no_workflow_installs_a_package_at_job_time \
    || sce_gate_fail "tree-wide hygiene gates"

# What this part ran, for the lane's last job to hold beside the other parts'
# (`ctest_shards.py verify`): a target counts when cargo started its binary,
# and one cargo never started fails the part here rather than reading as a pass.
if [[ -n "$SHARD_PLAN" ]]; then
    ran=0
    missing=()
    for unit in ${SHARD_UNITS[@]+"${SHARD_UNITS[@]}"}; do
        if grep -qE "Running.* tests/${unit#test }(\.rs|/main\.rs) \(" "$HYGIENE_LOG"; then
            ran=$((ran + 1))
        else
            missing+=("$unit")
        fi
    done
    if (( ${#missing[@]} > 0 )); then
        sce_gate_fail "cargo started no binary for ${#missing[@]} target(s) of shard $SCE_TREE_HYGIENE_SHARD: ${missing[*]}"
    fi
    python3 scripts/lib/ctest_shards.py report --out "$SHARD_PLAN" --executed "$ran" --disabled 0
    sce_gate_step "shard $SCE_TREE_HYGIENE_SHARD: $ran target(s) ran and passed"
    if (( SHARD_INDEX != 1 )); then
        exit 0
    fi
fi

# The pre-push hook exempts a bx landing queue's ref by what it holds, never by
# its name alone (tools/git-hooks/bx-land-refs.sh): the test builds a repository
# of each case, a commit of this repository pushed under a queue's name among them.
bash tools/git-hooks/test-bx-land-refs.sh \
    || sce_gate_fail "the pre-push hook's judgement of bx landing refs"

# An update to main is judged as a landing (`--landing`, no skip a pass), and
# the hook hands bx's status through as it came: the landing queue's integrator
# lands an item only on 0, so a 2 (a unit not measured) read as 0 would land it
# unjudged (tools/git-hooks/prepush-gates.sh). The test runs the whole hook on a
# repository it makes, with a stand-in bx and a stand-in runner.
bash tools/git-hooks/test-prepush-gates.sh \
    || sce_gate_fail "the pre-push hook's selection for main, or the status it hands through"
