// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A Rust CBOR codec with a string or a byte-string entry has an owned mirror
//! (SCE_FORGE.md §4.6.1).
//!
//! # What was missing
//!
//! A CBOR map decodes a string to a `&str` and a byte string to a `&[u8]`, views of
//! the buffer it was read from, so a Rust map with one borrows (`Name<'a>`). A
//! positional codec that borrows has `{Name}Owned<S>`, a lifetime-free copy held in
//! a storage profile's containers, which is what a container that keeps a decoded
//! value past its buffer stores (a queue, a bounded collection, a broadcast log).
//! A CBOR map had no such form, so a log refused it and a queue or a bounded
//! collection named a type that carries a lifetime.
//!
//! # What is held
//!
//! * A map whose every string and byte string states a bound (`sce:max-size`, or
//!   the `sce:length` of an exact byte string) has `{Name}Owned<S>`: each such
//!   entry is held in what the profile answers for its tag, which on `Inline` is
//!   the entry's bound, inside a `Bounded` of its bounds (docs/adr/0077 §3, §7),
//!   every other entry as the view holds it, and an entry that may be absent stays
//!   an `Option` of it.
//! * `try_into_owned_in::<S>()` stores each of them as `S` stores its tag, with
//!   `Bounded::store` and a `?`, which carries the bounds over and refuses only for
//!   a profile whose capacity is below them; `try_into_owned()` is the default
//!   profile's.
//! * The owned form is re-encoded exactly as the view is, through the codec's
//!   source interface: it implements `{Name}Source`, `as_borrowed` reads through it,
//!   and `encode` is `encode_source`.
//! * A map with a string or byte string that states no bound has a mirror as
//!   every other map does, the entry held at `{ usize::MAX }` in what the profile
//!   answers for its tag, which `Inline` does not answer for (docs/adr/0077 §7);
//!   a map of scalars borrows nothing and has none: one predicate
//!   (`cbor_codec::rust_owned_mirror`) says it for the codec and the answer a
//!   container is given for the element, and another
//!   (`cbor_codec::has_unbounded_text_or_bytes`) the fact a log is judged by.
//! * With `generate --owned-origin` the mirror gains `try_into_owned_in_origin`, as
//!   every other generated owned form does: a string is built by what the profile
//!   answers for the entry's tag, `OriginStr<Tag>::str_from`, and a byte string by
//!   `OriginBytes<Tag>::bytes_from`, each given the origin the view was decoded
//!   from, an entry that may be absent only when present, and every other entry as
//!   the copying projection moves it. What it asks of the profile is the map's
//!   `{Name}Origin`, declared beside it, which names every tag. The text is
//!   appended after the module the default output is, and a map with no mirror is
//!   the same with the option on.

use std::path::Path;

use sce_build::forge::cbor_codec::{has_unbounded_text_or_bytes, rust_owned_mirror};
use sce_build::forge::model::{CodecModel, ForgeDocument};
use sce_build::forge::parser::parse_forge;
use sce_build::generator::Language;
use sce_build::{
    compile_forge_from_string, compile_forge_with_imports, DocumentLabel, ForgeCompileOptions,
};

/// A CBOR codec in codec format 2 whose `<datamodel>` holds `entries` verbatim.
///
/// No probe declares `sce:encode-may-fail`: a string or byte string is presented
/// within its bounds (docs/adr/0077 §3), so none of these maps refuses a value.
fn document(entries: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:encoding="cbor" sce:codec-format="2" name="probe_cbor" version="1.0">
  <datamodel>
    <data id="raw" sce:type="bytes" sce:direction="in"/>
{entries}
  </datamodel>
</scxml>
"#
    )
}

/// Every entry kind a mirror holds: a bounded text, a bounded byte string and an
/// exact one, an optional text and an optional byte string, and the entries it
/// holds as the view does (a number, an optional truth value).
const BOUNDED: &str = r#"    <data id="note" sce:type="string" sce:key="1" sce:max-size="8" sce:direction="out"/>
    <data id="blob" sce:type="bytes" sce:key="2" sce:max-size="12" sce:direction="out"/>
    <data id="tag" sce:type="bytes" sce:key="3" sce:length="4" sce:direction="out"/>
    <data id="label" sce:type="optional&lt;string&gt;" sce:key="4" sce:max-size="5" sce:direction="out"/>
    <data id="extra" sce:type="optional&lt;bytes&gt;" sce:key="5" sce:max-size="6" sce:direction="out"/>
    <data id="level" sce:type="uint8" sce:key="6" sce:direction="out"/>
    <data id="urgent" sce:type="optional&lt;bool&gt;" sce:key="7" sce:direction="out"/>"#;

