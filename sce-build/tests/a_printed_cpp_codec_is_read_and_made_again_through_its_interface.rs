// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A procedure or a validator that holds a value of a codec the C++ backend
//! prints from its plan (docs/adr/0081 step 4) reads it and makes it again
//! through the interface the printed value has, and through nothing else.
//!
//! The value is made only by its constructor, which takes the elements the
//! value presents in their declaration order, a group of fields one gate
//! decides whole and the arm of the variant last; a number the encode derives
//! (a length, a count, an encoded length) is no element of the value
//! (docs/adr/0081 §3). Until 2026-11-17 an `<assign>` to one element made the
//! value again from every field of the codec, the derived numbers and the
//! members of a group among them, each read by an accessor the value does not
//! have, and handed a byte string or a text to the constructor as the
//! expression gave it, where the constructor takes its bounded view; a read of
//! a derived number called an accessor of the field's name, which the value
//! does not have either. None of it compiled.
//!
//! What is asserted here is the generated text; the C++ conformance arm
//! compiles and runs the two fixture documents
//! (`printed_codec_reader_generated_test`).

use std::path::{Path, PathBuf};

use sce_build::forge::error::ForgeError;
use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

/// The forge resources, where the codecs read here live.
fn resource_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources")
}

/// The two documents the C++ conformance arm compiles and runs.
fn reader_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/printed_codec_reader")
}

/// The C++ `scxml` generates, every file joined, its imports resolved against
/// `base_dir`; or the refusal it is answered with.
fn generate_in(name: &str, scxml: &str, base_dir: &Path) -> Result<String, ForgeError> {
    sce_build::compile_forge_with_imports(
        scxml,
        DocumentLabel::symmetric(name),
        Language::Cpp,
        base_dir,
        &ForgeCompileOptions::default(),
    )
    .map(|out| {
        out.files
            .into_iter()
            .map(|(_, text)| text)
            .collect::<Vec<_>>()
            .join("\n")
    })
    .map_err(|located| located.error)
}

/// The C++ the fixture document `stem` of [`reader_dir`] generates.
fn fixture(stem: &str) -> String {
    let dir = reader_dir();
    let path = dir.join(format!("{stem}.scxml"));
    let scxml = std::fs::read_to_string(&path).expect("the fixture document is readable");
    generate_in(stem, &scxml, &dir)
        .unwrap_or_else(|e| panic!("{stem}: the fixture generates, but was refused: {e}"))
}

/// The C++ header of the codec `stem` of the resources.
fn codec(stem: &str) -> String {
    let path = resource_dir().join(format!("{stem}.scxml"));
    let scxml = std::fs::read_to_string(&path).expect("the codec document is readable");
    generate_in(stem, &scxml, &resource_dir())
        .unwrap_or_else(|e| panic!("{stem}: the codec generates, but was refused: {e}"))
}

/// A procedure importing the codec `codec` of the resources as `held`, whose
/// one transition makes the assigns `assigns` and whose final state reports
/// `done`.
fn procedure(codec: &str, held: &str, inputs: &str, assigns: &str, done: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="procedure" initial="start" version="1.0">
  <sce:import src="{codec}.scxml" kind="codec" as="{held}"/>
  <datamodel>
{inputs}
  </datamodel>
  <state id="start">
    <transition target="done">
{assigns}
    </transition>
  </state>
  <final id="done">
    <donedata><param name="result" expr="{done}"/></donedata>
  </final>
</scxml>"#
    )
}

/// The generated text of a constructor call of `class` over `arguments`.
fn made_by(class: &str, arguments: &[&str]) -> String {
    format!("{class}({})", arguments.join(", "))
}

const ENVELOPE: &str = "::SCE::Generated::CodecOriginEnvelope::CodecOriginEnvelope";
const ENVELOPE_NS: &str = "::SCE::Generated::CodecOriginEnvelope";

