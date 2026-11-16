// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A field of an event schema a statechart imports, and a variant of an enum it
//! imports, become names the generated machine declares: a property of the
//! payload's and the record's struct, a member of the enum's type. A name one
//! backend reserves cannot be declared there, and it is refused once, for every
//! backend, where the document that declares it names it
//! (`validation/reserved-code-identifier`, SCE Accepted Subset §2.14), by the
//! parser's sweep of every forge document (`reject_malformed`,
//! `sce-build/src/scxml_identifier.rs`), which every imported schema and enum
//! passes through. No backend's gate asks it again (docs/adr/0086, section 7.3):
//! the Swift gate's arms for a field and a variant, and the Go and Python gates'
//! arms for a record's field, could never be reached past the parser and are
//! gone.
//!
//! # What is held
//!
//! * A field named by a word only Swift, only Go or only Python reserves is
//!   refused for that backend and for another, naming the backend that reserves
//!   it and the schema's document.
//! * A variant whose lowerCamelCase spelling Swift reserves is refused the same
//!   way, where the enum's document declares it.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::tempdir;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// An event schema of plain fields whose second field is named `word`.
fn schema(word: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="probe" sce:event-name="probe.sent">
  <datamodel>
    <data id="count" sce:type="uint8" sce:direction="in"/>
    <data id="{word}" sce:type="uint8" sce:direction="in"/>
  </datamodel>
</scxml>
"#
    )
}

/// A statechart that imports `event_probe.scxml`, takes its event and reads
/// its first field.
const SCHEMA_READER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="probe_reader">
  <sce:import kind="event-schema" src="event_probe.scxml" as="Probe"/>
  <datamodel>
    <data id="seen" sce:type="uint32" expr="0" sce:direction="out"/>
  </datamodel>
  <state id="idle">
    <transition event="probe.sent" target="idle">
      <assign location="seen" expr="seen + _event.data.count"/>
    </transition>
  </state>
</scxml>
"#;

/// An enum whose first variant is named `word`.
fn enumeration(word: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="enum" name="probe_mode" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="{word}" value="0"/>
      <sce:variant name="open" value="1"/>
    </data>
  </datamodel>
</scxml>
"#
    )
}

/// A statechart that imports `enum_probe_mode.scxml` and holds a variable of it.
const ENUM_READER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="mode_reader">
  <sce:import kind="enum" src="enum_probe_mode.scxml" as="ProbeMode"/>
  <datamodel>
    <data id="mode" sce:type="enum:ProbeMode" expr="ProbeMode.open" sce:direction="out"/>
  </datamodel>
  <state id="idle"/>
</scxml>
"#;

/// `sce-codegen check -l <language>` over `reader` written as `name` beside
/// `sibling` (its file name and text), and whether it succeeded, with what it
/// said.
fn check(language: &str, sibling: (&str, &str), name: &str, reader: &str) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    std::fs::write(dir.path().join(sibling.0), sibling.1).expect("write the sibling");
    std::fs::write(dir.path().join(name), reader).expect("write the reader");
    let out = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(["check", "-l", language])
        .arg(dir.path().join(name))
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// What a refusal of `word`, reserved by `reserver`, declared in `document`,
/// says in its JSON form.
fn refused_parts(word: &str, reserver: &str, element: &str, document: &str) -> Vec<String> {
    vec![
        "validation/reserved-code-identifier".to_string(),
        element.to_string(),
        format!("'{word}' is a reserved word in {reserver}"),
        document.to_string(),
    ]
}

/// The schema's documents read first: so the same document is refused for the
/// backend that reserves the word and for one that does not, the refusal being
/// the document's and no backend's gate's.
#[test]
fn a_field_a_backend_reserves_is_refused_for_every_backend_where_it_is_declared() {
    // Each word is reserved by one backend alone, so the refusal names it.
    for (word, reserver) in [
        ("fileprivate", "swift"),
        ("chan", "go"),
        ("nonlocal", "python"),
    ] {
        let document = schema(word);
        for language in [reserver, "rust"] {
            let (ok, out) = check(
                language,
                ("event_probe.scxml", &document),
                "probe_reader.scxml",
                SCHEMA_READER,
            );
            assert!(!ok, "{language}: a field `{word}` is no field:\n{out}");
            for part in refused_parts(
                word,
                reserver,
                &format!("<data id=\\\"{word}\\\">"),
                "event_probe.scxml",
            ) {
                assert!(
                    out.contains(&part),
                    "{language}, `{word}`: expected `{part}` in:\n{out}"
                );
            }
        }
    }
}

/// The same schema with a name no backend reserves is taken by every backend
/// asked above: the refusal is the word's, not the shape's.
#[test]
fn the_same_schema_with_a_name_no_backend_reserves_is_taken() {
    let document = schema("level");
    for language in ["swift", "go", "python", "rust"] {
        let (ok, out) = check(
            language,
            ("event_probe.scxml", &document),
            "probe_reader.scxml",
            SCHEMA_READER,
        );
        assert!(ok, "{language} takes a field `level`:\n{out}");
    }
}

/// A variant is a member of its enum's Swift type, spelled lowerCamelCase:
/// `internal` is a word Swift reserves, refused where the enum's document
/// declares it, for Swift and for another backend.
#[test]
fn a_variant_swift_reserves_is_refused_for_every_backend_where_it_is_declared() {
    let document = enumeration("internal");
    for language in ["swift", "rust"] {
        let (ok, out) = check(
            language,
            ("enum_probe_mode.scxml", &document),
            "mode_reader.scxml",
            ENUM_READER,
        );
        assert!(!ok, "{language}: a variant `internal` is no member:\n{out}");
        for part in refused_parts(
            "internal",
            "swift",
            "<sce:variant name=\\\"internal\\\">",
            "enum_probe_mode.scxml",
        ) {
            assert!(
                out.contains(&part),
                "{language}: expected `{part}` in:\n{out}"
            );
        }
    }
}

/// The same enum with a variant no backend reserves is taken.
#[test]
fn the_same_enum_with_a_variant_no_backend_reserves_is_taken() {
    let document = enumeration("closed");
    for language in ["swift", "rust"] {
        let (ok, out) = check(
            language,
            ("enum_probe_mode.scxml", &document),
            "mode_reader.scxml",
            ENUM_READER,
        );
        assert!(ok, "{language} takes a variant `closed`:\n{out}");
    }
}
