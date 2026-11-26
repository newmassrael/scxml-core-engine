# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `ecma262-lowered-swift`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The Swift twin of `ecma262-lowered-kotlin`: one machine that asks every case of the shared
# ECMA-262 table, generated with the build-time lowering (the only route Swift has), compiled
# and run. A step of the `swift-machines` job of `swift-backend.yml` after `w3c-swift`, for
# the same reason: the swift image and the generator built in it are what it shares.
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
    # `scripts/gate ecma262-lowered-swift` inside it, the generator build among its deps
    # ("Gates that run in an image" in tools/git-hooks/gate_registry.py).
    "container": "tools/ci-images/swift/Containerfile",
    "needs": ["docker"],
    "ci_only": "the toolchain is the swift image's (tools/ci-images/swift/"
               "Containerfile, at the release `[platform.linux.tools.swift]` "
               "of bx.toml pins), and a push would pull that image first; "
               "the gate refuses another release. swift-backend.yml runs it "
               "in the image, and an update to main runs it where bx placed "
               "it, on a machine with a rootless Docker.",
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
