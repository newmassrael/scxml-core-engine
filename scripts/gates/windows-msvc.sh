#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: windows-msvc.yml
#
# The C++ client parts of SCE built with MSVC on Windows (docs/adr/0107, D11),
# through both build channels a C++ consumer has:
#
#   CMake   the main project configured with the Visual Studio 2022 generator,
#           the COSE operations required (SCE_HOST_COSE=ON, libsodium from
#           vcpkg), and standard_host_scenario_test built and run: the reference
#           host of the standard host features (its system clock among them),
#           its COSE operations on libsodium, and the generated C++ machines that
#           ask for them, linked against sce_base alone.
#   Bazel   scripts/smoke_bazel_consumer.sh, both channels, with the libsodium of
#           the same vcpkg named to the checkout's channel, so the COSE probe is
#           built and run through @sce//:sce_host_cose.
#
# Run in Git Bash on a Windows host with Visual Studio 2022 (MSVC, its CMake
# or another on PATH), vcpkg (VCPKG_ROOT) with libxml2 and libsodium installed for
# the triplet, Rust, and bazelisk as `bazel`. The generator links libxml2 from
# vcpkg (VCPKGRS_DYNAMIC=1) and its bindings are generated with the libclang
# LIBCLANG_PATH names, which defaults here to the LLVM Visual Studio carries.
# Anywhere else the gate cannot run, and says so (exit 3).
#
# Generated C++ is not formatted here (SCE_FORMAT_GENERATED=OFF): what this lane
# holds is that MSVC compiles it, and the formatter's pinned clang-format 19 and
# the bytes it writes are held by the Linux lanes.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

case "$(uname -s)" in
    MINGW* | MSYS* | CYGWIN*) ;;
    *) sce_gate_cannot_run "it builds with MSVC, on a Windows host under Git Bash; this host is $(uname -s)" ;;
esac

[[ -n "${VCPKG_ROOT:-}" && -d "${VCPKG_ROOT}" ]] ||
    sce_gate_cannot_run "VCPKG_ROOT names no vcpkg checkout (got '${VCPKG_ROOT:-}')"
VCPKG_ROOT="$(cygpath -m "$VCPKG_ROOT")"
export VCPKG_ROOT
TRIPLET="${SCE_VCPKG_TRIPLET:-x64-windows}"
VCPKG_PREFIX="$VCPKG_ROOT/installed/$TRIPLET"
[[ -f "$VCPKG_PREFIX/include/sodium.h" ]] ||
    sce_gate_cannot_run "libsodium is not installed for $TRIPLET under $VCPKG_ROOT (vcpkg install libsodium:$TRIPLET)"
for tool in cmake ctest cargo bazel; do
    command -v "$tool" >/dev/null 2>&1 || sce_gate_cannot_run "no $tool on PATH"
done

# The DLLs of the vcpkg installation (libxml2 and what it links, libsodium) are
# found by the programs that load them through PATH: the generator, which the
# configure and the build run, and the test.
PATH="$(cygpath -u "$VCPKG_PREFIX/bin"):$PATH"
export PATH
export VCPKGRS_DYNAMIC=1

if [[ -z "${LIBCLANG_PATH:-}" ]]; then
    vswhere="$(cygpath -u "${ProgramFiles:-C:\\Program Files} (x86)")/Microsoft Visual Studio/Installer/vswhere.exe"
    [[ -x "$vswhere" ]] || sce_gate_cannot_run "LIBCLANG_PATH is unset and there is no vswhere at $vswhere"
    vs_install="$("$vswhere" -latest -products '*' -property installationPath | tr -d '\r')"
    LIBCLANG_PATH="$(cygpath -m "$vs_install")/VC/Tools/Llvm/x64/bin"
fi
[[ -f "$(cygpath -u "$LIBCLANG_PATH")/libclang.dll" ]] ||
    sce_gate_cannot_run "no libclang.dll in LIBCLANG_PATH ($LIBCLANG_PATH): install Visual Studio's C++ Clang tools"
export LIBCLANG_PATH

# The scripts this gate runs name the interpreter `python3`, which a Windows
# Python installs as `python`. A shim in a scratch directory names it so.
if ! python3 -c 'import sys' >/dev/null 2>&1; then
    command -v python >/dev/null 2>&1 || sce_gate_cannot_run "no python3 and no python on PATH"
    python_shim="$(mktemp -d)"
    sce_gate_on_exit "rm -rf '$python_shim'"
    printf '#!/usr/bin/env bash\nexec python "$@"\n' >"$python_shim/python3"
    chmod +x "$python_shim/python3"
    PATH="$python_shim:$PATH"
    export PATH
fi

BUILD_DIR=build_msvc
CONFIG=Debug

sce_gate_step "sce-codegen, the generator the C++ build runs"
cargo build --bin sce-codegen --features cli -p sce-build ||
    sce_gate_fail "sce-codegen did not build"

sce_gate_step "configure $BUILD_DIR: Visual Studio 2022, x64, the vcpkg toolchain for $TRIPLET"
cmake -S . -B "$BUILD_DIR" -G "Visual Studio 17 2022" -A x64 \
    -DCMAKE_TOOLCHAIN_FILE="$VCPKG_ROOT/scripts/buildsystems/vcpkg.cmake" \
    -DVCPKG_TARGET_TRIPLET="$TRIPLET" \
    -DSCE_HOST_COSE=ON \
    -DSCE_FORMAT_GENERATED=OFF \
    -DBUILD_EXAMPLES=OFF ||
    sce_gate_fail "the configure with MSVC failed"

sce_gate_step "build standard_host_scenario_test ($CONFIG)"
sce_gate_build "$BUILD_DIR" --config "$CONFIG" --target standard_host_scenario_test ||
    sce_gate_fail "standard_host_scenario_test did not build with MSVC"

sce_gate_step "run standard_host_scenario"
# Every test of the tree requires the fixture `sce_built`, whose setup
# (`sce_build_is_current`) builds every target of the tree before any test
# runs. This lane holds the targets ADR 0107 D11 names and builds exactly the
# one it runs, above; the rest of the tree (the vendored Lua and QuickJS
# under the project's warning policy, the POSIX-only Mesh transports) is not
# built on Windows yet, so the setup is excluded rather than left to fail on
# what this lane does not hold.
sce_gate_tested -- ctest --test-dir "$BUILD_DIR" -C "$CONFIG" -R '^standard_host_scenario$' \
    --fixture-exclude-setup sce_built --output-on-failure --no-tests=error ||
    sce_gate_fail "standard_host_scenario failed under MSVC"

sce_gate_step "Bazel consumers through both channels, the COSE probe on vcpkg's libsodium"
bazel_rc=0
SCE_BAZEL_SODIUM_PREFIX="$VCPKG_PREFIX" bash scripts/smoke_bazel_consumer.sh || bazel_rc=$?
case "$bazel_rc" in
    0) ;;
    3) sce_gate_cannot_run "the Bazel smoke found no bazel to run" ;;
    *) sce_gate_fail "the Bazel consumer smoke failed with MSVC" ;;
esac
