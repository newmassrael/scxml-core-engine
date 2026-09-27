//! `sce:encoding="cbor"` — a codec that writes one CBOR map (RFC 8949)
//! whose entries are keyed by small unsigned integers (SCE_FORGE.md §4.6).
//!
//! The first consumer is the SCE Mesh envelope (SCE_MESH.md §13), whose C++
//! codec is written by hand today: an integer-keyed map with six required
//! keys and optional ones, read in any order with unknown keys skipped. This
//! file holds the declaration — what the parser reads, what it refuses, and
//! that the page shows a CBOR codec and reads it back.

use sce_build::forge::model::{CborEntry, CodecEncoding, CodecModel, ForgeDocument, SceType};
use sce_build::forge::parser::parse_forge;
use sce_build::forge::{pseudo, unpseudo};
use sce_build::DocumentLabel;

/// A CBOR codec whose `<datamodel>` holds `entries` verbatim.
fn document(root_attrs: &str, entries: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:encoding="cbor"{root_attrs} name="probe_cbor" version="1.0">
  <datamodel>
    <data id="raw" sce:type="bytes" sce:direction="in"/>
{entries}
  </datamodel>
</scxml>
"#
    )
}

/// The shape of the Mesh envelope's head: required keys, an optional one,
/// an exact-length byte string and a bounded text.
const ENVELOPE_HEAD: &str = r#"    <data id="id" sce:type="bytes" sce:key="0" sce:required="true" sce:length="16" sce:direction="out"/>
    <data id="source" sce:type="string" sce:key="1" sce:required="true" sce:max-size="256" sce:direction="out"/>
    <data id="pattern" sce:type="uint8" sce:key="3" sce:required="true" sce:direction="out"/>
    <data id="deadline" sce:type="uint64" sce:key="12" sce:direction="out"/>
    <data id="urgent" sce:type="bool" sce:key="7" sce:direction="out"/>"#;

fn label() -> DocumentLabel<'static> {
    DocumentLabel {
        identifier: "probe_cbor",
        diagnostic_label: "probe_cbor",
    }
}

fn codec(text: &str) -> CodecModel {
    match parse_forge(text, label()) {
        Ok(Some(ForgeDocument::Codec(m))) => m,
        other => panic!("expected a codec, got {other:?}"),
    }
}

fn refusal(text: &str) -> String {
    match parse_forge(text, label()) {
        Err(e) => e.to_string(),
        Ok(other) => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn every_entry_is_read_with_its_key_and_its_bounds() {
    let m = codec(&document("", ENVELOPE_HEAD));
    assert_eq!(m.encoding, CodecEncoding::Cbor);
    assert!(
        m.fields.is_empty() && m.variant.is_none() && m.input_length.is_none(),
        "no positional member is filled for a CBOR codec: {m:?}"
    );
    let entry = |id: &str| -> &CborEntry {
        m.cbor_entries
            .iter()
            .find(|e| e.id == id)
            .unwrap_or_else(|| panic!("no entry '{id}' in {:?}", m.cbor_entries))
    };
    assert_eq!(m.cbor_entries.len(), 5, "the input frame is not an entry");
    assert_eq!(
        (entry("id").key, entry("id").required, entry("id").length),
        (0, true, Some(16))
    );
    assert_eq!(entry("source").max_size, Some(256));
    assert_eq!(
        (
            entry("deadline").sce_type.clone(),
            entry("deadline").required
        ),
        (SceType::Uint64, false)
    );
    assert_eq!(entry("urgent").sce_type, SceType::Bool);
}

#[test]
fn the_page_shows_a_cbor_codec_and_reads_it_back() {
    let doc = ForgeDocument::Codec(codec(&document("", ENVELOPE_HEAD)));
    let page = pseudo::render(&doc).expect("a CBOR codec renders");
    assert!(page.contains("encoding cbor"), "{page}");
    assert!(
        page.contains("entry id: bytes key 0 required length 16"),
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
fn a_position_on_an_entry_is_refused_rather_than_ignored() {
    let why = refusal(&document(
        "",
        r#"    <data id="n" sce:type="uint8" sce:key="0" sce:byte="0" sce:direction="out"/>"#,
    ));
    assert!(why.contains("sce:byte"), "{why}");

    let why = refusal(&document(
        r#" sce:default-endian="little""#,
        r#"    <data id="n" sce:type="uint8" sce:key="0" sce:direction="out"/>"#,
    ));
    assert!(why.contains("sce:default-endian"), "{why}");

    let why = refusal(&document(
        "",
        r#"    <sce:field id="n" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>"#,
    ));
    assert!(why.contains("field"), "{why}");
}

#[test]
fn a_key_is_one_byte_and_names_one_entry() {
    let why = refusal(&document(
        "",
        r#"    <data id="n" sce:type="uint8" sce:key="24" sce:direction="out"/>"#,
    ));
    assert!(why.contains("24"), "{why}");

    let why = refusal(&document(
        "",
        r#"    <data id="a" sce:type="uint8" sce:key="2" sce:direction="out"/>
    <data id="b" sce:type="uint8" sce:key="2" sce:direction="out"/>"#,
    ));
    assert!(why.contains("sce:key") && why.contains('2'), "{why}");

    let why = refusal(&document(
        "",
        r#"    <data id="n" sce:type="uint8" sce:direction="out"/>"#,
    ));
    assert!(why.contains("sce:key"), "{why}");
}

#[test]
fn a_kind_the_map_does_not_write_is_refused() {
    for ty in ["int32", "float64"] {
        let why = refusal(&document(
            "",
            &format!(r#"    <data id="n" sce:type="{ty}" sce:key="0" sce:direction="out"/>"#),
        ));
        assert!(why.contains(ty), "{ty}: {why}");
    }
}

#[test]
fn a_bound_is_held_to_the_kind_it_bounds() {
    let why = refusal(&document(
        "",
        r#"    <data id="s" sce:type="string" sce:key="0" sce:length="4" sce:direction="out"/>"#,
    ));
    assert!(why.contains("sce:length"), "{why}");

    let why = refusal(&document(
        "",
        r#"    <data id="n" sce:type="uint8" sce:key="0" sce:max-size="4" sce:direction="out"/>"#,
    ));
    assert!(why.contains("sce:max-size"), "{why}");

    let why = refusal(&document(
        "",
        r#"    <data id="b" sce:type="bytes" sce:key="0" sce:length="4" sce:max-size="8" sce:direction="out"/>"#,
    ));
    assert!(why.contains("sce:max-size"), "{why}");
}
