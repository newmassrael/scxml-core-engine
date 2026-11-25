# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `cpp-suite`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The other half of the main tree's ctest partition, and the half nothing
# ran. Measured 2026-08-12 with a logging shim in place of `ctest` during a
# full `scripts/gate --all`: 28 gates passed and the only runs against this
# build were `w3c-c11`'s two. 159 of 382 registered cases — every `mesh_*`
# case and every C++ unit suite — were executed by no gate and by no
# workflow, since none configures the main tree.
{
    "workflows": ["cpp-suite.yml"],
    "builds": ["main-tree"],
    "runner_workflow": True,
    "narrows": "the lane declares no `paths:` ALLOW filter, for the same "
               "reason `w3c-cpp`'s does not: that is a statement about a "
               "runner paid by the minute, not about what the suite "
               "reads. This one builds the whole C++ tree, so its real "
               "inputs are the engine, its tests, the mesh sources and "
               "the CMake definitions that register them. Since "
               "2026-08-29 it DOES declare a `paths-ignore:` deny list "
               "(docs/**, **/*.md, .claude/**), which narrows nothing "
               "this table claims: a deny list still starts the lane for "
               "every path it does not name, so the catch-all reading "
               "above holds. What the deny list changes is per-change "
               "delegation, and `ci_owed` is where that is read.",
    # The transport SDKs' pin comes first: the build machines install them
    # with that script (`[platform.linux.tools.mesh-transports]`). Then
    # what spreads the suite over the lane's shards (`SCE_CTEST_SHARD`)
    # and the reading of the tree it plans from, which the gate script
    # runs, and the report of where the build's time went.
    "extra": ["scripts/install_mesh_transports.sh",
              "scripts/lib/ctest_shards.py",
              "scripts/lib/ctest_registered_commands.cmake",
              "scripts/lib/build_time_report.py",
              "sce/**", "tests/**", "cmake/**", "CMakeLists.txt",
              "examples/**", "resources/**", "tools/codegen/templates/**",
              "sce-build/src/**",
              # The union documents every algorithm lane is held to, which
              # the Interpreter's lowered-algorithm test lowers and runs
              # (AnAlgorithmRunsLoweredUnderTheInterpreterTest.cpp).
              "sce-build/tests/fixtures/union/**",
              # The algorithms the catalog names in the statechart
              # fixtures (a map an algorithm is lent, docs/adr/0108 step
              # 7b), which the same test lowers and runs, and the
              # statecharts AStaticDatamodelRunsLoweredUnderTheInterpreter
              # Test lowers beside them.
              "sce-build/tests/fixtures/static_datamodel/**",
              # The test profile's documents the build generates the
              # suites from, their traces and their expected logs, which
              # ATestSuiteRunsEndToEndTest and
              # ACorpusSuiteReplaysToItsExpectedLogsTest read at run time.
              "sce-build/tests/fixtures/test_profile/**",
              # The documents of the corpus's orders_suite and
              # nested_suite, which the build generates.
              "sce-build/tests/fixtures/ttcn3_export/**"],
    "deps": ["codegen-build"],
    "ci_only": "110s building and running the whole C++ ctest tree. "
               "cpp-suite.yml runs it. Its `extra` list is wide on "
               "purpose — sce/**, tests/**, examples/** — so it was "
               "selected by most pushes, which is what made it a budget "
               "problem rather than an occasional one.",
    "cost_s": 110,
    "summary": "C++ ctest suite (engine + mesh), the non-c11 half",
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
