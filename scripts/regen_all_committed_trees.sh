#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Master regeneration for every committed §synth-6.2.6 generated tree
# tracked by the SCE workspace, as a list of STEPS. A step owns the trees it
# writes: no other step writes into them, and no step reads what another
# writes, so each step runs on its own and the order between them is not part
# of the result. The steps and the trees each owns are listed once, in
# scripts/lib/sce_generated_tree.sh (`sce_regen_steps`, `sce_generated_trees`);
# the work of each is a `step_<name>` function below.
#
# Each step is also a bx action of its own, `[actions.regen-<step>]` in
# bx.toml, whose `run` is this script given that step. bx keeps each action's
# result and spreads the actions over the build machines; `bx --actions` runs
# every one of them, which is "regenerate everything" on the build machines.
# bx.toml says why the steps are grouped as they are.
#
# The test trees of the build-time backends (cpp / c11 / Python) are
# intentionally absent — they materialise at CMake / CI time without a
# committed §synth-6.2.6 header to refresh. A runtime's engine tree is not a
# test tree: a consumer builds the runtime without the generator, so it is
# committed and has a step here whatever its language.
#
# Use after any edit that can change generated output — a template, the
# generator's own code, an input document. Regenerate and expect no diff
# beyond the files that edit actually changes: the §synth-6.2.6 header
# carries only facts about each file's inputs, so nothing is re-stamped
# that did not change. A generated file the generator no longer writes is
# removed, and named when its step ends, so the diff carries deletions as
# well as edits. Running this script + `git add -A` produces a
# single coherent commit across every tracked context in one shot, and
# `scripts/gate regen-reproduces` is the check that it did.
#
# Usage (from repo root):
#   scripts/regen_all_committed_trees.sh            # every step, in order
#   scripts/regen_all_committed_trees.sh STEP...    # only the steps named
#   scripts/regen_all_committed_trees.sh --list     # the names of the steps

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

# Each stage names itself as it starts and says how long it took when the
# next one does. A run of this script has been measured at three hours, and
# a stage is only worth attacking once it is measured: the log used to say
# which stages ran and not where the time went, so the one question a slow
# run raises could not be answered from the run itself.
STAGE_NAME=""
STAGE_START=$SECONDS
stage() {
    if [[ -n "$STAGE_NAME" ]]; then
        echo "    ${STAGE_NAME}: $((SECONDS - STAGE_START))s"
    fi
    STAGE_NAME="$1"
    STAGE_START=$SECONDS
    [[ -n "$STAGE_NAME" ]] && echo "==> $STAGE_NAME"
    return 0
}

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"
source "$REPO_ROOT/scripts/lib/sce_generated_tree.sh"

# The generator, resolved by the first step that calls it (and built when the
# binary is stale); the steps that only run scripts let each script resolve it.
CODEGEN=""
require_codegen() {
    if [[ -z "$CODEGEN" ]]; then
        stage "Resolving sce-codegen (builds it when the binary is stale)"
        CODEGEN="$(sce_codegen_require "$REPO_ROOT")"
    fi
}

# The Go the steps that start go run on (`go-round-trip` builds its tree,
# `forge-goldens` runs the conformance test that builds and vets the Go it
# writes): the release the Go modules declare, first on PATH with
# GOTOOLCHAIN=local where it is installed, else the go on PATH, said either way
# (scripts/lib/go_toolchain.sh). Chosen once, by the first such step.
GO_CHOSEN=""
require_go() {
    if [[ -z "$GO_CHOSEN" ]]; then
        # shellcheck source=lib/go_toolchain.sh
        source "$REPO_ROOT/scripts/lib/go_toolchain.sh"
        sce_go_select regen || true
        GO_CHOSEN=1
    fi
}

# ── runtime-rust: the standard engine documents of the Rust runtime ─────
#
# A declared tree (tools/codegen/trees/runtime-rust.toml): every document of
# stdlib/engine generated for Rust into backends/rust/runtime/generated, and
# its module index. It stays committed because the generator is compiled from
# it (ADR 0097, amendment of 2026-11-17), so it is not set aside before the
# generator is resolved; `materialize --replace` removes, after the generator
# has run, a file no action writes any more.
step_runtime_rust() {
    require_codegen

    stage "Rust runtime's standard engine tree"
    "$CODEGEN" materialize runtime-rust --into "$REPO_ROOT" --replace
}

