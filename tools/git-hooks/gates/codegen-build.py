# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `codegen-build`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# Prerequisite, not a gate: several gates execute target/debug/sce-codegen.
# Triggered by its own sources, and forced on by its dependents.
{
    # What a cargo build of the generator reads, derived (see "What a
    # build reads"): the workspace manifests, `sce-build` and its path
    # dependencies, the generator witness and its compiled-in files.
    "builds": ["sce-codegen"],
    "no_ci_reason": "not a check — it produces the binary other gates "
                    "run. CI builds it per workflow that needs it "
                    "(`debug_only_codegen_builds_drop_the_stale_release_binary` "
                    "counts those steps), so there is no verdict here "
                    "for a workflow to mirror.",
    # Stays local however expensive it gets: `drop_ci_only` removes a
    # gate without rescuing its dependents, so a `ci_only` dependency
    # would leave the gates that execute its binary with nothing to run.
    #
    # A CONDITIONAL cost, and the reason `cost_s_no_op` exists.
    #
    # Reached through `deps` on a tree whose `sce-build/**` has not
    # moved, cargo finds nothing to do: six runs on 2026-09-02 read
    # 0.099 0.100 0.087 [0.153] 0.107 0.111, the bracketed one queued
    # behind the cargo lock and discarded. That is `cost_s_no_op`.
    #
    # Selected by its OWN triggers, it rebuilds, and those triggers are
    # exactly `sce-build/**` and the manifests below. Three runs after
    # `touch sce-build/src/lib.rs` read 7.254 7.960 7.136 — one crate
    # recompiling. A `Cargo.lock` move rebuilds the dependency tree
    # instead, and this gate's own script records that at 24.6s "in a
    # tree those gates had already warmed". The budget bounds the WORST
    # case, so `cost_s` is that one.
    #
    # ⚠ 25 is pace-normalised — quoted from that script, not timed here.
    # `cargo clean` would be needed to reproduce it and this machine's
    # target/ is shared with sibling sessions. That is why this slug is
    # back in PACE_NORMALISED after a round that wrongly emptied it:
    # declaring the 0.111 alone left PUSH_BUDGET_S blind to tens of
    # seconds on precisely the pushes that select this gate.
    "cost_s": 25,
    "cost_s_no_op": 0.111,
    "summary": "build target/debug/sce-codegen",
    "cache": True,
    # Keyed on its triggers, which are what the build compiles, and on
    # what cargo reads beside them to load the workspace and fingerprint
    # the generator's build script (`gate_reads`, "What a build reads
    # that does not select it"). Read off `bx --checks prepush --learn`
    # of `forge-c11` and `forge-cpp` (2026-11-24), whose units run it.
    "inputs": INPUTS_ARE_TRIGGERS,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
