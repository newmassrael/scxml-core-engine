// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// watching-zenoh RFC §5.E B7-η' codegen wire-up integration tests.
//
// Each W1.N atomic in `claudedocs/rfc-b7-eta-prime-codegen-wireup.md`
// adds one structural assertion to the rendered Rust output. W1.2
// (this file's only test today) pins the per-link `deliver_link_X_sample`
// trait + impl emission shape; W1.3-1.5 will add body-content
// assertions in the same harness.
//
// Pipeline: write SCXML to a tempdir → `compile_scxml_lang` runs
// parse + analyze + render → assert structural fragments appear in the
// generated `.rs` file → `syn::parse_file` proves the output is
// syntactically valid Rust (drift gate against template typos).

use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

/// Minimal SCXML with one `<sce:on-sample>` block. Cross-ref
/// validation (`validate_on_sample_link_references`) is gated on
/// the build's `ForgeLinkRegistry`, which `compile_scxml_lang`
/// does not assemble — so this fixture compiles without a forge
/// document declaring `scout_link`. Structural validators in
/// `parser::parse_file` still run (placement / uniqueness /
/// event-name / callback-path); the fixture passes them.
const FIXTURE: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0"
       initial="running"
       datamodel="ecmascript"
       sce:kind="statechart"
       name="watcher">
  <state id="running">
    <sce:on-sample link="scout_link" event="scout.tick"/>
    <transition event="scout.tick" target="running"/>
  </state>
</scxml>
"##;

fn render_rust(scxml: &str) -> String {
    let tmp = tempdir().expect("tempdir");
    let path = tmp.path().join("watcher.scxml");
    fs::write(&path, scxml).expect("write fixture");

    let tdir: PathBuf = sce_build::find_template_dir_for(sce_build::generator::Language::Rust);
    let out = sce_build::compile_scxml_lang(
        path.to_str().unwrap(),
        &tdir,
        sce_build::generator::Language::Rust,
    )
    .expect("rust codegen");

    out.files.into_iter().next().expect("at least one file").1
}

#[test]
fn w1_2_emits_link_rx_trait_and_impl() {
    // W1.2 contract: when any state has `<sce:on-sample link="X">`, the
    // Rust state_machine template emits the `<MachineName>LinkRx` trait
    // and its `impl … for Engine<...Policy>` block, with one
    // `deliver_link_<x>_sample` method per unique link name.
    let code = render_rust(FIXTURE);

    assert!(
        code.contains("pub trait WatcherLinkRx"),
        "missing LinkRx trait declaration:\n{code}"
    );
    assert!(
        code.contains("impl WatcherLinkRx for ::sce_rust_runtime::Engine<WatcherPolicy>"),
        "missing LinkRx impl block:\n{code}"
    );
    assert!(
        code.contains("fn deliver_link_scout_link_sample"),
        "missing per-link deliver method:\n{code}"
    );
    // Generic over M: SampleMeta — Q-Wire-9 lock (codegen does not
    // know link's concrete metadata type).
    assert!(
        code.contains("M: ::sce_link_runtime::SampleMeta"),
        "missing SampleMeta where bound:\n{code}"
    );

    // Drift gate: rendered output must parse as syntactically-valid
    // Rust. Catches stray jinja tokens, misplaced delimiters, etc.
    syn::parse_file(&code)
        .unwrap_or_else(|e| panic!("rendered LinkRx surface fails syn parse: {e}\n{code}"));
}

#[test]
fn w1_2_skipped_when_no_on_sample_blocks() {
    // Templates must elide the LinkRx surface entirely when no state
    // declares `<sce:on-sample>`. Negative case: rendering must not
    // mention `LinkRx` or `deliver_link_` at all, and must not import
    // `sce_link_runtime`.
    const NO_ON_SAMPLE: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0"
       initial="running"
       datamodel="ecmascript"
       name="quiet">
  <state id="running"/>
</scxml>
"##;
    let code = render_rust(NO_ON_SAMPLE);

    assert!(
        !code.contains("LinkRx"),
        "LinkRx surface must elide for documents without <sce:on-sample>:\n{code}"
    );
    assert!(
        !code.contains("deliver_link_"),
        "deliver_link_* must elide for documents without <sce:on-sample>:\n{code}"
    );
}
