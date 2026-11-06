// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The unit an event schema generates declares the payload's schema id and its
//! JSON Schema on every backend, and `sce-codegen payload-schema` prints the same
//! JSON Schema, byte for byte, for a host that has no unit (docs/adr/0087,
//! section 1.1, item 5). The GBNF grammar, as long as the largest text bound, is
//! written only where a grammar is used: `payload-schema --format gbnf` prints
//! it, and no unit declares it ("Owner's answers", item 4).
//!
//! The fixture is `fixtures/payload_schema/answer_slot.scxml`, the schema the
//! decision writes out, in a directory of its own: `generate` and `payload-schema`
//! are each run on it with the root they infer, the document's directory, so the
//! source hash the JSON Schema names is one both compute.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::tempdir;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn answer_slot() -> PathBuf {
    repo_root().join("sce-build/tests/fixtures/payload_schema/answer_slot.scxml")
}

fn codegen() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sce-codegen"));
    command.arg("--workspace-root").arg(repo_root());
    command
}

/// What `payload-schema` prints for `document`: `global` stands before the
/// subcommand (`--limit`, `--error-format`), `own` after the document
/// (`--format`).
fn payload_schema(global: &[&str], document: &Path, own: &[&str]) -> Output {
    codegen()
        .args(global)
        .arg("payload-schema")
        .arg(document)
        .args(own)
        .output()
        .expect("invoke sce-codegen")
}

fn printed(global: &[&str], document: &Path, own: &[&str]) -> String {
    let out = payload_schema(global, document, own);
    assert!(
        out.status.success(),
        "payload-schema {} {own:?}:\n{}",
        document.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("UTF-8")
}

/// Every file `generate -l <language>` writes for `document`, as one text.
fn generated(document: &Path, language: &str) -> String {
    let out_dir = tempdir().expect("tempdir");
    let out = codegen()
        .arg("generate")
        .arg(document)
        .args([
            "-l",
            language,
            "--go-module-prefix",
            "example.test/generated",
        ])
        .arg("-o")
        .arg(out_dir.path())
        .output()
        .expect("invoke sce-codegen");
    assert!(
        out.status.success(),
        "generate {} -l {language}:\n{}",
        document.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut text = String::new();
    let mut pending = vec![out_dir.path().to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).expect("the output directory") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                text.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
            }
        }
    }
    text
}

/// The text of the string literal that follows `declaration` on its line,
/// its escapes read: `\\`, `\"`, `\n`, `\r`, `\t` and Kotlin's `\$`, the only
/// ones a payload schema's ASCII text takes in Rust, Kotlin, Go, Python and
/// Swift.
fn literal_after(unit: &str, declaration: &str) -> String {
    let at = unit
        .find(declaration)
        .unwrap_or_else(|| panic!("the unit declares `{declaration}`:\n{unit}"));
    let rest = &unit[at + declaration.len()..];
    let mut chars = rest.chars();
    let mut text = String::new();
    loop {
        match chars.next().expect("the literal ends") {
            '"' => return text,
            '\\' => text.push(match chars.next().expect("an escape") {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                other => other,
            }),
            c => text.push(c),
        }
    }
}

/// The text the `char` array initialized after `declaration` holds: its
/// character constants read up to the zero that ends it.
fn char_array_after(unit: &str, declaration: &str) -> String {
    let at = unit
        .find(declaration)
        .unwrap_or_else(|| panic!("the unit declares `{declaration}`:\n{unit}"));
    let rest = &unit[at + declaration.len()..];
    let open = rest.find('{').expect("an initializer");
    let mut bytes = Vec::new();
    let mut chars = rest[open + 1..].chars();
    loop {
        match chars.next().expect("the initializer ends") {
            '\'' => {
                let byte = match chars.next().expect("a character") {
                    '\\' => match chars.next().expect("an escape") {
                        'x' => {
                            let hex: String = [chars.next(), chars.next()]
                                .into_iter()
                                .map(|c| c.expect("a hex digit"))
                                .collect();
                            u8::from_str_radix(&hex, 16).expect("hex")
                        }
                        '0' => break,
                        other => other as u8,
                    },
                    c => c as u8,
                };
                assert_eq!(chars.next(), Some('\''), "one character a constant");
                bytes.push(byte);
            }
            '}' => panic!("the initializer ends at its zero"),
            _ => {}
        }
    }
    String::from_utf8(bytes).expect("UTF-8")
}

