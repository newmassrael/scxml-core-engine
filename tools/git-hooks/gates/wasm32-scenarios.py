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
    "ci_only": "107s measured 2026-10-08 with the dependencies built "
               "(128s on a first dev-profile build before Chrome was "
               "added, 620s in release). wasm32-scenarios.yml runs it, "
               "and needs the wasm32 standard library, Node, headless "
               "Chrome and a wasm-bindgen at the version Cargo.lock "
               "holds. An update to main runs it where bx placed it, on "
               "a machine carrying bx.toml's pinned Chrome and "
               "wasm-bindgen tools.",
    "cost_s": 107,
    "summary": "Rust engine built for wasm32, every scenario replayed in Node and Chrome",
    # The browser and the glue tool are pinned tools of bx.toml's platform
    # (`chrome`, Chrome for Testing at the version scripts/install_chrome.sh
    # names, and `wasm-bindgen` at Cargo.lock's), and the wasm32 standard
    # library is a target of the release the plan names, declared with it
    # in `[platform.linux.tools.rust]`. In a landing
    # only the pinned Chrome judges the module, so a pass kept from one is
    # that browser's; outside a landing the gate may replay in the Chrome on
    # the machine's PATH, which no key names.
    "cache": True,
    "kept_only_from_a_landing": "outside a landing it replays in whatever "
                                "Chrome is on PATH where the pinned Chrome "
                                "for Testing is absent",
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
