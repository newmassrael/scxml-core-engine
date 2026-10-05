// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `datamodel="sce-static"` — SCE's platform-defined, statically typed data
// model (docs/SCE_ACCEPTED_SUBSET.md §2.15).
//
// A variable under it is a native field of the generated machine, so every
// rule here is one a field needs: a declared type, an initial value that
// is a typed expression, an id generated code can spell. Each refusal is
// asserted on its wire code AND on the row it is placed on, because a
// refusal on the wrong line sends the author to the wrong element.
//
// The last cases pin the other half of the model's first step: until a
// backend's templates lower it, generation refuses it for that backend
// rather than handing forge-language expressions to a script engine.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::tempdir;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// Run `sce-codegen <args…> <doc>` with JSON diagnostics and return
/// `(exit_ok, stdout + stderr)`. `check` with no `--language` fails only
/// on the document axis; `check -l X` fails on X's backend axis too.
fn run(args: &[&str], doc: &str) -> (bool, String) {
    run_beside(args, doc, &[])
}

/// [`run`], with `siblings` — `(file name, text)` — written beside the
/// document, where its `<sce:import src>`s resolve.
fn run_beside(args: &[&str], doc: &str, siblings: &[(&str, &str)]) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    for (name, text) in siblings {
        std::fs::write(dir.path().join(name), text).expect("write sibling");
    }
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, doc).expect("write probe");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(&path)
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// The text of every C source `generate -l c` wrote into `dir`, joined.
fn generated_c(dir: &std::path::Path) -> String {
    let mut text = String::new();
    for entry in std::fs::read_dir(dir).expect("the output directory") {
        let path = entry.expect("an entry").path();
        if path.extension().is_some_and(|e| e == "c") {
            text.push_str(&std::fs::read_to_string(&path).expect("a generated source"));
        }
    }
    text
}

/// A statechart with the given `datamodel` value and `<datamodel>` body.
/// The `<datamodel>` opens on line 4, so its first child is on line 5.
fn doc(datamodel: &str, data: &str) -> String {
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="{datamodel}">
  <datamodel>
    {data}
  </datamodel>
  <state id="s"><transition event="go" target="done"/></state>
  <final id="done"/>
</scxml>
"##
    )
}

/// Assert a refusal carries `code` and is placed on `line`.
fn assert_refused_at(out: &str, code: &str, line: u32) {
    let record = out
        .lines()
        .find(|l| l.contains(&format!("\"code\":\"{code}\"")))
        .unwrap_or_else(|| panic!("expected {code}, got:\n{out}"));
    // A refusal placed on a row alone (an import) carries no column.
    assert!(
        record.contains(&format!("\"line\":{line},"))
            || record.contains(&format!("\"line\":{line}}}")),
        "{code} must be placed on line {line}:\n{record}"
    );
}

#[test]
fn a_typed_variable_is_accepted_under_the_static_data_model() {
    let (ok, out) = run(
        &["check"],
        &doc(
            "sce-static",
            r#"<data id="count" sce:type="uint32" expr="0"/>"#,
        ),
    );
    assert!(ok, "a typed <data> is what sce-static asks for:\n{out}");
}

