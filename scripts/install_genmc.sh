#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Install the pinned GenMC where this repository looks for it.
#
# GenMC is the memory-model checker the queue kind's layer 3 names for the C11
# and C++ runtimes (SCE Protocol-Synthesis RFC, section 5.P, Verification):
# it compiles a C/C++ program to LLVM IR and explores every execution the
# C11 memory model allows, so a `memory_order` weakened below what the
# algorithm needs shows up as a failed assertion instead of a rare loss on one
# machine. Nothing else in the tree answers that question for C and C++.
#
# The unit is the REVISION: `GENMC_REV` below is the one home of the pin, and
# the install path derives from it, so a host with a different revision is a
# host with no install at all rather than one the check mistakes for current.
#
# Idempotent: a binary of the pinned revision that answers `--version` is left
# alone, so a cache restore makes this a no-op rather than a rebuild.
#
# What it needs and does not install (an installer that also reaches for root
# is a second thing to get wrong): a C++20 compiler (g++ 14 or newer), clang
# and llvm development files of a major GenMC supports (19 or newer), cmake,
# and libffi, zlib and libedit development files. On Ubuntu 24.04:
#
#   sudo apt-get install -y g++-14 clang-19 llvm-19-dev cmake \
#       libffi-dev zlib1g-dev libedit-dev
#
# ⚠ THE BUILD MACHINES NEED IT TOO, and `bx` cannot tell a host that lacks it
# from one that has it. Run it once on each host:
#
#   bx --host <alias> -- scripts/install_genmc.sh
#
# and a pin bump is that many installs again. The queue checker
# (`scripts/gates/genmc-queue.sh`) looks for the binary at the path derived
# here and says how to install it when it is not there.
set -euo pipefail

# The pin. v0.19.0 (2026-09-24) supports LLVM 19 through 22.
GENMC_REV="v0.19.0"
GENMC_URL="https://github.com/MPI-SWS/genmc"
LLVM_MAJOR="${GENMC_LLVM_MAJOR:-19}"

GENMC_ROOT="${GENMC_ROOT:-$HOME/.local/genmc/$GENMC_REV}"
GENMC_BIN="$GENMC_ROOT/bin/genmc"

if [[ -x "$GENMC_BIN" ]] && "$GENMC_BIN" --version >/dev/null 2>&1; then
    echo "genmc $GENMC_REV is already installed at $GENMC_BIN"
    exit 0
fi

CXX_COMPILER="${GENMC_CXX:-g++-14}"
for tool in "$CXX_COMPILER" "clang-$LLVM_MAJOR" cmake; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "install_genmc: '$tool' is not installed; see the header of this script for the packages" >&2
        exit 2
    }
done
LLVM_DIR="/usr/lib/llvm-$LLVM_MAJOR/lib/cmake/llvm"
[[ -d "$LLVM_DIR" ]] || {
    echo "install_genmc: $LLVM_DIR is missing; install llvm-$LLVM_MAJOR-dev" >&2
    exit 2
}

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "building genmc $GENMC_REV (LLVM $LLVM_MAJOR) in $WORK"
git clone --quiet --depth 1 --branch "$GENMC_REV" "$GENMC_URL" "$WORK/genmc"
cmake -S "$WORK/genmc" -B "$WORK/build" -G Ninja \
    -DCMAKE_BUILD_TYPE=RelWithDebInfo \
    -DCMAKE_CXX_COMPILER="$CXX_COMPILER" \
    -DLLVM_DIR="$LLVM_DIR" \
    -DCMAKE_INSTALL_PREFIX="$GENMC_ROOT" >/dev/null
cmake --build "$WORK/build"
cmake --install "$WORK/build" >/dev/null

"$GENMC_BIN" --version
echo "genmc $GENMC_REV installed at $GENMC_BIN"