#[test]
fn every_unit_declares_the_json_schema_payload_schema_prints() {
    let document = answer_slot();
    let json_schema = printed(&[], &document, &[]);
    assert_eq!(
        printed(&[], &document, &["--format", "json-schema"]),
        json_schema,
        "the JSON Schema is the default"
    );
    assert!(json_schema.is_ascii(), "{json_schema}");
    // The hash `generate` stamps on the files of the same document, beside
    // the id, and the record's keys in the order the document declares them.
    let rust = generated(&document, "rust");
    let header = rust
        .lines()
        .find_map(|line| line.strip_prefix("// source-hash: "))
        .expect("a drift header");
    assert!(
        json_schema.starts_with(&format!(
            "{{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\
             \"$id\":\"urn:sce:schema:answer_slot:1\",\"x-sce-source-hash\":\"{header}\","
        )),
        "{json_schema}"
    );
    assert!(
        json_schema.ends_with(
            ",\"required\":[\"title\",\"slot\",\"kind\",\"note\"],\"additionalProperties\":false}"
        ),
        "{json_schema}"
    );

    for (language, id, json, grammar) in [
        (
            "rust",
            "pub const SCHEMA_ID: &str = \"",
            "pub const JSON_SCHEMA: &str = \"",
            "GBNF",
        ),
        (
            "kotlin",
            "const val SCHEMA_ID: String = \"",
            "const val JSON_SCHEMA: String = \"",
            "GBNF",
        ),
        (
            "go",
            "const SchemaID string = \"",
            "const JSONSchema string = \"",
            "GBNF",
        ),
        (
            "python",
            "SCHEMA_ID: Final[str] = \"",
            "JSON_SCHEMA: Final[str] = \"",
            "GBNF",
        ),
        (
            "swift",
            "public static let schemaID: String = \"",
            "public static let jsonSchema: String = \"",
            "gbnf",
        ),
    ] {
        let unit = generated(&document, language);
        assert_eq!(
            literal_after(&unit, id),
            "urn:sce:schema:answer_slot:1",
            "{language}"
        );
        assert_eq!(literal_after(&unit, json), json_schema, "{language}");
        assert!(
            !unit.contains(grammar),
            "{language}: no unit declares a grammar:\n{unit}"
        );
    }

    let cpp = generated(&document, "cpp");
    assert!(
        cpp.contains("inline constexpr const char *SCHEMA_ID = \"urn:sce:schema:answer_slot:1\";"),
        "{cpp}"
    );
    assert_eq!(char_array_after(&cpp, "JSON_SCHEMA[] ="), json_schema);
    assert!(!cpp.contains("GBNF"), "{cpp}");
    let c = generated(&document, "c");
    assert!(
        c.contains("#define ANSWER_SLOT_SCHEMA_ID \"urn:sce:schema:answer_slot:1\""),
        "{c}"
    );
    assert_eq!(
        char_array_after(&c, "ANSWER_SLOT_JSON_SCHEMA[] ="),
        json_schema
    );
    assert!(!c.contains("GBNF"), "{c}");
}

/// The GBNF grammar is the CLI's alone: the type's, counted in bytes, with no
/// source hash, and ASCII.
#[test]
fn payload_schema_prints_the_grammar_no_unit_declares() {
    let document = answer_slot();
    let gbnf = printed(&[], &document, &["--format", "gbnf"]);
    assert!(
        gbnf.starts_with(
            "root ::= \"{\\\"title\\\":\" \"\\\"\" t16 \"\\\"\" \",\\\"slot\\\":\" uint16 "
        ),
        "{gbnf}"
    );
    assert!(gbnf.contains("\nuint16 ::= \"0\" | "), "{gbnf}");
    assert!(
        gbnf.ends_with("t16 ::= \"\" | c1 t15 | c2 t14 | c3 t13 | c4 t12\n"),
        "{gbnf}"
    );
    assert!(gbnf.is_ascii() && !gbnf.contains("source-hash"), "{gbnf}");
}

/// A bound a build moves (`--limit`) moves in both texts, as it moves the
/// unit's buffers: `record_edit`'s `text` is `shared_typed_text`, 32 bytes as
/// written.
#[test]
fn a_limit_moves_the_bound_in_both_texts() {
    let document = repo_root().join("stdlib/shared/record_edit.scxml");
    let written = printed(&[], &document, &[]);
    assert!(written.contains("\"maxLength\":32,"), "{written}");
    let moved = printed(&["--limit", "shared_typed_text=48"], &document, &[]);
    assert!(
        moved.contains("\"maxLength\":48,") && moved.contains("\"x-sce-max-bytes\":48"),
        "{moved}"
    );
    let grammar = printed(
        &["--limit", "shared_typed_text=48"],
        &document,
        &["--format", "gbnf"],
    );
    assert!(grammar.contains("\"\\\"\" t48 \"\\\"\""), "{grammar}");
    assert!(
        grammar.ends_with("t48 ::= \"\" | c1 t47 | c2 t46 | c3 t45 | c4 t44\n"),
        "{grammar}"
    );
}

