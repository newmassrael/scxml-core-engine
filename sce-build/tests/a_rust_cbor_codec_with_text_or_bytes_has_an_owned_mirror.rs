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
//!   entry is held in the profile's `S::Str<N>` or `S::Bytes<N>` at the entry's
//!   bound, every other entry as the view holds it, and an entry that may be absent
//!   stays an `Option` of it.
//! * `try_into_owned_in::<S>()` copies each of them with `from_view` or
//!   `from_slice` and a `?`, so the inline profile refuses a value past its bound;
//!   `try_into_owned()` is the default profile's.
//! * The owned form is re-encoded exactly as the view is, through the codec's
//!   source interface: it implements `{Name}Source`, `as_borrowed` reads through it,
//!   and `encode` is `encode_source`.
//! * A map with one string or byte string that states no bound has no mirror, and
//!   neither has a map of scalars, which borrows nothing: one predicate
//!   (`cbor_codec::rust_owned_mirror`) says it for the codec, the answer a
//!   container is given for the element, and the fact a log is judged by.

use sce_build::forge::cbor_codec::{has_unbounded_text_or_bytes, rust_owned_mirror};
use sce_build::forge::model::{CodecModel, ForgeDocument};
use sce_build::forge::parser::parse_forge;
use sce_build::generator::Language;
use sce_build::{compile_forge_from_string, DocumentLabel};

