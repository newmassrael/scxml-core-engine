// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A statechart's map or set variable (docs/adr/0108, D1 to D5), judged by the
//! `sce-static` checker: `check` accepts a well-formed document
//! (`fixtures/map/map_faces.scxml`, which writes every read, statement and loop
//! the checker reads once), and refuses each read, statement and loop that is
//! no use of a map by the code D11 names, saying what is accepted. The
//! Interpreter's lowering, the first to hold a map ("Order of work", step 4),
//! lowers the document, Rust's `generate`, the second (step 5), and C11's, the
//! third (step 6), generate it, and every other generated lane's `generate`
//! refuses it by name, naming the record that lands a map lane by lane (D10).
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
/// statement and loop of a map the checker reads is accepted by `check`,
/// refused by name by every lane that generates a statechart but Rust and C11,
/// and lowered by the Interpreter's lowering, every statement of a map replaced
/// and every read of one a call of the library.
#[test]
fn a_map_document_is_checked_and_refused_by_every_lane_by_name() {
    let path = fixtures().join("map_faces.scxml");
    let (ok, out) = run(&["check"], &path);
    assert!(ok, "check accepts the statechart:\n{out}");
    let (ok, out) = check(&faces());
    assert!(ok, "check accepts it beside copies of its schemas:\n{out}");

    let place = "`map<record:PeerKey, record:Face>[face_max=4]` in a statechart's variable";
    for &language in sce_build::generator::Language::ALL {
        // Rust holds a map ("Order of work", step 5), and C11 (step 6), as the
        // tests below hold.
        if matches!(
            language,
            sce_build::generator::Language::Rust | sce_build::generator::Language::C11
        ) {
            continue;
        }
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

    // The Interpreter's lowering writes the document, which holds the
    // statechart's every map as the array of its entries.
    let lowered =
        sce_build::forge::static_js::lower_file(path.to_str().expect("a utf-8 path"), Vec::new())
            .unwrap_or_else(|refusal| panic!("the Interpreter lowers a map: {refusal:?}"));
    for held in [
        r#"<data id="faces" expr="[]"/>"#,
        "SceStatic.mapPut(faces, {key: {record: [[&apos;low&apos;, &apos;integer&apos;], \
         [&apos;high&apos;, &apos;integer&apos;]]}}, 4, peer, face)",
        "SceStatic.mapHas(faces, ",
        "SceStatic.mapGet(faces, ",
        "SceStatic.mapRemove(faces, ",
        "SceStatic.mapPutOrFail(byId, ",
        "faces.length",
        "f[1].id",
        "f[0].low",
    ] {
        assert!(lowered.contains(held), "no `{held}` in:\n{lowered}");
    }
    // Asked of the elements, not the text: the fixture's own comment, which the
    // lowering keeps as the author wrote it, names each statement.
    const SCE: &str = "http://sce.dev/ext";
    let document = roxmltree::Document::parse(&lowered).expect("the lowered document is XML");
    for element in document.descendants().filter(|node| node.is_element()) {
        let name = element.tag_name();
        assert!(
            !(name.namespace() == Some(SCE) && ["put", "remove", "clear"].contains(&name.name())),
            "<sce:{}> is left in the document the Interpreter runs:\n{lowered}",
            name.name()
        );
        assert!(
            element.attribute((SCE, "key")).is_none(),
            "a <{}> keeps its sce:key in the document the Interpreter runs:\n{lowered}",
            name.name()
        );
    }
}

/// The Rust lowering of `document`, written beside the schemas it imports:
/// whether `generate` passed, what it printed, and every Rust file it wrote,
/// joined.
fn generate_rust(document: &str) -> (bool, String, String) {
    let dir = tempfile::tempdir().expect("a scratch directory");
    for name in SIBLINGS {
        std::fs::copy(fixtures().join(name), dir.path().join(name)).expect("copy a schema");
    }
    let path = dir.path().join("map_faces.scxml");
    std::fs::write(&path, document).expect("write the statechart");
    let out_dir = tempfile::tempdir().expect("an output directory");
    let out = out_dir.path().to_str().expect("a utf-8 path").to_string();
    let (ok, printed) = run(&["generate", "-l", "rust", "-o", out.as_str()], &path);
    let mut code = String::new();
    for entry in std::fs::read_dir(out_dir.path()).expect("read the output") {
        let path = entry.expect("an output file").path();
        if path.extension().is_some_and(|ext| ext == "rs") {
            code.push_str(&std::fs::read_to_string(&path).expect("read a generated file"));
        }
    }
    (ok, printed, code)
}

/// docs/adr/0108, "Order of work", step 5: Rust generates the document, each
/// map held in the runtime's `BoundedMap` or `BoundedSet` built under a seed,
/// each read a call of `portable::map`'s, each record key and the payload a key
/// is looked up by made a key, and each statement a write of the map. D6 item 3:
/// a `<foreach>` whose body does not write the map it walks walks the map
/// itself, and one whose body does walks a copy.
#[test]
fn rust_holds_a_map_and_copies_one_only_for_a_loop_that_writes_it() {
    let (ok, printed, code) = generate_rust(&faces());
    assert!(ok, "Rust generates a map:\n{printed}");
    for held in [
        "::sce_rust_runtime::BoundedMap<",
        "::sce_rust_runtime::BoundedSet<u32, 4>",
        "::sce_rust_runtime::MapSeeds::new(::sce_rust_runtime::map_seed())",
        "::sce_rust_runtime::FromSeed::from_seed(sce_map_seeds.draw())",
        "::sce_rust_runtime::portable::map::holds(",
        "::sce_rust_runtime::portable::map::count(",
        "::sce_rust_runtime::portable::map::value(",
        "::sce_rust_runtime::portable::map::value_cloned(",
        "::sce_rust_runtime::BoundedMap::widened_map(",
        "impl ::sce_rust_runtime::MapKey for",
        "::sce_rust_runtime::PutOutcome::Full",
        "::sce_rust_runtime::RemoveOutcome::Removed",
        ".first();",
        ".next_after(",
    ] {
        assert!(
            code.contains(held),
            "no `{held}` in the generated Rust:\n{code}"
        );
    }
    // No loop of the document writes the map it walks, and the walk of
    // `faces`, whose body reaches the machine through its fields alone, lends
    // each face where the map holds it.
    assert!(!code.contains("sce_walk_"), "a loop copies a map:\n{code}");
    assert!(
        code.contains("let f = self.faces.value_at(sce_at_0);"),
        "the walk of `faces` lends no face:\n{code}"
    );

    // docs/adr/0108, D6: a body with a statement that takes the machine whole
    // (a `<send>`) beside an `<assign>` is handed a clone of each face, so the
    // map is not borrowed while the machine is.
    let sends = edited(
        &faces(),
        "<assign location=\"walked\" expr=\"i\"/>",
        "<assign location=\"walked\" expr=\"i\"/>\n        <send event=\"face.walked\"/>",
    );
    let (ok, printed, code) = generate_rust(&sends);
    assert!(ok, "Rust generates a walk whose body sends:\n{printed}");
    assert!(
        code.contains("let f = self.faces.value_at(sce_at_0).clone();"),
        "the walk of `faces` whose body sends lends its face:\n{code}"
    );

    // The walk of `seen` that removes each key it meets walks a copy of it.
    let writes_its_map = edited(
        &faces(),
        "<sce:put target=\"counts\" key=\"s\" expr=\"total\"/>",
        "<sce:remove target=\"seen\" key=\"s\"/>",
    );
    let (ok, printed, code) = generate_rust(&writes_its_map);
    assert!(ok, "Rust generates a loop that writes its map:\n{printed}");
    assert!(
        code.contains("let sce_walk_0 = self.seen.clone();"),
        "the loop that writes `seen` walks no copy of it:\n{code}"
    );
}

/// docs/adr/0108, D1 and D8: Rust holds a map keyed by a byte string and one
/// whose value is a byte string, and a set of byte strings, and its saved state
/// writes and reads each byte string as its Latin-1 text, as it does a
/// byte-string variable (§2.15, "A byte string"), the rest by their own form.
/// The runtime holds that a zero byte and a byte above 0x7f cross whole
/// (`saved_state::tests::a_byte_string_in_a_map_is_saved_as_its_latin1_text`).
#[test]
fn rust_saves_a_byte_string_of_a_map_as_its_latin1_text() {
    let document = edited(
        &faces(),
        "<data id=\"order\"",
        "<data id=\"blobs\" sce:type=\"map&lt;bytes[16], bytes[4]&gt;[2]\"/>\n    \
         <data id=\"tags\" sce:type=\"map&lt;bytes[16], uint8&gt;[2]\"/>\n    \
         <data id=\"ids\" sce:type=\"set&lt;bytes[16]&gt;[2]\"/>\n    <data id=\"order\"",
    );
    let (ok, printed, code) = generate_rust(&document);
    assert!(ok, "Rust generates a map of byte strings:\n{printed}");
    let bytes_writer = "|bytes| ::sce_rust_runtime::saved_state::bytes_to_saved(bytes)";
    let bytes_reader = "::sce_rust_runtime::saved_state::bounded_bytes";
    let own_writer = "::sce_rust_runtime::saved_state::SavedValue::to_saved";
    let own_reader = "::sce_rust_runtime::saved_state::SavedValue::from_saved";
    for held in [
        "::sce_rust_runtime::BoundedMap<::sce_rust_runtime::portable::SceBytes<16>, \
         ::sce_rust_runtime::portable::SceBytes<4>, 2>"
            .to_string(),
        format!(
            "::sce_rust_runtime::saved_state::map_to_saved(&policy.blobs, {bytes_writer}, \
             {bytes_writer})"
        ),
        format!(
            "::sce_rust_runtime::saved_state::map_from_saved(saved.variable(\"blobs\")?, \
             \"blobs\", {bytes_reader}, {bytes_reader})?"
        ),
        format!(
            "::sce_rust_runtime::saved_state::map_to_saved(&policy.tags, {bytes_writer}, \
             {own_writer})"
        ),
        format!(
            "::sce_rust_runtime::saved_state::map_from_saved(saved.variable(\"tags\")?, \
             \"tags\", {bytes_reader}, {own_reader})?"
        ),
        format!("::sce_rust_runtime::saved_state::set_to_saved(&policy.ids, {bytes_writer})"),
        format!(
            "::sce_rust_runtime::saved_state::set_from_saved(saved.variable(\"ids\")?, \
             \"ids\", {bytes_reader})?"
        ),
        // A map of no byte string writes each part by its own form.
        format!(
            "::sce_rust_runtime::saved_state::map_to_saved(&policy.counts, {own_writer}, \
             {own_writer})"
        ),
    ] {
        assert!(
            code.contains(&held),
            "no `{held}` in the generated Rust:\n{code}"
        );
    }
}

/// The C11 lowering of `document`, written beside the schemas it imports:
/// whether `generate` passed, what it printed, and every header and source it
/// wrote, joined.
fn generate_c11(document: &str) -> (bool, String, String) {
    let dir = tempfile::tempdir().expect("a scratch directory");
    for name in SIBLINGS {
        std::fs::copy(fixtures().join(name), dir.path().join(name)).expect("copy a schema");
    }
    let path = dir.path().join("map_faces.scxml");
    std::fs::write(&path, document).expect("write the statechart");
    let out_dir = tempfile::tempdir().expect("an output directory");
    let out = out_dir.path().to_str().expect("a utf-8 path").to_string();
    let (ok, printed) = run(&["generate", "-l", "c11", "-o", out.as_str()], &path);
    let mut code = String::new();
    let mut files: Vec<PathBuf> = std::fs::read_dir(out_dir.path())
        .expect("read the output")
        .map(|entry| entry.expect("an output file").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "h" || ext == "c"))
        .collect();
    files.sort();
    for path in files {
        code.push_str(&std::fs::read_to_string(&path).expect("read a generated file"));
    }
    (ok, printed, code)
}

