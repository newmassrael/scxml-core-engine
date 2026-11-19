// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A CBOR codec's entry holds any value of the language (docs/adr/0101,
//! section 1): its `sce:type` is read by the one type grammar of docs/adr/0086
//! with docs/adr/0092's `union:<alias>`, at any depth, every bound in the type.
//!
//! This holds the grammar and the validator of that record's Landing order
//! step 1, with no toolchain: every constructor read on an entry; `sce:max-size`
//! read as the bracket of a text or byte string and reported as superseded; a
//! bracket beside `sce:length` refused; a text with no bound refused at any
//! depth; a union import read by a codec; a content-line entry still refusing
//! every composition; every backend refusing by name an entry its printer does
//! not lower yet (section 5.4); and `validation/encoded-size-overflow` for a
//! type whose largest encoding is past 2^64 - 1 bytes (section 4.4).

use std::path::{Path, PathBuf};

use sce_build::forge::bound_spelling;
use sce_build::forge::cbor_codec;
use sce_build::forge::diagnostic::ToDiagnostics;
use sce_build::forge::error::{ForgeError, Located};
use sce_build::forge::model::{CodecModel, ForgeDocument, SceType};
use sce_build::forge::parser::parse_forge;
use sce_build::forge::value_type::{Bound, ValueType, WrittenType};
use sce_build::generator::Language;
use sce_build::{compile_forge_with_imports, DocumentLabel, ForgeCompileOptions};

/// The directory of the corpus's records, enums and unions, which a probe
/// imports from.
fn imports_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_profile/imports")
}

/// The imports a probe declares: an enum, two records and a union of the
/// corpus.
const IMPORTS: &str = r#"  <sce:import kind="enum" src="paint_color.scxml" as="Color"/>
  <sce:import kind="event-schema" src="order_customer.scxml" as="Customer"/>
  <sce:import kind="event-schema" src="order_line.scxml" as="Line"/>
  <sce:import kind="union" src="order_payment.scxml" as="Payment"/>"#;

/// A CBOR codec in codec format 2 with the import lines `imports` and the
/// entries `entries`, verbatim.
fn document(imports: &str, entries: &str) -> String {
    document_stating("", imports, entries)
}

/// [`document`], its root carrying `root_attrs` after its name.
fn document_stating(root_attrs: &str, imports: &str, entries: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:encoding="cbor" sce:codec-format="2" name="probe_any"{root_attrs} version="1.0">
{imports}
  <datamodel>
    <data id="raw" sce:type="bytes" sce:direction="in"/>
{entries}
  </datamodel>
</scxml>
"#
    )
}

/// One entry `v` under key 0 of the type `ty`, as an attribute spells it.
fn one_entry(ty: &str) -> String {
    format!(
        r#"    <data id="v" sce:type="{}" sce:key="0" sce:direction="out"/>"#,
        ty.replace('<', "&lt;").replace('>', "&gt;")
    )
}

fn label() -> DocumentLabel<'static> {
    DocumentLabel {
        identifier: "probe_any",
        diagnostic_label: "probe_any",
    }
}

fn codec(text: &str) -> CodecModel {
    match parse_forge(text, label()) {
        Ok(Some(ForgeDocument::Codec(m))) => m,
        other => panic!("expected a codec, got {other:?}"),
    }
}

fn refusal(text: &str) -> Located<ForgeError> {
    match parse_forge(text, label()) {
        Err(refusal) => refusal,
        Ok(other) => panic!("expected a refusal, got {other:?}"),
    }
}

/// The wire code of the first record `error` reports.
fn code_of(error: &ForgeError) -> String {
    serde_json::to_value(error.to_diagnostics()[0].code)
        .expect("a code serialises")
        .as_str()
        .expect("a code is a string")
        .to_string()
}

/// The type the grammar reads `ty` as, every bound in it.
fn type_of(ty: &str) -> ValueType {
    WrittenType::read(ty)
        .and_then(WrittenType::bounded)
        .unwrap_or_else(|why| panic!("`{ty}` is a type: {why:?}"))
}

fn options() -> ForgeCompileOptions {
    ForgeCompileOptions {
        go_module_prefix: Some("example.com/sce-forge".to_string()),
        ..ForgeCompileOptions::default()
    }
}

