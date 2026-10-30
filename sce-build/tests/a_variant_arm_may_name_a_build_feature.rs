// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A variant arm may name a build feature (docs/adr/0072).
//!
//! `<sce:arm feature="<name>">` names a build feature the arm is compiled
//! under: a build without the feature has no such arm, and the arm's tags
//! reach the catch-all `<sce:default>`.
//!
//! The parser refuses the attribute where a build without the feature could
//! not be built: on the catch-all, on the `default="true"` arm, in a variant
//! with no catch-all, and with a name outside Cargo's kebab case. The deploy
//! overlay may not make a gated arm the default either. An arm without the
//! attribute serializes as it did before the attribute existed.

use sce_build::forge::diagnostic::SingleDiagnostic;
use sce_build::forge::error::{ForgeError, Located, ValidationError};
use sce_build::forge::model::{ForgeDocument, VariantArm};
use sce_build::forge::parser::parse_forge_with_imports;
use sce_build::DocumentLabel;

/// A peek-byte variant over a one-bit tag, with `arms` as its children.
fn peek_variant(arms: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="gated">
  <sce:import src="codec_peek_arm_a.scxml" kind="codec" as="codec_peek_arm_a"/>
  <sce:import src="codec_peek_arm_b.scxml" kind="codec" as="codec_peek_arm_b"/>
  <datamodel>
    <sce:variant tag="peek.kind">
      <sce:peek-byte id="peek" sce:type="uint8">
        <sce:flag name="kind" bit="0" width="1"/>
      </sce:peek-byte>
{arms}
    </sce:variant>
  </datamodel>
</scxml>"#
    )
}

fn parse(text: &str) -> Result<ForgeDocument, Located<ForgeError>> {
    parse_forge_with_imports(text, DocumentLabel::symmetric("gated"))
        .map(|parsed| parsed.expect("a codec document").document)
}

fn refusal(text: &str) -> Located<ForgeError> {
    match parse(text) {
        Ok(_) => panic!("the document must be refused:\n{text}"),
        Err(e) => e,
    }
}

fn arms_of(document: &ForgeDocument) -> &[VariantArm] {
    match document {
        ForgeDocument::Codec(m) => &m.variant.as_ref().expect("a variant").arms,
        _ => panic!("expected a codec"),
    }
}

/// A name in the grammar reaches the arm it is written on, and only that arm;
/// an arm without one serializes as it did before the attribute existed.
#[test]
fn a_feature_reaches_its_arm_and_an_arm_without_one_serializes_as_before() {
    let document = parse(&peek_variant(
        r#"      <sce:arm value="0x00" type="codec_peek_arm_a" default="true"/>
      <sce:arm value="0x01" type="codec_peek_arm_b" feature="peek-arm-b2"/>
      <sce:default type="codec_peek_arm_a"/>"#,
    ))
    .expect("a gated arm beside a catch-all parses");
    let arms = arms_of(&document);
    assert_eq!(arms[0].feature, None);
    assert_eq!(arms[1].feature.as_deref(), Some("peek-arm-b2"));

    let plain = serde_json::to_value(&arms[0]).expect("an arm serializes");
    assert!(
        plain.get("feature").is_none(),
        "an arm without a feature must not carry the key on the wire: {plain}"
    );
    let gated = serde_json::to_value(&arms[1]).expect("an arm serializes");
    assert_eq!(gated["feature"], "peek-arm-b2");
}

/// The catch-all receives the tags of every arm a build leaves out, so it is
/// held by every build.
#[test]
fn the_catch_all_may_not_name_a_feature() {
    let err = refusal(&peek_variant(
        r#"      <sce:arm value="0x00" type="codec_peek_arm_a" default="true"/>
      <sce:arm value="0x01" type="codec_peek_arm_b"/>
      <sce:default type="codec_peek_arm_a" feature="peek-default"/>"#,
    ));
    assert_eq!(
        err.diagnostic_payload().code.as_str(),
        "codec/variant-feature-on-required-arm"
    );
    assert!(
        matches!(
            err.error,
            ForgeError::Validation(ref boxed) if matches!(
                **boxed,
                ValidationError::CodecVariantFeatureOnRequiredArm {
                    arm_value: None, ref feature, ..
                } if feature == "peek-default"
            )
        ),
        "the refusal names the catch-all and its feature; got {:?}",
        err.error
    );
}

