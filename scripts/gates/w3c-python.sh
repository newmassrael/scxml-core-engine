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

LOG="$(mktemp -d)"
sce_gate_on_exit "rm -rf '$LOG'"

sce_gate_step "running the Python conformance suite"
# `-v` matches the lane, whose summary step counts the per-case ` PASSED`
# lines out of this log.
status=0
PYTHONPATH="$SCE_REPO_ROOT/backends/python/runtime${PYTHONPATH:+:$PYTHONPATH}" \
    python3 -m pytest backends/python/tests/generated/ backends/python/tests/integration/ \
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
