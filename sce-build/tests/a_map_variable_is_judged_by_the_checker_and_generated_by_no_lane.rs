// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A statechart's map or set variable (docs/adr/0108, D1 to D5), judged by the
//! `sce-static` checker before any lane lowers one: `check` accepts a
//! well-formed document (`fixtures/map/map_faces.scxml`, which writes every
//! read, statement and loop the checker reads once), and refuses each read,
//! statement and loop that is no use of a map by the code D11 names, saying
//! what is accepted; and every lane's `generate`, the Interpreter's lowering
//! among them, refuses the document by name, naming the record that lands a
//! map lane by lane (D10).
//!
//! Each refusal is the well-formed document with one thing changed, so the
//! refusal is the change's and nothing else's.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn fixtures() -> PathBuf {
    repo_root().join("sce-build/tests/fixtures/map")
}

/// The schemas the statechart imports, copied beside a variant of it.
const SIBLINGS: [&str; 2] = ["map_peer_key.scxml", "map_face.scxml"];

/// The well-formed statechart, as committed.
fn faces() -> String {
    std::fs::read_to_string(fixtures().join("map_faces.scxml")).expect("read the statechart")
}

/// `text` with the one occurrence of `from` replaced by `to`.
fn edited(text: &str, from: &str, to: &str) -> String {
    assert_eq!(
        text.matches(from).count(),
        1,
        "`{from}` stands once in the statechart"
    );
    text.replacen(from, to, 1)
}