#[test]
fn an_assign_makes_the_value_again_from_the_elements_it_presents() {
    let cpp = fixture("printed_codec_rebuild");
    // `hdr`, `note`, the body always there, the gated body and the list, in
    // their declaration order; `note_len` and `m`, which the encode derives,
    // are not handed, and the gated body is handed on as the value holds it.
    let rebuilt = format!(
        "env_ = {};",
        made_by(
            ENVELOPE,
            &[
                "hdr_",
                "env_.note()",
                "env_.required()",
                "env_.optional()",
                "env_.items()"
            ]
        )
    );
    assert!(
        cpp.contains(&rebuilt),
        "the assign to `env.hdr` is not the rebuild from the presented elements \
         `{rebuilt}`:\n{cpp}"
    );
    for derived in ["env_.note_len()", "env_.m()"] {
        assert!(
            !cpp.contains(derived),
            "the rebuild reads `{derived}`, a number the encode derives and the value \
             has no accessor for:\n{cpp}"
        );
    }
}

#[test]
fn an_assigned_text_is_made_into_its_bounded_value_or_raises_an_error() {
    let cpp = fixture("printed_codec_rebuild");
    // The note is held first, made by `make` of the type the accessor presents
    // it as, which views it, and a refusal raises `error.execution` before the
    // value is made again.
    let made = "const auto _sce_made = std::decay_t<decltype(env_.note())>::make(_sce_value);";
    let at = cpp
        .find(made)
        .unwrap_or_else(|| panic!("the note is not made by its bounded type's `make`:\n{cpp}"));
    let after = &cpp[at..];
    let refused = after
        .find("return Event::ErrorExecution;")
        .expect("a note past its bound raises `error.execution`");
    let rebuilt = format!(
        "env_ = {};",
        made_by(
            ENVELOPE,
            &[
                "env_.hdr()",
                "*_sce_made",
                "env_.required()",
                "env_.optional()",
                "env_.items()"
            ]
        )
    );
    let assigned = after.find(&rebuilt).unwrap_or_else(|| {
        panic!("the made note is not handed to the constructor `{rebuilt}`:\n{cpp}")
    });
    assert!(
        refused < assigned,
        "the value is made again before the refusal of the note is tested:\n{cpp}"
    );
    let held = cpp[..at]
        .rfind("const auto _sce_value = ")
        .expect("the assigned note is held before it is viewed");
    assert!(
        !cpp[held..at].contains('}'),
        "the note `make` views is not the one held just before it:\n{cpp}"
    );
}

#[test]
fn a_derived_number_is_read_as_the_encode_derives_it() {
    let procedure = fixture("printed_codec_rebuild");
    let read = format!("{ENVELOPE_NS}::wire_note_len(env_)");
    assert!(
        procedure.contains(&read),
        "the procedure does not read `env.note_len` as the encode derives it `{read}`:\n{procedure}"
    );
    let validator = fixture("printed_codec_check");
    for read in [
        format!("{ENVELOPE_NS}::wire_m(env_)"),
        format!("{ENVELOPE_NS}::wire_note_len(env_)"),
    ] {
        assert!(
            validator.contains(&read),
            "the validator does not read the derived number as `{read}`:\n{validator}"
        );
    }
    assert!(
        !validator.contains("env_.m()") && !validator.contains("env_.note_len()"),
        "the validator reads a derived number by an accessor the value does not have:\n{validator}"
    );
}

#[test]
fn a_printed_codec_answers_each_number_its_encode_derives_off_any_source() {
    let header = codec("codec_origin_envelope");
    let note_len = "template <class S>\n\
                    [[nodiscard]] uint8_t wire_note_len(const S& src) noexcept(is_nothrow_source<S>::value) {\n\
                    \x20   static_assert(is_source<S>::value, \"S presents every element of CodecOriginEnvelope\");\n\
                    \x20   const auto& _r_note = source::note(src);\n\
                    \x20   const ::SCE::Forge::Text<16> _e_note = _r_note;\n\
                    \x20   return static_cast<uint8_t>(_e_note.size());\n\
                    }\n";
    assert!(
        header.contains(note_len),
        "the envelope does not answer `note_len` from the note it measures:\n{header}"
    );
    let m = "template <class S>\n\
             [[nodiscard]] uint8_t wire_m(const S& src) noexcept(is_nothrow_source<S>::value) {\n\
             \x20   static_assert(is_source<S>::value, \"S presents every element of CodecOriginEnvelope\");\n\
             \x20   const auto& _r_items = source::items(src);\n\
             \x20   return static_cast<uint8_t>(_r_items.size());\n\
             }\n";
    assert!(
        header.contains(m),
        "the envelope does not answer `m` from the entries it counts:\n{header}"
    );
}

