// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// build.rs for the scenario replay: the `datamodel="sce-static"` machines it
// replays are the tree `rust-static-datamodel`
// (tools/codegen/trees/rust-static-datamodel.toml), a build output
// (docs/adr/0097) written into OUT_DIR by its declared actions, through this
// machine's action cache, and mounted by src/integration.rs from the
// declaration this script writes. sce-rust-tests compiles the same tree from
// its own OUT_DIR; the actions are one, so the cache answers the second.
//
// This script is the generator of each action it runs
// (`sce_build::action::build_script`), and it names to Cargo every file a
// generation read, so a fixture or a template changed reruns it.

use sce_build::action::build_script::{
    materialize_trees, serve_generator_runs, write_module_declarations,
};

/// The directory of the tree, under the checkout's root, whose modules
/// src/integration.rs mounts: `static_datamodel` alone.
const MOUNTED: &str = "backends/rust/tests/src/integration";

fn main() {
    // First: a run of this process as the generator of an action is that
    // action and nothing else.
    serve_generator_runs();
    let trees = materialize_trees(&["rust-static-datamodel"])
        .unwrap_or_else(|e| panic!("the machines of sce-rust-scenarios are not written: {e}"));
    let modules = trees.modules_in(MOUNTED);
    assert!(
        modules.iter().any(|m| m.name == "static_datamodel"),
        "the tree placed no static_datamodel module in {MOUNTED}"
    );
    write_module_declarations("static_datamodel.rs", &modules)
        .unwrap_or_else(|e| panic!("the declaration of static_datamodel is not written: {e}"));
}
