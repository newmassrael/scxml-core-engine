// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C++ printer prints a CBOR map whose encode can refuse a value
//! (docs/adr/0081 step 4), and refuses what the Rust printer refuses: the same
//! `CodecError::CborOutOfRange` at the same entries, each found before the
//! first byte is written, so a refusal leaves the sink untouched.
//!
//! The totality analysis names three such values (`codec_totality`, the CBOR
//! arm): a negative value of an enum whose carrier is signed, a list of fewer
//! records than its lower count, and a record whose byte string, which the
//! schema's unit holds in the language's own vector, is past its bound. Each
//! has a codec of `tests/fixtures/cbor_codec`, generated here with no
//! toolchain; what the printed C++ does with real bytes is held by
//! `backends/cpp/forge-runtime/tests/conformance/codec_source_generated_test.cpp`.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{compile_forge_with_imports, DocumentLabel, ForgeCompileOptions};

/// The refusal every check of the printed C++ encode returns.
const REFUSED: &str = "return ::SCE::Forge::CodecError::CborOutOfRange;";

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

/// Each of `expected` is in `code`, and `code` refuses `refusals` times, each
/// before the map head; its value answers its bytes as an optional.
fn refuses_before_the_first_byte(stem: &str, code: &str, expected: &[&str], refusals: usize) {
    for expected in expected {
        assert!(
            code.contains(expected),
            "{stem}: missing `{expected}`:\n{code}"
        );
    }
    assert_eq!(code.matches(REFUSED).count(), refusals, "{stem}:\n{code}");
    let head = code
        .find(MAP_HEAD)
        .unwrap_or_else(|| panic!("{stem}: no map head:\n{code}"));
    let last = code.rfind(REFUSED).expect("a refusal");
    assert!(
        last < head,
        "{stem}: a refusal is checked after the map head is written:\n{code}"
    );
    assert!(code.contains(REFUSING_TO_VEC), "{stem}:\n{code}");
}

#[test]
fn a_negative_value_of_a_signed_carrier_is_refused_on_cpp_as_on_rust() {
    let stem = "codec_cbor_signed_enum";
    let code = printed_cpp(stem);
    refuses_before_the_first_byte(
        stem,
        &code,
        &[
            // The required entry read in a block of its own, the optional one
            // where the value holds it.
            "        {\n            const auto& v = _e_status;\n            if (",
            "        if (_e_detail) {\n            const auto& v = *_e_detail;\n            if (",
            // What is left is no negative number, so it is the unsigned
            // integer it is written as.
            "if (auto _e = cbor::write_uint(w, static_cast<std::uint64_t>(",
        ],
        2,
    );
    // Both refusals are of a negative carrier value.
    assert_eq!(
        code.matches("to_underlying(v) < 0) return ::SCE::Forge::CodecError::CborOutOfRange;")
            .count(),
        2,
        "{stem}:\n{code}"
    );
    let rust = generated(stem, Language::Rust);
    assert_eq!(
        rust.matches(".map_err(|_| CodecError::CborOutOfRange)?)?;")
            .count(),
        2,
        "the Rust encode refuses both entries:\n{rust}"
    );
}

#[test]
fn a_list_of_fewer_records_than_its_lower_count_is_refused_on_cpp_as_on_rust() {
    let stem = "codec_cbor_record_list_least";
    let code = printed_cpp(stem);
    let payload = "SCE::Generated::CborCausalEntry::CborCausalEntryPayload";
    // The view holds the most a list holds and not its lower count, as Rust's
    // `BoundedList` does, so the value can hold fewer and the encode refuses it.
    let seen = format!("[[nodiscard]] ::SCE::Forge::List<{payload}, 2> seen() const noexcept");
    let context = format!(
        "[[nodiscard]] std::optional<::SCE::Forge::List<{payload}, 3>> context() const noexcept"
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
            "if (!_len || *_len > 2 || *_len < 1) return std::nullopt;",
            "if (!_len || *_len > 3 || *_len < 1) return std::nullopt;",
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
    let each_record = "for (const auto& _record : v) {\n                if (_record.digest.size() > 4) return ::SCE::Forge::CodecError::CborOutOfRange;\n            }";
    refuses_before_the_first_byte(
        stem,
        &code,
        &[
            format!("            const auto& v = _e_latest;\n            {each_record}").as_str(),
            format!("        if (_e_pending) {{\n            const auto& v = *_e_pending;\n            {each_record}").as_str(),
            // The record holds the byte string in the schema's vector, read
            // within its bound and written as it is.
            "        std::optional<std::vector<uint8_t>> held_digest;\n",
            "held_digest = cbor::read_bytes(c, 4);",
            "if (auto _e = cbor::write_bytes(w, record.digest); _e) return _e;",
        ],
        2,
    );
    let rust = generated(stem, Language::Rust);
    assert_eq!(
        rust.matches("if record.digest.len() > 4 {").count(),
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
    assert!(!code.contains(REFUSED), "{stem}:\n{code}");
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
