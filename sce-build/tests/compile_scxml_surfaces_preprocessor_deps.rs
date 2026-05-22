// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Pins the contract that the typed codegen entry point surfaces every
// preprocessor dependency (xi:include targets, sce:use template
// fragments) on `GeneratedOutput.deps` — the canonical channel that
// the `compile_scxml` build.rs facade forwards to Cargo via
// `cargo::rerun-if-changed=`.
//
// `tests/preprocessor_depfile.rs` already pins the parser-side
// surface (`SCXMLParser::preprocessor_deps()`); this file pins the
// next layer up — that the same dep set survives the parse boundary
// inside `compile_model` and attaches to the per-entry codegen
// output. Without this guard, a future refactor that drops the
// `parser.preprocessor_deps().to_vec()` capture inside `compile_model`
// would silently regress the `<sce:use>` / `<xi:include>` rebuild
// behaviour the original report (pinion build.rs `Carry: vendor/sce
// RFC` workaround) flagged.

use sce_build::{compile_scxml_lang_typed, find_template_dir_for, generator::Language};
use std::fs;
use tempfile::tempdir;

fn write_file(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    fs::write(&path, body).expect("write fixture file");
    fs::canonicalize(&path).expect("canonicalize fixture path")
}

#[test]
fn typed_entry_surfaces_sce_use_fragment_dep() {
    // Smallest end-to-end shape: host -> one `<sce:use>` fragment.
    // The codegen output's `deps` must carry the fragment so the
    // build.rs facade can emit `cargo::rerun-if-changed=` for it.
    let tmp = tempdir().expect("tempdir");
    let fragment = write_file(
        tmp.path(),
        "shared.sce-template.xml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<sce:template xmlns="http://www.w3.org/2005/07/scxml"
              xmlns:sce="http://sce.dev/ext"
              name="shared">
  <state id="phase1">
    <transition event="tick" target="pass"/>
  </state>
  <final id="pass"/>
</sce:template>
"#,
    );
    let host = write_file(
        tmp.path(),
        "host.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" name="host" initial="phase1">
  <sce:use template="shared.sce-template.xml"/>
</scxml>
"#,
    );

    let template_dir = find_template_dir_for(Language::Rust);
    let output = compile_scxml_lang_typed(host.to_str().unwrap(), &template_dir, Language::Rust)
        .expect("compile_scxml_lang_typed: host with <sce:use> must succeed");

    assert_eq!(
        output.deps.len(),
        1,
        "exactly one fragment was opened by the parser; got {:?}",
        output.deps,
    );
    assert_eq!(
        output.deps[0], fragment,
        "GeneratedOutput.deps[0] must match the canonical fragment path that \
         Parser::preprocessor_deps() captured at the parse boundary"
    );
}

#[test]
fn typed_entry_surfaces_xinclude_fragment_dep() {
    // Sibling pipeline to <sce:use>: XInclude expansion is the other
    // preprocessor stage that contributes to `preprocessor_deps`. The
    // typed entry must surface it on the same `deps` channel — the
    // build.rs facade does not distinguish the two stages and emits
    // one `rerun-if-changed=` per dep regardless of origin.
    let tmp = tempdir().expect("tempdir");
    // XInclude expansion splices the *children* of the included
    // document's root element into the host (the root wrapper is
    // discarded — see `xinclude::render_root_children`). The fragment
    // therefore wraps its real payload in a `<fragment>` shell whose
    // sole purpose is to be stripped at splice time, leaving just the
    // `<transition>` child to land directly inside `<state id="phase1">`.
    // Without the wrapper the include would be a no-op and phase1 would
    // gain no transitions, which `scxml_reachability::validate`
    // (NL→IR Mapping Roadmap Item 3 Phase A) now catches as the
    // orphan `<final id="pass">`.
    let fragment = write_file(
        tmp.path(),
        "frag.xml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<fragment>
  <transition event="tick" target="pass" xmlns="http://www.w3.org/2005/07/scxml"/>
</fragment>
"#,
    );
    let host = write_file(
        tmp.path(),
        "host_xi.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:xi="http://www.w3.org/2001/XInclude"
       version="1.0" name="host_xi" initial="phase1">
  <state id="phase1">
    <xi:include href="frag.xml"/>
  </state>
  <final id="pass"/>
</scxml>
"#,
    );

    let template_dir = find_template_dir_for(Language::Rust);
    let output = compile_scxml_lang_typed(host.to_str().unwrap(), &template_dir, Language::Rust)
        .expect("compile_scxml_lang_typed: host with <xi:include> must succeed");

    assert!(
        output.deps.contains(&fragment),
        "GeneratedOutput.deps must contain the xi:include target; got {:?}",
        output.deps,
    );
}

#[test]
fn typed_entry_surfaces_empty_deps_for_pure_scxml() {
    // Regression guard symmetric to `preprocessor_deps_empty_for_pure_scxml`
    // in `tests/preprocessor_depfile.rs`: a host with no preprocessor
    // inputs must produce an empty `deps`. Without this, a future
    // refactor that always-populates `deps` (e.g. with the host path
    // itself, which would be a categorical mistake — the host is
    // already covered by the per-input `rerun-if-changed=` line that
    // `compile_scxml` emits) would silently double-count rerun rows.
    let tmp = tempdir().expect("tempdir");
    let host = write_file(
        tmp.path(),
        "plain.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" name="plain" initial="s">
  <state id="s">
    <transition event="go" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#,
    );

    let template_dir = find_template_dir_for(Language::Rust);
    let output = compile_scxml_lang_typed(host.to_str().unwrap(), &template_dir, Language::Rust)
        .expect("plain SCXML compiles");

    assert!(
        output.deps.is_empty(),
        "documents without preprocessor inputs must surface empty deps; got {:?}",
        output.deps,
    );
}
