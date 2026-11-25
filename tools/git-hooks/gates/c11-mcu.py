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
# `ci_only` for `genmc-queue`'s reason: the ARM compiler is a tool bx.toml
# declares (`[platform.linux.tools.arm-none-eabi-gcc]`, Arm's tarball of the
# release scripts/install_arm_gnu_toolchain.sh pins) and QEMU a package the
# unit requires (`packages` below), and not every build machine is verified
# to carry them; the gate refuses (exit 3) without them, so a push that
# selected it there would be refused for a reason that says nothing about
# the tree. c11-mcu.yml installs both on its runner.
{
    "workflows": ["c11-mcu.yml"],
    "runner_workflow": True,
    "deps": ["codegen-build"],
    "ci_only": "Arm's GNU Toolchain, a tool bx.toml pins by the install "
               "script's digests, and QEMU, a package the gate's unit "
               "requires, which not every build machine is verified to "
               "carry; the gate ends 'could not run' without them, so a "
               "push that selected it there would be refused for a reason "
               "that says nothing about the tree. c11-mcu.yml installs both.",
    "cost_s": 60,
    "summary": "C11 machines linked with no libc, weighed and run on a Cortex-M4",
    # QEMU's release does not move the verdict: a run passes or fails on
    # the scenario's expectations, on a counter clock the scenario alone
    # moves. So it is a requirement of the unit, not a tool with pinned
    # bytes (docs/adr/0110, D4).
    "packages": ["qemu-system-arm"],
    # The compiler is a tool of the platform at the release bx.toml checks
    # (and the gate refuses another build of it), the budget and the
    # scenarios are tracked, and the run is on a counter clock the
    # scenario alone moves.
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
