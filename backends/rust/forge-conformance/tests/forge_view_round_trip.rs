// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The Rust view of a decoded codec (docs/adr/0049): `{Codec}View` holds a
// bounded list as the frame it was read from, and its decode is one scan that
// checks what the eager `decode` checks.
//
// What is asserted is the gate of the ADR's step 1: for the codecs whose lists
// are scanned every way the generator scans one (a counted repeat, until-EOF,
// a gated repeat, a chain, a chain that threads a value from one entry to the
// next, a gated chain, an embed, a gated embed, a list of bodies that hold
// lists), a view decodes exactly the frames `decode` decodes, refuses the
// others with the same error, leaves the cursor where `decode` leaves it, and
// yields the entries `decode` returned. The frames are not hand-picked: every
// frame a seeded generator produces is decoded both ways, and for each the
// eager decode accepts, every prefix of it and every one-byte change of it, so
// the refusals a damaged frame meets (a list past its bound, a chain cut short
// or run past its depth, a body that stops early) are compared too. That a
// codec's sweep accepted enough frames to mean something is asserted with it.
//
// The view's compiling is the catalog's: `build.rs` generates every fixture
// with the option on.
//
// Gated on `alloc` like the other round-trip tests.
#![cfg(feature = "alloc")]

mod codecs {
    pub mod codec_zenoh_locator {
        include!(concat!(env!("OUT_DIR"), "/codec_zenoh_locator.rs"));
    }
    pub mod codec_nested_body {
        include!(concat!(env!("OUT_DIR"), "/codec_nested_body.rs"));
    }
    pub mod codec_nested_parent {
        include!(concat!(env!("OUT_DIR"), "/codec_nested_parent.rs"));
    }
    pub mod codec_tlv_entry {
        include!(concat!(env!("OUT_DIR"), "/codec_tlv_entry.rs"));
    }
    pub mod codec_tlv_chain_basic {
        include!(concat!(env!("OUT_DIR"), "/codec_tlv_chain_basic.rs"));
    }
    pub mod codec_tlv_chain_present_if_basic {
        include!(concat!(
            env!("OUT_DIR"),
            "/codec_tlv_chain_present_if_basic.rs"
        ));
    }
    pub mod codec_chain_prev_entry {
        include!(concat!(env!("OUT_DIR"), "/codec_chain_prev_entry.rs"));
    }
    pub mod codec_chain_prev_carrier {
        include!(concat!(env!("OUT_DIR"), "/codec_chain_prev_carrier.rs"));
    }
    pub mod codec_chain_prev_gated {
        include!(concat!(env!("OUT_DIR"), "/codec_chain_prev_gated.rs"));
    }
    pub mod codec_repeat_elem {
        include!(concat!(env!("OUT_DIR"), "/codec_repeat_elem.rs"));
    }
    pub mod codec_repeat_basic {
        include!(concat!(env!("OUT_DIR"), "/codec_repeat_basic.rs"));
    }
    pub mod codec_until_eof_basic {
        include!(concat!(env!("OUT_DIR"), "/codec_until_eof_basic.rs"));
    }
    pub mod codec_repeat_present_if_basic {
        include!(concat!(
            env!("OUT_DIR"),
            "/codec_repeat_present_if_basic.rs"
        ));
    }
    pub mod codec_chain_flag_entry {
        include!(concat!(env!("OUT_DIR"), "/codec_chain_flag_entry.rs"));
    }
    pub mod codec_repeat_flag_carrier {
        include!(concat!(env!("OUT_DIR"), "/codec_repeat_flag_carrier.rs"));
    }
    pub mod codec_chain_has_tagged_entry {
        include!(concat!(env!("OUT_DIR"), "/codec_chain_has_tagged_entry.rs"));
    }
    pub mod codec_chain_has_tagged {
        include!(concat!(env!("OUT_DIR"), "/codec_chain_has_tagged.rs"));
    }
    pub mod codec_embed_basic {
        include!(concat!(env!("OUT_DIR"), "/codec_embed_basic.rs"));
    }
    pub mod codec_variant_session_open {
        include!(concat!(env!("OUT_DIR"), "/codec_variant_session_open.rs"));
    }
    pub mod codec_variant_session_close {
        include!(concat!(env!("OUT_DIR"), "/codec_variant_session_close.rs"));
    }
    pub mod codec_variant_dispatch {
        include!(concat!(env!("OUT_DIR"), "/codec_variant_dispatch.rs"));
    }
    pub mod codec_peek_arm_a {
        include!(concat!(env!("OUT_DIR"), "/codec_peek_arm_a.rs"));
    }
    pub mod codec_peek_arm_b {
        include!(concat!(env!("OUT_DIR"), "/codec_peek_arm_b.rs"));
    }
    pub mod codec_variant_peek_basic {
        include!(concat!(env!("OUT_DIR"), "/codec_variant_peek_basic.rs"));
    }
    // The request and the response of the Zenoh network layer, with what they
    // embed, list and dispatch to: the codecs a view is for.
    pub mod codec_zenoh_timestamp {
        include!(concat!(env!("OUT_DIR"), "/codec_zenoh_timestamp.rs"));
    }
    pub mod codec_zenoh_encoding {
        include!(concat!(env!("OUT_DIR"), "/codec_zenoh_encoding.rs"));
    }
    pub mod codec_zenoh_ext_unit {
        include!(concat!(env!("OUT_DIR"), "/codec_zenoh_ext_unit.rs"));
    }
    pub mod codec_zenoh_ext_zint {
        include!(concat!(env!("OUT_DIR"), "/codec_zenoh_ext_zint.rs"));
    }
    pub mod codec_zenoh_ext_zbuf {
        include!(concat!(env!("OUT_DIR"), "/codec_zenoh_ext_zbuf.rs"));
    }
    pub mod codec_zenoh_ext_entry {
        include!(concat!(env!("OUT_DIR"), "/codec_zenoh_ext_entry.rs"));
    }
    pub mod codec_zenoh_wireexpr {
        include!(concat!(env!("OUT_DIR"), "/codec_zenoh_wireexpr.rs"));
    }
    pub mod codec_zenoh_put {
        include!(concat!(env!("OUT_DIR"), "/codec_zenoh_put.rs"));
    }
    pub mod codec_zenoh_del {
        include!(concat!(env!("OUT_DIR"), "/codec_zenoh_del.rs"));
    }
    pub mod codec_zenoh_msg_put {
        include!(concat!(env!("OUT_DIR"), "/codec_zenoh_msg_put.rs"));
    }
    pub mod codec_zenoh_msg_del {
        include!(concat!(env!("OUT_DIR"), "/codec_zenoh_msg_del.rs"));
    }
    pub mod codec_zenoh_query {
        include!(concat!(env!("OUT_DIR"), "/codec_zenoh_query.rs"));
    }
    pub mod codec_zenoh_push_body {
        include!(concat!(env!("OUT_DIR"), "/codec_zenoh_push_body.rs"));
    }
    pub mod codec_zenoh_reply {
        include!(concat!(env!("OUT_DIR"), "/codec_zenoh_reply.rs"));
    }
    pub mod codec_zenoh_err {
        include!(concat!(env!("OUT_DIR"), "/codec_zenoh_err.rs"));
    }
    pub mod codec_zenoh_request {
        include!(concat!(env!("OUT_DIR"), "/codec_zenoh_request.rs"));
    }
    pub mod codec_zenoh_response {
        include!(concat!(env!("OUT_DIR"), "/codec_zenoh_response.rs"));
    }
}

