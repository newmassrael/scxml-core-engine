// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A Go codec holds a value that may be absent in the one type every Go surface
//! of SCE spells it with (docs/adr/0055, section 8 "One representation").
//!
//! The codecs' optional entries and gated fields were a pointer, and a gated
//! list or byte string was the slice's own nilness. A statechart's variable of
//! `optional<T>` is `optional.Of[T]` (section 5), so a value read from a codec
//! and handed to a variable was two types, and an empty list that was given and
//! a list that was not were one value. This holds the generator to the one
//! type, for each kind of entry that may be absent:
//!
//! - a CBOR map's `optional<T>` entry, and a content-line codec's entry and
//!   parameter, in a record of lines too;
//! - a positional codec's field gated by `sce:present-if`, of every layout it
//!   admits: fixed width, variable width, a tail, a length-ref byte string or
//!   text, a repeat, a TLV chain and an embedded codec.
//!
//! What the Go does with the type (the decode into a statechart's variable, the
//! encode from one, a present `false` or zero told from absence) is held by
//! `backends/go/forge-runtime/round_trip/optional_field_test.go`; this holds
//! what the generator writes, with no toolchain: the field is declared in the
//! type, no presence test reads a pointer or a slice's nilness, and the file
//! imports the package it names only when it names it.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{compile_forge_with_imports, DocumentLabel, ForgeCompileOptions};

const OPTIONAL_IMPORT: &str = "\"github.com/newmassrael/sce-forge-runtime/optional\"";

fn resource_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources")
}

/// The Go a document generates as: the file of the package that carries its
/// name.
fn go_of(document: &str) -> String {
    let text = std::fs::read_to_string(resource_dir().join(format!("{document}.scxml")))
        .unwrap_or_else(|e| panic!("read {document}.scxml: {e}"));
    let options = ForgeCompileOptions {
        go_module_prefix: Some("example.com/sce-forge".to_string()),
        ..ForgeCompileOptions::default()
    };
    let output = compile_forge_with_imports(
        &text,
        DocumentLabel::symmetric(document),
        Language::Go,
        &resource_dir(),
        &options,
    )
    .unwrap_or_else(|e| panic!("{document}: {}", e.error));
    output
        .files
        .into_iter()
        .find(|(file, _)| {
            file.ends_with(".go") && Path::new(file).file_stem().is_some_and(|s| s == document)
        })
        .map(|(_, source)| source)
        .unwrap_or_else(|| panic!("{document}: no Go file for the document itself"))
}

/// The lines of the struct types `source` declares, between `type X struct {`
/// and the `}` that closes it, comments left out.
fn struct_fields(source: &str) -> Vec<&str> {
    let mut inside = false;
    let mut fields = Vec::new();
    for line in source.lines() {
        if line.starts_with("type ") && line.ends_with(" struct {") {
            inside = true;
        } else if inside && line == "}" {
            inside = false;
        } else if inside && line.starts_with('\t') && !line.trim_start().starts_with("//") {
            fields.push(line);
        }
    }
    fields
}

/// Every struct field of `source` that is a pointer: its type starts with `*`.
fn pointer_fields(source: &str) -> Vec<&str> {
    struct_fields(source)
        .into_iter()
        .filter(|line| {
            line.trim_start()
                .split(' ')
                .nth(1)
                .is_some_and(|t| t.starts_with('*'))
        })
        .collect()
}

/// A line of `source` that reads presence from a pointer or a slice: `!= nil`
/// or `== nil` on anything but an `err`, a content line or a variant's arm.
fn presence_by_nil(source: &str) -> Vec<&str> {
    source
        .lines()
        .filter(|line| {
            (line.contains("!= nil") || line.contains("== nil"))
                && !line.contains("err != nil")
                && !line.contains("err == nil")
                && !line.contains("line == nil")
                && !line.contains(".Body.")
        })
        .collect()
}