# ── runtime-kotlin / -go / -python / -swift: the other runtimes' trees ──
#
# Declared trees (tools/codegen/trees/runtime-<language>.toml): every document
# of stdlib/engine generated into the engine runtime of that language, which
# names the enum's declared names as the reasons it raises `error.*` for. No
# generator is compiled from them, so they are set aside and settled as every
# other tree is (`sce_generated_trees`). They stay committed until the landing
# that makes a runtime's package carry its generated sources (ADR 0097,
# amendment of 2026-11-17, "The other runtimes' engine trees").
step_runtime_kotlin() {
    require_codegen
    stage "Kotlin runtime's standard engine tree"
    "$CODEGEN" materialize runtime-kotlin --into "$REPO_ROOT" --replace
}

step_runtime_go() {
    require_codegen
    stage "Go runtime's standard engine tree"
    "$CODEGEN" materialize runtime-go --into "$REPO_ROOT" --replace
}

step_runtime_python() {
    require_codegen
    stage "Python runtime's standard engine tree"
    "$CODEGEN" materialize runtime-python --into "$REPO_ROOT" --replace
}

step_runtime_swift() {
    require_codegen
    stage "Swift runtime's standard engine tree"
    "$CODEGEN" materialize runtime-swift --into "$REPO_ROOT" --replace
}

# ── runtime-c: the C11 runtime's engine tree ────────────────────────────
#
# A declared tree (tools/codegen/trees/runtime-c.toml), the same as the four
# above, written under the runtime's public include directory, which every
# consumer of a generated machine names already (ADR 0097, amendment of
# 2026-11-17, "The C11 engine's tree").
step_runtime_c() {
    require_codegen
    stage "C11 runtime's standard engine tree"
    "$CODEGEN" materialize runtime-c --into "$REPO_ROOT" --replace
}

# ── rust-tests: the committed trees of backends/rust/tests ──────────────
#
# Every tree of the step is declared (ADR 0097): every document, flag and
# module index of them is in tools/codegen/trees/rust-w3c.toml,
# rust-integration.toml and rust-static-datamodel.toml, and `materialize`
# writes each as its actions produce it, rustfmt included, removing a
# generated file no action produces any more.
step_rust_tests() {
    require_codegen

    stage "W3C, integration and static datamodel Rust trees"
    "$CODEGEN" materialize rust-w3c --into "$REPO_ROOT" --replace
    "$CODEGEN" materialize rust-integration --into "$REPO_ROOT" --replace
    "$CODEGEN" materialize rust-static-datamodel --into "$REPO_ROOT" --replace
}

# ── kotlin-tests: the committed trees of the Kotlin test modules ────────
#
# The W3C suite is a module of its own, backends/kotlin/w3c, which nothing else
# writes into. The static datamodel's algorithms are packages under
# com/sce/generated of backends/kotlin/tests, and its machines have their own
# directory, com/sce/integration/static_datamodel, beside the integration stems.
#
# Every tree of the step is declared (ADR 0097): every document, flag and
# package of them is in tools/codegen/trees/kotlin-w3c.toml,
# kotlin-integration.toml and kotlin-static-datamodel.toml, and `materialize`
# writes each as its actions produce it, removing a generated file no action
# produces any more.
step_kotlin_tests() {
    require_codegen

    stage "W3C, integration and static datamodel Kotlin trees"
    "$CODEGEN" materialize kotlin-w3c --into "$REPO_ROOT" --replace
    "$CODEGEN" materialize kotlin-integration --into "$REPO_ROOT" --replace
    "$CODEGEN" materialize kotlin-static-datamodel --into "$REPO_ROOT" --replace
}

