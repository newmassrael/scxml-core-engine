# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `ecma262-lowered-swift`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The Swift twin of `ecma262-lowered-kotlin`: one machine that asks every case of the shared
# ECMA-262 table, generated with the build-time lowering (the only route Swift has), compiled
# and run. A step of the `swift-machines` job of `swift-backend.yml` after `w3c-swift`, for
# the same reason: the generator built and the pinned toolchain installed are steps it shares.
#
# `ci_only` for the reason `swift-backend` is: the toolchain pin.
{
    "workflows": ["swift-backend.yml"],
    "runner_workflow": True,
    "deps": ["codegen-build"],
    "ci_only": "the 6.4.0 toolchain is a hand-installed pin (`[platform.linux.tools.swift]` "
               "in bx.toml); the gate refuses without it, so a push of "
               "another ref run on a machine without the pin would be "
               "refused for a reason that says nothing about the tree. An "
               "update to main runs it where bx placed it, on a machine "
               "that carries the pin. swift-backend.yml installs the pin "
               "by its digest.",
    # 11s, `scripts/test_swift_backend.sh ecma262-lowered` through `bx` on a build machine
    # on 2026-11-01 (the script under the gate: a gate run is refused locally for this
    # repository). 107 cases, one generated machine. Warm, like its siblings: a runner
    # starts from nothing.
    "cost_s": 11,
    "summary": "ECMA-262 lowered artifact, Swift",
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