/// A document's Go declares each of `declarations` as a field of its types and
/// holds every value that may be absent by the one type, never by a pointer or a
/// slice's nilness.
fn assert_holds_by_the_optional_type(document: &str, source: &str, declarations: &[&str]) {
    assert!(
        source.contains(OPTIONAL_IMPORT),
        "{document}: names optional.Of and imports no optional package:\n{source}"
    );
    for declaration in declarations {
        assert!(
            struct_fields(source)
                .iter()
                .any(|line| *line == format!("\t{declaration}")),
            "{document}: no field `{declaration}` in the generated types:\n{source}"
        );
    }
    assert!(
        pointer_fields(source).is_empty(),
        "{document}: a field is held by a pointer: {:?}",
        pointer_fields(source)
    );
    assert!(
        presence_by_nil(source).is_empty(),
        "{document}: presence is read from a nil: {:?}",
        presence_by_nil(source)
    );
    assert!(
        !source.contains("= &_v")
            && !source.contains("Held != nil")
            && !source.contains("Held == nil"),
        "{document}: a value is taken by its address:\n{source}"
    );
}

/// A CBOR map's entry that may be absent is the optional type, a required one is
/// its bare type, and the decode tells a key given twice and a key never given
/// through the same type.
#[test]
fn a_cbor_entry_that_may_be_absent_is_the_one_optional_type() {
    let source = go_of("codec_cbor_map");
    assert_holds_by_the_optional_type(
        "codec_cbor_map",
        &source,
        &["Deadline optional.Of[uint64]", "Urgent optional.Of[bool]"],
    );
    for required in [
        "Source string",
        "Pattern uint8",
        "Payload []byte",
        "Id []byte",
    ] {
        assert!(
            struct_fields(&source)
                .iter()
                .any(|line| *line == format!("\t{required}")),
            "a required entry is its bare type, `{required}`:\n{source}"
        );
    }
    for line in [
        "var deadlineHeld optional.Of[uint64]",
        "deadlineHeld = optional.Some(v)",
        "if deadlineHeld.IsPresent() {",
        "if s.Deadline.IsPresent() {",
        "v := s.Deadline.Proven()",
        "Deadline: deadlineHeld,",
        // A required entry is read through the same held value and handed over
        // as the value it was proven to be.
        "if !sourceHeld.IsPresent() {",
        "Source: sourceHeld.Proven(),",
    ] {
        assert!(
            source.contains(line),
            "the CBOR codec lacks `{line}`:\n{source}"
        );
    }
}

/// A content-line codec's entry and its parameter that may be absent are the
/// optional type; a list is a slice and a required value is its bare type.
#[test]
fn a_content_line_entry_or_parameter_that_may_be_absent_is_the_one_optional_type() {
    let source = go_of("codec_content_line_event");
    assert_holds_by_the_optional_type(
        "codec_content_line_event",
        &source,
        &[
            "Summary optional.Of[string]",
            "OrganizerCn optional.Of[string]",
            "Sequence optional.Of[uint16]",
            "Priority optional.Of[int32]",
            "AllDay optional.Of[bool]",
            "Exdate []string",
            "Uid string",
        ],
    );
    for line in [
        "out.Summary = optional.Some(v)",
        "out.OrganizerCn = optional.Some(pv)",
        "if s.Summary.IsPresent() {",
        "v := s.Summary.Proven()",
        // A parameter with no value of its property has no line to stand on.
        "} else if s.OrganizerCn.IsPresent() {",
        "out.Param(\"CN\", s.OrganizerCn.Proven(), 24)",
    ] {
        assert!(
            source.contains(line),
            "the content-line codec lacks `{line}`:\n{source}"
        );
    }
}

/// A line record's parameter that may be absent is the optional type as well,
/// read into the record of the line and written from it.
#[test]
fn a_line_records_parameter_that_may_be_absent_is_the_one_optional_type() {
    let source = go_of("codec_content_line_records");
    assert_holds_by_the_optional_type(
        "codec_content_line_records",
        &source,
        &[
            "AttendeeCn optional.Of[string]",
            "ExdateTzid optional.Of[string]",
        ],
    );
    for line in [
        "rec.AttendeeCn = optional.Some(pv)",
        "if rec.AttendeeCn.IsPresent() {",
        "out.Param(\"CN\", rec.AttendeeCn.Proven(), 24)",
    ] {
        assert!(
            source.contains(line),
            "the record's parameter lacks `{line}`:\n{source}"
        );
    }
}

/// A codec none of whose values may be absent names no optional package and
/// imports none: an unused import is a build error in Go.
#[test]
fn a_codec_with_no_value_that_may_be_absent_imports_no_optional_package() {
    for document in ["codec_simple_frame", "codec_tail", "codec_vle_zint_u64"] {
        let source = go_of(document);
        let code: Vec<&str> = source
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect();
        assert!(
            !source.contains(OPTIONAL_IMPORT)
                && !code.iter().any(|line| line.contains("optional.")),
            "{document}: has nothing that may be absent and names the optional package:\n{source}"
        );
    }
}

