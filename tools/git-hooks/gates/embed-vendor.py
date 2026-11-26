# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `embed-vendor`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
{
    "workflows": ["embed-vendor-smoke.yml"],
    # The reason this lane kept its own copy — "it also calls a sibling
    # gate, so its steps are a superset" — did not hold: the sibling
    # (`embed-manifest-failfast`) runs in a SEPARATE job of the same
    # workflow, and this job restated the three scripts verbatim.
    "runner_workflow": True,
    # Its script runs one of its three parts when SCE_EMBED_VENDOR_SHARD names
    # `k/N` (the `EMBED_VENDOR_PARTS` lines of scripts/gates/embed-vendor.sh),
    # as the lane's three jobs do: whole, the gate ran 1142 s on a hosted
    # runner (job 138811665797). A plan makes one unit per part
    # (`shard_count`).
    "shard_env": "SCE_EMBED_VENDOR_SHARD",
    "extra": ["sce/include/**", "embed/MANIFEST.json",
              "backends/cpp/forge-runtime/include/**",
              "scripts/smoke_bazel_consumer.sh", "scripts/sce_repo.bzl",
              "scripts/lib/bazel_pins.sh", "tests/bazel_consumer/**"],
    # The most expensive gate in the set: three scratch-directory builds,
    # one of which packages embed/ from scratch and builds a consumer
    # against it.
    "ci_only": "311s, and on its own it is more than the whole push "
               "budget of 300s. Three scratch-directory builds, one of "
               "which packages embed/ from scratch and builds a consumer "
               "against it — none of that gets cheaper on a warm tree, "
               "because the scratch directories are the point.",
    "cost_s": 311,
    "summary": "embed manifest drift + payload lag + consumer smoke",
    # It runs in the host image wherever it runs (ADR 0145): the runner runs
    # `scripts/gate` inside the image its Containerfile addresses, so the
    # hosted lane, a build machine and a workstation judge it in one
    # environment, and no lane installs a package for it. A machine takes its
    # unit only where docker runs (`needs`).
    "container": "tools/ci-images/host/Containerfile",
    "needs": ["docker"],
    # The Bazel consumer's versions are pinned in the tree
    # (`scripts/lib/bazel_pins.sh`), and a machine without bazel ends the
    # smoke "could not measure" (exit 3) rather than passing.
    # `package_embed.sh` writes `git describe` into the package's VERSION
    # file, which no step compares, so the gate reads no commit.
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