/// A text with no bound, among entries that all state theirs.
const ONE_UNBOUNDED: &str = r#"    <data id="note" sce:type="string" sce:key="1" sce:max-size="8" sce:direction="out"/>
    <data id="free" sce:type="string" sce:key="2" sce:direction="out"/>"#;

/// A byte string with no bound and nothing else.
const ONLY_UNBOUNDED: &str =
    r#"    <data id="free" sce:type="bytes" sce:key="1" sce:direction="out"/>"#;

/// Entries that borrow nothing.
const SCALARS: &str = r#"    <data id="level" sce:type="uint8" sce:key="1" sce:direction="out"/>
    <data id="urgent" sce:type="optional&lt;bool&gt;" sce:key="2" sce:direction="out"/>"#;

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

/// The generated Rust of the codec `text`.
fn generated(text: &str) -> String {
    let output = compile_forge_from_string(text, label(), Language::Rust)
        .unwrap_or_else(|e| panic!("rust generates the codec: {e:?}"));
    output
        .files
        .into_iter()
        .find(|(name, _)| name == "probe_cbor.rs")
        .map(|(_, code)| code)
        .expect("the codec's module")
}

/// The generated Rust of the codec `text`, with `--owned-origin` on or off.
fn generated_with_origin(text: &str, owned_origin: bool) -> String {
    let options = ForgeCompileOptions {
        owned_origin,
        ..ForgeCompileOptions::default()
    };
    let output =
        compile_forge_with_imports(text, label(), Language::Rust, Path::new("."), &options)
            .unwrap_or_else(|e| panic!("rust generates the codec: {e:?}"));
    output
        .files
        .into_iter()
        .find(|(name, _)| name == "probe_cbor.rs")
        .map(|(_, code)| code)
        .expect("the codec's module")
}

/// The text of a generated module must hold every line of `needles`.
fn assert_holds(code: &str, needles: &[&str]) {
    for needle in needles {
        assert!(code.contains(needle), "missing `{needle}` in:\n{code}");
    }
}

#[test]
fn one_predicate_says_which_map_has_a_mirror() {
    // Every map that borrows has a mirror, a string that states no bound among
    // them, and the map that borrows nothing has none.
    for (entries, mirror, unbounded) in [
        (BOUNDED, true, false),
        (ONE_UNBOUNDED, true, true),
        (ONLY_UNBOUNDED, true, true),
        (SCALARS, false, false),
    ] {
        let m = codec(&document(entries));
        assert_eq!(rust_owned_mirror(&m), mirror, "{entries}");
        assert_eq!(has_unbounded_text_or_bytes(&m), unbounded, "{entries}");
    }
}

#[test]
fn the_owned_form_holds_each_entry_in_a_container_of_its_bound() {
    let code = generated(&document(BOUNDED));
    assert_holds(
        &code,
        &[
            // The view still borrows, and the mirror is a type of its own with the
            // positional codec's derives and its profile parameter and default.
            "pub struct ProbeCbor<'a> {",
            "#[allow(dead_code)]\n#[derive(Debug, Clone, PartialEq)]\npub struct ProbeCborOwned<S: ProbeCborStorage = ::sce_forge_runtime::codec::DefaultStorage> {",
            "    /// Map key 1, required.\n    pub note: ::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsStr<ProbeCborNoteField>>::Str, 8>,",
            "    /// Map key 2, required.\n    pub blob: ::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsBytes<ProbeCborBlobField>>::Bytes, 12>,",
            "    /// Map key 3, required.\n    pub tag: ::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsBytes<ProbeCborTagField>>::Bytes, 4, 4>,",
            "    /// Map key 4.\n    pub label: Option<::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsStr<ProbeCborLabelField>>::Str, 5>>,",
            "    /// Map key 5.\n    pub extra: Option<::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsBytes<ProbeCborExtraField>>::Bytes, 6>>,",
            // What the inline profile holds each at: its bound, or an exact length.
            "impl ::sce_forge_runtime::codec::HoldsStr<ProbeCborNoteField> for ::sce_forge_runtime::codec::Inline {\n    type Str = ::sce_forge_runtime::codec::InlineStr<8>;\n}",
            "impl ::sce_forge_runtime::codec::HoldsBytes<ProbeCborBlobField> for ::sce_forge_runtime::codec::Inline {\n    type Bytes = ::sce_forge_runtime::codec::InlineBytes<12>;\n}",
            "impl ::sce_forge_runtime::codec::HoldsBytes<ProbeCborTagField> for ::sce_forge_runtime::codec::Inline {\n    type Bytes = ::sce_forge_runtime::codec::InlineBytes<4>;\n}",
            "    pub level: u8,",
            "    pub urgent: Option<bool>,",
        ],
    );
}

