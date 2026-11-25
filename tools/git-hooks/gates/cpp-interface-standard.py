# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `cpp-interface-standard`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# ADR 0059, section 7: the C++ interface compiled at the standard a
# consumer compiles it at. Every public header, every Forge C++ golden and
# every C++ header the generators write for this configure, each compiled
# ALONE with -Werror at C++17 and at C++20, beside the scan that refuses a
# standard-selecting `#if` in the headers and the C++ templates.
#
# SCE's own build compiles every target at C++20, so the C++17 four
# targets declared had never been compiled, and a downstream C++17 build
# found the first defect under GCC 15. No other gate can see that class:
# each builds SCE the way SCE builds itself.
#
# ⚠ `cost_s` is an ESTIMATE, not a stopwatch reading, and the slug is
# absent from COST_MEASURED for that reason. Its parts: a cold scratch
# configure and the generation rules (about two minutes), then some 2,600
# syntax-only compiles that `scripts/lib/sce_build_jobs.sh` spreads over
# the machine (about three on a 32-core build host). Time
# `scripts/gate cpp-interface-standard` on such a host and date it here.
{
    "workflows": ["cpp-interface-standard.yml"],
    "builds": ["main-tree"],
    "runner_workflow": True,
    # What the gate compiles or reads to decide what to compile: the
    # engine and its headers, the C++ runtime and the C backend it
    # compiles, the C++ templates and the generator that fills them, the
    # Forge goldens, the CMake that registers the corpora, and the tool.
    # The workflow's `paths:` lists every one, which `ci-only-coverage`
    # holds.
    "extra": ["sce/**", "backends/cpp/**", "backends/c/**",
              "tools/codegen/templates/**", "sce-build/src/**",
              "tests/forge/expected/**", "tests/CMakeLists.txt",
              "CMakeLists.txt", "cmake/**", "tools/interface-standard/**"],
    "deps": ["codegen-build"],
    "ci_only": "minutes long, estimated rather than measured: a cold "
               "configure, every generation rule and about 2,600 "
               "compiles, in a GCC 15 image the runner builds first. "
               "cpp-interface-standard.yml runs it, and an update to main "
               "runs it on a build machine with a rootless Docker.",
    "cost_s": 300,
    "summary": "C++ interface compiled alone at C++17 and C++20 (ADR 0059)",
    # The verdict is GCC 15's: its libstdc++ is the one that drops a
    # transitive include first. So the gate runs in one image wherever it
    # runs, the hosted lane, a build machine or a workstation alike: the
    # runner builds `container` (gcc:15.3.0 and what the configure needs)
    # and runs `scripts/gate cpp-interface-standard` inside it, generator
    # build included (see "Gates that run in an image"). A machine's own
    # GCC never judges it. A build machine takes the unit only where its
    # run account has a rootless Docker (`needs`).
    "container": "tools/interface-standard/Containerfile",
    "needs": ["docker"],
    "cache": False,
    "no_cache_reason": "the image is built from a tag (gcc:15.3.0) and the "
                       "Debian archive, and neither is fixed by the tree: "
                       "a rebuilt image can carry another compiler patch "
                       "or package, and the verdict moves with them",
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
