// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Foreign-namespace handling — locks the behavior documented in
// `SCE_FORGE.md` §3.1 "Foreign Namespace Policy (non-`sce:` extensions)".
// The doc table makes three concrete claims:
//
//   1. XSD validation preserves foreign-namespace elements / attributes
//      (`<xs:any namespace="##any" processContents="lax">` + matching
//      `<xs:anyAttribute>`). No diagnostic is raised.
//
//   2. SCXML → IR parsing drops foreign-namespace elements whose local
//      name is outside the W3C SCXML vocabulary, because the parser
//      dispatches by local element name and has no model slot for
//      unrecognised children.
//
//   3. Caveat: a foreign-namespace element whose local name COLLIDES
//      with a W3C name (e.g. `<framework:onentry>`) is currently
//      matched as if it were the W3C element. This is documented as a
//      footgun, not a bug to be silently fixed — if it changes, the
//      doc must change with it.
//
// Without these tests a later parser refactor could silently invert any
// of the three claims while the doc keeps asserting them. The doc
// becomes a regression vector rather than a contract.

use std::fs;
use std::path::Path;

use tempfile::tempdir;

use sce_build::compile_scxml_lang;
use sce_build::generator::Language;

fn template_dir() -> std::path::PathBuf {
    sce_build::find_template_dir_for(Language::Rust)
}

fn write_doc(dir: &Path, basename: &str, content: &str) -> std::path::PathBuf {
    let path = dir.join(basename);
    fs::write(&path, content).expect("write scxml");
    path
}

/// Doc claim #1 + #2: foreign-namespace attribute and foreign-namespace
/// child element with a non-W3C local name pass XSD validation
/// (preserve) and do not surface in the generated code (IR drop).
#[test]
fn foreign_ns_attribute_and_unique_local_name_child_pass_xsd_and_drop_from_ir() {
    let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:framework="http://example.com/framework"
       version="1.0" initial="s1">
  <state id="s1" framework:meta="hello-from-framework">
    <framework:widget id="w1" payload="framework-only-marker"/>
    <transition event="go" target="s2"/>
  </state>
  <final id="s2"/>
</scxml>
"##;

    let dir = tempdir().expect("tempdir");
    let path = write_doc(dir.path(), "foreign_ns_unique.scxml", scxml);
    let tdir = template_dir();

    let out = compile_scxml_lang(path.to_str().unwrap(), &tdir, Language::Rust)
        .expect("foreign-NS attribute + unique-local-name child must pass XSD (preserve) and parser");

    let code = out
        .files
        .iter()
        .map(|(_, content)| content.as_str())
        .collect::<String>();

    // IR drop: the foreign-namespace local names and their attribute
    // values must not reach the generated source. If any of these
    // appear, the parser has started preserving foreign-NS nodes —
    // re-evaluate the SCE_FORGE.md §3.1 claim before adjusting this
    // assertion.
    assert!(
        !code.contains("widget"),
        "foreign-NS child <framework:widget> leaked into generated code"
    );
    assert!(
        !code.contains("framework-only-marker"),
        "foreign-NS attribute payload leaked into generated code"
    );
    assert!(
        !code.contains("hello-from-framework"),
        "foreign-NS attribute value leaked into generated code"
    );
}

/// Doc claim #3 (collision caveat): a foreign-namespace element whose
/// local name collides with a W3C SCXML element (`onentry`) is
/// currently matched by the parser as if it were the W3C element,
/// because dispatch is by local name only. The `<log>` action inside
/// it therefore reaches the generated code.
///
/// This test LOCKS the documented footgun. If the parser is later
/// hardened to filter by namespace, this test will fail; at that
/// point the SCE_FORGE.md §3.1 caveat row must be updated in the
/// same change so the doc and code stay consistent.
#[test]
fn foreign_ns_w3c_local_name_collision_is_matched_as_w3c_element() {
    let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:framework="http://example.com/framework"
       version="1.0" initial="s1">
  <state id="s1">
    <framework:onentry>
      <log expr="'collision-caveat-locked'"/>
    </framework:onentry>
    <transition event="go" target="s2"/>
  </state>
  <final id="s2"/>
</scxml>
"##;

    let dir = tempdir().expect("tempdir");
    let path = write_doc(dir.path(), "foreign_ns_collision.scxml", scxml);
    let tdir = template_dir();

    let out = compile_scxml_lang(path.to_str().unwrap(), &tdir, Language::Rust)
        .expect("compile must succeed for collision case (parser does not reject)");

    let code = out
        .files
        .iter()
        .map(|(_, content)| content.as_str())
        .collect::<String>();

    // Expect the action body to leak through because the parser
    // matches `<framework:onentry>` as W3C `<onentry>`. If this
    // assertion ever fails because the marker is absent, foreign-NS
    // filtering has been tightened — update SCE_FORGE.md §3.1
    // caveat row before adjusting.
    assert!(
        code.contains("collision-caveat-locked"),
        "expected the <log> inside <framework:onentry> to be picked up as a W3C <onentry> action; \
         if this assertion now fails, the parser has been hardened to filter by namespace and \
         SCE_FORGE.md §3.1 caveat row must be updated to match"
    );
}
