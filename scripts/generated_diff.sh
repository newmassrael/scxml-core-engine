#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The generated diff of a change (ADR 0097, section 6): every tree
# tools/codegen/trees/ declares, materialized at two commits, and what differs
# between the two, one patch per tree and a summary. A tree that leaves version
# control leaves the change's own diff too; this is where a reviewer reads it.
#
# Each commit is materialized by its own generator. An action is keyed by the
# generator that runs it, and `sce-codegen` refuses to run against a checkout
# whose sources it was not built from (`verify-generator`), so the base side
# builds the generator of the base. A side whose commit is this checkout's
# HEAD, with nothing modified or untracked, is materialized here and reuses the
# binary this checkout holds; any other side gets a worktree of its commit.
# The two sides share this machine's action cache, so an action the change did
# not move runs once.
#
# This renders; it judges nothing. It exits non-zero when it cannot produce the
# head side of a tree, since a tree its own commit cannot materialize is a
# defect the reviewer must see, and only notes a base side it cannot produce
# (an older commit may declare a tree the generator of its day could not
# materialize, or none at all).
#
# Usage (from anywhere in the checkout):
#   scripts/generated_diff.sh [--out <dir>] <base> <head> [<tree>...]
#
# With no tree named, every tree either commit declares. A tree one commit
# does not declare is empty on that side. Writes into <dir> (by default a new
# temporary directory, whose path is printed on stdout):
#   <tree>.patch   the difference, with paths relative to the checkout, so
#                  `git apply` takes it onto a checkout of the base
#   summary.md     per tree, the files changed and the lines added and
#                  removed, then the first lines of each tree's patch
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing),
#   and what the trees' generations need (rustfmt, gofmt, clang-format-19).

set -euo pipefail

usage() {
    echo "usage: scripts/generated_diff.sh [--out <dir>] <base> <head> [<tree>...]" >&2
    exit 2
}

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

source "$REPO_ROOT/scripts/lib/sce_codegen.sh"

