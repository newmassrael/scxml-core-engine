# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `w3c-c11`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# Its script runs one shard of the suite when SCE_W3C_C11_SHARD names
# `k/N`, as w3c-tests.yml's three jobs do (scripts/lib/ctest_shards.py plans
# the part and builds only what its tests run). No `shard_env`: the build
# machines run it whole, as they run `cpp-suite`.
{
    "workflows": ["w3c-tests.yml"],
    "builds": ["main-tree"],
    "runner_workflow": True,
    "narrows": "same catch-all as `w3c-cpp`, same reason. This arm's real "
               "inputs are the C backend, the C templates, the "
               "generator, and the inbox probe it shares with the C++ "
               "engine's inbox test (docs/adr/0084).",
    # The shared Lua the machines embed, and the shared tables every Lua
    # engine's readers ask (`system_variable_writes.json` among them).
    "extra": ["backends/c/**", "tools/codegen/templates/**",
              # What spreads the suite over the lane's shards and the
              # reading of the tree it plans from, which the gate script
              # runs.
              "scripts/lib/ctest_shards.py",
              "scripts/lib/ctest_registered_commands.cmake",
              "sce-build/src/**", "tests/integration/inbox_probe.scxml",
              "sce/include/scripting/*.lua", "tests/ecmascript/**",
              # The registry the route registers every fixture from, the
              # documents it generates, and the modules it registers and
              # plans them through.
              "tests/w3c/conformance/fixtures.json", "resources/**",
              "cmake/SCEW3CRegistry.cmake", "cmake/SCEStaticW3CTest.cmake",
              "cmake/SCECodegenPlan.cmake",
              # The test log every runtime writes of the generated suite,
              # which `test_a_test_suite_runs_through_its_runner.c` reads
              # at run time, and the wire schema whose status
              # `test_case_run_test.c` holds the runtime's to.
              "sce-build/tests/fixtures/test_profile/runtime/expected_logs/**",
              "schemas/sce-test-log.v1.schema.json",
              # The suite's committed traces, which the same test and the
              # runner program's replay, and the trace's wire schema,
              # whose status `input_trace_test.c` holds the runtime's to,
              # all read at run time.
              "sce-build/tests/fixtures/test_profile/runtime/traces/**",
              "schemas/sce-test-trace.v1.schema.json",
              # The documents of the corpus's boundary_suite, which the
              # build generates, and the traces and expected logs of the
              # corpus's other suites, which
              # `test_a_corpus_suite_replays_to_its_expected_logs.c`
              # replays and reads at run time.
              "sce-build/tests/fixtures/test_profile/boundary/**",
              "sce-build/tests/fixtures/test_profile/runs/**",
              # The documents of job_suite the build generates, and the
              # job's completion of the kind examples its port sends.
              "sce-build/tests/fixtures/test_profile/*.scxml",
              "sce-build/kind-examples/event-schema.scxml",
              # The documents of orders_suite and nested_suite, the records
              # and templates over a union they import, and the unions.
              "sce-build/tests/fixtures/ttcn3_export/**",
              "sce-build/tests/fixtures/test_profile/imports/**",
              "sce-build/tests/fixtures/union/**",
              # The configurations of every form the build generates,
              # whose readers and writers
              # `test_a_record_reads_back_the_form_it_writes.c` round-trips,
              # and the reader of a received message's every part the
              # build generates and
              # `test_a_received_message_is_made_in_the_machines_types.c`
              # runs.
              "sce-build/tests/fixtures/test_profile/trace_forms/**",
              "sce-build/tests/fixtures/test_profile/received/**"],
    "deps": ["codegen-build"],
    "ci_only": "391s inside a push, measured 2026-08-23 — and that number "
               "is the point. The same 245 tests read 85s and 95s run on "
               "their own, 258s and 391s inside the push that also builds "
               "the generator and runs three other suites. Four readings "
               "spanning 4.6x is not a cost that can be declared, and no "
               "value fits a 300s ceiling that this gate alone can exceed. "
               "It kept its place through two re-prices before the numbers "
               "settled the argument; the owner's rule decides it — the "
               "slow ones go to CI so a push stays under five minutes. "
               "`w3c-tests.yml` has no `paths:` filter, so it runs on every "
               "push and nothing narrows what CI sees.",
    "cost_s": 258,
    "summary": "W3C conformance, C11 MCU backend",
    # It runs in the host image wherever it runs (ADR 0145): the runner runs
    # `scripts/gate` inside the image its Containerfile addresses, so the
    # hosted lane, a build machine and a workstation judge it in one
    # environment, and no lane installs a package for it. A machine takes its
    # unit only where docker runs (`needs`).
    "container": "tools/ci-images/host/Containerfile",
    "needs": ["docker"],
    # What its lane sets for it, handed into the image (`container_env`):
    # the part of the suite a lane job runs.
    "container_env": ["SCE_W3C_C11_SHARD"],
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
