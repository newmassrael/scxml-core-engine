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
//! arm A's codec. This crate declares `gated-arm` and turns it on by default.
//!
//! The default build holds arm B. `cargo test -p sce-forge-conformance
//! --no-default-features --features alloc --test forge_feature_gated_arm`
//! builds the codec without it: the module of arm B's codec is not compiled
//! either, so that build also shows the generated codec names arm B nowhere
//! outside the items its feature gates.

mod gated_arm_codecs {
    pub mod codec_default_marker_arm_a {
        include!(concat!(env!("OUT_DIR"), "/codec_default_marker_arm_a.rs"));
    }
    // Arm B's own codec is compiled only where the arm is: a build without the
    // feature that still named it anywhere would not compile.
    #[cfg(feature = "gated-arm")]
    pub mod codec_default_marker_arm_b {
        include!(concat!(env!("OUT_DIR"), "/codec_default_marker_arm_b.rs"));
    }
    pub mod codec_feature_gated_arm {
        include!(concat!(env!("OUT_DIR"), "/codec_feature_gated_arm.rs"));
    }
}

use gated_arm_codecs::codec_feature_gated_arm::{
    CodecFeatureGatedArm, CodecFeatureGatedArmVariant,
};
use sce_forge_runtime::codec::SceCursor;

/// A frame arm B's codec writes: the header byte with tag 0x02, then a
/// two-byte payload.
const ARM_B_FRAME: [u8; 3] = [0x02, 0x12, 0x34];

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

/// With the feature off, arm B does not exist, and tag 0x02 reaches the
/// catch-all: its body codec, arm A's, reads the frame as its own. It does not
/// refuse it, because a `<sce:flag value=...>` constant is the value a body's
/// `Default` holds and is not checked when a frame is decoded: arm A reads the
/// header byte (0x02) and one payload byte (0x12), and the third byte is left
/// on the cursor.
#[cfg(not(feature = "gated-arm"))]
#[test]
fn where_the_feature_is_off_its_tag_reaches_the_catch_all() {
    let mut cursor = SceCursor::new(&ARM_B_FRAME);
    let decoded = CodecFeatureGatedArm::decode(&mut cursor)
        .expect("the catch-all's body decodes the frame as its own");
    match &decoded.body {
        CodecFeatureGatedArmVariant::Default { tag, body } => {
            assert_eq!(*tag, 0x02, "the catch-all keeps the tag it could not name");
            assert_eq!((body.header, body.payload), (0x02, 0x12));
        }
        other => {
            panic!("tag 0x02 must reach the catch-all where `gated-arm` is off; got {other:?}")
        }
    }
    assert_eq!(
        cursor.remaining(),
        1,
        "arm A's codec reads two of the three bytes"
    );
}
