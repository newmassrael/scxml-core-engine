// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
    let seen = cbor_codec::record_list(entry("seen")).expect("`seen` holds records");
    assert_eq!((seen.record, seen.max, seen.min), ("CausalEntry", 2, 0));
    assert!(entry("seen").is_required());
    assert_eq!(entry("seen").leaf_type(), None);

    let context = cbor_codec::record_list(entry("context")).expect("`context` holds records");
    assert_eq!((context.record, context.max), ("CausalEntry", 3));
    assert!(!entry("context").is_required());

    // A leaf beside them is the entry it always was.
    assert!(cbor_codec::record_list(entry("source")).is_none());
    assert!(entry("source").leaf_type().is_some());
}

#[test]
fn a_list_of_anything_but_a_record_is_read_and_refused_by_name_where_it_is_generated() {
    // docs/adr/0101, section 1: a list of integers is a value of the language.
    // A backend that prints every entry type (Rust and C++, Landing order
    // steps 4a and 4b) generates it, holding it in its list of the bound; every
    // other one prints only the shapes of docs/adr/0085 step 3a, and refuses
    // the codec by name (section 5.4), the entry and the list named in the one
    // clause every refusal of the kind says.
    let text = document(
        "",
        CAUSAL_IMPORT,
        r#"    <data id="n" sce:type="list&lt;uint8&gt;[4]" sce:key="0" sce:direction="out"/>"#,
    );
    let m = codec(&text);
    assert!(cbor_codec::record_list(&m.cbor_entries[0]).is_none());
    let clause = "codec 'probe_list': CBOR entry 'n' holds `list<uint8>[4]`, a type of the \
                  language whose list the printer does not lower in a CBOR codec yet";
    let mut refused = 0;
    for &lang in Language::ALL {
        match generated(&text, "probe_list", lang) {
            Ok(code) => {
                assert!(
                    cbor_codec::lowers_every_entry_type(lang),
                    "{lang:?} generated a list of integers it does not lower"
                );
                let held = match lang {
                    Language::Rust => "pub n: ::sce_forge_runtime::codec::BoundedList<u8, 4>,",
                    Language::Cpp => "::SCE::Forge::OwnedList<uint8_t, 4> n_{};",
                    other => {
                        panic!("{other:?} prints a list of integers this test names no type of")
                    }
                };
                assert!(code.contains(held), "{lang:?}:\n{code}");
            }
            Err(why) => {
                assert!(
                    !cbor_codec::lowers_every_entry_type(lang),
                    "{lang:?} lowers every entry type and refused: {why}"
                );
                assert!(why.contains(clause), "{lang:?}: {why}");
                refused += 1;
            }
        }
    }
    assert!(cbor_codec::lowers_every_entry_type(Language::Rust));
    assert!(cbor_codec::lowers_every_entry_type(Language::Cpp));
    // Every backend but the reference and C++ still refuses it.
    assert_eq!(refused, Language::ALL.len() - 2);
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
        "list.push(decode_record_cbor_causal_entry(&mut c)?)?;",
        // The record is read and written by functions private to the module,
        // named for the schema's document (docs/adr/0101, section 5.1).
        "fn decode_record_cbor_causal_entry(c: &mut SceCursor<'_>) -> Result<cbor_causal_entry::CborCausalEntryPayload, CodecError> {",
        "Ok(cbor_causal_entry::CborCausalEntryPayload {",
        "::sce_forge_runtime::codec::SceString::<16>::from_text(cbor::read_text(c, Some(16))?)?",
        // The write is the array head and each record's map.
        "cbor::write_array_head(w, ::sce_forge_runtime::codec::widen_usize(v.as_slice().len()))?;",
        "for item in v.as_slice() {",
        "encode_record_cbor_causal_entry(w, item)?;",
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
        .split("fn encode_record_cbor_causal_entry")
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

/// The encodes of a printed C++ map: its `Wire` struct and each record's
/// write. The decodes beside them refuse a frame by the same kinds
/// (docs/adr/0099, amendment of 2026-11-16), which say nothing of an encode.
fn cpp_encodes(code: &str) -> String {
    let mut encodes = String::new();
    for open in ["struct Wire {", "encode_record_"] {
        let mut rest = code;
        while let Some(start) = rest.find(open) {
            let body = &rest[start..];
            // A record's write ends where the record's read begins, the
            // `Wire` struct where it closes: whichever comes first.
            let end = ["static ::SCE::Forge::Decoded", "\n};"]
                .iter()
                .filter_map(|stop| body.find(stop))
                .min()
                .unwrap_or(body.len());
            encodes.push_str(&body[..end]);
            rest = &body[end..];
        }
    }
    encodes
}

#[test]
fn the_cpp_map_is_printed_with_its_lists_of_records_that_hold_a_text() {
    // `cbor_causal_entry`'s `author` is a `string[16]`, which the schema's C++
    // unit holds in the `OwnedText<16>` of its bound (docs/adr/0086, section
    // 7.4): the list of the records holds it within its bound, so the map is
    // printed from its plan and its encode refuses nothing, as on Rust. Red if
    // the printer refuses such a list again.
    let unlowered = sce_build::cpp_codec_unlowered_file(
        &fixtures().join(format!("{CODEC}.scxml")),
        &[],
        &options(),
    )
    .unwrap_or_else(|e| panic!("{CODEC}: {e:?}"))
    .expect("the fixture is a codec");
    assert!(unlowered.is_empty(), "{CODEC}: {unlowered:?}");

    let code = generated_fixture(Language::Cpp);
    holds(
        Language::Cpp,
        &code,
        &[
            "#include \"cbor_causal_entry.h\"",
            // The list is held in the type a schema unit holds it in
            // (docs/adr/0101, section 5.3), and lent.
            "::SCE::Forge::OwnedList<SCE::Generated::CborCausalEntry::CborCausalEntryPayload, 2> \
             seen_{};",
            "[[nodiscard]] const ::SCE::Forge::OwnedList<SCE::Generated::CborCausalEntry::CborCausalEntryPayload, 2>& \
             seen() const noexcept { return seen_; }",
            // A read past the bound is refused before any record is read.
            "if (*_len0 > 2) return ::SCE::Forge::CodecError::CborOutOfRange;",
            "if (*_len0 > 3) return ::SCE::Forge::CodecError::CborOutOfRange;",
            // The record is read by the one function named for its document
            // (docs/adr/0101, section 5.1).
            "auto _r1 = Composites::decode_record_cbor_causal_entry(c);",
            // The text is read within its bound and held in the field's type.
            "auto _r0 = cbor::read_text(c, 16);",
            "auto _b0 = ::SCE::Forge::OwnedText<16>::make(std::move(*_r0));",
            "held_author = std::move(*_b0);",
            // The write is the array head and each record's map; the types
            // hold every bound, so nothing is checked.
            "if (auto _e = cbor::write_array_head(w, static_cast<std::uint64_t>(v.size())); _e) return _e;",
            "if (auto _e = Composites::encode_record_cbor_causal_entry(w, _item0); _e) return _e;",
            "if (auto _e = cbor::write_text(w, record.author.inner()); _e) return _e;",
            "if (auto _e = cbor::write_map_head(w, 2); _e) return _e;",
        ],
        "encode_record_cbor_causal_entry(::SCE::Forge::SceSink& w",
        [
            "cbor::write_text(w, \"seq\")",
            "cbor::write_text(w, \"author\")",
        ],
    );
    assert!(!cpp_encodes(&code).contains("CborOutOfRange"), "{code}");
}

/// The repository's standard Mesh documents, whose envelope holds a causal
/// route's vector as a list of `causal_entry` records (docs/adr/0085, section
/// 3.2), each record three unsigned integers.
fn stdlib_mesh() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../stdlib/mesh")
}

