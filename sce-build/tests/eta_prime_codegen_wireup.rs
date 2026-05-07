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
fn w1_3_emits_active_state_filter_match_arm() {
    // W1.3 contract: the body iterates the engine's active configuration
    // and dispatches a per-state match arm for every state whose
    // `<sce:on-sample link="X">` matches this link. W1.4/1.5 fill the
    // arm bodies; W1.3 just pins the structural surface.
    let code = render_rust(FIXTURE);

    assert!(
        code.contains("self.get_active_states()"),
        "missing active configuration iteration:\n{code}"
    );
    assert!(
        code.contains("WatcherState::Running =>"),
        "missing per-state match arm for Running:\n{code}"
    );
    assert!(
        code.contains("_ => {}"),
        "missing default match arm:\n{code}"
    );
    syn::parse_file(&code)
        .unwrap_or_else(|e| panic!("W1.3 body fails syn parse: {e}\n{code}"));
}

#[test]
fn w1_3_multi_state_same_link_emits_arms_per_state() {
    // Q-OnSample-5 (a) allows multiple states to subscribe to the same
    // link (uniqueness is per-state-per-link, not per-link). The deliver
    // method must emit a match arm for EACH such state.
    const MULTI_STATE: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0"
       initial="watching"
       datamodel="ecmascript"
       sce:kind="statechart"
       name="watcher">
  <state id="watching">
    <sce:on-sample link="scout_link" event="scout.tick"/>
    <transition event="scout.tick" target="settling"/>
  </state>
  <state id="settling">
    <sce:on-sample link="scout_link" event="settle.tick"/>
    <transition event="settle.tick" target="watching"/>
  </state>
</scxml>
"##;
    let code = render_rust(MULTI_STATE);
    assert!(
        code.contains("WatcherState::Watching =>"),
        "missing arm for Watching:\n{code}"
    );
    assert!(
        code.contains("WatcherState::Settling =>"),
        "missing arm for Settling:\n{code}"
    );
    syn::parse_file(&code)
        .unwrap_or_else(|e| panic!("multi-state body fails syn parse: {e}\n{code}"));
}

#[test]
fn w1_4_emits_callback_dispatch_with_stripped_prefix() {
    // W1.4 contract: when `<sce:on-sample callback="rust:...">`
    // is present, the per-state match arm body emits
    // `<bare-rust-path>(&sample);` — the `rust:` language prefix
    // is stripped (Q-Callback-2 v1) so rustc resolves the user's
    // module path directly. Q-Callback-3's signature enforcement
    // flows through rustc at the consumer crate's compile time;
    // this test pins the codegen surface only.
    const WITH_CALLBACK: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0"
       initial="running"
       datamodel="ecmascript"
       sce:kind="statechart"
       name="watcher">
  <state id="running">
    <sce:on-sample link="scout_link" event="scout.tick"
                   callback="rust:my_app::on_scout"/>
    <transition event="scout.tick" target="running"/>
  </state>
</scxml>
"##;
    let code = render_rust(WITH_CALLBACK);

    // The bare path appears as a function call site, with `&sample`.
    assert!(
        code.contains("my_app::on_scout(&sample);"),
        "missing callback dispatch line:\n{code}"
    );
    // The `rust:` language prefix MUST be stripped at the call site.
    // Doc-comments (e.g. `callback="rust:..."`) are allowed; what
    // matters is that the actual emitted Rust expression is the bare
    // path, not `rust:my_app::on_scout(&sample)` which would not
    // compile.
    assert!(
        !code.contains("rust:my_app::on_scout"),
        "language prefix leaked into call-site path:\n{code}"
    );
    syn::parse_file(&code)
        .unwrap_or_else(|e| panic!("W1.4 callback emission fails syn parse: {e}\n{code}"));
}

#[test]
fn w1_4_no_callback_emits_no_call_site() {
    // Negative case: `<sce:on-sample link="X" event="Y">` without a
    // callback attribute must not emit any function-call line in the
    // arm body — only the W1.5 event-raise placeholder remains.
    let code = render_rust(FIXTURE);
    assert!(
        !code.contains("(&sample);"),
        "fixture without callback must not emit call site:\n{code}"
    );
}

#[test]
fn w1_5_emits_event_raise_always() {
    // W1.5 contract: per Q-Wire-3 lock, every per-state arm emits
    // `self.raise_external_by_name("<event>", "")` regardless of
    // whether a callback is present. Event-data is empty per
    // Q-Wire-4 (typed payload flows through callback path; event
    // path is for SCXML transition reaction only).

    // Case A: no callback — only event raise in arm body.
    let no_cb_code = render_rust(FIXTURE);
    assert!(
        no_cb_code.contains("self.raise_external_by_name(\"scout.tick\", \"\");"),
        "missing event-raise call site in callback-absent fixture:\n{no_cb_code}"
    );

    // Case B: with callback — both callback and event raise present
    // (Q-Wire-3: callback fires synchronously first, then event).
    const WITH_CALLBACK: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0"
       initial="running"
       datamodel="ecmascript"
       sce:kind="statechart"
       name="watcher">
  <state id="running">
    <sce:on-sample link="scout_link" event="scout.tick"
                   callback="rust:my_app::on_scout"/>
    <transition event="scout.tick" target="running"/>
  </state>
</scxml>
"##;
    let with_cb_code = render_rust(WITH_CALLBACK);
    assert!(
        with_cb_code.contains("my_app::on_scout(&sample);"),
        "missing callback line in callback-present fixture:\n{with_cb_code}"
    );
    assert!(
        with_cb_code.contains("self.raise_external_by_name(\"scout.tick\", \"\");"),
        "missing event-raise line in callback-present fixture:\n{with_cb_code}"
    );

    // Drift gate: order in source is callback before event.
    let cb_pos = with_cb_code
        .find("my_app::on_scout(&sample);")
        .expect("callback position");
    let raise_pos = with_cb_code
        .find("self.raise_external_by_name(\"scout.tick\", \"\");")
        .expect("event-raise position");
    assert!(
        cb_pos < raise_pos,
        "callback must precede event raise per Q-Wire-3 (callback synchronously, then event)"
    );

    syn::parse_file(&with_cb_code)
        .unwrap_or_else(|e| panic!("W1.5 emission fails syn parse: {e}\n{with_cb_code}"));
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
