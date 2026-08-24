#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: w3c-tests.yml
#
# The Python AOT conformance arm: 202 W3C cases plus the integration fixtures,
# generated into a gitignored tree and run against the lupa-backed runtime.
#
# This is the family-member AOT channel (sce-codegen -> *_sm.py + lupa), not
# the pybind11 -> C++ Interpreter one, which is a separate job and a separate
# question.
#
# Two differences from the lane, both deliberate. The lane installs the
# runtime with `pip install -e`; a gate must not mutate the developer's
# environment, so the package is reached through PYTHONPATH instead — the same
# import either way, and `backends/python/tests/conftest.py` already shims
# sys.path for its own harness. And no fixture server is started here: that
# conftest spawns its own `http.server` on 8080 for the BasicHTTP fixtures, so
# starting a second listener would take the port from it.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# Not skip-capable, for the reason spelled out in `w3c-go`: this gate is
# selected when the Python backend changed, so a skip is silence about the
# change that asked for the check. Python itself is a hard dependency of the
# runner (`scripts/gate` shells out to python3 for the registry), so only the
# suite's own packages are worth reporting on.
python3 -c "import lupa, pytest" 2>/dev/null \
    || sce_gate_fail "the Python AOT suite needs lupa and pytest (python3 -m pip install lupa pytest)"

sce_gate_requires_free_http_port

CODEGEN="$(sce_gate_codegen)"

# Same pin as `w3c-go`, for the same reason: a generation that stamps
# wall-clock `generated-at` values leaves the drift suite reporting churn.
export SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-0}"

sce_gate_step "generating the Python W3C and integration suites"
"$CODEGEN" generate-w3c -l python >/dev/null \
    || sce_gate_fail "Python W3C generation"
"$CODEGEN" generate-integration -l python >/dev/null \
    || sce_gate_fail "Python integration generation"
# `generate-integration` enumerates `integration_resources/`, and one committed
# Python suite is not from there: `event_schema_native` compiles the
# EventSchema fixtures under `sce-build/tests/fixtures/event_schema/`, so its
# modules come from a per-stem script instead. Its TEST file is tracked while
# its modules are gitignored, so a checkout that runs only the two commands
# above has a test importing a module nothing produced. That is what CI hit —
# `ModuleNotFoundError: statechart_bytes_sm` — while this gate passed on any
# tree where the script had been run by hand at some point.
"$SCE_REPO_ROOT/scripts/regen_event_schema_native_python.sh" >/dev/null \
    || sce_gate_fail "Python event_schema_native generation"
# §scxml-6.2.5, and here for the same reason: the host-served fixture lives
# beside the build tests rather than under `integration_resources/`, and it
# needs a per-stem `--host-processor` flag the fan-out has no way to pass.
# Its test is tracked while its module is gitignored, so a checkout running
# only the commands above has a test importing a module nothing produced.
"$SCE_REPO_ROOT/scripts/regen_host_processor_python.sh" >/dev/null \
    || sce_gate_fail "Python host_processor generation"
# §scxml-G-7, and here for the same reason again: the `<sce:action>` fixture
# lives beside the build tests, its test is tracked while its module is
# gitignored, and `generate-integration` does not reach it.
"$SCE_REPO_ROOT/scripts/regen_native_action_python.sh" >/dev/null \
    || sce_gate_fail "Python native_action generation"
# The AI supervision loop, and here for the same reason a fourth time: its input
# is `examples/ai_loop/ai_loop.scxml`, a worked EXAMPLE rather than a stem under
# `integration_resources/`, so the fan-out does not enumerate it and it needs a
# per-stem `--host-processor` flag besides. Its test is tracked while its module
# is gitignored, so a checkout running only the commands above has a test
# importing a module nothing produced.
#
# `sce-build/tests/ai_loop_channel_parity.rs` holds this channel to the same 27
# scenarios as the C++, Rust, Go and Kotlin ones, so a missing module here is
# not a quiet gap: it is the one channel of five that could not answer.
"$SCE_REPO_ROOT/scripts/regen_ai_loop_python.sh" >/dev/null \
    || sce_gate_fail "Python ai_loop generation"

