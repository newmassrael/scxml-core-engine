# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `swift-backend`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# ⚠ The Swift backend's own lane, and the one the family tables could not
# hold. `<family>-<canonical name>` slugs name a backend's arm of a lane the
# backend has: `w3c-<lang>` and `forge-<lang>` need an emitter that writes a
# W3C statechart or a Forge kind. What Swift had first was a runtime, a
# reference host and two hundred generated machines that replay the shared
# scenarios, and until this entry nothing a runner ran compiled any of it:
# the Swift lane was `bx -- scripts/test_swift_backend.sh` by hand.
#
# The slug is `swift-backend` and not `forge-swift` on purpose. A
# `forge-swift` would have been a Forge conformance gate that conforms
# nothing, and `a_backend_holds_what_the_other_backends_hold` reads the slug
# to decide that the backend HOLDS such a gate, which would erase the `NotYet`
# row that says it does not. The Forge and W3C arms exist now and took
# `forge-swift` and `w3c-swift`, and this gate keeps judging the packages that
# are neither.
#
# `deps` is the generator, which the script regenerates the machines with.
{
    "workflows": ["swift-backend.yml"],
    "runner_workflow": True,
    # Its script runs one part of itself when SCE_SWIFT_BACKEND_SHARD names
    # `k/N` (the `SWIFT_BACKEND_SHARDS` lines of scripts/gates/swift-backend.sh),
    # as the lane's three jobs do: whole, the gate ran 1209 s on a hosted
    # runner (job 138967193099). A plan makes one unit per part
    # (`shard_count`), each on a machine that carries the pin.
    "shard_env": "SCE_SWIFT_BACKEND_SHARD",
    "deps": ["codegen-build"],
    # 38s and 19s, the gate script run twice end to end through `bx` on a
    # build machine on 2026-10-28 (the first run built the packages, the
    # second found them built); 38 is declared, the worse of the two. Not
    # taken by `scripts/gate --measure`, which is refused locally for this
    # repository. Both figures include the transfer.
    #
    # `ci_only` is not about the 38s, which the push budget could spend. The
    # toolchain kept it out of every push: a hand-installed pin that
    # `provision` puts on a build machine, and a push the hook ran in place
    # on a machine that lacked it would be refused for a reason that says
    # nothing about the tree. The gate fails loudly without a toolchain
    # rather than skipping. An update to main runs it all the same (see
    # "What `ci_only` means"): the pin is a tool of bx.toml's platform, so
    # bx runs the unit only on a machine that carries it.
    "ci_only": "38s, and the 6.4.0 toolchain is a hand-installed pin "
               "(`[platform.linux.tools.swift]` in bx.toml); the gate "
               "refuses without it, so a push of another ref run on a "
               "machine without the pin would be refused for a reason that "
               "says nothing about the tree. An update to main runs it "
               "where bx placed it, on a machine that carries the pin. "
               "swift-backend.yml installs the pin by its digest.",
    "cost_s": 38,
    # The suite's committed traces, which `ATestSuiteRunsThroughItsRunnerTests.swift`
    # replays through the function and the runner program, and the trace's wire schema,
    # whose status `TraceTests.swift` holds the runtime's to, both read at run time. The
    # workflow's `paths:` names both, as a `ci_only` gate's triggers must be
    # (`ci-only-coverage`).
    "extra": ["sce-build/tests/fixtures/test_profile/runtime/traces/**",
              "schemas/sce-test-trace.v1.schema.json",
              # The corpus's suites `ACorpusSuiteReplaysToItsExpectedLogsTests.swift`
              # replays: the traces and expected logs of each, read at run time, and the
              # documents of orders_suite and nested_suite, which the regeneration
              # writes into the machines' target.
              "sce-build/tests/fixtures/test_profile/runs/**",
              "sce-build/tests/fixtures/ttcn3_export/**"],
    "summary": "Swift backend: runtime, host features and generated machines",
    # The toolchain is a pinned tool of the platform, so its version is in
    # the key, and a machine without it fails the gate loudly.
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
