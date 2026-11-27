#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
# conftest spawns its own `http.server` for the BasicHTTP fixtures, in the
# pytest process, on a port the kernel assigns unless SCE_W3C_HTTP_PORT names
# one. It needs no port free before it starts, so this gate asks for none.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# Not skip-capable, for the reason spelled out in `w3c-go`: this gate is
# selected when the Python backend changed, so a skip is silence about the
# change that asked for the check. Python itself is a hard dependency of the
# runner (`scripts/gate` shells out to python3 for the registry), so only the
# suite's own packages are worth reporting on.
#
# The suite runs on the repository's Python environment where there is one
# (`sce_gate_python`). `import lupa` is not the question: the runtime imports
# `lupa.lua54`, which lupa ships from 2.0 on, and a precondition that asked only
# for `lupa` passed on a build machine whose python3-lupa 1.14.1 has none, after
# which every test that imports the runtime failed at collection (2026-11-08).
sce_gate_python
sce_gate_python_requires_lupa "the Python AOT suite"
sce_gate_python_requires "the Python AOT suite" pytest

CODEGEN="$(sce_gate_codegen)"

sce_gate_step "generating the Python W3C and integration suites"
"$CODEGEN" generate-w3c -l python >/dev/null \
    || sce_gate_fail "Python W3C generation"
# The integration machines and the `datamodel="sce-static"` machines (SCE
# Accepted Subset §2.15) are two declared trees (ADR 0097): every document they
# are generated from, every stem of `integration_resources/` and every
# document beside the build tests, an example or `tests/integration/`, with
# the host surfaces each declares, is in tools/codegen/trees/
# python-integration.toml and python-static-datamodel.toml. Their tests are
# tracked while their modules are gitignored, so a module neither tree wrote
# would be a test importing a module nothing produced: what CI once hit as
# `ModuleNotFoundError: statechart_bytes_sm`. `--replace` removes a module the
# tree no longer writes, which a test could otherwise still import.
#
# `sce-build/tests/ai_loop_channel_parity.rs` holds the AI loop channel to the
# same 27 scenarios as the C++, Rust, Go and Kotlin ones, so a missing module
# there is not a quiet gap: it is the one channel that could not answer.
for python_tree in python-integration python-static-datamodel; do
    "$CODEGEN" materialize "$python_tree" --into "$SCE_REPO_ROOT" --replace >/dev/null \
        || sce_gate_fail "materializing the $python_tree tree"
done

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
#
# `microstep/` is the fifth, for the same reason again: it holds the runtime's
# Appendix D transcription to hand-worked answers over a document written out
# by hand — the answers the Rust and C++ transcriptions are held to — and no
# document drives it.
#
# `json_text/` and `durations/` are the sixth and seventh: each reads a table
# every engine is held to (`tests/json_text/string_escape.json`,
# `tests/durations/css2_time.json`), and a table no runner reads here holds
# this backend to nothing. `json_text/` sat outside this list from the day it
# was written, 2026-09-28, until it was named.
#
# `mesh/` is the eighth, for the same reason: it reads
# `tests/mesh/mesh_target_cases.json`, the Mesh-peer predicate every engine
# and the build are held to (SCE_MESH.md §mesh-19).
#
# `scheduler/` is the ninth: it holds the delayed-send scheduler's `<cancel
# sendid>` to the answers the Rust and Go schedulers give, over no document.
# A cancel used to be remembered against its id and dropped the next send that
# carried it (W3C SCXML 6.3), which no fixture in the suite re-arms a timer
# under the same id to ask.
#
# `io_processors/` is the tenth: it holds the SCXML processor's address codec
# (`scxml_location` and its inverse) and the reading of a stored origin to the
# answers the C++ `IOProcessorHelper` gives (docs/adr/0090, test 6), over no
# document.
#
# `testprofile/` is the eleventh: it holds the test profile's runtime
# (`sce_runtime.testprofile`, docs/SCE_TEST_PROFILE.md §12.2) to the cases of
# the Rust one, over components written by hand, which no document drives.
#
# `trace_port/` is the twelfth: it holds the runtime's trace port to the run
# trace format (docs/adr/0115, D4 and D8) over no document, and the runs of the
# W3C documents `docs/formal` has models of, which it drives from the machines
# the generation above writes, to the recorded runs `docs/formal/traces.json`
# declares. It is no fixture stem either.
sce_gate_tested --log "$LOG/pytest.log" \
    -- env PYTHONPATH="$SCE_REPO_ROOT/backends/python/runtime${PYTHONPATH:+:$PYTHONPATH}" \
    "$SCE_GATE_PYTHON" -m pytest backends/python/tests/generated/ backends/python/tests/integration/ \
        backends/python/tests/ecmascript/ backends/python/tests/configuration_entry/ \
        backends/python/tests/microstep/ backends/python/tests/json_text/ \
        backends/python/tests/durations/ backends/python/tests/mesh/ \
        backends/python/tests/scheduler/ backends/python/tests/io_processors/ \
        backends/python/tests/testprofile/ backends/python/tests/trace_port/ \
        --no-header -v || status=$?

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
( cd "$SUITE/backends/python/tests" && env -u PYTHONPATH "$SCE_GATE_PYTHON" -m pytest . --no-header -v ) \
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