# ── kotlin-method-size: the module of the JVM method-size document ──────
#
# The document whose transition content, written as one method, passes the
# 65,535 bytes of code one JVM method holds sits beside the long-text documents
# for the same reason: the limit it reaches is the class file's. It lives in a
# module of its own (backends/kotlin/method-size), so it is a step of its own.
step_kotlin_method_size() {
    stage "Method-size document Kotlin tree"
    scripts/regen_jvm_method_size_kotlin.sh
}

# ── go-w3c / python-w3c: the gitignored W3C trees ──────────────────────
#
# The Go and Python W3C trees are .gitignored, so nothing committed changes
# when they are regenerated — but they are still real trees on a developer's
# disk, and leaving them out of the master refresh let them keep serving
# citations from templates that had already been corrected. A stale gitignored
# artifact is invisible to the drift gate and visible to every other check that
# walks the filesystem — and to the test suite: dropping a runtime symbol a
# template no longer emits left the stale Python tree importing it, failing
# collection for all 202 W3C modules at once.
step_go_w3c() {
    require_codegen
    stage "W3C Go tree"
    "$CODEGEN" generate-w3c -l go
}

step_python_w3c() {
    require_codegen
    stage "W3C Python tree"
    "$CODEGEN" generate-w3c -l python
}

# ── go-integration: backends/go/tests/integration ───────────────────────
#
# Both trees are declared (ADR 0097): every document, flag and package of them
# is in tools/codegen/trees/go-integration.toml and go-static-datamodel.toml,
# and `materialize` writes each as its actions produce it, removing a
# generated file no action produces any more.
step_go_integration() {
    require_codegen

    stage "Integration and static datamodel Go trees"
    "$CODEGEN" materialize go-integration --into "$REPO_ROOT" --replace
    "$CODEGEN" materialize go-static-datamodel --into "$REPO_ROOT" --replace
}

# ── python-integration: backends/python/tests/integration (gitignored) ─
step_python_integration() {
    require_codegen

    # Both trees are declared (ADR 0097) and not committed
    # (tools/codegen/trees/python-integration.toml, python-static-datamodel.toml),
    # so they contribute no diff here. They are written anyway: this script is
    # the one place that says "regenerate everything", and a local tree whose
    # Python module is missing fails at import rather than at an assertion.
    stage "Integration and static datamodel Python trees"
    "$CODEGEN" materialize python-integration --into "$REPO_ROOT" --replace
    "$CODEGEN" materialize python-static-datamodel --into "$REPO_ROOT" --replace
}

# ── swift-tests: the committed machines of backends/swift/tests ─────────
step_swift_tests() {
    require_codegen

    # Every tree of the step is declared (ADR 0097), a target each of the Swift
    # test package: the integration machines, the statecharts of
    # `sce-build/tests/fixtures/swift_statecharts`, the machines and units of the
    # static datamodel, and the `datamodel="ecmascript"` machines lowered to Lua,
    # the target that links the `SceLua` engine. `materialize` writes each as its
    # actions produce it, removing a generated file no action produces any more.
    stage "Integration, statechart, static datamodel and ECMAScript Swift trees"
    "$CODEGEN" materialize swift-integration --into "$REPO_ROOT" --replace
    "$CODEGEN" materialize swift-statecharts --into "$REPO_ROOT" --replace
    "$CODEGEN" materialize swift-static-datamodel --into "$REPO_ROOT" --replace
    "$CODEGEN" materialize swift-ecmascript --into "$REPO_ROOT" --replace
}

# ── go-round-trip: the forge round-trip Go codec tree ──────────────────
step_go_round_trip() {
    require_go
    stage "Forge round-trip Go codec tree"
    backends/go/forge-runtime/round_trip/generate.sh
}

# ── mesh-*: the Mesh trees, one step for each ──────────────────────────
#
# The Mesh trees are generated from the standard library's stdlib/mesh by
# their own scripts, and this script did not know about them — so a change
# to what a source-hash covers left all three stale with nothing here to
# restore them, and the C++ one sits outside `backends/`, where
# `regen-reproduces` does not even look. Named here so "regenerate
# everything" includes them (measured 2026-09-28: the source-hash fix that
# stopped folding the whole library into them regenerated every other tree
# and not these).
step_mesh_rust() {
    stage "Mesh Rust tree"
    backends/rust/mesh/generate.sh
}

