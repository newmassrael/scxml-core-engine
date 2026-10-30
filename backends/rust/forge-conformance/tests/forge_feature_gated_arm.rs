// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The round trip reaches `encode_to_vec`, which is gated on `alloc`, as the
// other generated-codec tests of this crate are.
#![cfg(feature = "alloc")]

//! An arm compiled under a build feature (docs/adr/0072), compiled and run in
//! both builds.
//!
//! `fixtures/codec_feature_gated_arm.scxml` dispatches on the low two bits of
//! the next byte: 0x01 to arm A (the default arm), 0x02 to arm B, which names
//! the feature `gated-arm`, and anything else to the catch-all, whose body is
//! arm A's codec. `fixtures/codec_feature_gated_arm_no_catch_all.scxml`
//! dispatches on one bit: 0x00 to its arm A, 0x01 to its arm B under the same
//! feature, and has no catch-all. This crate declares `gated-arm` and turns it
//! on by default.
//!
//! The default build holds both arms B. `cargo test -p sce-forge-conformance
//! --no-default-features --features alloc --test forge_feature_gated_arm`
//! builds the codecs without them: the modules of the arm B codecs are not
//! compiled either, so that build also shows the generated codecs name arm B
//! nowhere outside the items its feature gates. There, a frame of arm B is
//! refused with `CodecError::VariantArmNotBuilt`: the tag is one the document
//! names, so the catch-all does not read it as its own.

mod gated_arm_codecs {
    pub mod codec_default_marker_arm_a {
        include!(concat!(env!("OUT_DIR"), "/codec_default_marker_arm_a.rs"));
    }
    // An arm B's own codec is compiled only where the arm is: a build without
    // the feature that still named it anywhere would not compile.
    #[cfg(feature = "gated-arm")]
    pub mod codec_default_marker_arm_b {
        include!(concat!(env!("OUT_DIR"), "/codec_default_marker_arm_b.rs"));
    }
    pub mod codec_feature_gated_arm {
        include!(concat!(env!("OUT_DIR"), "/codec_feature_gated_arm.rs"));
    }
    pub mod codec_peek_arm_a {
        include!(concat!(env!("OUT_DIR"), "/codec_peek_arm_a.rs"));
    }
    #[cfg(feature = "gated-arm")]
    pub mod codec_peek_arm_b {
        include!(concat!(env!("OUT_DIR"), "/codec_peek_arm_b.rs"));
    }
    pub mod codec_feature_gated_arm_no_catch_all {
        include!(concat!(
            env!("OUT_DIR"),
            "/codec_feature_gated_arm_no_catch_all.rs"
        ));
    }
}

use gated_arm_codecs::codec_feature_gated_arm::{
    CodecFeatureGatedArm, CodecFeatureGatedArmVariant,
};
use gated_arm_codecs::codec_feature_gated_arm_no_catch_all::{
    CodecFeatureGatedArmNoCatchAll, CodecFeatureGatedArmNoCatchAllVariant,
};
use sce_forge_runtime::codec::SceCursor;

/// A frame arm B's codec writes: the header byte with tag 0x02, then a
/// two-byte payload.
const ARM_B_FRAME: [u8; 3] = [0x02, 0x12, 0x34];

/// A frame the peek-byte arm B writes: the header byte with tag 0x01, then a
/// two-byte payload.
const PEEK_ARM_B_FRAME: [u8; 3] = [0x01, 0x12, 0x34];

/// Arm A is the default arm in every build: the default value is arm A's, and
/// it decodes back into arm A.
#[test]
fn the_default_arm_is_held_by_every_build() {
    let bytes = CodecFeatureGatedArm::default().encode_to_vec();
    assert_eq!(
        bytes,
        vec![0x01, 0x00],
        "the default value is arm A's (header 0x01, one payload byte)"
    );
    let mut cursor = SceCursor::new(&bytes);
    let decoded = CodecFeatureGatedArm::decode(&mut cursor).expect("arm A's frame decodes");
    assert_eq!(cursor.remaining(), 0);
    match decoded.body {
        CodecFeatureGatedArmVariant::CodecDefaultMarkerArmA(a) => {
            assert_eq!((a.header, a.payload), (0x01, 0x00));
        }
        other => panic!("tag 0x01 must decode into arm A; got {other:?}"),
    }
}