/// A CBOR codec in codec format 2 whose `<datamodel>` holds `entries` verbatim.
fn document(root_attrs: &str, entries: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:encoding="cbor" sce:codec-format="2"{root_attrs} name="probe_cbor" version="1.0">
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

const MAY_FAIL: &str = r#" sce:encode-may-fail="true""#;

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

/// The text of a generated module must hold every line of `needles`.
fn assert_holds(code: &str, needles: &[&str]) {
    for needle in needles {
        assert!(code.contains(needle), "missing `{needle}` in:\n{code}");
    }
}

#[test]
fn one_predicate_says_which_map_has_a_mirror() {
    // The map the mirror is generated for, and the three that have none: one with a
    // string that states no bound, one whose only entry is a byte string with no
    // bound, and one that borrows nothing.
    for (root, entries, mirror, unbounded) in [
        (MAY_FAIL, BOUNDED, true, false),
        (MAY_FAIL, ONE_UNBOUNDED, false, true),
        ("", ONLY_UNBOUNDED, false, true),
        ("", SCALARS, false, false),
    ] {
        let m = codec(&document(root, entries));
        assert_eq!(rust_owned_mirror(&m), mirror, "{entries}");
        assert_eq!(has_unbounded_text_or_bytes(&m), unbounded, "{entries}");
    }
}

#[test]
fn the_owned_form_holds_each_entry_in_a_container_of_its_bound() {
    let code = generated(&document(MAY_FAIL, BOUNDED));
    assert_holds(
        &code,
        &[
            // The view still borrows, and the mirror is a type of its own with the
            // positional codec's derives and its profile parameter and default.
            "pub struct ProbeCbor<'a> {",
            "#[allow(dead_code)]\n#[derive(Debug, Clone, PartialEq)]\npub struct ProbeCborOwned<S: ::sce_forge_runtime::codec::CodecStorage = ::sce_forge_runtime::codec::DefaultStorage> {",
            "    /// Map key 1, required.\n    pub note: S::Str<8>,",
            "    /// Map key 2, required.\n    pub blob: S::Bytes<12>,",
            "    /// Map key 3, required.\n    pub tag: S::Bytes<4>,",
            "    /// Map key 4.\n    pub label: Option<S::Str<5>>,",
            "    /// Map key 5.\n    pub extra: Option<S::Bytes<6>>,",
            "    pub level: u8,",
            "    pub urgent: Option<bool>,",
        ],
    );
}

#[test]
fn the_projection_copies_each_entry_within_its_bound_and_the_others_as_they_are() {
    let code = generated(&document(MAY_FAIL, BOUNDED));
    assert_holds(
        &code,
        &[
            "impl<'a> ProbeCbor<'a> {",
            "pub fn try_into_owned_in<S: ::sce_forge_runtime::codec::CodecStorage>(self) -> Result<ProbeCborOwned<S>, CodecError> {",
            "        Ok(ProbeCborOwned {",
            "            note: <S::Str<8> as ::sce_forge_runtime::codec::SceStr>::from_view(self.note)?,",
            "            blob: <S::Bytes<12> as ::sce_forge_runtime::codec::SceByteBuf>::from_slice(self.blob)?,",
            "            tag: <S::Bytes<4> as ::sce_forge_runtime::codec::SceByteBuf>::from_slice(self.tag)?,",
            "            label: self.label.map(<S::Str<5> as ::sce_forge_runtime::codec::SceStr>::from_view).transpose()?,",
            "            extra: self.extra.map(<S::Bytes<6> as ::sce_forge_runtime::codec::SceByteBuf>::from_slice).transpose()?,",
            "            level: self.level,",
            "            urgent: self.urgent,",
            // The default profile's form, and the move between profiles.
            "pub fn try_into_owned(self) -> Result<ProbeCborOwned, CodecError> {\n        self.try_into_owned_in()\n    }",
            "pub fn transcode_in<D: ::sce_forge_runtime::codec::CodecStorage>(&self) -> Result<ProbeCborOwned<D>, CodecError> {\n        self.as_borrowed().try_into_owned_in::<D>()\n    }",
        ],
    );
}

#[test]
fn the_owned_form_is_written_through_the_source_interface_as_the_view_is() {
    let code = generated(&document(MAY_FAIL, BOUNDED));
    assert_holds(
        &code,
        &[
            // The interface says each entry of the owned form as the view's does.
            "impl<S: ::sce_forge_runtime::codec::CodecStorage> ProbeCborSource for ProbeCborOwned<S> {",
            "    fn note(&self) -> &str {\n        <S::Str<8> as ::sce_forge_runtime::codec::SceStr>::as_str(&self.note)\n    }",
            "    fn blob(&self) -> &[u8] {\n        <S::Bytes<12> as ::sce_forge_runtime::codec::SceByteBuf>::as_slice(&self.blob)\n    }",
            "    fn tag(&self) -> &[u8] {\n        <S::Bytes<4> as ::sce_forge_runtime::codec::SceByteBuf>::as_slice(&self.tag)\n    }",
            "    fn label(&self) -> Option<&str> {\n        self.label.as_ref().map(<S::Str<5> as ::sce_forge_runtime::codec::SceStr>::as_str)\n    }",
            "    fn extra(&self) -> Option<&[u8]> {\n        self.extra.as_ref().map(<S::Bytes<6> as ::sce_forge_runtime::codec::SceByteBuf>::as_slice)\n    }",
            "    fn level(&self) -> u8 {\n        self.level\n    }",
            // The re-borrow reads through it, and the encode is the one encode.
            "pub fn as_borrowed(&self) -> ProbeCbor<'_> {\n        ProbeCbor {\n            note: <Self as ProbeCborSource>::note(self),",
            "            label: <Self as ProbeCborSource>::label(self),",
            "pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), CodecError> where CodecError: From<K::Error> {\n        ProbeCbor::encode_source(self, w)\n    }",
            "/// Encode `self` into the caller-owned sink, straight from the owned fields:",
        ],
    );
}

#[test]
fn a_map_with_no_bound_on_one_entry_or_no_borrow_has_no_mirror() {
    for (root, entries) in [
        (MAY_FAIL, ONE_UNBOUNDED),
        ("", ONLY_UNBOUNDED),
        ("", SCALARS),
    ] {
        let code = generated(&document(root, entries));
        assert!(!code.contains("ProbeCborOwned"), "{entries}\n{code}");
        assert!(!code.contains("try_into_owned"), "{entries}\n{code}");
        assert!(!code.contains("transcode_in"), "{entries}\n{code}");
        // What every map has is still there: the view and the source interface.
        assert_holds(&code, &["pub trait ProbeCborSource {"]);
    }
    // The map that borrows and has no mirror keeps its view as it was.
    let code = generated(&document("", ONLY_UNBOUNDED));
    assert_holds(&code, &["pub struct ProbeCbor<'a> {"]);
}