use codecs::codec_chain_has_tagged::{CodecChainHasTagged, CodecChainHasTaggedView};
use codecs::codec_chain_prev_carrier::{CodecChainPrevCarrier, CodecChainPrevCarrierView};
use codecs::codec_chain_prev_gated::{CodecChainPrevGated, CodecChainPrevGatedView};
use codecs::codec_embed_basic::{CodecEmbedBasic, CodecEmbedBasicView};
use codecs::codec_nested_parent::{CodecNestedParent, CodecNestedParentView};
use codecs::codec_repeat_basic::{CodecRepeatBasic, CodecRepeatBasicView};
use codecs::codec_repeat_flag_carrier::{CodecRepeatFlagCarrier, CodecRepeatFlagCarrierView};
use codecs::codec_repeat_present_if_basic::{
    CodecRepeatPresentIfBasic, CodecRepeatPresentIfBasicView,
};
use codecs::codec_tlv_chain_basic::{CodecTlvChainBasic, CodecTlvChainBasicView};
use codecs::codec_tlv_chain_present_if_basic::{
    CodecTlvChainPresentIfBasic, CodecTlvChainPresentIfBasicView,
};
use codecs::codec_until_eof_basic::{CodecUntilEofBasic, CodecUntilEofBasicView};
use codecs::codec_variant_dispatch::{CodecVariantDispatch, CodecVariantDispatchView};
use codecs::codec_variant_peek_basic::{CodecVariantPeekBasic, CodecVariantPeekBasicView};
use codecs::codec_zenoh_request::{CodecZenohRequest, CodecZenohRequestView};
use codecs::codec_zenoh_response::{CodecZenohResponse, CodecZenohResponseView};
use sce_forge_runtime::codec::{SceCursor, VecSink};

