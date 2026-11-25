// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A codec that is one variant dispatching on a peeked byte, with no field of
//! its own, generates a reader of the frames of it laid one after the other,
//! read one record at a time (docs/adr/0130): `{Codec}Records`, whose `next`
//! hands out a `{Codec}Record` of the arm the tag names, a handle whose size the
//! module asserts. What the reader does with a batch is held by
//! `forge_source_round_trip`'s `records` module.

use std::path::PathBuf;

use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

fn resource_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build is at the repository root")
        .join("tests/forge/resources")
}

/// The Rust `document` generates as `name`, every file joined.
fn rust_of_document(document: &str, name: &str, views: bool) -> String {
    sce_build::compile_forge_with_imports(
        document,
        DocumentLabel::symmetric(name),
        Language::Rust,
        &resource_dir(),
        &ForgeCompileOptions {
            views,
            ..ForgeCompileOptions::default()
        },
    )
    .unwrap_or_else(|e| panic!("{name} generates for Rust: {e}"))
    .files
    .into_iter()
    .map(|(_, code)| code)
    .collect::<Vec<_>>()
    .join("\n")
}

/// The Rust the resource codec `name` generates.
fn rust_of(name: &str, views: bool) -> String {
    let path = resource_dir().join(format!("{name}.scxml"));
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} is readable: {e}", path.display()));
    rust_of_document(&source, name, views)
}

/// A peeked tag over two arms, the second under `feature` where it is given,
/// and the first under `first_feature` where that is. Two arms take every
/// value of a one-bit tag; arms that may both be left out are joined by a
/// catch-all, which takes a two-bit tag's other values.
fn peek_variant(first_feature: Option<&str>, feature: Option<&str>) -> String {
    let gate = |f: Option<&str>| f.map_or(String::new(), |f| format!(" feature=\"{f}\""));
    let first_default = if first_feature.is_some() {
        String::new()
    } else {
        " default=\"true\"".to_string()
    };
    let (width, catch_all) = if first_feature.is_some() {
        (2, "\n      <sce:default type=\"codec_peek_arm_a\"/>")
    } else {
        (1, "")
    };
    let first_gate = gate(first_feature);
    let second_gate = gate(feature);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="gated_records">
  <sce:import src="codec_peek_arm_a.scxml" kind="codec" as="codec_peek_arm_a"/>
  <sce:import src="codec_peek_arm_b.scxml" kind="codec" as="codec_peek_arm_b"/>
  <datamodel>
    <sce:variant tag="peek.kind">
      <sce:peek-byte id="peek" sce:type="uint8">
        <sce:flag name="kind" bit="0" width="{width}"/>
      </sce:peek-byte>
      <sce:arm value="0x00" type="codec_peek_arm_a"{first_default}{first_gate}/>
      <sce:arm value="0x01" type="codec_peek_arm_b"{second_gate}/>{catch_all}
    </sce:variant>
  </datamodel>
</scxml>"#
    )
}

/// The line before the one that reads `item`, trimmed.
fn line_before<'c>(lines: &[&'c str], item: &str) -> &'c str {
    let at = lines
        .iter()
        .position(|l| l.trim() == item)
        .unwrap_or_else(|| panic!("no line `{item}`"));
    lines[at - 1].trim()
}

/// The network envelope's view form generates the reader of its records, a
/// record enum of one handle per arm, and the assertion of the record's size.
#[test]
fn the_network_envelope_generates_its_records_and_bounds_their_size() {
    let code = rust_of("codec_zenoh_network_envelope", true);
    for item in [
        "pub struct CodecZenohNetworkEnvelopeRecords<'f>(::sce_forge_runtime::codec::Records<'f>);",
        "pub enum CodecZenohNetworkEnvelopeRecord<'r, 'f> {",
        "const _: () = assert!(core::mem::size_of::<CodecZenohNetworkEnvelopeRecord<'static, 'static>>() <= 64);",
        "pub fn next(&mut self) -> Option<CodecZenohNetworkEnvelopeRecord<'_, 'f>> {",
        "let _peek = self.0.head(Self::skip)?;",
        "type View<'f> = CodecZenohOamView<'f>;",
        "CodecZenohOamView::read_with(reader, hook)",
    ] {
        assert!(code.contains(item), "the envelope does not generate `{item}`:\n{code}");
    }
    // One handle per arm the document names, each read as its body's view.
    for arm in [
        "CodecZenohInterest",
        "CodecZenohResponseFinal",
        "CodecZenohResponse",
        "CodecZenohRequest",
        "CodecZenohPush",
        "CodecZenohDeclare",
        "CodecZenohOam",
    ] {
        let handle = format!(
            "    {arm}(::sce_forge_runtime::codec::Record<'r, 'f, CodecZenohNetworkEnvelope{arm}Record>),"
        );
        assert!(
            code.contains(&handle),
            "no handle of the arm {arm}:\n{code}"
        );
        let made = format!("Some(CodecZenohNetworkEnvelopeRecord::{arm}(self.0.record())),");
        assert!(
            code.contains(&made),
            "`next` does not hand out the arm {arm}:\n{code}"
        );
    }
    // A record not read is skipped out of line, as the codec's own view.
    let skip = code
        .find("    fn skip(reader: &mut SceReader<'f>) {")
        .expect("the records skip a record not read");
    assert!(
        code[..skip].ends_with("    #[cold]\n    #[inline(never)]\n"),
        "the skip is not out of line"
    );
    assert!(code[skip..].contains("CodecZenohNetworkEnvelopeView::read(reader);"));
}