/// `sce-codegen --error-format=json <args> <path>`: whether it passed, and
/// what it printed.
fn run(args: &[&str], path: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(path)
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// `check` of `document`, written beside the schemas it imports.
fn check(document: &str) -> (bool, String) {
    let dir = tempfile::tempdir().expect("a scratch directory");
    for name in SIBLINGS {
        std::fs::copy(fixtures().join(name), dir.path().join(name)).expect("copy a schema");
    }
    let path = dir.path().join("map_faces.scxml");
    std::fs::write(&path, document).expect("write the statechart");
    run(&["check"], &path)
}

/// `document` is refused by `check` with `code`, its message holding `part`.
fn refused(document: &str, code: &str, part: &str) {
    let (ok, out) = check(document);
    assert!(!ok, "expected a refusal holding `{part}`:\n{out}");
    assert!(
        out.contains(&format!("\"code\":\"{code}\"")) && out.contains(part),
        "expected {code} holding `{part}`:\n{out}"
    );
}

/// docs/adr/0108, D3 to D5 and D10: the document that writes every read,
/// statement and loop of a map the checker reads is accepted by `check`, and
/// refused by every lane that generates a statechart, and by the
/// Interpreter's lowering, by name.
#[test]
fn a_map_document_is_checked_and_refused_by_every_lane_by_name() {
    let path = fixtures().join("map_faces.scxml");
    let (ok, out) = run(&["check"], &path);
    assert!(ok, "check accepts the statechart:\n{out}");
    let (ok, out) = check(&faces());
    assert!(ok, "check accepts it beside copies of its schemas:\n{out}");

    let place = "`map<record:PeerKey, record:Face>[face_max=4]` in a statechart's variable";
    for &language in sce_build::generator::Language::ALL {
        let lane = language.canonical_name();
        let out_dir = tempfile::tempdir().expect("an output directory");
        let out_dir = out_dir.path().to_str().expect("a utf-8 path").to_string();
        let mut args = vec!["generate", "-l", lane, "-o", out_dir.as_str()];
        if language == sce_build::generator::Language::Go {
            args.extend(["--go-module-prefix", "example.test/generated"]);
        }
        let (ok, out) = run(&args, &path);
        assert!(!ok, "{lane} generates no map yet:\n{out}");
        assert!(
            out.contains("\"code\":\"generate/unsupported-feature\"")
                && out.contains(place)
                && out.contains(&format!(
                    "has no {lane} lowering of its map yet (docs/adr/0108, D10)"
                )),
            "{lane}:\n{out}"
        );
    }

    let out_dir = tempfile::tempdir().expect("an output directory");
    let out_dir = out_dir.path().to_str().expect("a utf-8 path").to_string();
    let (ok, out) = run(&["lower", "-o", out_dir.as_str()], &path);
    assert!(!ok, "the Interpreter's lowering holds no map yet:\n{out}");
    assert!(
        out.contains("\"code\":\"generate/unsupported-feature\"")
            && out.contains(place)
            && out.contains("has no ecmascript lowering of its map yet (docs/adr/0108, D10)"),
        "{out}"
    );
}

/// docs/adr/0108, D2, D3 and D11: a map is read through `len`, `has` and the
/// value at a key, and is no operand; a key fits the map's key as a value
/// assigned to a place of its type does; a set holds no value at a key.
#[test]
fn a_read_of_a_map_that_is_no_read_of_it_is_refused() {
    let faces = faces();
    let unsupported = "expression/unsupported-construct";
    for (from, to, code, part) in [
        // A map as an operand, and two maps compared.
        (
            "expr=\"refused + 1\"",
            "expr=\"refused + counts\"",
            unsupported,
            "reading the map or set `counts` as a value",
        ),
        (
            "cond=\"putResult == PutOutcome.full\"",
            "cond=\"counts == spare\"",
            unsupported,
            "reading the map or set `counts` as a value",
        ),
        // `has` of other than two arguments, and of no map.
        (
            "has(counts, 7)",
            "has(counts)",
            "expression/argument-count-mismatch",
            "has",
        ),
        (
            "has(counts, 7)",
            "has(total, 7)",
            unsupported,
            "whose first argument names no map or set variable",
        ),
        // A key of another kind than the map's.
        (
            "has(counts, 7)",
            "has(counts, 'seven')",
            "expression/type-mismatch",
            "uint32",
        ),
        (
            "has(counts, 7)",
            "has(counts, PutOutcome.full)",
            unsupported,
            "a value of an enum is no number, bool, text or byte string",
        ),
        (
            "flags['on']",
            "flags[7]",
            "expression/type-mismatch",
            "string",
        ),
        // A record key is named whole, by a record of its schema.
        (
            "has(faces, peer) &amp;&amp;",
            "has(faces, face) &amp;&amp;",
            unsupported,
            "a record key is named whole, by a record of its schema",
        ),
        (
            "has(faces, peer) &amp;&amp;",
            "has(faces, 1) &amp;&amp;",
            unsupported,
            "a record key is named whole, by a record of its schema",
        ),
        // A field the value's schema does not declare.
        (
            "faces[peer].active",
            "faces[peer].nope",
            unsupported,
            "a field the element's schema does not declare",
        ),
        // A set's entries carry no value.
        (
            "expr=\"len(seen)\"",
            "expr=\"seen[3]\"",
            unsupported,
            "a value read at a key of the set `seen`",
        ),
    ] {
        refused(&edited(&faces, from, to), code, part);
    }
}

/// docs/adr/0108, D2 and D3: an entry is written by `<sce:put>`, a map takes
/// whole only another map variable whose type it fits, and the value at a key
/// taken whole is the record its key reads once, of the variable's schema.
#[test]
fn a_map_is_written_by_its_statements_and_taken_whole_only_from_a_map() {
    let faces = faces();
    let unsupported = "expression/unsupported-construct";
    let rule = "scxml/static-datamodel-rule";
    for (from, to, code, part) in [
        (
            "<assign location=\"spare\" expr=\"counts\"/>",
            "<assign location=\"counts[7]\" expr=\"1\"/>",
            unsupported,
            "a part of an entry of the map `counts`",
        ),
        (
            "<assign location=\"spare\" expr=\"counts\"/>",
            "<assign location=\"spare\" expr=\"total\"/>",
            unsupported,
            "an assignment to the whole map `spare` from `total`",
        ),
        (
            "<assign location=\"spare\" expr=\"counts\"/>",
            "<assign location=\"counts\" expr=\"spare\"/>",
            unsupported,
            "an assignment to the whole map `counts` from `spare`: ",
        ),
        (
            "expr=\"byId[face.id]\"",
            "expr=\"byId[last.id]\"",
            rule,
            "the key of the entry reads `last`",
        ),
        (
            "<assign location=\"last\" expr=\"byId[face.id]\"/>",
            "<assign location=\"peer\" expr=\"byId[face.id]\"/>",
            rule,
            "a value of the map `byId` is a record of the schema Face",
        ),
    ] {
        refused(&edited(&faces, from, to), code, part);
    }
}

/// docs/adr/0108, D3, D4 and D11: `<sce:put>`, `<sce:remove>` and
/// `<sce:clear>` name a map or a set; a put's key and value fit the map's, the
/// value required of a map and refused on a set; and an outcome is a variable
/// of the standard enum of the statement.
#[test]
fn a_statement_of_a_map_is_refused_naming_what_it_takes() {
    let faces = faces();
    let rule = "scxml/static-datamodel-rule";
    for (from, to, code, part) in [
        (
            "<sce:put target=\"counts\" key=\"7\" expr=\"1\"/>",
            "<sce:put target=\"total\" key=\"7\" expr=\"1\"/>",
            rule,
            "target names a map or a set variable: one of faces, counts",
        ),
        (
            "<sce:put target=\"counts\" key=\"7\" expr=\"1\"/>",
            "<sce:put target=\"counts\" key=\"7\"/>",
            rule,
            "a put into the map `counts` names the value it puts at the key",
        ),
        (
            "<sce:put target=\"seen\" key=\"3\" outcome=\"putResult\"/>",
            "<sce:put target=\"seen\" key=\"3\" expr=\"1\" outcome=\"putResult\"/>",
            rule,
            "the set `seen` holds keys that carry no value",
        ),
        (
            "<sce:put target=\"counts\" key=\"7\" expr=\"1\"/>",
            "<sce:put target=\"counts\" key=\"'x'\" expr=\"1\"/>",
            "expression/type-mismatch",
            "uint32",
        ),
        (
            "<sce:put target=\"counts\" key=\"7\" expr=\"1\"/>",
            "<sce:put target=\"counts\" key=\"7\" expr=\"true\"/>",
            "expression/type-mismatch",
            "uint64",
        ),
        (
            "key=\"peer\" expr=\"face\" outcome=\"putResult\"",
            "key=\"face\" expr=\"face\" outcome=\"putResult\"",
            rule,
            "a whole record of the schema PeerKey is taken by name",
        ),
        (
            "key=\"peer\" expr=\"face\" outcome=\"putResult\"",
            "key=\"peer\" expr=\"peer\" outcome=\"putResult\"",
            rule,
            "a whole record of the schema Face is taken by name",
        ),
        (
            "key=\"peer\" expr=\"face\" outcome=\"putResult\"",
            "key=\"peer\" expr=\"face\" outcome=\"total\"",
            rule,
            "`total` is a variable of `uint64`, and outcome names a variable of the standard \
             enum sce:std/collection/put_outcome.scxml (inserted, replaced, full)",
        ),
        (
            "outcome=\"removeResult\"",
            "outcome=\"putResult\"",
            rule,
            "outcome names a variable of the standard enum \
             sce:std/collection/remove_outcome.scxml (removed, absent)",
        ),
        (
            "<sce:remove target=\"seen\" key=\"3\"/>",
            "<sce:remove target=\"seen\" key=\"3\" expr=\"1\"/>",
            rule,
            "a remove from `seen` names the key of the entry it removes, and takes no expr",
        ),
        (
            "<sce:remove target=\"seen\" key=\"3\"/>",
            "<sce:remove target=\"order\" key=\"3\"/>",
            rule,
            "target names a map or a set variable",
        ),
        (
            "<sce:clear target=\"flags\"/>",
            "<sce:clear target=\"total\"/>",
            rule,
            "target names a list, a map or a set variable: one of",
        ),
    ] {
        refused(&edited(&faces, from, to), code, part);
    }
}

/// docs/adr/0108, D5: a `<foreach>` over a map binds its key by `sce:key`,
/// held to the rules its item and index are; a set's item is its key and a
/// list's walk binds none; and the key is read, never written.
#[test]
fn a_walk_over_a_map_binds_its_key_by_the_rules_of_a_loop() {
    let faces = faces();
    let rule = "scxml/static-datamodel-rule";
    for (from, to, code, part) in [
        (
            "<foreach array=\"seen\" item=\"s\">",
            "<foreach array=\"seen\" item=\"s\" sce:key=\"t\">",
            rule,
            "the set `seen` holds keys that carry no value",
        ),
        (
            "<foreach array=\"order\" item=\"o\">",
            "<foreach array=\"order\" item=\"o\" sce:key=\"t\">",
            rule,
            "a <foreach> over a list binds each element and its index",
        ),
        (
            "sce:key=\"k\"",
            "sce:key=\"f\"",
            rule,
            "a <foreach> sce:key is a name other than the item's",
        ),
        (
            "index=\"i\"",
            "index=\"k\"",
            rule,
            "a <foreach> index is a name other than the key's",
        ),
        (
            "sce:key=\"k\"",
            "sce:key=\"total\"",
            rule,
            "a <foreach> sce:key is a name nothing in scope already means",
        ),
        (
            "<assign location=\"walked\" expr=\"i\"/>",
            "<assign location=\"k.low\" expr=\"1\"/>",
            "expression/unsupported-construct",
            "which a <foreach> binds",
        ),
        (
            "<sce:put target=\"counts\" key=\"f.id\" expr=\"k.low\"/>",
            "<sce:put target=\"counts\" key=\"f.id\" expr=\"k.nope\"/>",
            "expression/unknown-member",
            "nope",
        ),
    ] {
        refused(&edited(&faces, from, to), code, part);
    }
}

/// docs/adr/0108, D1 and D10: a map whose value is no value an entry's read
/// reads (a list, another map, an optional value) is the language's, and is
/// refused by name; and a map, its statements and its loop are a `sce-static`
/// statechart's alone.
#[test]
fn a_map_the_checker_does_not_read_is_refused_by_name() {
    refused(
        &edited(
            &faces(),
            "map&lt;uint32, uint64&gt;[16]",
            "map&lt;uint32, list&lt;uint8&gt;[2]&gt;[16]",
        ),
        "generate/unsupported-feature",
        "as the value of `map<uint32, list<uint8>[2]>[16]`",
    );
    for (statement, part) in [
        (
            "<sce:put target=\"m\" key=\"1\"/>",
            "a map or a set variable, and so the statements that write one, exist only under",
        ),
        (
            "<foreach array=\"m\" item=\"v\" sce:key=\"k\"/>",
            "a <foreach> binds the key of a map's entry only where a variable holds a map",
        ),
    ] {
        let document = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="s" datamodel="ecmascript" name="probe">
  <datamodel>
    <data id="m" expr="0"/>
  </datamodel>
  <state id="s">
    <onentry>{statement}</onentry>
  </state>
</scxml>
"#
        );
        refused(&document, "scxml/static-datamodel-rule", part);
    }
}
