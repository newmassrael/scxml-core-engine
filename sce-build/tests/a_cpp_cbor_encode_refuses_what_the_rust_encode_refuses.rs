// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C++ printer prints a CBOR map whose encode can refuse a value
//! (docs/adr/0081 step 4), and refuses what the Rust printer refuses: the same
//! `CodecError::CborOutOfRange` at the same entries, each found before the
//! first byte is written, so a refusal leaves the sink untouched.
//!
//! The totality analysis names two such values (`codec_totality`, the CBOR
//! arm): a list of fewer records than its lower count, and a record whose byte
//! string, which the schema's unit holds in the language's own vector, is past
//! its bound. Each has a codec of `tests/fixtures/cbor_codec`, generated here
//! with no toolchain; what the printed C++ does with real bytes is held by
//! `backends/cpp/forge-runtime/tests/conformance/codec_source_generated_test.cpp`.
//! A negative value of an enum whose carrier is signed is no such value: both
//! encodes write it as the integer it is, major type 1 below zero
//! (docs/adr/0101, section 2.1).

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{compile_forge_with_imports, DocumentLabel, ForgeCompileOptions};

/// The refusal every check of the printed C++ encode returns.
const REFUSED: &str = "return ::SCE::Forge::CodecError::CborOutOfRange;";

/// The call of the check of a record or a union document the encode makes,
/// whose refusal it returns as it is (docs/adr/0101, section 5.1).
const CHECKED: &str = "if (auto _e = Composites::check_";

/// The map head the printed C++ encode writes first: every check is before it.
const MAP_HEAD: &str = "if (auto _e = cbor::write_map_head(w, _count); _e) return _e;";

/// The vector of the bytes a printed value answers where its encode can refuse.
const REFUSING_TO_VEC: &str =
    "[[nodiscard]] std::optional<std::vector<std::uint8_t>> encode_to_vec() const {";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cbor_codec")
}

fn document(stem: &str) -> PathBuf {
    fixtures().join(format!("{stem}.scxml"))
}

/// What `lang` generates for the fixture `stem`: the text of every file.
fn generated(stem: &str, lang: Language) -> String {
    let text = std::fs::read_to_string(document(stem))
        .unwrap_or_else(|e| panic!("read {stem}.scxml: {e}"));
    compile_forge_with_imports(
        &text,
        DocumentLabel {
            identifier: stem,
            diagnostic_label: stem,
        },
        lang,
        &fixtures(),
        &ForgeCompileOptions::default(),
    )
    .map(|output| {
        output
            .files
            .into_iter()
            .map(|(_, code)| code)
            .collect::<Vec<_>>()
            .join("\n")
    })
    .unwrap_or_else(|e| panic!("{stem} on {lang:?}: {}", e.error))
}

/// The C++ of the fixture `stem`, which the printer prints from its plan.
fn printed_cpp(stem: &str) -> String {
    let unlowered =
        sce_build::cpp_codec_unlowered_file(&document(stem), &[], &ForgeCompileOptions::default())
            .unwrap_or_else(|e| panic!("{stem}: {e:?}"))
            .expect("the fixture is a codec");
    assert!(unlowered.is_empty(), "{stem}: {unlowered:?}");
    generated(stem, Language::Cpp)
}

/// The encode of the printed map, its `Wire` struct: where an encode refuses.
/// The decode beside it refuses frames with the same kinds (docs/adr/0099,
/// amendment of 2026-11-16), which say nothing of the encode.
fn wire_of<'c>(stem: &str, code: &'c str) -> &'c str {
    let start = code
        .find("struct Wire {")
        .unwrap_or_else(|| panic!("{stem}: no encode:\n{code}"));
    let wire = &code[start..];
    let end = wire
        .find("\n};")
        .unwrap_or_else(|| panic!("{stem}: the encode does not close:\n{code}"));
    &wire[..end]
}

