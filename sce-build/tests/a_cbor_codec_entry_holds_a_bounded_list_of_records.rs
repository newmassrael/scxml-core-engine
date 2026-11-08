// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A CBOR codec's entry may hold a bounded list of records (docs/adr/0085,
//! Landing order step 3a): `list<record:R>[N]`, or `optional<…>` over one, read
//! by the one type grammar of docs/adr/0086, `R` an event schema the codec
//! imports. On the wire it is a definite-length array of maps, each a record
//! keyed by its fields' ids as text (docs/adr/0086, section 9.2).
//!
//! This holds what the build does with one, with no toolchain: the model the
//! parser reads, its refusals, the page that shows it and reads it back, the
//! Rust module that writes and reads it, and the refusal by name of every
//! backend that does not lower it yet. What the Rust module does with real
//! bytes is held by
//! `backends/rust/forge-conformance/tests/forge_cbor_record_list.rs`, and the
//! size the build counts by `forge::codec_size`'s unit tests.

use std::path::{Path, PathBuf};

use sce_build::forge::cbor_codec;
use sce_build::forge::model::{CodecModel, ForgeDocument};
use sce_build::forge::parser::parse_forge;
use sce_build::forge::{pseudo, unpseudo};
use sce_build::generator::Language;
use sce_build::{compile_forge_with_imports, DocumentLabel, ForgeCompileOptions};

const CODEC: &str = "codec_cbor_record_list";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cbor_codec")
}

fn fixture_text() -> String {
    std::fs::read_to_string(fixtures().join(format!("{CODEC}.scxml")))
        .unwrap_or_else(|e| panic!("read {CODEC}.scxml: {e}"))
}

/// The import of the fixture's record of a causal context, as `CausalEntry`.
const CAUSAL_IMPORT: &str =
    r#"  <sce:import kind="event-schema" src="cbor_causal_entry.scxml" as="CausalEntry"/>"#;

/// A CBOR codec of the fixture directory with the import lines `imports` and
/// the entries `entries`, verbatim; `root_attrs` follow the root's `name`.
fn document(root_attrs: &str, imports: &str, entries: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:encoding="cbor" sce:codec-format="2" name="probe_list"{root_attrs} version="1.0">
{imports}
  <datamodel>
    <data id="raw" sce:type="bytes" sce:direction="in"/>
{entries}
  </datamodel>
</scxml>
"#
    )
}

fn label(name: &str) -> DocumentLabel<'_> {
    DocumentLabel {
        identifier: name,
        diagnostic_label: name,
    }
}

fn codec(text: &str) -> CodecModel {
    match parse_forge(text, label("probe_list")) {
        Ok(Some(ForgeDocument::Codec(m))) => m,
        other => panic!("expected a codec, got {other:?}"),
    }
}

fn refusal(text: &str) -> String {
    match parse_forge(text, label("probe_list")) {
        Err(e) => e.to_string(),
        Ok(other) => panic!("expected a refusal, got {other:?}"),
    }
}

fn options() -> ForgeCompileOptions {
    ForgeCompileOptions {
        go_module_prefix: Some("example.com/sce-forge".to_string()),
        ..ForgeCompileOptions::default()
    }
}

/// What `lang` generates for `text`, a document of the fixture directory named
/// `name`: the text of every file, or the refusal.
fn generated(text: &str, name: &str, lang: Language) -> Result<String, String> {
    compile_forge_with_imports(text, label(name), lang, &fixtures(), &options())
        .map(|output| {
            output
                .files
                .into_iter()
                .map(|(_, code)| code)
                .collect::<Vec<_>>()
                .join("\n")
        })
        .map_err(|e| e.error.to_string())
}

fn rust_of(text: &str, name: &str) -> String {
    generated(text, name, Language::Rust).unwrap_or_else(|why| panic!("{name}: {why}"))
}

#[test]
fn a_list_of_records_is_read_by_the_one_type_grammar() {
    let m = codec(&fixture_text().replace(CODEC, "probe_list"));
    let entry = |id: &str| {
        m.cbor_entries
            .iter()
            .find(|e| e.id == id)
            .unwrap_or_else(|| panic!("no entry '{id}'"))
    };
    let seen = entry("seen").record_list().expect("`seen` holds records");
    assert_eq!((seen.record, seen.max, seen.min), ("CausalEntry", 2, 0));
    assert!(entry("seen").is_required());
    assert_eq!(entry("seen").leaf_type(), None);

    let context = entry("context")
        .record_list()
        .expect("`context` holds records");
    assert_eq!((context.record, context.max), ("CausalEntry", 3));
    assert!(!entry("context").is_required());

    // A leaf beside them is the entry it always was.
    assert!(entry("source").record_list().is_none());
    assert!(entry("source").leaf_type().is_some());
}

