# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `rust-version`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# Shares clippy's workflow because it shares clippy's inputs: the
# declarations clippy measures its lints against are the ones this keeps
# true. Not the warm figure this table asks for: 451s is the first run on
# a build machine that had none of the six toolchains, so it installed
# every one and built every package cold, the state a CI runner is always
# in. The six builds alone read 21, 16, 4.5, 60, 8 and 14 seconds.
{
    "workflows": ["clippy-check.yml"],
    "runner_workflow": True,
    "ci_only": "451s cold, six toolchains installed. clippy-check.yml "
               "runs it beside clippy.",
    "cost_s": 451,
    "summary": "every package builds with the Rust it declares",
    # Each toolchain it installs is the version a tracked manifest declares.
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