/// Each of `expected` is in `code`, and its encode refuses `refusals` times,
/// each before the map head, by a refusal of its own or by the check of a
/// record's document; its value answers its bytes as an optional.
fn refuses_before_the_first_byte(stem: &str, code: &str, expected: &[&str], refusals: usize) {
    for expected in expected {
        assert!(
            code.contains(expected),
            "{stem}: missing `{expected}`:\n{code}"
        );
    }
    let wire = wire_of(stem, code);
    assert_eq!(
        wire.matches(REFUSED).count() + wire.matches(CHECKED).count(),
        refusals,
        "{stem}:\n{code}"
    );
    let head = wire
        .find(MAP_HEAD)
        .unwrap_or_else(|| panic!("{stem}: no map head:\n{code}"));
    let last = [wire.rfind(REFUSED), wire.rfind(CHECKED)]
        .into_iter()
        .flatten()
        .max()
        .expect("a refusal");
    assert!(
        last < head,
        "{stem}: a refusal is checked after the map head is written:\n{code}"
    );
    assert!(code.contains(REFUSING_TO_VEC), "{stem}:\n{code}");
}

#[test]
fn a_negative_value_of_a_signed_carrier_is_written_on_cpp_as_on_rust() {
    // docs/adr/0101, section 2.1: an enum is the integer of its declared
    // number, major type 1 below zero, so neither encode refuses one and the
    // value answers its bytes, not an optional of them.
    let stem = "codec_cbor_signed_enum";
    let code = printed_cpp(stem);
    let wire = wire_of(stem, &code);
    assert!(!wire.contains(REFUSED), "{stem}:\n{code}");
    assert_eq!(
        wire.matches("if (auto _e = cbor::write_int(w, static_cast<std::int64_t>(")
            .count(),
        2,
        "{stem}: both entries are written as integers of either sign:\n{code}"
    );
    assert!(!code.contains(REFUSING_TO_VEC), "{stem}:\n{code}");
    // Each read takes a number of either sign within the carrier's range.
    assert_eq!(
        code.matches("cbor::read_int_within(c, static_cast<std::int64_t>(std::numeric_limits<")
            .count(),
        2,
        "{stem}:\n{code}"
    );
    let rust = generated(stem, Language::Rust);
    assert_eq!(
        rust.matches("cbor::write_int(w, i64::from(").count(),
        2,
        "the Rust encode writes both entries as integers of either sign:\n{rust}"
    );
    assert_eq!(
        rust.matches("cbor::read_int_within(&mut c, i64::from(")
            .count(),
        2,
        "the Rust decode reads both entries within the carrier's range:\n{rust}"
    );
    assert!(!rust.contains("u64::try_from("), "{rust}");
}

#[test]
fn a_list_of_fewer_records_than_its_lower_count_is_refused_on_cpp_as_on_rust() {
    let stem = "codec_cbor_record_list_least";
    let code = printed_cpp(stem);
    let payload = "SCE::Generated::CborCausalEntry::CborCausalEntryPayload";
    // The list holds the most it holds and not its lower count, as Rust's
    // `BoundedList` does, so the value can hold fewer and the encode refuses it.
    let seen =
        format!("[[nodiscard]] const ::SCE::Forge::OwnedList<{payload}, 2>& seen() const noexcept");
    let context = format!(
        "[[nodiscard]] const std::optional<::SCE::Forge::OwnedList<{payload}, 3>>& context() const \
         noexcept"
    );
    refuses_before_the_first_byte(
        stem,
        &code,
        &[
            seen.as_str(),
            context.as_str(),
            "            const auto& v = _e_seen;\n            if (v.size() < 1) return ::SCE::Forge::CodecError::CborOutOfRange;",
            "        if (_e_context) {\n            const auto& v = *_e_context;\n            if (v.size() < 1) return ::SCE::Forge::CodecError::CborOutOfRange;",
            // The decode refuses a frame of fewer records before any is read.
            "if (*_len0 > 2 || *_len0 < 1) return ::SCE::Forge::CodecError::CborOutOfRange;",
            "if (*_len0 > 3 || *_len0 < 1) return ::SCE::Forge::CodecError::CborOutOfRange;",
        ],
        2,
    );
    assert!(!code.contains(", 2, 1>"), "{stem}:\n{code}");
    assert!(!code.contains(", 3, 1>"), "{stem}:\n{code}");
    let rust = generated(stem, Language::Rust);
    assert_eq!(
        rust.matches("if len < 1 {").count(),
        2,
        "the Rust encode refuses both lists:\n{rust}"
    );
}

