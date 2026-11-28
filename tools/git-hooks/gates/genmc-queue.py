# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `genmc-queue`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The C11 and C++ queues under GenMC (SCE Protocol-Synthesis RFC §synth-5-P,
# layer 3), and the mutants that show the check reads orderings at all. It needs
# GenMC, built from source at the revision `scripts/install_genmc.sh` pins,
# which the host image holds, so it runs in that image.
#
# ⚠ `ci_only` because a build machine starts the image only under a rootless
# Docker, which not every one is verified to carry: the gate fails loudly
# where GenMC is absent rather than skipping, and a push that selected it on
# such a host would be refused for a reason that says nothing about the tree.
# The image names the revision the installer pins, and the registry's
# self-test holds the two to one (`a-gate-image-holds-the-pins-its-gate-reads`).
{
    "workflows": ["genmc-queue.yml"],
    "runner_workflow": True,
    # Its script starts every N-th of its runs when SCE_GENMC_QUEUE_SHARD
    # names `k/N` ("IN PARTS" in scripts/gates/genmc-queue.sh), as the lane's
    # three jobs do: whole, the 189 runs took 1006 s on a hosted runner (job
    # 138783228903). A plan makes one unit per part (`shard_count`).
    "shard_env": "SCE_GENMC_QUEUE_SHARD",
    # What the gate reads, and no wider: the runtime headers GenMC compiles
    # (the C11 inbox's among them), the harnesses, and the hosted atomics
    # every C harness includes whole.
    # For C++ that is the three headers a mutant rewrites (the two queue
    # headers and the broadcast log) and the
    # harnesses with their shim and support headers; every other header of
    # that runtime is not read. The same set as the workflow's `paths:`,
    # which the self-test holds.
    "extra": [
        "backends/c/forge-runtime/include/**",
        "backends/c/runtime/include/sce/text.h",
        # The memory functions sce/text.h declares, in a header of their own.
        "backends/c/runtime/include/sce/mem.h",
        # The C11 inbox with what it includes, and the token check, which
        # the inbox harnesses read (ADR 0147, D5).
        "backends/c/runtime/include/sce/inbox.h",
        "backends/c/runtime/include/sce/types.h",
        "backends/c/runtime/include/sce/portability.h",
        "backends/c/runtime/include/sce/host_processor.h",
        "backends/c/forge-runtime/tests/genmc/**",
        "backends/c/forge-runtime/tests/conformance/sce_atomic_host.c",
        "backends/cpp/forge-runtime/include/sce/forge/queue.h",
        "backends/cpp/forge-runtime/include/sce/forge/queue_segmented.h",
        "backends/cpp/forge-runtime/include/sce/forge/broadcast_log.h",
        "backends/cpp/forge-runtime/tests/genmc/**",
        "scripts/install_genmc.sh",
    ],
    "ci_only": "72s measured 2026-10-10 on a host that has GenMC, and the "
               "host is the constraint: GenMC is built from source at the "
               "revision scripts/install_genmc.sh pins and held by the "
               "host image, which a build machine starts only under a "
               "rootless Docker that not every one is verified to carry. "
               "genmc-queue.yml runs it in that image.",
    "cost_s": 72,
    "summary": "C11 and C++ queues under GenMC + weakened-ordering mutants",
    # It runs in the host image wherever it runs (ADR 0145): the runner runs
    # `scripts/gate` inside the image its Containerfile addresses, so the
    # hosted lane, a build machine and a workstation judge it in one
    # environment, and no lane installs a package for it. A machine takes its
    # unit only where docker runs (`needs`).
    "container": "tools/ci-images/host/Containerfile",
    "needs": ["docker"],
    # GenMC explores every interleaving its model admits rather than
    # sampling some, its revision is pinned by the tracked installer, and the
    # image that holds it is addressed by its tracked Containerfile, which
    # selects the gate and so is in its key.
    "cache": True,
    # Keyed on its triggers: `bx --checks prepush --learn genmc-queue`
    # (2026-11-24) found 55 reads and 4 listings, every one inside the
    # triggers and the runner's inputs.
    "inputs": INPUTS_ARE_TRIGGERS,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