#[test]
fn the_view_holds_each_entry_within_its_bound_and_made_through_the_one_door() {
    let code = generated(&document(BOUNDED));
    assert_holds(
        &code,
        &[
            "    /// Map key 1, required.\n    pub note: ::sce_forge_runtime::codec::Text<'a, 8>,",
            "    /// Map key 2, required.\n    pub blob: ::sce_forge_runtime::codec::Bytes<'a, 12>,",
            "    /// Map key 3, required.\n    pub tag: ::sce_forge_runtime::codec::Bytes<'a, 4, 4>,",
            "    /// Map key 4.\n    pub label: Option<::sce_forge_runtime::codec::Text<'a, 5>>,",
            "    /// Map key 5.\n    pub extra: Option<::sce_forge_runtime::codec::Bytes<'a, 6>>,",
            // The door's refusal is the error the read gives for the same value.
            "note = Some(::sce_forge_runtime::codec::Bounded::new(cbor::read_text(&mut c, Some(8))?).ok_or(CodecError::CborOutOfRange)?);",
            "blob = Some(::sce_forge_runtime::codec::Bounded::new(cbor::read_bytes(&mut c, Some(12))?).ok_or(CodecError::CborOutOfRange)?);",
            "tag = Some(::sce_forge_runtime::codec::Bounded::new(cbor::read_bytes_exact(&mut c, 4)?).ok_or(CodecError::CborWrongLength)?);",
        ],
    );
}

#[test]
fn an_entry_with_no_bound_is_held_at_no_bound_in_one_shape() {
    let code = generated(&document(ONE_UNBOUNDED));
    assert_holds(
        &code,
        &[
            "pub note: ::sce_forge_runtime::codec::Text<'a, 8>,",
            "pub free: ::sce_forge_runtime::codec::Text<'a, { usize::MAX }>,",
            "free = Some(::sce_forge_runtime::codec::Bounded::new(cbor::read_text(&mut c, None)?).ok_or(CodecError::CborOutOfRange)?);",
            // The mirror holds it in what the profile answers for its tag, and
            // `Inline` answers only for the entry that states its bound.
            "    pub free: ::sce_forge_runtime::codec::Bounded<<S as ::sce_forge_runtime::codec::HoldsStr<ProbeCborFreeField>>::Str, { usize::MAX }>,",
            "impl ::sce_forge_runtime::codec::HoldsStr<ProbeCborNoteField> for ::sce_forge_runtime::codec::Inline {",
            "// `Inline` does not answer for `free`",
        ],
    );
    assert!(
        !code.contains("HoldsStr<ProbeCborFreeField> for ::sce_forge_runtime::codec::Inline"),
        "{code}"
    );
}

#[test]
fn the_projection_copies_each_entry_within_its_bound_and_the_others_as_they_are() {
    let code = generated(&document(BOUNDED));
    assert_holds(
        &code,
        &[
            "impl<'a> ProbeCbor<'a> {",
            "pub fn try_into_owned_in<S: ProbeCborFrom<'a>>(self) -> Result<ProbeCborOwned<S>, CodecError> {",
            "        Ok(ProbeCborOwned {",
            "            note: self.note.store::<S, ProbeCborNoteField>()?,",
            "            blob: self.blob.store::<S, ProbeCborBlobField>()?,",
            "            tag: self.tag.store::<S, ProbeCborTagField>()?,",
            "            label: self.label.map(|held| held.store::<S, ProbeCborLabelField>()).transpose()?,",
            "            extra: self.extra.map(|held| held.store::<S, ProbeCborExtraField>()).transpose()?,",
            "            level: self.level,",
            "            urgent: self.urgent,",
            // The default profile's form, and the move between profiles.
            "pub fn try_into_owned(self) -> Result<ProbeCborOwned, CodecError> {\n        self.try_into_owned_in()\n    }",
            "pub fn transcode_in<D: for<'x> ProbeCborFrom<'x>>(&self) -> Result<ProbeCborOwned<D>, CodecError> {\n        self.as_borrowed().try_into_owned_in::<D>()\n    }",
        ],
    );
}

