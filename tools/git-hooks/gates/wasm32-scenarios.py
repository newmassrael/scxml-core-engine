# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `wasm32-scenarios`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The Rust engine as the module a browser loads, run under Node and in
# headless Chrome on the scenarios every backend replays. No other lane can
# say whether the engine starts on `wasm32-unknown-unknown`: that target
# gives a module no clock, and the failure compiles and traps only when the
# module runs. Chrome is the web target's own host and Node the faster one.
# And the browser node (docs/adr/0117): its module, the HTTPS binding's
# client core, held with the page's shell around it to its kit under Node and
# in headless Chrome and Firefox, against the binding's own server, which the
# gate builds for the host (`sce-rust-mesh`'s example `https_echo`). And the
# page surface (docs/adr/0117, D7): `sce-codegen surface` writes it from a
# fixture, the pinned TypeScript compiles a page that uses it right and must
# refuse each wrong use of one that does not, and Node runs its module's checks.
# Neither cost is in `cost_s` below, which was measured before they were added.
#
# `ci_only` for what it needs rather than for the number alone: the target's
# standard library, Node, Chrome, and a `wasm-bindgen` tool at exactly the
# version `Cargo.lock` holds, none of which a push can assume. The
# workflow's filter IS the trigger, as `visualizer-wasm`'s is.
#
# Measured 2026-10-08 as the gate script itself, through `bx --local`
# (`scripts/gate --measure` is refused locally for this repository): 107s
# with the dependencies built (the runtime and the replay crate rebuilt),
# of which Chrome's replay is 20s. The first dev-profile build of the target
# on that tree took 128s before Chrome was added. The same gate built in
# release took 620s (`lto = "thin"` with one codegen unit over 44 generated
# machines), which is why it builds the dev profile.
{
    "workflows": ["wasm32-scenarios.yml"],
    "runner_workflow": True,
    # Two parts when SCE_WASM32_SCENARIOS_SHARD names `k/2`: clippy for the
    # target, and the module built and replayed, which share nothing. Whole,
    # the hosted job took 11 minutes (job 139040902859). A plan makes one unit
    # per part (`shard_count`).
    "shard_env": "SCE_WASM32_SCENARIOS_SHARD",
    # The page surface is written by the generator the run builds once
    # (`codegen-build`), which the replay part obtains for `sce-codegen surface`.
    "deps": ["codegen-build"],
    "ci_only": "107s measured 2026-10-08 with the dependencies built "
               "(128s on a first dev-profile build before Chrome was "
               "added, 620s in release). wasm32-scenarios.yml runs it, "
               "and needs the wasm32 standard library, Node, headless "
               "Chrome and a wasm-bindgen at the version Cargo.lock "
               "holds. It runs in the host image, which holds Node and "
               "the pinned Chrome and wasm-bindgen; the runner hands it "
               "the selected Rust with its wasm32 standard library.",
    "cost_s": 107,
    "summary": "Rust engine built for wasm32, every scenario replayed in Node and Chrome; "
               "the browser node's kit in Node, Chrome and Firefox; the page surface "
               "compiled by TypeScript and run in Node",
    # It runs in the host image wherever it runs (ADR 0145): the runner runs
    # `scripts/gate` inside the image its Containerfile addresses, so the
    # hosted lane, a build machine and a workstation judge it in one
    # environment, and no lane installs a package for it. A machine takes its
    # unit only where docker runs (`needs`).
    "container": "tools/ci-images/host/Containerfile",
    "needs": ["docker"],
    # The browser and the glue tool are the image's, pinned (Chrome for
    # Testing at the version scripts/install_chrome.sh names, which the
    # registry's self-test holds the image to, and `wasm-bindgen` at
    # Cargo.lock's, which the gate checks), and the wasm32 standard library
    # is a target of the release the plan names, declared with it in
    # `[platform.linux.tools.rust]` and handed to the image by the runner. In a landing
    # only the pinned Chrome judges the module, so a pass kept from one is
    # that browser's; outside a landing the gate may replay in the Chrome on
    # the machine's PATH, which no key names.
    "cache": True,
    "kept_only_from_a_landing": "outside a landing it replays in whatever "
                                "Chrome is on PATH where the pinned Chrome "
                                "for Testing is absent, and runs the kit in "
                                "whatever Firefox is on PATH where the pinned "
                                "one is absent",
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
