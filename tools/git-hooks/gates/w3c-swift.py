# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `w3c-swift`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The Swift W3C arm, the twin of `w3c-go`: `sce-codegen generate-w3c -l swift` writes the
# suite and the engine runs it. It rides the workflow of `swift-backend` as the first step of
# the `swift-machines` job, with `ecma262-lowered-swift` and `forge-swift`, because what the
# three need is the same: the swift image, which holds the pinned toolchain, and the generator
# built in it. The `swift-backend` gate's parts run in jobs of their own beside it
# (`SCE_SWIFT_BACKEND_SHARD`), so the three gates no longer wait for its twenty minutes.
#
# `ci_only` for the reason `swift-backend` is: the toolchain is the swift image's, which a push
# would pull first. An update to main runs it where bx placed it.
{
    "workflows": ["swift-backend.yml"],
    "runner_workflow": True,
    "deps": ["codegen-build"],
    # An edit of the image is a change the gate judges.
    "extra": ["tools/ci-images/swift/**"],
    # The swift image (ADR 0145, family `swift`): the pinned Swift toolchain and
    # the packages it links against, the same wherever the gate runs. The
    # runner pulls the image its Containerfile's digest names and runs
    # `scripts/gate w3c-swift` inside it, the generator build among its deps
    # ("Gates that run in an image" in tools/git-hooks/gate_registry.py).
    "container": "tools/ci-images/swift/Containerfile",
    "needs": ["docker"],
    "ci_only": "the toolchain is the swift image's (tools/ci-images/swift/"
               "Containerfile, at the release `[platform.linux.tools.swift]` "
               "of bx.toml pins), and a push would pull that image first; "
               "the gate refuses another release. swift-backend.yml runs it "
               "in the image, and an update to main runs it where bx placed "
               "it, on a machine with a rootless Docker.",
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