OUT=""
if [[ "${1:-}" == "--out" ]]; then
    [[ $# -ge 2 ]] || usage
    OUT="$2"
    shift 2
fi
[[ $# -ge 2 ]] || usage

commit_of() {
    git rev-parse --verify --quiet "$1^{commit}" || {
        echo "error: $1 names no commit in this checkout" >&2
        exit 2
    }
}
BASE="$(commit_of "$1")"
HEAD_COMMIT="$(commit_of "$2")"
shift 2

WORK="$(mktemp -d)"
WORKTREES=()
cleanup() {
    local wt
    for wt in "${WORKTREES[@]}"; do
        git -C "$REPO_ROOT" worktree remove --force "$wt" >/dev/null 2>&1 || true
    done
    rm -rf "$WORK"
}
trap cleanup EXIT

# The checkout a side is materialized from, in SIDE_ROOT: this one when it
# holds the commit as committed, else a new worktree of the commit.
prepare_side() {
    local commit="$1" side="$2"
    if [[ "$(git rev-parse HEAD)" == "$commit" && -z "$(git status --porcelain)" ]]; then
        SIDE_ROOT="$REPO_ROOT"
        return
    fi
    SIDE_ROOT="$WORK/checkout-$side"
    git worktree add --detach --quiet "$SIDE_ROOT" "$commit"
    WORKTREES+=("$SIDE_ROOT")
}

# The trees a checkout declares, one name a line.
declared_trees() {
    local manifest
    for manifest in "$1"/tools/codegen/trees/*.toml; do
        [[ -f "$manifest" ]] || continue
        basename "$manifest" .toml
    done
}

prepare_side "$BASE" base
BASE_ROOT="$SIDE_ROOT"
prepare_side "$HEAD_COMMIT" head
HEAD_ROOT="$SIDE_ROOT"

# Made after the sides are prepared: an output directory inside this checkout
# would be an untracked file that turns HEAD into a side needing a worktree.
if [[ -z "$OUT" ]]; then
    OUT="$(mktemp -d)"
fi
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"

if [[ $# -gt 0 ]]; then
    TREES=("$@")
else
    mapfile -t TREES < <({ declared_trees "$BASE_ROOT"; declared_trees "$HEAD_ROOT"; } | sort -u)
fi
if [[ ${#TREES[@]} -eq 0 ]]; then
    echo "error: neither commit declares a tree under tools/codegen/trees" >&2
    exit 1
fi

# `materialize_side <checkout> <side>`: every tree of TREES the checkout
# declares, into $WORK/out/<side>/<tree>; a tree it cannot materialize is
# named in UNPRODUCED as `<side>:<tree>`, with its log beside the patches.
UNPRODUCED=()
materialize_side() {
    local root="$1" side="$2" codegen tree
    if ! codegen="$(sce_codegen_require "$root")"; then
        echo "error: no generator agrees with the $side checkout at $root" >&2
        exit 1
    fi
    for tree in "${TREES[@]}"; do
        mkdir -p "$WORK/out/$side/$tree"
        # A tree the commit does not declare is empty on its side.
        [[ -f "$root/tools/codegen/trees/$tree.toml" ]] || continue
        # `SCE_WORKSPACE_ROOT`, because the binary otherwise prefers the
        # checkout it was built in over the one it runs in, and a linked
        # worktree's `.git` is a file (scripts/gates/regen-reproduces.sh).
        if ! (cd "$root" && SCE_WORKSPACE_ROOT="$root" \
                "$codegen" materialize "$tree" --into "$WORK/out/$side/$tree") \
                >"$OUT/$side-$tree.log" 2>&1; then
            UNPRODUCED+=("$side:$tree")
            continue
        fi
        rm -f "$OUT/$side-$tree.log"
    done
}

materialize_side "$BASE_ROOT" base
materialize_side "$HEAD_ROOT" head

unproduced() {
    local entry
    for entry in "${UNPRODUCED[@]}"; do
        [[ "$entry" == "$1" ]] && return 0
    done
    return 1
}

SUMMARY="$OUT/summary.md"
{
    printf '## Generated diff\n\n'
    # The backticks are Markdown, not a command substitution.
    # shellcheck disable=SC2016
    printf 'Base `%s`, head `%s`.\n\n' "$BASE" "$HEAD_COMMIT"
    printf '| Tree | Files | Added | Removed |\n'
    printf '|---|---:|---:|---:|\n'
} >"$SUMMARY"

UNCHANGED=()
CHANGED=()
for tree in "${TREES[@]}"; do
    rm -f "$OUT/$tree.patch"
    if unproduced "head:$tree" || unproduced "base:$tree"; then
        continue
    fi
    # `git diff --no-index` exits 1 when the two sides differ, which is the
    # answer and not a failure; any other non-zero status is.
    status=0
    (cd "$WORK/out" && git diff --no-index --no-color --binary \
        -- "base/$tree" "head/$tree") >"$WORK/raw.patch" || status=$?
    if (( status > 1 )); then
        echo "error: could not compare the two sides of $tree" >&2
        exit 1
    fi
    if (( status == 0 )); then
        UNCHANGED+=("$tree")
        continue
    fi
    CHANGED+=("$tree")
    # Paths relative to the checkout: each side's directory is dropped from
    # the header lines, so the patch reads as a change to the repository.
    sed -E "/^(diff --git |--- |\+\+\+ |rename from |rename to )/ s#(^|[ ])([ab]/)?(base|head)/$tree/#\1\2#g" \
        "$WORK/raw.patch" >"$OUT/$tree.patch"
    status=0
    (cd "$WORK/out" && git diff --no-index --numstat -- "base/$tree" "head/$tree") \
        >"$WORK/numstat" || status=$?
    if (( status > 1 )); then
        echo "error: could not count the difference of $tree" >&2
        exit 1
    fi
    # A binary file counts `-` lines, read as none.
    awk -v tree="$tree" -F '\t' '
        { files++; if ($1 != "-") added += $1; if ($2 != "-") removed += $2 }
        END { printf "| %s | %d | %d | %d |\n", tree, files, added, removed }
    ' "$WORK/numstat" >>"$SUMMARY"
done

{
    printf '\n'
    if [[ ${#UNCHANGED[@]} -gt 0 ]]; then
        printf 'No difference: %s.\n\n' "${UNCHANGED[*]}"
    fi
    for entry in "${UNPRODUCED[@]}"; do
        # shellcheck disable=SC2016
        printf 'Not materialized at the %s: `%s` (log: `%s-%s.log`).\n\n' \
            "${entry%%:*}" "${entry#*:}" "${entry%%:*}" "${entry#*:}"
    done
    for tree in "${CHANGED[@]}"; do
        printf '### %s\n\n' "$tree"
        printf '<details><summary>The first lines of %s.patch</summary>\n\n' "$tree"
        printf '````diff\n'
        head -n 60 "$OUT/$tree.patch"
        printf '````\n\n</details>\n\n'
    done
} >>"$SUMMARY"

printf '%s\n' "$OUT"

for entry in "${UNPRODUCED[@]}"; do
    if [[ "$entry" == head:* ]]; then
        echo "error: the head does not materialize ${entry#head:} (see $OUT/head-${entry#head:}.log)" >&2
        exit 1
    fi
done
