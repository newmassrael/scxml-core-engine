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

// ── `c:` callback axis (C11 side of the symmetry) ───────────────

/// The `rust:` arm has always emitted a synchronous call into a
/// host-supplied function; the C11 backend carried the same
/// `<sce:on-sample callback>` attribute and silently dropped it, so a
/// C11 deployment could declare a callback and get nothing but the
/// event raise. `c:` closes that.
///
/// The emitted call matches `sce_sub_callback_t` in `<sce/sample.h>`
/// exactly — `(const sce_sample_t *, void *ctx)` with the statechart as
/// the context — so one host function satisfies both this generated
/// dispatch and a direct registration on the link's RX path. An
/// adapter shim between the two would be the thing that drifts.
const C_CALLBACK_FIXTURE: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0"
       initial="running"
       datamodel="ecmascript"
       sce:kind="statechart"
       name="watcher">
  <state id="running">
    <sce:on-sample link="scout_link" event="scout.tick"
                   callback="c:app_on_scout"/>
    <transition event="scout.tick" target="running"/>
  </state>
</scxml>
"##;

#[test]
fn c_callback_emits_dispatch_and_prototype_on_c11() {
    let (header, source) = render_c11(C_CALLBACK_FIXTURE);

    // The prototype has to be emitted, not assumed: without it the
    // call below is an implicit declaration, which C99 onward rejects
    // and which would otherwise let a mismatched host signature link.
    assert!(
        header.contains("void app_on_scout(const sce_sample_t *sample"),
        "header must declare the host callback with the sample borrow:\n{header}"
    );
    assert!(
        header.contains("void *ctx);"),
        "prototype must match sce_sub_callback_t's context parameter:\n{header}"
    );

    // Call site: context is the statechart instance, so a callback can
    // reach the machine that received the sample without the host
    // threading its own registry.
    assert!(
        source.contains("app_on_scout(sample, sm);"),
        "missing C callback dispatch line:\n{source}"
    );
    // The language prefix must not survive into C, where `:` is not
    // part of an identifier.
    assert!(
        !source.contains("c:app_on_scout"),
        "language prefix leaked into the call site:\n{source}"
    );

    // The event raise is unconditional on both axes; a callback
    // replaces neither the raise nor its ordering.
    let cb_pos = source.find("app_on_scout(sample, sm);").expect("callback");
    let raise_pos = source
        .find("_raise_external(sm, &_on_sample_evt);")
        .expect("raise");
    assert!(cb_pos < raise_pos, "callback must precede the event raise");
}

#[test]
fn rust_backend_ignores_a_c_callback() {
    // Symmetric to the C11 backend ignoring `rust:`: a `c:` path has no
    // Rust lowering, and emitting it would produce a call to an
    // identifier rustc cannot resolve. The event raise still happens,
    // so the document remains meaningful on both backends.
    let code = render_rust(C_CALLBACK_FIXTURE);
    assert!(
        !code.contains("app_on_scout"),
        "a c: callback must not reach the Rust call site:\n{code}"
    );
    assert!(
        code.contains("self.raise_external_by_name(\"scout.tick\", \"\");"),
        "the event raise must survive on the backend that skips the callback:\n{code}"
    );
    syn::parse_file(&code).unwrap_or_else(|e| panic!("c-callback fixture must still parse: {e}"));
}

#[test]
fn c11_backend_still_ignores_a_rust_callback() {
    // The pre-existing half of the symmetry, pinned so adding `c:`
    // cannot accidentally make C11 emit a Rust path.
    const RUST_CALLBACK: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
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
    let (header, source) = render_c11(RUST_CALLBACK);
    assert!(
        !source.contains("my_app") && !header.contains("my_app"),
        "a rust: callback must not reach the C11 output"
    );
    assert!(
        source.contains("_raise_external(sm, &_on_sample_evt);"),
        "the event raise must survive on the backend that skips the callback:\n{source}"
    );
}

