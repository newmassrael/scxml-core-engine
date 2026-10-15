// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// build.rs for the world simulation: the documents under `machines/`, the shared
// documents every backend replays (`sheet_peer`, `todo_peer`, `presence_peer`, with their schemas,
// under `sce-build/tests/fixtures/static_datamodel/`), and the standard ones they
// import, are generated in-process by sce_build into OUT_DIR, and a tests file mounts
// them through `machines.rs`, which this script writes. Nothing generated is
// committed, so there is no tree to regenerate or to drift.
//
// The shared documents are read from the fixture directory and not kept here as well:
// a run of this world is written out as the scenarios the other backends replay
// (tests/a_world_run_is_what_every_backend_replays.rs), and they replay the machines
// of that directory, so the machine that ran and the machine that is replayed are one
// file.
//
// A generated statechart calls the algorithms it imports as siblings of its own
// (`super::lww_write`), and an algorithm the record schemas and algorithms it
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
    "../../../sce-build/tests/fixtures/static_datamodel/sheet_peer.scxml",
    "../../../sce-build/tests/fixtures/static_datamodel/todo_peer.scxml",
    "../../../sce-build/tests/fixtures/static_datamodel/presence_peer.scxml",
    "machines/todo_wide_sender.scxml",
];

/// The algorithms and record schemas the statecharts import, the standard ones from
/// the library of this repository and the others from `machines/`. A statechart is
/// generated without them, so each is generated here, by name.
const FORGE: &[&str] = &[
    "../../../stdlib/merge/hlc_timestamp.scxml",
    "../../../stdlib/merge/hlc_compare.scxml",
    "../../../stdlib/merge/hlc_send.scxml",
    "../../../stdlib/merge/hlc_receive.scxml",
    "../../../stdlib/merge/lww_cell.scxml",
    "../../../stdlib/merge/lww_write.scxml",
    "../../../stdlib/merge/orset_entry.scxml",
    "../../../stdlib/merge/orset_union.scxml",
    "../../../stdlib/merge/orset_live.scxml",
    "../../../stdlib/merge/orset_observed.scxml",
    "../../../stdlib/merge/mv_value.scxml",
    "../../../stdlib/merge/mv_value_holds.scxml",
    "../../../stdlib/merge/mv_value_keep.scxml",
    "../../../stdlib/merge/mv_value_drop.scxml",
    "../../../stdlib/merge/merkle_node.scxml",
    "../../../stdlib/merge/merkle_insert.scxml",
    "../../../stdlib/merge/rga_node.scxml",
    "../../../stdlib/merge/rga_gone.scxml",
    "../../../stdlib/merge/rga_insert.scxml",
    "../../../stdlib/merge/rga_holds.scxml",
    "../../../stdlib/merge/rga_remove.scxml",
    "../../../stdlib/merge/sheet_value.scxml",
    "../../../stdlib/merge/sheet_shown.scxml",
    "../../../stdlib/merge/sheet_apply.scxml",
    "../../../stdlib/merge/sheet_settle.scxml",
    "../../../stdlib/merge/sheet_show.scxml",
    "../../../sce-build/tests/fixtures/static_datamodel/sheet_cell.scxml",
    "../../../sce-build/tests/fixtures/static_datamodel/sheet_edit.scxml",
    "../../../sce-build/tests/fixtures/static_datamodel/sheet_formula.scxml",
    "../../../sce-build/tests/fixtures/static_datamodel/sheet_join.scxml",
    "../../../sce-build/tests/fixtures/static_datamodel/sheet_row.scxml",
    "../../../sce-build/tests/fixtures/static_datamodel/sheet_gone.scxml",
    "../../../sce-build/tests/fixtures/static_datamodel/sheet_add_row.scxml",
    "../../../sce-build/tests/fixtures/static_datamodel/sheet_drop_row.scxml",
    "../../../sce-build/tests/fixtures/static_datamodel/sheet_col.scxml",
    "../../../sce-build/tests/fixtures/static_datamodel/sheet_gone_col.scxml",
    "../../../sce-build/tests/fixtures/static_datamodel/sheet_add_col.scxml",
    "../../../sce-build/tests/fixtures/static_datamodel/sheet_drop_col.scxml",
    "../../../sce-build/tests/fixtures/static_datamodel/todo_join.scxml",
    "../../../sce-build/tests/fixtures/static_datamodel/todo_edit.scxml",
    "../../../sce-build/tests/fixtures/static_datamodel/todo_write.scxml",
    "../../../sce-build/tests/fixtures/static_datamodel/todo_supersede.scxml",
    "../../../sce-build/tests/fixtures/static_datamodel/todo_keep.scxml",
    "../../../sce-build/tests/fixtures/static_datamodel/todo_node.scxml",
    "../../../sce-build/tests/fixtures/static_datamodel/todo_visit.scxml",
    "../../../sce-build/tests/fixtures/static_datamodel/todo_share.scxml",
    "../../../sce-build/tests/fixtures/static_datamodel/presence_join.scxml",
    "../../../sce-build/tests/fixtures/static_datamodel/presence_beat.scxml",
    "../../../sce-build/tests/fixtures/static_datamodel/presence_alive.scxml",
    "../../../sce-build/tests/fixtures/static_datamodel/presence_seen.scxml",
    "../../../sce-build/tests/fixtures/static_datamodel/presence_left.scxml",
    "machines/todo_shout.scxml",
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