/// A positional field gated by `sce:present-if` is the optional type for every
/// layout the gate admits, handed over with `optional.Some` where the gate holds
/// and read through `Proven()` only under the test that proves it present.
#[test]
fn a_gated_field_is_the_one_optional_type_in_every_layout() {
    // (document, the declarations its gated fields make, a line its decode
    // writes, a line its encode writes)
    let cases: &[(&str, &[&str], &str, &str)] = &[
        (
            "codec_present_if_basic",
            &["Seq optional.Of[uint16]"],
            "Seq = optional.Some(_v)",
            "_v := s.Seq.Proven()",
        ),
        (
            "codec_present_if_vle",
            &["OptionalId optional.Of[uint64]"],
            "OptionalId = optional.Some(_v)",
            "_v := s.OptionalId.Proven()",
        ),
        (
            "codec_present_if_tail",
            &["Payload optional.Of[[]byte]"],
            "Payload = optional.Some(append([]byte(nil), raw...))",
            "w.WriteBytes(s.Payload.Proven())",
        ),
        (
            "codec_present_if_length_ref",
            &["Payload optional.Of[[]byte]"],
            "Payload = optional.Some(append([]byte(nil), raw...))",
            "w.WriteBytes(s.Payload.Proven())",
        ),
        (
            "codec_present_if_string",
            &["TextLen optional.Of[uint8]", "Text optional.Of[string]"],
            "Text = optional.Some(string(raw))",
            "w.WriteBytes([]byte(s.Text.Proven()))",
        ),
        (
            "codec_repeat_present_if_basic",
            &["Elems optional.Of[[]codec_repeat_elem.CodecRepeatElem]"],
            "Elems = optional.Some(_list)",
            "_list := s.Elems.Proven()",
        ),
        (
            "codec_tlv_chain_present_if_basic",
            &["Entries optional.Of[[]codec_tlv_entry.CodecTlvEntry]"],
            "Entries = optional.Some(_list)",
            "_list := s.Entries.Proven()",
        ),
        (
            "codec_zenoh_msg_put",
            &[
                "Timestamp optional.Of[codec_zenoh_timestamp.CodecZenohTimestamp]",
                "Encoding optional.Of[codec_zenoh_encoding.CodecZenohEncoding]",
            ],
            "Timestamp = optional.Some(*_emb)",
            "_body := s.Timestamp.Proven()",
        ),
    ];
    for (document, declarations, decoded, encoded) in cases {
        let source = go_of(document);
        assert_holds_by_the_optional_type(document, &source, declarations);
        assert!(
            source.contains(decoded),
            "{document}: the decode lacks `{decoded}`:\n{source}"
        );
        assert!(
            source.contains(encoded),
            "{document}: the encode lacks `{encoded}`:\n{source}"
        );
    }
}

/// A gated field read as a length or a count is read where the same gate proves
/// it present, through the optional type's own read and not a dereference; a
/// chain that is itself gated is walked where it is present, and the encoder's
/// check of a gated field asks the type, not a nil, whether it was given.
#[test]
fn a_gated_length_or_count_or_chain_is_read_through_the_optional_type() {
    let source = go_of("codec_init_cookie_body");
    assert!(
        source.contains("int(CookieSize.Proven())"),
        "a gated length-ref's sibling is not read through Proven():\n{source}"
    );
    assert!(
        !source.contains("int(*"),
        "a gated sibling is dereferenced:\n{source}"
    );

    let source = go_of("codec_chain_has_marker");
    assert!(
        source.contains("_n := SliceCount.Proven()"),
        "a gated repeat's count is not read through Proven():\n{source}"
    );
    assert!(
        source.contains("if Extensions.IsPresent() {")
            && source.contains("for _, _e := range Extensions.Proven() {"),
        "a gated chain is not walked where it is present:\n{source}"
    );
    assert!(
        source.contains("if !s.PayloadLen.IsPresent() {")
            && source.contains("} else if s.PayloadLen.IsPresent() {"),
        "the encoder's check of a gated field does not ask the optional type:\n{source}"
    );
}
