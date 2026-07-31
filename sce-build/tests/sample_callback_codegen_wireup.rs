// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// `<sce:on-sample>` sample-delivery codegen wire-up integration tests
// (sample lifecycle surface per SCE Protocol-Synthesis RFC §synth-5-E).
//
// Pins the rendered sample-delivery surface: the per-link
// `deliver_link_X_sample` trait + impl emission shape, the
// active-state filter match arms, callback dispatch, the
// always-raised event (Rust), and the C11 mirror.
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
/// the build's `SceCrossDocRegistry`, which `compile_scxml_lang`
/// does not assemble — so this fixture compiles without a forge
/// document declaring `scout_link`. Structural validators in
/// `parser::parse_file` still run (placement / uniqueness /
/// event-name / callback-path); the fixture passes them. The
/// orchestrator-aware entry point `compile_scxml_with_imports`
/// DOES build the registry and
/// run cross-ref validation; consumers that need that surface
/// switch to that entry point.
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

fn render_c11(scxml: &str) -> (String, String) {
    let tmp = tempdir().expect("tempdir");
    let path = tmp.path().join("watcher.scxml");
    fs::write(&path, scxml).expect("write fixture");

    let tdir: PathBuf = sce_build::find_template_dir_for(sce_build::generator::Language::C11);
    let out = sce_build::compile_scxml_lang(
        path.to_str().unwrap(),
        &tdir,
        sce_build::generator::Language::C11,
    )
    .expect("c11 codegen");

    let mut header = None;
    let mut source = None;
    for (name, content) in out.files {
        if name.ends_with(".h") {
            header = Some(content);
        } else if name.ends_with(".c") {
            source = Some(content);
        }
    }
    (header.expect("missing .h"), source.expect("missing .c"))
}

#[test]
fn w1_2_emits_link_rx_trait_and_impl() {
    // Contract: when any state has `<sce:on-sample link="X">`, the
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
    // Generic over M: SampleMeta (codegen does not
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
    // Contract: the body iterates the engine's active configuration
    // and dispatches a per-state match arm for every state whose
    // `<sce:on-sample link="X">` matches this link. The arm-body
    // content (callback dispatch + event raise) is pinned by the
    // tests below; this test just pins the structural surface.
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
        .unwrap_or_else(|e| panic!("match-arm body fails syn parse: {e}\n{code}"));
}

#[test]
fn w1_3_multi_state_same_link_emits_arms_per_state() {
    // Multiple states may subscribe to the same
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
    // Contract: when `<sce:on-sample callback="rust:...">`
    // is present, the per-state match arm body emits
    // `<bare-rust-path>(&sample);` — the `rust:` language prefix
    // is stripped so rustc resolves the user's module path
    // directly. Callback signature enforcement flows through
    // rustc at the consumer crate's compile time;
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
        .unwrap_or_else(|e| panic!("callback emission fails syn parse: {e}\n{code}"));
}

#[test]
fn w1_4_no_callback_emits_no_call_site() {
    // Negative case: `<sce:on-sample link="X" event="Y">` without a
    // callback attribute must not emit any function-call line in the
    // arm body — only the always-raised event remains.
    let code = render_rust(FIXTURE);
    assert!(
        !code.contains("(&sample);"),
        "fixture without callback must not emit call site:\n{code}"
    );
}

#[test]
fn on_sample_emits_event_raise_always() {
    // Contract: every per-state arm emits
    // `self.raise_external_by_name("<event>", "")` regardless of
    // whether a callback is present. Event-data is empty
    // (typed payload flows through callback path; event
    // path is for SCXML transition reaction only).

    // Case A: no callback — only event raise in arm body.
    let no_cb_code = render_rust(FIXTURE);
    assert!(
        no_cb_code.contains("self.raise_external_by_name(\"scout.tick\", \"\");"),
        "missing event-raise call site in callback-absent fixture:\n{no_cb_code}"
    );

    // Case B: with callback — both callback and event raise present
    // (callback fires synchronously first, then event).
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
        "callback must precede event raise (callback synchronously, then event)"
    );

    syn::parse_file(&with_cb_code)
        .unwrap_or_else(|e| panic!("event-raise emission fails syn parse: {e}\n{with_cb_code}"));
}

// ── C11 mirror ──────────────────────────────────────────────────

#[test]
fn w2_c11_emits_per_link_deliver_function() {
    // Contract: C11 1:1 mirror of the Rust sample-delivery
    // surface pinned above. The header
    // declares one `<machine>_deliver_link_<x>_sample(sm, sample)`
    // per `<sce:on-sample link>` and gates the `<sce/sample.h>`
    // include on the same condition. The .c file emits the
    // definition with active-state filter (`_in_state` predicate)
    // + event raise via `_raise_external` + `event_with_meta_t`
    // setup. The event is always raised. The callback path
    // is Rust-only; the C11 backend ignores
    // `<sce:on-sample callback="rust:...">` (a `rust:`-prefixed
    // path has no C11 lowering).
    let (header, source) = render_c11(FIXTURE);

    // Header
    assert!(
        header.contains("#include \"sce/sample.h\""),
        "missing sample.h include in header:\n{header}"
    );
    assert!(
        header.contains("_deliver_link_scout_link_sample("),
        "missing per-link function declaration:\n{header}"
    );
    assert!(
        header.contains("const sce_sample_t *sample"),
        "missing sce_sample_t borrow parameter:\n{header}"
    );

    // Source
    assert!(
        source.contains("_deliver_link_scout_link_sample("),
        "missing per-link function definition:\n{source}"
    );
    assert!(
        source.contains("_in_state(sm, ") && source.contains("_STATE_RUNNING)"),
        "missing active-state filter via _in_state predicate:\n{source}"
    );
    assert!(
        source.contains("_raise_external(sm, &_on_sample_evt);"),
        "missing event raise call:\n{source}"
    );
    assert!(
        source.contains("_EVENT_SCOUT_TICK"),
        "missing event enum value (analyzer must register on-sample events into model.events):\n{source}"
    );
}

#[test]
fn w2_c11_skipped_when_no_on_sample_blocks() {
    // Negative case: documents without `<sce:on-sample>` must not
    // include `<sce/sample.h>` (header-include hygiene) and must
    // not declare any deliver function. This pins the elision
    // contract symmetric to the Rust w1_2 negative test.
    const NO_ON_SAMPLE: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0"
       initial="running"
       datamodel="ecmascript"
       name="quiet">
  <state id="running"/>
</scxml>
"##;
    let (header, source) = render_c11(NO_ON_SAMPLE);
    assert!(
        !header.contains("sce/sample.h"),
        "sample.h must elide for documents without <sce:on-sample>:\n{header}"
    );
    assert!(
        !header.contains("deliver_link_") && !source.contains("deliver_link_"),
        "deliver_link_* must elide for documents without <sce:on-sample>"
    );
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
