// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The types `map<K, T>[B]` and `set<K>[B]` (docs/adr/0108, D1): read by the
//! one type grammar at every position that declares a value, their key judged
//! where the type is read and, for a record key, where the record is read
//! (`validation/map-key-type`); refused in every position that crosses the
//! wire (`validation/map-in-wire-position`, D7); and refused by name in every
//! position a later step lands them in, a map held inside another type of a
//! statechart's variable and an algorithm's slot
//! (`generate/unsupported-feature`, D10). A statechart's variable that is
//! itself a map or a set is checked.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::diagnostic::ToDiagnostics;
use sce_build::forge::error::{ForgeError, Located, ValidationError};
use sce_build::forge::model::ForgeDocument;
use sce_build::forge::parser::parse_forge;
use sce_build::DocumentLabel;

fn label() -> DocumentLabel<'static> {
    DocumentLabel {
        identifier: "probe",
        diagnostic_label: "probe.scxml",
    }
}

/// The wire code of the first record `refusal` reports.
fn code_of(refusal: &Located<ForgeError>) -> String {
    serde_json::to_value(refusal.error.to_diagnostics()[0].code)
        .expect("a code serialises")
        .as_str()
        .expect("a code is a string")
        .to_string()
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// `ty` as an attribute spells it, `<` and `>` escaped.
fn escaped(ty: &str) -> String {
    ty.replace('<', "&lt;").replace('>', "&gt;")
}

/// An event schema with no imports whose field `f` is typed `ty`.
fn schema(ty: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="event-schema" name="probe" sce:event-name="probe.sent">
  <datamodel>
    <data id="n" sce:type="uint32" sce:direction="in"/>
    <data id="f" sce:type="{}" sce:direction="in"/>
  </datamodel>
</scxml>
"#,
        escaped(ty)
    )
}

/// A CBOR codec with no imports whose one entry `v` is typed `ty`.
fn cbor_codec(ty: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:encoding="cbor" sce:codec-format="2" name="probe_any" version="1.0">
  <datamodel>
    <data id="raw" sce:type="bytes" sce:direction="in"/>
    <data id="v" sce:type="{}" sce:key="0" sce:direction="out"/>
  </datamodel>
</scxml>
"#,
        escaped(ty)
    )
}

/// An algorithm that takes `param` and holds the local `local`.
fn algorithm(param: &str, local: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe" version="1.0">
  <sce:signature>
    <sce:param name="faces" type="{}"/>
    <sce:return type="int64"/>
  </sce:signature>
  <sce:body>
    <sce:var name="held" type="{}"/>
    <sce:return expr="1"/>
  </sce:body>
</scxml>
"#,
        escaped(param),
        escaped(local)
    )
}

/// A `sce-static` statechart that imports the event schemas `imports` (an
/// alias and a file beside it) and declares the variable `faces` of `ty`.
fn statechart(imports: &[(&str, &str)], ty: &str) -> String {
    let imports: String = imports
        .iter()
        .map(|(alias, src)| {
            format!("\n  <sce:import kind=\"event-schema\" src=\"{src}\" as=\"{alias}\"/>")
        })
        .collect();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="s" datamodel="sce-static" name="probe">{imports}
  <datamodel>
    <data id="faces" sce:type="{}"/>
  </datamodel>
  <state id="s"/>
</scxml>
"#,
        escaped(ty)
    )
}

/// An event schema named `name` whose fields are `fields`, each an id and a
/// type, every one of them `in`.
fn record(name: &str, fields: &[(&str, &str)]) -> String {
    let fields: String = fields
        .iter()
        .map(|(id, ty)| {
            format!(
                "\n    <data id=\"{id}\" sce:type=\"{}\" sce:direction=\"in\"/>",
                escaped(ty)
            )
        })
        .collect();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="event-schema" name="{name}" sce:event-name="{name}.sent">
  <datamodel>{fields}
  </datamodel>
</scxml>
"#
    )
}