/// A codec with a field of its own reads its frame from that field and is no
/// record of a batch, and a codec generated without views has no view to read
/// a record as.
#[test]
fn only_a_peeked_variant_with_no_field_of_its_own_has_records() {
    for name in [
        "codec_zenoh_oam",
        "codec_zenoh_request",
        "codec_variant_dispatch",
    ] {
        let code = rust_of(name, true);
        assert!(!code.contains("Records<'f>"), "{name} generates records");
    }
    let code = rust_of("codec_zenoh_network_envelope", false);
    assert!(
        !code.contains("Records<'f>"),
        "a codec without views generates records"
    );
    let code = rust_of("codec_variant_peek_basic", true);
    assert!(code.contains("pub struct CodecVariantPeekBasicRecords<'f>("));
}

/// An arm a build leaves out has its handle, its kind and its arm in `next`
/// under its feature, and in a build without it `next` refuses its tag: the tag
/// is one the document names, and the frame it starts is not read as another.
#[test]
fn a_gated_arms_tag_is_refused_by_the_records_of_a_build_without_it() {
    let code = rust_of_document(
        &peek_variant(None, Some("gated-arm")),
        "gated_records",
        true,
    );
    let cfg = "#[cfg(feature = \"gated-arm\")]";
    let cfg_off = "#[cfg(not(feature = \"gated-arm\"))]";
    let lines: Vec<&str> = code.lines().collect();
    assert!(
        code.contains(&format!(
            "{cfg}\n#[allow(dead_code)]\npub struct GatedRecordsCodecPeekArmBRecord;"
        )),
        "the gated arm's kind is not under its feature:\n{code}"
    );
    assert_eq!(
        line_before(
            &lines,
            "impl ::sce_forge_runtime::codec::RecordRead for GatedRecordsCodecPeekArmBRecord {"
        ),
        cfg
    );
    assert_eq!(
        line_before(&lines, "CodecPeekArmB(::sce_forge_runtime::codec::Record<'r, 'f, GatedRecordsCodecPeekArmBRecord>),"),
        cfg
    );
    assert_eq!(
        line_before(
            &lines,
            "1u8 => Some(GatedRecordsRecord::CodecPeekArmB(self.0.record())),"
        ),
        cfg
    );
    let refusal = lines
        .iter()
        .position(|l| l.trim() == "self.0.fail(CodecError::VariantArmNotBuilt);")
        .unwrap_or_else(|| panic!("the records refuse no tag:\n{code}"));
    assert_eq!(lines[refusal - 1].trim(), "1u8 => {");
    assert_eq!(lines[refusal - 2].trim(), cfg_off);
    // The arm every build holds is under no feature.
    assert!(!line_before(
        &lines,
        "0u8 => Some(GatedRecordsRecord::CodecPeekArmA(self.0.record())),"
    )
    .starts_with("#[cfg"));
}

/// Records whose every arm a build may leave out would be an enum of no
/// variant in that build, which names its lifetimes nowhere: such a codec has
/// no records.
#[test]
fn a_variant_every_arm_of_which_is_gated_has_no_records() {
    let code = rust_of_document(
        &peek_variant(Some("first-arm"), Some("gated-arm")),
        "gated_records",
        true,
    );
    assert!(
        !code.contains("Records<'f>"),
        "records over gated arms alone:\n{code}"
    );
}
