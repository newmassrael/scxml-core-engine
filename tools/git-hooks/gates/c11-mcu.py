# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `c11-mcu`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# A C11 machine's core links with no C library, is weighed against a budget
# and runs its replay scenario on a Cortex-M4 under QEMU (docs/adr/0110, D3):
# the set the ADR names, generated, compiled with the ADR's flags, linked
# with `-nostdlib -lgcc` beside the board's startup, memory functions and
# seams (backends/c/mcu/), and held to backends/c/mcu/footprint.budget.
#
# `cost_s` is an ESTIMATE, absent from COST_MEASURED: the gate was written
# where no ARM compiler could run. Six generations, a few dozen small C
# compiles and six QEMU runs of a second or two each, after the generator
# build `codegen-build` owns. Time it on its first run and date it here.
#
# `ci_only` for `genmc-queue`'s reason: the ARM compiler (Arm's tarball of
# the release scripts/install_arm_gnu_toolchain.sh pins) and QEMU are the
# host image's, and a build machine starts that image only under a rootless
# Docker, which not every one is verified to carry; the gate refuses (exit 3)
# without it, so a push that selected it there would be refused for a reason
# that says nothing about the tree. c11-mcu.yml runs it in the image.
{
    "workflows": ["c11-mcu.yml"],
    "runner_workflow": True,
    "deps": ["codegen-build"],
    "ci_only": "Arm's GNU Toolchain, at the release the install script "
               "pins, and QEMU are the host image's, which a build machine "
               "starts only under a rootless Docker that not every one is "
               "verified to carry; the gate ends 'could not run' without "
               "it, so a push that selected it there would be refused for a "
               "reason that says nothing about the tree. c11-mcu.yml runs "
               "it in that image.",
    "cost_s": 60,
    "summary": "C11 machines linked with no libc, weighed and run on a Cortex-M4",
    # It runs in the host image wherever it runs (ADR 0145): the runner runs
    # `scripts/gate` inside the image its Containerfile addresses, so the
    # hosted lane, a build machine and a workstation judge it in one
    # environment, and no lane installs a package for it. A machine takes its
    # unit only where docker runs (`needs`).
    "container": "tools/ci-images/host/Containerfile",
    "needs": ["docker"],
    # The compiler is the image's at the release the install script pins
    # (and the gate refuses another build of it), the image is addressed by
    # its tracked Containerfile, the budget and the scenarios are tracked,
    # and the run is on a counter clock the scenario alone moves. QEMU's
    # release does not move the verdict (docs/adr/0110, D4).
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