#[test]
fn a_record_byte_string_past_its_bound_is_refused_on_cpp_as_on_rust() {
    let stem = "codec_cbor_record_list_bytes";
    let code = printed_cpp(stem);
    // Each record of either list is held to the one check of its document
    // (docs/adr/0101, section 5.1), which each list calls before the map head
    // and whose refusal it returns.
    let each_record = "for (const auto& _item0 : v) {\n                \
                       if (auto _e = Composites::check_record_cbor_digest_entry(_item0); _e) return _e;\n            }";
    refuses_before_the_first_byte(
        stem,
        &code,
        &[
            format!("            const auto& v = _e_latest;\n            {each_record}").as_str(),
            format!("        if (_e_pending) {{\n            const auto& v = *_e_pending;\n            {each_record}").as_str(),
            // The record holds the byte string in the schema's vector, read
            // within its bound (a read answers `Decoded`, its refusal kind
            // handed back as it is), written as it is, and held to its bound
            // by its document's check.
            "        std::optional<std::vector<uint8_t>> held_digest;\n",
            "auto _r0 = cbor::read_bytes(c, 4);\n                if (!_r0) return _r0.error();\n                held_digest = std::move(*_r0);",
            "if (auto _e = cbor::write_bytes(w, record.digest); _e) return _e;",
            "static std::optional<::SCE::Forge::CodecError> check_record_cbor_digest_entry(\
             const SCE::Generated::CborDigestEntry::CborDigestEntryPayload& record) noexcept {\n        \
             if (record.digest.size() > 4) return ::SCE::Forge::CodecError::CborOutOfRange;\n        \
             return std::nullopt;\n",
        ],
        2,
    );
    assert_eq!(
        code.matches("if (record.digest.size() > 4)").count(),
        1,
        "{stem}: a record of `cbor_digest_entry` is checked in one place:\n{code}"
    );
    // Rust holds every record of either list to the one check of its
    // document (docs/adr/0101, section 5.1), which each list calls.
    let rust = generated(stem, Language::Rust);
    assert_eq!(
        rust.matches("if record.digest.len() > 4 {").count(),
        1,
        "the Rust encode checks a record of `cbor_digest_entry` in one place:\n{rust}"
    );
    assert_eq!(
        rust.matches("check_record_cbor_digest_entry(item)?;")
            .count(),
        2,
        "the Rust encode refuses a record of either list:\n{rust}"
    );
}

#[test]
fn a_map_whose_encode_refuses_nothing_answers_its_bytes_as_they_are() {
    // `codec_cbor_record_list` holds a record of a text, which the schema's
    // unit holds within its bound, and lists with no lower count.
    let stem = "codec_cbor_record_list";
    let code = printed_cpp(stem);
    // The encode, not the decode: a decode answers `CborOutOfRange` for a
    // value its bound does not hold (docs/adr/0099, amendment of 2026-11-16).
    assert!(!wire_of(stem, &code).contains(REFUSED), "{stem}:\n{code}");
    assert!(!code.contains(REFUSING_TO_VEC), "{stem}:\n{code}");
    for expected in [
        "[[nodiscard]] std::vector<std::uint8_t> encode_to_vec() const {",
        "(void)encode(sink);",
        "shortest form (RFC 8949 §4.2.1). It refuses nothing; what it returns is",
    ] {
        assert!(
            code.contains(expected),
            "{stem}: missing `{expected}`:\n{code}"
        );
    }
}
