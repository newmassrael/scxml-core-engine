# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `w3c-python`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
{
    "workflows": ["w3c-tests.yml"],
    "runner_workflow": True,
    "narrows": "same catch-all as `w3c-cpp`, same reason. This arm reads "
               "the Python backend, the Python templates and the "
               "generator.",
    # The shared Lua the engine loads, and the shared tables its readers
    # ask. The installer of the repository's Python environment, whose
    # revision names the interpreter the suite runs on wherever that
    # environment was built (`sce_gate_python` in scripts/gates/lib.sh).
    "extra": ["backends/python/**", "tools/codegen/templates/**",
              "sce-build/src/**",
              "sce/include/scripting/*.lua", "tests/ecmascript/**",
              "scripts/install_sce_python.sh",
              # The test log every runtime writes of the generated suite,
              # which `test_a_test_suite_runs_through_its_runner.py` reads
              # at run time.
              "sce-build/tests/fixtures/test_profile/runtime/expected_logs/**",
              # The suite's committed traces, which the same test replays,
              # read at run time.
              "sce-build/tests/fixtures/test_profile/runtime/traces/**",
              # The trace's wire schema, whose status
              # `test_a_recorded_trace_is_read_and_replayed.py` holds the
              # runtime's to, read at run time.
              "schemas/sce-test-trace.v1.schema.json",
              # The documents of the corpus's boundary_suite, which the
              # gate's regeneration writes the modules of (they are not
              # committed), and the traces and expected logs of the
              # corpus's other suites, which
              # `test_a_corpus_suite_replays_to_its_expected_logs.py`
              # replays and reads at run time.
              "sce-build/tests/fixtures/test_profile/boundary/**",
              "sce-build/tests/fixtures/test_profile/runs/**",
              # The documents of job_suite the same regeneration writes,
              # and the job's completion of the kind examples its port
              # sends.
              "sce-build/tests/fixtures/test_profile/*.scxml",
              "sce-build/kind-examples/event-schema.scxml",
              # The documents of orders_suite and nested_suite, the records
              # and templates over a union they import, and the unions.
              "sce-build/tests/fixtures/ttcn3_export/**",
              "sce-build/tests/fixtures/test_profile/imports/**",
              "sce-build/tests/fixtures/union/**"],
    "deps": ["codegen-build"],
    "ci_only": "72s. w3c-tests.yml is unfiltered, so CI runs this arm on "
               "every push.",
    "cost_s": 72,
    "summary": "W3C conformance, Python AOT",
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 1,
    "parallelism": PARALLELISM_SCRIPT,
}