/// docs/adr/0108, "Order of work", step 6: C11 generates the document, each map
/// and set the type `sce/forge/map.h` declares for its parts and its bound,
/// started under a seed drawn from the one every construction takes; each read a
/// call of the header's over the map's address with a probe of the key, a record
/// key and the payload taken whole each by the function that writes its
/// canonical bytes; each statement a write of the map; a whole map taken into
/// another's storage; and the saved form written and read by each map's type.
/// D6 item 3: a `<foreach>` whose body does not write the map it walks walks the
/// map itself, and one whose body does walks a copy the machine holds.
#[test]
fn c11_holds_a_map_and_copies_one_only_for_a_loop_that_writes_it() {
    let (ok, printed, code) = generate_c11(&faces());
    assert!(ok, "C11 generates a map:\n{printed}");
    for held in [
        "#include \"sce/forge/map.h\"",
        "SCE_MAP_DECLARE(sce_static_map_uint32_uint64_8, uint32_t, uint64_t, 8u, \
         sce_static_map_uint32_uint64_8_key_feed)",
        "SCE_MAP_DECLARE(sce_static_map_uint32_uint64_16, uint32_t, uint64_t, 16u, \
         sce_static_map_uint32_uint64_16_key_feed)",
        "SCE_SET_DECLARE(sce_static_set_uint32_4, uint32_t, 4u, sce_static_set_uint32_4_key_feed)",
        "sce_map_key_text(key, sink);",
        "static inline void map_faces_record_peer_key_map_key(const void *key, sce_map_sink_t *sink)",
        "static inline void map_faces_peer_seen_payload_map_key(const void *key, sce_map_sink_t *sink)",
        "sce_map_seed_t map_seed",
        "sce_map_seed_derive(map_seed, 0u)",
        "sce_map_holds(&(sm->policy.v_faces), &(sm->policy.v_peer), NULL)",
        "map_faces_peer_seen_payload_map_key)",
        "sce_map_count(&(sm->policy.v_faces))",
        "sce_map_lookup_take(&sce_failure_, sce_map_lookup(&(sm->policy.v_counts), SCE_MAP_KEY_UINT(",
        "SCE_MAP_KEY_TEXT(",
        "SCE_MAP_VALUE_OF(sm->policy.v_faces, ",
        "sce_map_assign(&(sm->policy.v_spare), &(sm->policy.v_counts));",
        "sce_map_clear(&(sm->policy.v_flags));",
        "SCE_MAP_PUT_FULL",
        "SCE_MAP_REMOVE_REMOVED",
        "sce_static_map_uint32_uint64_8_t_saved_write(w, &",
        "sce_static_map_uint32_uint64_8_t_saved_read(v, what, at, out, err)",
        "sce_map_first(&(sm->policy.v_faces))",
    ] {
        assert!(
            code.contains(held),
            "no `{held}` in the generated C11:\n{code}"
        );
    }
    // No loop of the document writes the map it walks.
    assert!(
        !code.contains("sce_map_copy_for_walk("),
        "a loop copies a map:\n{code}"
    );

    // The walk of `seen` that removes each key it meets walks a copy of it, held
    // in the machine's scratch at the loop's depth.
    let writes_its_map = edited(
        &faces(),
        "<sce:put target=\"counts\" key=\"s\" expr=\"total\"/>",
        "<sce:remove target=\"seen\" key=\"s\"/>",
    );
    let (ok, printed, code) = generate_c11(&writes_its_map);
    assert!(ok, "C11 generates a loop that writes its map:\n{printed}");
    for held in [
        "sce_map_copy_for_walk(sce_walk_",
        "&sm->scratch.loop_0.of_sce_static_set_uint32_4",
        "sce_static_set_uint32_4_t of_sce_static_set_uint32_4;",
    ] {
        assert!(
            code.contains(held),
            "no `{held}` in the generated C11:\n{code}"
        );
    }
}