#[test]
fn the_cpp_envelope_is_printed_with_its_vector_lent_in_the_list_of_the_schemas_payload() {
    let path = stdlib_mesh().join("envelope.scxml");
    let unlowered = sce_build::cpp_codec_unlowered_file(&path, &[], &options())
        .unwrap_or_else(|e| panic!("envelope: {e:?}"))
        .expect("the envelope is a codec");
    // Red if the printer does not lower a list of records of unsigned integers.
    assert!(unlowered.is_empty(), "{unlowered:?}");

    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read envelope: {e}"));
    let code = compile_forge_with_imports(
        &text,
        label("envelope"),
        Language::Cpp,
        &stdlib_mesh(),
        &options(),
    )
    .map(|output| {
        output
            .files
            .into_iter()
            .map(|(_, code)| code)
            .collect::<Vec<_>>()
            .join("\n")
    })
    .unwrap_or_else(|e| panic!("envelope: {}", e.error));
    let list = "::SCE::Forge::OwnedList<SCE::Generated::CausalEntry::CausalEntryPayload, 64>";
    let accessor = format!(
        "const std::optional<{list}>& causal_vector() const noexcept {{ return causal_vector_; }}"
    );
    let held = format!("std::optional<{list}> causal_vector_{{}};");
    let made = format!("{list} _list0;");
    holds(
        Language::Cpp,
        &code,
        &[
            // The value holds the list in the type a schema unit holds it in
            // (docs/adr/0101, section 5.3), and lends it.
            accessor.as_str(),
            held.as_str(),
            // A read past the bound is refused before any record is read, and
            // each record read is appended to the list of that bound.
            "if (*_len0 > 64) return ::SCE::Forge::CodecError::CborOutOfRange;",
            made.as_str(),
            "auto _r1 = Composites::decode_record_causal_entry(c);",
            "if (!_list0.push(std::move(_item0))) return ::SCE::Forge::CodecError::CborOutOfRange;",
            "held_seq = std::move(*_r0);",
            // The write is the array head and each record's map; the list
            // holds the count, so nothing is checked.
            "if (auto _e = cbor::write_array_head(w, static_cast<std::uint64_t>(v.size())); _e) return _e;",
            "if (auto _e = Composites::encode_record_causal_entry(w, _item0); _e) return _e;",
            "if (auto _e = cbor::write_map_head(w, 3); _e) return _e;",
        ],
        "encode_record_causal_entry(::SCE::Forge::SceSink& w",
        [
            "cbor::write_text(w, \"lo\")",
            "cbor::write_text(w, \"seq\")",
        ],
    );
    // The template's struct, whose encode refuses a count past the bound, is
    // not printed beside it.
    assert!(!cpp_encodes(&code).contains("CborOutOfRange"), "{code}");
}