/// `sce-codegen --error-format=json check` of `document`, written in a
/// scratch directory beside each of `siblings` (a file name and its text):
/// whether it passed, and what it printed.
fn check(document: &str, siblings: &[(&str, String)]) -> (bool, String) {
    let dir = tempfile::tempdir().expect("a scratch directory");
    for (name, text) in siblings {
        std::fs::write(dir.path().join(name), text).expect("write a sibling");
    }
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, document).expect("write the statechart");
    let out = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .arg("check")
        .arg(&path)
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// docs/adr/0108, D7: an event schema's field holds no map or set at any
/// depth, and neither does a codec's entry. Each is refused where its type is
/// read, naming the map and the record that decides its wire form, the type
/// as the document spells it the record's `actual`.
#[test]
fn a_map_is_refused_in_a_wire_position() {
    for (ty, place) in [
        ("map<uint32, bool>[4]", "map<uint32, bool>[4]"),
        ("set<string[8]>[2]", "set<string[8]>[2]"),
        ("list<set<uint8>[2]>[3]", "set<uint8>[2]"),
        (
            "optional<map<uint8, uint8>[faces=4]>",
            "map<uint8, uint8>[faces=4]",
        ),
    ] {
        for document in [schema(ty), cbor_codec(ty)] {
            let refused = parse_forge(&document, label()).expect_err(ty);
            assert_eq!(code_of(&refused), "validation/map-in-wire-position", "{ty}");
            let ForgeError::Validation(why) = &refused.error else {
                panic!("{ty}: a validation error, got {:?}", refused.error);
            };
            let ValidationError::MapInWirePosition {
                value, place: held, ..
            } = &**why
            else {
                panic!("{ty}: the map refused in a wire position, got {why:?}");
            };
            assert_eq!(value, &escaped(ty), "{ty}: the type as written");
            assert_eq!(held, place, "{ty}: the map it holds");
            assert!(
                refused.to_string().contains("(docs/adr/0108, D7)"),
                "{ty}: {refused}"
            );
        }
    }
    // A list of entries crosses as it always did.
    assert!(matches!(
        parse_forge(&schema("list<uint32>[4]"), label()),
        Ok(Some(ForgeDocument::EventSchema(_)))
    ));
}

/// docs/adr/0108, D1: a key whose equality is not exact, or which is no one
/// bounded value, is refused where the type is read, wherever it stands: a
/// float, an optional, a union, a list, a map and a set. The refusal names the
/// key and the map, and a lower count in a map's bracket is refused as one
/// nothing reads.
#[test]
fn a_map_key_is_refused_where_its_type_is_read() {
    for (key, why) in [
        ("float64", "a float is no key"),
        ("optional<uint8>", "an optional value is no key"),
        ("list<uint8>[2]", "a list, a map or a set is no key"),
        ("set<uint8>[2]", "a list, a map or a set is no key"),
    ] {
        let ty = format!("map<{key}, bool>[4]");
        let refused = parse_forge(&schema(&ty), label()).expect_err(&ty);
        assert_eq!(code_of(&refused), "validation/map-key-type", "{ty}");
        let sentence = refused.to_string();
        assert!(
            sentence.contains(&format!("`{key}` is no key of `{ty}`: {why}")),
            "{ty}: {sentence}"
        );
        let refused =
            parse_forge(&algorithm(&format!("set<{key}>"), "int64"), label()).expect_err(&ty);
        assert_eq!(code_of(&refused), "validation/map-key-type", "set<{key}>");
    }
    let refused =
        parse_forge(&algorithm("int64", "map<uint8, bool>[1..4]"), label()).expect_err("[1..4]");
    assert_eq!(code_of(&refused), "validation/lower-count-misplaced");
    assert!(
        refused
            .to_string()
            .contains("a map or a set holds what its puts and removes leave"),
        "{refused}"
    );
}

