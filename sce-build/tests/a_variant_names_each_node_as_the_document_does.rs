// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A variant names each node as the document does (ADR 0057 §2, §10): the
//! sourcemap of `generate --features` and the markers of the output for every
//! build name a transition, an `<onentry>` block and a block's action by where
//! it stands in the document, so each symbol a variant writes is the
//! document's symbol of the same node, on the same row. A transition, block or
//! action after a sibling the variant leaves out was named by its place in the
//! variant until 2026-11-04, so the output for every build cited, on the
//! marker of the row it was written on, the symbol of the sibling some builds
//! leave out.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::sourcemap::{self, Sourcemap};
use sce_build::forge::symbol_mangling::mangle;

const FEATURES: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="feature-model" name="session_features">
  <sce:feature name="session-unicast"/>
  <sce:feature name="session-unicast-accept" requires="session-unicast"/>
</scxml>
"#;

/// Each list of `idle` holds a node after a sibling that only some variants
/// keep: the third `<onentry>` follows a gated one and one whose only action
/// is gated, which a variant without it holds no block of; `kept` follows a
/// gated action in its block; and `close` follows a gated transition.
const MACHINE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="shift" initial="idle" datamodel="sce-static">
  <sce:import kind="feature-model" src="features.scxml" as="features"/>
  <state id="idle">
    <onentry sce:requires="session-unicast">
      <sce:action name="first" effect="notice"/>
    </onentry>
    <onentry>
      <sce:action name="only" effect="notice" requires="session-unicast"/>
    </onentry>
    <onentry>
      <sce:action name="gated" effect="notice" requires="session-unicast-accept"/>
      <sce:action name="kept" effect="notice"/>
    </onentry>
    <transition event="connect" target="done" sce:requires="session-unicast"/>
    <transition event="close" target="done">
      <sce:action name="on_close" effect="notice"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>
"#;

const CONFIGURATIONS: [&[&str]; 3] = [
    &[],
    &["session-unicast"],
    &["session-unicast", "session-unicast-accept"],
];

fn product_line() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("features.scxml"), FEATURES).unwrap();
    let path = dir.path().join("shift.scxml");
    std::fs::write(&path, MACHINE).unwrap();
    (dir, path)
}

/// Generate `machine` for Rust into `out`, the variant of `features` when
/// given, and read the sidecar the run wrote.
fn generate(machine: &Path, out: &Path, features: Option<&[&str]>) -> Sourcemap {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sce-codegen"));
    command
        .arg("generate")
        .arg(machine)
        .args(["-l", "rust", "-o"])
        .arg(out);
    if let Some(features) = features {
        command.arg(format!("--features={}", features.join(",")));
    }
    let run = command.output().expect("spawn sce-codegen");
    assert!(
        run.status.success(),
        "generate {features:?} failed:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let text = std::fs::read_to_string(out.join("sce_sourcemap.json")).expect("a sidecar");
    sourcemap::from_json(&text).expect("the sidecar reads")
}

/// Every symbol of `variant` is the document's symbol of the same node.
fn held_to_the_document(document: &Sourcemap, variant: &Sourcemap, at: &str) {
    for (name, symbol) in &variant.symbols {
        let of_the_document = document
            .symbols
            .get(name)
            .unwrap_or_else(|| panic!("{at}: `{name}` is no symbol of the document"));
        assert_eq!(
            (symbol.line_range, &symbol.kind),
            (of_the_document.line_range, &of_the_document.kind),
            "{at}: `{name}` names another node than the document's symbol does"
        );
    }
}

#[test]
fn every_symbol_of_a_variants_sourcemap_is_the_documents_symbol_of_the_same_node() {
    let (dir, machine) = product_line();
    let document = generate(&machine, &dir.path().join("every-build"), None);
    for (n, on) in CONFIGURATIONS.iter().enumerate() {
        let variant = generate(&machine, &dir.path().join(format!("variant-{n}")), Some(on));
        held_to_the_document(&document, &variant, &format!("{on:?}"));
    }
    // The variant with no feature on keeps the last of each list, and names it
    // as the document does.
    let none = generate(&machine, &dir.path().join("none"), Some(&[]));
    for name in ["shift__idle___transition_1", "shift__idle___on_entry_2_1"] {
        assert!(
            none.symbols.contains_key(name),
            "the variant with no feature on has no `{name}`: {:?}",
            none.symbols.keys().collect::<Vec<_>>()
        );
    }
    for name in [
        "shift__idle___transition_0",
        "shift__idle___on_entry_0_0",
        "shift__idle___on_entry_1_0",
        "shift__idle___on_entry_2_0",
    ] {
        assert!(
            !none.symbols.contains_key(name),
            "the variant with no feature on names `{name}`, a node it leaves out"
        );
    }
}

#[test]
fn every_marker_of_the_output_for_every_build_names_the_symbol_of_its_row() {
    let (dir, machine) = product_line();
    let out = dir.path().join("every-build");
    let document = generate(&machine, &out, None);
    let source = std::fs::read_to_string(out.join("shift_sm.rs")).expect("the machine");
    let mut read: BTreeMap<String, u32> = BTreeMap::new();
    for line in source.lines() {
        let Some(marker) = sourcemap::read_marker(line) else {
            continue;
        };
        let marker = marker.expect("a marker the encoder wrote");
        let name = match marker.attribution.as_slice() {
            [artifact] => mangle("shift", "", artifact),
            [state, artifact] => mangle("shift", state, artifact),
            other => panic!("a marker of {} fields: {line}", other.len()),
        };
        let symbol = document
            .symbols
            .get(&name)
            .unwrap_or_else(|| panic!("`{line}` names no symbol of the sidecar"));
        assert_eq!(
            symbol.line_range[0], marker.scxml_line,
            "`{line}` names `{name}`, which the sidecar places on another row"
        );
        read.insert(name, marker.scxml_line);
    }
    assert!(
        read.contains_key("shift__idle___transition_1"),
        "no marker names the transition after the gated one: {read:?}"
    );
}