/// The types of docs/adr/0101 section 1.1, each at depth, with whether the
/// printers of docs/adr/0085 step 3a print an entry of it.
const TYPES: &[(&str, bool)] = &[
    ("int8", false),
    ("int64", false),
    ("float32", false),
    ("float64", false),
    ("enum:Color", true),
    ("optional<enum:Color>", true),
    ("list<uint8>[4]", false),
    ("list<optional<uint16>>[3]", false),
    ("list<list<uint8>[2]>[2]", false),
    ("list<bool>[1..2]", false),
    ("record:Customer", false),
    ("optional<record:Customer>", false),
    ("union:Payment", false),
    ("optional<union:Payment>", false),
    ("list<union:Payment>[1..3]", false),
    ("list<record:Line>[4]", true),
    ("optional<list<record:Line>[4]>", true),
    ("string[8]", true),
    ("optional<bytes[4]>", true),
];

#[test]
fn every_type_of_the_language_is_read_as_a_cbor_entry() {
    for &(ty, printed) in TYPES {
        let m = codec(&document(IMPORTS, &one_entry(ty)));
        let entry = &m.cbor_entries[0];
        assert_eq!(entry.sce_type, type_of(ty), "{ty}");
        assert_eq!(entry.is_required(), !ty.starts_with("optional<"), "{ty}");
        assert_eq!(entry.length, None, "{ty}");
        assert_eq!(
            cbor_codec::unlowered_part(entry).is_none(),
            printed,
            "{ty}: which part the printers do not lower"
        );
    }
}

#[test]
fn the_part_a_printer_does_not_lower_is_named_by_its_constructor() {
    for (ty, part) in [
        ("int8", "int8"),
        ("optional<float64>", "float64"),
        ("list<uint8>[4]", "list<uint8>[4]"),
        ("record:Customer", "record:Customer"),
        ("optional<union:Payment>", "union:Payment"),
    ] {
        let m = codec(&document(IMPORTS, &one_entry(ty)));
        let got = cbor_codec::unlowered_part(&m.cbor_entries[0]).map(ToString::to_string);
        assert_eq!(got.as_deref(), Some(part), "{ty}");
    }
}

#[test]
fn a_max_size_is_the_bracket_of_its_text_and_is_reported_superseded() {
    // docs/adr/0101, section 1.3: `sce:max-size` on a top-level text or byte
    // string is the legacy spelling of its bracket, with the `sce:limit` beside it.
    let legacy = document(
        "",
        r#"    <data id="note" sce:type="optional&lt;string&gt;" sce:key="0" sce:max-size="8" sce:limit="note_text" sce:direction="out"/>
    <data id="blob" sce:type="bytes" sce:key="1" sce:max-size="4" sce:direction="out"/>"#,
    );
    let bracket = document(
        "",
        r#"    <data id="note" sce:type="optional&lt;string[note_text=8]&gt;" sce:key="0" sce:direction="out"/>
    <data id="blob" sce:type="bytes[4]" sce:key="1" sce:direction="out"/>"#,
    );
    let (from_legacy, from_bracket) = (codec(&legacy), codec(&bracket));
    assert_eq!(from_legacy.cbor_entries, from_bracket.cbor_entries);
    assert_eq!(
        from_legacy.cbor_entries[0].sce_type,
        ValueType::Optional {
            elem: Box::new(ValueType::Text {
                max: Bound::named("note_text", 8)
            })
        }
    );
    let records = bound_spelling::superseded(&legacy, "probe_any");
    let codes: Vec<String> = records.iter().map(|r| code_of(&r.error)).collect();
    assert_eq!(
        codes,
        [
            "validation/bound-attribute-superseded",
            "validation/bound-attribute-superseded"
        ]
    );
    assert!(bound_spelling::superseded(&bracket, "probe_any").is_empty());
    // The rewrite prints the document the bracket spells.
    let rewritten = bound_spelling::rewritten_document(&legacy, "probe_any").expect("rewrites");
    assert_eq!(codec(&rewritten).cbor_entries, from_bracket.cbor_entries);
}

#[test]
fn an_exact_length_is_the_bound_of_a_byte_string_that_writes_no_bracket() {
    let m = codec(&document(
        "",
        r#"    <data id="id" sce:type="optional&lt;bytes&gt;" sce:key="0" sce:length="16" sce:direction="out"/>"#,
    ));
    let entry = &m.cbor_entries[0];
    assert_eq!(entry.length, Some(16));
    assert_eq!(entry.sce_type, type_of("optional<bytes[16]>"));
    assert_eq!(entry.size_bound(), None, "an exact length is its own bound");
    assert_eq!(entry.text_bound(), Some(16));
}

