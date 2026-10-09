//! `sce:encoding="content-line"` — a codec that reads and writes one RFC 5545
//! component of content lines (SCE_FORGE.md §4.6.4, docs/adr/0010).
//!
//! This file holds the declaration — what the parser reads, what it refuses, that
//! the page shows a content-line codec and reads it back, and that no backend
//! generates one yet and says so by name. The wire rules are held where a backend
//! generates the codec, against the conformance corpus.

use sce_build::forge::content_line_codec;
use sce_build::forge::model::{
    CodecEncoding, CodecModel, ContentLineEntry, ForgeDocument, SceType,
};
use sce_build::forge::parser::parse_forge;
use sce_build::forge::{pseudo, unpseudo};
use sce_build::generator::Language;
use sce_build::DocumentLabel;
use std::process::Command;
use tempfile::tempdir;

/// A content-line codec of component `VEVENT` whose `<datamodel>` holds `entries`.
fn document(root_attrs: &str, entries: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:encoding="content-line" sce:component="VEVENT"{root_attrs} name="probe_event" version="1.0">
  <datamodel>
    <data id="raw" sce:type="bytes" sce:direction="in"/>
{entries}
  </datamodel>
</scxml>
"#
    )
}

/// The shape of the component the calendar reads: required text, a TEXT, a
/// property with a parameter, a rule carried as written, a repeated property and
/// a bool and an integer.
const EVENT: &str = r#"    <data id="uid" sce:type="string" sce:property="UID" sce:required="true" sce:max-size="256"/>
    <data id="summary" sce:type="string" sce:property="SUMMARY" sce:value="text" sce:max-size="512"/>
    <data id="dtstart" sce:type="string" sce:property="DTSTART" sce:required="true" sce:max-size="32"/>
    <data id="dtstartTzid" sce:type="string" sce:property="DTSTART" sce:param="TZID" sce:max-size="64"/>
    <data id="rrule" sce:type="string" sce:property="RRULE" sce:max-size="256"/>
    <data id="exdate" sce:type="string" sce:property="EXDATE" sce:max-count="64" sce:max-size="32"/>
    <data id="sequence" sce:type="uint32" sce:property="SEQUENCE"/>
    <data id="allDay" sce:type="bool" sce:property="X-ALL-DAY"/>"#;