#[test]
fn a_list_of_anything_but_a_record_is_refused_with_the_kinds_an_entry_holds() {
    let why = refusal(&document(
        "",
        CAUSAL_IMPORT,
        r#"    <data id="n" sce:type="list&lt;uint8&gt;[4]" sce:key="0" sce:direction="out"/>"#,
    ));
    assert!(why.contains("a bounded list of records"), "{why}");
}

#[test]
fn a_list_with_no_bound_is_refused_as_every_unbounded_type_is() {
    let why = refusal(&document(
        "",
        CAUSAL_IMPORT,
        r#"    <data id="n" sce:type="list&lt;record:CausalEntry&gt;" sce:key="0" sce:direction="out"/>"#,
    ));
    assert!(why.contains("has no bound"), "{why}");
}

#[test]
fn a_record_no_event_schema_import_declares_is_refused() {
    let why = refusal(&document(
        "",
        CAUSAL_IMPORT,
        r#"    <data id="n" sce:type="list&lt;record:Missing&gt;[2]" sce:key="0" sce:direction="out"/>"#,
    ));
    assert!(why.contains("record:CausalEntry"), "{why}");
}

#[test]
fn a_size_attribute_on_a_list_is_refused_since_its_bound_is_in_its_type() {
    let why = refusal(&document(
        "",
        CAUSAL_IMPORT,
        r#"    <data id="n" sce:type="list&lt;record:CausalEntry&gt;[2]" sce:key="0" sce:max-size="2" sce:direction="out"/>"#,
    ));
    assert!(why.contains("sce:max-size"), "{why}");
    assert!(why.contains("its bound is written in its type"), "{why}");
}

#[test]
fn the_page_shows_a_list_of_records_and_reads_it_back() {
    let doc = ForgeDocument::Codec(codec(&fixture_text().replace(CODEC, "probe_list")));
    let page = pseudo::render(&doc).expect("a CBOR codec renders");
    assert!(
        page.contains("entry seen: list<record:CausalEntry>[2] key 3"),
        "{page}"
    );
    assert!(
        page.contains("entry context: optional<list<record:CausalEntry>[3]> key 4"),
        "{page}"
    );
    let back = unpseudo::parse(&page).expect("the page reads back");
    assert_eq!(
        unpseudo::ir_for_comparison(&back).expect("serializes"),
        unpseudo::ir_for_comparison(&doc).expect("serializes"),
        "the round trip changed the codec:\n{page}"
    );
}

#[test]
fn every_backend_generates_the_codec_exactly_where_it_lowers_a_list_of_records() {
    let text = fixture_text();
    for &lang in Language::ALL {
        match generated(&text, CODEC, lang) {
            Ok(_) => assert!(
                cbor_codec::lowers_record_list(lang),
                "{lang:?} generated a codec it does not lower"
            ),
            Err(why) => {
                assert!(
                    !cbor_codec::lowers_record_list(lang),
                    "{lang:?} lowers a list of records and refused the codec: {why}"
                );
                assert!(
                    why.contains("list of records") && why.contains("'seen'"),
                    "{lang:?}: {why}"
                );
            }
        }
    }
    assert!(cbor_codec::lowers_record_list(Language::Rust));
}

#[test]
fn the_rust_map_holds_each_list_in_the_bounded_list_of_the_schemas_payload() {
    let rust = rust_of(&fixture_text(), CODEC);
    for expected in [
        // The record is the payload type of the schema's own unit, brought in
        // as an enum's unit is; the codec declares no type of its own for it.
        "use super::cbor_causal_entry;",
        // The map holds each list at its bound, the optional one in an Option.
        "pub seen: ::sce_forge_runtime::codec::BoundedList<cbor_causal_entry::CborCausalEntryPayload, 2>,",
        "pub context: Option<::sce_forge_runtime::codec::BoundedList<cbor_causal_entry::CborCausalEntryPayload, 3>>,",
        // A read past the bound is refused before any record is read.
        "let len = cbor::read_array_len(&mut c)?;",
        "if len > 2 {",
        "if len > 3 {",
        "list.push(decode_record_causal_entry(&mut c)?)?;",
        // The record is read and written by functions private to the module.
        "fn decode_record_causal_entry(c: &mut SceCursor<'_>) -> Result<cbor_causal_entry::CborCausalEntryPayload, CodecError> {",
        "Ok(cbor_causal_entry::CborCausalEntryPayload {",
        "::sce_forge_runtime::codec::SceString::<16>::from_text(cbor::read_text(c, Some(16))?)?",
        // The write is the array head and each record's map.
        "let len = v.as_slice().len();",
        "cbor::write_array_head(w, ::sce_forge_runtime::codec::widen_usize(len))?;",
        "encode_record_causal_entry(w, record)?;",
    ] {
        assert!(rust.contains(expected), "missing `{expected}`:\n{rust}");
    }
    assert!(
        !rust.contains("struct CodecCborRecordListCausalEntry"),
        "{rust}"
    );
}