/// A deterministic byte source: the same frames on every run and every host.
struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as usize
    }
}

/// What one frame did, decoded as the codec and as its view.
enum Verdict {
    /// Both decoded it and agree on the value and on where the cursor stopped.
    Accepted,
    /// Both refused it with the same error.
    Refused,
}

/// A view prints as the codec it stands for with `View` after each name; the
/// two spell the same value once that is taken out.
fn as_eager(view_debug: String) -> String {
    view_debug
        .replace("ViewVariant", "Variant")
        .replace("View", "")
}

/// Decode `frame` both ways and say what they agreed on, or panic on the first
/// thing they do not.
macro_rules! agree {
    ($eager:ty, $view:ty, $frame:expr) => {{
        let frame: &[u8] = $frame;
        let mut eager_cursor = SceCursor::new(frame);
        let mut view_cursor = SceCursor::new(frame);
        let eager = <$eager>::decode(&mut eager_cursor);
        let view = <$view>::decode(&mut view_cursor);
        match (eager, view) {
            (Ok(eager), Ok(view)) => {
                assert_eq!(
                    as_eager(format!("{view:?}")),
                    format!("{eager:?}"),
                    "the view says another value for frame {frame:02x?}"
                );
                assert_eq!(
                    view_cursor.remaining(),
                    eager_cursor.remaining(),
                    "the view stops elsewhere for frame {frame:02x?}"
                );
                // The view says the same thing in the builder form: filling a
                // codec from it gives the value `decode` returned.
                let mut rebuilt = <$eager>::default();
                view.into_builder(&mut rebuilt)
                    .unwrap_or_else(|e| panic!("into_builder failed ({e:?}) for {frame:02x?}"));
                assert_eq!(
                    rebuilt, eager,
                    "into_builder says another value for frame {frame:02x?}"
                );
                // And it writes the bytes the builder form writes, or refuses
                // as it does, without the builder.
                let mut from_builder = Vec::new();
                let built = rebuilt.encode(&mut VecSink::new(&mut from_builder));
                let mut from_view = Vec::new();
                let written = view.encode(&mut VecSink::new(&mut from_view));
                assert_eq!(
                    written, built,
                    "the view encodes to another verdict for frame {frame:02x?}"
                );
                assert_eq!(
                    from_view, from_builder,
                    "the view encodes other bytes for frame {frame:02x?}"
                );
                Verdict::Accepted
            }
            (Err(eager), Err(view)) => {
                assert_eq!(view, eager, "another error for frame {frame:02x?}");
                Verdict::Refused
            }
            (eager, view) => panic!(
                "eager {:?} but view {:?} for frame {frame:02x?}",
                eager.map(|_| ()),
                view.map(|_| ())
            ),
        }
    }};
}

/// Sweep: the `seeds` and `frames` seeded frames of up to `max_len` bytes drawn
/// from `pool`; for each the eager decode accepts, its prefixes and its one-byte
/// changes. A codec whose frames a random byte string rarely makes is given
/// seeds, which are the frames to change. Returns how many frames were
/// accepted, so a sweep that proved nothing says so.
macro_rules! sweep {
    ($eager:ty, $view:ty, $pool:expr, $frames:expr, $max_len:expr) => {
        sweep!($eager, $view, $pool, Vec::new(), $frames, $max_len)
    };
    ($eager:ty, $view:ty, $pool:expr, $seeds:expr, $frames:expr, $max_len:expr) => {{
        let pool: &[u8] = $pool;
        let seeds: Vec<Vec<u8>> = $seeds;
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        let mut accepted = 0usize;
        let random = (0..$frames).map(|_| {
            let len = rng.below($max_len + 1);
            (0..len)
                .map(|_| pool[rng.below(pool.len())])
                .collect::<Vec<u8>>()
        });
        let frames: Vec<Vec<u8>> = seeds.into_iter().chain(random).collect();
        for frame in &frames {
            if let Verdict::Accepted = agree!($eager, $view, frame) {
                accepted += 1;
                for cut in 0..frame.len() {
                    agree!($eager, $view, &frame[..cut]);
                }
                for at in 0..frame.len() {
                    for &b in pool {
                        let mut changed = frame.clone();
                        changed[at] = b;
                        agree!($eager, $view, &changed);
                    }
                }
            }
        }
        accepted
    }};
}