/// A tag no arm names reaches the catch-all in every build.
#[test]
fn a_tag_the_document_does_not_name_reaches_the_catch_all() {
    let frame = [0x03, 0x07];
    let mut cursor = SceCursor::new(&frame);
    let decoded = CodecFeatureGatedArm::decode(&mut cursor).expect("the catch-all decodes");
    match &decoded.body {
        CodecFeatureGatedArmVariant::Default { tag, body } => {
            assert_eq!(*tag, 0x03);
            assert_eq!((body.header, body.payload), (0x03, 0x07));
        }
        other => panic!("tag 0x03 must reach the catch-all; got {other:?}"),
    }
}

/// With the feature on, tag 0x02 decodes into arm B, which reads its
/// two-byte payload, and the frame re-encodes byte for byte.
#[cfg(feature = "gated-arm")]
#[test]
fn where_the_feature_is_on_its_tag_decodes_into_its_arm() {
    let mut cursor = SceCursor::new(&ARM_B_FRAME);
    let decoded = CodecFeatureGatedArm::decode(&mut cursor).expect("arm B's frame decodes");
    assert_eq!(cursor.remaining(), 0, "arm B reads the whole frame");
    match &decoded.body {
        CodecFeatureGatedArmVariant::CodecDefaultMarkerArmB(b) => {
            assert_eq!((b.header, b.payload), (0x02, 0x1234));
        }
        other => panic!("tag 0x02 must decode into arm B where `gated-arm` is on; got {other:?}"),
    }
    assert_eq!(decoded.encode_to_vec(), ARM_B_FRAME.to_vec());
}

/// With the feature off, arm B does not exist, and tag 0x02 is refused: it is
/// a tag the document names, and reading the frame with the catch-all's body
/// (arm A's codec) would take a frame of arm B for one of arm A.
#[cfg(not(feature = "gated-arm"))]
#[test]
fn where_the_feature_is_off_its_tag_is_refused() {
    let mut cursor = SceCursor::new(&ARM_B_FRAME);
    match CodecFeatureGatedArm::decode(&mut cursor) {
        Err(e) => assert_eq!(e, sce_forge_runtime::codec::CodecError::VariantArmNotBuilt),
        Ok(decoded) => panic!(
            "tag 0x02 must be refused where `gated-arm` is off; got {:?}",
            decoded.body
        ),
    }
}

/// Without a catch-all, the feature on: tag 0x01 decodes into arm B.
#[cfg(feature = "gated-arm")]
#[test]
fn without_a_catch_all_the_feature_on_decodes_the_gated_arm() {
    let mut cursor = SceCursor::new(&PEEK_ARM_B_FRAME);
    let decoded = CodecFeatureGatedArmNoCatchAll::decode(&mut cursor)
        .expect("the peek-byte arm B's frame decodes");
    assert_eq!(cursor.remaining(), 0, "arm B reads the whole frame");
    match &decoded.body {
        CodecFeatureGatedArmNoCatchAllVariant::CodecPeekArmB(b) => {
            assert_eq!((b.header, b.payload), (0x01, 0x1234));
        }
        other => panic!("tag 0x01 must decode into arm B; got {other:?}"),
    }
}

/// Without a catch-all, the feature off: tag 0x01 is refused, and the decode
/// reaches no branch the build-time proof of the arms called unreachable.
#[cfg(not(feature = "gated-arm"))]
#[test]
fn without_a_catch_all_the_feature_off_refuses_the_gated_arms_tag() {
    let mut cursor = SceCursor::new(&PEEK_ARM_B_FRAME);
    match CodecFeatureGatedArmNoCatchAll::decode(&mut cursor) {
        Err(e) => assert_eq!(e, sce_forge_runtime::codec::CodecError::VariantArmNotBuilt),
        Ok(decoded) => panic!(
            "tag 0x01 must be refused where `gated-arm` is off; got {:?}",
            decoded.body
        ),
    }
}

/// Without a catch-all, the arm every build holds decodes in every build. The
/// comparison is by value because a build without the feature has arm A alone,
/// where a `match` with a second branch would not compile under the crate's
/// lints.
#[test]
fn without_a_catch_all_the_ungated_arm_decodes_in_every_build() {
    let frame = [0x00, 0x07];
    let mut cursor = SceCursor::new(&frame);
    let decoded = CodecFeatureGatedArmNoCatchAll::decode(&mut cursor).expect("arm A decodes");
    assert_eq!(
        decoded.body,
        CodecFeatureGatedArmNoCatchAllVariant::CodecPeekArmA(
            gated_arm_codecs::codec_peek_arm_a::CodecPeekArmA {
                header: 0x00,
                payload: 0x07,
            }
        )
    );
}