#[test]
fn a_record_writes_its_keys_in_the_order_their_encodings_sort() {
    let rust = rust_of(&fixture_text(), CODEC);
    let write = rust
        .split("fn encode_record_causal_entry")
        .nth(1)
        .expect("the record has an encode");
    let seq = write
        .find("cbor::write_text(w, \"seq\")?;")
        .expect("the record writes `seq`");
    let author = write
        .find("cbor::write_text(w, \"author\")?;")
        .expect("the record writes `author`");
    // `seq` is three bytes and `author` six, so `seq`'s key sorts first
    // (RFC 8949 section 4.2.1), whatever the order the schema declares them in.
    assert!(seq < author, "{write}");
    assert!(write.contains("cbor::write_map_head(w, 2)?;"), "{write}");
}

/// What `lang` generates for the fixture, or the panic that names why not.
fn generated_fixture(lang: Language) -> String {
    generated(&fixture_text(), CODEC, lang).unwrap_or_else(|why| panic!("{lang:?}: {why}"))
}

/// Each of `expected` is in `code`, and the record's write, the text after
/// `encode_at`, writes `keys[0]` (`seq`, three bytes) before `keys[1]`
/// (`author`, six): the order of their keys' encodings (RFC 8949 section
/// 4.2.1), whatever the order the schema declares them in.
fn holds(lang: Language, code: &str, expected: &[&str], encode_at: &str, keys: [&str; 2]) {
    for expected in expected {
        assert!(
            code.contains(expected),
            "{lang:?}: missing `{expected}`:\n{code}"
        );
    }
    let write = code
        .split(encode_at)
        .nth(1)
        .unwrap_or_else(|| panic!("{lang:?}: no `{encode_at}`:\n{code}"));
    let seq = write
        .find(keys[0])
        .unwrap_or_else(|| panic!("{lang:?}: the record writes no `seq`:\n{write}"));
    let author = write
        .find(keys[1])
        .unwrap_or_else(|| panic!("{lang:?}: the record writes no `author`:\n{write}"));
    assert!(
        seq < author,
        "{lang:?}: `seq` is written after `author`:\n{write}"
    );
}

#[test]
fn the_kotlin_map_holds_each_list_in_a_list_of_the_schemas_payload() {
    holds(
        Language::Kotlin,
        &generated_fixture(Language::Kotlin),
        &[
            "import com.sce.generated.cbor_causal_entry.*",
            "var seen: List<CborCausalEntryPayload> = emptyList(),",
            "var context: List<CborCausalEntryPayload>? = null,",
            // A read past the bound is refused before any record is read.
            "if (len > 2UL) return null",
            "if (len > 3UL) return null",
            "list.add(decodeRecordCausalEntry(c) ?: return null)",
            "held_author = Cbor.readText(c, 16) ?: return null",
            // The list carries no bound, so the encode refuses past it.
            "if (v.size > 2) return CodecError.CborOutOfRange",
            "Cbor.writeArrayHead(w, v.size.toULong())?.let { return it }",
            "encodeRecordCausalEntry(w, record)?.let { return it }",
            "if (record.author.encodeToByteArray().size > 16) return CodecError.CborOutOfRange",
            "Cbor.writeMapHead(w, 2UL)?.let { return it }",
        ],
        "private fun encodeRecordCausalEntry(",
        [
            "Cbor.writeText(w, \"seq\")",
            "Cbor.writeText(w, \"author\")",
        ],
    );
}

