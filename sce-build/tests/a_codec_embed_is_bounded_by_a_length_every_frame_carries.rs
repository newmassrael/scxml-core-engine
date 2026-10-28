// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An embedded codec is bounded by a sibling field every frame carries
//! (`sce:length-from`, docs/adr/0055, section 8).
//!
//! A field gated by `sce:present-if` is a value that may be absent, held in the
//! backend's optional type (`Option<T>`, `std::optional<T>`, `T?`,
//! `optional.Of[T]`, `Optional[T]`, a flag beside the value in C11). The decode of
//! an embed read its length sibling as the number it is, so a gated sibling was a
//! type error in the generated code of every backend, and an absent length bounds
//! nothing. The codec document is refused where it is read, naming the sibling.
//!
//! # What was wrong
//!
//! Measured in the code on 2026-10-28: `validate_codec_embed_length_from` held the
//! sibling to be declared earlier and to be an integer, and nothing held it to be
//! present, so a gated one reached the generators.

use std::path::{Path, PathBuf};

use sce_build::forge::codegen_matrix::emitting;
use sce_build::forge::model::ForgeKind;
use sce_build::generator::Language;
use sce_build::{compile_forge_with_imports, DocumentLabel, ForgeCompileOptions};
use tempfile::tempdir;

fn resource_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources")
}

/// A codec whose embed is bounded by `ext_size`, which is declared as `field`.
fn codec(field: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="bounded_ext">
  <sce:import src="codec_zenoh_timestamp.scxml" kind="codec" as="codec_zenoh_timestamp"/>
  <datamodel>
    <sce:flags id="header" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="L" bit="7"/>
    </sce:flags>
    {field}
    <sce:embed id="ts" type="codec_zenoh_timestamp" sce:byte="2"
               sce:length-from="ext_size"/>
  </datamodel>
</scxml>
"#
    )
}

/// What compiling `text` as a codec of the language `lang` answers: the refusal's
/// text, or `None` when it compiles. The document is written beside the
/// timestamp codec it embeds.
fn refusal(text: &str, lang: Language) -> Option<String> {
    let dir = tempdir().expect("tempdir");
    std::fs::copy(
        resource_dir().join("codec_zenoh_timestamp.scxml"),
        dir.path().join("codec_zenoh_timestamp.scxml"),
    )
    .expect("copy the embedded codec");
    let options = ForgeCompileOptions {
        go_module_prefix: Some("example.com/sce-forge".to_string()),
        ..ForgeCompileOptions::default()
    };
    compile_forge_with_imports(
        text,
        DocumentLabel::symmetric("bounded_ext"),
        lang,
        dir.path(),
        &options,
    )
    .err()
    .map(|e| e.error.to_string())
}

/// The backends that emit a codec, from the registry: a backend that gains an emitter
/// joins the run by itself.
fn every_codec_backend() -> Vec<Language> {
    emitting(ForgeKind::Codec).collect()
}

/// A sibling that every frame carries bounds the embed, on every backend.
#[test]
fn an_embed_is_bounded_by_a_sibling_every_frame_carries() {
    let document =
        codec(r#"<sce:field id="ext_size" sce:type="uint64" sce:byte="1" sce:bit-size="vle"/>"#);
    for lang in every_codec_backend() {
        assert_eq!(
            refusal(&document, lang),
            None,
            "{lang:?} compiles the embed bounded by an ungated sibling"
        );
    }
}

/// A sibling gated by `sce:present-if` is refused, naming it and the attribute that
/// gates it, whatever backend is asked for.
#[test]
fn an_embed_is_not_bounded_by_a_gated_sibling() {
    let document = codec(
        r#"<sce:field id="ext_size" sce:type="uint64" sce:byte="1" sce:bit-size="vle"
               sce:present-if="header.L"/>"#,
    );
    for lang in every_codec_backend() {
        let refused = refusal(&document, lang)
            .unwrap_or_else(|| panic!("{lang:?} compiles an embed bounded by a gated sibling"));
        for fragment in [
            "invalid sce:length-from value 'ext_size'",
            "length source must be a field every frame carries",
            "'ext_size' is gated by sce:present-if",
        ] {
            assert!(
                refused.contains(fragment),
                "{lang:?} refuses it saying `{fragment}`:\n{refused}"
            );
        }
    }
}
