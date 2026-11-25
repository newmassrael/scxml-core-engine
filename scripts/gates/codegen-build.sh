#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: none
#
# Build the generator every downstream gate executes.
#
# A prerequisite rather than a gate in its own right: `nostd-mcu`,
# `forge-go`, `forge-cpp` and `example-codegen` all run
# `target/debug/sce-codegen`, and declare it through `deps` in the registry
# so selecting any of them pulls this in.
#
# Debug profile. The generator's cost is process start-up and I/O, not
# optimisation — measured at 2.75s vs 2.89s over forty runs of the largest
# fixture, with the two profiles producing byte-identical output across all
# 1588 committed generated files. Building it in release compiled the entire
# dependency tree a second time, sharing nothing with the clippy or workspace
# test runs: 145.7s against 24.6s in a tree those gates had already warmed.
# `cargo test --release` in `forge-rust` is a different thing and stays —
# that one pins optimised floating-point output, which is what ships.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

cargo build --bin sce-codegen --features cli -p sce-build \
    || sce_gate_fail "sce-codegen build"

# The generator names no path of the machine that built it, so that two builds
# of one source in two accounts or two checkouts are the same bytes and a cache
# can hand one's output to the other (bx RFC 0084, 4.19: a unit is cut off by
# the digest of an action's output).
#
#   - Cargo's home appears nowhere in the file. Registry crates carried it in
#     every panic location and in their debug info until the gate library mapped
#     it (scripts/lib/reproducible_paths.sh).
#   - The checkout appears nowhere but in the debug info. The generator found
#     its templates, schemas and checkout from the path it was compiled in
#     until it searched from where it runs (sce-build/src/source_tree.rs). The
#     debug info keeps the checkout as its compilation directory, which no
#     stable flag maps without naming the checkout (the library says why), so
#     the check reads the file with its debug sections stripped.
#
# ELF only: `objcopy --strip-debug` is what separates the two readings, and the
# generator a cache keeps is built on Linux.
if [[ "$(uname -s)" != Linux ]]; then
    printf 'NOTE gate[%s]: the build-path check reads an ELF file; this is %s\n' \
        "$SCE_GATE_SLUG" "$(uname -s)" >&2
    exit 0
fi
[[ -n "${SCE_CARGO_HOME_MAPPED:-}" ]] \
    || sce_gate_fail "Cargo's home was not mapped for this build (scripts/lib/reproducible_paths.sh said why above), so sce-codegen carries it"
source "$SCE_REPO_ROOT/scripts/lib/sce_codegen.sh"
codegen="$(sce_codegen_path "$SCE_REPO_ROOT")" \
    || sce_gate_fail "the build left no sce-codegen to check"
command -v objcopy >/dev/null 2>&1 \
    || sce_gate_cannot_run "no objcopy (binutils) to strip the debug info of $codegen with"
stripped="$(mktemp)"
sce_gate_on_exit "rm -f '$stripped'"
objcopy --strip-debug "$codegen" "$stripped" \
    || sce_gate_cannot_run "objcopy could not strip the debug info of $codegen"

# Whether file $1 holds text $2: 0 when it does, 1 when it does not; a file
# that cannot be read ends the gate as one that could not run.
holds() {
    local rc=0
    grep -a -F -q -e "$2" -- "$1" || rc=$?
    ((rc <= 1)) || sce_gate_cannot_run "could not read $1"
    return "$rc"
}

found=0
IFS=: read -r -a cargo_homes <<<"$SCE_CARGO_HOME_MAPPED"
for home in "${cargo_homes[@]}"; do
    if holds "$codegen" "$home/"; then
        sce_gate_error "$codegen names Cargo's home $home/: a crate was compiled without the mapping of scripts/lib/reproducible_paths.sh"
        found=1
    fi
done
for root in "$SCE_REPO_ROOT" "$(pwd -P)"; do
    if holds "$stripped" "$root/"; then
        sce_gate_error "$codegen names the checkout $root/ outside its debug info: a path fixed at compile time (env!(\"CARGO_MANIFEST_DIR\"), file!() of a file outside the workspace, include!() of a build script's output) reached what it carries"
        found=1
    fi
done
((found == 0)) || exit 1
