// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// build.rs for the world simulation: the documents under `machines/`, and the
// standard ones they import, are generated in-process by sce_build into OUT_DIR, and
// a tests file mounts them through `machines.rs`, which this script writes. Nothing
// generated is committed, so there is no tree to regenerate or to drift.
//
// A generated statechart calls the algorithms it imports as siblings of its own
// (`super::stamp_later`), and an algorithm the record schemas and algorithms it
// imports the same way, so every module is mounted side by side in one file, under
// the name of its source (a statechart's `_sm` suffix is left off). A generated file
// opens with inner attributes and doc comments, which `include!` cannot take, so each
// is declared as a module by `#[path]`.

use std::path::Path;

use sce_build::generator::Language;
use sce_build::{compile_forge_with_imports, DocumentLabel, ForgeCompileOptions};

/// The statecharts.
const STATECHARTS: &[&str] = &[
    "machines/echo_client.scxml",
    "machines/echo_server.scxml",
    "machines/sheet_peer.scxml",
];

/// The algorithms and record schemas the statecharts import, the standard ones from
/// the library of this repository and the others from `machines/`. A statechart is
/// generated without them, so each is generated here, by name.
const FORGE: &[&str] = &[
    "../../../stdlib/merge/hlc_timestamp.scxml",
    "../../../stdlib/merge/hlc_compare.scxml",
    "../../../stdlib/merge/hlc_send.scxml",
    "../../../stdlib/merge/hlc_receive.scxml",
    "machines/sheet_cell.scxml",
    "machines/sheet_edit.scxml",
    "machines/sheet_join.scxml",
    "machines/sheet_row.scxml",
    "machines/sheet_gone.scxml",
    "machines/sheet_add_row.scxml",
    "machines/sheet_drop_row.scxml",
    "machines/stamp_later.scxml",
    "machines/clock_send_wall.scxml",
    "machines/clock_send_counter.scxml",
    "machines/clock_receive_wall.scxml",
    "machines/clock_receive_counter.scxml",
];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    for document in STATECHARTS.iter().chain(FORGE) {
        println!("cargo:rerun-if-changed={document}");
    }
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR");

    for document in FORGE {
        let path = Path::new(document);
        let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {document}: {e}"));
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .expect("a document has a name");
        let base_dir = path.parent().expect("a document sits in a directory");
        let output = compile_forge_with_imports(
            &text,
            DocumentLabel::symmetric(name),
            Language::Rust,
            base_dir,
            &ForgeCompileOptions::default(),
        )
        .unwrap_or_else(|e| panic!("generate {document}: {e}"));
        for (filename, code) in output.files {
            let target = Path::new(&out_dir).join(&filename);
            std::fs::write(&target, code)
                .unwrap_or_else(|e| panic!("write {}: {e}", target.display()));
        }
    }
    sce_build::compile_scxml(STATECHARTS);

    let mut modules: Vec<(String, String)> = Vec::new();
    for entry in std::fs::read_dir(&out_dir).expect("OUT_DIR is readable") {
        let path = entry.expect("an entry").path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.ends_with(".rs") || name == "machines.rs" || name.ends_with(".include.rs") {
            continue;
        }
        let stem = name.trim_end_matches(".rs");
        let module = stem.strip_suffix("_sm").unwrap_or(stem);
        modules.push((module.to_string(), path.display().to_string()));
    }
    modules.sort();
    let mut mounted = String::new();
    for (module, path) in &modules {
        mounted.push_str(&format!(
            "#[path = {path:?}]\n#[allow(unused)]\npub mod {module};\n"
        ));
    }
    std::fs::write(Path::new(&out_dir).join("machines.rs"), mounted).expect("write machines.rs");
}