#[test]
fn a_counted_repeat_is_scanned_as_it_is_decoded() {
    let accepted = sweep!(
        CodecRepeatBasic,
        CodecRepeatBasicView,
        &[0, 1, 2, 3, 4, 5, 6, 7, 0xff],
        4000,
        14
    );
    assert!(accepted > 100, "only {accepted} frames were accepted");
}

#[test]
fn an_until_eof_repeat_is_scanned_as_it_is_decoded() {
    let accepted = sweep!(
        CodecUntilEofBasic,
        CodecUntilEofBasicView,
        &[0, 1, 2, 3, 0xff],
        4000,
        14
    );
    assert!(accepted > 100, "only {accepted} frames were accepted");
}

#[test]
fn a_gated_repeat_is_scanned_as_it_is_decoded() {
    let accepted = sweep!(
        CodecRepeatPresentIfBasic,
        CodecRepeatPresentIfBasicView,
        &[0, 1, 2, 3, 4, 0x80, 0x81, 0xff],
        6000,
        14
    );
    assert!(accepted > 100, "only {accepted} frames were accepted");
}

#[test]
fn a_repeat_whose_entries_take_a_flag_from_the_parent_is_scanned() {
    let accepted = sweep!(
        CodecRepeatFlagCarrier,
        CodecRepeatFlagCarrierView,
        &[0, 1, 2, 3, 0x80, 0x81, 0x01, 0xff],
        6000,
        14
    );
    assert!(accepted > 100, "only {accepted} frames were accepted");
}

#[test]
fn a_chain_is_scanned_as_it_is_decoded() {
    let accepted = sweep!(
        CodecTlvChainBasic,
        CodecTlvChainBasicView,
        &[0, 1, 2, 3, 0x80, 0x81, 0x82, 0xff],
        6000,
        16
    );
    assert!(accepted > 100, "only {accepted} frames were accepted");
}

#[test]
fn a_gated_chain_is_scanned_as_it_is_decoded() {
    let accepted = sweep!(
        CodecTlvChainPresentIfBasic,
        CodecTlvChainPresentIfBasicView,
        &[0, 1, 2, 3, 0x80, 0x81, 0x82, 0xff],
        6000,
        16
    );
    assert!(accepted > 100, "only {accepted} frames were accepted");
}

#[test]
fn a_chain_that_threads_a_value_from_entry_to_entry_is_scanned() {
    // 0x04 is the marker the next entry is laid out after, 0x84 the same with
    // a successor declared; 0x01 and 0x81 an ordinary entry and the same with
    // a successor.
    let accepted = sweep!(
        CodecChainPrevCarrier,
        CodecChainPrevCarrierView,
        &[0x00, 0x01, 0x04, 0x84, 0x81, 0x05, 0x85, 0x12, 0x34],
        8000,
        14
    );
    assert!(accepted > 100, "only {accepted} frames were accepted");
}

#[test]
fn a_gated_chain_that_threads_a_value_is_scanned() {
    let accepted = sweep!(
        CodecChainPrevGated,
        CodecChainPrevGatedView,
        &[0x00, 0x01, 0x02, 0x04, 0x84, 0x81, 0x05, 0x85, 0x12, 0x34],
        8000,
        14
    );
    assert!(accepted > 100, "only {accepted} frames were accepted");
}

#[test]
fn bodies_that_hold_lists_are_scanned_inside_a_list_and_behind_a_gate() {
    let accepted = sweep!(
        CodecNestedParent,
        CodecNestedParentView,
        &[0, 1, 2, 3, b'a', b'b', b'c'],
        20000,
        14
    );
    assert!(accepted > 50, "only {accepted} frames were accepted");
}