#[test]
fn a_length_the_value_states_is_read_as_the_frame_carries_it() {
    // `span_len` measures two gated byte strings, and the value states it
    // where neither is there: the read is the first measure there, else what
    // the value states, else zero.
    let header = codec("codec_length_stated");
    let answer = "    return (_f_span_len.has_value()) ? static_cast<uint8_t>(*_f_span_len) : \
                  _e_span_len.value_or(uint8_t{0});\n";
    assert!(
        header.contains("wire_span_len(const S& src)") && header.contains(answer),
        "the stated length is not answered as the frame carries it `{answer}`:\n{header}"
    );
    let cpp = generate_in(
        "stated_reader",
        &procedure(
            "codec_length_stated",
            "cs",
            r#"    <data id="n" sce:type="uint8" sce:direction="in"/>"#,
            r#"      <assign location="cs.span_len" expr="n"/>"#,
            "cs.span_len",
        ),
        &resource_dir(),
    )
    .expect("a procedure over a codec whose value states a length generates");
    assert!(
        cpp.contains("::SCE::Generated::CodecLengthStated::wire_span_len(cs_)"),
        "the stated length is not read as the frame carries it:\n{cpp}"
    );
    // The value presents the length it states, so it is assigned as an element.
    assert!(
        cpp.contains("cs_ = ::SCE::Generated::CodecLengthStated::CodecLengthStated(cs_.flags(), n_, cs_.alpha(), cs_.beta());"),
        "the stated length is not assigned by the rebuild from the presented elements:\n{cpp}"
    );
}

#[test]
fn an_assign_to_a_derived_number_is_refused() {
    let refused = generate_in(
        "derived_assign",
        &procedure(
            "codec_origin_envelope",
            "env",
            r#"    <data id="n" sce:type="uint8" sce:direction="in"/>"#,
            r#"      <assign location="env.m" expr="n"/>"#,
            "'done'",
        ),
        &resource_dir(),
    )
    .expect_err("an assign to a number the encode derives must be refused");
    let said = refused.to_string();
    assert!(
        said.contains("env.m") && said.contains("derives `m`"),
        "the refusal does not say the encode derives `m`: {said}"
    );
}

#[test]
fn a_group_is_handed_whole_and_its_members_are_no_members_of_the_value() {
    // `state` and `attempts` are one group, which `flags.has_session` decides.
    let cpp = generate_in(
        "group_rebuild",
        &procedure(
            "codec_group_enum",
            "g",
            r#"    <data id="f" sce:type="uint8" sce:direction="in"/>"#,
            r#"      <assign location="g.flags" expr="f"/>"#,
            "'done'",
        ),
        &resource_dir(),
    )
    .expect("an assign to an element beside a group generates");
    let start = cpp
        .find("g_ = ::SCE::Generated::CodecGroupEnum::CodecGroupEnum(f_, g_.")
        .unwrap_or_else(|| panic!("the group is not handed whole after the flags:\n{cpp}"));
    let call = &cpp[start..start + cpp[start..].find(';').expect("the statement ends")];
    assert!(
        !call.contains("g_.state()") && !call.contains("g_.attempts()"),
        "the rebuild reads a member of the group by an accessor the value does not have: {call}"
    );
    for member in ["state", "attempts"] {
        let refused = generate_in(
            "group_member_read",
            &procedure(
                "codec_group_enum",
                "g",
                r#"    <data id="f" sce:type="uint8" sce:direction="in"/>"#,
                "",
                &format!("g.{member}"),
            ),
            &resource_dir(),
        )
        .expect_err("a read of a member of a group the value presents whole must be refused");
        let said = refused.to_string();
        assert!(
            said.contains(&format!("g.{member} is not a member of g")),
            "the read of the group member `{member}` is not refused as no member: {said}"
        );
    }
}