#[test]
fn one_bound_has_one_spelling() {
    // A bracket beside `sce:length`, and a bracket beside `sce:max-size`, each
    // state the bound twice: `validation/incompatible-attributes`.
    for entry in [
        r#"    <data id="id" sce:type="bytes[16]" sce:key="0" sce:length="16" sce:direction="out"/>"#,
        r#"    <data id="note" sce:type="string[8]" sce:key="0" sce:max-size="8" sce:direction="out"/>"#,
    ] {
        let refused = refusal(&document("", entry));
        assert_eq!(
            code_of(&refused.error),
            "validation/incompatible-attributes",
            "{entry}"
        );
    }
    // An exact length on what is no byte string, and a size on what is no text
    // or byte string, are refused at the attribute.
    for (entry, attr) in [
        (
            r#"    <data id="n" sce:type="string[8]" sce:key="0" sce:length="8" sce:direction="out"/>"#,
            "sce:length",
        ),
        (
            r#"    <data id="n" sce:type="int8" sce:key="0" sce:max-size="8" sce:direction="out"/>"#,
            "sce:max-size",
        ),
        (
            r#"    <data id="n" sce:type="list&lt;uint8&gt;" sce:key="0" sce:max-size="8" sce:direction="out"/>"#,
            "sce:max-size",
        ),
    ] {
        let refused = refusal(&document("", entry));
        assert_eq!(
            code_of(&refused.error),
            "validation/attribute-rule-violated",
            "{entry}"
        );
        assert!(refused.error.to_string().contains(attr), "{entry}");
    }
}

#[test]
fn a_text_with_no_bound_is_refused_at_any_depth() {
    // docs/adr/0101, section 1.3: a CBOR entry is a value of the language, and
    // a text, byte string or list that states no bound is no type of it.
    for ty in [
        "string",
        "optional<bytes>",
        "list<string>[4]",
        "list<list<uint8>>[2]",
        "list<uint8>",
    ] {
        let refused = refusal(&document("", &one_entry(ty)));
        assert_eq!(
            code_of(&refused.error),
            "validation/attribute-rule-violated",
            "{ty}"
        );
        let why = refused.error.to_string();
        assert!(
            why.contains("entry 'v'") && why.contains("has no bound"),
            "{ty}: {why}"
        );
    }
}

#[test]
fn a_union_import_is_read_by_a_cbor_codec() {
    let m = codec(&document(IMPORTS, &one_entry("union:Payment")));
    assert_eq!(
        m.cbor_entries[0].sce_type,
        ValueType::Union {
            alias: "Payment".to_string()
        }
    );
    // A union no import declares is refused with the unions the document
    // imports as its candidates.
    let refused = refusal(&document(IMPORTS, &one_entry("union:Missing")));
    assert_eq!(code_of(&refused.error), "validation/invalid-attribute");
    assert!(
        refused.error.to_string().contains("union:Payment"),
        "{}",
        refused.error
    );
}

#[test]
fn a_content_line_entry_still_refuses_every_composition() {
    // docs/adr/0101, section 1.4: a content-line codec keeps its entry type.
    for ty in ["list<uint8>[4]", "record:Customer", "string[8]"] {
        let text = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:encoding="content-line" sce:codec-format="2" sce:component="VEVENT" name="probe_any" version="1.0">
{IMPORTS}
  <datamodel>
    <data id="raw" sce:type="bytes" sce:direction="in"/>
    <data id="v" sce:type="{}" sce:property="V"/>
  </datamodel>
</scxml>
"#,
            ty.replace('<', "&lt;").replace('>', "&gt;")
        );
        let refused = refusal(&text);
        assert_eq!(
            code_of(&refused.error),
            "validation/invalid-attribute",
            "{ty}"
        );
    }
}

#[test]
fn every_backend_refuses_by_name_an_entry_its_printer_does_not_lower() {
    // docs/adr/0101, section 5.4: a backend that does not print these yet
    // refuses the codec naming the entry and the type, and no generated file
    // changes; Rust, the reference, prints them (Landing order step 4a).
    for ty in [
        "int8",
        "float64",
        "list<uint8>[4]",
        "list<optional<uint16>>[3]",
    ] {
        let text = document("", &one_entry(ty));
        for &lang in Language::ALL {
            if cbor_codec::lowers_every_entry_type(lang) {
                compile_forge_with_imports(&text, label(), lang, &imports_dir(), &options())
                    .unwrap_or_else(|e| panic!("{lang:?} prints an entry of `{ty}`: {}", e.error));
                continue;
            }
            let refused =
                compile_forge_with_imports(&text, label(), lang, &imports_dir(), &options())
                    .err()
                    .unwrap_or_else(|| panic!("{lang:?} refuses an entry of `{ty}`"));
            assert_eq!(
                code_of(&refused.error),
                "generate/unsupported-feature",
                "{lang:?}: {ty}"
            );
            let why = refused.error.to_string();
            assert!(
                why.contains("CBOR entry 'v'") && why.contains(&format!("`{ty}`")),
                "{lang:?}: {why}"
            );
        }
    }
}

