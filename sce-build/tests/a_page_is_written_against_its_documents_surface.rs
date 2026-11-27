// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A page is written against the surface its document generates (docs/adr/0117,
//! D7): `sce-codegen surface` writes a JavaScript module and its TypeScript
//! declaration that name the events the machine takes and the snapshot it
//! publishes, and check what a page hands the machine by the payload schema each
//! event's schema unit declares.
//!
//! These cases hold the two files to the document, on the binary a page's build
//! runs: the module checks each event by the very text `payload-schema` prints
//! for its schema; the declaration types the events the machine takes from
//! outside and no other, and the variables it publishes and no other; and a
//! document of another data model is refused. That TypeScript refuses a page
//! that raises a wrong event, and that the module refuses one at run time, is
//! held where a compiler and Node run: the lane `wasm32-scenarios`
//! (`backends/rust/browser-node/surface`).

use std::path::{Path, PathBuf};
use std::process::Command;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn fixtures() -> PathBuf {
    repo_root().join("sce-build/tests/fixtures/static_datamodel")
}

fn scratch(label: &str) -> PathBuf {
    let dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// The module and the declaration `sce-codegen surface` writes for
/// `static_optional`: values that may be absent, in variables, in a record and a
/// list of it, and in the payloads of the two events whose schemas it imports.
fn surface(label: &str) -> (String, String) {
    let dir = scratch(label);
    let out = Command::new(sce_codegen_bin())
        .arg("surface")
        .arg(fixtures().join("static_optional.scxml"))
        .arg("--out-dir")
        .arg(&dir)
        .output()
        .expect("run sce-codegen surface");
    assert!(
        out.status.success(),
        "static_optional is a static machine every backend's scenarios read, and its surface \
         must be written: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let read = |file: &str| {
        std::fs::read_to_string(dir.join(file)).unwrap_or_else(|e| panic!("read {file}: {e}"))
    };
    let written = (read("static_optional.mjs"), read("static_optional.d.mts"));
    let _ = std::fs::remove_dir_all(&dir);
    written
}

/// `schema`, as `payload-schema` prints it, without the source hash of the
/// files it was printed for: the member stands between `$id` and `description`.
fn without_source_hash(schema: &str) -> String {
    const KEY: &str = "\"x-sce-source-hash\":\"";
    let start = schema
        .find(KEY)
        .expect("payload-schema names the source hash of what it read");
    let close = start
        + KEY.len()
        + schema[start + KEY.len()..]
            .find('"')
            .expect("the hash is a JSON string");
    // The closing quote, and the comma after it.
    format!("{}{}", &schema[..start], &schema[close + 2..])
}

/// The module checks each event by the payload schema its schema unit declares,
/// the one text `payload-schema` prints, so what the boundary admits is what the
/// machine's reader of the payload admits.
#[test]
fn the_module_checks_each_event_by_the_schema_its_unit_declares() {
    let (module, _) = surface("by-its-schema");
    for schema in ["schema_due.scxml", "schema_due_sent.scxml"] {
        let out = Command::new(sce_codegen_bin())
            .arg("payload-schema")
            .arg(fixtures().join(schema))
            .output()
            .expect("run sce-codegen payload-schema");
        assert!(
            out.status.success(),
            "{schema}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let printed = String::from_utf8(out.stdout).expect("a payload schema is ASCII");
        let expected = without_source_hash(printed.trim_end());
        assert!(
            module.contains(&expected),
            "the module does not check its event by the schema {schema} declares:\n{expected}\n\
             -- module --\n{module}"
        );
    }
}

/// The events a page may raise are those the machine takes from outside: the two
/// its schemas declare, with their data, and every other event a transition
/// takes, with none. The platform's own, which no page raises, are not among
/// them.
#[test]
fn the_declaration_types_the_events_the_machine_takes_and_no_other() {
    let (module, declaration) = surface("events");
    for event in ["due.set", "due.sent"] {
        let case =
            format!("{{ readonly name: \"{event}\"; readonly data: Payloads[\"{event}\"] }}");
        assert!(
            declaration.contains(&case),
            "no case {case}:\n{declaration}"
        );
    }
    for event in ["due.clear", "due.read", "due.fallback", "relay", "finish"] {
        let case = format!("{{ readonly name: \"{event}\" }}");
        assert!(
            declaration.contains(&case),
            "no case {case}:\n{declaration}"
        );
    }
    assert_eq!(
        declaration.matches("\n  | { readonly name: ").count(),
        7,
        "the declaration's Event has a case for an event the machine does not take:\n{declaration}"
    );
    assert!(
        !declaration.contains("error.execution") && !module.contains("error.execution"),
        "an event the platform raises is offered to a page"
    );
    assert!(
        declaration.contains("readonly \"id\": number;")
            && declaration.contains("readonly \"due\": string | null;")
            && declaration
                .contains("readonly \"status\": \"open\" | \"in_progress\" | \"done\" | null;"),
        "the payload of due.set is not typed by its schema:\n{declaration}"
    );
}

/// What a page is told is what the document publishes (`sce:direction="out"`),
/// each value in its wire form, and nothing the machine keeps to itself.
#[test]
fn the_declaration_types_what_the_machine_publishes_and_nothing_it_keeps() {
    let (_, declaration) = surface("published");
    for member in [
        "readonly \"other\": string | null;",
        "readonly \"title\": string | null;",
        "readonly \"item\": Definitions[\"schema_due\"];",
        "readonly \"items\": readonly (Definitions[\"schema_due\"])[];",
        "readonly \"days\": string;",
        "readonly \"late\": boolean;",
        "readonly \"plain\": \"open\" | \"in_progress\" | \"done\";",
        "readonly \"missing\": number;",
        "readonly \"schema_due\": {",
    ] {
        assert!(
            declaration.contains(member),
            "no {member} in the declaration:\n{declaration}"
        );
    }
    assert!(
        !declaration.contains("\"huge\""),
        "a variable the machine keeps to itself is published to a page"
    );
    assert!(
        declaration.contains("export type StateId = \"done\" | \"tracking\";"),
        "the states of the machine are not its StateId:\n{declaration}"
    );
}

/// Both files carry the header every generated source carries, so
/// `sce-codegen verify` can say whether they were written from the document as
/// it stands.
#[test]
fn each_file_names_the_document_it_was_written_from() {
    let (module, declaration) = surface("header");
    for (file, text) in [("module", &module), ("declaration", &declaration)] {
        assert!(
            text.starts_with("// ") && text.contains("// source-hash: "),
            "the {file} carries no source hash:\n{text}"
        );
        assert!(
            text.contains("static_optional.scxml"),
            "the {file} does not name its document:\n{text}"
        );
    }
}

/// A document whose values have no type has no surface to write: it is refused
/// by name, and nothing is written.
#[test]
fn a_document_of_another_data_model_is_refused() {
    let dir = scratch("untyped");
    let document = dir.join("untyped.scxml");
    std::fs::write(
        &document,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="a" name="untyped">
  <datamodel><data id="n" expr="0"/></datamodel>
  <state id="a"><transition event="go" target="b"/></state>
  <final id="b"/>
</scxml>
"#,
    )
    .expect("write the document");
    let out_dir = dir.join("out");
    let out = Command::new(sce_codegen_bin())
        .arg("surface")
        .arg(&document)
        .arg("--out-dir")
        .arg(&out_dir)
        .output()
        .expect("run sce-codegen surface");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let wrote = out_dir.join("untyped.mjs").exists() || out_dir.join("untyped.d.mts").exists();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        !out.status.success(),
        "a document of datamodel=\"ecmascript\" was given a surface"
    );
    assert!(
        stderr.contains("sce-static"),
        "the refusal does not say what a surface is written from: {stderr}"
    );
    assert!(!wrote, "a refused document left a file of its surface");
}
