// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The library's entry points that return one machine's source refuse a
//! document carrying an in-line child, naming the child and the entry point
//! that returns every file, rather than return the parent alone.
//!
//! `compile_scxml_to_string` and `compile_from_string` (and their typed forms)
//! hand back a single Rust string. A child written in `<content>` is a machine
//! of its own, written beside the parent's, and the parent's machine names it,
//! so the string they returned for such a document was a machine whose child
//! nobody could build. They now refuse it with
//! `generate/inline-child-unreturnable`, on the `<invoke>` that starts the
//! child, as they refuse a declaration whose output they cannot supply
//! (`generate/source-hash-unavailable`).

use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::generator::Language;

/// A statechart with no child, which both entry points compile.
const LONE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="idle" datamodel="ecmascript">
  <state id="idle"><transition event="go" target="pass"/></state>
  <final id="pass"/>
</scxml>
"#;

/// A statechart that starts, in its one state, a child written in
/// `<content>`, which the generator generates.
const HOST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="running" datamodel="ecmascript">
  <state id="running">
    <invoke type="scxml" id="kid">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="c" datamodel="ecmascript">
          <state id="c"><transition event="go" target="end"/></state>
          <final id="end"/>
        </scxml>
      </content>
    </invoke>
    <transition event="done.invoke.kid" target="pass"/>
  </state>
  <final id="pass"/>
</scxml>
"#;

/// The name the parent's machine starts the child by.
const CHILD: &str = "host__sce_synth_invoke__kid";

/// The 1-based row of the `<invoke>` that starts the child.
fn invoke_row() -> u32 {
    let row = HOST
        .lines()
        .position(|line| line.contains("<invoke"))
        .expect("the host starts a child");
    u32::try_from(row + 1).expect("a row fits in u32")
}

/// The refusal `refused` carries: its code, its file, its row and its
/// message, which names what the record names.
fn read(refused: &sce_build::CompileError) -> (DiagnosticCode, String, Option<u32>, String) {
    let record = refused
        .to_diagnostics()
        .into_iter()
        .next()
        .expect("the refusal reports a record");
    (
        record.code,
        refused.location.file.clone(),
        refused.location.line,
        record.message,
    )
}

#[test]
fn the_in_memory_entry_refuses_a_document_carrying_a_child() {
    let templates = sce_build::template_registry::embedded_templates_for(Language::Rust);
    sce_build::compile_from_string_typed(LONE, "lone", &templates)
        .expect("a document carrying no child compiles to one source");

    let refused = sce_build::compile_from_string_typed(HOST, "host", &templates)
        .expect_err("the entry returns one source and the document carries a second machine");
    let (code, file, line, message) = read(&refused);
    assert!(
        matches!(code, DiagnosticCode::GenerateInlineChildUnreturnable),
        "{code:?}: {message}"
    );
    assert_eq!(file, "host", "the file its author wrote");
    assert_eq!(
        line,
        Some(invoke_row()),
        "the <invoke> that starts the child"
    );
    assert!(
        message.contains(CHILD) && message.contains("compile_from_string_lang"),
        "the refusal names the child and the entry that carries it: {message}"
    );

    // The entry it names carries the child.
    let output =
        sce_build::compile_from_string_lang_typed(HOST, "host", &templates, Language::Rust)
            .expect("the entry that returns every file compiles the document");
    let child_machine = format!("{CHILD}_sm.rs");
    assert!(
        output.files.iter().any(|(name, _)| *name == child_machine),
        "the named entry does not return {child_machine}"
    );
}

#[test]
fn the_file_entry_refuses_a_document_carrying_a_child() {
    let dir = tempfile::tempdir().expect("tempdir");
    let doc = dir.path().join("host.scxml");
    std::fs::write(&doc, HOST).expect("write the document");
    let doc_str = doc.to_str().expect("a UTF-8 path");
    let template_dir = sce_build::find_template_dir_for(Language::Rust);

    let refused = sce_build::compile_scxml_to_string_typed(doc_str, &template_dir)
        .expect_err("the entry returns one source and the document carries a second machine");
    let (code, file, line, message) = read(&refused);
    assert!(
        matches!(code, DiagnosticCode::GenerateInlineChildUnreturnable),
        "{code:?}: {message}"
    );
    assert_eq!(file, doc_str, "the file its author wrote");
    assert_eq!(
        line,
        Some(invoke_row()),
        "the <invoke> that starts the child"
    );
    assert!(
        message.contains(CHILD) && message.contains("compile_scxml_lang"),
        "the refusal names the child and the entry that carries it: {message}"
    );
}
