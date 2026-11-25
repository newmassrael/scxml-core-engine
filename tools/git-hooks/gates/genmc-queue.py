# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `genmc-queue`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The C11 and C++ queues under GenMC (SCE Protocol-Synthesis RFC §synth-5-P,
# layer 3), and the mutants that show the check reads orderings at all. It needs
# GenMC, which no hosted runner carries and which is built from source
# (`scripts/install_genmc.sh`), so it runs where that was done.
#
# ⚠ `ci_only` because the install is the cost and the hosts do not all have
# it: the gate fails loudly where GenMC is absent rather than skipping, and
# a push that selected it on such a host would be refused for a reason that
# says nothing about the tree. The workflow caches the build under a key
# made of the installer's hash, so the pin and the cache move together.
{
    "workflows": ["genmc-queue.yml"],
    "runner_workflow": True,
    # What the gate reads, and no wider: the runtime headers GenMC compiles,
    # the harnesses, and the hosted atomics every C harness includes whole.
    # For C++ that is the three headers a mutant rewrites (the two queue
    # headers and the broadcast log) and the
    # harnesses with their shim and support headers; every other header of
    # that runtime is not read. The same set as the workflow's `paths:`,
    # which the self-test holds.
    "extra": [
        "backends/c/forge-runtime/include/**",
        "backends/c/runtime/include/sce/text.h",
        "backends/c/forge-runtime/tests/genmc/**",
        "backends/c/forge-runtime/tests/conformance/sce_atomic_host.c",
        "backends/cpp/forge-runtime/include/sce/forge/queue.h",
        "backends/cpp/forge-runtime/include/sce/forge/queue_segmented.h",
        "backends/cpp/forge-runtime/include/sce/forge/broadcast_log.h",
        "backends/cpp/forge-runtime/tests/genmc/**",
        "scripts/install_genmc.sh",
    ],
    "ci_only": "72s measured 2026-10-10 on a host that has GenMC, and the "
               "host is the constraint: GenMC is built from source "
               "(scripts/install_genmc.sh), no hosted runner carries it, "
               "and the build machines do not all have it. An update to "
               "main runs it where bx placed it: GenMC is a tool of "
               "bx.toml's platform, needed by this gate's runs. "
               "genmc-queue.yml builds it once per pin and caches it.",
    "cost_s": 72,
    "summary": "C11 and C++ queues under GenMC + weakened-ordering mutants",
    # GenMC explores every interleaving its model admits rather than
    # sampling some, and its revision is pinned by the tracked installer.
    "cache": True,
    # Keyed on its triggers: `bx --checks prepush --learn genmc-queue`
    # (2026-11-24) found 55 reads and 4 listings, every one inside the
    # triggers and the runner's inputs.
    "inputs": INPUTS_ARE_TRIGGERS,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
