// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An algorithm builds working lists beside the list it returns
//! (SCE_FORGE.md §4.12, byte-buffer-build v2), on every backend that generates
//! algorithms: each buffer is held to its own declared bound, the output in the
//! caller's destination on Rust and C11 and every working list in the body's
//! own storage, and a loop walks a working list or the output as it walks a
//! list parameter.
//!
//! The document's three buffers declare three bounds (3, 7 and 5), so a
//! backend that gave one buffer another's bound, or held a working list in the
//! destination, writes a text this test does not find.

use std::path::{Path, PathBuf};

use sce_build::forge::codegen_matrix::emitting_in_part;
use sce_build::forge::model::ForgeKind;
use sce_build::generator::Language;
use sce_build::{compile_forge_file, ForgeCompileOptions};

const SCHEMA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="probe_id" sce:event-name="probe.id">
  <datamodel>
    <data id="hi" sce:type="uint64" sce:direction="in"/>
    <data id="lo" sce:type="uint64" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// The ids of `ids` without repeats, at most three, then returned in the order
/// of their `hi` as first seen, at most five: a working list of records
/// (`seen`, bound 3), a working list of numbers (`his`, bound 7) and the
/// output (`out`, bound 5), each walked by a loop that does not append to it.
const DISTINCT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="probe_distinct" version="1.0">
  <sce:import kind="event-schema" src="probe_id.scxml" as="Id"/>
  <sce:signature>
    <sce:param name="ids" type="list&lt;record:Id&gt;"/>
    <sce:return type="list&lt;record:Id&gt;[5]"/>
  </sce:signature>
  <sce:body>
    <sce:var name="seen" type="list&lt;record:Id&gt;[3]"/>
    <sce:var name="his" type="list&lt;uint64&gt;[7]"/>
    <sce:var name="known" type="bool" init="false"/>
    <sce:foreach item="given" in="ids">
      <sce:assign target="known" expr="false"/>
      <sce:foreach item="s" in="seen">
        <sce:if cond="s.hi === given.hi &amp;&amp; s.lo === given.lo">
          <sce:assign target="known" expr="true"/>
        </sce:if>
      </sce:foreach>
      <sce:if cond="!known">
        <sce:append target="seen" expr="given"/>
        <sce:append target="his" expr="given.hi"/>
      </sce:if>
    </sce:foreach>
    <sce:var name="out" type="list&lt;record:Id&gt;[5]"/>
    <sce:foreach item="h" in="his">
      <sce:foreach item="t" in="seen">
        <sce:if cond="t.hi === h">
          <sce:append target="out" expr="t"/>
        </sce:if>
      </sce:foreach>
    </sce:foreach>
    <sce:return expr="out"/>
  </sce:body>
</scxml>
"#;

/// The document and its schema, written into one directory.
fn fixture() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("probe_id.scxml"), SCHEMA).expect("write the schema");
    let document = dir.path().join("probe_distinct.scxml");
    std::fs::write(&document, DISTINCT).expect("write the algorithm");
    (dir, document)
}

/// Every file `language` writes for `document`, as one text.
fn generated(document: &Path, language: Language) -> String {
    let mut options = ForgeCompileOptions::default();
    // Go names every import by its module path.
    if matches!(language, Language::Go) {
        options.go_module_prefix = Some("example.com/probe".to_string());
    }
    let output = compile_forge_file(document, language, &[], &options).unwrap_or_else(|e| {
        panic!(
            "{}: an algorithm with working lists is generated: {e}",
            language.canonical_name()
        )
    });
    output
        .files
        .into_iter()
        .map(|(_, text)| text)
        .collect::<Vec<_>>()
        .join("\n")
}

/// What `language` writes for each buffer: the bound each holds, where it
/// holds one, and the loops that walk the two working lists.
fn expected(language: Language) -> &'static [&'static str] {
    match language {
        // The output is the caller's destination; each working list is a local
        // of its own bound, `mut` since the body appends to it.
        Language::Rust => &[
            "out: &mut SceOwnedList<",
            ", 5>",
            "out.clear();",
            "out.reserve(5);",
            "let mut seen = SceOwnedList::<",
            ", 3>::with_capacity(3);",
            "let mut his = SceOwnedList::<u64, 7>::with_capacity(7);",
            "for &s in seen.iter() {",
            "for &h in his.iter() {",
        ],
        // The output behind the destination's pointer; each working list a
        // struct of the body's frame, every append bounded by its own array.
        Language::C11 => &[
            "(*out).len = 0u;",
            "if ((*out).len < 5u)",
            "items[3];",
            "seen.len = 0u;",
            "if (seen.len < 3u)",
            "items[7];",
            "his.len = 0u;",
            "if (his.len < 7u)",
            "seen.items[__s_i]",
            "his.items[__h_i]",
        ],
        Language::Cpp => &[
            "seen.reserve(3);",
            "his.reserve(7);",
            "out.reserve(5);",
            "uint64_t h : his) {",
        ],
        Language::Go => &[
            ", 0, 3)",
            "his := make([]uint64, 0, 7)",
            ", 0, 5)",
            "for _, h := range his {",
            "for _, s := range seen {",
        ],
        // A list of numbers is a `SceListBuf`, walked through the element's
        // reader of its slots; a list of records an `ArrayList`.
        Language::Kotlin => &[
            "import com.sce.forge.runtime.SceListBuf",
            ">(3)",
            "val his = SceListBuf(7)",
            ">(5)",
            "for (sceAt_h in 0 until his.size) {",
            "val h = his.getULong(sceAt_h)",
            "for (s in seen) {",
        ],
        // A Python list states no bound: it grows, as the other growable
        // backends' do (SCE_FORGE.md §4.12).
        Language::Python => &[
            "seen: list[",
            "his: list[",
            "for h in his:",
            "for s in seen:",
        ],
        Language::Swift => &[
            "seen.reserveCapacity(3)",
            "var his: [UInt64] = []",
            "his.reserveCapacity(7)",
            "out.reserveCapacity(5)",
            "for h in his {",
            "for s in seen {",
        ],
    }
}

#[test]
fn each_buffer_is_held_to_its_own_bound_on_every_algorithm_backend() {
    let (_dir, document) = fixture();
    let languages: Vec<Language> = emitting_in_part(ForgeKind::Algorithm).collect();
    assert!(
        languages.contains(&Language::Swift),
        "Swift generates algorithms in part: {languages:?}"
    );
    let mut missing = Vec::new();
    for language in languages {
        let code = generated(&document, language);
        for text in expected(language) {
            if !code.contains(text) {
                missing.push(format!("{}: `{text}`", language.canonical_name()));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "what each backend writes for the three buffers is not found:\n{}",
        missing.join("\n")
    );
}

/// Rust and C11 take the output alone from the caller: a working list is no
/// parameter of the function.
#[test]
fn a_working_list_is_no_parameter_of_the_function() {
    let (_dir, document) = fixture();
    let rust = generated(&document, Language::Rust);
    assert!(!rust.contains("seen: &mut"), "{rust}");
    assert!(!rust.contains("his: &mut"), "{rust}");
    let c11 = generated(&document, Language::C11);
    assert!(!c11.contains("*seen"), "{c11}");
    assert!(!c11.contains("*his"), "{c11}");
}