step_mesh_cpp() {
    stage "Mesh C++ envelope tree"
    sce/src/mesh/generate_envelope.sh
}

step_mesh_kotlin() {
    stage "Mesh Kotlin tree"
    backends/kotlin/mesh/generate.sh
}

step_mesh_swift() {
    stage "Mesh Swift tree"
    backends/swift/mesh/generate.sh
}

step_mesh_swift_wss() {
    stage "Mesh Swift loopback peer table"
    backends/swift/mesh-wss/generate.sh
}

# ── specs: the specs crate's request machines ──────────────────────────
#
# The specs folder's request machine is generated from the document that holds the
# rules of a generation request, and it lives in the specs crate, outside
# `backends/`. Named here so that a change to the generator or to the document
# is carried into it by "regenerate everything", and so that `regen-reproduces`,
# which compares the whole tree after this script has run, judges it.
step_specs() {
    stage "Specs request machine"
    sce-specs/machines/generate.sh
}

# ── forge-goldens: tests/forge/expected ────────────────────────────────
#
# The forge conformance goldens are generated from the same templates as
# everything above, and this script did not know about them.
#
# Measured: an `#include` added to `state_machine.jinja2` left
# `tests/forge/expected/inline_mixed_sm.{h,inl}` stale. The full regeneration
# above rewrote 1593 files and not those two; `regen-reproduces` passed them
# as well, because neither reads that tree. The drift surfaced four gates
# later in `workspace-tests` — twenty minutes into a push, on a template edit
# made hours earlier.
#
# Regenerating them here is what makes this script's stated aim true for the
# whole tree rather than for the parts it happened to enumerate. The
# conformance test is the generator: with UPDATE_GOLDEN set it writes the
# expectation instead of asserting against it. It reads no tree another step
# writes: the runtimes it compiles against are written by hand, and of the Go
# forge runtime it imports packages, never `round_trip/generated`.
#
# ⚠ It writes only the goldens a test still reads. Those two files are gone:
# `c8d25c2f920` replaced the inline-kind goldens with a comparison against the
# standalone document path, after which nothing read or wrote them, and the
# sixteen `inline_*` files were deleted on 2026-09-28. A golden no test names
# is not refreshed by this step, so it goes stale without failing anything.
step_forge_goldens() {
    require_go
    stage "Regenerating the forge conformance goldens"
    UPDATE_GOLDEN=1 cargo test -p sce-build --features cli --test forge_conformance --quiet
}

# ── embed-manifest: embed/MANIFEST.json and its stamp ──────────────────
#
# The embed manifest is a committed generated file too: clang's reading of the
# public headers package_embed.sh ships. It was outside this script, so a
# header edit left it stale until the CI lane `embed-vendor` noticed after the
# push, and five of eight manifest commits in October and November 2026 were
# repairs made after the fact. Its stamp, embed/MANIFEST.inputs, records what
# it was emitted from, and the pre-commit and push gates refuse a commit whose
# inputs moved without it (scripts/embed_manifest.py).
#
# Packaged into a scratch directory, not over embed/: package_embed.sh starts
# by removing its output. The manifest's label fields (the HEAD of the emit and
# the compiler) change with every run, so where the rest of it is unchanged the
# committed bytes are kept, as every other step keeps a file it writes with
# the same content; the stamp holds no label and is taken as emitted.
#
# The C++ engines' standard engine tree (tools/codegen/trees/runtime-cpp.toml)
# is written first, by this step: it is a public header directory the static
# engine and the Interpreter include, so the manifest is read from it, and a
# step reads nothing another step writes (ADR 0097, amendment of 2026-11-17,
# "The C++ engines' tree").
step_embed_manifest() {
    local scratch
    require_codegen
    stage "C++ engines' standard engine tree"
    "$CODEGEN" materialize runtime-cpp --into "$REPO_ROOT" --replace

    stage "Emitting the embed manifest and its stamp (clang-19)"
    scratch="$(mktemp -d --suffix=.embed)"
    "$REPO_ROOT/scripts/package_embed.sh" -o "$scratch" --skip-manifest >/dev/null
    "$REPO_ROOT/scripts/emit_embed_manifest.sh" -d "$scratch"
    if [[ "$(python3 scripts/embed_manifest.py canon "$scratch/MANIFEST.json")" \
          != "$(python3 scripts/embed_manifest.py canon embed/MANIFEST.json)" ]]; then
        cp "$scratch/MANIFEST.json" embed/MANIFEST.json
    fi
    cp "$scratch/MANIFEST.inputs" embed/MANIFEST.inputs
    rm -rf "$scratch"
}