#[test]
fn an_untyped_variable_is_refused_on_its_own_line() {
    let (ok, out) = run(
        &["check"],
        &doc("sce-static", r#"<data id="count" expr="0"/>"#),
    );
    assert!(!ok, "a <data> with no sce:type must be refused:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
}

#[test]
fn a_variable_read_from_src_is_refused() {
    let (ok, out) = run(
        &["check"],
        &doc(
            "sce-static",
            r#"<data id="count" sce:type="uint32" src="count.json"/>"#,
        ),
    );
    assert!(!ok, "src has no type to give a field:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
    assert!(
        out.contains("count.json"),
        "`actual` is the src as written:\n{out}"
    );
}

#[test]
fn a_variable_with_in_line_content_is_refused() {
    let (ok, out) = run(
        &["check"],
        &doc(
            "sce-static",
            r#"<data id="count" sce:type="uint32">7</data>"#,
        ),
    );
    assert!(!ok, "in-line content has no type to give a field:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
}

#[test]
fn script_text_is_refused_under_the_static_data_model() {
    let (ok, out) = run(
        &["check"],
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s"
       datamodel="sce-static">
  <state id="s">
    <onentry><script>x = 1</script></onentry>
    <transition event="go" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"##,
    );
    assert!(!ok, "script text needs a scripting language:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
}

#[test]
fn a_declared_type_under_ecmascript_is_the_authoring_io_declaration() {
    // Under `ecmascript`, `sce:type` with `sce:direction` is the typed
    // input/output declaration the authoring tool drives a statechart
    // through. The first cut of this model refused it as unread, and the
    // authoring suite's run-driven cases went red in CI: the attribute was
    // read, only not by the generator.
    let (ok, out) = run(
        &["check"],
        &doc(
            "ecmascript",
            r#"<data id="count" sce:type="int32" sce:direction="out" expr="0"/>"#,
        ),
    );
    assert!(ok, "the authoring I/O declaration is admitted:\n{out}");
}

#[test]
fn a_misspelled_type_is_refused_on_its_line() {
    // Two layers refuse this, and which one answers depends on whether the
    // build loads the XSD: the schema types `sce:type` as
    // `sceTypeOrEnumRef` (`xml/schema-validation`), and without it the
    // parser's `read_type_attr` refuses the same value
    // (`validation/invalid-attribute`). The case pins the outcome both
    // share — refused, on the attribute's line, naming what was written.
    let (ok, out) = run(
        &["check"],
        &doc(
            "sce-static",
            r#"<data id="count" sce:type="uint33" expr="0"/>"#,
        ),
    );
    assert!(!ok, "uint33 is no type:\n{out}");
    let record = out
        .lines()
        .find(|l| {
            l.contains("\"code\":\"xml/schema-validation\"")
                || l.contains("\"code\":\"validation/invalid-attribute\"")
        })
        .unwrap_or_else(|| panic!("expected a type refusal, got:\n{out}"));
    assert!(
        record.contains("\"line\":5") && record.contains("uint33"),
        "the refusal names `uint33` on line 5:\n{record}"
    );
}

#[test]
fn a_variable_id_is_held_to_the_code_identifier_grammar() {
    // `raw-value` is a valid xs:ID and so a valid statechart <data id>;
    // under sce-static it names a field, and `-` is an operator in every
    // target language.
    let (ok, out) = run(
        &["check"],
        &doc(
            "sce-static",
            r#"<data id="raw-value" sce:type="uint32" expr="0"/>"#,
        ),
    );
    assert!(!ok, "a field cannot be spelled `raw-value`:\n{out}");
    assert!(
        out.contains("validation/malformed-code-identifier"),
        "expected the code-identifier refusal, got:\n{out}"
    );
}

#[test]
fn the_same_id_is_admitted_under_ecmascript() {
    // The grammar is widened only where the id reaches generated code.
    let (ok, out) = run(
        &["check"],
        &doc("ecmascript", r#"<data id="raw-value" expr="0"/>"#),
    );
    assert!(ok, "an ecmascript <data id> stays an xs:ID:\n{out}");
}

#[test]
fn cpp_names_each_construct_it_does_not_lower_yet() {
    // C++ lowers scalar, enum, record and list variables, guards, `<assign>`,
    // `<if>`, `<foreach>`, `<log>`, `In()`, host actions, an event's typed
    // payload, a call of an imported algorithm, a final's `<donedata>` and an
    // `<invoke type="scxml">`. What is past that is refused by name where the
    // document is read, not left as an undefined name in the generated code.
    let fixtures = repo_root().join("sce-build/tests/fixtures/static_datamodel");
    let bytes_variable = doc(
        "sce-static",
        r#"<data id="frame" sce:type="bytes" expr="''"/>"#,
    );
    let host_invoke = machine(
        r#"<state id="s">
    <invoke type="x-sce-host" id="h"><param name="k" expr="count"/></invoke>
    <transition event="done.invoke.h" target="done"/>
  </state>"#,
    );
    let siblings: Vec<(String, String)> = std::fs::read_dir(&fixtures)
        .expect("the fixture directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "scxml"))
        .map(|path| {
            (
                path.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&path).expect("a fixture"),
            )
        })
        .collect();
    let siblings: Vec<(&str, &str)> = siblings
        .iter()
        .map(|(name, text)| (name.as_str(), text.as_str()))
        .collect();
    let cases = [
        ("a bytes variable", bytes_variable, "of a bytes type"),
        ("a host-run <invoke>", host_invoke, "a host-run <invoke>"),
    ];
    // C++, Go and Python start a scxml child and refuse the rest by name.
    for (lang, name) in [("cpp", "C++"), ("go", "Go"), ("python", "Python")] {
        for (what, document, names) in &cases {
            let (ok, out) = run_beside(
                &["check", "-l", lang, "--go-module-prefix", "x/y"],
                document,
                &siblings,
            );
            assert!(
                !ok,
                "{lang}, {what}: {name} has no lowering for it yet:\n{out}"
            );
            assert!(
                out.contains("generate/unsupported-feature")
                    && out.contains(&format!("no {name} lowering yet")),
                "{lang}, {what}: expected the unsupported-feature refusal naming {name}:\n{out}"
            );
            assert!(
                out.contains(names),
                "{lang}, {what}: it names the construct:\n{out}"
            );
        }
    }
}

#[test]
fn c11_names_each_construct_it_does_not_lower_yet() {
    // C lowers integer, bool, enum, bounded string and 64-bit real variables,
    // records of numbers, bools and enums, lists of integers, bools and such records with
    // `<sce:append>`, `<sce:clear>` and `<foreach>`, guards, `<assign>`,
    // `<if>`, `<log>`, `<raise>`, `In()`, `<cancel>`, an event's typed payload of
    // numbers, bools, strings and enums, a call of an imported algorithm, a host action
    // whose arguments are typed expressions of them, the `<param>`s of a final's
    // `<donedata>` (and its inline `<content>`) and of a `<send>` to the machine's
    // own processor or to one the host is declared to serve (and the literal
    // `<content>` of one), an `<invoke type="scxml">` handing numbers,
    // bools and strings, and an `<invoke>` the host is declared to serve. A 32-bit
    // real, alone or in a list, is a `float` the wire writes as the double it
    // widens to. What is
    // past that — a record with a string field or a 32-bit real field, a bytes
    // variable, a `<send>` to a processor no host is
    // declared to serve, an `<invoke>` of a type none is, a `<param>` name that
    // repeats, a payload field that
    // is bytes — is refused by
    // name where the document is read, not left as an undefined name in the
    // generated code.
    let fixtures = repo_root().join("sce-build/tests/fixtures/static_datamodel");
    let variable = |data: &str| doc("sce-static", data);
    let cases = [
        (
            "a bytes variable",
            variable(r#"<data id="frame" sce:type="bytes" expr="''"/>"#),
            r#"<data id="frame" sce:type="bytes">"#,
        ),
        (
            "a host-run <invoke>",
            machine(
                r#"<state id="s">
    <invoke type="x-sce-host" id="h"><param name="k" expr="count"/></invoke>
    <transition event="done.invoke.h" target="done"/>
  </state>"#,
            ),
            "a host-run <invoke>",
        ),
        (
            "a <send> to a processor no host is declared to serve",
            machine(
                r#"<state id="s"><onentry><send type="x-sce-host" event="x"><param name="k" expr="count"/></send></onentry></state>"#,
            ),
            "a <send> of type `x-sce-host`",
        ),
        (
            "a typed payload with a bytes field",
            r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <sce:import kind="event-schema" src="schema_blob.scxml" as="Blob"/>
  <datamodel><data id="count" sce:type="uint32" expr="0"/></datamodel>
  <state id="s"><transition event="blob.sent" cond="_event.data.size &gt; 1" target="done"/></state>
  <final id="done"/>
</scxml>
"##
            .to_string(),
            "an event whose payload carries `frame` of type bytes",
        ),
    ];
    // A schema with a bytes field, which no fixture of the tree declares.
    let blob = (
        "schema_blob.scxml".to_string(),
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="schema_blob" sce:event-name="blob.sent">
  <datamodel>
    <data id="frame" sce:type="bytes" sce:direction="in"/>
    <data id="size" sce:type="uint8" sce:direction="in"/>
  </datamodel>
</scxml>
"##
        .to_string(),
    );
    let siblings: Vec<(String, String)> = std::fs::read_dir(&fixtures)
        .expect("the fixture directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "scxml"))
        .map(|path| {
            (
                path.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&path).expect("a fixture"),
            )
        })
        .chain(std::iter::once(blob))
        .collect();
    let siblings: Vec<(&str, &str)> = siblings
        .iter()
        .map(|(name, text)| (name.as_str(), text.as_str()))
        .collect();
    for (what, document, names) in &cases {
        let (ok, out) = run_beside(&["check", "-l", "c11"], document, &siblings);
        assert!(!ok, "{what}: C has no lowering for it yet:\n{out}");
        assert!(
            out.contains("generate/unsupported-feature") && out.contains("no C11 lowering yet"),
            "{what}: expected the unsupported-feature refusal naming C11:\n{out}"
        );
        // The refusal quotes the construct inside a JSON string.
        let quoted = names.replace('"', "\\\"");
        assert!(
            out.contains(&quoted) || out.contains(names),
            "{what}: it names the construct `{names}`:\n{out}"
        );
    }
}

/// A record whose schema has a `string` field is held by the languages that have
/// lowered it (docs/adr/0005, decision 1) and refused by name by the rest, which
/// lift the refusal one at a time: the third column is the one value a language
/// changes when it does. Go is checked with a module prefix, which it needs to
/// write its imports.
const RECORD_STRING_FIELD: &[(&str, &str, bool)] = &[
    ("rust", "Rust", true),
    ("kotlin", "Kotlin", true),
    ("go", "Go", true),
    ("cpp", "C++", true),
    ("python", "Python", true),
    ("c11", "C11", false),
];

/// `check -l <lang>` of `document`, with what that language needs to read it.
fn check_in(lang: &str, document: &str, siblings: &[(&str, &str)]) -> (bool, String) {
    match lang {
        "go" => run_beside(
            &["check", "-l", "go", "--go-module-prefix", "x/y"],
            document,
            siblings,
        ),
        _ => run_beside(&["check", "-l", lang], document, siblings),
    }
}

/// A document that holds `data` (a `<data>` element of a record or a list of
/// records of `schema_label.scxml`) and does nothing else.
fn holding_label(data: &str) -> String {
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static">
  <sce:import kind="event-schema" src="schema_label.scxml" as="Label"/>
  <datamodel>
    {data}
  </datamodel>
  <state id="idle"/>
</scxml>
"##
    )
}

/// The schema of `holding_label`: a small integer and a string, whose bound is
/// `max_size` (`sce:max-size="8"`), or none when that is empty.
fn label_schema(max_size: &str) -> String {
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="schema_label" sce:event-name="label.taken">
  <datamodel>
    <data id="sensor" sce:type="uint8" sce:direction="in"/>
    <data id="label" sce:type="string" {max_size} sce:direction="in"/>
  </datamodel>
</scxml>
"##
    )
}

/// The two ways a record is declared: a variable built whole, and a list.
fn labelled_documents() -> [(&'static str, String); 2] {
    [
        (
            "a record variable",
            holding_label(
                r#"<data id="last" sce:type="record:Label" sce:direction="out">
      <sce:set name="sensor" expr="1"/>
      <sce:set name="label" expr="'a'"/>
    </data>"#,
            ),
        ),
        (
            "a list of records",
            holding_label(
                r#"<data id="labels" sce:type="list&lt;record:Label&gt;" sce:capacity="4" sce:direction="out"/>"#,
            ),
        ),
    ]
}

#[test]
fn a_language_holds_a_record_string_field_or_refuses_it_by_name() {
    // Measured 2026-10-06, before any language held the field: `check` answered
    // ok for Rust, Kotlin, Go, C++ and Python, and Rust wrote
    // `#[derive(Clone, Copy)]` over a `String` field, which does not compile
    // (E0204); only C11 refused. A refusal is asked once, in `lower`, and a
    // language that has not lowered the field is refused by name.
    let schema = label_schema(r#"sce:max-size="8""#);
    let siblings = [("schema_label.scxml", schema.as_str())];
    for (held, document) in &labelled_documents() {
        for (lang, name, holds) in RECORD_STRING_FIELD {
            let (ok, out) = check_in(lang, document, &siblings);
            if *holds {
                assert!(ok, "{lang}, {held}: it lowers the field:\n{out}");
                continue;
            }
            assert!(!ok, "{lang}, {held}: no lowering for it yet:\n{out}");
            assert!(
                out.contains("generate/unsupported-feature")
                    && out.contains(&format!("no {name} lowering yet"))
                    && out.contains("record:Label with the field `label` of type string"),
                "{lang}, {held}: expected the refusal naming {name} and the field:\n{out}"
            );
        }
    }
}

#[test]
fn a_record_string_field_the_schema_does_not_bound_is_refused_by_every_language() {
    // No default stands in for a bound the author did not write: the machine
    // would fail a value at run time for a limit nobody declared. The refusal is
    // the data model's, so it is the same wherever the record would be held.
    let schema = label_schema("");
    let siblings = [("schema_label.scxml", schema.as_str())];
    for (held, document) in &labelled_documents() {
        for (lang, _, _) in RECORD_STRING_FIELD {
            let (ok, out) = check_in(lang, document, &siblings);
            assert!(!ok, "{lang}, {held}: the field has no bound:\n{out}");
            assert!(
                out.contains("scxml/static-datamodel-rule")
                    && out.contains("the field `label` of record:Label is a string")
                    && out.contains("sce:max-size"),
                "{lang}, {held}: expected the data model's refusal naming the field:\n{out}"
            );
        }
    }
}

#[test]
fn a_record_string_field_starts_at_a_literal_that_fits_its_bound() {
    // The machine is built with no error to raise, as for a string variable:
    // the field starts at a literal of at most the bound's UTF-8 bytes.
    let schema = label_schema(r#"sce:max-size="8""#);
    let siblings = [("schema_label.scxml", schema.as_str())];
    for (what, label, expected) in [
        ("nine bytes", "'123456789'", "past the sce:max-size of 8"),
        // Counted in bytes: four characters, nine bytes (2 + 3 + 2 + 2).
        (
            "nine bytes in four characters",
            "'é€éé'",
            "past the sce:max-size of 8",
        ),
        (
            "a value computed",
            "'a' + 'b'",
            "starts at a string literal",
        ),
    ] {
        let document = holding_label(&format!(
            r#"<data id="last" sce:type="record:Label" sce:direction="out">
      <sce:set name="sensor" expr="1"/>
      <sce:set name="label" expr="{label}"/>
    </data>"#
        ));
        for (lang, _, _) in RECORD_STRING_FIELD {
            let (ok, out) = check_in(lang, &document, &siblings);
            assert!(
                !ok,
                "{lang}, {what}: the field starts past its bound:\n{out}"
            );
            assert!(
                out.contains("scxml/static-datamodel-rule") && out.contains(expected),
                "{lang}, {what}: expected `{expected}`:\n{out}"
            );
        }
    }
    // Eight bytes in three characters (2 + 3 + 3) fit: the bound counts bytes.
    let fits = holding_label(
        r#"<data id="last" sce:type="record:Label" sce:direction="out">
      <sce:set name="sensor" expr="1"/>
      <sce:set name="label" expr="'é€€'"/>
    </data>"#,
    );
    for (lang, _, holds) in RECORD_STRING_FIELD {
        let (ok, out) = check_in(lang, &fits, &siblings);
        // A language that holds the field accepts it; one that does not refuses
        // the field, never for its length.
        assert!(
            ok == *holds && (ok || !out.contains("past the sce:max-size")),
            "{lang}: eight bytes fit a bound of eight:\n{out}"
        );
    }
}

#[test]
fn a_payload_enum_field_the_document_does_not_import_is_refused_by_name_in_every_language() {
    // Measured 2026-10-04: Rust, Go, Kotlin, Python and C++ stopped with a panic
    // (exit 101, "SceType::Enum(alias='ViewMode') reached a context built by
    // LangCtx::primitive") on a transition that reads `_event.data.zoom` of an
    // event whose schema also declares `layout`, an enum. The typed channel
    // holds every field its schema declares, in the machine's own type for the
    // enum, so the document imports the enum under the alias the schema writes,
    // as it does for a record's field; one that does not is refused naming the
    // alias where it is lowered, and not left to stop the generator.
    //
    // C11 held the same panic's place with its own refusal of every transition
    // on such an event, until it held the field in the machine's enum too.
    //
    // A transition on the event that reads nothing from it still generates, so
    // the refusal is of the READ.
    let fixtures = repo_root().join("sce-build/tests/fixtures/static_datamodel");
    let siblings: Vec<(String, String)> = ["schema_view.scxml", "enum_view_mode.scxml"]
        .iter()
        .map(|name| {
            (
                (*name).to_string(),
                std::fs::read_to_string(fixtures.join(name)).expect("a fixture"),
            )
        })
        .collect();
    let siblings: Vec<(&str, &str)> = siblings
        .iter()
        .map(|(name, text)| (name.as_str(), text.as_str()))
        .collect();
    let document = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <sce:import kind="event-schema" src="schema_view.scxml" as="View"/>
  <datamodel><data id="zoom" sce:type="uint8" expr="0" sce:direction="out"/></datamodel>
  <state id="s">
    <transition event="view.shown" type="internal">
      <assign location="zoom" expr="_event.data.zoom"/>
    </transition>
  </state>
</scxml>
"##;
    for language in ["kotlin", "rust", "cpp", "go", "python", "c"] {
        let out_dir = tempdir().expect("tempdir");
        let (ok, out) = run_beside(
            &[
                "generate",
                "-l",
                language,
                "--go-module-prefix",
                "x/y",
                "-o",
                out_dir.path().to_str().expect("a path"),
            ],
            document,
            &siblings,
        );
        assert!(!ok, "{language}: the machine is refused:\n{out}");
        assert!(
            !out.contains("panicked"),
            "{language}: a refusal, not a panic:\n{out}"
        );
        assert!(
            out.contains("generate/unsupported-feature"),
            "{language}: an unsupported feature:\n{out}"
        );
        assert!(
            out.contains("`layout`")
                && out.contains("the enum `ViewMode`")
                && out.contains("<sce:import kind=\\\"enum\\\" as=\\\"ViewMode\\\">"),
            "{language}: it names the field and the import the document lacks:\n{out}"
        );
    }
}

#[test]
fn a_payload_enum_field_is_held_in_the_machines_own_enum_on_every_backend_that_lowers_it() {
    // Every backend that carries the typed channel holds the field in the type
    // the machine declares for the enum (here named by a payload alone: no
    // variable of the machine holds it), lift it from the variant's declared
    // name and refuse a name the enum does not declare; the scenario
    // `static_payload_enum` runs the machines. What is read is the generated
    // text, since a machine that did not hold the enum would stop on it.
    let fixtures = repo_root().join("sce-build/tests/fixtures/static_datamodel");
    let siblings: Vec<(String, String)> = ["schema_view.scxml", "enum_view_mode.scxml"]
        .iter()
        .map(|name| {
            (
                (*name).to_string(),
                std::fs::read_to_string(fixtures.join(name)).expect("a fixture"),
            )
        })
        .collect();
    let siblings: Vec<(&str, &str)> = siblings
        .iter()
        .map(|(name, text)| (name.as_str(), text.as_str()))
        .collect();
    let document = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <sce:import kind="event-schema" src="schema_view.scxml" as="View"/>
  <sce:import kind="enum" src="enum_view_mode.scxml" as="ViewMode"/>
  <datamodel><data id="zoom" sce:type="uint8" expr="0" sce:direction="out"/></datamodel>
  <state id="s">
    <transition event="view.shown" type="internal">
      <assign location="zoom" expr="_event.data.zoom"/>
    </transition>
  </state>
</scxml>
"##;
    for (language, extension, held) in [
        ("kotlin", "kt", "declaredName == name"),
        ("rust", "rs", "is not a variant of ViewMode"),
        // The policy's members, the lift among them, are in the header.
        ("cpp", "h", "is not a variant of ViewMode"),
        ("go", "go", "is not a variant of ViewMode"),
        ("python", "py", "ViewModeEnum), (\"zoom\", int, 0, 255)"),
        // The lift is in the machine's own file, beside the header that declares
        // the enum before the channel names it.
        ("c", "c", "is not a variant of ViewMode"),
    ] {
        let out_dir = tempdir().expect("tempdir");
        let (ok, out) = run_beside(
            &[
                "generate",
                "-l",
                language,
                "--go-module-prefix",
                "x/y",
                "-o",
                out_dir.path().to_str().expect("a path"),
            ],
            document,
            &siblings,
        );
        assert!(ok, "{language}: the machine generates:\n{out}");
        assert!(
            out.contains("\"needs_script_engine\":false"),
            "{language}: an enum payload field needs no engine:\n{out}"
        );
        let generated: Vec<_> = std::fs::read_dir(out_dir.path())
            .expect("the output directory")
            .map(|entry| entry.expect("an entry").path())
            .filter(|path| path.extension().is_some_and(|e| e == extension))
            .collect();
        assert_eq!(generated.len(), 1, "{language}: one machine: {generated:?}");
        let source = std::fs::read_to_string(&generated[0]).expect("a generated machine");
        assert!(
            source.contains(held),
            "{language}: the payload field is lifted into the machine's enum"
        );
    }
}

#[test]
fn a_host_run_invoke_is_lowered_where_its_params_are_read_from_the_fields() {
    // An `<invoke>` the host serves (`--host-invoker`) carries `<param>`s that
    // are typed expressions of the machine's own fields, which every backend reads
    // there when the invocation starts. An invoke of a type the build was not told
    // the host serves is another matter: nothing starts it, and each backend
    // refuses it by name (`cpp_names_each_construct_it_does_not_lower_yet`,
    // `c11_names_each_construct_it_does_not_lower_yet`).
    let document = std::fs::read_to_string(
        repo_root()
            .join("sce-build/tests/fixtures/host_processor/statechart_static_host_invoke.scxml"),
    )
    .expect("the host invoke fixture");
    for lang in ["rust", "kotlin", "go", "cpp", "python", "c11"] {
        let (ok, out) = run(
            &[
                "check",
                "-l",
                lang,
                "--go-module-prefix",
                "x/y",
                "--host-invoker",
                "x-sce-host",
            ],
            &document,
        );
        assert!(ok, "{lang}: starts an invoke the host serves:\n{out}");
    }
}

#[test]
fn every_backend_lowers_the_model_with_no_script_engine() {
    // Each holds the variables as fields and lowers every expression
    // natively, so the machine it generates carries no engine — the manifest
    // says so, and each integration suite (StaticDatamodelTest.kt,
    // backends/rust/tests/tests/static_datamodel.rs,
    // tests/integration/AStaticDatamodelRunsGeneratedCppTest.cpp,
    // backends/go/tests/integration/static_datamodel,
    // backends/python/tests/integration/static_datamodel,
    // backends/c/tests/integration/test_static_scalars.c) drives the machines.
    //
    // A backend that did not lower it would have to refuse instead — a
    // document evaluated by a script engine in a language it never declared is
    // what `datamodel` exists to prevent — and one added to the language list
    // with neither fails here, not for a user.
    for lang in ["kotlin", "rust", "cpp", "go", "python", "c11"] {
        let (ok, out) = run(
            &["check", "-l", lang],
            &machine(
                r#"<state id="s">
    <transition event="tick" cond="count &lt; 10 &amp;&amp; In('s')" type="internal">
      <assign location="count" expr="count + 1"/>
    </transition>
  </state>"#,
            ),
        );
        assert!(ok, "--lang {lang} lowers sce-static:\n{out}");
        assert!(
            out.contains("\"needs_script_engine\":false"),
            "--lang {lang}: a sce-static machine needs no script engine:\n{out}"
        );
    }
}

#[test]
fn a_32_bit_real_is_held_by_every_backend_alone_in_a_list_and_as_a_record_field() {
    // `static_real32` holds one alone and in a list, `static_record_real32` as a
    // field of a record its payload carries. Every backend lowers both: a
    // backend that refused one would leave the single the one real width some
    // engine cannot hold.
    let fixtures = repo_root().join("sce-build/tests/fixtures/static_datamodel");
    let read = |name: &str| std::fs::read_to_string(fixtures.join(name)).expect("a fixture");
    let alone = read("static_real32.scxml");
    let record = read("static_record_real32.scxml");
    let schema = read("schema_reading32.scxml");
    for lang in ["kotlin", "rust", "cpp", "go", "python", "c11"] {
        let (ok, out) = run(&["check", "-l", lang], &alone);
        assert!(
            ok,
            "--lang {lang} lowers a float32 variable and a list of them:\n{out}"
        );
        let (ok, out) = run_beside(
            &["check", "-l", lang],
            &record,
            &[("schema_reading32.scxml", &schema)],
        );
        assert!(
            ok,
            "--lang {lang} lowers a record with a float32 field:\n{out}"
        );
    }
}

// ── Every expression judged against the typed scope ─────────────────────

/// A `sce-static` machine with two variables — `count: uint32` on line 5,
/// `ready: bool` on line 6 — and `states`, which open on line 8.
fn machine(states: &str) -> String {
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <datamodel>
    <data id="count" sce:type="uint32" expr="0"/>
    <data id="ready" sce:type="bool" expr="false"/>
  </datamodel>
  {states}
  <final id="done"/>
</scxml>
"##
    )
}

#[test]
fn a_machine_whose_every_expression_is_typed_is_accepted() {
    let (ok, out) = run(
        &["check"],
        &machine(
            r#"<state id="s">
    <onentry><log label="n" expr="count"/></onentry>
    <transition event="tick" cond="count &lt; 10 &amp;&amp; In('s')" target="s">
      <assign location="count" expr="count + 1"/>
      <if cond="count === 5"><assign location="ready" expr="true"/></if>
    </transition>
    <transition event="go" cond="ready" target="done"/>
  </state>"#,
        ),
    );
    assert!(ok, "every expression here is typed:\n{out}");
}

#[test]
fn a_variable_initialised_with_a_value_of_another_kind_is_refused() {
    let (ok, out) = run(
        &["check"],
        &doc(
            "sce-static",
            r#"<data id="ready" sce:type="bool" expr="1"/>"#,
        ),
    );
    assert!(
        !ok,
        "an integer does not stand where a bool is declared:\n{out}"
    );
    assert_refused_at(&out, "expression/type-mismatch", 5);
}

#[test]
fn a_variable_with_no_initial_value_is_refused() {
    let (ok, out) = run(
        &["check"],
        &doc("sce-static", r#"<data id="count" sce:type="uint32"/>"#),
    );
    assert!(
        !ok,
        "a field's initial value is written, not implied:\n{out}"
    );
    assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
}

#[test]
fn a_condition_that_is_not_a_bool_is_refused_on_its_line() {
    let (ok, out) = run(
        &["check"],
        &machine(
            r#"<state id="s"><transition event="go" cond="count + 1" target="done"/></state>"#,
        ),
    );
    assert!(!ok, "a condition is a bool:\n{out}");
    assert_refused_at(&out, "expression/type-mismatch", 8);
}

#[test]
fn a_name_nothing_declares_is_refused() {
    // The scope is closed: there is no script engine behind it to supply
    // a name the document does not declare.
    let (ok, out) = run(
        &["check"],
        &machine(
            r#"<state id="s"><transition event="go" cond="cuont &gt; 1" target="done"/></state>"#,
        ),
    );
    assert!(!ok, "`cuont` is declared nowhere:\n{out}");
    assert!(
        out.contains("cuont"),
        "the refusal names what was written:\n{out}"
    );
}

#[test]
fn an_assignment_of_another_kind_is_refused() {
    let (ok, out) = run(
        &["check"],
        &machine(
            r#"<state id="s"><onentry><assign location="count" expr="true"/></onentry></state>"#,
        ),
    );
    assert!(!ok, "a bool does not stand in a uint32:\n{out}");
    assert_refused_at(&out, "expression/type-mismatch", 8);
}

#[test]
fn an_expression_the_model_has_no_typed_form_for_is_refused() {
    // `targetexpr` is evaluated as script text by every backend's templates,
    // so admitting it would run part of the document in a language it never
    // declared.
    let (ok, out) = run(
        &["check"],
        &machine(
            r#"<state id="s"><onentry><send event="go" targetexpr="'#_internal'"/></onentry></state>"#,
        ),
    );
    assert!(!ok, "targetexpr has no typed form here:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 8);
}

// ── A host action's arguments are typed expressions ─────────────────────

#[test]
fn a_host_action_takes_typed_datamodel_arguments_with_no_event_in_scope() {
    // Under any other data model an argument is a bare `_event.data.<field>`
    // and an eventless action takes none. Here an argument is any typed
    // expression over the scope, so a variable is admitted in `<onentry>`.
    let (ok, out) = run(
        &["check", "-l", "kotlin"],
        &machine(
            r#"<state id="s">
    <onentry>
      <sce:action name="show">
        <sce:arg name="n" expr="count"/>
        <sce:arg name="done" expr="count &gt;= 3 || ready"/>
      </sce:action>
    </onentry>
  </state>"#,
        ),
    );
    assert!(ok, "typed arguments are admitted and lowered:\n{out}");
}

#[test]
fn a_host_action_argument_reading_a_payload_with_none_in_scope_is_refused() {
    // With no triggering event there is no payload to type `_event.data`.
    let (ok, out) = run(
        &["check"],
        &machine(
            r#"<state id="s">
    <onentry>
      <sce:action name="show"><sce:arg expr="_event.data.n"/></sce:action>
    </onentry>
  </state>"#,
        ),
    );
    assert!(!ok, "no payload is in scope in <onentry>:\n{out}");
    assert!(
        out.contains("validation/native-action-argument"),
        "expected the native-action argument refusal:\n{out}"
    );
}

// ── A record variable is built whole and updated a field at a time ──────

/// The event-schema a `record:Day` variable is held in.
const SCHEMA_DAY: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="schema_day" sce:event-name="day.picked">
  <datamodel>
    <data id="year" sce:type="uint16" sce:direction="in"/>
    <data id="month" sce:type="uint8" sce:direction="in"/>
    <data id="dayOfMonth" sce:type="uint8" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// Every field of `Day`, each given once — lines 7 to 9 of [`record`].
const EVERY_FIELD: &str = r#"<sce:set name="year" expr="2026"/>
      <sce:set name="month" expr="9"/>
      <sce:set name="dayOfMonth" expr="24"/>"#;

/// A `sce-static` machine holding one `record:Day` variable, `shown`, whose
/// `<data>` opens on line 6 and whose `sets` start on line 7, and `states`.
fn record(sets: &str, states: &str) -> String {
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <sce:import kind="event-schema" src="schema_day.scxml" as="Day"/>
  <datamodel>
    <data id="shown" sce:type="record:Day">
      {sets}
    </data>
  </datamodel>
  {states}
</scxml>
"##
    )
}

fn run_record(args: &[&str], doc: &str) -> (bool, String) {
    run_beside(args, doc, &[("schema_day.scxml", SCHEMA_DAY)])
}

/// A list of records (SCE_FORGE.md §4.12) is a machine variable too: it starts
/// empty, takes a bound, and its elements are declared as a record
/// variable's are. What it is filled and walked with is
/// `a_list_holds_records.rs`'s.
#[test]
fn a_list_of_records_variable_is_accepted() {
    let (ok, out) = run_record(
        &["check", "-l", "kotlin"],
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <sce:import kind="event-schema" src="schema_day.scxml" as="Day"/>
  <datamodel>
    <data id="days" sce:type="list&lt;record:Day&gt;" sce:capacity="4"/>
  </datamodel>
  <state id="s"/>
</scxml>
"##,
    );
    assert!(ok, "a machine variable is a list of records:\n{out}");
}

#[test]
fn a_record_variable_is_read_and_updated_a_field_at_a_time() {
    // A field is read in a condition and in a host action's argument, and
    // assigned from its own old value and from the payload of the schema's
    // event — every read going through the one scope.
    let (ok, out) = run_record(
        &["check", "-l", "kotlin"],
        &record(
            EVERY_FIELD,
            r#"<state id="s">
    <onentry><sce:action name="show"><sce:arg name="day" expr="shown.dayOfMonth"/></sce:action></onentry>
    <transition event="next" cond="shown.dayOfMonth &lt; 28" type="internal">
      <assign location="shown.dayOfMonth" expr="shown.dayOfMonth + 1"/>
    </transition>
    <transition event="day.picked" type="internal">
      <assign location="shown.year" expr="_event.data.year"/>
    </transition>
  </state>"#,
        ),
    );
    assert!(ok, "a record read and updated field by field:\n{out}");
}

#[test]
fn a_record_missing_a_field_is_refused_at_its_type() {
    let (ok, out) = run_record(
        &["check"],
        &record(
            r#"<sce:set name="year" expr="2026"/>
      <sce:set name="month" expr="9"/>"#,
            r#"<state id="s"/>"#,
        ),
    );
    assert!(!ok, "`dayOfMonth` is given by nothing:\n{out}");
    assert_refused_at(&out, "validation/attribute-rule-violated", 6);
    assert!(
        out.contains("dayOfMonth"),
        "the refusal names the field:\n{out}"
    );
}

#[test]
fn a_field_the_schema_does_not_declare_is_refused_at_its_name() {
    let (ok, out) = run_record(
        &["check"],
        &record(
            &format!("{EVERY_FIELD}\n      <sce:set name=\"weekday\" expr=\"1\"/>"),
            r#"<state id="s"/>"#,
        ),
    );
    assert!(!ok, "Day has no `weekday`:\n{out}");
    assert_refused_at(&out, "validation/attribute-rule-violated", 10);
}

#[test]
fn a_field_given_twice_is_refused_at_the_second() {
    let (ok, out) = run_record(
        &["check"],
        &record(
            &format!("{EVERY_FIELD}\n      <sce:set name=\"year\" expr=\"2027\"/>"),
            r#"<state id="s"/>"#,
        ),
    );
    assert!(!ok, "`year` is given twice:\n{out}");
    assert_refused_at(&out, "validation/attribute-rule-violated", 10);
}

#[test]
fn a_field_value_of_another_kind_is_refused_on_its_line() {
    let (ok, out) = run_record(
        &["check"],
        &record(
            r#"<sce:set name="year" expr="2026"/>
      <sce:set name="month" expr="true"/>
      <sce:set name="dayOfMonth" expr="24"/>"#,
            r#"<state id="s"/>"#,
        ),
    );
    assert!(!ok, "a bool does not stand in a uint8 field:\n{out}");
    assert_refused_at(&out, "expression/type-mismatch", 8);
}

#[test]
fn a_record_variable_with_an_expr_is_refused() {
    // There is no record literal: the `<sce:set>`s are the initial value.
    let (ok, out) = run_record(
        &["check"],
        &record(EVERY_FIELD, r#"<state id="s"/>"#).replace(
            r#"sce:type="record:Day">"#,
            r#"sce:type="record:Day" expr="0">"#,
        ),
    );
    assert!(!ok, "a record variable takes no expr:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 6);
}

#[test]
fn a_field_nothing_declares_is_refused_where_it_is_read() {
    let (ok, out) = run_record(
        &["check"],
        &record(
            EVERY_FIELD,
            r#"<state id="s"><transition event="next" cond="shown.weekday &gt; 1" type="internal"/></state>"#,
        ),
    );
    assert!(!ok, "`shown` is closed over Day's fields:\n{out}");
    assert_refused_at(&out, "expression/unknown-member", 12);
}

#[test]
fn a_whole_record_is_assigned_only_from_a_record_by_name() {
    // Nothing in an expression makes a record: one is taken whole from another
    // record, by name, from the payload of an event whose schema it is, or from
    // the item of a loop over a list of it. `a_list_holds_records.rs` holds the
    // first and `a_payload_is_a_record_of_its_schema.rs` the payload.
    let (ok, out) = run_record(
        &["check"],
        &record(
            EVERY_FIELD,
            r#"<state id="s"><transition event="day.picked" type="internal"><assign location="shown" expr="0"/></transition></state>"#,
        ),
    );
    assert!(!ok, "a record is taken whole from a record:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 12);
}

// ── A child session is handed what its invoke's <param>s and namelist name ──
//
// Under `sce-static` a variable is a native field, and a field is set by the
// machine's own code, so a value handed to a child session is accepted only
// where the child gives it a field to arrive in: the child is a `sce-static`
// document this build read, declares the name as a top-level `<data>`, and
// declares it as a bool, a string, an integer or a real; the value is held to
// that type as an `<assign>` to it would be. Anything else would be typed here,
// accepted, and never delivered — measured 2026-10-01 on Rust and Kotlin, when
// no generated code handed a parent's `<param>` to a child — so it is refused
// where it is written. Each of Rust and Kotlin delivers the value
// (`a_static_child_is_handed_its_params` in each backend's tests).

/// A `sce-static` parent holding `count: uint32`, whose first state invokes a
/// child session with `params` written inside the `<invoke>`. The `<state>`
/// opens on line 8 and the `<invoke>` on line 9, so a param written on its own
/// line is on line 10.
fn invoking(params: &str) -> String {
    invoking_with(
        "",
        params,
        r#"<data id="start" sce:type="uint32" expr="0"/>"#,
    )
}

/// [`invoking`], with `invoke_attrs` on the `<invoke>` and `child_data` as the
/// child's `<datamodel>`.
fn invoking_with(invoke_attrs: &str, params: &str, child_data: &str) -> String {
    machine(&format!(
        r#"<state id="s">
    <invoke type="scxml" id="child" {invoke_attrs}>
      {params}
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
               version="1.0" initial="busy" datamodel="sce-static">
          <datamodel>{child_data}</datamodel>
          <state id="busy"><transition event="finish" target="end"/></state>
          <final id="end"/>
        </scxml>
      </content>
    </invoke>
    <transition event="done.invoke.child" target="done"/>
  </state>"#
    ))
}

#[test]
fn a_child_session_with_no_param_is_accepted() {
    // The control: the document the cases below differ from only by a
    // `<param>`.
    for lang in ["rust", "kotlin"] {
        let (ok, out) = run(&["check", "-l", lang], &invoking(""));
        assert!(ok, "{lang}: an invoke with no param is fine:\n{out}");
    }
}

#[test]
fn a_value_the_child_declares_a_variable_for_is_accepted() {
    for (what, param) in [
        ("an expression", r#"<param name="start" expr="count"/>"#),
        ("a literal", r#"<param name="start" expr="7"/>"#),
        ("a location", r#"<param name="start" location="count"/>"#),
        (
            "a computed value",
            r#"<param name="start" expr="count + 1"/>"#,
        ),
    ] {
        for lang in ["rust", "kotlin"] {
            let (ok, out) = run(&["check", "-l", lang], &invoking(param));
            assert!(ok, "{lang}, {what}: the child declares `start`:\n{out}");
        }
    }
}

#[test]
fn a_namelist_name_is_handed_as_the_param_it_abbreviates() {
    // `namelist="count"` is `<param name="count" expr="count"/>`, so the child
    // declares `count` and the value is the parent's.
    let document = invoking_with(
        r#"namelist="count""#,
        "",
        r#"<data id="count" sce:type="uint32" expr="0"/>"#,
    );
    for lang in ["rust", "kotlin"] {
        let (ok, out) = run(&["check", "-l", lang], &document);
        assert!(ok, "{lang}: the child declares `count`:\n{out}");
    }
    let (ok, out) = run(
        &["check"],
        &invoking_with(
            r#"namelist="count""#,
            "",
            r#"<data id="other" sce:type="uint32" expr="0"/>"#,
        ),
    );
    assert!(!ok, "the child declares no `count`:\n{out}");
    assert!(out.contains("count"), "it names what was written:\n{out}");
}

#[test]
fn c11_starts_a_static_child_and_hands_it_a_number() {
    // C holds a child in the parent's own struct and starts it in two steps,
    // handing it what the invoke's `<param>`s name between them
    // (docs/SCE_ACCEPTED_SUBSET.md §2.15, "Child sessions").
    let (ok, out) = run(
        &["check", "-l", "c11"],
        &invoking(r#"<param name="start" expr="count + 1"/>"#),
    );
    assert!(ok, "c11: the child declares `start`, a uint32:\n{out}");
}

#[test]
fn every_backend_hands_a_string_to_a_child_that_declares_its_bound() {
    // A string in a child is a buffer of the bound its variable declares, and a
    // value arriving from another machine is held to that bound as an
    // `<assign>` to it would be (docs/SCE_ACCEPTED_SUBSET.md §2.15, "Child
    // sessions"), so every backend that starts a child takes one.
    let document = invoking_with(
        "",
        r#"<param name="title" expr="'x'"/>"#,
        r#"<data id="title" sce:type="string" sce:capacity="8" expr="''"/>"#,
    );
    for lang in ["rust", "kotlin", "cpp", "python", "c11"] {
        let (ok, out) = run(&["check", "-l", lang], &document);
        assert!(ok, "{lang}: the child declares `title`, a string:\n{out}");
    }
    let (ok, out) = run(
        &["check", "-l", "go", "--go-module-prefix", "x/y"],
        &document,
    );
    assert!(ok, "go: the child declares `title`, a string:\n{out}");
}

#[test]
fn every_language_refuses_a_child_that_needs_a_host_to_perform_its_actions() {
    // The child's machine takes the host that performs its acts when it is
    // built, and its parent has none to give it: what a parent would write is
    // that constructor called without one, which Rust, Kotlin, Go and C++ do not
    // compile and Python fails at when the invoke starts. C11 holds the child as
    // a value with no host to give its act table to.
    let document = machine(
        r#"<state id="s">
    <invoke type="scxml" id="child">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
               version="1.0" initial="busy" datamodel="sce-static">
          <datamodel><data id="start" sce:type="uint32" expr="0"/></datamodel>
          <state id="busy">
            <onentry>
              <sce:action name="announce"><sce:arg name="seen" expr="start"/></sce:action>
            </onentry>
            <transition event="finish" target="end"/>
          </state>
          <final id="end"/>
        </scxml>
      </content>
    </invoke>
    <transition event="done.invoke.child" target="done"/>
  </state>"#,
    );
    for (lang, name) in [
        ("rust", "Rust"),
        ("kotlin", "Kotlin"),
        ("cpp", "C++"),
        ("python", "Python"),
        ("c11", "C11"),
    ] {
        let (ok, out) = run(&["check", "-l", lang], &document);
        assert!(!ok, "{lang} has no lowering for it yet:\n{out}");
        assert!(
            out.contains("generate/unsupported-feature")
                && out.contains(&format!("no {name} lowering yet")),
            "{lang}: expected the unsupported-feature refusal naming {name}:\n{out}"
        );
        assert!(
            out.contains(r#"an <invoke id=\"child\"> of a child that declares <sce:action>s"#),
            "{lang}: it names the invoke:\n{out}"
        );
    }
    let (ok, out) = run(
        &["check", "-l", "go", "--go-module-prefix", "x/y"],
        &document,
    );
    assert!(!ok, "go has no lowering for it yet:\n{out}");
    assert!(
        out.contains("no Go lowering yet")
            && out.contains(r#"an <invoke id=\"child\"> of a child that declares <sce:action>s"#),
        "go: expected the refusal naming Go and the invoke:\n{out}"
    );
}

/// A child written beside the document, declaring `start: uint32`.
const CHILD_BESIDE: &str = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="busy" datamodel="sce-static">
  <datamodel><data id="start" sce:type="uint32" expr="0"/></datamodel>
  <state id="busy"><transition event="finish" target="end"/></state>
  <final id="end"/>
</scxml>
"##;

/// `invoking`, with the child in a document beside the parent (`src`) rather
/// than written inside the `<invoke>`. The `<param>` is on line 10 here too.
fn invoking_beside(params: &str) -> String {
    machine(&format!(
        r#"<state id="s">
    <invoke type="scxml" id="child" src="child.scxml">
      {params}
    </invoke>
    <transition event="done.invoke.child" target="done"/>
  </state>"#
    ))
}

#[test]
fn a_child_written_beside_the_document_is_held_to_its_own_variables() {
    let siblings = [("child.scxml", CHILD_BESIDE)];
    for lang in ["rust", "kotlin"] {
        let (ok, out) = run_beside(
            &["check", "-l", lang],
            &invoking_beside(r#"<param name="start" expr="count"/>"#),
            &siblings,
        );
        assert!(ok, "{lang}: the child beside declares `start`:\n{out}");
    }
    let (ok, out) = run_beside(
        &["check"],
        &invoking_beside(r#"<param name="other" expr="count"/>"#),
        &siblings,
    );
    assert!(!ok, "the child beside declares no `other`:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 10);
}

#[test]
fn a_value_the_child_has_no_variable_for_is_refused_on_its_param() {
    for (what, param, says) in [
        (
            "a name the child does not declare",
            r#"<param name="other" expr="count"/>"#,
            "declares no top-level",
        ),
        (
            "two values for one name",
            r#"<param name="start" expr="count"/><param name="start" expr="7"/>"#,
            "handed this name twice",
        ),
        (
            "a param whose value is blank",
            r#"<param name="start" expr=" "/>"#,
            "names none",
        ),
    ] {
        let (ok, out) = run(&["check"], &invoking(param));
        assert!(!ok, "{what}: the value would never arrive:\n{out}");
        assert_refused_at(&out, "scxml/static-datamodel-rule", 10);
        assert!(out.contains(says), "{what}: it says why:\n{out}");
    }
}

#[test]
fn a_value_of_another_type_than_the_childs_variable_is_refused() {
    // The value is held to the variable's type as an `<assign>` to it would be:
    // a bool does not land in a `uint32`.
    let (ok, out) = run(
        &["check"],
        &invoking(r#"<param name="start" expr="ready"/>"#),
    );
    assert!(!ok, "a bool is not a uint32:\n{out}");
    assert!(out.contains("ready"), "it names what was written:\n{out}");
}

#[test]
fn a_child_under_another_data_model_is_given_no_typed_value() {
    // Nothing here knows the type of a variable an engine holds, so the value
    // has no typed place to arrive in.
    let document = invoking_with(
        "",
        r#"<param name="start" expr="count"/>"#,
        r#"<data id="start" expr="0"/>"#,
    )
    .replacen(
        r#"initial="busy" datamodel="sce-static""#,
        r#"initial="busy" datamodel="ecmascript""#,
        1,
    );
    let (ok, out) = run(&["check"], &document);
    assert!(!ok, "an ecmascript child has no typed variable:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 10);
}

#[test]
fn a_param_is_still_given_to_a_child_under_ecmascript() {
    // The refusals above are of this data model's lacking a typed field to
    // arrive in, not of `<param>`: under `ecmascript` the value is seeded into
    // the child's datamodel.
    let document = invoking(r#"<param name="start" expr="count"/>"#)
        .replace(r#"datamodel="sce-static""#, r#"datamodel="ecmascript""#)
        .replace(r#" sce:type="uint32""#, "")
        .replace(r#" sce:type="bool""#, "");
    let (ok, out) = run(&["check"], &document);
    assert!(ok, "ecmascript hands a param to its child:\n{out}");
}

// ── A hybrid <invoke> starts the candidate its srcexpr names ────────────
//
// An `<invoke srcexpr>` names its child when it starts. Under `sce-static`
// that child is one of the documents `sce:candidates` declares — the value
// names one by its document stem, and the build has generated each — so a
// document that does not declare any, or that PRODUCES the child's text with a
// `<content expr>`, has no finite set of children to lower and is refused where
// it is written. The invoke's arguments are the same whichever child is
// chosen, and what each candidate keeps of them is its own (W3C SCXML 6.4.3):
// a name it declares is typed against that variable, and a name no candidate
// declares would be dropped by every one.

/// A candidate declaring `start: uint32` and `ready: bool`.
const HYBRID_FIRST: &str = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="busy" datamodel="sce-static" name="first">
  <datamodel>
    <data id="start" sce:type="uint32" expr="0"/>
    <data id="ready" sce:type="bool" expr="false"/>
  </datamodel>
  <state id="busy"><transition event="finish" target="end"/></state>
  <final id="end"/>
</scxml>
"##;

/// A candidate declaring `start: uint32` and nothing else.
const HYBRID_SECOND: &str = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="busy" datamodel="sce-static" name="second">
  <datamodel>
    <data id="start" sce:type="uint32" expr="0"/>
  </datamodel>
  <state id="busy"><transition event="finish" target="end"/></state>
  <final id="end"/>
</scxml>
"##;

/// The `<invoke>` attributes of a hybrid invoke of the two candidates above.
const HYBRID_ATTRS: &str = r#"srcexpr="pick" sce:candidates="first.scxml second.scxml""#;

fn hybrid_siblings() -> [(&'static str, &'static str); 2] {
    [
        ("first.scxml", HYBRID_FIRST),
        ("second.scxml", HYBRID_SECOND),
    ]
}

/// A `sce-static` parent holding `pick: string` (line 5), `count: uint32` and
/// `ready: bool`, whose first state opens on line 9 and hybrid-invokes with
/// `invoke_attrs` on line 10, so a child written on its own line is on line 11.
fn hybrid_invoking(invoke_attrs: &str, params: &str) -> String {
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <datamodel>
    <data id="pick" sce:type="string" sce:capacity="32" expr="'first.scxml'"/>
    <data id="count" sce:type="uint32" expr="0"/>
    <data id="ready" sce:type="bool" expr="false"/>
  </datamodel>
  <state id="s">
    <invoke type="scxml" id="child" {invoke_attrs}>
      {params}
    </invoke>
    <transition event="done.invoke.child" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"##
    )
}

#[test]
fn a_hybrid_invoke_among_declared_static_candidates_is_accepted() {
    // `start` is declared by both candidates and `ready` by one: each keeps the
    // names it declares and leaves out the rest (W3C SCXML 6.4.3).
    let document = hybrid_invoking(
        r#"srcexpr="pick" namelist="ready" sce:candidates="first.scxml second.scxml""#,
        r#"<param name="start" expr="count + 1"/>"#,
    );
    for args in [&["check"][..], &["check", "-l", "rust"][..]] {
        let (ok, out) = run_beside(args, &document, &hybrid_siblings());
        assert!(ok, "{args:?}: both candidates are static documents:\n{out}");
    }
}

#[test]
fn a_hybrid_invoke_that_declares_no_candidate_is_refused_on_its_srcexpr() {
    let (ok, out) = run_beside(
        &["check"],
        &hybrid_invoking(r#"srcexpr="pick""#, ""),
        &hybrid_siblings(),
    );
    assert!(!ok, "there is no child to start:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 10);
    assert!(
        out.contains("sce:candidates"),
        "it says what to write:\n{out}"
    );
}

#[test]
fn a_hybrid_invoke_that_produces_its_child_is_refused_on_its_content() {
    // `sce:candidates` beside a `contentexpr` is refused by the parser, before
    // this model is asked (`validation/incompatible-attributes`), so what
    // reaches it is a `contentexpr` that declares no set at all.
    let (ok, out) = run_beside(
        &["check"],
        &hybrid_invoking("", r#"<content expr="pick"/>"#),
        &hybrid_siblings(),
    );
    assert!(
        !ok,
        "a produced document has no finite set to lower:\n{out}"
    );
    assert_refused_at(&out, "scxml/static-datamodel-rule", 11);
    assert!(
        out.contains("no finite set of documents"),
        "it says why:\n{out}"
    );
}

#[test]
fn a_candidate_under_another_data_model_is_refused_on_the_invoke() {
    let ecmascript = HYBRID_FIRST
        .replace(r#"datamodel="sce-static""#, r#"datamodel="ecmascript""#)
        .replace(r#" sce:type="uint32""#, "")
        .replace(r#" sce:type="bool""#, "");
    let (ok, out) = run_beside(
        &["check"],
        &hybrid_invoking(HYBRID_ATTRS, ""),
        &[
            ("first.scxml", &ecmascript),
            ("second.scxml", HYBRID_SECOND),
        ],
    );
    assert!(!ok, "a value has no typed variable to arrive in:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 10);
    assert!(
        out.contains("first.scxml"),
        "it names the candidate:\n{out}"
    );
}

#[test]
fn a_name_no_candidate_declares_is_refused_on_its_param() {
    let (ok, out) = run_beside(
        &["check"],
        &hybrid_invoking(HYBRID_ATTRS, r#"<param name="other" expr="count"/>"#),
        &hybrid_siblings(),
    );
    assert!(!ok, "every candidate would drop it:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 11);
    assert!(out.contains("no candidate declares"), "it says why:\n{out}");
}

#[test]
fn a_value_of_another_type_than_a_candidates_variable_is_refused() {
    // Held to the variable as an `<assign>` to it would be: a bool does not
    // land in a `uint32`, even though the OTHER candidate would take it.
    let (ok, out) = run_beside(
        &["check"],
        &hybrid_invoking(HYBRID_ATTRS, r#"<param name="start" expr="ready"/>"#),
        &hybrid_siblings(),
    );
    assert!(!ok, "a bool is not a uint32:\n{out}");
    assert!(out.contains("ready"), "it names what was written:\n{out}");
}

#[test]
fn a_srcexpr_that_is_not_a_string_is_refused() {
    let (ok, out) = run_beside(
        &["check"],
        &hybrid_invoking(
            r#"srcexpr="count" sce:candidates="first.scxml second.scxml""#,
            "",
        ),
        &hybrid_siblings(),
    );
    assert!(!ok, "a number names no document:\n{out}");
    assert!(out.contains("count"), "it names what was written:\n{out}");
}

#[test]
fn a_hybrid_invoke_is_still_given_its_candidates_under_ecmascript() {
    // The refusals above are of this data model's having no typed place for a
    // value and no finite set of documents, not of `sce:candidates`.
    let document = hybrid_invoking(HYBRID_ATTRS, r#"<param name="start" expr="count"/>"#)
        .replace(r#"datamodel="sce-static""#, r#"datamodel="ecmascript""#)
        .replace(r#" sce:type="uint32""#, "")
        .replace(r#" sce:type="bool""#, "")
        .replace(r#" sce:type="string" sce:capacity="32""#, "");
    let (ok, out) = run_beside(&["check"], &document, &hybrid_siblings());
    assert!(ok, "ecmascript starts a candidate by its value:\n{out}");
}

// ── A <finalize> is run by no machine of this data model ────────────────
//
// The model keeps an `<invoke>`'s `<finalize>` as script text (the parser
// transpiles its actions to one string), so no type rule reaches it, and the
// generated code hands that string to a script engine this data model never
// builds: the Rust body is an empty block and Kotlin reads the engine
// `executeFinalizeForChildEvent` finds absent. Measured 2026-10-01 on the
// generated code of a `sce-static` parent: the document was accepted, `check`
// said nothing, and the assignment never ran.
//
// Running one natively is not a matter of lowering the body: a `<finalize>` runs
// before the next event from ANY of the child's events is processed, to read that
// event's `_event.data`, and no type rule reaches a payload that arrives from
// whichever event comes next. A body that reads no payload has no consumer, so it
// stays refused until a typed form of the payload exists.

/// The control and the refusal's document: `invoking` with a `<finalize>` that
/// assigns `count`, written on the line after the `<invoke>` opens (line 10).
fn finalizing() -> String {
    invoking(r#"<finalize><assign location="count" expr="count + 1"/></finalize>"#)
}

#[test]
fn a_finalize_no_machine_here_would_run_is_refused_on_its_invoke() {
    let (ok, out) = run(&["check"], &finalizing());
    assert!(!ok, "no generated code runs it:\n{out}");
    // The model records the element the `<finalize>` is written in, not the
    // `<finalize>` itself, so the refusal sits on the `<invoke>` (line 9).
    assert_refused_at(&out, "scxml/static-datamodel-rule", 9);
}

#[test]
fn the_finalize_refusal_names_the_invoke_and_says_what_would_happen() {
    let (_, out) = run(&["check"], &finalizing());
    let record = out
        .lines()
        .find(|l| l.contains("\"code\":\"scxml/static-datamodel-rule\""))
        .unwrap_or_else(|| panic!("expected a static-datamodel-rule refusal:\n{out}"));
    assert!(
        record.contains("finalize"),
        "it names the element:\n{record}"
    );
    assert!(record.contains("child"), "it names the invoke:\n{record}");
    assert!(
        record.contains("never run"),
        "it says the body would be accepted and not executed:\n{record}"
    );
}

#[test]
fn an_empty_finalize_beside_a_param_is_refused_as_the_update_it_stands_for() {
    // §scxml-6.5.2 gives an empty `<finalize/>` the meaning "update each
    // `<param location>` from the event's data of that name", and the model
    // writes that update out as script text. Now that a child's `<param>` is
    // accepted, the pair reaches the same refusal a written body does, rather
    // than being accepted and never run.
    let (ok, out) = run(
        &["check"],
        &invoking(r#"<param name="start" location="count"/><finalize/>"#),
    );
    assert!(!ok, "the update it stands for is never run:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 9);
    assert!(out.contains("finalize"), "it names the element:\n{out}");

    // An empty `<finalize/>` with nothing to update is inert, as the clause
    // leaves it.
    let (ok, out) = run(&["check"], &invoking("<finalize/>"));
    assert!(ok, "there is nothing for it to update:\n{out}");
}

#[test]
fn a_finalize_is_still_run_under_ecmascript() {
    // The refusal is of this data model's having no engine to run the text, not
    // of `<finalize>`: `ecmascript` builds one.
    let document = finalizing()
        .replace(r#"datamodel="sce-static""#, r#"datamodel="ecmascript""#)
        .replace(r#" sce:type="uint32""#, "")
        .replace(r#" sce:type="bool""#, "");
    let (ok, out) = run(&["check"], &document);
    assert!(ok, "ecmascript runs a finalize:\n{out}");
}

#[test]
fn a_host_run_invokes_param_is_still_accepted() {
    // The refusal is of a CHILD SESSION's `<param>`. A host-run invoke's is part
    // of the request the host receives, so it has somewhere to arrive, and is
    // judged as a typed expression like any other.
    let host = machine(
        r#"<state id="s">
    <invoke type="x-sce-host" id="h"><param name="k" expr="count"/></invoke>
    <transition event="done.invoke.h" target="done"/>
  </state>"#,
    );
    let (ok, out) = run(&["check"], &host);
    assert!(ok, "a param addressed to the host is not refused:\n{out}");

    // ... and it is judged: a name nothing declares is still refused there.
    let (ok, out) = run(
        &["check"],
        &host.replace(r#"expr="count""#, r#"expr="cuont""#),
    );
    assert!(!ok, "`cuont` is declared nowhere:\n{out}");
    assert!(out.contains("cuont"), "it names what was written:\n{out}");
}

// ── What a <param> carries across to the host ───────────────────────────
//
// A `<param>` of a `<send>` or of an `<invoke>` the host runs is a typed
// expression, read from the machine's fields when the element runs, and it
// crosses as text and as a JSON value. The values every backend spells alike
// are carried; any other is refused where it is written, not dropped or carried
// differently by two backends.

/// A `sce-static` machine whose `<state>` (line 8) sends to the host from its
/// `<onentry>`, with `params` on line 11.
fn sending(params: &str) -> String {
    machine(&format!(
        r#"<state id="s">
    <onentry>
      <send type="x-sce-host" event="notify">
        {params}
      </send>
    </onentry>
    <transition event="go" target="done"/>
  </state>"#
    ))
}

#[test]
fn every_value_a_param_can_carry_is_accepted() {
    for expr in ["count", "ready", "count + 1", "7", "1.5", "'text'"] {
        let params = format!(r#"<param name="k" expr="{expr}"/>"#);
        let (ok, out) = run(&["check"], &sending(&params));
        assert!(
            ok,
            "`{expr}` is a bool, a string, a narrow integer or a real:\n{out}"
        );
    }
    // A location names a variable and is read as one.
    let (ok, out) = run(
        &["check"],
        &sending(r#"<param name="k" location="count"/>"#),
    );
    assert!(ok, "a location names a variable:\n{out}");
}

#[test]
fn a_param_whose_value_has_no_wire_spelling_is_refused_on_its_line() {
    // `big` is a 64-bit integer: a backend that reads numbers through a double
    // would carry one past 2^53 with its low bits wrong, and say nothing.
    let document = sending(r#"<param name="k" expr="big"/>"#).replace(
        r#"<data id="ready" sce:type="bool" expr="false"/>"#,
        r#"<data id="big" sce:type="int64" expr="0"/>"#,
    );
    let (ok, out) = run(&["check"], &document);
    assert!(
        !ok,
        "a 64-bit integer has no wire spelling every backend shares:\n{out}"
    );
    assert_refused_at(&out, "scxml/static-datamodel-rule", 11);
    assert!(out.contains("64-bit"), "it says why:\n{out}");
}

#[test]
fn a_param_reading_the_events_payload_is_accepted_in_a_transition_on_that_event() {
    // The transition's content runs only for a delivery that carried the
    // payload, as an `<assign>` that reads it does, so a `<send>` of it is
    // carried on like any other value.
    for expr in ["_event.data.dayOfMonth", "_event.data.month + 1"] {
        let (ok, out) = run_record(
            &["check"],
            &record(
                EVERY_FIELD,
                &format!(
                    r#"<state id="s"><transition event="day.picked" type="internal"><send type="x-sce-host" event="forward"><param name="d" expr="{expr}"/></send></transition></state>"#
                ),
            ),
        );
        assert!(
            ok,
            "`{expr}` is read from the payload of the event the transition is on:\n{out}"
        );
    }
}

#[test]
fn a_param_reading_a_payload_that_is_not_in_scope_is_refused_on_its_line() {
    // An entry runs when no event's payload is in scope, a transition on an
    // event that declares no schema has no typed payload to read, and what a
    // `<final>` hands its done event is read as the state is entered.
    let param = r#"<param name="d" expr="_event.data.dayOfMonth"/>"#;
    for (what, states) in [
        (
            "an entry",
            format!(
                r#"<state id="s"><onentry><send type="x-sce-host" event="forward">{param}</send></onentry></state>"#
            ),
        ),
        (
            "a transition on an event with no schema",
            format!(
                r#"<state id="s"><transition event="other" type="internal"><send type="x-sce-host" event="forward">{param}</send></transition></state>"#
            ),
        ),
        (
            "a donedata",
            format!(
                r#"<state id="s"><transition event="go" target="d"/></state><final id="d"><donedata>{param}</donedata></final>"#
            ),
        ),
    ] {
        let (ok, out) = run_record(&["check"], &record(EVERY_FIELD, &states));
        assert!(!ok, "{what}: no payload is in scope to read:\n{out}");
        assert_refused_at(&out, "scxml/static-datamodel-rule", 12);
        assert!(
            out.contains("declares a schema"),
            "{what}: it says where the payload is read:\n{out}"
        );
    }
}

/// A `sce-static` machine whose entry (line 8) sends with `namelist`.
fn sending_by_namelist(namelist: &str) -> String {
    machine(&format!(
        r#"<state id="s"><onentry><send event="notify" namelist="{namelist}"/></onentry></state>"#
    ))
}

#[test]
fn a_send_namelist_names_variables_the_machine_holds() {
    // Each name is the `<param name="x" expr="x"/>` it abbreviates, so a bool
    // and a narrow integer cross as they do in a param.
    let (ok, out) = run(&["check"], &sending_by_namelist("count ready"));
    assert!(ok, "a namelist of variables is typed values:\n{out}");
}

#[test]
fn a_send_namelist_is_lowered_to_the_pairs_it_abbreviates_and_needs_no_engine() {
    // The names are read from the machine's fields when the send runs, as a
    // `<param>`'s value is, so no engine evaluates them and the manifest says so.
    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run(
        &[
            "generate",
            "-l",
            "go",
            "-o",
            out_dir.path().to_str().expect("a path"),
        ],
        &sending_by_namelist("count ready"),
    );
    assert!(ok, "the machine generates:\n{out}");
    assert!(
        out.contains("\"needs_script_engine\":false"),
        "a namelist of variables needs no script engine:\n{out}"
    );
    let generated: Vec<_> = std::fs::read_dir(out_dir.path())
        .expect("the output directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "go"))
        .collect();
    assert_eq!(generated.len(), 1, "one machine: {generated:?}");
    let source = std::fs::read_to_string(&generated[0]).expect("a generated machine");
    for name in ["count", "ready"] {
        assert!(
            source.contains(&format!("Name: \"{name}\"")),
            "`{name}` is carried as a pair of the event's data"
        );
    }
    assert!(
        !source.contains("EvaluateExpression"),
        "no pair is read by an engine"
    );
}

#[test]
fn a_send_namelist_name_is_held_to_the_rule_a_param_is() {
    // A name nothing declares reads no variable.
    let (ok, out) = run(&["check"], &sending_by_namelist("count missing"));
    assert!(!ok, "a name that is no variable is refused:\n{out}");
    assert!(out.contains("missing"), "it names the name:\n{out}");

    // A 64-bit variable has no wire spelling every backend shares, and the
    // refusal is placed at the attribute, which is where the name is written.
    let document = sending_by_namelist("big").replace(
        r#"<data id="ready" sce:type="bool" expr="false"/>"#,
        r#"<data id="big" sce:type="int64" expr="0"/>"#,
    );
    let (ok, out) = run(&["check"], &document);
    assert!(!ok, "a 64-bit variable crosses with no spelling:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 8);
    assert!(out.contains("64-bit"), "it says why:\n{out}");
}

// ── A `<send>`'s `<content expr>` names a record ────────────────────────

/// A `sce-static` machine holding the record variable `shown` whose `states`
/// are on line 12.
fn sending_content(states: &str) -> String {
    record(EVERY_FIELD, states)
}

#[test]
fn a_send_content_that_names_a_record_is_accepted() {
    // A record variable, and the payload of the event the transition is on,
    // each the pairs of its fields.
    for states in [
        r#"<state id="s"><onentry><send event="out"><content expr="shown"/></send></onentry></state>"#,
        r#"<state id="s"><transition event="day.picked" type="internal"><send event="out"><content expr="_event.data"/></send></transition></state>"#,
    ] {
        let (ok, out) = run_record(&["check"], &sending_content(states));
        assert!(ok, "a record crosses as the pairs of its fields:\n{out}");
    }
}

#[test]
fn a_send_content_that_names_one_value_is_accepted() {
    // An expression that is no record is the one value the event carries, held
    // to the rule a param's value is: a field of a record and an operation on one.
    for expr in ["shown.year + 1", "shown.month", "shown.year > 2000"] {
        let states = format!(
            r#"<state id="s"><onentry><send event="out"><content expr="{expr}"/></send></onentry></state>"#
        );
        let (ok, out) = run_record(&["check"], &sending_content(&states));
        assert!(ok, "`{expr}` is one value the event carries:\n{out}");
    }
    // A variable of the machine, a bool and a number, each crossing alone.
    for expr in ["count", "ready", "count * 2"] {
        let (ok, out) = run(&["check"], &sending_content_of_a_value(expr));
        assert!(ok, "`{expr}` is one value the event carries:\n{out}");
    }
}

#[test]
fn a_send_content_that_names_no_record_or_value_is_refused_on_its_line() {
    for (what, states) in [
        (
            "the payload of an entry, where none is in scope",
            r#"<state id="s"><onentry><send event="out"><content expr="_event.data"/></send></onentry></state>"#,
        ),
        (
            "a field of the payload of an entry, where none is in scope",
            r#"<state id="s"><onentry><send event="out"><content expr="_event.data.year"/></send></onentry></state>"#,
        ),
        (
            "a record beside a param",
            r#"<state id="s"><onentry><send event="out"><content expr="shown"/><param name="k" expr="1"/></send></onentry></state>"#,
        ),
        (
            "a value beside a param",
            r#"<state id="s"><onentry><send event="out"><content expr="shown.year + 1"/><param name="k" expr="1"/></send></onentry></state>"#,
        ),
    ] {
        let (ok, out) = run_record(&["check"], &sending_content(states));
        assert!(!ok, "{what}: no record or value is named alone:\n{out}");
        assert_refused_at(&out, "scxml/static-datamodel-rule", 12);
    }
}

/// A `sce-static` machine whose entry (line 8) sends the one value `expr`
/// computes as its `<content>`.
fn sending_content_of_a_value(expr: &str) -> String {
    machine(&format!(
        r#"<state id="s"><onentry><send event="out"><content expr="{expr}"/></send></onentry></state>"#
    ))
}

#[test]
fn a_send_content_value_is_held_to_the_rule_a_param_is() {
    // A name nothing declares reads no variable.
    let (ok, out) = run(&["check"], &sending_content_of_a_value("missing"));
    assert!(!ok, "a name that is no variable is refused:\n{out}");
    assert!(out.contains("missing"), "it names the name:\n{out}");

    // A 64-bit variable has no wire spelling every backend shares, and the
    // refusal is placed at the attribute and says it is the content it refuses.
    let document = sending_content_of_a_value("big").replace(
        r#"<data id="ready" sce:type="bool" expr="false"/>"#,
        r#"<data id="big" sce:type="int64" expr="0"/>"#,
    );
    let (ok, out) = run(&["check"], &document);
    assert!(!ok, "a 64-bit variable crosses with no spelling:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 8);
    assert!(out.contains("64-bit"), "it says why:\n{out}");
    assert!(
        out.contains("<content expr=\\\"big\\\">"),
        "it names the content, not a param:\n{out}"
    );
}

#[test]
fn a_send_content_value_is_lowered_to_one_typed_value_and_needs_no_engine() {
    // The value is read from the machine's fields when the send runs, as a
    // `<param>`'s is, so no engine evaluates it and the manifest says so; it is
    // the event's whole data, so it is written as one value and not as a pair.
    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run(
        &[
            "generate",
            "-l",
            "go",
            "-o",
            out_dir.path().to_str().expect("a path"),
        ],
        &sending_content_of_a_value("count * 2"),
    );
    assert!(ok, "the machine generates:\n{out}");
    assert!(
        out.contains("\"needs_script_engine\":false"),
        "a value named by a content needs no script engine:\n{out}"
    );
    let generated: Vec<_> = std::fs::read_dir(out_dir.path())
        .expect("the output directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "go"))
        .collect();
    assert_eq!(generated.len(), 1, "one machine: {generated:?}");
    let source = std::fs::read_to_string(&generated[0]).expect("a generated machine");
    assert!(
        source.contains("sce.ScriptValueToJSON(contentValue)"),
        "the value is the event's whole data"
    );
    assert!(
        !source.contains("Name: \"") && !source.contains("EvaluateExpression"),
        "no pair is built and no engine reads the value"
    );
}

#[test]
fn a_send_content_record_is_lowered_to_the_pairs_of_its_fields_and_needs_no_engine() {
    // The record is read from the machine's fields when the send runs, as a
    // `<param>`'s value is, so no engine evaluates it and the manifest says so.
    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run_record(
        &[
            "generate",
            "-l",
            "go",
            "-o",
            out_dir.path().to_str().expect("a path"),
        ],
        &sending_content(
            r#"<state id="s"><onentry><send event="out"><content expr="shown"/></send></onentry></state>"#,
        ),
    );
    assert!(ok, "the machine generates:\n{out}");
    assert!(
        out.contains("\"needs_script_engine\":false"),
        "a record named by a content needs no script engine:\n{out}"
    );
    let generated: Vec<_> = std::fs::read_dir(out_dir.path())
        .expect("the output directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "go"))
        .collect();
    assert_eq!(generated.len(), 1, "one machine: {generated:?}");
    let source = std::fs::read_to_string(&generated[0]).expect("a generated machine");
    for field in ["year", "month", "dayOfMonth"] {
        assert!(
            source.contains(&format!("Name: \"{field}\"")),
            "`{field}` is carried as a pair of the event's data"
        );
    }
    assert!(
        !source.contains("EvaluateExpression"),
        "no pair is read by an engine"
    );
}

#[test]
fn a_go_machine_that_joins_text_and_a_number_imports_what_it_names() {
    // The join is written as `strconv.FormatUint` where the statement stands, and
    // a Go file that names a package without importing it does not compile: the
    // import is read from the text the machine's own lowering wrote.
    let document = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <datamodel>
    <data id="n" sce:type="uint32" expr="7"/>
    <data id="label" sce:type="string" sce:capacity="16" expr="''"/>
  </datamodel>
  <state id="s">
    <transition event="go" type="internal">
      <assign location="label" expr="'n' + n"/>
    </transition>
  </state>
</scxml>
"#;
    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run(
        &[
            "generate",
            "-l",
            "go",
            "-o",
            out_dir.path().to_str().expect("a path"),
        ],
        document,
    );
    assert!(ok, "the machine generates:\n{out}");
    let generated: Vec<_> = std::fs::read_dir(out_dir.path())
        .expect("the output directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "go"))
        .collect();
    assert_eq!(generated.len(), 1, "one machine: {generated:?}");
    let source = std::fs::read_to_string(&generated[0]).expect("a generated machine");
    assert!(
        source.contains("strconv.Format"),
        "the join is written with strconv:\n{source}"
    );
    assert!(
        source.contains("\t\"strconv\""),
        "and the file imports the package it names"
    );
}

#[test]
fn a_namelist_name_that_repeats_a_param_is_refused_for_c() {
    // C11 collects the `<param>`s of one name into one array, in document order,
    // as every engine does. Where a `namelist` name stands among the `<param>`s
    // that share it is not a document order the engines were held to, so a
    // `namelist` name that a `<param>` of the same send already names is refused
    // by name.
    let document = machine(
        r#"<state id="s"><onentry><send event="out" namelist="count"><param name="count" expr="1"/></send></onentry></state>"#,
    );
    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run(
        &[
            "generate",
            "-l",
            "c",
            "-o",
            out_dir.path().to_str().expect("a path"),
        ],
        &document,
    );
    assert!(!ok, "a name carried twice has no C11 lowering:\n{out}");
    assert!(
        out.contains("a <send> whose namelist names `count`"),
        "it says why:\n{out}"
    );
}

/// The text of the C source `generate -l c` writes for `document`, with `args`.
fn generated_c_source(document: &str, args: &[&str]) -> String {
    let out_dir = tempdir().expect("tempdir");
    let mut all = vec![
        "generate",
        "-l",
        "c",
        "-o",
        out_dir.path().to_str().expect("a path"),
    ];
    all.extend_from_slice(args);
    let (ok, out) = run(&all, document);
    assert!(ok, "C generation is not refused:\n{out}");
    generated_c(out_dir.path())
}

/// How many times the generated `source` writes a pair named `name` into the wire.
fn pairs_named(source: &str, name: &str) -> usize {
    source
        .matches(&format!(
            "sce_forge_wire_pair(&sce_wire_, \"\\\"{name}\\\"\""
        ))
        .count()
        + source
            .matches(&format!(
                "sce_forge_wire_pair(&_host_inv_wire, \"\\\"{name}\\\"\""
            ))
            .count()
}

#[test]
fn a_param_name_that_repeats_is_written_as_pairs_the_wire_collects_for_c() {
    // One array on every engine (ARCHITECTURE.md, "JSON Object Key Order"): the
    // generated code lists the pairs of a name one after another, in document
    // order, and the wire writer of the forge runtime collects them.
    let send = generated_c_source(
        &machine(
            r#"<state id="s"><onentry><send event="x"><param name="k" expr="count"/><param name="j" expr="count"/><param name="k" expr="count + 1"/></send></onentry></state>"#,
        ),
        &[],
    );
    assert_eq!(pairs_named(&send, "k"), 2, "a <send>'s two `k`s:\n{send}");
    assert_eq!(pairs_named(&send, "j"), 1, "and the one `j`:\n{send}");
    let k = send.find("\\\"k\\\"").expect("the first pair of k");
    let k2 = send[k + 1..]
        .find("\\\"k\\\"")
        .expect("the second pair of k")
        + k
        + 1;
    assert!(
        !send[k..k2].contains("\\\"j\\\""),
        "the pairs of a name are listed together, `j` not between them:\n{send}"
    );
    let donedata = generated_c_source(
        &machine(
            r#"<state id="s"><transition event="go" target="fin"/></state>
  <final id="fin"><donedata><param name="k" expr="count"/><param name="k" expr="count + 1"/></donedata></final>"#,
        ),
        &[],
    );
    assert_eq!(
        pairs_named(&donedata, "k"),
        2,
        "a <donedata>'s two `k`s:\n{donedata}"
    );
    let invoke = generated_c_source(
        &invoking_the_host(
            r#"<invoke type="x-sce-host" id="h"><param name="k" expr="count"/><param name="k" expr="count + 1"/></invoke>"#,
        ),
        &["--host-invoker", "x-sce-host"],
    );
    assert_eq!(
        pairs_named(&invoke, "k"),
        2,
        "a host-run <invoke>'s two `k`s:\n{invoke}"
    );
}

/// A `sce-static` machine whose entry (line 8) sends with the attributes `send`.
fn sending_with(send: &str) -> String {
    machine(&format!(
        r#"<state id="s"><onentry><send event="notify" {send}/></onentry></state>"#
    ))
}

#[test]
fn a_send_delayexpr_is_a_string_the_machine_computes() {
    // A literal, and a number joined to its unit: the CSS2 time a delay is
    // written in, read from the machine's fields when the send runs.
    for expr in ["'100ms'", "count + 'ms'", "count + 's'"] {
        let (ok, out) = run(&["check"], &sending_with(&format!(r#"delayexpr="{expr}""#)));
        assert!(ok, "`{expr}` is a string:\n{out}");
    }
}

#[test]
fn a_send_delayexpr_that_is_no_string_or_is_doubled_is_refused_on_its_line() {
    // Each is refused on the line of the send, under the code of the rule it
    // breaks: a type is the expression pass's, a delay written twice the model's.
    for (what, send, code) in [
        (
            "a number, which is no time",
            r#"delayexpr="count""#,
            "expression/type-mismatch",
        ),
        (
            "a delay beside it",
            r#"delay="1s" delayexpr="'2s'""#,
            "scxml/static-datamodel-rule",
        ),
    ] {
        let (ok, out) = run(&["check"], &sending_with(send));
        assert!(!ok, "{what}: no delay is had from it:\n{out}");
        assert_refused_at(&out, code, 8);
    }
    // A name nothing declares reads no variable, whatever the pass that says so.
    let (ok, out) = run(&["check"], &sending_with(r#"delayexpr="missing + 'ms'""#));
    assert!(!ok, "a name no variable declares is refused:\n{out}");
    assert!(out.contains("missing"), "it names the name:\n{out}");
}

#[test]
fn a_send_delayexpr_is_lowered_to_the_string_it_computes_and_needs_no_engine() {
    // Read from the machine's fields when the send runs, as a `<param>`'s value
    // is, so no engine evaluates it: the manifest says so, and says that the
    // machine must be driven with `tick()` for the delay to come due.
    let document = sending_with(r#"delayexpr="count + 'ms'""#);
    for language in ["rust", "go", "kotlin", "python", "cpp"] {
        let out_dir = tempdir().expect("tempdir");
        let (ok, out) = run(
            &[
                "generate",
                "-l",
                language,
                "--go-module-prefix",
                "x/y",
                "-o",
                out_dir.path().to_str().expect("a path"),
            ],
            &document,
        );
        assert!(ok, "{language}: the machine generates:\n{out}");
        assert!(
            out.contains("\"needs_script_engine\":false"),
            "{language}: a delay computed from the fields needs no engine:\n{out}"
        );
        assert!(
            out.contains("\"needs_event_scheduler\":true"),
            "{language}: a delayed send needs the host to drive tick():\n{out}"
        );
    }

    // Go writes the delay as the string it computes and reads it with the one
    // duration reader every engine shares, with no engine to ask.
    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run(
        &[
            "generate",
            "-l",
            "go",
            "-o",
            out_dir.path().to_str().expect("a path"),
        ],
        &document,
    );
    assert!(ok, "the machine generates:\n{out}");
    let generated: Vec<_> = std::fs::read_dir(out_dir.path())
        .expect("the output directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "go"))
        .collect();
    assert_eq!(generated.len(), 1, "one machine: {generated:?}");
    let source = std::fs::read_to_string(&generated[0]).expect("a generated machine");
    assert!(
        source.contains("sce.ParseDelayToMs(sceDelay)"),
        "the computed string is read as a CSS2 time"
    );
    assert!(
        !source.contains("EvaluateExpression"),
        "no engine evaluates the delay"
    );
}

#[test]
fn a_send_delayexpr_that_joins_text_is_written_for_c_into_a_buffer_the_model_sizes() {
    // A C string is a bounded buffer, so the join is written into one sized from
    // what the data model declares: the ten digits of a `uint32`, the two bytes
    // of `ms` and the terminator. A literal needs none.
    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run(
        &[
            "generate",
            "-l",
            "c",
            "-o",
            out_dir.path().to_str().expect("a path"),
        ],
        &sending_with(r#"delayexpr="count + 'ms'""#),
    );
    assert!(ok, "operands the model sizes have a C11 lowering:\n{out}");
    let source = generated_c(out_dir.path());
    assert!(
        source.contains("SCE_FORGE_CONCAT((char[13]){0}, 13, sce_forge_wire_uint("),
        "the join is a buffer of 10 + 2 + 1 bytes:\n{source}"
    );
    assert!(
        source.contains(r#"sce_forge_wire_string("ms")"#),
        "the literal is a string part:\n{source}"
    );

    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run(
        &[
            "generate",
            "-l",
            "c",
            "-o",
            out_dir.path().to_str().expect("a path"),
        ],
        &sending_with(r#"delayexpr="'100ms'""#),
    );
    assert!(
        ok,
        "a string that is only written generates for C11:\n{out}"
    );
}

/// A `sce-static` machine whose entry (line 8) sends with the attributes `send`
/// and no `event` of its own.
fn sending_event(send: &str) -> String {
    machine(&format!(
        r#"<state id="s"><onentry><send {send}/></onentry></state>"#
    ))
}

#[test]
fn a_send_eventexpr_is_a_string_the_machine_computes() {
    // The name of the event is a string, written out or joined from the machine's
    // fields, read when the send runs.
    for expr in ["'notify'", "'on.' + count", "'go'"] {
        let (ok, out) = run(
            &["check"],
            &sending_event(&format!(r#"eventexpr="{expr}""#)),
        );
        assert!(ok, "`{expr}` is a string:\n{out}");
    }
}

#[test]
fn a_send_eventexpr_that_is_no_string_or_is_doubled_is_refused_on_its_line() {
    for (what, send, code) in [
        (
            "a number, which names no event",
            r#"eventexpr="count""#,
            "expression/type-mismatch",
        ),
        (
            "an event beside it",
            r#"event="a" eventexpr="'b'""#,
            "scxml/static-datamodel-rule",
        ),
    ] {
        let (ok, out) = run(&["check"], &sending_event(send));
        assert!(!ok, "{what}: no event is named by it:\n{out}");
        assert_refused_at(&out, code, 8);
    }
    let (ok, out) = run(&["check"], &sending_event(r#"eventexpr="missing + 'x'""#));
    assert!(!ok, "a name no variable declares is refused:\n{out}");
    assert!(out.contains("missing"), "it names the name:\n{out}");
}

#[test]
fn a_send_eventexpr_is_lowered_to_the_string_it_computes_and_needs_no_engine() {
    // Read from the machine's fields when the send runs, as a `<param>`'s value
    // is, so no engine evaluates it and the manifest says so.
    let document = sending_event(r#"eventexpr="'on.' + count""#);
    for language in ["rust", "go", "kotlin", "python", "cpp"] {
        let out_dir = tempdir().expect("tempdir");
        let (ok, out) = run(
            &[
                "generate",
                "-l",
                language,
                "--go-module-prefix",
                "x/y",
                "-o",
                out_dir.path().to_str().expect("a path"),
            ],
            &document,
        );
        assert!(ok, "{language}: the machine generates:\n{out}");
        assert!(
            out.contains("\"needs_script_engine\":false"),
            "{language}: an event named by the fields needs no engine:\n{out}"
        );
    }
    // Go delivers the name it computed, with no engine to ask.
    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run(
        &[
            "generate",
            "-l",
            "go",
            "-o",
            out_dir.path().to_str().expect("a path"),
        ],
        &document,
    );
    assert!(ok, "the machine generates:\n{out}");
    let generated: Vec<_> = std::fs::read_dir(out_dir.path())
        .expect("the output directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "go"))
        .collect();
    assert_eq!(generated.len(), 1, "one machine: {generated:?}");
    let source = std::fs::read_to_string(&generated[0]).expect("a generated machine");
    assert!(
        source.contains("engine.SendNamedExternal(sendEventName"),
        "the computed name is the event delivered"
    );
    assert!(
        !source.contains("EvaluateExpression"),
        "no engine evaluates the name"
    );
}

#[test]
fn a_send_eventexpr_that_joins_text_is_written_for_c_into_a_buffer_the_model_sizes() {
    // The join of a literal and a `uint32` is a buffer of 3 + 10 + 1 bytes; a
    // name that is only written out needs none.
    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run(
        &[
            "generate",
            "-l",
            "c",
            "-o",
            out_dir.path().to_str().expect("a path"),
        ],
        &sending_event(r#"eventexpr="'on.' + count""#),
    );
    assert!(ok, "operands the model sizes have a C11 lowering:\n{out}");
    let source = generated_c(out_dir.path());
    assert!(
        source.contains(r#"SCE_FORGE_CONCAT((char[14]){0}, 14, sce_forge_wire_string("on.")"#),
        "the join is a buffer of 3 + 10 + 1 bytes:\n{source}"
    );

    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run(
        &[
            "generate",
            "-l",
            "c",
            "-o",
            out_dir.path().to_str().expect("a path"),
        ],
        &sending_event(r#"eventexpr="'notify'""#),
    );
    assert!(ok, "a name that is only written generates for C11:\n{out}");
}

/// [`sending_event`] over a machine that also holds the string `prefix`, bounded
/// to six bytes, and the `int8` `level`.
fn sending_event_over_variables(send: &str) -> String {
    sending_event(send).replace(
        r#"<data id="ready" sce:type="bool" expr="false"/>"#,
        r#"<data id="ready" sce:type="bool" expr="false"/>
    <data id="prefix" sce:type="string" sce:capacity="6" expr="'ab'"/>
    <data id="level" sce:type="int8" expr="0"/>"#,
    )
}

#[test]
fn a_join_for_c_is_sized_by_a_variables_capacity_and_an_integers_width() {
    // The C source of the machine whose `eventexpr` is `event`.
    let generate = |event: &str| {
        let out_dir = tempdir().expect("tempdir");
        let (ok, out) = run(
            &[
                "generate",
                "-l",
                "c",
                "-o",
                out_dir.path().to_str().expect("a path"),
            ],
            &sending_event_over_variables(&format!(r#"eventexpr="{event}""#)),
        );
        assert!(ok, "`{event}` has a C11 lowering:\n{out}");
        generated_c(out_dir.path())
    };

    // A string variable by the capacity it declares (6), a literal by its text
    // (1), an `int8` by its three digits and its sign (4), and the terminator.
    let source = generate("prefix + '.' + level");
    assert!(
        source.contains("SCE_FORGE_CONCAT((char[12]){0}, 12, sce_forge_wire_string("),
        "6 + 1 + 4 + 1 bytes:\n{source}"
    );
    // A chain is one buffer, not a join of a join.
    assert_eq!(
        source.matches("SCE_FORGE_CONCAT(").count(),
        1,
        "one join for the chain:\n{source}"
    );

    // A conditional is as long as its longer branch, whichever it is.
    let source = generate("(ready ? 'ab' : prefix) + 'x'");
    assert!(
        source.contains("SCE_FORGE_CONCAT((char[8]){0}, 8, "),
        "6 + 1 + 1 bytes:\n{source}"
    );
}

// ── A `<cancel>`'s `sendidexpr` is a string the machine computes ────────

/// A `sce-static` machine whose entry (line 8) cancels with the attributes
/// `cancel`.
fn cancelling(cancel: &str) -> String {
    machine(&format!(
        r#"<state id="s"><onentry><cancel {cancel}/></onentry></state>"#
    ))
}

#[test]
fn a_cancel_sendidexpr_is_a_string_the_machine_computes() {
    // The id of the send to remove is a string, written out or joined from the
    // machine's fields, read when the cancel runs.
    for expr in ["'timer'", "'timer.' + count", "ready ? 'a' : 'b'"] {
        let (ok, out) = run(&["check"], &cancelling(&format!(r#"sendidexpr="{expr}""#)));
        assert!(ok, "`{expr}` is a string:\n{out}");
    }
}

#[test]
fn a_cancel_sendidexpr_that_is_no_string_or_is_doubled_is_refused_on_its_line() {
    for (what, cancel, code) in [
        (
            "a number, which names no send",
            r#"sendidexpr="count""#,
            "expression/type-mismatch",
        ),
        (
            "a sendid beside it",
            r#"sendid="a" sendidexpr="'b'""#,
            "scxml/static-datamodel-rule",
        ),
    ] {
        let (ok, out) = run(&["check"], &cancelling(cancel));
        assert!(!ok, "{what}: no send is named by it:\n{out}");
        assert_refused_at(&out, code, 8);
    }
    let (ok, out) = run(&["check"], &cancelling(r#"sendidexpr="missing + 'x'""#));
    assert!(!ok, "an id no variable declares is refused:\n{out}");
    assert!(out.contains("missing"), "it names the name:\n{out}");
}

#[test]
fn a_cancel_sendidexpr_is_lowered_to_the_string_it_computes_and_needs_no_engine() {
    // Read from the machine's fields when the cancel runs, so no engine
    // evaluates it and the manifest says so.
    let document = cancelling(r#"sendidexpr="'timer.' + count""#);
    for language in ["rust", "go", "kotlin", "python", "cpp"] {
        let out_dir = tempdir().expect("tempdir");
        let (ok, out) = run(
            &[
                "generate",
                "-l",
                language,
                "--go-module-prefix",
                "x/y",
                "-o",
                out_dir.path().to_str().expect("a path"),
            ],
            &document,
        );
        assert!(ok, "{language}: the machine generates:\n{out}");
        assert!(
            out.contains("\"needs_script_engine\":false"),
            "{language}: an id computed from the fields needs no engine:\n{out}"
        );
    }
    // Go hands the scheduler the id it computed, with no engine to ask.
    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run(
        &[
            "generate",
            "-l",
            "go",
            "-o",
            out_dir.path().to_str().expect("a path"),
        ],
        &document,
    );
    assert!(ok, "the machine generates:\n{out}");
    let generated: Vec<_> = std::fs::read_dir(out_dir.path())
        .expect("the output directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "go"))
        .collect();
    assert_eq!(generated.len(), 1, "one machine: {generated:?}");
    let source = std::fs::read_to_string(&generated[0]).expect("a generated machine");
    assert!(
        source.contains("engine.CancelEvent(cancelSendID)"),
        "the computed id is the one cancelled"
    );
    assert!(
        !source.contains("EvaluateExpression"),
        "no engine evaluates the id"
    );
}

#[test]
fn a_cancel_sendidexpr_that_joins_text_is_written_for_c_into_a_buffer_the_model_sizes() {
    // The join of a literal and a `uint32` is a buffer of 6 + 10 + 1 bytes; an
    // id that is only written out needs none.
    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run(
        &[
            "generate",
            "-l",
            "c",
            "-o",
            out_dir.path().to_str().expect("a path"),
        ],
        &cancelling(r#"sendidexpr="'timer.' + count""#),
    );
    assert!(ok, "operands the model sizes have a C11 lowering:\n{out}");
    let source = generated_c(out_dir.path());
    assert!(
        source.contains(r#"SCE_FORGE_CONCAT((char[17]){0}, 17, sce_forge_wire_string("timer.")"#),
        "the join is a buffer of 6 + 10 + 1 bytes:\n{source}"
    );

    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run(
        &[
            "generate",
            "-l",
            "c",
            "-o",
            out_dir.path().to_str().expect("a path"),
        ],
        &cancelling(r#"sendidexpr="'timer'""#),
    );
    assert!(ok, "an id that is only written generates for C11:\n{out}");
}

// ── A `<send>`'s `idlocation` is a string variable the machine writes to ──

/// A `sce-static` machine holding `data` (on line 5) whose entry (line 7) sends
/// with the attributes `send`.
fn identifying(data: &str, send: &str) -> String {
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <datamodel>
    {data}
  </datamodel>
  <state id="s"><onentry><send {send}/></onentry></state>
  <final id="done"/>
</scxml>
"##
    )
}

const ID_HOLDER: &str = r#"<data id="id" sce:type="string" sce:capacity="32" expr="''"/>"#;

#[test]
fn a_send_idlocation_names_a_string_variable_the_id_fits() {
    // A string variable declares the most bytes it holds; one that holds the id
    // declares at least the longest the machine generates (31 bytes) and a
    // terminator's room, which is what a C buffer needs.
    for data in [
        ID_HOLDER,
        r#"<data id="id" sce:type="string" sce:capacity="64" expr="''"/>"#,
    ] {
        let (ok, out) = run(
            &["check"],
            &identifying(data, r#"idlocation="id" event="e" delay="1s""#),
        );
        assert!(ok, "`{data}` holds the id:\n{out}");
    }
}

#[test]
fn an_invoke_idlocation_is_refused_for_want_of_a_reader_and_says_so() {
    // The id it would store is the one the build already wrote, and a machine of
    // this model reads no `_event.invokeid`, so nothing could compare it.
    let (ok, out) = run(
        &["check"],
        &machine(
            r#"<state id="s"><invoke type="scxml" idlocation="count"><content><scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="c"><final id="c"/></scxml></content></invoke></state>"#,
        ),
    );
    assert!(!ok, "an invoke's idlocation has no reader here:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 8);
    assert!(
        out.contains("_event.invokeid") && out.contains("write `id`"),
        "it says what is missing and what to write instead:\n{out}"
    );
}

#[test]
fn a_send_idlocation_that_cannot_hold_the_id_or_names_no_variable_is_refused_on_its_line() {
    for (what, data, send) in [
        (
            "a variable whose bound the id does not fit",
            r#"<data id="id" sce:type="string" sce:capacity="31" expr="''"/>"#,
            r#"idlocation="id" event="e""#,
        ),
        (
            "a variable that is no string",
            r#"<data id="id" sce:type="uint32" expr="0"/>"#,
            r#"idlocation="id" event="e""#,
        ),
        (
            "a variable no <data> declares",
            ID_HOLDER,
            r#"idlocation="missing" event="e""#,
        ),
        (
            "an id written beside the one generated",
            ID_HOLDER,
            r#"id="a" idlocation="id" event="e""#,
        ),
    ] {
        let (ok, out) = run(&["check"], &identifying(data, send));
        assert!(!ok, "{what}: the id has nowhere to go:\n{out}");
        assert_refused_at(&out, "scxml/static-datamodel-rule", 7);
    }
    // The refusal says what bound the id needs.
    let (_, out) = run(
        &["check"],
        &identifying(
            r#"<data id="id" sce:type="string" sce:capacity="31" expr="''"/>"#,
            r#"idlocation="id" event="e""#,
        ),
    );
    assert!(
        out.contains("31 UTF-8 bytes") && out.contains("at least 32"),
        "it names the longest id and the bound it needs:\n{out}"
    );
}

#[test]
fn the_bound_the_judge_holds_an_id_holder_to_is_the_one_every_runtime_states() {
    // The judge's number and the runtimes' are one fact written twice: the
    // longest id (`_auto_send_` and the twenty digits of the largest count).
    let runtime = std::fs::read_to_string(
        repo_root().join("backends/rust/runtime/src/helpers/unique_id_generator.rs"),
    )
    .expect("the Rust runtime's id generator");
    let stated = format!(
        "pub const AUTO_SEND_ID_MAX_LEN: usize = {};",
        sce_build::forge::static_datamodel::AUTO_SEND_ID_MAX_BYTES
    );
    assert!(
        runtime.contains(&stated),
        "the runtime states the same longest id: `{stated}`"
    );
    assert_eq!(
        "_auto_send_".len() + u64::MAX.to_string().len(),
        sce_build::forge::static_datamodel::AUTO_SEND_ID_MAX_BYTES as usize,
        "and it is what the format makes of the largest count"
    );
}

#[test]
fn a_send_idlocation_is_lowered_to_a_write_of_the_machines_own_count_and_needs_no_engine() {
    // The machine counts the ids it generates and writes the next one to the
    // variable, ahead of everything else the send does, so no engine stores it
    // and the manifest says so. The send is then known by the id that variable
    // holds, read back from it. Each backend spells these its own way.
    let document = identifying(ID_HOLDER, r#"idlocation="id" event="e" delay="1s""#);
    for (language, counter, read_back, no_engine_store) in [
        (
            "rust",
            "engine.next_auto_send_id()",
            "let send_id_text: String = self.id.to_string();",
            "store_id_in_location",
        ),
        (
            "go",
            "engine.NextAutoSendID()",
            "sendID := p.vId",
            "storeIDInLocation",
        ),
        (
            "kotlin",
            "nextAutoSendId()",
            "val sendIdGenerated: String = id",
            "storeIdInLocation",
        ),
        (
            "python",
            "engine._next_auto_sendid()",
            "_sid = self.v_id",
            "self._resolve_send_id(",
        ),
        (
            "cpp",
            "engine.nextAutoSendId()",
            "std::string sendId = v_id;",
            "storeIdInLocation",
        ),
        (
            "c",
            // Taken once, into a local: the copy into the variable reads its
            // value twice, and a count taken twice is two ids.
            "const char *sce_fresh_id_ = sce_next_auto_send_id(&sm->auto_send_seq",
            "const char *_sce_send_id = sm->policy.v_id.data;",
            "_idloc_ok",
        ),
    ] {
        let (out, files) = generated_code(language, &document);
        assert!(
            out.contains("\"needs_script_engine\":false"),
            "{language}: an id the machine counts needs no engine:\n{out}"
        );
        let code: String = files.values().cloned().collect::<Vec<_>>().join("\n");
        assert!(
            code.contains(counter),
            "{language}: the machine generates the id from its own count (`{counter}`)"
        );
        assert!(
            code.contains(read_back),
            "{language}: the send is known by the id the variable holds (`{read_back}`)"
        );
        assert!(
            !code.contains(no_engine_store),
            "{language}: no engine stores the id (`{no_engine_store}`)"
        );
    }
}

#[test]
fn a_c_machine_carries_a_count_and_a_buffer_only_when_it_generates_an_id() {
    let (_, with_id) = generated_code(
        "c",
        &identifying(ID_HOLDER, r#"idlocation="id" event="e" delay="1s""#),
    );
    let header: String = with_id
        .iter()
        .filter(|(name, _)| name.ends_with(".h"))
        .map(|(_, text)| text.as_str())
        .collect();
    assert!(header.contains("uint64_t auto_send_seq;"), "the count");
    assert!(
        header.contains("char auto_send_id[SCE_AUTO_SEND_ID_BUF_LEN];"),
        "the buffer the id is formatted into"
    );
    assert!(
        header.contains("SCE_STATIC_ASSERT(SCE_MAX_ID_LEN >= SCE_AUTO_SEND_ID_BUF_LEN"),
        "an id is never cut short into another's"
    );

    let (_, without_id) = generated_code("c", &identifying(ID_HOLDER, r#"event="e" delay="1s""#));
    let header: String = without_id
        .iter()
        .filter(|(name, _)| name.ends_with(".h"))
        .map(|(_, text)| text.as_str())
        .collect();
    assert!(
        !header.contains("auto_send_seq"),
        "a machine that generates no id carries no count"
    );
}

/// A `sce-static` machine holding the record variable `shown` whose `<final>`
/// carries a `<donedata>` of `content`, all on line 12.
fn finishing_with_a_record(content: &str) -> String {
    record(
        EVERY_FIELD,
        &format!(
            r#"<state id="s"><transition event="go" target="d"/></state><final id="d"><donedata>{content}</donedata></final>"#
        ),
    )
}

#[test]
fn a_donedata_content_that_names_a_record_is_the_pairs_of_its_fields() {
    // Read as the state is entered, from the machine's fields, so no engine
    // evaluates it and the manifest says so.
    let content = r#"<content expr="shown"/>"#;
    let (ok, out) = run_record(&["check"], &finishing_with_a_record(content));
    assert!(ok, "a record crosses as the pairs of its fields:\n{out}");

    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run_record(
        &[
            "generate",
            "-l",
            "go",
            "-o",
            out_dir.path().to_str().expect("a path"),
        ],
        &finishing_with_a_record(content),
    );
    assert!(ok, "the machine generates:\n{out}");
    assert!(
        out.contains("\"needs_script_engine\":false"),
        "a record named by a donedata content needs no script engine:\n{out}"
    );
    let generated: Vec<_> = std::fs::read_dir(out_dir.path())
        .expect("the output directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "go"))
        .collect();
    assert_eq!(generated.len(), 1, "one machine: {generated:?}");
    let source = std::fs::read_to_string(&generated[0]).expect("a generated machine");
    for field in ["year", "month", "dayOfMonth"] {
        assert!(
            source.contains(&format!("\\\"{field}\\\":")),
            "`{field}` is a pair of the done event's data"
        );
    }

    // C11 writes the pairs as one JSON object of its own, so it is asked too.
    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run_record(
        &[
            "generate",
            "-l",
            "c",
            "-o",
            out_dir.path().to_str().expect("a path"),
        ],
        &finishing_with_a_record(content),
    );
    assert!(ok, "the machine generates for C11:\n{out}");
}

#[test]
fn a_donedata_content_that_names_no_record_or_value_is_refused_on_its_line() {
    for (what, content) in [
        (
            "the payload, which no event has where a state is entered",
            r#"<content expr="_event.data"/>"#,
        ),
        (
            "a field of the payload, which no event has where a state is entered",
            r#"<content expr="_event.data.year"/>"#,
        ),
        (
            "a record beside a param",
            r#"<content expr="shown"/><param name="k" expr="1"/>"#,
        ),
        (
            "a value beside a param",
            r#"<content expr="shown.year + 1"/><param name="k" expr="1"/>"#,
        ),
    ] {
        let (ok, out) = run_record(&["check"], &finishing_with_a_record(content));
        assert!(!ok, "{what}: no record or value is named alone:\n{out}");
        assert_refused_at(&out, "scxml/static-datamodel-rule", 12);
    }
}

#[test]
fn a_donedata_content_that_names_one_value_is_accepted() {
    // An expression that is no record is the one value the done event carries,
    // held to the rule a param's value is, as a `<send>`'s content is.
    for expr in ["shown.year + 1", "shown.month", "shown.year > 2000"] {
        let content = format!(r#"<content expr="{expr}"/>"#);
        let (ok, out) = run_record(&["check"], &finishing_with_a_record(&content));
        assert!(ok, "`{expr}` is one value the done event carries:\n{out}");
    }
    for expr in ["count", "ready", "count * 2"] {
        let content = format!(r#"<content expr="{expr}"/>"#);
        let (ok, out) = run(&["check"], &finishing(&content));
        assert!(ok, "`{expr}` is one value the done event carries:\n{out}");
    }
}

#[test]
fn a_donedata_content_value_is_held_to_the_rule_a_param_is() {
    // A name nothing declares reads no variable.
    let (ok, out) = run(&["check"], &finishing(r#"<content expr="missing"/>"#));
    assert!(!ok, "a name that is no variable is refused:\n{out}");
    assert!(out.contains("missing"), "it names the name:\n{out}");

    // A 64-bit variable has no wire spelling every backend shares, and the
    // refusal is placed at the attribute and says it is the content it refuses.
    let document = finishing(r#"<content expr="big"/>"#).replace(
        r#"<data id="ready" sce:type="bool" expr="false"/>"#,
        r#"<data id="big" sce:type="int64" expr="0"/>"#,
    );
    let (ok, out) = run(&["check"], &document);
    assert!(!ok, "a 64-bit variable crosses with no spelling:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 11);
    assert!(out.contains("64-bit"), "it says why:\n{out}");
    assert!(
        out.contains("<content expr=\\\"big\\\">"),
        "it names the content, not a param:\n{out}"
    );
}

#[test]
fn a_donedata_content_value_is_lowered_to_one_typed_value_and_needs_no_engine() {
    // The value is read from the machine's fields when the state is entered, as
    // a `<param>`'s is, so no engine evaluates it and the manifest says so; it is
    // the done event's whole data, so it is written as one value, not as pairs.
    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run(
        &[
            "generate",
            "-l",
            "go",
            "-o",
            out_dir.path().to_str().expect("a path"),
        ],
        &finishing(r#"<content expr="count * 2"/>"#),
    );
    assert!(ok, "the machine generates:\n{out}");
    assert!(
        out.contains("\"needs_script_engine\":false"),
        "a value named by a donedata content needs no script engine:\n{out}"
    );
    let generated: Vec<_> = std::fs::read_dir(out_dir.path())
        .expect("the output directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "go"))
        .collect();
    assert_eq!(generated.len(), 1, "one machine: {generated:?}");
    let source = std::fs::read_to_string(&generated[0]).expect("a generated machine");
    assert!(
        source.contains("doneEventData = sce.ScriptValueToJSON(contentValue)"),
        "the value is the done event's whole data"
    );
    assert!(
        !source.contains("EvaluateExpression"),
        "no engine reads the value"
    );
}

// ── A `<donedata>` param is the same value on the same wire ─────────────

/// A `sce-static` machine whose `<final>` (line 9) carries a `<donedata>`
/// (line 10) with `params` on line 11.
fn finishing(params: &str) -> String {
    machine(r#"<state id="s"><transition event="go" target="done"/></state>"#).replace(
        r#"<final id="done"/>"#,
        &format!(
            "<final id=\"done\">\n    <donedata>\n      {params}\n    </donedata>\n  </final>"
        ),
    )
}

#[test]
fn every_value_a_donedata_param_can_carry_is_accepted() {
    for expr in ["count", "ready", "count + 1", "7", "1.5", "'text'"] {
        let params = format!(r#"<param name="k" expr="{expr}"/>"#);
        let (ok, out) = run(&["check"], &finishing(&params));
        assert!(
            ok,
            "`{expr}` is a bool, a string, a narrow integer or a real:\n{out}"
        );
    }
    // A location names a variable and is read as one.
    let (ok, out) = run(
        &["check"],
        &finishing(r#"<param name="k" location="count"/>"#),
    );
    assert!(ok, "a location names a variable:\n{out}");
}

#[test]
fn a_donedata_param_whose_value_has_no_wire_spelling_is_refused_on_its_line() {
    // The rule is the one a `<send>`'s param is held to, stated once.
    let document = finishing(r#"<param name="k" expr="big"/>"#).replace(
        r#"<data id="ready" sce:type="bool" expr="false"/>"#,
        r#"<data id="big" sce:type="int64" expr="0"/>"#,
    );
    let (ok, out) = run(&["check"], &document);
    assert!(
        !ok,
        "a 64-bit integer has no wire spelling every backend shares:\n{out}"
    );
    assert_refused_at(&out, "scxml/static-datamodel-rule", 11);
    assert!(out.contains("64-bit"), "it says why:\n{out}");
}

#[test]
fn a_donedata_param_naming_a_variable_that_is_not_declared_is_refused_on_its_line() {
    // Only an `expr` was judged once; a `location` reached an engine unread.
    let (ok, out) = run(
        &["check"],
        &finishing(r#"<param name="k" location="missing"/>"#),
    );
    assert!(
        !ok,
        "a location names a variable the machine declares:\n{out}"
    );
    assert_refused_at(&out, "expression/unknown-identifier", 11);
}

#[test]
fn a_donedata_param_is_lowered_to_native_code_and_not_handed_to_an_engine() {
    // Measured 2026-10-03: judged, accepted and never lowered. The Rust machine
    // called `ensure_script_engine`, which a sce-static machine does not have,
    // and the Kotlin machine raised its done event with no data at all. The
    // manifest says `needs_script_engine:false` either way, so what is read is
    // the generated machine.
    let document = finishing(r#"<param name="k" expr="count + 1"/>"#);
    for (language, extension, native, engine) in [
        ("rust", "rs", "json_parts", "ensure_script_engine"),
        ("kotlin", "kt", "doneParams", "evaluateExpr"),
        (
            "go",
            "go",
            "ScriptValueToJSON(sceValue)",
            "EvaluateExpression",
        ),
        // The generated module keeps its engine helpers whatever the document
        // is, so the spelling that reads a donedata param is the one looked for:
        // a pair whose value is the engine's answer, joined to its name.
        (
            "python",
            "py",
            "_ScriptValue.of(",
            "+ engine._script_engine.evaluate_expression(",
        ),
    ] {
        let out_dir = tempdir().expect("tempdir");
        let (ok, out) = run(
            &[
                "generate",
                "-l",
                language,
                "-o",
                out_dir.path().to_str().expect("a path"),
            ],
            &document,
        );
        assert!(ok, "{language}: the machine generates:\n{out}");
        let generated: Vec<_> = std::fs::read_dir(out_dir.path())
            .expect("the output directory")
            .map(|entry| entry.expect("an entry").path())
            .filter(|path| path.extension().is_some_and(|e| e == extension))
            .collect();
        assert_eq!(generated.len(), 1, "{language}: one machine: {generated:?}");
        let source = std::fs::read_to_string(&generated[0]).expect("a generated machine");
        assert!(
            source.contains(native),
            "{language}: the donedata is built from the lowered value"
        );
        assert!(
            !source.contains(engine),
            "{language}: no script engine reads a donedata param"
        );
    }
}

#[test]
fn a_go_send_param_is_lowered_to_native_code_and_not_handed_to_an_engine() {
    // Go refused a <send> carrying a <param> by name until its value was lowered
    // as the other targets lower it. The manifest says `needs_script_engine:false`
    // either way, so what is read is the generated machine: the value is computed
    // into a local from the machine's own fields and no engine evaluates it.
    let document = machine(
        r#"<state id="s">
    <transition event="go" type="internal">
      <send event="note"><param name="k" expr="count + 1"/></send>
    </transition>
  </state>"#,
    );
    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run(
        &[
            "generate",
            "-l",
            "go",
            "-o",
            out_dir.path().to_str().expect("a path"),
        ],
        &document,
    );
    assert!(ok, "the machine generates:\n{out}");
    assert!(
        out.contains("\"needs_script_engine\":false"),
        "a sce-static machine's params are read from its fields:\n{out}"
    );
    let generated: Vec<_> = std::fs::read_dir(out_dir.path())
        .expect("the output directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "go"))
        .collect();
    assert_eq!(generated.len(), 1, "one machine: {generated:?}");
    let source = std::fs::read_to_string(&generated[0]).expect("a generated machine");
    assert!(
        source.contains("sce.EventDataParam{Name: \"k\", Value: sceValue}"),
        "the pair is built from the lowered value"
    );
    assert!(
        !source.contains("EvaluateExpression"),
        "no script engine reads a send param"
    );
}

#[test]
fn a_python_send_param_is_lowered_to_native_code_and_not_handed_to_an_engine() {
    // Python refused a <send> carrying a <param> by name until its value was
    // lowered as the other targets lower it. The generated module keeps the
    // helper that evaluates a payload through the engine whatever the document
    // is, and a host-run `<invoke>`'s helper calls it too, so what is looked for
    // is the call a `<send>` makes: absent, the pair is built from the lowered
    // value.
    let document = machine(
        r#"<state id="s">
    <transition event="go" type="internal">
      <send event="note"><param name="k" expr="count + 1"/></send>
    </transition>
  </state>"#,
    );
    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run(
        &[
            "generate",
            "-l",
            "python",
            "-o",
            out_dir.path().to_str().expect("a path"),
        ],
        &document,
    );
    assert!(ok, "the machine generates:\n{out}");
    assert!(
        out.contains("\"needs_script_engine\":false"),
        "a sce-static machine's params are read from its fields:\n{out}"
    );
    let generated: Vec<_> = std::fs::read_dir(out_dir.path())
        .expect("the output directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "py"))
        .collect();
    assert_eq!(generated.len(), 1, "one machine: {generated:?}");
    let source = std::fs::read_to_string(&generated[0]).expect("a generated machine");
    assert!(
        source.contains("self._put_param(_data, _data_repeats, \"k\", "),
        "the pair is built from the lowered value"
    );
    assert!(
        !source.contains("_data = self._eval_send_payload("),
        "no script engine reads a send param"
    );
}

#[test]
fn a_cpp_send_param_is_lowered_to_native_code_and_not_handed_to_an_engine() {
    // C++ refused a <send> carrying a <param> or a <content> by name. The value is
    // now computed into a `ScriptValue` from the machine's own fields and put in
    // the typed map the event's JSON is built from; the manifest says
    // `needs_script_engine:false` either way, so what is read is the machine.
    let document = machine(
        r#"<state id="s">
    <transition event="go" type="internal">
      <send event="note"><param name="k" expr="count + 1"/></send>
    </transition>
  </state>"#,
    );
    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run(
        &[
            "generate",
            "-l",
            "cpp",
            "-o",
            out_dir.path().to_str().expect("a path"),
        ],
        &document,
    );
    assert!(ok, "the machine generates:\n{out}");
    assert!(
        out.contains("\"needs_script_engine\":false"),
        "a sce-static machine's params are read from its fields:\n{out}"
    );
    let generated: Vec<_> = std::fs::read_dir(out_dir.path())
        .expect("the output directory")
        .map(|entry| entry.expect("an entry").path())
        // The C++ generator writes a machine's code to the `.inl` its header
        // includes, so that is the file a send's code is in.
        .filter(|path| path.extension().is_some_and(|e| e == "inl"))
        .collect();
    assert_eq!(generated.len(), 1, "one machine: {generated:?}");
    let source = std::fs::read_to_string(&generated[0]).expect("a generated machine");
    assert!(
        source.contains("typedParams[\"k\"].push_back(std::move(paramValue))"),
        "the pair is built from the lowered value"
    );
    assert!(
        !source.contains("scriptEngine.evaluateExpression("),
        "no script engine reads a send param"
    );
}

#[test]
fn a_basic_http_send_carrying_a_param_is_refused_by_name_in_cpp() {
    // A BasicHTTP send carries each value as the text a form does, which the C++
    // machine does not spell from a typed value; it is named, not left to
    // generate a request with no body. The refusal is decided by the construct,
    // not by where the request would go, so the target names no endpoint of the
    // suite (the fixture port is spelled once, in `basic_http_test_endpoint.h`).
    let document = machine(
        r#"<state id="s">
    <transition event="go" type="internal">
      <send type="http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor" target="http://example.invalid/hook" event="note"><param name="k" expr="count"/></send>
    </transition>
  </state>"#,
    );
    let (ok, out) = run(&["check", "-l", "cpp"], &document);
    assert!(!ok, "C++ has no lowering for it yet:\n{out}");
    assert!(
        out.contains("generate/unsupported-feature")
            && out.contains("a BasicHTTP <send> carrying a <param>"),
        "expected the unsupported-feature refusal naming the construct:\n{out}"
    );
}

#[test]
fn a_python_host_action_argument_that_can_fail_is_received_where_the_call_stands() {
    // Python refused a `<sce:action>` by name. A failing argument is an
    // exception, so the call stands in a `try`: the exception leaves it before the
    // host is called and `error.execution` is raised in its place, without ending
    // the block (as on the other backends). A call that reads the event's payload
    // is more than one line, and sits in a block that checks the delivery
    // carried one — the shape no scenario runs, so what is read here is the
    // machine, and Python itself reads it as source.
    let (ok, out_dir) = {
        let out_dir = tempdir().expect("tempdir");
        let (ok, out) = run_record(
            &[
                "generate",
                "-l",
                "python",
                "-o",
                out_dir.path().to_str().expect("a path"),
            ],
            &record(
                EVERY_FIELD,
                r#"<state id="s">
    <transition event="day.picked" type="internal">
      <sce:action name="show"><sce:arg name="d" expr="_event.data.dayOfMonth + 1"/></sce:action>
    </transition>
  </state>"#,
            ),
        );
        assert!(ok, "the machine generates:\n{out}");
        (ok, out_dir)
    };
    assert!(ok);
    let generated: Vec<_> = std::fs::read_dir(out_dir.path())
        .expect("the output directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "py"))
        .collect();
    assert_eq!(generated.len(), 1, "one machine: {generated:?}");
    let source = std::fs::read_to_string(&generated[0]).expect("a generated machine");
    let lines: Vec<&str> = source.lines().map(str::trim).collect();
    let call = lines
        .iter()
        .position(|line| line.starts_with("self._actions.show("))
        .expect("the host action is called");
    assert_eq!(lines[call - 1], "try:", "the call stands in a try");
    assert!(
        lines[call + 1].starts_with("except sce_algorithm.AlgorithmFailure:"),
        "a failed argument is received where the call stands: {}",
        lines[call + 1]
    );
    assert!(
        lines[..call]
            .iter()
            .rev()
            .take(3)
            .any(|line| line.starts_with("if self._pending_day_picked_payload is not None:")),
        "the call reads the payload only for a delivery that carried one"
    );
    let parsed = Command::new("python3")
        .arg("-c")
        .arg("import ast, sys; ast.parse(open(sys.argv[1]).read())")
        .arg(&generated[0])
        .output()
        .expect("run python3");
    assert!(
        parsed.status.success(),
        "the generated machine is not Python:\n{}",
        String::from_utf8_lossy(&parsed.stderr)
    );
}

#[test]
fn a_send_content_is_the_text_it_spells_and_no_engine_reads_it() {
    // Measured 2026-10-03: Go, Python and C++ refused a literal `<content>` by
    // name, and Kotlin accepted it and generated
    // `evaluateSendContent(ScriptSource.lua(…))` — a call to the script engine in
    // a machine whose manifest says it needs none, so the data depended on an
    // engine nothing had given the machine. Rust already wrote the text out.
    //
    // Under a data model with no engine the text is the value (the rung Go and
    // Rust take when `needs_script_engine` is false): the string it spells,
    // whitespace-normalised and JSON-quoted. What is read is the generated
    // machine, since the manifest says `needs_script_engine:false` either way.
    let document = machine(
        r#"<state id="s">
    <transition event="go" type="internal">
      <send event="note"><content>two   words</content></send>
    </transition>
    <transition event="note" type="internal"/>
  </state>"#,
    );
    // The wire text, as each language spells the string literal carrying it.
    let wire = r#""\"two words\"""#;
    for (language, extension, native, engine) in [
        ("kotlin", "kt", wire, "evaluateSendContent("),
        ("rust", "rs", wire, "evaluate_expression"),
        ("go", "go", wire, "EvaluateExpression"),
        (
            "python",
            "py",
            "_data = \"two words\"",
            "_data = self._eval_send_payload(",
        ),
        // C++ hands the normalised text to the helper `<donedata>` takes with no
        // data model, which quotes it as the others do here.
        (
            "cpp",
            "inl",
            "emitContentLiteral(\"two words\"",
            "DoneDataHelper::evaluateContent(",
        ),
        // C copies the wire text into the event's data, with no call into the
        // Lua engine the machine was not given.
        ("c", "c", wire, "luaL_dostring("),
    ] {
        let out_dir = tempdir().expect("tempdir");
        let (ok, out) = run(
            &[
                "generate",
                "-l",
                language,
                "-o",
                out_dir.path().to_str().expect("a path"),
            ],
            &document,
        );
        assert!(ok, "{language}: the machine generates:\n{out}");
        assert!(
            out.contains("\"needs_script_engine\":false"),
            "{language}: a sce-static machine's content needs no engine:\n{out}"
        );
        let generated: Vec<_> = std::fs::read_dir(out_dir.path())
            .expect("the output directory")
            .map(|entry| entry.expect("an entry").path())
            .filter(|path| path.extension().is_some_and(|e| e == extension))
            .collect();
        assert_eq!(generated.len(), 1, "{language}: one machine: {generated:?}");
        let source = std::fs::read_to_string(&generated[0]).expect("a generated machine");
        assert!(
            source.contains(native),
            "{language}: the event's data is the text the content spells"
        );
        assert!(
            !source.contains(engine),
            "{language}: no script engine reads a send content"
        );
    }
}

#[test]
fn a_donedata_content_is_the_text_it_spells_and_no_engine_reads_it() {
    // Measured 2026-10-04: Rust, Go, Kotlin, Python and C++ accepted an inline
    // `<content>` in a final's `<donedata>` and generated a call to the script
    // engine for it, which made `needs_script_engine` true for a machine that was
    // to have none, and C refused it by name. A `<send>`'s literal content had
    // been finished at build time since 2026-10-03.
    //
    // Under a data model with no engine the text is the value: the string it
    // spells, whitespace-normalised and JSON-quoted, or the XML as written. What
    // is read is the generated machine, as for a `<send>`'s content, and the
    // scenario `static_donedata_content` runs it on every backend.
    let document = machine(
        r#"<state id="s">
    <transition event="go" target="fin"/>
  </state>
  <final id="fin"><donedata><content>two   words</content></donedata></final>"#,
    );
    for (language, extension, native, engine) in [
        (
            "kotlin",
            "kt",
            r##"doneEventData = "\"two words\"""##,
            "engineDD.evaluateExpr(",
        ),
        (
            "rust",
            "rs",
            r##"String::from("\"two words\"")"##,
            "evaluate_expression",
        ),
        (
            "go",
            "go",
            r##"doneEventData = "\"two words\"""##,
            "EvaluateExpression",
        ),
        (
            "python",
            "py",
            r##"_done_data = "\"two words\"""##,
            "_done_data = engine._script_engine.evaluate_expression(",
        ),
        (
            "cpp",
            "inl",
            r##"eventData = "\"two words\"";"##,
            "DoneDataHelper::evaluateContent(",
        ),
        (
            "c",
            "c",
            r##"static const char sce_lit_[] = "\"two words\"";"##,
            "luaL_dostring(",
        ),
    ] {
        let out_dir = tempdir().expect("tempdir");
        let (ok, out) = run(
            &[
                "generate",
                "-l",
                language,
                "-o",
                out_dir.path().to_str().expect("a path"),
            ],
            &document,
        );
        assert!(ok, "{language}: the machine generates:\n{out}");
        assert!(
            out.contains("\"needs_script_engine\":false"),
            "{language}: a sce-static machine's donedata content needs no engine:\n{out}"
        );
        let generated: Vec<_> = std::fs::read_dir(out_dir.path())
            .expect("the output directory")
            .map(|entry| entry.expect("an entry").path())
            .filter(|path| path.extension().is_some_and(|e| e == extension))
            .collect();
        assert_eq!(generated.len(), 1, "{language}: one machine: {generated:?}");
        let source = std::fs::read_to_string(&generated[0]).expect("a generated machine");
        assert!(
            source.contains(native),
            "{language}: the done event's data is the text the content spells"
        );
        assert!(
            !source.contains(engine),
            "{language}: no script engine reads a donedata content"
        );
    }
}

#[test]
fn a_host_run_invoke_content_expr_is_a_string_the_machine_computes() {
    // The `content` the host is handed — the body the service runs — is a
    // string, written out or joined from the machine's fields, read when the
    // invocation starts, as its `srcexpr` is.
    for expr in ["'select 1'", "'report ' + count"] {
        let (ok, out) = run(
            &["check"],
            &invoking_the_host(&format!(
                r#"<invoke type="x-sce-host" id="h"><content expr="{expr}"/></invoke>"#
            )),
        );
        assert!(ok, "`{expr}` is a string:\n{out}");
    }
}

#[test]
fn a_host_run_invoke_content_expr_that_is_no_string_is_refused_on_its_line() {
    // A body is text: a number is no body, and a name no variable declares
    // reads nothing.
    let (ok, out) = run(
        &["check"],
        &invoking_the_host(r#"<invoke type="x-sce-host" id="h"><content expr="count"/></invoke>"#),
    );
    assert!(!ok, "a number is no body:\n{out}");
    assert_refused_at(&out, "expression/type-mismatch", 9);
    let (ok, out) = run(
        &["check"],
        &invoking_the_host(
            r#"<invoke type="x-sce-host" id="h"><content expr="missing + 'x'"/></invoke>"#,
        ),
    );
    assert!(!ok, "a name no variable declares is refused:\n{out}");
    assert!(out.contains("missing"), "it names the name:\n{out}");
}

#[test]
fn a_host_run_invoke_content_expr_is_lowered_to_the_string_it_computes_and_needs_no_engine() {
    // Read from the machine's fields when the invocation starts, so no engine
    // evaluates it and the manifest says so, in every backend that runs one.
    let document = invoking_the_host_from_a_place(
        r#"<invoke type="x-sce-host" id="h"><content expr="place"/><param name="k" expr="count"/></invoke>"#,
    );
    // Each backend's assignment of the body from the variable, and the one it
    // makes when an engine evaluates the attribute, which this machine must not
    // carry: the helpers an engine's machines hold are in every generated
    // machine, so only that assignment tells the two apart.
    for (language, read, engine_call) in [
        (
            "rust",
            "host_invoke_content = self.place",
            "host_invoke_content = ::sce_rust_runtime",
        ),
        (
            "kotlin",
            "val hostInvokeContent: String = place",
            "val hostInvokeContent = try",
        ),
        (
            "go",
            "hostInvokeContent = p.vPlace",
            "hostInvokeContent = sce.ToWireString",
        ),
        (
            "python",
            "_host_content = self.v_place",
            "_host_content = engine._script_engine",
        ),
        (
            "c",
            "_host_inv_content_native = sm->policy.v_place",
            "char _host_inv_content[",
        ),
        (
            "cpp",
            "std::string computedContent = v_place",
            "hostInvoke.content = ::SCE::ScriptResultUtils",
        ),
    ] {
        let (out, code) = generated_code(language, &document);
        assert!(
            out.contains("\"needs_script_engine\":false"),
            "{language}: a body computed from the fields needs no engine:\n{out}"
        );
        let text: String = code.values().cloned().collect();
        assert!(
            text.contains(read),
            "{language}: the body the host is handed is read from the variable (`{read}`)"
        );
        assert!(
            !text.contains(engine_call),
            "{language}: no engine evaluates the body (`{engine_call}`)"
        );
    }
}

#[test]
fn a_host_run_invoke_content_expr_that_joins_text_is_written_for_c_into_a_buffer_the_model_sizes() {
    // The join of a literal and a `uint32` is a buffer of 7 + 10 + 1 bytes.
    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run(
        &[
            "generate",
            "-l",
            "c",
            "-o",
            out_dir.path().to_str().expect("a path"),
            "--host-invoker",
            "x-sce-host",
        ],
        &invoking_the_host(
            r#"<invoke type="x-sce-host" id="h"><content expr="'report ' + count"/></invoke>"#,
        ),
    );
    assert!(ok, "operands the model sizes have a C11 lowering:\n{out}");
    let source = generated_c(out_dir.path());
    assert!(
        source.contains(r#"SCE_FORGE_CONCAT((char[18]){0}, 18, sce_forge_wire_string("report ")"#),
        "the join is a buffer of 7 + 10 + 1 bytes:\n{source}"
    );
}

/// [`invoking_the_host`] over a machine that also holds the string `place`,
/// which a `srcexpr` can read.
fn invoking_the_host_from_a_place(invoke: &str) -> String {
    invoking_the_host(invoke).replace(
        r#"<data id="ready" sce:type="bool" expr="false"/>"#,
        r#"<data id="ready" sce:type="bool" expr="false"/>
    <data id="place" sce:type="string" sce:capacity="16" expr="'job://report'"/>"#,
    )
}

/// A `sce-static` machine whose first state holds `invoke` on line 9.
fn invoking_the_host(invoke: &str) -> String {
    machine(&format!(
        "<state id=\"s\">\n    {invoke}\n    <transition event=\"done.invoke.h\" target=\"done\"/>\n  </state>"
    ))
}

/// What `language` generates from `document` for a host that runs
/// `x-sce-host` invokes, by file name, and the run's own output. Left out is
/// what says where the document was read from and what is not code — its hash
/// and path, comments, includes and the source map — so two documents that mean
/// one machine compare equal and two that do not, do not.
fn generated_code(
    language: &str,
    document: &str,
) -> (String, std::collections::BTreeMap<String, String>) {
    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run(
        &[
            "generate",
            "-l",
            language,
            "-o",
            out_dir.path().to_str().expect("a path"),
            "--host-invoker",
            "x-sce-host",
        ],
        document,
    );
    assert!(ok, "{language}: the machine generates:\n{out}");
    let mut files = std::collections::BTreeMap::new();
    for entry in std::fs::read_dir(out_dir.path()).expect("the output directory") {
        let path = entry.expect("an entry").path();
        let name = path
            .file_name()
            .expect("a file name")
            .to_string_lossy()
            .into_owned();
        if name == "sce_sourcemap.json" {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("a generated file");
        let code: Vec<&str> = text
            .lines()
            .filter(|line| {
                let line = line.trim_start();
                !(line.contains("source-hash")
                    || line.starts_with("//")
                    || line.starts_with("# ")
                    || line.starts_with("#include"))
            })
            .collect();
        files.insert(name, code.join("\n"));
    }
    (out, files)
}

#[test]
fn a_host_run_invoke_namelist_names_variables_the_machine_holds() {
    let (ok, out) = run(
        &["check"],
        &invoking_the_host(r#"<invoke type="x-sce-host" id="h" namelist="count ready"/>"#),
    );
    assert!(ok, "a namelist of variables is typed values:\n{out}");
}

#[test]
fn a_host_run_invoke_namelist_is_the_params_it_abbreviates_and_needs_no_engine() {
    // The host is handed the pairs of a `namelist` beside the `<param>`s, so
    // `namelist="count ready"` is `<param name="count" expr="count"/>` and
    // `<param name="ready" expr="ready"/>` after them: every backend generates
    // the same code for the two spellings, and none of it asks an engine.
    let short = invoking_the_host(
        r#"<invoke type="x-sce-host" id="h" namelist="count ready"><param name="k" expr="count * 2"/></invoke>"#,
    );
    let written = invoking_the_host(
        r#"<invoke type="x-sce-host" id="h"><param name="k" expr="count * 2"/><param name="count" expr="count"/><param name="ready" expr="ready"/></invoke>"#,
    );
    for language in ["rust", "kotlin", "go", "python", "c", "cpp"] {
        let (out, from_short) = generated_code(language, &short);
        let (_, from_written) = generated_code(language, &written);
        assert!(
            out.contains("\"needs_script_engine\":false"),
            "{language}: a namelist of variables needs no script engine:\n{out}"
        );
        assert!(
            from_short.values().any(|text| text.contains("ready")),
            "{language}: the request carries the names: {from_short:?}"
        );
        assert_eq!(
            from_short, from_written,
            "{language}: a namelist and the params it abbreviates generate alike"
        );
    }
}

#[test]
fn a_host_run_invoke_namelist_name_is_held_to_the_rule_a_param_is() {
    // A name nothing declares reads no variable.
    let (ok, out) = run(
        &["check"],
        &invoking_the_host(r#"<invoke type="x-sce-host" id="h" namelist="count missing"/>"#),
    );
    assert!(!ok, "a name that is no variable is refused:\n{out}");
    assert!(out.contains("missing"), "it names the name:\n{out}");

    // A 64-bit variable has no wire spelling every backend shares, and the
    // refusal is placed at the invoke, where the name is written.
    let document = invoking_the_host(r#"<invoke type="x-sce-host" id="h" namelist="big"/>"#)
        .replace(
            r#"<data id="ready" sce:type="bool" expr="false"/>"#,
            r#"<data id="big" sce:type="int64" expr="0"/>"#,
        );
    let (ok, out) = run(&["check"], &document);
    assert!(!ok, "a 64-bit variable crosses with no spelling:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 9);
    assert!(out.contains("64-bit"), "it says why:\n{out}");
}

#[test]
fn a_host_run_invoke_namelist_name_that_repeats_a_param_is_refused_for_c() {
    // C11 collects the `<param>`s of one name into one array, as every engine
    // does, but where a `namelist` name stands among the `<param>`s that share it
    // is not a document order the engines were held to, so a `namelist` name that
    // a `<param>` of the same invoke already names is refused by name there. The
    // other backends carry it as they carry any name written twice.
    let document = invoking_the_host(
        r#"<invoke type="x-sce-host" id="h" namelist="count"><param name="count" expr="1"/></invoke>"#,
    );
    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run(
        &[
            "generate",
            "-l",
            "c",
            "-o",
            out_dir.path().to_str().expect("a path"),
            "--host-invoker",
            "x-sce-host",
        ],
        &document,
    );
    assert!(!ok, "a name carried twice has no C11 lowering:\n{out}");
    assert!(
        out.contains("whose namelist names `count`, which a <param> or the namelist already does"),
        "it says why:\n{out}"
    );
}

#[test]
fn a_host_run_invoke_srcexpr_is_a_string_the_machine_computes() {
    // The `src` the host is handed is a string, written out or joined from the
    // machine's fields, read when the invocation starts.
    for expr in ["'job://report'", "'job://' + count"] {
        let (ok, out) = run(
            &["check"],
            &invoking_the_host(&format!(
                r#"<invoke type="x-sce-host" id="h" srcexpr="{expr}"/>"#
            )),
        );
        assert!(ok, "`{expr}` is a string:\n{out}");
    }
}

#[test]
fn a_host_run_invoke_srcexpr_that_is_no_string_or_is_doubled_is_refused_on_its_line() {
    for (what, invoke, code) in [
        (
            "a number, which is no source",
            r#"<invoke type="x-sce-host" id="h" srcexpr="count"/>"#,
            "expression/type-mismatch",
        ),
        (
            "a src beside it",
            r#"<invoke type="x-sce-host" id="h" src="a" srcexpr="'b'"/>"#,
            "scxml/static-datamodel-rule",
        ),
    ] {
        let (ok, out) = run(&["check"], &invoking_the_host(invoke));
        assert!(!ok, "{what}: no source is named by it:\n{out}");
        assert_refused_at(&out, code, 9);
    }
    let (ok, out) = run(
        &["check"],
        &invoking_the_host(r#"<invoke type="x-sce-host" id="h" srcexpr="missing + 'x'"/>"#),
    );
    assert!(!ok, "a name no variable declares is refused:\n{out}");
    assert!(out.contains("missing"), "it names the name:\n{out}");
}

#[test]
fn a_host_run_invoke_srcexpr_is_lowered_to_the_string_it_computes_and_needs_no_engine() {
    // Read from the machine's fields when the invocation starts, so no engine
    // evaluates it and the manifest says so, in every backend that runs one.
    let document = invoking_the_host_from_a_place(
        r#"<invoke type="x-sce-host" id="h" srcexpr="place"><param name="k" expr="count"/></invoke>"#,
    );
    // Each backend's assignment of the source from the variable, and the
    // assignment it makes of the source when an engine evaluates the attribute,
    // which this machine must not carry: the helpers an engine's machines hold
    // are in every generated machine, so only that assignment tells the two apart.
    for (language, read, engine_call) in [
        (
            "rust",
            "host_invoke_src = self.place",
            "host_invoke_src = ::sce_rust_runtime",
        ),
        (
            "kotlin",
            "val hostInvokeSrc: String = place",
            "val hostInvokeSrc = try",
        ),
        (
            "go",
            "hostInvokeSrc = p.vPlace",
            "hostInvokeSrc = sce.ToWireString",
        ),
        (
            "python",
            "_host_src = self.v_place",
            "_host_src = engine._script_engine",
        ),
        (
            "c",
            "_host_inv_src_native = sm->policy.v_place",
            "char _host_inv_src[",
        ),
        (
            "cpp",
            "std::string computedSrc = v_place",
            "hostInvoke.src = ::SCE::ScriptResultUtils",
        ),
    ] {
        let (out, code) = generated_code(language, &document);
        assert!(
            out.contains("\"needs_script_engine\":false"),
            "{language}: a source computed from the fields needs no engine:\n{out}"
        );
        let text: String = code.values().cloned().collect();
        assert!(
            text.contains(read),
            "{language}: the source the host is handed is read from the variable (`{read}`)"
        );
        assert!(
            !text.contains(engine_call),
            "{language}: no engine evaluates the source (`{engine_call}`)"
        );
    }
}

#[test]
fn a_host_run_invoke_srcexpr_that_joins_text_is_written_for_c_into_a_buffer_the_model_sizes() {
    // The join of a literal and a `uint32` is a buffer of 6 + 10 + 1 bytes.
    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run(
        &[
            "generate",
            "-l",
            "c",
            "-o",
            out_dir.path().to_str().expect("a path"),
            "--host-invoker",
            "x-sce-host",
        ],
        &invoking_the_host(r#"<invoke type="x-sce-host" id="h" srcexpr="'job://' + count"/>"#),
    );
    assert!(ok, "operands the model sizes have a C11 lowering:\n{out}");
    let source = generated_c(out_dir.path());
    assert!(
        source.contains(r#"SCE_FORGE_CONCAT((char[17]){0}, 17, sce_forge_wire_string("job://")"#),
        "the join is a buffer of 6 + 10 + 1 bytes:\n{source}"
    );
}

#[test]
fn a_static_machine_that_sends_and_invokes_the_host_needs_no_script_engine() {
    // The manifest says which backends need an engine to run the machine. The
    // `<param>`s are lowered to native code, so this one needs none; the same
    // document under `ecmascript` evaluates them in an engine and still does.
    let document = machine(
        r#"<state id="s">
    <onentry><send type="x-sce-host" event="notify"><param name="k" expr="count"/></send></onentry>
    <invoke type="x-sce-host" id="h"><param name="k" expr="count"/></invoke>
    <transition event="done.invoke.h" target="done"/>
  </state>"#,
    );
    let generate = |language: &str, document: &str| {
        let out_dir = tempdir().expect("tempdir");
        run(
            &[
                "generate",
                "-l",
                language,
                "-o",
                out_dir.path().to_str().expect("a path"),
                "--host-processor",
                "x-sce-host",
                "--host-invoker",
                "x-sce-host",
            ],
            document,
        )
    };
    for language in ["rust", "kotlin"] {
        let (ok, out) = generate(language, &document);
        assert!(ok, "{language}: the machine generates:\n{out}");
        assert!(
            out.contains("\"needs_script_engine\":false"),
            "{language}: a sce-static machine's params are read from its fields:\n{out}"
        );
    }
    let ecmascript = document
        .replace(r#"datamodel="sce-static""#, r#"datamodel="ecmascript""#)
        .replace(r#" sce:type="uint32""#, "")
        .replace(r#" sce:type="bool""#, "");
    let (ok, out) = generate("kotlin", &ecmascript);
    assert!(ok, "the same document under ecmascript generates:\n{out}");
    assert!(
        out.contains("\"needs_script_engine\":true"),
        "under ecmascript a param is evaluated by an engine:\n{out}"
    );
}

// ── A variable is published by sce:direction="out" ──────────────────────

#[test]
fn a_variable_the_host_would_write_is_refused() {
    // `out` publishes a variable and `internal` keeps it the machine's own;
    // `in` would hand the host a write nothing lowers.
    let (ok, out) = run(
        &["check"],
        &doc(
            "sce-static",
            r#"<data id="count" sce:type="uint32" expr="0" sce:direction="in"/>"#,
        ),
    );
    assert!(
        !ok,
        "a sce-static variable is written by the machine:\n{out}"
    );
    assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
}

#[test]
fn a_published_and_an_internal_variable_are_accepted() {
    let (ok, out) = run(
        &["check", "-l", "kotlin"],
        &doc(
            "sce-static",
            r#"<data id="count" sce:type="uint32" expr="0" sce:direction="out"/>
    <data id="step" sce:type="uint32" expr="1" sce:direction="internal"/>"#,
        ),
    );
    assert!(ok, "out and internal both mean something here:\n{out}");
}

// ── A list variable starts empty, is appended to and cleared ────────────

/// A list variable, `days: list<uint8>` of capacity 4, on line 5.
const DAYS: &str = r#"<data id="days" sce:type="list&lt;uint8&gt;" sce:capacity="4"/>"#;

/// A machine under `datamodel` whose first variable is `data` (line 5),
/// followed by `ready: bool` (line 6), and whose `states` open on line 8.
fn list_doc(datamodel: &str, data: &str, states: &str) -> String {
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="{datamodel}">
  <datamodel>
    {data}
    <data id="ready" sce:type="bool" expr="false"/>
  </datamodel>
  {states}
</scxml>
"##
    )
}

#[test]
fn a_list_is_appended_to_and_cleared() {
    let (ok, out) = run(
        &["check", "-l", "kotlin"],
        &list_doc(
            "sce-static",
            DAYS,
            r#"<state id="s"><onentry><sce:append target="days" expr="7"/><sce:clear target="days"/></onentry></state>"#,
        ),
    );
    assert!(
        ok,
        "a list filled and emptied by its two statements:\n{out}"
    );
}

#[test]
fn a_list_without_a_capacity_is_refused() {
    let (ok, out) = run(
        &["check"],
        &list_doc(
            "sce-static",
            r#"<data id="days" sce:type="list&lt;uint8&gt;"/>"#,
            r#"<state id="s"/>"#,
        ),
    );
    assert!(!ok, "a list declares its bound:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
}

#[test]
fn a_list_capacity_that_is_not_a_positive_count_is_refused_by_its_rule() {
    // `sce:capacity` is untyped in the schema — the event queue's bound on
    // `<scxml>` shares the name — so this rule is the list's only guard.
    for written in ["0", "many"] {
        let (ok, out) = run(
            &["check"],
            &list_doc(
                "sce-static",
                &format!(
                    r#"<data id="days" sce:type="list&lt;uint8&gt;" sce:capacity="{written}"/>"#
                ),
                r#"<state id="s"/>"#,
            ),
        );
        assert!(!ok, "sce:capacity=\"{written}\" is no count:\n{out}");
        assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
    }
}

#[test]
fn a_list_with_an_initial_value_is_refused() {
    let (ok, out) = run(
        &["check"],
        &list_doc(
            "sce-static",
            r#"<data id="days" sce:type="list&lt;uint8&gt;" sce:capacity="4" expr="0"/>"#,
            r#"<state id="s"/>"#,
        ),
    );
    assert!(!ok, "a list starts empty:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
}

#[test]
fn a_capacity_on_a_variable_that_is_not_a_list_is_refused() {
    let (ok, out) = run(
        &["check"],
        &list_doc(
            "sce-static",
            r#"<data id="days" sce:type="uint8" sce:capacity="4" expr="0"/>"#,
            r#"<state id="s"/>"#,
        ),
    );
    assert!(!ok, "only a list or a string has a capacity:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
}

// ── A string is bounded as a list is ─────────────────────────────────────

#[test]
fn a_string_without_a_capacity_is_refused() {
    let (ok, out) = run(
        &["check"],
        &list_doc(
            "sce-static",
            r#"<data id="label" sce:type="string" expr="'ab'"/>"#,
            r#"<state id="s"/>"#,
        ),
    );
    assert!(!ok, "a string declares its bound:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
    assert!(
        out.contains("UTF-8 bytes"),
        "the refusal says what the bound counts:\n{out}"
    );
}

#[test]
fn a_string_capacity_that_is_not_a_positive_count_is_refused() {
    for written in ["0", "many", "-1"] {
        let (ok, out) = run(
            &["check"],
            &list_doc(
                "sce-static",
                &format!(
                    r#"<data id="label" sce:type="string" sce:capacity="{written}" expr="''"/>"#
                ),
                r#"<state id="s"/>"#,
            ),
        );
        assert!(!ok, "sce:capacity=\"{written}\" is no count:\n{out}");
        assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
    }
}

#[test]
fn a_string_that_fits_its_capacity_in_bytes_is_accepted() {
    // `é` is two bytes and `€` three: the bound counts bytes, so two characters
    // of two bytes fit four, and a literal as long as the bound fits it.
    for (written, capacity) in [("abcd", 4), ("\u{e9}\u{e9}", 4), ("", 1)] {
        let (ok, out) = run(
            &["check"],
            &list_doc(
                "sce-static",
                &format!(
                    r#"<data id="label" sce:type="string" sce:capacity="{capacity}" expr="'{written}'"/>"#
                ),
                r#"<state id="s"/>"#,
            ),
        );
        assert!(ok, "'{written}' fits {capacity} bytes:\n{out}");
    }
}

#[test]
fn a_string_that_starts_past_its_capacity_is_refused_in_bytes() {
    // Two characters, five bytes: a count of characters would accept it.
    for (written, capacity) in [("abcde", 4), ("\u{e9}\u{20ac}", 4)] {
        let (ok, out) = run(
            &["check"],
            &list_doc(
                "sce-static",
                &format!(
                    r#"<data id="label" sce:type="string" sce:capacity="{capacity}" expr="'{written}'"/>"#
                ),
                r#"<state id="s"/>"#,
            ),
        );
        assert!(!ok, "'{written}' is past {capacity} bytes:\n{out}");
        assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
        assert!(
            out.contains("past the sce:capacity"),
            "the refusal names the bound:\n{out}"
        );
    }
}

#[test]
fn a_string_starts_at_a_literal_and_not_at_another_variable() {
    // The machine is built with no error to raise, so a value that could fail
    // to fit is refused where it is written rather than copied at run time.
    let (ok, out) = run(
        &["check"],
        &list_doc(
            "sce-static",
            r#"<data id="first" sce:type="string" sce:capacity="8" expr="'ab'"/>
    <data id="second" sce:type="string" sce:capacity="4" expr="first"/>"#,
            r#"<state id="s"/>"#,
        ),
    );
    assert!(!ok, "a string starts at a literal:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 6);
}

#[test]
fn a_list_of_an_element_with_a_length_is_refused() {
    let (ok, out) = run(
        &["check"],
        &list_doc(
            "sce-static",
            r#"<data id="days" sce:type="list&lt;string&gt;" sce:capacity="4"/>"#,
            r#"<state id="s"/>"#,
        ),
    );
    assert!(!ok, "a list element has a fixed width:\n{out}");
    assert_refused_at(&out, "validation/attribute-rule-violated", 5);
}

#[test]
fn an_append_to_a_variable_that_is_not_a_list_is_refused_at_its_target() {
    let (ok, out) = run(
        &["check"],
        &list_doc(
            "sce-static",
            DAYS,
            r#"<state id="s"><onentry><sce:append target="ready" expr="1"/></onentry></state>"#,
        ),
    );
    assert!(!ok, "`ready` is a bool:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 8);
    assert!(
        out.contains("one of days"),
        "the refusal names the lists:\n{out}"
    );
}

#[test]
fn an_element_of_another_kind_is_refused() {
    let (ok, out) = run(
        &["check"],
        &list_doc(
            "sce-static",
            DAYS,
            r#"<state id="s"><onentry><sce:append target="days" expr="true"/></onentry></state>"#,
        ),
    );
    assert!(!ok, "a bool is not a uint8 element:\n{out}");
    assert_refused_at(&out, "expression/type-mismatch", 8);
}

#[test]
fn a_list_read_as_a_value_is_refused() {
    let (ok, out) = run(
        &["check"],
        &list_doc(
            "sce-static",
            DAYS,
            r#"<state id="s"><transition event="go" cond="days" target="s"/></state>"#,
        ),
    );
    assert!(
        !ok,
        "a list is read by the host, not by an expression:\n{out}"
    );
    assert_refused_at(&out, "expression/unsupported-construct", 8);
}

#[test]
fn a_list_is_measured_by_len() {
    let (ok, out) = run(
        &["check", "-l", "kotlin"],
        &list_doc(
            "sce-static",
            &format!("{DAYS}\n    <data id=\"n\" sce:type=\"uint32\" expr=\"0\"/>"),
            r#"<state id="s">
    <onentry><assign location="n" expr="len(days)"/></onentry>
    <transition event="go" cond="len(days) &gt; 2" target="s"/>
  </state>"#,
        ),
    );
    assert!(
        ok,
        "len(days) reads a list's length, in a value and in a guard:\n{out}"
    );
}

#[test]
fn a_list_passed_to_a_host_action_is_refused() {
    // A host method takes values, and a list is not one — refused with the
    // same wording as in any other expression, not as an undeclared name.
    let (ok, out) = run(
        &["check"],
        &list_doc(
            "sce-static",
            DAYS,
            r#"<state id="s"><onentry><sce:action name="show"><sce:arg name="d" expr="days"/></sce:action></onentry></state>"#,
        ),
    );
    assert!(!ok, "a list is no host-method argument:\n{out}");
    assert!(
        out.contains("expression/unsupported-construct") && out.contains("reading the list `days`"),
        "{out}"
    );
}

#[test]
fn an_assignment_to_a_whole_list_is_refused() {
    let (ok, out) = run(
        &["check"],
        &list_doc(
            "sce-static",
            DAYS,
            r#"<state id="s"><onentry><assign location="days" expr="0"/></onentry></state>"#,
        ),
    );
    assert!(
        !ok,
        "a list is filled by append and emptied by clear:\n{out}"
    );
    assert_refused_at(&out, "expression/unsupported-construct", 8);
}

#[test]
fn a_list_statement_under_another_data_model_is_refused() {
    // Before, the parser skipped an `sce:` element it did not know, so this
    // statement was dropped without a word.
    let (ok, out) = run(
        &["check"],
        &list_doc(
            "ecmascript",
            r#"<data id="n" expr="0"/>"#,
            r#"<state id="s"><onentry><sce:append target="n" expr="1"/></onentry></state>"#,
        ),
    );
    assert!(!ok, "a list exists only under sce-static:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 8);
}

// ── An imported algorithm is called as `Alias(args)` ────────────────────

/// A scalar algorithm: `clamp(n, top)`.
const ALGORITHM_CLAMP: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="clamp" version="1.0">
  <sce:signature>
    <sce:param name="n" type="uint32"/>
    <sce:param name="top" type="uint32"/>
    <sce:return type="uint32"/>
  </sce:signature>
  <sce:body>
    <sce:var name="r" type="uint32" init="n"/>
    <sce:if cond="r &gt; top"><sce:assign target="r" expr="top"/></sce:if>
    <sce:return expr="r"/>
  </sce:body>
</scxml>
"#;

/// An algorithm returning a list — callable only by a host.
const ALGORITHM_LIST: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="upto" version="1.0">
  <sce:signature>
    <sce:param name="n" type="uint32"/>
    <sce:return type="list&lt;uint32&gt;" returns-max-size="4"/>
  </sce:signature>
  <sce:body>
    <sce:var name="out" type="list&lt;uint32&gt;" capacity="4"/>
    <sce:append target="out" expr="n"/>
    <sce:return expr="out"/>
  </sce:body>
</scxml>
"#;

/// A machine under `datamodel` importing `algorithm_clamp.scxml` as `Clamp`
/// on line 4 (and `algorithm_upto.scxml` as `Upto` on line 5), with
/// `count: uint32` and `states` from line 10.
fn calling_doc(datamodel: &str, states: &str) -> String {
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="{datamodel}">
  <sce:import kind="algorithm" src="algorithm_clamp.scxml" as="Clamp"/>
  <sce:import kind="algorithm" src="algorithm_upto.scxml" as="Upto"/>
  <datamodel>
    <data id="count" sce:type="uint32" expr="0"/>
  </datamodel>

  {states}
</scxml>
"##
    )
}

fn run_calling(args: &[&str], doc: &str) -> (bool, String) {
    run_beside(
        args,
        doc,
        &[
            ("algorithm_clamp.scxml", ALGORITHM_CLAMP),
            ("algorithm_upto.scxml", ALGORITHM_LIST),
        ],
    )
}

#[test]
fn an_imported_algorithm_is_called_in_a_guard_and_an_assignment() {
    let (ok, out) = run_calling(
        &["check", "-l", "kotlin"],
        &calling_doc(
            "sce-static",
            r#"<state id="s">
    <transition event="tick" cond="Clamp(count + 1, 5) &gt; count" type="internal">
      <assign location="count" expr="Clamp(count + 1, 5)"/>
    </transition>
    <transition event="list" type="internal"><sce:action name="show"><sce:arg name="n" expr="Clamp(count, 3)"/></sce:action></transition>
  </state>"#,
        )
        // Upto is imported to be refused below; here it is not in the file.
        .replace(
            r#"  <sce:import kind="algorithm" src="algorithm_upto.scxml" as="Upto"/>
"#,
            "",
        ),
    );
    assert!(ok, "a scalar algorithm is called like a function:\n{out}");
}

#[test]
fn an_imported_algorithm_nothing_calls_is_refused_at_its_import() {
    let (ok, out) = run_calling(
        &["check"],
        &calling_doc(
            "sce-static",
            r#"<state id="s"><transition event="tick" type="internal"><assign location="count" expr="Clamp(count, 5)"/></transition></state>"#,
        ),
    );
    assert!(!ok, "`Upto` is imported and never called:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 5);
    assert!(out.contains("Upto"), "the refusal names the import:\n{out}");
}

#[test]
fn a_list_returning_algorithm_is_refused_where_it_is_called() {
    let (ok, out) = run_calling(
        &["check"],
        &calling_doc(
            "sce-static",
            r#"<state id="s"><transition event="tick" type="internal">
      <assign location="count" expr="Clamp(count, 5)"/>
      <log label="l" expr="Upto(count)"/>
    </transition></state>"#,
        ),
    );
    assert!(
        !ok,
        "only a host calls an algorithm returning a list:\n{out}"
    );
    assert!(
        out.contains("list<uint32>"),
        "the refusal names the slot:\n{out}"
    );
}

/// A `sce-static` machine receives the failures of what it runs (SCE
/// Accepted Subset §2.15, SCE_FORGE.md §3.4.1): a `may-fail` algorithm is
/// called, and its failure becomes `error.execution` in place of the
/// statement — on every backend that lowers the model.
///
/// ⚠ This test used to hold the opposite, that such a call is refused: the
/// machine had no failure channel, so the call could only be lowered as if
/// it returned a value. E12 D5 gave it one.
#[test]
fn a_may_fail_algorithm_is_received_where_a_static_machine_calls_it() {
    let may_fail_clamp = ALGORITHM_CLAMP.replace(
        r#"<sce:return type="uint32"/>"#,
        r#"<sce:return type="uint32" may-fail="true"/>"#,
    );
    let document = calling_doc(
        "sce-static",
        r#"<state id="s"><transition event="tick" type="internal"><assign location="count" expr="Clamp(count, 5)"/></transition></state>"#,
    )
    .replace(
        r#"  <sce:import kind="algorithm" src="algorithm_upto.scxml" as="Upto"/>
"#,
        "",
    );
    for lang in ["kotlin", "rust", "python"] {
        let (ok, out) = run_beside(
            &["check", "-l", lang],
            &document,
            &[("algorithm_clamp.scxml", may_fail_clamp.as_str())],
        );
        assert!(
            ok,
            "--lang {lang}: the machine receives Clamp's failure:\n{out}"
        );
    }
}

/// A machine calling `Clamp`, importing it alone: the document the tests below
/// generate for the backends whose imports are written by the machine.
fn calling_clamp_alone() -> String {
    calling_doc(
        "sce-static",
        r#"<state id="s"><transition event="tick" type="internal"><assign location="count" expr="Clamp(count, 5)"/></transition></state>"#,
    )
    .replace(
        r#"  <sce:import kind="algorithm" src="algorithm_upto.scxml" as="Upto"/>
"#,
        "",
    )
}

#[test]
fn a_go_machine_calls_the_package_its_algorithms_generation_put_it_in() {
    // The call is the package's name and the algorithm's exported symbol, and
    // the import is the module path the packages live under: the identity a forge
    // kind importing the same algorithm derives, so the two agree on where it is.
    // A prefix's trailing slash is the same prefix.
    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run_beside(
        &[
            "generate",
            "-l",
            "go",
            "-o",
            out_dir.path().to_str().expect("a path"),
            "--go-module-prefix",
            "github.com/acme/gen/",
        ],
        &calling_clamp_alone(),
        &[("algorithm_clamp.scxml", ALGORITHM_CLAMP)],
    );
    assert!(ok, "the machine generates:\n{out}");
    let generated: Vec<_> = std::fs::read_dir(out_dir.path())
        .expect("the output directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "go"))
        .collect();
    assert_eq!(generated.len(), 1, "one machine: {generated:?}");
    let source = std::fs::read_to_string(&generated[0]).expect("a generated machine");
    // The package is imported under the alias a forge kind importing the same
    // algorithm uses (a name no author's local can be), and the call carries it.
    assert!(
        source.contains("\tsce_clamp \"github.com/acme/gen/clamp\"\n"),
        "the machine imports the algorithm's package by the module path"
    );
    assert!(
        source.contains("sce_clamp.Clamp(p.vCount, 5)"),
        "the call names the package's alias and the exported function"
    );
}

#[test]
fn a_python_machine_calls_the_module_its_algorithms_generation_put_it_in() {
    // The call is the module's name and the algorithm's function, and the import
    // is the sibling-module one a forge kind importing the same algorithm
    // writes: the two agree on where the algorithm is.
    let out_dir = tempdir().expect("tempdir");
    let (ok, out) = run_beside(
        &[
            "generate",
            "-l",
            "python",
            "-o",
            out_dir.path().to_str().expect("a path"),
        ],
        &calling_clamp_alone(),
        &[("algorithm_clamp.scxml", ALGORITHM_CLAMP)],
    );
    assert!(ok, "the machine generates:\n{out}");
    let generated: Vec<_> = std::fs::read_dir(out_dir.path())
        .expect("the output directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "py"))
        .collect();
    assert_eq!(generated.len(), 1, "one machine: {generated:?}");
    let source = std::fs::read_to_string(&generated[0]).expect("a generated machine");
    // The module is imported under the alias a forge kind importing the same
    // algorithm uses (a name no author's local can be), and the call carries it.
    assert!(
        source.contains("\nfrom . import clamp as sce_clamp\n"),
        "the machine imports the algorithm's module as a sibling"
    );
    assert!(
        source.contains("sce_clamp.clamp(self.v_count, 5)"),
        "the call names the module's alias and the function"
    );
}

#[test]
fn a_go_machine_calling_an_algorithm_needs_the_module_its_packages_live_under() {
    // A Go import path has no valid bare form, so without the module path there
    // is nothing to write the import from. It is a configuration error naming the
    // flag, not a construct the backend lacks: the same refusal a forge kind
    // importing another gives.
    let (ok, out) = run_beside(
        &["check", "-l", "go"],
        &calling_clamp_alone(),
        &[("algorithm_clamp.scxml", ALGORITHM_CLAMP)],
    );
    assert!(!ok, "no module path was given:\n{out}");
    assert!(
        out.contains("generate/invalid-config") && out.contains("go_module_prefix"),
        "the refusal names what is missing:\n{out}"
    );
    let (ok, out) = run_beside(
        &[
            "check",
            "-l",
            "go",
            "--go-module-prefix",
            "github.com/acme/gen",
        ],
        &calling_clamp_alone(),
        &[("algorithm_clamp.scxml", ALGORITHM_CLAMP)],
    );
    assert!(ok, "given the module path, the machine lowers:\n{out}");
}

#[test]
fn an_algorithm_import_whose_file_is_missing_is_refused_at_its_import() {
    let (ok, out) = run_beside(
        &["check"],
        &calling_doc(
            "sce-static",
            r#"<state id="s"><transition event="tick" type="internal"><assign location="count" expr="Clamp(count, 5)"/></transition></state>"#,
        ),
        &[("algorithm_upto.scxml", ALGORITHM_LIST)],
    );
    assert!(
        !ok,
        "algorithm_clamp.scxml is not beside the document:\n{out}"
    );
    assert_refused_at(&out, "import/file-not-found", 4);
    assert!(out.contains("algorithm_clamp.scxml"), "{out}");
}

#[test]
fn an_algorithm_import_under_another_data_model_is_refused() {
    let (ok, out) = run_calling(
        &["check"],
        &calling_doc("ecmascript", r#"<state id="s"/>"#)
            .replace(r#"sce:type="uint32" expr="0""#, r#"expr="0""#),
    );
    assert!(!ok, "only sce-static lowers an algorithm call:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 4);
}

// ── Every scenario is replayed on every backend that lowers the model ────

/// The driver that replays scenarios on a backend, and the text it names a
/// scenario by (`{s}`, the scenario file's own name — a machine can have
/// several). A backend that lowers sce-static and is not listed here fails
/// the test below until it has one.
const SCENARIO_DRIVERS: &[(&str, &str, &str)] = &[
    (
        "rust",
        "backends/rust/tests/tests/static_scenarios.rs",
        "static_datamodel/scenarios/{s}.json",
    ),
    (
        "kotlin",
        "backends/kotlin/tests/src/test/kotlin/com/sce/integration/StaticScenarioTest.kt",
        "scenario(\"{s}\")",
    ),
    (
        "cpp",
        "tests/integration/AStaticDatamodelRunsGeneratedCppTest.cpp",
        "replay(\"{s}\"",
    ),
    (
        "go",
        "backends/go/tests/integration/static_datamodel/static_scenarios_test.go",
        "replay(t, \"{s}\"",
    ),
    (
        "python",
        "backends/python/tests/integration/static_datamodel/test_static_scenarios.py",
        "replay(\"{s}\")",
    ),
    // Built by CMake from the fixtures, so there is no machine to commit.
    (
        "c11",
        "backends/c/tests/integration/test_static_scalars.c",
        "_scenario(\"{s}\"",
    ),
];

/// Where each backend's regen script commits a machine: the file that exists
/// only if it did.
const COMMITTED_MACHINES: &[(&str, &str)] = &[
    (
        "rust",
        "backends/rust/tests/src/integration/static_datamodel/{m}_sm.rs",
    ),
    (
        "kotlin",
        "backends/kotlin/tests/src/main/kotlin/com/sce/integration/{m}/{m}Sm.kt",
    ),
    (
        "go",
        "backends/go/tests/integration/static_datamodel/{m}/{m}_sm.go",
    ),
];

/// The Go mutation casefile declares the machines `scripts/regen_static_datamodel_go.sh`
/// commits as targets, because a mutated template reaches the tests only through
/// them, and it cannot derive that list. A machine the script commits and the
/// casefile does not name is one the round would regenerate and never restore.
///
/// A directory the script commits holds a machine (`<m>_sm.go`, with one
/// `<m>__sce_synth_invoke__<id>_sm.go` beside it for each child an `<invoke>`
/// declares in place, and one `<stem>_sm.go` for each document a hybrid
/// `<invoke>` declares in `sce:candidates`) or an imported algorithm's package
/// (`<m>.go`). Every
/// `*_sm.go` is a target, the children included: a child is generated from the
/// same templates as its parent, and one the casefile left out is one the round
/// would regenerate and never restore. An algorithm's package is not: its
/// generation reads neither a statechart template nor a lowering spelling, so it
/// comes back from the regeneration as it was.
#[test]
fn the_go_mutation_casefile_names_every_committed_go_machine() {
    let root = repo_root();
    let casefile = std::fs::read_to_string(root.join(
        "sce-build/tests/mutations/a_static_go_machine_ends_its_block_at_a_failure_go.cases",
    ))
    .expect("the Go casefile");
    let committed = root.join("backends/go/tests/integration/static_datamodel");
    let mut directories: Vec<String> = std::fs::read_dir(&committed)
        .expect("the committed Go machines")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.is_dir())
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    directories.sort();
    assert!(!directories.is_empty(), "no committed Go machine to judge");
    let mut machines = 0;
    for directory in directories {
        let mut generated: Vec<String> = std::fs::read_dir(committed.join(&directory))
            .expect("a committed Go directory")
            .map(|entry| {
                entry
                    .expect("an entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .filter(|name| name.ends_with("_sm.go"))
            .collect();
        generated.sort();
        if generated.is_empty() {
            let algorithm_file = committed.join(&directory).join(format!("{directory}.go"));
            assert!(
                algorithm_file.exists(),
                "{directory} holds neither a machine nor an algorithm's package"
            );
            continue;
        }
        assert!(
            generated.contains(&format!("{directory}_sm.go")),
            "{directory} holds machine files {generated:?} and not the machine it is named for"
        );
        for file in generated {
            machines += 1;
            let target =
                format!("backends/go/tests/integration/static_datamodel/{directory}/{file}");
            assert!(
                casefile.contains(&target),
                "the Go casefile does not declare {target} as a mutation target"
            );
        }
    }
    assert!(machines > 0, "no committed Go machine to judge");
}

/// A scenario (`fixtures/static_datamodel/scenarios/<name>.json`) is the
/// behaviour of a sce-static machine, stated once and replayed by every
/// backend that lowers the model. Which backends those are is ASKED of the
/// generator — each scenario's machine is checked in every language — not
/// written here, so a backend that starts lowering the model is held to
/// every scenario at once. The regen scripts derive their machines from the
/// fixture directory, so what is held is that they do, and that the machine
/// is committed where each driver builds it from — or the driver would replay
/// a machine nobody regenerates.
#[test]
fn every_scenario_is_replayed_on_every_backend_that_lowers_its_machine() {
    let root = repo_root();
    let fixtures = root.join("sce-build/tests/fixtures/static_datamodel");
    let mut scenarios: Vec<PathBuf> = std::fs::read_dir(fixtures.join("scenarios"))
        .expect("the scenarios directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "json"))
        .collect();
    scenarios.sort();
    assert!(!scenarios.is_empty(), "no scenario to judge");
    let regen: Vec<String> = ["rust", "kotlin", "go", "python"]
        .iter()
        .map(|lang| {
            std::fs::read_to_string(root.join(format!("scripts/regen_static_datamodel_{lang}.sh")))
                .expect("a regen script")
        })
        .collect();
    for scenario in &scenarios {
        let text = std::fs::read_to_string(scenario).expect("a scenario");
        let value: serde_json::Value = serde_json::from_str(&text).expect("a scenario is JSON");
        let machine = value["machine"]
            .as_str()
            .expect("a scenario names its machine");
        let document = std::fs::read_to_string(fixtures.join(format!("{machine}.scxml")))
            .unwrap_or_else(|_| panic!("{}: no fixture {machine}.scxml", scenario.display()));
        for script in &regen {
            assert!(
                script.contains(r#"grep -l 'datamodel="sce-static"'"#),
                "a regen script lists its machines by hand, so {machine} is one it may not commit"
            );
        }
        let stem = scenario
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("a scenario file has a name");
        // Beside its siblings, where its `<sce:import src>`s resolve: a probe
        // written alone would be refused for an import it cannot find, and
        // read as a machine no backend lowers.
        let siblings: Vec<(String, String)> = std::fs::read_dir(&fixtures)
            .expect("the fixture directory")
            .map(|entry| entry.expect("an entry").path())
            .filter(|path| path.extension().is_some_and(|e| e == "scxml"))
            .map(|path| {
                (
                    path.file_name().unwrap().to_string_lossy().into_owned(),
                    std::fs::read_to_string(&path).expect("a fixture"),
                )
            })
            .collect();
        let siblings: Vec<(&str, &str)> = siblings
            .iter()
            .map(|(name, text)| (name.as_str(), text.as_str()))
            .collect();
        let lowering: Vec<&str> = ["cpp", "c11", "go", "python", "kotlin", "rust"]
            .into_iter()
            .filter(|lang| run_beside(&["check", "-l", lang], &document, &siblings).0)
            .collect();
        assert!(
            !lowering.is_empty(),
            "{machine}: no backend lowers it, so no scenario runs it"
        );
        // A backend that lowers the machine commits it where its driver builds
        // it from; one that refuses it has nothing to commit.
        for (lang, committed) in COMMITTED_MACHINES {
            if !lowering.contains(lang) {
                continue;
            }
            let path = root.join(committed.replace("{m}", machine));
            assert!(
                path.exists(),
                "{machine}: its {lang} regen script has not committed {}",
                path.display()
            );
        }
        for lang in lowering {
            let (_, driver, marker) = SCENARIO_DRIVERS
                .iter()
                .find(|(l, _, _)| *l == lang)
                .unwrap_or_else(|| panic!("{lang} lowers sce-static but has no scenario driver"));
            let source = std::fs::read_to_string(root.join(driver)).expect("a driver");
            assert!(
                source.contains(&marker.replace("{s}", stem)),
                "{lang}: {driver} does not replay the scenario {stem}"
            );
        }
    }
}