/// The Rust module of a codec of the corpus's imports whose one entry `v` is
/// typed `ty`, stating that its encode may fail where `may_fail`.
fn rust_of(ty: &str, may_fail: bool) -> String {
    let stated = if may_fail {
        r#" sce:encode-may-fail="true""#
    } else {
        ""
    };
    let text = document_stating(stated, IMPORTS, &one_entry(ty));
    compile_forge_with_imports(&text, label(), Language::Rust, &imports_dir(), &options())
        .map(|output| {
            output
                .files
                .into_iter()
                .map(|(_, code)| code)
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_else(|e| panic!("Rust prints an entry of `{ty}`: {}", e.error))
}

#[test]
fn rust_prints_an_entry_of_every_type_of_the_language() {
    // docs/adr/0101, Landing order step 4a: each shape of section 5.1, read and
    // written in its one form of section 2.1. `may_fail` where a value of the
    // type can have no encoding: a list below its lower count, a record's byte
    // string past its bound (the `voucher` case of `Payment`).
    let cases: &[(&str, bool, &[&str])] = &[
        (
            "int8",
            false,
            &[
                "cbor::read_int_within(&mut c, i64::from(i8::MIN), i64::from(i8::MAX))? as i8",
                "cbor::write_int(w, i64::from(v))?;",
            ],
        ),
        (
            "int64",
            false,
            &[
                "cbor::read_int_within(&mut c, i64::MIN, i64::MAX)?",
                "cbor::write_int(w, v)?;",
            ],
        ),
        (
            "float32",
            false,
            &[
                "cbor::read_float32(&mut c)?",
                "cbor::write_float(w, f64::from(v))?;",
            ],
        ),
        (
            "optional<float64>",
            false,
            &["cbor::read_float(&mut c)?", "cbor::write_float(w, v)?;"],
        ),
        (
            "list<optional<uint16>>[3]",
            false,
            &[
                "pub v: ::sce_forge_runtime::codec::BoundedList<Option<u16>, 3>,",
                "if cbor::take_null(&mut c)? {",
                "None => cbor::write_null(w)?,",
            ],
        ),
        (
            "list<list<uint8>[2]>[2]",
            false,
            &[
                "let len1 = cbor::read_array_len(&mut c)?;",
                "for item1 in item.as_slice() {",
            ],
        ),
        (
            "list<bool>[1..2]",
            true,
            &["if !(1..=2).contains(&len) {", "if len < 1 {"],
        ),
        (
            "record:Customer",
            false,
            &[
                "pub v: order_customer::OrderCustomerPayload,",
                "fn decode_record_order_customer(c: &mut SceCursor<'_>) -> Result<order_customer::OrderCustomerPayload, CodecError> {",
                "encode_record_order_customer(w, v)?;",
            ],
        ),
        (
            "list<record:Line>[4]",
            false,
            &[
                "paint_color::PaintColor::from_underlying(cbor::read_uint_upto(c, u64::from(u8::MAX))? as u8).ok_or(CodecError::UndeclaredEnumValue)?",
                "cbor::write_uint(w, u64::from(record.color.to_underlying()))?;",
            ],
        ),
        (
            "union:Payment",
            true,
            &[
                // The case records the union names, which the codec does not
                // import itself.
                "use super::card_payment;",
                "use super::voucher_payment;",
                "match cbor::read_variant(c, &[\"card\", \"cash\", \"voucher\"])? {",
                "cbor::read_empty_case(c)?;",
                "cbor::write_variant(w, \"cash\")?;",
                "if record.voucher_code.len() > 4 {",
                "check_union_order_payment(v)?;",
            ],
        ),
        (
            "optional<union:Payment>",
            true,
            &["pub v: Option<order_payment::OrderPayment>,"],
        ),
    ];
    for &(ty, may_fail, expected) in cases {
        let rust = rust_of(ty, may_fail);
        for expected in expected {
            assert!(
                rust.contains(expected),
                "{ty}: missing `{expected}`:\n{rust}"
            );
        }
    }
    // A map that requires a record or a union has no value by default.
    assert!(!rust_of("record:Customer", false).contains("pub fn new() -> Self"));
    assert!(rust_of("optional<record:Customer>", false).contains("pub fn new() -> Self"));
}

#[test]
fn a_record_of_no_field_is_refused_at_any_depth_alike_for_every_language() {
    // docs/adr/0101, section 5.3: a record is held in its schema's payload
    // type, and a schema of no field declares none, so a codec that holds one
    // at any depth is refused by name before any language renders, whether or
    // not the backend prints every shape.
    let dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/event_schema_payload_free");
    let text = document(
        r#"  <sce:import kind="event-schema" src="job_requested.scxml" as="Requested"/>"#,
        &one_entry("list<optional<record:Requested>>[2]"),
    );
    for &lang in Language::ALL {
        let refused = compile_forge_with_imports(&text, label(), lang, &dir, &options())
            .err()
            .unwrap_or_else(|| panic!("{lang:?} refuses a record of no field"));
        assert_eq!(
            code_of(&refused.error),
            "generate/unsupported-feature",
            "{lang:?}"
        );
        let why = refused.error.to_string();
        assert!(
            why.contains(
                "CBOR entry 'v' holds a record of `job_requested`, which declares no field"
            ),
            "{lang:?}: {why}"
        );
    }
}

/// A type whose largest value is past 2^64 - 1 bytes in CBOR and in JSON: three
/// lists of 2^32 - 1 elements, each element at least a byte.
const HUGE: &str = "list<list<list<uint64>[4294967295]>[4294967295]>[4294967295]";

#[test]
fn a_cbor_entry_past_the_largest_size_is_refused_as_an_overflow() {
    // docs/adr/0101, section 4.4: one fact, one code, where the codec saturated
    // to `u64::MAX` before.
    let text = document("", &one_entry(HUGE));
    let refused =
        compile_forge_with_imports(&text, label(), Language::Rust, &imports_dir(), &options())
            .err()
            .expect("a codec of no size the build counts is refused");
    assert_eq!(
        code_of(&refused.error),
        "validation/encoded-size-overflow",
        "{}",
        refused.error
    );
    let diagnostic = &refused.error.to_diagnostics()[0];
    assert_eq!(diagnostic.actual.as_deref(), Some(HUGE));
    assert!(
        refused
            .error
            .to_string()
            .contains("entry 'v' of codec 'probe_any'"),
        "{}",
        refused.error
    );
}

#[test]
fn an_event_payload_past_the_largest_size_is_refused_as_an_overflow() {
    let text = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="event-schema" sce:event-name="probe.huge" name="probe_huge" version="1.0">
  <datamodel>
    <data id="v" sce:type="{}" sce:direction="in"/>
  </datamodel>
</scxml>
"#,
        HUGE.replace('<', "&lt;").replace('>', "&gt;")
    );
    let label = DocumentLabel {
        identifier: "probe_huge",
        diagnostic_label: "probe_huge",
    };
    let refused =
        compile_forge_with_imports(&text, label, Language::Rust, &imports_dir(), &options())
            .err()
            .expect("a payload of no size the build counts is refused");
    assert_eq!(
        code_of(&refused.error),
        "validation/encoded-size-overflow",
        "{}",
        refused.error
    );
}

#[test]
fn a_leaf_entry_keeps_the_view_the_printers_read() {
    let m = codec(&document(
        IMPORTS,
        r#"    <data id="n" sce:type="optional&lt;uint16&gt;" sce:key="0" sce:direction="out"/>
    <data id="note" sce:type="string[8]" sce:key="1" sce:direction="out"/>
    <data id="color" sce:type="enum:Color" sce:key="2" sce:direction="out"/>"#,
    ));
    let [n, note, color] = &m.cbor_entries[..] else {
        panic!("three entries")
    };
    assert_eq!(n.leaf_type(), Some(&SceType::Uint16));
    assert!(!n.is_required());
    assert_eq!(note.leaf_type(), Some(&SceType::String));
    assert_eq!(note.size_bound(), Some(8));
    assert_eq!(note.storage_bytes(), Some(8));
    assert!(matches!(color.leaf_type(), Some(SceType::Enum(r)) if r.alias == "Color"));
    assert_eq!(color.storage_bytes(), None);
}
