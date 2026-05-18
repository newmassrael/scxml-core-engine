// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Foreign-namespace handling — locks the behavior documented in
// `SCE_FORGE.md` §3.1 "Foreign Namespace Policy (non-`sce:` extensions)".
// The doc table makes two concrete claims:
//
//   1. XSD validation preserves foreign-namespace elements / attributes
//      (`<xs:any namespace="##any" processContents="lax">` + matching
//      `<xs:anyAttribute>`). No diagnostic is raised.
//
//   2. SCXML → IR parsing drops foreign-namespace elements unconditionally,
//      whether their local name is outside the W3C vocabulary OR collides
//      with a W3C element name. Filtering happens at the parser-helper
//      level (`scxml_child` / `scxml_children` in `parser.rs`) by both
//      local name AND namespace via `is_scxml_ns`.
//
// Without these tests a later parser refactor could silently invert
// either claim while the doc keeps asserting them. The doc becomes a
// regression vector rather than a contract.

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

    let out = compile_scxml_lang(path.to_str().unwrap(), &tdir, Language::Rust).expect(
        "foreign-NS attribute + unique-local-name child must pass XSD (preserve) and parser",
    );

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

/// Strict-namespace enforcement: an SCXML document that omits the
/// root `xmlns="http://www.w3.org/2005/07/scxml"` declaration MUST
/// be rejected. W3C SCXML §3.5 requires the namespace, and
/// `xsd_validator::validate_or_skip` (run on every input at the
/// `parse_impl` boundary) raises `xml/schema-validation` before the
/// parser-helper namespace filter is ever consulted. If a future
/// refactor relaxes this — for example, by skipping XSD on some path
/// or making `is_scxml_ns` lenient on `None` — this test will pass
/// when it should fail; at that point the strictness chain in both
/// SCE_FORGE.md §3.1 and the parser must be re-examined.
#[test]
fn scxml_without_xmlns_declaration_is_rejected_by_xsd() {
    let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml version="1.0" initial="s1">
  <state id="s1">
    <transition event="go" target="s2"/>
  </state>
  <final id="s2"/>
</scxml>
"##;

    let dir = tempdir().expect("tempdir");
    let path = write_doc(dir.path(), "no_xmlns_decl.scxml", scxml);
    let tdir = template_dir();

    // `GeneratedOutput` is not Debug, so `expect_err` doesn't apply;
    // hand-roll the Ok-rejection so the error string is still in scope.
    let result = compile_scxml_lang(path.to_str().unwrap(), &tdir, Language::Rust);
    let err = match result {
        Ok(_) => {
            panic!("SCXML without xmlns must be rejected by XSD validation, but compile succeeded")
        }
        Err(e) => e,
    };
    assert!(
        err.contains("No matching global declaration")
            || err.to_lowercase().contains("xmlns")
            || err.to_lowercase().contains("validation"),
        "expected schema-validation error citing xmlns / global declaration, got: {err}"
    );
}

/// Doc claim #2 (collision case): a foreign-namespace element whose
/// local name collides with a W3C SCXML element (`onentry`) is
/// dropped by the parser, because `scxml_child` / `scxml_children`
/// filter by namespace via [`is_scxml_ns`] in addition to local
/// name. The `<log>` action inside it therefore does NOT reach the
/// generated code, identically to the non-colliding case above.
///
/// This test locks the namespace-filter contract. If a parser
/// refactor regresses this — e.g. by reverting the helpers to local-
/// name-only matching — this assertion will fail; at that point the
/// regression must be fixed at the helper, not by adjusting the test.
#[test]
fn foreign_ns_w3c_local_name_collision_is_still_dropped() {
    let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:framework="http://example.com/framework"
       version="1.0" initial="s1">
  <state id="s1">
    <framework:onentry>
      <log expr="'collision-marker-must-not-leak'"/>
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
        .expect("compile must succeed — foreign-NS elements are dropped, not rejected");

    let code = out
        .files
        .iter()
        .map(|(_, content)| content.as_str())
        .collect::<String>();

    // The action body must NOT leak. <framework:onentry> is in a
    // foreign namespace and `scxml_children(state, "onentry")`
    // filters it out before its `<log>` is visited. If this marker
    // ever appears in generated code, the namespace filter in
    // `parser.rs::is_scxml_ns` / `scxml_child` / `scxml_children`
    // has regressed.
    assert!(
        !code.contains("collision-marker-must-not-leak"),
        "<log> inside <framework:onentry> leaked into generated code; \
         namespace filter on scxml_child/scxml_children has regressed"
    );
}

/// Root-level namespace gate: a foreign-namespace element with the
/// W3C local name `scxml` (e.g. `<framework:scxml>`) MUST NOT be
/// accepted as a valid SCXML root, even after XSD validation might
/// pass on some permissive path. `parse_impl` checks both local name
/// and `is_scxml_ns(root)` for the root, mirroring
/// `sce/src/parsing/SCXMLParser.cpp::parseInternal`. Without this
/// gate, the parser would walk the foreign tree and silently emit
/// an empty model — exactly the `feedback_silently_broken_hooks`
/// anti-pattern the line-413 comment warns about.
#[test]
fn foreign_ns_w3c_local_name_root_is_rejected() {
    let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<framework:scxml xmlns:framework="http://example.com/framework"
                 xmlns="http://example.com/framework"
                 version="1.0" initial="s1">
  <state id="s1">
    <transition event="go" target="s2"/>
  </state>
  <final id="s2"/>
</framework:scxml>
"##;

    let dir = tempdir().expect("tempdir");
    let path = write_doc(dir.path(), "foreign_ns_root.scxml", scxml);
    let tdir = template_dir();

    // Compile must fail. The error may surface from XSD upstream
    // (root not in SCXML namespace) or from the explicit root NS
    // gate in `parse_impl` — both are valid rejection paths under
    // the strict-namespace policy. What matters is that the parser
    // does NOT silently produce an empty model.
    let result = compile_scxml_lang(path.to_str().unwrap(), &tdir, Language::Rust);
    assert!(
        result.is_err(),
        "<framework:scxml> root must be rejected (XSD or explicit root NS gate), \
         but compile succeeded — the strict-NS root check has regressed"
    );
}