/// The three string assertions above prove the text is emitted; they
/// cannot tell an emitted prototype that compiles from one that does
/// not. `SCE_PARAM_TYPESTATE` in particular is only in scope because
/// `<sce/sample.h>` is included ahead of the prototype — an ordering a
/// `contains` check would never notice breaking.
///
/// The negative half is the point: a host whose definition disagrees
/// with the generated prototype must fail its own compile. Without the
/// emitted prototype the call site would be an implicit declaration and
/// the mismatch would reach the linker instead.
#[test]
fn c_callback_output_compiles_and_pins_the_host_signature() {
    use std::process::Command;

    let Some(cc) = which("gcc").or_else(|| which("cc")) else {
        eprintln!("SKIP c_callback_output_compiles: no gcc/cc on PATH");
        return;
    };

    let tmp = tempdir().expect("tempdir");
    let dir = tmp.path();
    let src = dir.join("watcher.scxml");
    fs::write(&src, C_CALLBACK_FIXTURE).expect("write fixture");
    let tdir: PathBuf = sce_build::find_template_dir_for(sce_build::generator::Language::C11);
    let out = sce_build::compile_scxml_lang(
        src.to_str().unwrap(),
        &tdir,
        sce_build::generator::Language::C11,
    )
    .expect("c11 codegen");
    for (name, content) in &out.files {
        fs::write(dir.join(name), content).expect("write generated");
    }

    let runtime_inc = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("backends/c/runtime/include");

    let syntax_only = |file: &std::path::Path| -> Result<(), String> {
        let out = Command::new(&cc)
            .args(["-std=c11", "-Wall", "-Wextra", "-Werror", "-fsyntax-only"])
            .arg("-I")
            .arg(&runtime_inc)
            .arg("-I")
            .arg(dir)
            .arg(file)
            .output()
            .expect("run cc");
        if out.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).into_owned())
        }
    };

    let generated_c = dir.join("watcher_sm.c");
    syntax_only(&generated_c)
        .unwrap_or_else(|e| panic!("generated C must compile clean at -Werror:\n{e}"));

    // A conforming host definition compiles against the prototype.
    let host_ok = dir.join("host_ok.c");
    fs::write(
        &host_ok,
        "#include \"watcher_sm.h\"\n\
         void app_on_scout(const sce_sample_t *sample, void *ctx) { (void)sample; (void)ctx; }\n",
    )
    .expect("write host");
    syntax_only(&host_ok)
        .unwrap_or_else(|e| panic!("a conforming host callback must compile:\n{e}"));

    // A host that drops `ctx` must not. If this ever passes, the
    // prototype stopped being emitted and the mismatch moved to link
    // time, where nothing reports it.
    let host_bad = dir.join("host_bad.c");
    fs::write(
        &host_bad,
        "#include \"watcher_sm.h\"\n\
         void app_on_scout(const sce_sample_t *sample) { (void)sample; }\n",
    )
    .expect("write host");
    let err = syntax_only(&host_bad)
        .expect_err("a host signature that disagrees with the prototype must fail to compile");
    assert!(
        err.contains("conflicting types") || err.contains("incompatible"),
        "expected a conflicting-declaration diagnostic, got:\n{err}"
    );
}

fn which(tool: &str) -> Option<PathBuf> {
    let out = std::process::Command::new("which")
        .arg(tool)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let p = String::from_utf8(out.stdout).ok()?.trim().to_string();
    if p.is_empty() {
        None
    } else {
        Some(PathBuf::from(p))
    }
}

/// SCE Protocol-Synthesis RFC §synth-5-E lines 1367-1378 + 1462-1484:
/// the runtime ownership boundary must enclose the host callback
/// in the generated delivery function.
///
/// This is the position that makes the layers work at all. Clang's
/// consumed analysis stops at the indirect call into host code — and
/// on the C API it never started, since the typestate attributes do
/// not apply there. The generated dispatch is the one place SCE still
/// controls when the borrow crosses into code it did not write, so the
/// enter/exit pair has to bracket that call rather than sit anywhere
/// else in the function.
///
/// Ordering is the assertion: an exit emitted before the callback
/// would poison a payload the handler is about to read, and an enter
/// emitted after it would validate a borrow that has already been
/// handed over.
#[test]
fn c11_deliver_function_brackets_the_callback_with_the_ownership_boundary() {
    let (_header, source) = render_c11(C_CALLBACK_FIXTURE);

    let enter = source
        .find("sce_ownership_callback_enter(sample)")
        .expect("delivery function must open an ownership scope");
    let callback = source
        .find("app_on_scout(sample, sm);")
        .expect("callback dispatch");
    let exit = source
        .find("sce_ownership_callback_exit(&_own, sample)")
        .expect("delivery function must close the ownership scope");

    assert!(
        enter < callback && callback < exit,
        "the boundary must bracket the callback (enter={enter}, \
         callback={callback}, exit={exit}):\n{source}"
    );

    // Exactly one pair per delivery function: a duplicated enter would
    // re-read a borrow already validated, and a duplicated exit would
    // poison twice — the second time against memory the pool may have
    // handed to another slot holder.
    assert_eq!(
        source.matches("sce_ownership_callback_enter(").count(),
        1,
        "exactly one scope open per delivery function:\n{source}"
    );
    assert_eq!(
        source.matches("sce_ownership_callback_exit(").count(),
        1,
        "exactly one scope close per delivery function:\n{source}"
    );

    // Both sides sit behind the same switch, so a build with the
    // layers off gets neither half — a lone enter would leave `_own`
    // set and unused, which `-Werror` consumers reject.
    assert_eq!(
        source.matches("#if SCE_OWNERSHIP_CHECKED").count(),
        2,
        "enter and exit must each be gated on SCE_OWNERSHIP_CHECKED:\n{source}"
    );
}
