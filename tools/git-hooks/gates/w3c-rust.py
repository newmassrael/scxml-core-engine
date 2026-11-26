# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `w3c-rust`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The Rust arm, and the last W3C lane to get a gate: `test-rust` in
# w3c-tests.yml carried its generation, fixture server and `cargo test`
# inline while every sibling job ran `scripts/gate w3c-<backend>`. A
# backend's lanes are derived from this table as
# `<family>-<Language::canonical_name()>`, so the missing slug was a lane
# the Rust row could not be read from.
#
# ⚠ No `cost_s`: nobody has timed this script yet, and a figure borrowed
# from the lane's timeout would be a guess wearing a measurement's unit.
# `run_order` sorts an unmeasured gate last, and UNMEASURED_COST_CEILING
# counts it, which is the honest place for it until a run is read.
#
# Its script runs one shard of the crate's targets when SCE_W3C_RUST_SHARD
# names `k/N`, as w3c-tests.yml's four jobs do (scripts/lib/workspace_shards.py
# plans the part from cargo). No `shard_env`: the build machines run it
# whole, as they run `w3c-c11`.
{
    "workflows": ["w3c-tests.yml"],
    "runner_workflow": True,
    "narrows": "same catch-all as `w3c-cpp`, same reason. This arm reads "
               "the Rust runtime and Lua crates, the Rust templates, the "
               "generator, the W3C fixtures it generates from and the "
               "build fixtures its hand-written tests compile in.",
    "extra": ["backends/rust/**", "tools/codegen/templates/**",
              "sce-build/src/**", "resources/**", "tests/w3c/**",
              "sce-build/tests/fixtures/**",
              # The shared Lua the Lua crate loads, and the shared tables
              # its readers ask.
              "sce/include/scripting/*.lua", "tests/ecmascript/**",
              # What the crate's build script generates its trees from
              # (docs/adr/0097): their manifests, the standard library and
              # the documents outside the fixture directories.
              "tools/codegen/trees/**", "stdlib/**",
              "integration_resources/**", "tests/integration/**",
              "examples/ai_loop/**",
              # What spreads the crate's targets over the lane's shards and
              # writes and reports each shard's plan, which the gate script
              # runs.
              "scripts/lib/workspace_shards.py", "scripts/lib/ctest_shards.py"],
    # No `deps`: the crate's build script is the generator of its own
    # trees, built from sce-build as a build-dependency, so no gate's
    # sce-codegen is executed.
    "ci_only": "unmeasured, and the lane it mirrors says why it cannot "
               "be a push gate: `test-rust` carries a 65-minute timeout, "
               "the longest of the W3C jobs, because it links the "
               "release test binaries under thin LTO with one codegen "
               "unit. w3c-tests.yml declares no `paths:` filter, so CI "
               "runs this arm on every push.",
    "summary": "W3C conformance, Rust AOT",
    # It runs in the host image wherever it runs (ADR 0145): the runner runs
    # `scripts/gate` inside the image its Containerfile addresses, so the
    # hosted lane, a build machine and a workstation judge it in one
    # environment, and no lane installs a package for it. A machine takes its
    # unit only where docker runs (`needs`).
    "container": "tools/ci-images/host/Containerfile",
    "needs": ["docker"],
    # What its lane sets for it, handed into the image (`container_env`):
    # the part of the crate's targets a lane job runs.
    "container_env": ["SCE_W3C_RUST_SHARD"],
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
