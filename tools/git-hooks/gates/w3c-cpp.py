# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `w3c-cpp`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The two conformance surfaces nothing local ran. Both narrow their
# workflow's catch-all (see `narrows` in `gate_triggers`) and state their
# real inputs here: the engine, the generator, the templates that emit the
# AOT runners, and the fixtures the runners are registered from.
#
# The Rust arm of the same workflow was left out here once, on the grounds
# that `sce-rust-tests` is a workspace member and `workspace-tests` already
# runs its cases. That run is not the lane's: it takes the committed tree
# in the test profile, while the lane regenerates and runs in release. So
# the lane kept its own inline spelling, and it is `w3c-rust` now.
#
# Its script runs one shard of the suite when SCE_W3C_CPP_SHARD names `k/N`,
# as w3c-tests.yml's six jobs do (scripts/lib/ctest_shards.py plans the part
# and builds only the W3C programs its tests run). No `shard_env`: the build
# machines run it whole, as they run `cpp-suite`.
{
    "workflows": ["w3c-tests.yml"],
    "builds": ["main-tree"],
    "runner_workflow": True,
    "narrows": "the lane declares no `paths:` filter, which is a statement "
               "about a runner paid by the minute, not about what the "
               "suite reads. Inheriting it would run the longest gate in "
               "the set for a README edit.",
    "extra": ["sce/**", "tests/w3c/**", "tests/CMakeLists.txt",
              "resources/**", "tools/codegen/templates/**",
              "sce-build/src/**",
              # The modules the route registers and plans its fixtures
              # through, and the one the test tree takes the warning
              # classes its test programs turn off from.
              "cmake/SCEW3CRegistry.cmake", "cmake/SCEStaticW3CTest.cmake",
              "cmake/SCECodegenPlan.cmake", "cmake/SCETestWarnings.cmake",
              # What spreads the suite over the lane's shards and the
              # reading of the tree it plans from, which the gate script
              # runs, and the mock the runner core compiles.
              "scripts/lib/ctest_shards.py",
              "scripts/lib/ctest_registered_commands.cmake",
              "tests/mocks/**",
              # The recorded runs of the C++ lane, which the
              # `cpp_a_run_of_test_*_is_the_recorded_run_of_its_model`
              # programs hold their runs to at run time (docs/adr/0115, D4
              # and D8).
              "docs/formal/traces/**",
              # The turnstile of the properties judge's fixture, which the
              # build generates, and its scenarios with the C++ lane's
              # recorded runs of them, which
              # `cpp_a_scenario_of_the_turnstile_is_its_run` replays and
              # holds its runs to at run time (docs/adr/0115, D15).
              "sce-build/tests/fixtures/formal_properties/**"],
    "deps": ["codegen-build"],
    "ci_only": "55s, and it is the gate that decides whether the budget "
               "is met on a good run or on every run. Without it the set "
               "measured 286s against a 300s ceiling, and the same gates "
               "measured twice varied by as much as 27s (clippy alone "
               "swung 15s to 42s), so fourteen seconds of headroom is "
               "none. w3c-tests.yml declares no `paths:` filter, so CI "
               "runs this arm on every push regardless.",
    "cost_s": 55,
    "summary": "W3C conformance, C++ Interpreter + AOT",
    # It runs in the host image wherever it runs (ADR 0145): the runner runs
    # `scripts/gate` inside the image its Containerfile addresses, so the
    # hosted lane, a build machine and a workstation judge it in one
    # environment, and no lane installs a package for it. A machine takes its
    # unit only where docker runs (`needs`).
    "container": "tools/ci-images/host/Containerfile",
    "needs": ["docker"],
    # What its lane sets for it, handed into the image (`container_env`):
    # the part a lane job runs, the log level of the runner, and where
    # its report goes.
    "container_env": ["SCE_W3C_CPP_SHARD", "SPDLOG_LEVEL", "SCE_W3C_REPORT_DIR"],
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