LOG="$(mktemp -d)"
sce_gate_on_exit "rm -rf '$LOG'"

sce_gate_step "running the Python conformance suite"
# `-v` matches the lane, whose summary step counts the per-case ` PASSED`
# lines out of this log.
status=0
# `ecmascript/` is the third directory: it holds the reader that measures this
# backend's Lua runtime against `tests/ecmascript/ecma262_semantics.json`,
# which is a different question from conformance — the W3C suite is green on a
# backend that answers `[1,2,3].indexOf(2)` with -1, because no fixture in it
# asks. Named here rather than left to collection so a directory that stops
# being collected shows up as a count, not as silence.
#
# `configuration_entry/` is the fourth, and the same reasoning: it drives
# §scxml-3.2 `Engine.enter_at` over documents that already exist in the tree
# rather than adding a fixture of its own, so it is not a stem and nothing else
# names it. A directory of witnesses no runner enumerates is a directory of
# files.
PYTHONPATH="$SCE_REPO_ROOT/backends/python/runtime${PYTHONPATH:+:$PYTHONPATH}" \
    python3 -m pytest backends/python/tests/generated/ backends/python/tests/integration/ \
        backends/python/tests/ecmascript/ backends/python/tests/configuration_entry/ \
        --no-header -v >"$LOG/pytest.log" 2>&1 || status=$?
cat "$LOG/pytest.log"

# pytest's summary line is the count. A collection error exits non-zero with
# no tests run, and a generator that emitted nothing would collect zero and
# exit 5 — both are failures, but only the floor says which.
passed="$(grep -oE '[0-9]+ passed' "$LOG/pytest.log" | tail -1 | grep -oE '[0-9]+' || true)"
passed="${passed:-0}"

if (( status != 0 )); then
    tail -n 30 "$LOG/pytest.log" >&2
    sce_gate_fail "Python conformance suite failed"
fi

if (( passed < 200 )); then
    tail -n 10 "$LOG/pytest.log" >&2
    sce_gate_fail "only $passed Python case(s) passed (expected at least 200) — the suite covered less than its name claims"
fi

sce_gate_step "$passed Python case(s) passed"

# The same generator, asked for a tree of the caller's own.
#
# Python names no suite — its wrappers import the machine beside them by path —
# so what makes an emitted tree standalone is the conftest, and the conftest
# used to reach the runtime by walking up from its own location. Outside this
# repository that walk lands nowhere, and every fixture fails on an import
# error naming neither cause. The run below is deliberately given no
# PYTHONPATH: if the emitted conftest does not find the runtime by itself,
# nothing else will.
sce_gate_step "running an emitted Python suite with no PYTHONPATH of its own"
SUITE="$(mktemp -d)"
sce_gate_on_exit "rm -rf '$SUITE'"

"$CODEGEN" generate-w3c -l python --output-dir "$SUITE" -t 144 >/dev/null \
    || sce_gate_fail "emitting a standalone Python suite"

suite_status=0
( cd "$SUITE/backends/python/tests" && env -u PYTHONPATH python3 -m pytest . --no-header -v ) \
    >"$LOG/pytest-suite.log" 2>&1 || suite_status=$?
if (( suite_status != 0 )); then
    cat "$LOG/pytest-suite.log" >&2
    sce_gate_fail "an emitted Python suite must pass from its own tree — that is what --output-dir claims"
fi

# One case, so "collected nothing" and "ran everything" differ by a single
# line; the floor is what tells them apart.
suite_passed="$(grep -oE '[0-9]+ passed' "$LOG/pytest-suite.log" | tail -1 | grep -oE '[0-9]+' || true)"
if (( ${suite_passed:-0} < 1 )); then
    cat "$LOG/pytest-suite.log" >&2
    sce_gate_fail "the emitted Python suite collected no cases"
fi

sce_gate_step "the emitted Python suite passed ${suite_passed} case(s) on its own"