/// Fields gated on whether a chain holds an entry of some identifier are
/// decided during the scan, from the entries the view then does not keep.
#[test]
fn a_field_gated_on_the_entries_of_a_chain_is_decided_by_the_scan() {
    let accepted = sweep!(
        CodecChainHasTagged,
        CodecChainHasTaggedView,
        &[0x00, 0x01, 0x07, 0x09, 0x80, 0x81, 0x87, 0x89, 0x05],
        8000,
        12
    );
    assert!(accepted > 100, "only {accepted} frames were accepted");
}

#[test]
fn an_embedded_body_is_decoded_as_its_own_view() {
    let accepted = sweep!(
        CodecEmbedBasic,
        CodecEmbedBasicView,
        &[0, 1, 2, 3, b'a', b'b'],
        4000,
        10
    );
    assert!(accepted > 100, "only {accepted} frames were accepted");
}

#[test]
fn a_variant_is_decoded_as_an_enum_of_views() {
    let accepted = sweep!(
        CodecVariantDispatch,
        CodecVariantDispatchView,
        &[0, 1, 2, 3, 4, 5, 0x80, 0xff],
        6000,
        10
    );
    assert!(accepted > 100, "only {accepted} frames were accepted");
    let accepted = sweep!(
        CodecVariantPeekBasic,
        CodecVariantPeekBasicView,
        &[0, 1, 2, 3, 4, 5, 0x80, 0xff],
        6000,
        10
    );
    assert!(accepted > 100, "only {accepted} frames were accepted");
}

/// The request and the response of the Zenoh network layer: an embedded
/// key expression, a chain of extensions ended by a flag, and a body chosen by
/// the byte the frame opens with, every one of which holds a list or a body that
/// does. Random bytes rarely make such a frame, so the sweep changes the frame
/// each codec encodes by default.
#[test]
fn the_zenoh_request_and_response_are_decoded_as_views() {
    let pool = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x1c, 0x1d, 0x20, 0x40, 0x80, 0x81, 0xa0, 0xff,
    ];
    let accepted = sweep!(
        CodecZenohRequest,
        CodecZenohRequestView,
        &pool,
        vec![CodecZenohRequest::default()
            .encode_to_vec()
            .expect("the default request encodes")],
        4000,
        24
    );
    assert!(accepted > 20, "only {accepted} frames were accepted");
    let accepted = sweep!(
        CodecZenohResponse,
        CodecZenohResponseView,
        &pool,
        vec![CodecZenohResponse::default()
            .encode_to_vec()
            .expect("the default response encodes")],
        4000,
        24
    );
    assert!(accepted > 20, "only {accepted} frames were accepted");
}

/// The walk of a chain hands each entry the value the scan handed it: the entry
/// after a marker is laid out as the entry after a marker, though nothing but
/// the closure the generator wrote tells the walk that.
#[test]
fn a_walk_rebuilds_the_value_a_chain_carries_from_one_entry_to_the_next() {
    // head: T clear; a marker entry (0x84: id 4, a successor follows) with its
    // one-byte body; then an entry laid out after a marker, with a two-byte
    // body (0x01: id 1, last).
    let frame = [0x00, 0x84, 0xAA, 0x01, 0x12, 0x34];
    let eager = CodecChainPrevCarrier::decode(&mut SceCursor::new(&frame)).unwrap();
    let view = CodecChainPrevCarrierView::decode(&mut SceCursor::new(&frame)).unwrap();
    assert_eq!(view.entries.len(), 2);
    let walked: Vec<_> = view.entries.iter().collect();
    let decoded: Vec<_> = eager.entries.iter().cloned().collect();
    assert_eq!(walked, decoded);
    assert_eq!(walked[1].long_value, Some(0x1234));
    // Reading it again, and by index, gives the same.
    assert_eq!(view.entries.iter().collect::<Vec<_>>(), walked);
    assert_eq!(view.entries.get(1), Some(decoded[1].clone()));
    assert_eq!(view.entries.get(2), None);
}

#[test]
fn a_view_does_not_grow_with_the_bound_of_its_lists() {
    use core::mem::size_of;
    // A chain bounded at 8 and a gated chain bounded at 8: the eager value
    // holds every entry it could hold, the view none.
    assert!(size_of::<CodecTlvChainBasicView>() < size_of::<CodecTlvChainBasic>());
    assert!(size_of::<CodecNestedParentView>() < size_of::<CodecNestedParent>());
}
