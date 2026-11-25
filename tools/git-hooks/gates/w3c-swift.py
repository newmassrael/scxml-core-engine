# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `w3c-swift`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The Swift W3C arm, the twin of `w3c-go`: `sce-codegen generate-w3c -l swift` writes the
# suite and the engine runs it. It rides the workflow of `swift-backend` as the first step of
# the `swift-machines` job, with `ecma262-lowered-swift` and `forge-swift`, because what the
# three need is the same: the generator built, the pinned toolchain installed and checked
# against its digest. The `swift-backend` gate's parts run in jobs of their own beside it
# (`SCE_SWIFT_BACKEND_SHARD`), so the three gates no longer wait for its twenty minutes.
#
# `ci_only` for the reason `swift-backend` is: the 6.4.0 toolchain is a hand-installed pin,
# and the gate fails loudly without it. An update to main runs it where bx placed it.
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
    # 76s, `scripts/test_swift_backend.sh w3c` through `bx` on a build machine on
    # 2026-11-01, which is what the gate runs and reads (a gate run is refused locally
    # for this repository, so the script under it was timed). 156 fixtures lowered and
    # 156 tests run. Warm, like `swift-backend`'s figure: a runner starts from nothing.
    "cost_s": 76,
    "summary": "W3C conformance, Swift AOT",
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