/// docs/adr/0108, D8: a C11 machine holding a map takes its seed from the host
/// that constructs it, and neither a parent nor a test component's runner hands
/// one yet, so a document that invokes a child holding a map is refused by name
/// on C11, naming the record.
#[test]
fn c11_refuses_a_child_that_holds_a_map_by_name() {
    let dir = tempfile::tempdir().expect("a scratch directory");
    for name in SIBLINGS {
        std::fs::copy(fixtures().join(name), dir.path().join(name)).expect("copy a schema");
    }
    std::fs::write(dir.path().join("map_faces.scxml"), faces()).expect("write the child");
    let parent = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="running" datamodel="sce-static" name="map_parent">
  <state id="running">
    <invoke id="faces" type="scxml" src="map_faces.scxml"/>
  </state>
</scxml>
"#;
    let path = dir.path().join("map_parent.scxml");
    std::fs::write(&path, parent).expect("write the parent");
    let out_dir = tempfile::tempdir().expect("an output directory");
    let out = out_dir.path().to_str().expect("a utf-8 path").to_string();
    let (ok, printed) = run(&["generate", "-l", "c11", "-o", out.as_str()], &path);
    assert!(
        !ok,
        "C11 generates a parent of a child that holds a map:\n{printed}"
    );
    assert!(
        printed.contains("\"code\":\"generate/unsupported-feature\"")
            && printed.contains("of a child that holds a map or a set")
            && printed.contains("(docs/adr/0108, D10)"),
        "{printed}"
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
            "cond=\"putResult === PutOutcome.full\"",
            "cond=\"counts === spare\"",
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
