#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Cargo's home kept out of what a Rust build writes (reproducible-builds.org,
# "build path"). Sourced; prints nothing on stdout.
#
# WHAT IT FIXES. Cargo hands rustc the sources of a registry or git dependency
# by their absolute path under Cargo's home, so every panic location of such a
# crate (`unwrap`, `expect`, an index out of bounds) and its debug info carry
# `$HOME/.cargo/registry/src/...`, and the C those crates compile through the
# cc crate carries it in its assert messages and debug info. Measured on an
# sce-codegen built on a build machine: 1033 strings naming
# `/home/bxfleet/.cargo/registry/src/`. Two builds of one source by two
# accounts then differ in their bytes, and no cache can hand one's output to
# the other. Each path under Cargo's home is named by its path under it
# instead (`registry/src/index.crates.io-.../serde-1.0.228/src/de.rs`), as
# bx names it for a rustc call it runs remotely (bx RFC 0045, 4.5), so the two
# agree.
#
# WHY HERE AND NOT IN .cargo/config.toml. A config file holds literal strings,
# and the prefix to map is this machine's. Cargo's own remedy, the `trim-paths`
# profile option (RFC 3127), maps it without naming it, and is unstable in the
# release rust-toolchain.toml pins (`-Z trim-paths`). So the builds this
# repository drives source this file: every gate (scripts/gates/lib.sh) and
# every script that builds the generator (scripts/lib/sce_codegen.sh). Cargo's
# home is the same for every checkout of one account, so the flag is too, and a
# build cache shared by the checkouts of one machine stays valid for all of
# them.
#
# WHY NOT THE CHECKOUT AS WELL. Cargo already names the sources of the
# workspace's own packages relative to its root, and sce-codegen compiles in no
# path of its checkout (sce-build/src/source_tree.rs). What still names the
# checkout is the compilation directory of the debug info, which a flag could
# map only by naming the checkout: a flag that differs between two checkouts
# makes Cargo rebuild everything in the second (its fingerprint holds the
# flags), and a build cache shared by tree copies at different paths would
# never be fresh. The remedies that map it without naming it are unstable
# (`-Z remap-cwd-prefix`, `trim-paths`). The C and C++ of the CMake tree are
# mapped by the root CMakeLists.txt, which configures one build tree for one
# source tree.
#
# HOW.
#   - rustc: `--remap-path-prefix=<cargo home>/=`, in scope `object` (macro
#     expansions, panic locations among them, and debug info), so a compiler
#     message still names the file a reader opens. Given as
#     CARGO_BUILD_RUSTFLAGS, which is `build.rustflags`: Cargo joins it with the
#     array of a config file or a `--config` (the loom gates' `--cfg loom`),
#     where RUSTFLAGS would replace them. A target's own rustflags
#     (thumbv7em-none-eabihf in .cargo/config.toml) and a caller's RUSTFLAGS
#     still take precedence over it, as they do over the config file.
#     `--remap-path-scope` is stable since Rust 1.95; the one gate that builds
#     with an older release (`rust-version`) sets RUSTFLAGS.
#   - C and C++ a build script compiles through the cc crate:
#     `-ffile-prefix-map=<cargo home>/=` in HOST_CFLAGS and TARGET_CFLAGS (and
#     the CXXFLAGS pair), which cc adds to CFLAGS for a build for the host and
#     for a cross build. CMake reads neither.
#   - Not on Windows: the shell spells a path there as `/c/...` and Cargo as
#     `C:\...`, so no prefix would match, and MSVC has no prefix map.
#
# SCE_CARGO_HOME_MAPPED names the prefixes once they are mapped, so that a
# second source changes nothing and a check can ask whether they were.

sce_map_cargo_home() {
    [[ -z "${SCE_CARGO_HOME_MAPPED:-}" ]] || return 0
    case "$(uname -s)" in
        MINGW* | MSYS* | CYGWIN*) return 0 ;;
    esac
    local home="${CARGO_HOME:-${HOME:-}/.cargo}"
    home="${home%/}"
    [[ -n "$home" ]] || return 0
    local -a prefixes=("$home")
    local physical
    if physical="$(cd "$home" 2>/dev/null && pwd -P)" && [[ "$physical" != "$home" ]]; then
        prefixes+=("$physical")
    fi
    local prefix rust="" c="" mapped=""
    for prefix in "${prefixes[@]}"; do
        case "$prefix" in
            *[[:space:]]*)
                # CARGO_BUILD_RUSTFLAGS and CFLAGS are split on white space.
                printf "warning: Cargo's home '%s' holds white space, which a flag of the environment cannot carry; builds keep its path\n" \
                    "$prefix" >&2
                continue
                ;;
        esac
        rust+=" --remap-path-prefix=$prefix/="
        c+=" -ffile-prefix-map=$prefix/="
        mapped+="${mapped:+:}$prefix"
    done
    [[ -n "$mapped" ]] || return 0
    rust+=" --remap-path-scope=object"
    export CARGO_BUILD_RUSTFLAGS="${CARGO_BUILD_RUSTFLAGS:+$CARGO_BUILD_RUSTFLAGS }${rust# }"
    local var
    for var in HOST_CFLAGS TARGET_CFLAGS HOST_CXXFLAGS TARGET_CXXFLAGS; do
        export "$var=${!var:+${!var} }${c# }"
    done
    export SCE_CARGO_HOME_MAPPED="$mapped"
}

sce_map_cargo_home
