// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Where a document writes, versus where the same document reads.
//
// `<assign location="arr[0]"/>` and `<log expr="arr[0]"/>` name one cell
// in the language the author is writing. §scxml-B-2 puts that language's
// semantics on both: ECMA-262 11.2.1 defines the index the same way
// whichever side of the assignment it appears on.
//
// The read went through `crate::ecmascript` and came out `arr[1]`,
// because a Lua table's first element is at 1. The write did not go
// through anything: every template spliced the author's text into the
// target source verbatim, so the machine wrote `arr[0]` and read
// `arr[1]` — two different cells, no diagnostic, exit 0. A generated
// Python machine running
//
//     <assign location="arr[0]" expr="99"/>
//     <transition cond="arr[0] == 99" target="pass"/>
//
// reached `fail`.
//
// `crate::ecmascript::to_lua_location` — the entry point written for
// exactly this, documented as "an assignment target ... rejects anything
// else instead of emitting Lua that would assign to a temporary" — had
// no caller at all. The three roles the frontend knew about were
// `expr`, `cond` and `script`, all of them reads.
//
// So this file asks the four write seams the same two questions the read
// seams have always been asked: does a legal target land where the
// frontend says it lands, and does an illegal one reach the author.
// Claims about what a lowering *means* are made by running it on
// `sce-rust-lua`, the engine a generated Rust machine uses.

use std::path::PathBuf;

use sce_build::ecmascript::{to_lua_location, to_lua_value, DocumentScope};
use sce_build::ecmascript_acceptance::{refusals, ExpressionRole};
use sce_build::generator::Language;
use sce_build::model::SCXMLModel;
use sce_build::parser::SCXMLParser;
use sce_rust_lua::LuaEngine;
use sce_rust_runtime::scripting::{IScriptEngine, ScriptValue};

/// The backends that run the datamodel on a Lua interpreter, i.e. the
/// ones whose artifacts carry a lowered write target to compare against.
///
/// C++ and Kotlin hand the authored ECMAScript to a JavaScript engine,
/// where `arr[0]` already denotes what the author wrote; the split is
/// the one `ecmascript_acceptance_parity` pins.
const LOWERING_BACKENDS: &[Language] = &[
    Language::Rust,
    Language::Go,
    Language::Python,
    Language::C11,
];

// ── The rule ─────────────────────────────────────────────────────

/// A write and a read of the same authored text reach the same cell.
///
/// Run rather than compared: the pair is executed on the engine a
/// generated Rust machine uses, so "the same cell" is a measurement and
/// not an argument about two strings.
#[test]
fn a_write_and_a_read_of_the_same_text_reach_the_same_cell() {
    let scope = DocumentScope::declaring(["arr", "obj"]);
    let engine = LuaEngine::new();
    assert!(engine.initialize(), "the Lua engine must start");
    let session = "write_targets".to_string();
    engine.create_session(&session);
    engine
        .execute_script(&session, "arr = {10, 20, 30}; obj = {k = 1}")
        .expect("the fixture datamodel establishes");

    // Every form `emit_location` admits, in both spellings ECMA-262
    // 11.2.1 gives the index one.
    for (n, source) in ["arr[0]", "arr[2]", "obj.k", "obj['k']"].iter().enumerate() {
        let target = to_lua_location(source).unwrap_or_else(|err| {
            panic!("{source} is a legal write target and was refused: {err}")
        });
        let written = 700 + n as i64;
        engine
            .execute_script(&session, &format!("{target} = {written}"))
            .unwrap_or_else(|err| panic!("{source} lowered to {target}, which did not run: {err}"));

        let read = to_lua_value(source, &scope)
            .unwrap_or_else(|err| panic!("{source} is a legal read and was refused: {err}"));
        let seen = engine
            .evaluate_expression(&session, &read)
            .unwrap_or_else(|err| panic!("{source} lowered to {read}, which did not run: {err}"));
        assert_eq!(
            seen,
            ScriptValue::Int(written),
            "the document wrote {written} at {source} (lowered {target}) and read \
             {seen:?} back from the same text (lowered {read}) — the two seams \
             disagree about which cell the author named"
        );
    }
}

/// A target that is not an assignment target is refused, rather than
/// spliced into the target language and discovered by whoever ran the
/// machine.
#[test]
fn a_target_that_cannot_be_written_is_refused() {
    for source in ["holder'x", "1 + 1", "'continue'", "f()", "arr[0] + 1"] {
        assert!(
            to_lua_location(source).is_err(),
            "{source} is not an assignment target and was accepted"
        );
    }
}

// ── The seams ────────────────────────────────────────────────────

/// Every backend that lowers writes where the frontend says.
///
/// The probe is `arr[0]`, whose lowering is textually distinct from what
/// the author wrote, and the document carries no *read* of it — so the
/// lowered spelling can only be in the artifact because the write seam
/// put it there. A template that splices the author's text emits
/// `arr[0]` and nothing else, which is what every one of them did.
#[test]
fn every_lowering_backend_writes_where_the_frontend_says() {
    const DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="ecmascript" initial="s0" name="written">
  <datamodel>
    <data id="arr" expr="[10,20,30]"/>
  </datamodel>
  <state id="s0">
    <onentry>
      <assign location="arr[0]" expr="99"/>
    </onentry>
    <transition target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;
    let model = parse("written", DOCUMENT);
    let target = to_lua_location("arr[0]").expect("arr[0] is a legal write target");

    for lang in LOWERING_BACKENDS {
        let rendered = render(&model, *lang, "written");
        assert!(
            rendered.contains(&target),
            "{lang:?} carries no write to {target} — the artifact assigns to the \
             text the author wrote instead of to what §scxml-B-2 says it names, \
             so the machine writes a cell its own reads never see"
        );
    }
}