/// A schema whose type has no grammar is refused by name in either format, and
/// its unit declares its id and says why it declares no JSON Schema. A schema
/// is named by its document's file (`schema_uds_response_open`).
#[test]
fn a_schema_with_an_open_enum_has_no_grammar_and_says_why() {
    let document =
        repo_root().join("sce-build/tests/fixtures/event_schema/schema_uds_response_open.scxml");
    for own in [&[][..], &["--format", "gbnf"][..]] {
        let out = payload_schema(&["--error-format=json"], &document, own);
        assert!(
            !out.status.success(),
            "an open enum has no grammar: {own:?}"
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        let record: serde_json::Value = stderr
            .lines()
            .find_map(|line| serde_json::from_str(line.trim()).ok())
            .unwrap_or_else(|| panic!("a diagnostic record:\n{stderr}"));
        assert_eq!(record["code"], "generate/unsupported-feature", "{record}");
        let message = record["message"].as_str().unwrap_or_default();
        assert!(
            message.contains("the payload of `schema_uds_response_open` has no grammar")
                && message.contains("the enum `UdsNrc` is open"),
            "{record}"
        );
    }
    let unit = generated(&document, "rust");
    assert!(
        unit.contains("pub const SCHEMA_ID: &str = \"urn:sce:schema:schema_uds_response_open:1\";"),
        "{unit}"
    );
    assert!(
        unit.contains("// No JSON_SCHEMA: the enum `UdsNrc` is open"),
        "{unit}"
    );
    assert!(!unit.contains("pub const JSON_SCHEMA"), "{unit}");
}

/// `payload-schema` reads a union document (docs/adr/0092, section 3.1, item
/// 4): the JSON Schema of a value of the union, the `oneOf` of its cases at the
/// root, named by its tag enum's revision and the source hash `generate`
/// computes, and its GBNF grammar, one alternative per case, the tag first.
#[test]
fn payload_schema_prints_a_union_documents_one_of_and_grammar() {
    let document = repo_root().join("sce-build/tests/fixtures/union/union_shape.scxml");
    let json_schema = printed(&[], &document, &[]);
    assert!(
        json_schema.starts_with(
            "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\
             \"$id\":\"urn:sce:schema:union_shape:1\",\"x-sce-source-hash\":\""
        ),
        "{json_schema}"
    );
    assert!(
        json_schema.contains(
            ",\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"dot\"}},\
             \"required\":[\"kind\"],\"additionalProperties\":false},"
        ),
        "{json_schema}"
    );
    assert!(
        json_schema.ends_with(
            "\"required\":[\"kind\",\"width\",\"height\"],\"additionalProperties\":false}]}"
        ),
        "{json_schema}"
    );
    assert!(json_schema.is_ascii(), "{json_schema}");
    let gbnf = printed(&[], &document, &["--format", "gbnf"]);
    assert!(
        gbnf.starts_with(
            "root ::= \"{\\\"kind\\\":\\\"dot\\\"}\" | \
             \"{\\\"kind\\\":\\\"circle\\\",\\\"radius\\\":\" uint16 \"}\" | "
        ),
        "{gbnf}"
    );
    assert!(gbnf.is_ascii() && !gbnf.contains("source-hash"), "{gbnf}");
}

/// The unit of a union document declares the JSON Schema `payload-schema`
/// prints for it, under the id its tag enum's revision names (docs/adr/0092,
/// section 3.1, item 4), on Kotlin, the backend that lowers a union.
#[test]
fn a_union_unit_declares_the_json_schema_payload_schema_prints() {
    let document = repo_root().join("sce-build/tests/fixtures/union/union_shape.scxml");
    let json_schema = printed(&[], &document, &[]);
    let unit = generated(&document, "kotlin");
    assert_eq!(
        literal_after(&unit, "const val SCHEMA_ID: String = \""),
        "urn:sce:schema:union_shape:1"
    );
    assert_eq!(
        literal_after(&unit, "const val JSON_SCHEMA: String = \""),
        json_schema
    );
}

/// A union whose cases are not one to one with its tag enum's variants is
/// refused in its own words before any grammar is written.
#[test]
fn a_union_whose_cases_miss_a_variant_has_no_payload_schema() {
    let document =
        repo_root().join("sce-build/tests/fixtures/union/union_shape_missing_case.scxml");
    let out = payload_schema(&["--error-format=json"], &document, &[]);
    assert!(!out.status.success(), "a union with a missing case");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let record: serde_json::Value = stderr
        .lines()
        .find_map(|line| serde_json::from_str(line.trim()).ok())
        .unwrap_or_else(|| panic!("a diagnostic record:\n{stderr}"));
    assert_eq!(record["code"], "validation/union-cases", "{record}");
}

/// `payload-schema` reads an event schema or a union and refuses every other
/// document.
#[test]
fn a_document_that_is_no_event_schema_has_no_payload_schema() {
    for document in [
        repo_root().join("sce-build/tests/fixtures/payload_schema/answer_kind.scxml"),
        repo_root().join("sce-build/tests/fixtures/static_datamodel/static_counter.scxml"),
    ] {
        let out = payload_schema(&[], &document, &[]);
        assert!(!out.status.success(), "{}", document.display());
        assert!(
            String::from_utf8_lossy(&out.stderr).contains(
                "payload-schema reads an event schema (sce:kind=\"event-schema\") or a union \
                 (sce:kind=\"union\")"
            ),
            "{}:\n{}",
            document.display(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
