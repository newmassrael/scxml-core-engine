// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// build.rs for the Rust test crate: the three trees it compiles are build
// outputs (docs/adr/0097), written into OUT_DIR by their declared actions
// (tools/codegen/trees/<tree>.toml) through this machine's action cache, and
// declared as modules from what was placed:
//
//   - `rust-w3c`: the W3C machines, the module `generated` of src/lib.rs, and
//     a test file each, the modules of the one target tests/w3c.rs;
//   - `rust-integration` and `rust-static-datamodel`: a directory each under
//     src/integration, the modules src/integration/mod.rs declares.
//
// This script is the generator of each action it runs: Cargo builds no binary
// of another package for a build script, so it links sce-build with its
// commands and is started as `sce-codegen` would be
// (`sce_build::action::build_script`). It names to Cargo every file a
// generation read, so a template, a fixture or a manifest changed reruns it.

use sce_build::action::build_script::{
    materialize_trees, serve_generator_runs, write_module_declarations, Materialized,
};

/// The trees this crate compiles.
const TREES: &[&str] = &["rust-w3c", "rust-integration", "rust-static-datamodel"];

/// Each file of declarations the crate includes, and the directory, under the
/// checkout's root, whose placed modules it declares.
const DECLARATIONS: &[(&str, &str)] = &[
    // src/lib.rs: `generated`, the W3C machines.
    ("generated.rs", "backends/rust/tests/src"),
    // src/integration/mod.rs: one module per directory.
    ("integration.rs", "backends/rust/tests/src/integration"),
    // tests/w3c.rs: one module per W3C test.
    ("w3c.rs", "backends/rust/tests/tests"),
];

fn main() {
    // First: a run of this process as the generator of an action is that
    // action and nothing else.
    serve_generator_runs();
    let trees = materialize_trees(TREES)
        .unwrap_or_else(|e| panic!("the trees of sce-rust-tests are not written: {e}"));
    declare(&trees);
}

/// Write each file of declarations over what `trees` placed.
fn declare(trees: &Materialized) {
    for (file, dir) in DECLARATIONS {
        let modules = trees.modules_in(dir);
        assert!(
            !modules.is_empty(),
            "the trees placed no module in {dir}, which {file} declares"
        );
        write_module_declarations(file, &modules)
            .unwrap_or_else(|e| panic!("the declarations of {dir} are not written: {e}"));
    }
}
