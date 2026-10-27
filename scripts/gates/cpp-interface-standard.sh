#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: cpp-interface-standard.yml
#
# SCE's C++ interface compiles at C++17 and at C++20 (ADR 0059, section 7).
#
# The interface is what a consumer compiles: every public header, and every
# C++ file a generator writes, which lands in the consumer's own translation
# units at the consumer's standard. SCE's own build compiles all of it at
# C++20, so the C++17 the targets declare was a claim nothing had compiled. A
# downstream build at -std=c++17 found that out under GCC 15, whose libstdc++
# stopped providing <cstdint> through <string> at C++17.
#
# The measurement, and why it has this shape:
#
#   1. The tool's own cases, then `guards`: no public header and no C++
#      template may pick a definition by `__cplusplus` or a `__cpp_*` macro,
#      because one program holds translation units of both standards.
#   2. A scratch configure with tests, mesh and the GLib dispatcher on, so the
#      corpora the C++ suites generate are all in the build graph.
#   3. The generation rules and nothing else: the generated headers the graph
#      names, and the few tools their rules need. Building the tree would
#      compile everything, and on 2026-10-16 that ran a machine out of memory.
#   4. `jobs` pairs every header with the flags of a translation unit that
#      reaches it, and `run` compiles each one ALONE, with -Werror, at C++17
#      and C++20. Without -Werror, GCC accepts a designated initializer at
#      C++17 as an extension and only warns.
#
# What it reports as NOT MEASURED, by name, rather than as passed: a header no
# translation unit of this configure reaches, and a header whose compile needs
# a file this machine does not have (the mesh SDKs on a runner without them).
# A floor per kind fails the run when the count measured is too small to be
# evidence, so a blind include reader cannot end in a quiet green.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

TOOL="tools/interface-standard/interface_standard.py"

sce_gate_step "the measuring tool's own cases"
python3 -m unittest discover -s tools/interface-standard/tests -q \
    || sce_gate_fail "the interface-standard tool failed its own cases, so its verdict cannot be read"

sce_gate_step "no standard-selecting guard in the public headers and the C++ templates"
python3 "$TOOL" guards \
    || sce_gate_fail "the interface selects a definition by language standard (ADR 0059, section 3); the directives are listed above"

# Generation-only needs the build graph's list of outputs, which Ninja answers
# directly. Falling back to building the whole tree is the thing this gate is
# shaped to avoid, so a machine without Ninja cannot run it.
command -v ninja >/dev/null 2>&1 \
    || sce_gate_cannot_run "ninja is not installed (apt install ninja-build); running only the generation rules reads Ninja's build graph"

CODEGEN="$(sce_gate_codegen)" \
    || sce_gate_fail "could not provide sce-codegen, which the configure and every generation rule need"

WORK="$(mktemp -d)"
sce_gate_on_exit "rm -rf '$WORK'"
# The root CMakeLists refuses a binary directory not named `build` or `build_*`.
BUILD_DIR="$WORK/build_interface"

sce_gate_step "configuring $BUILD_DIR (tests, mesh, GLib dispatcher)"
if ! cmake -S . -B "$BUILD_DIR" -G Ninja \
        -DCMAKE_EXPORT_COMPILE_COMMANDS=ON \
        -DBUILD_TESTS=ON -DSCE_ENABLE_MESH=ON -DSCE_DISPATCHER_GLIB=ON \
        -DSCE_CODEGEN="$CODEGEN" \
        -Wno-dev >"$WORK/configure.log" 2>&1; then
    tail -60 "$WORK/configure.log" >&2
    sce_gate_fail "configure"
fi

# The optional transports register their targets only when `find_package`
# finds them. Said up front, so the not-measured list that follows has a cause
# a reader can see. The `_DIR` entries are what those same calls left behind.
absent=()
for pkg in vsomeip3 zenohcxx CycloneDDS; do
    if grep -q "^${pkg}_DIR:PATH=.*-NOTFOUND$" "$BUILD_DIR/CMakeCache.txt"; then
        absent+=("$pkg")
    fi
done
if (( ${#absent[@]} > 0 )); then
    sce_gate_step "not on this machine: ${absent[*]}; headers that need them are reported as not measured"
fi

sce_gate_step "running the generation rules"
ninja -C "$BUILD_DIR" -t targets all >"$WORK/targets.txt" \
    || sce_gate_fail "could not read the build graph"
python3 "$TOOL" generation-outputs --ninja-targets "$WORK/targets.txt" \
        --build-dir "$BUILD_DIR" --out "$WORK/generation.txt" \
    || sce_gate_fail "could not list the generated headers in the build graph"
mapfile -t OUTPUTS <"$WORK/generation.txt"
sce_gate_build "$BUILD_DIR" --target "${OUTPUTS[@]}" \
    || sce_gate_fail "generation (${#OUTPUTS[@]} output(s))"

sce_gate_step "pairing every header with the flags it is compiled with"
python3 "$TOOL" jobs --compile-commands "$BUILD_DIR/compile_commands.json" \
        --build-dir "$BUILD_DIR" --out "$WORK/jobs.json" \
    || sce_gate_fail "could not build the compile jobs"

sce_gate_step "compiling the interface alone at C++17 and at C++20"
python3 "$TOOL" run --jobs "$WORK/jobs.json" --std c++17 --std c++20 \
        --parallel "$(sce_build_jobs)" \
    || sce_gate_fail "the interface does not hold at C++17 and C++20; the failures and floors are listed above"
