#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Which files of a generated tree the generator wrote, and how a regeneration
# replaces them, so that a file the generator no longer writes leaves the tree
# with the regeneration that stopped writing it.
#
# A regeneration that only writes what the generator produces now never removes
# what it produced before. Measured 2026-10-21: when the escaping of a
# synthesized invoke id changed (`invokedChild` became `0zinvokedzuchild`),
# regenerating the W3C trees wrote `test192__sce_synth_invoke__0zinvokedzuchild_sm.rs`
# and its Kotlin twin beside the old `..._invokedChild_sm.rs` and
# `...invokedChildSm.kt`. Both old files stayed committed for days.
# `regen-reproduces` compared the files a run wrote and could not see one it no
# longer wrote; what finally refused them was a sourcemap test reading their
# markers.
#
# So a regeneration REPLACES a tree's generated set instead of writing over it,
# in one of two shapes:
#
#   * A script that generates into a fresh directory and copies the result in
#     calls `sce_generated_tree_clear` on the directory it copies into, just
#     before the copy. That replaced a `find -name <pattern> -delete` in every
#     regen script: a pattern names the files of the day, and a transition
#     function, a sourcemap or a child the pattern did not name stayed behind.
#   * A run that writes its trees in place -- `sce-codegen generate-w3c`, and
#     `regen_all_committed_trees.sh` as a whole -- sets the generated files of
#     its trees aside with `sce_generated_tree_stash` first and ends with
#     `sce_generated_tree_settle`. A file the run wrote again with the same
#     bytes gets its old copy back, so its mtime still says when its bytes last
#     changed (the reason the generator leaves an unchanged file alone); a file
#     it rewrote keeps the new bytes; a file it did not write again is gone,
#     and the settle names it. A run that FAILED proves nothing about what it
#     no longer writes, so then every file is put back as it was.
#
# A file is generated when it says so, never when its name looks generated, so a
# hand-written test, harness or build file beside generated ones is not touched.
# What says so, by what each producer writes (`sce_generated_file`):
#
#   * the drift header's banner on the first line, behind the language's
#     comment prefix: `// SCE-GENERATED <dash> DO NOT EDIT`, `# ...` for Python
#     (`HEADER_BANNER` in sce-build/src/forge/drift.rs). The generator puts it
#     on every `.rs .kt .go .py .h .c .cpp` it writes;
#   * `GENERATED -- DO NOT EDIT` on the first line: the module index each Rust
#     regen script writes for its directory, and the generator's own aggregator
#     and test files that carry no drift header;
#   * `sce_sourcemap.json` holding a `"source_hash"` member: the sidecar the
#     generator writes beside every machine directory, which as JSON cannot
#     carry a comment header and states its provenance in that member instead;
#   * a `.scxml` that is exactly the stub the generator writes for a hybrid
#     invoke's child under its own name (`HybridInvokeInfo::stub_document` in
#     sce-build/src/model.rs). The stub is a document the generator reads back,
#     so it carries no header either.
#
# `a_regeneration_removes_what_it_no_longer_writes` (sce-build/tests) holds each
# rule to what the generator writes and holds every file of the committed W3C
# trees to being recognised by one of them.

