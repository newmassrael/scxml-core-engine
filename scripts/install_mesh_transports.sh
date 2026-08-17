#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Build and install the three mesh transport SDKs `tests/CMakeLists.txt` gates
# its transport suites on: vsomeip3, zenohcxx and CycloneDDS.
#
# `find_package(<pkg> QUIET)` is what registers those suites, so a machine
# without the SDKs registers a smaller suite — and `scripts/gates/cpp-suite.sh`
# refuses to run there rather than report green over the remainder. The CI
# runner was such a machine: 130 non-c11 cases against a floor of 140, exit 3
# on every push, nothing claimed about the tree either way. A developer's
# workstation registers 178 because these three were installed there by hand,
# with no record of how. This script is that record.
#
# The versions are the ones a workstation that registers 178 carries, checked
# against upstream release tags. They are pinned rather than tracked: a
# transport SDK that moves under a conformance suite changes what the suite
# measures without anything in this repository changing.
#
# Idempotent, and cheap to re-run: each package is skipped when its CMake
# package file is already present under the prefix, so a restored cache makes
# this a no-op instead of a rebuild.
#
# Usage:
#   scripts/install_mesh_transports.sh [prefix]
#
# `prefix` defaults to /usr/local, which is where `find_package` looks without
# help. A developer who cannot write there can pass ~/.local and export
# CMAKE_PREFIX_PATH, which is how the zenoh half is installed on the
# workstation this was read from.

set -euo pipefail

PREFIX="${1:-/usr/local}"
JOBS="$(nproc 2>/dev/null || echo 2)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

CYCLONEDDS_TAG="11.0.1"
CYCLONEDDS_CXX_TAG="11.0.1"
VSOMEIP_TAG="3.7.3"
ZENOH_C_TAG="1.5.0"
ZENOH_CPP_TAG="1.5.0"

# The prefix is created before anything asks whether it is writable, because
# `-w` on a path that does not exist is false — which made every install
# escalate to `sudo` and leave a root-owned tree behind, including under a
# home directory the caller owns. A cache of that tree is one the restoring
# job cannot write into.
mkdir -p "$PREFIX" 2>/dev/null || sudo mkdir -p "$PREFIX"

# `sudo` only when the prefix is not writable: a system prefix needs it, a
# prefix under the caller's home does not, and a container already running as
# root has no sudo to call.
as_owner() {
    if [[ -w "$PREFIX" ]]; then
        "$@"
    else
        sudo "$@"
    fi
}

have_package() {
    local name="$1"
    [[ -f "$PREFIX/lib/cmake/$name/${name}Config.cmake" ]] \
        || [[ -f "$PREFIX/lib/cmake/$name/${name}-config.cmake" ]]
}

cmake_build_install() {
    local src="$1"
    shift
    cmake -S "$src" -B "$src/build" -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_INSTALL_PREFIX="$PREFIX" "$@"
    cmake --build "$src/build" --parallel "$JOBS"
    as_owner cmake --install "$src/build"
}

clone() {
    git clone --depth 1 --branch "$2" "https://github.com/$1" "$WORK/$3"
}

# ── CycloneDDS, then its C++ binding ──────────────────────────────
#
# The C++ binding is a separate repository that finds the C one, so the order
# is not a preference.
if have_package CycloneDDS; then
    echo "install_mesh_transports: CycloneDDS already at $PREFIX"
else
    clone eclipse-cyclonedds/cyclonedds "$CYCLONEDDS_TAG" cyclonedds
    cmake_build_install "$WORK/cyclonedds" -DBUILD_EXAMPLES=OFF -DBUILD_TESTING=OFF
fi

if have_package CycloneDDS-CXX; then
    echo "install_mesh_transports: CycloneDDS-CXX already at $PREFIX"
else
    clone eclipse-cyclonedds/cyclonedds-cxx "$CYCLONEDDS_CXX_TAG" cyclonedds-cxx
    cmake_build_install "$WORK/cyclonedds-cxx" \
        -DBUILD_EXAMPLES=OFF -DBUILD_TESTING=OFF \
        -DCMAKE_PREFIX_PATH="$PREFIX"
fi

# ── vsomeip3 ──────────────────────────────────────────────────────
#
# Needs Boost (system, thread, filesystem); the caller installs it, because a
# script that apt-installs behind a developer's back is not one they can read
# and repeat.
if have_package vsomeip3; then
    echo "install_mesh_transports: vsomeip3 already at $PREFIX"
else
    clone COVESA/vsomeip "$VSOMEIP_TAG" vsomeip
    cmake_build_install "$WORK/vsomeip" -DENABLE_SIGNAL_HANDLING=1
fi

# ── zenoh-c, then zenoh-cpp ───────────────────────────────────────
#
# zenoh-c is a Rust build with a CMake wrapper, and zenoh-cpp is headers that
# find it — so the same ordering reason applies, and the toolchain the caller
# already has for this repository is the one it uses.
if have_package zenohc; then
    echo "install_mesh_transports: zenoh-c already at $PREFIX"
else
    clone eclipse-zenoh/zenoh-c "$ZENOH_C_TAG" zenoh-c
    cmake_build_install "$WORK/zenoh-c" -DZENOHC_BUILD_WITH_UNSTABLE_API=TRUE
fi

if have_package zenohcxx; then
    echo "install_mesh_transports: zenohcxx already at $PREFIX"
else
    clone eclipse-zenoh/zenoh-cpp "$ZENOH_CPP_TAG" zenoh-cpp
    cmake_build_install "$WORK/zenoh-cpp" \
        -DZENOHCXX_ZENOHC=ON -DZENOHCXX_ZENOHPICO=OFF \
        -DCMAKE_PREFIX_PATH="$PREFIX"
fi

echo "install_mesh_transports: installed under $PREFIX"
ls "$PREFIX/lib/cmake" | grep -iE "cyclone|vsomeip|zenoh" || true
