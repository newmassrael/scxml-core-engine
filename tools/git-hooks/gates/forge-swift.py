# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `forge-swift`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The Swift twin of `forge-kotlin`: the forge runtime's own tests and every unit the generator
# writes for a forge document, held to the numerical oracle. A step of the `swift-machines`
# job of `swift-backend.yml` after `w3c-swift` and `ecma262-lowered-swift`, for the same
# reason. It takes the
# `forge-<canonical name>` slug the `swift-backend` entry said it would, now that Swift writes
# forge kinds in part and holds what it writes to the oracle; the queue and broadcast-log
# history checks the other arms make are read here when Swift has those kinds.
#
# `ci_only` for the reason `swift-backend` is: the toolchain is the swift image's.
{
    "workflows": ["swift-backend.yml"],
    "runner_workflow": True,
    "deps": ["codegen-build"],
    # An edit of the image is a change the gate judges.
    "extra": ["tools/ci-images/swift/**"],
    # The swift image (ADR 0145, family `swift`): the pinned Swift toolchain and
    # the packages it links against, the same wherever the gate runs. The
    # runner pulls the image its Containerfile's digest names and runs
    # `scripts/gate forge-swift` inside it, the generator build among its deps
    # ("Gates that run in an image" in tools/git-hooks/gate_registry.py).
    "container": "tools/ci-images/swift/Containerfile",
    "needs": ["docker"],
    "ci_only": "the toolchain is the swift image's (tools/ci-images/swift/"
               "Containerfile, at the release `[platform.linux.tools.swift]` "
               "of bx.toml pins), and a push would pull that image first; "
               "the gate refuses another release. swift-backend.yml runs it "
               "in the image, and an update to main runs it where bx placed "
               "it, on a machine with a rootless Docker.",
    # 102s, `scripts/test_swift_backend.sh forge` through `bx` on a build machine on
    # 2026-11-01 (the script under the gate: a gate run is refused locally for this
    # repository). The forge runtime's 73 tests and 184 conformance tests. Warm, like its
    # siblings: a runner starts from nothing.
    "cost_s": 102,
    "summary": "Forge conformance, Swift",
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