# Whether FILE is generator output, by the rules above.
sce_generated_file() {
    local file="$1" first="" name stem content
    [[ -f "$file" && ! -L "$file" ]] || return 1
    IFS= read -r first < "$file" || true
    case "$first" in
        *"SCE-GENERATED "*"DO NOT EDIT"* | *"GENERATED -- DO NOT EDIT"*)
            return 0
            ;;
    esac
    name="${file##*/}"
    case "$name" in
        sce_sourcemap.json)
            grep -q '"source_hash"' -- "$file"
            return
            ;;
        *.scxml)
            stem="${name%.scxml}"
            content="$(<"$file")"
            [[ "$content" == "<?xml version=\"1.0\"?>
<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" name=\"$stem\" initial=\"final\" version=\"1.0\">
  <final id=\"final\"/>
</scxml>" ]]
            return
            ;;
    esac
    return 1
}

# Print every generated file under each ROOT, one per line, in a stable order;
# with --shallow, only the files directly inside each ROOT. A ROOT that does not
# exist holds none.
sce_generated_tree_files() {
    local depth=() root file
    if [[ "${1:-}" == "--shallow" ]]; then
        depth=(-maxdepth 1)
        shift
    fi
    for root in "$@"; do
        [[ -d "$root" ]] || continue
        while IFS= read -r -d '' file; do
            if sce_generated_file "$file"; then
                printf '%s\n' "$file"
            fi
        done < <(find "$root" -mindepth 1 ${depth[@]+"${depth[@]}"} -type f -print0 | LC_ALL=C sort -z)
    done
}

# Remove every generated file directly inside each DIR; with --recursive, every
# one at any depth, and then each directory that leaves empty. The
# fresh-directory shape: called on the directory a script is about to copy a
# fresh generation into, so that the directory ends up holding what this run
# produced and what was written by hand, and nothing an earlier run produced.
sce_generated_tree_clear() {
    local scope=--shallow dir file
    if [[ "${1:-}" == "--recursive" ]]; then
        scope=""
        shift
    fi
    for dir in "$@"; do
        while IFS= read -r file; do
            rm -f -- "$file"
        done < <(sce_generated_tree_files $scope "$dir")
        if [[ -z "$scope" && -d "$dir" ]]; then
            find "$dir" -mindepth 1 -type d -empty -delete
        fi
    done
}

# The trees `regen_all_committed_trees.sh` regenerates whole, relative to the
# repository root, one per line. Every generated file under them is set aside
# before the first stage and has to be written again by one of the stages, so a
# tree belongs here only when every file of it that is generated is written by
# a stage of that script.
#
# `backends/python/tests/integration` is the tree that is NOT here, for that
# reason: four of its directories are written by scripts the w3c-python gate
# runs and the umbrella does not (host_processor, native_action,
# route_rests_on_a_question, static_datamodel). Its modules are gitignored, so
# nothing committed is at stake; each of its regen scripts clears its own
# directory before it copies.
#
# Every tree is a declared output of `[actions.regen]` in bx.toml, which is what
# lets a removal made on a build machine come back to the tree that asked for it
# (`a_regeneration_removes_what_it_no_longer_writes` holds the two together).
sce_generated_trees() {
    local tree
    for tree in \
        backends/rust/tests/src/generated \
        backends/rust/tests/src/integration \
        backends/rust/tests/tests \
        backends/kotlin/tests/src/main/kotlin/com/sce/generated \
        backends/kotlin/tests/src/main/kotlin/com/sce/integration \
        backends/kotlin/tests/src/test/kotlin/com/sce/w3c \
        backends/go/tests/generated \
        backends/go/tests/integration \
        backends/python/tests/generated \
        backends/go/forge-runtime/round_trip/generated \
        backends/rust/mesh/src/generated \
        backends/rust/mesh/examples/generated \
        backends/kotlin/mesh/src/commonMain/kotlin/generated \
        backends/kotlin/mesh/src/jvmTest/kotlin/com/sce/generated \
        sce/src/mesh/generated \
        sce-specs/src/*/generated; do
        printf '%s\n' "$tree"
    done
}

# Where `sce_generated_tree_stash` set the files aside, and the trees it took
# them from. Empty when no stash is open. Kept when this file is sourced a
# second time, so a second `source` cannot drop an open stash on the floor.
SCE_GENERATED_TREE_STASH="${SCE_GENERATED_TREE_STASH:-}"
if [[ -z "$SCE_GENERATED_TREE_STASH" ]]; then
    SCE_GENERATED_TREE_ROOTS=()
fi

# Move every generated file under each ROOT aside, into a directory outside the
# repository, until `sce_generated_tree_settle` judges what the run wrote.
#
# Outside, because a directory inside the tree would read as an untracked
# change to a run that compares the tree with git, and a build machine running
# the regen action refuses a change outside the action's declared outputs.
sce_generated_tree_stash() {
    local stash file parent made="" count=0
    if [[ -n "$SCE_GENERATED_TREE_STASH" ]]; then
        echo "sce_generated_tree_stash: a stash is already open at $SCE_GENERATED_TREE_STASH" >&2
        return 1
    fi
    stash="$(mktemp -d "${TMPDIR:-/tmp}/sce-generated-tree.XXXXXX")" || return 1
    : > "$stash/list"
    # Recorded before the first move, so that a stash that fails half way is
    # settled -- put back -- like one that completed.
    SCE_GENERATED_TREE_STASH="$stash"
    SCE_GENERATED_TREE_ROOTS=("$@")
    while IFS= read -r file; do
        # Two ROOTs that overlap list a file twice; the first move took it.
        [[ -e "$file" ]] || continue
        parent="${file%/*}"
        if [[ "$parent" != "$made" ]]; then
            mkdir -p -- "$stash/tree/$parent" || return 1
            made="$parent"
        fi
        mv -- "$file" "$stash/tree/$file" || return 1
        printf '%s\0' "$file" >> "$stash/list"
        count=$((count + 1))
    done < <(sce_generated_tree_files "$@")
    echo "sce_generated_tree: set aside $count generated file(s); the run has to write each again" >&2
}

# Judge the files `sce_generated_tree_stash` set aside against what the run
# left, given the run's exit STATUS. Prints, on stderr, each generated file the
# run did not write again; it is not put back. A failed run (STATUS other than
# 0) gets every file put back instead.
#
# Returns non-zero only when a file could not be put back; the stash is then
# kept, and its path printed, so that nothing set aside is lost.
sce_generated_tree_settle() {
    local status="${1:-0}" stash="$SCE_GENERATED_TREE_STASH" file kept root failed=0
    local -a unproduced=()
    [[ -n "$stash" ]] || return 0
    while IFS= read -r -d '' file; do
        kept="$stash/tree/$file"
        if [[ "$status" != 0 ]]; then
            if ! { mkdir -p -- "${file%/*}" && mv -f -- "$kept" "$file"; }; then
                failed=1
            fi
        elif [[ -e "$file" ]]; then
            # Written again. The same bytes get the old file back, mtime and
            # all; new bytes stay, and the old copy goes with the stash.
            if cmp -s -- "$kept" "$file" && ! mv -f -- "$kept" "$file"; then
                failed=1
            fi
        else
            unproduced+=("$file")
        fi
    done < "$stash/list"
    if [[ "$status" == 0 ]]; then
        if (( ${#unproduced[@]} > 0 )); then
            echo "sce_generated_tree: removed ${#unproduced[@]} generated file(s) the run did not write again:" >&2
            printf '  %s\n' "${unproduced[@]}" >&2
        fi
        for root in ${SCE_GENERATED_TREE_ROOTS[@]+"${SCE_GENERATED_TREE_ROOTS[@]}"}; do
            if [[ -d "$root" ]]; then
                find "$root" -mindepth 1 -type d -empty -delete 2>/dev/null || true
            fi
        done
    else
        echo "sce_generated_tree: the run failed (status $status); every generated file set aside was put back" >&2
    fi
    SCE_GENERATED_TREE_STASH=""
    SCE_GENERATED_TREE_ROOTS=()
    if (( failed )); then
        echo "sce_generated_tree: a file could not be put back; everything set aside is kept at $stash" >&2
        return 1
    fi
    rm -rf -- "$stash"
}

# For `trap sce_generated_tree_on_exit EXIT` after a stash: settle with the
# status the script is exiting with, and exit with it -- or with 1 when the
# settle itself failed, so a tree left half put back is not reported as done.
sce_generated_tree_on_exit() {
    local status=$?
    if ! sce_generated_tree_settle "$status" && (( status == 0 )); then
        status=1
    fi
    exit "$status"
}

# The tracked files missing from the work tree at DIR (default: the current
# directory). Run after a regeneration in a clean checkout, these are the
# committed generated files the regeneration no longer produces, which it
# therefore removed; `regen-reproduces` fails naming each one.
sce_generated_tree_unproduced() {
    git -C "${1:-.}" ls-files --deleted
}