#[test]
fn the_owned_form_is_written_through_the_source_interface_as_the_view_is() {
    let code = generated(&document(BOUNDED));
    assert_holds(
        &code,
        &[
            // The interface says each entry of the owned form as the view's does.
            "impl<S: ProbeCborStorage> ProbeCborSource for ProbeCborOwned<S> {",
            "    fn note(&self) -> ::sce_forge_runtime::codec::Text<'_, 8> {\n        ::sce_forge_runtime::codec::Bounded::as_text(&self.note)\n    }",
            "    fn blob(&self) -> ::sce_forge_runtime::codec::Bytes<'_, 12> {\n        ::sce_forge_runtime::codec::Bounded::as_bytes(&self.blob)\n    }",
            "    fn tag(&self) -> ::sce_forge_runtime::codec::Bytes<'_, 4, 4> {\n        ::sce_forge_runtime::codec::Bounded::as_bytes(&self.tag)\n    }",
            "    fn label(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 5>> {\n        self.label.as_ref().map(::sce_forge_runtime::codec::Bounded::as_text)\n    }",
            "    fn extra(&self) -> Option<::sce_forge_runtime::codec::Bytes<'_, 6>> {\n        self.extra.as_ref().map(::sce_forge_runtime::codec::Bounded::as_bytes)\n    }",
            "    fn level(&self) -> u8 {\n        self.level\n    }",
            // The re-borrow reads through it, and the encode is the one encode.
            "pub fn as_borrowed(&self) -> ProbeCbor<'_> {\n        ProbeCbor {\n            note: <Self as ProbeCborSource>::note(self),",
            "            label: <Self as ProbeCborSource>::label(self),",
            "pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), K::Error> {\n        ProbeCbor::encode_source(self, w)\n    }",
            "pub fn encode_to_vec(&self) -> Vec<u8> {",
            "/// Encode `self` into the caller-owned sink, straight from the owned fields:",
        ],
    );
}

#[test]
fn a_map_that_borrows_nothing_has_no_mirror() {
    let code = generated(&document(SCALARS));
    assert!(!code.contains("ProbeCborOwned"), "{code}");
    assert!(!code.contains("try_into_owned"), "{code}");
    assert!(!code.contains("transcode_in"), "{code}");
    // What every map has is still there: the source interface.
    assert_holds(&code, &["pub trait ProbeCborSource {"]);
}

#[test]
fn a_map_with_an_entry_of_no_bound_has_a_mirror_at_the_profiles_that_hold_it() {
    for entries in [ONE_UNBOUNDED, ONLY_UNBOUNDED] {
        let code = generated(&document(entries));
        assert_holds(
            &code,
            &[
                "pub struct ProbeCbor<'a> {",
                "pub fn try_into_owned_in<S: ProbeCborFrom<'a>>(self) -> Result<ProbeCborOwned<S>, CodecError> {",
                // The default profile of the heap-free tier is `Inline`, which
                // does not hold the map, so no item names the map there.
                "// No projection at the build's default profile",
            ],
        );
        assert!(
            !code.contains("pub fn try_into_owned(self)"),
            "{entries}\n{code}"
        );
    }
}

#[test]
fn the_origin_projection_has_the_signature_every_other_owned_form_has() {
    let code = generated_with_origin(&document(BOUNDED), true);
    assert_holds(
        &code,
        &[
            // The same method, over the same origin parameter, for a profile that answers
            // the map's `ProbeCborOrigin`; the copying projection is still there beside it.
            "pub fn try_into_owned_in_origin<S: ProbeCborOrigin>(self, origin: &<S as ::sce_forge_runtime::codec::OriginStorage>::Origin) -> Result<ProbeCborOwned<S>, CodecError> {\n        Ok(ProbeCborOwned {",
            "pub fn try_into_owned_in<S: ProbeCborFrom<'a>>(self) -> Result<ProbeCborOwned<S>, CodecError> {",
        ],
    );
}

#[test]
fn the_origin_projection_asks_for_a_container_built_from_the_origin_for_each_tag() {
    // A profile chooses the container of each string and byte string by its tag, so
    // what the projection asks names every tag: a tag the profile gave no container
    // built from the origin is a compile error, and the profile can share one entry
    // and copy the one beside it.
    let code = generated_with_origin(&document(BOUNDED), true);
    let head = "pub trait ProbeCborOrigin: ProbeCborStorage + ::sce_forge_runtime::codec::OriginStorage + ";
    let declared = code
        .lines()
        .find(|l| l.starts_with(head))
        .unwrap_or_else(|| panic!("no `{head}` in:\n{code}"));
    for bound in [
        "::sce_forge_runtime::codec::OriginStr<ProbeCborNoteField>",
        "::sce_forge_runtime::codec::OriginBytes<ProbeCborBlobField>",
        "::sce_forge_runtime::codec::OriginBytes<ProbeCborTagField>",
        "::sce_forge_runtime::codec::OriginStr<ProbeCborLabelField>",
        "::sce_forge_runtime::codec::OriginBytes<ProbeCborExtraField>",
    ] {
        assert!(
            declared.contains(bound),
            "`{bound}` missing from:\n{declared}"
        );
    }
    // Every profile that answers the bounds answers the trait.
    let bounds = declared
        .trim_start_matches("pub trait ProbeCborOrigin: ")
        .trim_end_matches(" {}");
    assert_holds(
        &code,
        &[&format!("impl<S: {bounds}> ProbeCborOrigin for S {{}}")],
    );
}