#[test]
fn the_swift_map_holds_each_list_in_an_array_of_the_schemas_payload() {
    holds(
        Language::Swift,
        &generated_fixture(Language::Swift),
        &[
            "public var seen: [CborCausalEntryPayload] = []",
            "public var context: [CborCausalEntryPayload]? = nil",
            // A read past the bound is refused before any record is read.
            "if len > 2 { throw CodecError.cborOutOfRange }",
            "if len > 3 { throw CodecError.cborOutOfRange }",
            "list.append(try CodecCborRecordList.decodeRecordCausalEntry(&reader))",
            "author_held = try Cbor.readText(&reader, maxSize: 16)",
            // The array carries no bound, so the encode refuses past it.
            "if value.count > 2 { throw CodecError.cborOutOfRange }",
            "Cbor.writeArrayHead(UInt64(value.count), to: &frame)",
            "try CodecCborRecordList.encodeRecordCausalEntry(record, to: &frame)",
            "if record.author.utf8.count > 16 { throw CodecError.cborOutOfRange }",
            "Cbor.writeMapHead(2, to: &frame)",
        ],
        "private static func encodeRecordCausalEntry(",
        [
            "Cbor.writeText(\"seq\", to: &frame)",
            "Cbor.writeText(\"author\", to: &frame)",
        ],
    );
}

#[test]
fn a_record_with_a_field_no_codec_writes_is_refused_by_name() {
    let text = document(
        "",
        r#"  <sce:import kind="event-schema" src="cbor_signed_entry.scxml" as="SignedEntry"/>"#,
        r#"    <data id="n" sce:type="list&lt;record:SignedEntry&gt;[2]" sce:key="0" sce:direction="out"/>"#,
    );
    let why = generated(&text, "probe_list", Language::Rust)
        .err()
        .unwrap_or_else(|| panic!("a record with an int32 field generated"));
    assert!(why.contains("whose field `delta` is `int32`"), "{why}");
}

#[test]
fn a_lower_count_is_read_and_its_write_refuses_fewer_records() {
    let entries = r#"    <data id="n" sce:type="list&lt;record:CausalEntry&gt;[1..2]" sce:key="0" sce:direction="out"/>"#;
    // A value of no record has no encoding, so the codec states its encode may
    // fail or is refused at the entry.
    let why = generated(
        &document("", CAUSAL_IMPORT, entries),
        "probe_list",
        Language::Rust,
    )
    .err()
    .unwrap_or_else(|| panic!("an undeclared refusal generated"));
    assert!(why.contains("lower count 1"), "{why}");

    let rust = rust_of(
        &document(r#" sce:encode-may-fail="true""#, CAUSAL_IMPORT, entries),
        "probe_list",
    );
    assert!(rust.contains("if len > 2 || len < 1 {"), "{rust}");
    // The write refuses a value of fewer records than the lower count.
    assert!(rust.contains("if len < 1 {"), "{rust}");
}

/// The import of a record whose field is a byte string of at most four bytes.
const DIGEST_IMPORT: &str =
    r#"  <sce:import kind="event-schema" src="cbor_digest_entry.scxml" as="DigestEntry"/>"#;

/// A list of at most two records of `cbor_digest_entry`.
const DIGEST_ENTRY: &str = r#"    <data id="n" sce:type="list&lt;record:DigestEntry&gt;[2]" sce:key="0" sce:direction="out"/>"#;

#[test]
fn a_record_byte_string_is_held_to_its_bound_by_the_encode() {
    // The schema's Rust unit holds the byte string in `Vec<u8>`, which carries
    // no bound, so a record past it has no encoding: the codec states its
    // encode may fail or is refused at the entry.
    let why = generated(
        &document("", DIGEST_IMPORT, DIGEST_ENTRY),
        "probe_list",
        Language::Rust,
    )
    .err()
    .unwrap_or_else(|| panic!("an undeclared refusal generated"));
    assert!(
        why.contains("whose byte string `digest` is past its bound 4"),
        "{why}"
    );

    let rust = rust_of(
        &document(
            r#" sce:encode-may-fail="true""#,
            DIGEST_IMPORT,
            DIGEST_ENTRY,
        ),
        "probe_list",
    );
    for expected in [
        "if record.digest.len() > 4 {",
        "cbor::read_bytes(c, Some(4))?.to_vec()",
        "cbor::write_bytes(w, &record.digest)?;",
    ] {
        assert!(rust.contains(expected), "missing `{expected}`:\n{rust}");
    }
}

#[test]
fn c11_refuses_a_record_whose_byte_string_its_unit_borrows() {
    let why = generated(
        &document(
            r#" sce:encode-may-fail="true""#,
            DIGEST_IMPORT,
            DIGEST_ENTRY,
        ),
        "probe_list",
        Language::C11,
    )
    .err()
    .unwrap_or_else(|| panic!("C11 generated a list of records it cannot own"));
    // Refused before any backend renders, by the record and its field.
    assert!(
        why.contains("field `digest` is a byte string") && why.contains("borrowed pointer"),
        "{why}"
    );
}