/// docs/adr/0108, D1: a record is a key when every field of it is, through
/// every record a field names, judged where the statechart reads the records
/// it imports; one that is not is refused naming its field.
#[test]
fn a_record_key_is_judged_through_its_fields() {
    let siblings = [
        (
            "zid.scxml",
            record("zid", &[("low", "uint64"), ("high", "uint64")]),
        ),
        (
            "ratio.scxml",
            record("ratio", &[("id", "uint8"), ("ratio", "float64")]),
        ),
    ];
    let (ok, out) = check(
        &statechart(&[("Ratio", "ratio.scxml")], "set<record:Ratio>[4]"),
        &siblings,
    );
    assert!(!ok, "a record key with a float field:\n{out}");
    assert!(
        out.contains("\"code\":\"validation/map-key-type\"")
            && out.contains("its field `ratio` is a `float64`")
            && out.contains("\"actual\":\"set&lt;record:Ratio&gt;[4]\""),
        "{out}"
    );
    // A record of two integers is a key: the variable is accepted, its key
    // judged through its fields.
    let (ok, out) = check(
        &statechart(&[("Zid", "zid.scxml")], "map<record:Zid, bool>[4]"),
        &siblings,
    );
    assert!(ok, "a map variable of a record key:\n{out}");
}

/// docs/adr/0108, D3 and D10: a statechart's variable that is itself a map or
/// a set is checked (each lane refuses it where it generates,
/// `a_map_variable_is_judged_by_the_checker_and_generated_by_no_lane`), and
/// one held inside another type is refused by name, the type and the record
/// that lands it named; and, as a list does, a map starts empty and takes no
/// `expr`.
#[test]
fn a_map_variable_is_checked_and_a_map_inside_another_type_is_refused_by_name() {
    for ty in ["map<uint32, bool>[4]", "set<string[8]>[faces=2]"] {
        let (ok, out) = check(&statechart(&[], ty), &[]);
        assert!(ok, "{ty} in a variable is checked:\n{out}");
    }
    let (ok, out) = check(&statechart(&[], "list<set<uint8>[2]>[3]"), &[]);
    assert!(!ok, "a set inside a list:\n{out}");
    assert!(
        out.contains("\"code\":\"generate/unsupported-feature\"")
            && out.contains(
                "`set<uint8>[2]`: a map or a set is a type of the language, and no lowering \
                 holds one here yet (docs/adr/0108, D10)"
            ),
        "{out}"
    );
    let (ok, out) = check(
        &statechart(&[], "map<uint32, bool>[4]")
            .replace("/>\n  </datamodel>", " expr=\"0\"/>\n  </datamodel>"),
        &[],
    );
    assert!(!ok, "a map variable with an expr:\n{out}");
    assert!(
        out.contains("a map or set variable starts empty and is filled by <sce:put>"),
        "{out}"
    );
    // A key that is no key is refused for its key, before the variable's
    // shape is asked.
    let (ok, out) = check(&statechart(&[], "map<float64, bool>[4]"), &[]);
    assert!(!ok, "a float key:\n{out}");
    assert!(
        out.contains("\"code\":\"validation/map-key-type\""),
        "{out}"
    );
}

/// docs/adr/0108, D10: no algorithm's slot holds a map yet, a parameter, a
/// local or an element of a list, each refused by name with the record that
/// lands it.
#[test]
fn a_map_in_an_algorithm_slot_is_refused_by_name() {
    for (param, local) in [
        ("map<uint32, int64>", "int64"),
        ("set<uint32>", "int64"),
        ("int64", "map<uint32, int64>[4]"),
        ("list<map<uint8, bool>[2]>", "int64"),
    ] {
        let refused = parse_forge(&algorithm(param, local), label()).expect_err(param);
        assert_eq!(
            code_of(&refused),
            "generate/unsupported-feature",
            "{param} / {local}"
        );
        let sentence = refused.to_string();
        assert!(
            sentence.contains("in an algorithm is a type of the language")
                && sentence.contains("(docs/adr/0108, D10)"),
            "{param} / {local}: {sentence}"
        );
    }
}