# The function that does the work of STEP.
step_function() {
    printf 'step_%s\n' "${1//-/_}"
}

# Run STEP: set its trees' generated files aside, do its work, and settle its
# trees. Every tree is REPLACED, not written over: the step has to write again
# what it still produces, and the settle removes -- and names -- each file it
# did not write again. A file it wrote with the same bytes gets its old copy
# back, mtime and all.
#
# Writing over a tree removes nothing. Measured 2026-10-21: after the escaping
# of synthesized invoke ids changed, this script wrote
# `test192__sce_synth_invoke__0zinvokedzuchild_sm.rs` (and its Kotlin twin)
# beside the `..._invokedChild_sm.rs` the generator no longer wrote, and both
# old files stayed committed for days. The rules for which file is generated,
# and the reason each tree belongs to the step it does, are in
# scripts/lib/sce_generated_tree.sh.
#
# Settled when the step ends, not when the run does: a removal is a change of
# that step's trees, and its bx action carries it back only as part of that
# step's result. A step that fails puts its files back (the EXIT trap), since a
# step that stopped half way says nothing about what the generator no longer
# writes; the steps before it stay settled, each having finished.
run_step() {
    local step="$1"
    local -a trees=()
    mapfile -t trees < <(sce_generated_trees "$step")
    echo "=== step $step"
    if (( ${#trees[@]} > 0 )); then
        stage "Setting aside the generated files of the trees of $step"
        sce_generated_tree_stash "${trees[@]}"
    fi
    "$(step_function "$step")"
    if (( ${#trees[@]} > 0 )); then
        stage "Settling the trees of $step: what it did not write again is removed"
        sce_generated_tree_settle 0
    fi
}

mapfile -t ALL_STEPS < <(sce_regen_steps)

# Every step the list names has its function here, and the other way round, so
# that a step added to one place and not the other is refused before any tree
# is touched rather than skipped.
for step in "${ALL_STEPS[@]}"; do
    if ! declare -F "$(step_function "$step")" >/dev/null; then
        echo "regen_all_committed_trees.sh: step '$step' has no $(step_function "$step") function" >&2
        exit 2
    fi
done
while read -r _ _ fn; do
    case "$fn" in
        step_function) continue ;;
        step_*) ;;
        *) continue ;;
    esac
    found=0
    for step in "${ALL_STEPS[@]}"; do
        if [[ "$(step_function "$step")" == "$fn" ]]; then
            found=1
        fi
    done
    if (( ! found )); then
        echo "regen_all_committed_trees.sh: $fn is no step of sce_regen_steps" >&2
        exit 2
    fi
done < <(declare -F)

if [[ "${1:-}" == "--list" ]]; then
    printf '%s\n' "${ALL_STEPS[@]}"
    exit 0
fi

if (( $# == 0 )); then
    STEPS=("${ALL_STEPS[@]}")
else
    STEPS=("$@")
    for step in "${STEPS[@]}"; do
        known=0
        for name in "${ALL_STEPS[@]}"; do
            if [[ "$name" == "$step" ]]; then
                known=1
            fi
        done
        if (( ! known )); then
            echo "regen_all_committed_trees.sh: no step named '$step' (the steps: ${ALL_STEPS[*]})" >&2
            exit 2
        fi
    done
fi

trap sce_generated_tree_on_exit EXIT
for step in "${STEPS[@]}"; do
    run_step "$step"
done

stage ""
echo "Regenerated the committed §6.2.6 trees of ${STEPS[*]} in ${SECONDS}s."