fn label() -> DocumentLabel<'static> {
    DocumentLabel {
        identifier: "probe_event",
        diagnostic_label: "probe_event",
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
fn every_entry_is_read_with_its_property_its_parameter_and_its_bounds() {
    let m = codec(&document("", EVENT));
    assert_eq!(m.encoding, CodecEncoding::ContentLine);
    assert!(
        m.fields.is_empty()
            && m.variant.is_none()
            && m.input_length.is_none()
            && m.cbor_entries.is_empty(),
        "no member of another encoding is filled: {m:?}"
    );
    let content = m.content_line.as_ref().expect("the content-line shape");
    assert_eq!(content.component, "VEVENT");
    assert_eq!(content.entries.len(), 8, "the input frame is not an entry");
    let entry = |id: &str| -> &ContentLineEntry {
        content
            .entries
            .iter()
            .find(|e| e.id == id)
            .unwrap_or_else(|| panic!("no entry '{id}' in {:?}", content.entries))
    };
    assert_eq!(
        (
            entry("uid").property.as_str(),
            entry("uid").required,
            entry("uid").max_size
        ),
        ("UID", true, Some(256))
    );
    assert!(entry("summary").text && !entry("rrule").text);
    assert_eq!(
        (
            entry("dtstartTzid").property.as_str(),
            entry("dtstartTzid").param.as_deref()
        ),
        ("DTSTART", Some("TZID"))
    );
    assert_eq!(entry("exdate").max_count, Some(64));
    assert_eq!(entry("sequence").sce_type, SceType::Uint32);
    assert_eq!(entry("allDay").sce_type, SceType::Bool);
}

#[test]
fn the_page_shows_a_content_line_codec_and_reads_it_back() {
    let doc = ForgeDocument::Codec(codec(&document("", EVENT)));
    let page = pseudo::render(&doc).expect("a content-line codec renders");
    assert!(
        page.contains("encoding content-line component VEVENT"),
        "{page}"
    );
    assert!(
        page.contains("entry uid: string property UID required max-size 256"),
        "{page}"
    );
    assert!(
        page.contains("entry dtstartTzid: string property DTSTART param TZID max-size 64"),
        "{page}"
    );
    assert!(
        page.contains("entry exdate: string property EXDATE max-size 32 max-count 64"),
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
fn a_position_a_key_or_an_exact_length_is_refused_rather_than_ignored() {
    for attr in [r#"sce:byte="0""#, r#"sce:key="0""#, r#"sce:length="4""#] {
        let why = refusal(&document(
            "",
            &format!(
                r#"    <data id="n" sce:type="uint8" sce:property="N" {attr} sce:direction="out"/>"#
            ),
        ));
        assert!(
            why.contains(attr.split('=').next().unwrap_or("")),
            "{attr}: {why}"
        );
    }

    let why = refusal(&document(
        r#" sce:default-endian="little""#,
        r#"    <data id="n" sce:type="uint8" sce:property="N"/>"#,
    ));
    assert!(why.contains("sce:default-endian"), "{why}");

    let why = refusal(&document(
        "",
        r#"    <sce:field id="n" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>"#,
    ));
    assert!(why.contains("field"), "{why}");
}

#[test]
fn the_component_and_every_property_are_named_with_a_name_of_the_format() {
    let no_component = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:encoding="content-line" name="probe_event" version="1.0">
  <datamodel>
    <data id="uid" sce:type="string" sce:property="UID" sce:max-size="8"/>
  </datamodel>
</scxml>
"#;
    assert!(refusal(no_component).contains("sce:component"));

    let why = refusal(&document("", r#"    <data id="n" sce:type="uint8"/>"#));
    assert!(why.contains("sce:property"), "{why}");

    let why = refusal(&document(
        "",
        r#"    <data id="n" sce:type="uint8" sce:property="A:B"/>"#,
    ));
    assert!(why.contains("sce:property") && why.contains("A:B"), "{why}");

    let why = refusal(
        &document(
            "",
            r#"    <data id="n" sce:type="uint8" sce:property="N"/>"#,
        )
        .replace(r#"sce:component="VEVENT""#, r#"sce:component="VEVENT;X""#),
    );
    assert!(why.contains("sce:component"), "{why}");
}

#[test]
fn an_unknown_encoding_is_refused_naming_the_three_there_are() {
    let why = refusal(
        &document("", EVENT).replace(r#"sce:encoding="content-line""#, r#"sce:encoding="csv""#),
    );
    assert!(
        why.contains("content-line") && why.contains("cbor") && why.contains("positional"),
        "{why}"
    );
}

#[test]
fn a_property_is_one_entry_and_a_parameter_follows_the_entry_of_its_property() {
    let why = refusal(&document(
        "",
        r#"    <data id="a" sce:type="uint8" sce:property="N"/>
    <data id="b" sce:type="uint8" sce:property="n"/>"#,
    ));
    assert!(why.contains("sce:property"), "{why}");

    let why = refusal(&document(
        "",
        r#"    <data id="p" sce:type="string" sce:property="DTSTART" sce:param="TZID" sce:max-size="8"/>
    <data id="v" sce:type="string" sce:property="DTSTART" sce:max-size="8"/>"#,
    ));
    assert!(
        why.contains("sce:param") && why.contains("DTSTART"),
        "{why}"
    );

    let why = refusal(&document(
        "",
        r#"    <data id="v" sce:type="string" sce:property="DTSTART" sce:max-size="8"/>
    <data id="p" sce:type="string" sce:property="DTSTART" sce:param="TZID" sce:max-size="8"/>
    <data id="q" sce:type="string" sce:property="dtstart" sce:param="tzid" sce:max-size="8"/>"#,
    ));
    assert!(why.contains("sce:param"), "{why}");

    let why = refusal(&document(
        "",
        r#"    <data id="a" sce:type="uint8" sce:property="N"/>
    <data id="a" sce:type="uint8" sce:property="M"/>"#,
    ));
    assert!(why.contains("entry id"), "{why}");
}

#[test]
fn a_kind_a_line_does_not_carry_is_refused() {
    for ty in ["bytes", "float64"] {
        let why = refusal(&document(
            "",
            &format!(r#"    <data id="n" sce:type="{ty}" sce:property="N"/>"#),
        ));
        assert!(why.contains(ty), "{ty}: {why}");
    }
    // A parameter is a text or a name, never a number.
    let why = refusal(&document(
        "",
        r#"    <data id="v" sce:type="string" sce:property="P" sce:max-size="8"/>
    <data id="q" sce:type="uint8" sce:property="P" sce:param="Q"/>"#,
    ));
    assert!(why.contains("uint8") && why.contains("parameter"), "{why}");
}

/// An `enum:<alias>` entry maps a property's text to a variant's declared name, which
/// no backend's enum import offers a codec yet. It is read as the type it is, so an
/// alias that names no import is reported as that, and one that names an import is
/// refused as what it waits on.
#[test]
fn an_enum_entry_waits_on_a_codec_that_names_variants() {
    let with_import = |entry: &str| {
        document("", entry).replace(
            "<datamodel>",
            r#"<sce:import src="mode.scxml" kind="enum" as="mode"/>
  <datamodel>"#,
        )
    };
    let why = refusal(&with_import(
        r#"    <data id="m" sce:type="enum:mode" sce:property="STATUS"/>"#,
    ));
    assert!(
        why.contains("enum:mode") && why.contains("waits on a codec that names variants"),
        "{why}"
    );

    let why = refusal(&with_import(
        r#"    <data id="v" sce:type="string" sce:property="P" sce:max-size="8"/>
    <data id="q" sce:type="enum:mode" sce:property="P" sce:param="Q"/>"#,
    ));
    assert!(
        why.contains("enum:mode") && why.contains("parameter"),
        "{why}"
    );

    let why = refusal(&document(
        "",
        r#"    <data id="m" sce:type="enum:nope" sce:property="STATUS"/>"#,
    ));
    assert!(why.contains("nope"), "{why}");
}

/// A parameter is missing only from a property that is there to be missing it from.
#[test]
fn a_required_parameter_needs_a_required_property() {
    let why = refusal(&document(
        "",
        r#"    <data id="v" sce:type="string" sce:property="DTSTART" sce:max-size="8"/>
    <data id="p" sce:type="string" sce:property="DTSTART" sce:param="TZID" sce:required="true" sce:max-size="8"/>"#,
    ));
    assert!(
        why.contains("sce:required") && why.contains("DTSTART"),
        "{why}"
    );

    let model = codec(&document(
        "",
        r#"    <data id="v" sce:type="string" sce:property="DTSTART" sce:required="true" sce:max-size="8"/>
    <data id="p" sce:type="string" sce:property="DTSTART" sce:param="TZID" sce:required="true" sce:max-size="8"/>"#,
    ));
    let entries = &model.content_line.expect("a content-line codec").entries;
    assert!(entries.iter().all(|e| e.required));
}

#[test]
fn a_bound_and_a_text_mark_are_held_to_the_kind_they_qualify() {
    // A string names its bound.
    let why = refusal(&document(
        "",
        r#"    <data id="s" sce:type="string" sce:property="S"/>"#,
    ));
    assert!(why.contains("sce:max-size"), "{why}");

    let why = refusal(&document(
        "",
        r#"    <data id="n" sce:type="uint8" sce:property="N" sce:max-size="4"/>"#,
    ));
    assert!(why.contains("sce:max-size"), "{why}");

    let why = refusal(&document(
        "",
        r#"    <data id="n" sce:type="uint8" sce:property="N" sce:value="text"/>"#,
    ));
    assert!(why.contains("sce:value"), "{why}");

    let why = refusal(&document(
        "",
        r#"    <data id="n" sce:type="string" sce:property="N" sce:value="raw" sce:max-size="4"/>"#,
    ));
    // The schema check reads the attribute before the parser does, and names it by
    // its local name.
    assert!(why.contains("value") && why.contains("raw"), "{why}");

    // A list is at least two lines, of a string value.
    let why = refusal(&document(
        "",
        r#"    <data id="l" sce:type="string" sce:property="L" sce:max-count="1" sce:max-size="4"/>"#,
    ));
    assert!(why.contains("sce:max-count"), "{why}");

    let why = refusal(&document(
        "",
        r#"    <data id="l" sce:type="uint8" sce:property="L" sce:max-count="4"/>"#,
    ));
    assert!(why.contains("sce:max-count"), "{why}");
}

#[test]
fn a_codec_with_no_property_is_refused() {
    let why = refusal(&document("", ""));
    assert!(why.contains("sce:property"), "{why}");
}

/// A repeated property declares the parameters its lines carry, and `sce:separator`
/// with `sce:max-values` makes the value of a line a list (docs/adr/0014).
const RECORDS: &str = r#"    <data id="exdate" sce:type="string" sce:property="EXDATE" sce:max-count="64" sce:max-size="32" sce:separator="," sce:max-values="8"/>
    <data id="exdateTzid" sce:type="string" sce:property="EXDATE" sce:param="TZID" sce:max-size="64"/>
    <data id="attendee" sce:type="string" sce:property="ATTENDEE" sce:max-count="100" sce:max-size="256"/>
    <data id="attendeeCn" sce:type="string" sce:property="ATTENDEE" sce:param="CN" sce:max-size="64"/>
    <data id="categories" sce:type="string" sce:property="CATEGORIES" sce:value="text" sce:max-size="32" sce:separator="," sce:max-values="4"/>"#;

#[test]
fn a_repeated_property_takes_parameters_and_a_line_may_hold_a_list_of_values() {
    let m = codec(&document(
        "",
        &format!(
            "    <data id=\"uid\" sce:type=\"string\" sce:property=\"UID\" sce:max-size=\"8\"/>\n{RECORDS}"
        ),
    ));
    let content = m.content_line.as_ref().expect("a content-line codec");
    let exdate = content.entries.iter().find(|e| e.id == "exdate").unwrap();
    assert_eq!(exdate.max_count, Some(64));
    assert_eq!(exdate.separator.as_deref(), Some(","));
    assert_eq!(exdate.max_values, Some(8));
    let tzid = content
        .entries
        .iter()
        .find(|e| e.id == "exdateTzid")
        .unwrap();
    assert_eq!(tzid.param.as_deref(), Some("TZID"));
    let categories = content
        .entries
        .iter()
        .find(|e| e.id == "categories")
        .unwrap();
    assert!(categories.text && categories.max_count.is_none());
    assert!(content.uses_line_records());

    // Neither shape is read for a codec without them: the one that has a repeated
    // property and nothing else keeps its present form.
    let plain = codec(&document("", EVENT));
    assert!(!plain.content_line.as_ref().unwrap().uses_line_records());

    // The page shows them and reads them back.
    let doc = ForgeDocument::Codec(m);
    let page = pseudo::render(&doc).expect("a content-line codec renders");
    assert!(
        page.contains("separator , max-values 8") && page.contains("separator , max-values 4"),
        "{page}"
    );
    let back = unpseudo::parse(&page).expect("the page reads back");
    assert_eq!(
        unpseudo::ir_for_comparison(&back).expect("serializes"),
        unpseudo::ir_for_comparison(&doc).expect("serializes"),
        "the round trip changed the codec:\n{page}"
    );
}

/// A parameter that is required is required of each line that exists, so on a
/// repeated property it needs no required property to be missing from; on a
/// single-valued property it still does (docs/adr/0014).
#[test]
fn a_required_parameter_of_a_repeated_property_is_required_of_each_line() {
    let repeated = |required: &str| {
        document(
            "",
            &format!(
                r#"    <data id="a" sce:type="string" sce:property="A" sce:max-count="3" sce:max-size="8"{required}/>
    <data id="aRole" sce:type="string" sce:property="A" sce:param="ROLE" sce:max-size="8" sce:required="true"/>"#
            ),
        )
    };
    let m = codec(&repeated(""));
    let content = m.content_line.as_ref().expect("a content-line codec");
    let role = content.entries.iter().find(|e| e.id == "aRole").unwrap();
    assert!(role.required);
    assert!(content.uses_line_records());

    let single = refusal(&document(
        "",
        r#"    <data id="a" sce:type="string" sce:property="A" sce:max-size="8"/>
    <data id="aRole" sce:type="string" sce:property="A" sce:param="ROLE" sce:max-size="8" sce:required="true"/>"#,
    ));
    assert!(single.contains("required"), "{single}");
}

#[test]
fn a_separator_and_its_bound_are_held_to_the_entry_they_qualify() {
    let one = |extra: &str| {
        refusal(&document(
            "",
            &format!(
                r#"    <data id="v" sce:type="string" sce:property="V" sce:max-size="8"{extra}/>"#
            ),
        ))
    };
    // Not a separator this encoding has.
    assert!(
        one(r#" sce:separator=":" sce:max-values="4""#).contains("sce:separator"),
        "a colon ends the property name"
    );
    // The two stand together.
    assert!(one(r#" sce:separator="," "#).contains("sce:max-values"));
    assert!(one(r#" sce:max-values="4""#).contains("sce:separator"));
    // A list has two members at least.
    assert!(one(r#" sce:separator="," sce:max-values="1""#).contains("at least 2"));
    // On a string value entry only.
    let why = refusal(&document(
        "",
        r#"    <data id="n" sce:type="uint8" sce:property="N" sce:separator="," sce:max-values="4"/>"#,
    ));
    assert!(why.contains("sce:separator"), "{why}");
    let why = refusal(&document(
        "",
        r#"    <data id="v" sce:type="string" sce:property="V" sce:max-size="8"/>
    <data id="p" sce:type="string" sce:property="V" sce:param="P" sce:max-size="8" sce:separator="," sce:max-values="4"/>"#,
    ));
    assert!(
        why.contains("sce:separator") && why.contains("parameter"),
        "{why}"
    );
}

/// The shapes the decision adds are declared and checked, and a backend refuses
/// them by name until its own commit generates them; the codec that uses neither
/// is generated everywhere. `GENERATING` is the list of backends that have landed.
#[test]
fn a_line_record_is_refused_by_name_until_a_backend_generates_it() {
    const GENERATING: [Language; 5] = [
        Language::Python,
        Language::Go,
        Language::Kotlin,
        Language::Rust,
        Language::Cpp,
    ];
    let m = codec(&document(
        "",
        &format!(
            "    <data id=\"uid\" sce:type=\"string\" sce:property=\"UID\" sce:max-size=\"8\"/>\n{RECORDS}"
        ),
    ));
    for lang in [
        Language::Rust,
        Language::Kotlin,
        Language::Cpp,
        Language::Go,
        Language::Python,
        Language::C11,
    ] {
        let generating = GENERATING.contains(&lang);
        assert_eq!(
            content_line_codec::lowers_line_records(lang),
            generating,
            "{lang:?}"
        );
        let refusal = content_line_codec::refusal(lang, &m);
        if generating {
            assert!(refusal.is_none(), "{lang:?}: {refusal:?}");
        } else {
            let why = refusal.expect("refused by name");
            assert!(
                why.contains("record") && why.contains("docs/adr/0014"),
                "{lang:?}: {why}"
            );
        }
    }
}

/// Every backend generates a content-line codec, and the generator's refusal and
/// the conformance harness's schedule read one answer for it
/// (`content_line_codec::refusal`). A backend that had not landed would be
/// refused by name there, as each was until its own commit.
#[test]
fn every_backend_generates_a_content_line_codec() {
    let m = codec(&document("", EVENT));
    for lang in [
        Language::Rust,
        Language::Kotlin,
        Language::Cpp,
        Language::Go,
        Language::Python,
        Language::C11,
    ] {
        assert!(content_line_codec::lowers(lang), "{lang:?}");
        assert_eq!(content_line_codec::refusal(lang, &m), None, "{lang:?}");
    }
}

#[test]
fn the_generator_writes_the_codec_in_every_language() {
    let dir = tempdir().expect("tempdir");
    let source = dir.path().join("probe_event.scxml");
    std::fs::write(&source, document("", EVENT)).expect("write the document");
    for lang in ["rust", "kotlin", "cpp", "go", "python", "c11"] {
        let out = dir.path().join(format!("out_{lang}"));
        let mut command = Command::new(env!("CARGO_BIN_EXE_sce-codegen"));
        command
            .args(["generate", "-l", lang, "-o"])
            .arg(&out)
            .arg(&source);
        if lang == "go" {
            command.args(["--go-module-prefix", "x/y"]);
        }
        let output = command.output().expect("run sce-codegen");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.status.success(), "{lang} refused:\n{text}");
        assert!(
            !text.contains("generation yet"),
            "{lang} still says it has no generation:\n{text}"
        );
    }
}

#[test]
fn rust_generates_the_codec_and_it_reads_and_writes_through_the_runtime_alone() {
    let dir = tempdir().expect("tempdir");
    let source = dir.path().join("probe_event.scxml");
    std::fs::write(&source, document("", EVENT)).expect("write the document");
    let out = dir.path().join("out_rust");
    let output = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .args(["generate", "-l", "rust", "-o"])
        .arg(&out)
        .arg(&source)
        .output()
        .expect("run sce-codegen");
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let generated = std::fs::read_to_string(out.join("probe_event.rs")).expect("generated file");
    // The line grammar is the runtime's: the codec calls it and spells none of it.
    for needle in [
        "ContentLineReader::begin(",
        "ContentLineWriter::begin(",
        "heapless::String<256>",
        "heapless::Vec<heapless::String<32>, 64>",
        "Option<u32>",
        "Option<bool>",
        "CodecError::LineRequiredMissing",
        "CodecError::LineTooMany",
    ] {
        assert!(generated.contains(needle), "missing {needle}:\n{generated}");
    }
    for borrowed in ["\\r\\n", "b\"BEGIN", "BEGIN:"] {
        assert!(
            !generated.contains(borrowed),
            "the generated codec spells the wire itself ({borrowed}):\n{generated}"
        );
    }
}
