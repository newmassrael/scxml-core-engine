#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: w3c-tests.yml
#
# The pybind11 channel: the same 202 W3C fixtures driven through the Python
# bindings into the C++ Interpreter. Distinct from `w3c-python`, which is the
# AOT family member (sce-codegen -> *_sm.py + lupa) — this one exercises the
# wrapper layer, and a defect in that layer shows up in neither the C++ suite
# nor the AOT one.
#
# The trigger is the binding sources rather than the whole engine on purpose.
# The engine underneath is judged 404 cases at a time by `w3c-cpp`; what only
# this gate can see is the wrapper.
#
# Skip-capable, and here the pairing is the real one: the extension module
# needs Python's development headers, which are an apt package a developer may
# not have (measured on 2026-08-11: configure fails at `find_package(Python3
# ... Development)` on a machine with Python 3.12 and no python3-dev). The
# lane installs python3-dev and sets SCE_REQUIRE_TOOLS, so the skip that keeps
# a local suite running is a failure where the check is claimed to have run.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

sce_gate_requires_tool python3-config python3-dev || exit 0

sce_gate_requires_free_http_port

BUILD_DIR="${SCE_PYBIND_BUILD_DIR:-build_python}"

sce_gate_step "building the pybind11 extension module"
cmake -B "$BUILD_DIR" \
      -DBUILD_PYTHON_BINDINGS=ON \
      -DCMAKE_BUILD_TYPE=Release \
      -DBUILD_TESTS=OFF \
      -DBUILD_EXAMPLES=OFF \
      -G Ninja -Wno-dev >/dev/null \
    || sce_gate_fail "python bindings configure"
cmake --build "$BUILD_DIR" --target _sce --parallel "$(nproc)" >/dev/null \
    || sce_gate_fail "python bindings build"

LOG="$(mktemp -d)"
sce_gate_on_exit "rm -rf '$LOG'"

sce_gate_step "running the W3C fixtures through the bindings"
status=0
SPDLOG_LEVEL=off \
PYTHONPATH="$BUILD_DIR/sce-python:backends/python/bindings/python" \
    python3 backends/python/bindings/tests/test_w3c.py >"$LOG/bindings.log" 2>&1 || status=$?
cat "$LOG/bindings.log"

if (( status != 0 )); then
    tail -n 30 "$LOG/bindings.log" >&2
    sce_gate_fail "the pybind11 W3C suite failed"
fi

# The harness prints `PASS: <n>/<total>`. A run that collected nothing prints
# no such line, which the status alone would read as success.
passed="$(grep -oE 'PASS: [0-9]+' "$LOG/bindings.log" | tail -1 | grep -oE '[0-9]+' || true)"
passed="${passed:-0}"
if (( passed < 200 )); then
    tail -n 10 "$LOG/bindings.log" >&2
    sce_gate_fail "only $passed binding case(s) passed (expected at least 200) — the suite covered less than its name claims"
fi

sce_gate_step "$passed binding case(s) passed"