/// docs/adr/0081 step 4: the Kotlin map is printed from its plan. Each list is
/// held in the value class of its most (`{Codec}{Entry}List`), and the payload
/// type of the schema's Kotlin unit holds a record's text in a bare `String`,
/// so the encode refuses a record whose text is past its bound before the map
/// head, as it refuses one past a byte string's.
///
/// Turns red where `kotlin_source::cbor::checks` checks a record after the
/// first byte or drops the check of a record's text, or where `held` holds a
/// list in a bare `List`.
#[test]
fn the_kotlin_map_holds_each_list_in_a_list_of_the_schemas_payload() {
    holds(
        Language::Kotlin,
        &generated_fixture(Language::Kotlin),
        &[
            "import com.sce.generated.cbor_causal_entry.*",
            "value class CodecCborRecordListSeenList private constructor(\
             val entries: List<CborCausalEntryPayload>) : com.sce.forge.runtime.Counted {",
            "        const val MOST: Int = 2\n",
            "override var seen: CodecCborRecordListSeenList = CodecCborRecordListSeenList.fewest(),",
            "override var context: CodecCborRecordListContextList? = null,",
            // A read past the bound is refused before any record is read.
            "if (len > 2UL) return Decoded.Refused(CodecError.CborOutOfRange)",
            "if (len > 3UL) return Decoded.Refused(CodecError.CborOutOfRange)",
            "list.add(decodeRecordCausalEntry(c).valueOr { _refused -> return _refused })",
            "held_author = Cbor.readText(c, 16).valueOr { _refused -> return _refused }",
            // A required list the map lacks is refused by its kind, and one
            // its class does not hold as the reader refuses it.
            "seen = CodecCborRecordListSeenList.of(seen ?: return \
             Decoded.Refused(CodecError.CborRequiredKeyMissing)) ?: return \
             Decoded.Refused(CodecError.CborOutOfRange),",
            // Each record is held to the bound its payload does not hold
            // before the map head is written.
            "            for (_record in _e_seen.entries) {\n                \
             if ((com.sce.forge.runtime.Utf8.size(_record.author) ?: return \
             CodecError.CborOutOfRange) > 16) return CodecError.CborOutOfRange\n            }\n",
            "            if (_e_context != null) {\n                \
             for (_record in _e_context.entries) {\n",
            "            var count = 3UL\n            if (_e_context != null) count++\n",
            "Cbor.writeArrayHead(w, _e_seen.size.toULong())?.let { return it }",
            "                encodeRecordCausalEntry(w, _record)?.let { return it }\n",
            "Cbor.writeMapHead(w, 2UL)?.let { return it }",
            "Cbor.writeUint(w, record.seq)?.let { return it }",
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
fn the_c11_map_holds_each_list_in_a_fixed_array_of_the_schemas_payload() {
    let c = generated_fixture(Language::C11);
    holds(
        Language::C11,
        &c,
        &[
            "#include \"cbor_causal_entry.h\"",
            "uint32_t len;",
            "CborCausalEntryPayload_t items[2];",
            "CborCausalEntryPayload_t items[3];",
            // A read past the bound is refused before any record is read.
            "if (raw > 2U) {",
            "if (raw > 3U) {",
            "s = codec_cbor_record_list_decode_record_causal_entry(&c, &v->seen.items[item]);",
            "s = sce_forge_cbor_read_text(c, 16U, 16U, out->author.data, &text_len);",
            // The length is a number the array does not bound, so the encode
            // refuses one past it.
            "if (self->seen.len > 2U) {",
            "s = sce_forge_cbor_write_array_head(w, (uint64_t)self->seen.len);",
            "s = codec_cbor_record_list_encode_record_causal_entry(&self->seen.items[item], w);",
            "if (record->author.data[16] != '\\0') {",
        ],
        "static inline sce_forge_codec_status_t codec_cbor_record_list_encode_record_causal_entry(",
        [
            "sce_forge_cbor_write_text(w, \"seq\", 3U)",
            "sce_forge_cbor_write_text(w, \"author\", 6U)",
        ],
    );
    // The most the map writes counts each list at its bound, by C(T): the map
    // head 1, `id` 1 + 17, `source` 1 + 2 + 64, `seen` 1 + 1 + 2 * 38 and
    // `context` 1 + 1 + 3 * 38, a record being its head 1, "author" 7 and its
    // text 17, "seq" 4 and its number 9: 1 + 18 + 67 + 78 + 116 = 280.
    assert!(c.contains("_MAX_BYTES 280U"), "{c}");
}

#[test]
fn the_go_map_holds_each_list_in_a_slice_of_the_schemas_payload() {
    holds(
        Language::Go,
        &generated_fixture(Language::Go),
        &[
            "\"example.com/sce-forge/cbor_causal_entry\"",
            ".CborCausalEntryPayload\n",
            // A read past the bound is refused before any record is read.
            "if n > 2 {",
            "if n > 3 {",
            "record, err := decodeCodecCborRecordListRecordCausalEntry(c)",
            "v, err := codec.CborReadText(c, 16)",
            "record.Author = v",
            // The slice carries no bound, so the encode refuses past it.
            "if len(v) > 2 {",
            "if err := codec.CborWriteArrayHead(w, uint64(len(v))); err != nil {",
            "if err := encodeCodecCborRecordListRecordCausalEntry(w, &v[item]); err != nil {",
            "if len(record.Author) > 16 {",
            "if err := codec.CborWriteMapHead(w, 2); err != nil {",
        ],
        "func encodeCodecCborRecordListRecordCausalEntry(",
        [
            "codec.CborWriteText(w, \"seq\")",
            "codec.CborWriteText(w, \"author\")",
        ],
    );
}

/// docs/adr/0081 step 4: the Python map is printed from its plan. Each list is
/// held in the class of its most (`BoundedTuple`), and the payload type of the
/// schema's Python unit holds a record's text and integer in a bare `str` and
/// `int`, so the encode refuses a record past either bound before the map
/// head, as it refuses one past a byte string's.
///
/// Turns red where `python_source::cbor::checks` checks a record after the
/// first byte, or drops the check of a record's text, or where `held` holds a
/// list in a bare `list`.
#[test]
fn the_python_map_holds_each_list_in_a_list_of_the_schemas_payload() {
    holds(
        Language::Python,
        &generated_fixture(Language::Python),
        &[
            "from . import cbor_causal_entry as ",
            "from sce_forge_runtime.bounded import BoundedBytes, BoundedText, BoundedTuple\n",
            "class CodecCborRecordListSeenList(BoundedTuple):\n    \"\"\"",
            "    LEAST = 0\n    MOST = 2\n",
            "    seen: CodecCborRecordListSeenList = CodecCborRecordListSeenList()\n",
            "    context: Optional[CodecCborRecordListContextList] = None\n",
            // A read past the bound is refused before any record is read.
            "if n > 2:",
            "if n > 3:",
            "v = [CodecCborRecordList._decode_record_causal_entry(c) for _ in range(n)]",
            "held_author = cbor.read_text(c, 16)",
            // Each record is held to the bounds its payload does not hold
            // before the map head is written.
            "    v = src.seen\n    for _record in v:\n        \
             if not 0 <= _record.seq <= 0xFFFFFFFFFFFFFFFF:\n            \
             raise cbor.CborOutOfRange()\n        \
             if len(_record.author.encode(\"utf-8\")) > 16:\n            \
             raise cbor.CborOutOfRange()\n",
            "                raise cbor.CborOutOfRange()\n    count = 3\n",
            "cbor.write_array_head(w, len(v))",
            "    for _record in v:\n        _encode_record_causal_entry(w, _record)\n",
            "cbor.write_map_head(w, 2)",
        ],
        "def _encode_record_causal_entry(",
        [
            "cbor.write_text(w, \"seq\")",
            "cbor.write_text(w, \"author\")",
        ],
    );
}

#[test]
fn a_record_with_a_field_of_another_type_is_refused_by_name_where_it_is_not_lowered() {
    // docs/adr/0101, section 5.4: a backend whose printer writes the records
    // of docs/adr/0085 Landing order step 3a alone refuses the codec, naming
    // the field; Rust and C++ print every shape, and write the field as its
    // type.
    let text = document(
        "",
        r#"  <sce:import kind="event-schema" src="cbor_signed_entry.scxml" as="SignedEntry"/>"#,
        r#"    <data id="n" sce:type="list&lt;record:SignedEntry&gt;[2]" sce:key="0" sce:direction="out"/>"#,
    );
    for &lang in Language::ALL {
        match generated(&text, "probe_list", lang) {
            Ok(code) => {
                assert!(cbor_codec::lowers_every_entry_type(lang), "{lang:?}");
                let written = match lang {
                    Language::Rust => "cbor::write_int(w, i64::from(record.delta))?;",
                    Language::Cpp => {
                        "if (auto _e = cbor::write_int(w, static_cast<std::int64_t>(record.delta)); \
                         _e) return _e;"
                    }
                    other => panic!("{other:?} prints a signed field this test names no write of"),
                };
                assert!(code.contains(written), "{lang:?}:\n{code}");
            }
            Err(why) => {
                assert!(
                    !cbor_codec::lowers_every_entry_type(lang),
                    "{lang:?}: {why}"
                );
                assert!(
                    why.contains("whose field `delta` is `int32`"),
                    "{lang:?}: {why}"
                );
            }
        }
    }
    assert!(cbor_codec::lowers_every_entry_type(Language::Rust));
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
    assert!(rust.contains("if !(1..=2).contains(&len) {"), "{rust}");
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
    // The reason names the place in the type (docs/adr/0101, section 5.2):
    // the field `digest` of each record of the list `n`.
    assert!(
        why.contains("byte string `n[].digest` past its bound 4"),
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
        // The check of the record, which each record of the list is held to
        // before the map is written.
        "fn check_record_cbor_digest_entry(record: &cbor_digest_entry::CborDigestEntryPayload) -> Result<(), CodecError> {",
        "if record.digest.len() > 4 {",
        "check_record_cbor_digest_entry(item)?;",
        "cbor::read_bytes(c, Some(4))?.to_vec()",
        "cbor::write_bytes(w, record.digest.as_slice())?;",
    ] {
        assert!(rust.contains(expected), "missing `{expected}`:\n{rust}");
    }
}

/// The C11 unit of the schema holds the byte string in the buffer of its bound
/// beside its length, which a list of the codec's value owns by value: the
/// read fills both, refusing a string past the bound, and the write refuses a
/// length past it rather than writing it.
#[test]
fn c11_holds_a_records_byte_string_in_its_buffer_with_its_length() {
    let c = generated(
        &document(
            r#" sce:encode-may-fail="true""#,
            DIGEST_IMPORT,
            DIGEST_ENTRY,
        ),
        "probe_list",
        Language::C11,
    )
    .unwrap_or_else(|why| panic!("C11 generates a list of records of a byte string: {why}"));
    for expected in [
        "s = sce_forge_cbor_read_bytes(c, 4U, 4U, out->digest.data, &out->digest.len);",
        "if (record->digest.len > 4U) {",
        "s = sce_forge_cbor_write_bytes(w, record->digest.data, record->digest.len);",
    ] {
        assert!(c.contains(expected), "missing `{expected}`:\n{c}");
    }
}

/// The import of a record whose schema grew: `id` of its first revision, and a
/// number, a truth value, a text and a byte string its second added, each with
/// its default.
const REVISED_IMPORT: &str =
    r#"  <sce:import kind="event-schema" src="cbor_revised_entry.scxml" as="RevisedEntry"/>"#;

/// A list of at most two records of `cbor_revised_entry`.
const REVISED_ENTRY: &str = r#"    <data id="n" sce:type="list&lt;record:RevisedEntry&gt;[2]" sce:key="0" sce:direction="out"/>"#;

/// What `lang` generates for a list of records of `cbor_revised_entry`, whose
/// byte string makes the encode refusable.
fn revised(lang: Language) -> String {
    generated(
        &document(
            r#" sce:encode-may-fail="true""#,
            REVISED_IMPORT,
            REVISED_ENTRY,
        ),
        "probe_list",
        lang,
    )
    .unwrap_or_else(|why| panic!("{lang:?} generates a list of records that grew: {why}"))
}

/// What each backend's read of a record of `cbor_revised_entry` holds, by the
/// language: the refusal of a map that lacks `id`, and the read of each field
/// the second revision added from the bytes of its default.
type LaneReads = (Language, &'static [&'static str]);

/// A record of the schema's first revision lacks the four keys its second
/// added: every backend that lowers a list of records reads each from the
/// bytes of its default (`hops` 7 as `07`, `live` as `f5`, `tag` "none" as
/// `64 6e 6f 6e 65`, `salt` "ab" as `42 61 62`), by the reader of the field's
/// own value, and still refuses a record that lacks `id`, a field of the first
/// revision (docs/adr/0044; SCE_FORGE.md section 4.6.1). Red if a printer
/// refuses a field a later revision added where its key is missing, or reads
/// a field of the first revision as a default.
#[test]
fn every_backend_reads_a_field_a_later_revision_added_from_its_default() {
    let lanes: [LaneReads; 7] = [
        (
            Language::Rust,
            &[
                "        id: held_id.ok_or(CodecError::CborRequiredKeyMissing)?,\n",
                "let mut lacked = SceCursor::new(&[0x07]);",
                "let mut lacked = SceCursor::new(&[0xf5]);",
                "let mut lacked = SceCursor::new(&[0x64, 0x6e, 0x6f, 0x6e, 0x65]);",
                "let mut lacked = SceCursor::new(&[0x42, 0x61, 0x62]);",
                "cbor::read_bool(&mut lacked)?",
            ],
        ),
        (
            Language::Cpp,
            &[
                "if (!held_id.has_value()) return ::SCE::Forge::CodecError::CborRequiredKeyMissing;",
                "static constexpr std::uint8_t kLacked[] = {0x07};",
                "static constexpr std::uint8_t kLacked[] = {0x42, 0x61, 0x62};",
                "auto _r0 = cbor::read_bool(lacked);",
            ],
        ),
        (
            Language::Go,
            &[
                "return record, codec.ErrCborRequiredKeyMissing",
                "lacked := codec.NewSceCursor([]byte{7})",
                "lacked := codec.NewSceCursor([]byte{245})",
                "lacked := codec.NewSceCursor([]byte{100, 110, 111, 110, 101})",
                "lacked := codec.NewSceCursor([]byte{66, 97, 98})",
                "v, err := codec.CborReadBool(&lacked)",
                "raw, err := codec.CborReadUintUpto(&lacked, 255)",
            ],
        ),
        (
            Language::Kotlin,
            &[
                "?: return Decoded.Refused(CodecError.CborRequiredKeyMissing)",
                "Cbor.readUintUpto(SceCursor(byteArrayOf(7.toByte())), 255UL)",
                "Cbor.readBool(SceCursor(byteArrayOf(245.toByte())))",
                "Cbor.readBytes(SceCursor(byteArrayOf(66.toByte(), 97.toByte(), 98.toByte())), 4)",
            ],
        ),
        (
            Language::Python,
            &[
                "raise cbor.CborRequiredKeyMissing()",
                "held_hops = cbor.read_uint_upto(SceCursor(bytes([7])), 255)",
                "held_live = cbor.read_bool(SceCursor(bytes([245])))",
                "held_tag = cbor.read_text(SceCursor(bytes([100, 110, 111, 110, 101])), 4)",
            ],
        ),
        (
            Language::C11,
            &[
                "return SCE_FORGE_CODEC_CBOR_REQUIRED_KEY_MISSING;",
                "static const uint8_t lacked_bytes[] = {7};",
                "sce_forge_cursor_t lacked = sce_forge_cursor_init(lacked_bytes, sizeof lacked_bytes);",
                "s = sce_forge_cbor_read_bool(&lacked, &out->live);",
                "s = sce_forge_cbor_read_bytes(&lacked, 4U, 4U, out->salt.data, &out->salt.len);",
            ],
        ),
        (
            Language::Swift,
            &[
                "guard let id_value = id_held else { throw CodecError.cborRequiredKeyMissing }",
                "var lacked = SceCursor([7])",
                "var lacked = SceCursor([245])",
                "live_value = try Cbor.readBool(&lacked)",
            ],
        ),
    ];
    for (lang, expected) in lanes {
        let code = revised(lang);
        for expected in expected {
            assert!(
                code.contains(expected),
                "{lang:?}: missing `{expected}`:\n{code}"
            );
        }
    }
}