/// The codec's `Default` value is built from the `default="true"` arm, so it
/// is held by every build.
#[test]
fn the_default_arm_may_not_name_a_feature() {
    let err = refusal(&peek_variant(
        r#"      <sce:arm value="0x00" type="codec_peek_arm_a" default="true" feature="peek-a"/>
      <sce:arm value="0x01" type="codec_peek_arm_b"/>
      <sce:default type="codec_peek_arm_a"/>"#,
    ));
    assert_eq!(
        err.diagnostic_payload().code.as_str(),
        "codec/variant-feature-on-required-arm"
    );
    assert!(
        matches!(
            err.error,
            ForgeError::Validation(ref boxed) if matches!(
                **boxed,
                ValidationError::CodecVariantFeatureOnRequiredArm {
                    arm_value: Some(0x00), ref feature, ..
                } if feature == "peek-a"
            )
        ),
        "the refusal names the default arm and its feature; got {:?}",
        err.error
    );
}

/// Without a catch-all the arms are proven to cover the tag only all
/// together: here the two values of a one-bit tag, which a build without the
/// feature would no longer cover.
#[test]
fn a_feature_needs_a_catch_all_for_the_tags_it_leaves_out() {
    let err = refusal(&peek_variant(
        r#"      <sce:arm value="0x00" type="codec_peek_arm_a" default="true"/>
      <sce:arm value="0x01" type="codec_peek_arm_b" feature="peek-b"/>"#,
    ));
    assert_eq!(
        err.diagnostic_payload().code.as_str(),
        "codec/variant-feature-needs-catch-all"
    );
    assert!(
        matches!(
            err.error,
            ForgeError::Validation(ref boxed) if matches!(
                **boxed,
                ValidationError::CodecVariantFeatureNeedsCatchAll {
                    arm_value: 0x01, ref feature, ..
                } if feature == "peek-b"
            )
        ),
        "the refusal names the gated arm and its feature; got {:?}",
        err.error
    );
}

/// A name outside `[a-z][a-z0-9-]*` is no Cargo feature in kebab case, or
/// spells the same C macro as another name.
#[test]
fn a_feature_name_outside_the_kebab_grammar_is_refused() {
    for name in ["", "Peek", "peek_b", "9peek", "-peek", "peek.b"] {
        let err = refusal(&peek_variant(&format!(
            r#"      <sce:arm value="0x00" type="codec_peek_arm_a" default="true"/>
      <sce:arm value="0x01" type="codec_peek_arm_b" feature="{name}"/>
      <sce:default type="codec_peek_arm_a"/>"#
        )));
        assert_eq!(
            err.diagnostic_payload().code.as_str(),
            "validation/attribute-rule-violated",
            "feature=\"{name}\" must be refused for its spelling"
        );
        assert!(
            matches!(
                err.error,
                ForgeError::Validation(ref boxed) if matches!(
                    **boxed,
                    ValidationError::AttributeRuleViolated { ref attr, ref value, .. }
                        if attr == "feature" && value == name
                )
            ),
            "the refusal names the attribute and the name as written; got {:?}",
            err.error
        );
    }
}

/// The deploy overlay chooses the default arm after the parser ran, and an
/// arm under a feature is no more a default there than in the document.
#[test]
fn the_overlay_may_not_make_a_gated_arm_the_default() {
    use sce_build::forge::variant_default_overlay::apply_variant_default_overlay;
    use sce_build::mesh::deploy::parse_deploy_str;

    let mut document = parse(&peek_variant(
        r#"      <sce:arm value="0x00" type="codec_peek_arm_a" default="true"/>
      <sce:arm value="0x01" type="codec_peek_arm_b" feature="peek-b"/>
      <sce:default type="codec_peek_arm_a"/>"#,
    ))
    .expect("the document parses");
    let deploy =
        parse_deploy_str("version: \"1.0\"\ntopology: {}\nvariant_defaults:\n  gated: 0x1\n")
            .expect("the deploy parses");
    let err = match apply_variant_default_overlay(&mut document, &deploy, "gated") {
        Ok(()) => panic!("the overlay names the gated arm 0x01 and must be refused"),
        Err(e) => e,
    };
    assert_eq!(
        err.diagnostic_payload().code.as_str(),
        "codec/variant-feature-on-required-arm"
    );
    assert!(
        matches!(
            err.error,
            ForgeError::Validation(ref boxed) if matches!(
                **boxed,
                ValidationError::CodecVariantFeatureOnRequiredArm {
                    arm_value: Some(0x01), ref feature, ..
                } if feature == "peek-b"
            )
        ),
        "got {:?}",
        err.error
    );
}
