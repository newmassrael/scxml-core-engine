# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `w3c-python-bindings`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The wrapper layer, not the engine under it: the trigger is the binding
# sources, because `w3c-cpp` already judges the same interpreter 404 cases
# at a time. This carried no `cost_s` for as long as no machine with the
# Python development headers had run it, which put it last by default.
# Measured 2026-08-12 on a machine that has them: it builds the pybind11
# extension and runs all 202 fixtures through it in 47s.
{
    "workflows": ["w3c-tests.yml"],
    # The extension links the engine, so it configures and builds the
    # main tree (tests and examples off).
    "builds": ["main-tree"],
    "runner_workflow": True,
    "narrows": "same catch-all as `w3c-cpp`, same reason. This arm reads "
               "the pybind11 wrapper sources.",
    "extra": ["backends/python/bindings/**"],
    # The bindings tree is the main CMake tree, whose configure resolves the
    # generator (cmake/SCECodegen.cmake) and stops without it. The lane built
    # it on the runner before the gate ran; in the image that build is not
    # there, and a landing's unit never had it, so the gate builds it first.
    "deps": ["codegen-build"],
    "ci_only": "184s, the most expensive gate a push could still run once "
               "the table was priced at what a loaded machine charges. "
               "w3c-tests.yml declares no `paths:` filter, so CI runs this "
               "arm on every push.",
    "cost_s": 184,
    "summary": "W3C conformance, pybind11 -> C++ Interpreter",
    # It runs in the host image wherever it runs (ADR 0145): the runner runs
    # `scripts/gate` inside the image its Containerfile addresses, so the
    # hosted lane, a build machine and a workstation judge it in one
    # environment, and no lane installs a package for it. A machine takes its
    # unit only where docker runs (`needs`).
    "container": "tools/ci-images/host/Containerfile",
    "needs": ["docker"],
    # What its lane sets for it, handed into the image (`container_env`):
    # a missing python3-dev as a failure rather than a skip.
    "container_env": ["SCE_REQUIRE_TOOLS"],
    "cache": True,
    "kept_only_from_a_landing": "it skips and exits 0 on a machine without "
                            "python3-config (python3-dev)",
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