/// The same question for the other unvalidated write seam: the id a
/// `<send>` stores.
#[test]
fn every_lowering_backend_stores_a_send_id_where_the_frontend_says() {
    const DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="ecmascript" initial="s0" name="stored">
  <datamodel>
    <data id="slots" expr="[0,0]"/>
  </datamodel>
  <state id="s0">
    <onentry>
      <send event="e" idlocation="slots[0]"/>
    </onentry>
    <transition target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;
    let model = parse("stored", DOCUMENT);
    let target = to_lua_location("slots[0]").expect("slots[0] is a legal write target");

    for lang in LOWERING_BACKENDS {
        let rendered = render(&model, *lang, "stored");
        assert!(
            rendered.contains(&target),
            "{lang:?} stores the send id at the text the author wrote rather than \
             at {target}, so `<cancel sendidexpr>` reading the same location by \
             name finds nothing there"
        );
    }
}

// ── The author ───────────────────────────────────────────────────

/// A write target the frontend refuses is reported at the element that
/// wrote it, on every seam that carries one.
///
/// `<param location>` is absent on purpose: it is a *read* — the value
/// at that location becomes the payload — and the walker has always
/// checked it through the value seam.
#[test]
fn every_write_seam_reports_a_target_it_cannot_lower() {
    const DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="ecmascript" initial="s0" name="refused">
  <datamodel>
    <data id="holder" expr="0"/>
    <data id="list" expr="[1,2]"/>
  </datamodel>
  <state id="s0">
    <onentry>
      <assign location="holder'x" expr="1"/>
      <foreach array="list" item="'continue'" index="idx'z"/>
      <send event="e" idlocation="holder'w"/>
    </onentry>
    <transition target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;
    let model = parse("refused", DOCUMENT);
    let reported = refusals(&model);
    let sites: Vec<&str> = reported
        .iter()
        .filter(|r| r.role == ExpressionRole::Location)
        .map(|r| r.site.as_str())
        .collect();

    for seam in [
        "<assign location>",
        "<foreach item>",
        "<foreach index>",
        "<send idlocation>",
    ] {
        assert!(
            sites.contains(&seam),
            "{seam} wrote a target the frontend cannot lower and nobody was told; \
             reported: {sites:?}"
        );
    }
}

/// The refusal the artifact carries raises, on the engine that runs it,
/// with the message the author needs.
///
/// A refused target cannot be lowered to an assignable name, and a
/// generated assignment still has to be a *statement* the engine
/// accepts: `error("…") = 1` does not parse, so the message would be
/// replaced by a syntax error at the one moment it is needed. Indexing
/// the refusal keeps the statement grammatical (Lua 5.4 §3.5 admits
/// `'(' exp ')' '[' exp ']'` as a var) and evaluates the raise before
/// anything is written.
#[test]
fn a_refused_target_raises_the_authors_message_on_the_engine() {
    let engine = LuaEngine::new();
    assert!(engine.initialize(), "the Lua engine must start");
    let session = "refused_target".to_string();
    engine.create_session(&session);

    let emitted = sce_build::filters::to_lua_location("holder'x".to_string())
        .expect("the filter answers rather than failing the render");
    let err = engine
        .execute_script(&session, &format!("{emitted} = 1"))
        .expect_err("a refused write target must not assign anything");
    let text = err.to_string();
    assert!(
        text.contains("is not valid ECMAScript") && text.contains("holder'x"),
        "the engine raised {text:?}, which does not name the target the author \
         wrote — the codegen-time verdict did not survive to the runtime the \
         clause describes"
    );
}

// ── Machinery ────────────────────────────────────────────────────

fn parse(stem: &str, document: &str) -> SCXMLModel {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("write-targets");
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    let path = dir.join(format!("{stem}.scxml"));
    std::fs::write(&path, document).expect("write document");
    let mut parser = SCXMLParser::new();
    let mut model = parser
        .parse_file(path.to_str().expect("utf-8 path"))
        .expect("the document parses");
    sce_build::analyzer::analyze(&mut model, path.to_str().expect("utf-8 path"));
    model
}

fn render(model: &SCXMLModel, lang: Language, stem: &str) -> String {
    let dir = sce_build::find_template_dir_for(lang);
    let rendered = match lang {
        Language::Rust => sce_build::generator::generate(model, &dir, false).ok(),
        Language::Go => sce_build::generator::generate_go(model, &dir).ok(),
        Language::Python => sce_build::generator::generate_python(model, &dir).ok(),
        Language::C11 => sce_build::generator::generate_c11(model, &dir, stem, None)
            .ok()
            .map(|out| {
                out.files
                    .iter()
                    .map(|(_, body)| body.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            }),
        other => panic!("{other:?} does not lower authored ECMAScript here"),
    };
    rendered.unwrap_or_else(|| panic!("{lang:?} renders the document"))
}
