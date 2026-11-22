// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// build.rs for the server suite: the generated machines its server process
// serves are the tree `rust-server-suite` (tools/codegen/trees/rust-server-suite.toml),
// a build output (docs/adr/0097) written into OUT_DIR by its declared actions,
// through this machine's action cache, and mounted by src/lib.rs from the
// declaration this script writes.
//
// This script is the generator of each action it runs
// (`sce_build::action::build_script`), and it names to Cargo every file a
// generation read, so a document or a template changed reruns it.

use sce_build::action::build_script::{
    materialize_trees, serve_generator_runs, write_module_declarations,
};

/// The directory, under the checkout's root, the tree places its module in.
const MOUNTED: &str = "tests/server/src";

fn main() {
    // First: a run of this process as the generator of an action is that
    // action and nothing else.
    serve_generator_runs();
    let trees = materialize_trees(&["rust-server-suite"])
        .unwrap_or_else(|e| panic!("the machines of sce-server-suite are not written: {e}"));
    let modules = trees.modules_in(MOUNTED);
    assert!(
        modules.iter().any(|module| module.name == "generated"),
        "the tree placed no generated module in {MOUNTED}"
    );
    write_module_declarations("generated.rs", &modules)
        .unwrap_or_else(|e| panic!("the declaration of generated is not written: {e}"));
}