#[test]
fn the_origin_projection_builds_text_and_bytes_through_the_profile_at_each_capacity() {
    let code = generated_with_origin(&document(BOUNDED), true);
    assert_holds(
        &code,
        &[
            // A string by `str_from` and a byte string by `bytes_from` of what the
            // profile answers for the entry's tag, at the capacity of the entry, which
            // for an exact byte string is its length, and given the origin; the other
            // entries move as they do in the copying projection.
            "            note: ::sce_forge_runtime::codec::Bounded::from_built_text(<S as ::sce_forge_runtime::codec::OriginStr<ProbeCborNoteField>>::str_from(origin, self.note.into_inner()))?,",
            "            blob: ::sce_forge_runtime::codec::Bounded::from_built(<S as ::sce_forge_runtime::codec::OriginBytes<ProbeCborBlobField>>::bytes_from(origin, self.blob.into_inner()))?,",
            "            tag: ::sce_forge_runtime::codec::Bounded::from_built(<S as ::sce_forge_runtime::codec::OriginBytes<ProbeCborTagField>>::bytes_from(origin, self.tag.into_inner()))?,",
            "            level: self.level,\n            urgent: self.urgent,\n        })",
        ],
    );
}

#[test]
fn the_origin_projection_builds_an_optional_entry_only_when_it_is_present() {
    let code = generated_with_origin(&document(BOUNDED), true);
    assert_holds(
        &code,
        &[
            "            label: self.label.map(|_v| ::sce_forge_runtime::codec::Bounded::from_built_text(<S as ::sce_forge_runtime::codec::OriginStr<ProbeCborLabelField>>::str_from(origin, _v.into_inner()))).transpose()?,",
            "            extra: self.extra.map(|_v| ::sce_forge_runtime::codec::Bounded::from_built(<S as ::sce_forge_runtime::codec::OriginBytes<ProbeCborExtraField>>::bytes_from(origin, _v.into_inner()))).transpose()?,",
        ],
    );
}

#[test]
fn the_default_output_has_no_origin_projection() {
    // `--owned-origin` is the only thing that writes it.
    let code = generated_with_origin(&document(BOUNDED), false);
    for absent in [
        "try_into_owned_in_origin",
        "ProbeCborOrigin",
        "OriginStorage",
        "OriginBytes",
        "OriginStr<",
        "str_from",
        "bytes_from",
    ] {
        // None of these is a name the default module has: it stores through
        // `Bounded::store`.
        assert!(
            !code.contains(absent),
            "`{absent}` in the default output:\n{code}"
        );
    }
    // The copying projection is the one the default output has.
    assert_holds(&code, &["pub fn try_into_owned_in<S: ProbeCborFrom<'a>>"]);
}

#[test]
fn the_option_only_appends_the_projection_to_a_map_with_a_mirror() {
    for entries in [BOUNDED, ONE_UNBOUNDED, ONLY_UNBOUNDED, SCALARS] {
        let text = document(entries);
        let off = generated_with_origin(&text, false);
        let on = generated_with_origin(&text, true);
        if rust_owned_mirror(&codec(&text)) {
            // The module without the option is the beginning of the module with it, and
            // what follows is the projection, written after the source interface.
            let head = off.trim_end();
            assert!(
                on.starts_with(head),
                "the option changed the default output:\n{on}"
            );
            let tail = on[head.len()..].trim_start();
            assert!(
                tail.starts_with("// ── Owned projection with an origin (`--owned-origin`)"),
                "the text the option adds is the projection:\n{tail}"
            );
            assert!(tail.contains("try_into_owned_in_origin"), "{tail}");
            let source = on
                .find("pub trait ProbeCborSource {")
                .expect("the source interface");
            let projection = on.find("try_into_owned_in_origin").expect("the projection");
            assert!(
                source < projection,
                "the projection comes after the source interface"
            );
        } else {
            // A map with no mirror has no owned form to project into.
            assert_eq!(on, off, "{entries}");
        }
    }
}
