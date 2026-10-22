// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The generated views, run against the eager codecs they sit beside (ADR 0049).
//!
//! `tests/forge/expected/views/` holds the codecs the generator writes with
//! `--views`, and the conformance test holds their text to what it generates. That
//! text compiles, but says nothing of what a view DOES with a frame. A view claims
//! to refuse exactly the frames the eager decode refuses, with the same error, to
//! consume the same bytes, and to hold the same list as the eager decode does; the
//! lists are read again by a walk (`crate::view::ListWalk`) that rebuilds, per
//! entry, the values the scan saw. This module is that claim asked of the real
//! output, over a few tens of thousands of frames drawn from a fixed seed: for
//! every codec with a list of each kind (a repeat to a count, to the end of the
//! frame, a repeat with a flag carried to its entries, a chain that ends on a flag,
//! a chain that carries the previous entry's flag), the view and the eager decode
//! must answer alike, and the builder a view fills must equal what the eager decode
//! returned.
//!
//! The goldens are compiled as modules of this crate, from where they are, so what
//! is run is what is committed. Each is `#[rustfmt::skip]`: a golden is the
//! generator's text to the byte, which the conformance test compares, and a crate
//! that names a file as a module is a crate `cargo fmt` rewrites.

// `into_builder(&self, ..)` fills a builder the caller owns, and is public in the
// codecs a consumer compiles, where clippy leaves a public method's name alone
// (`avoid-breaking-exported-api`). Here the goldens are private modules of a test
// build, which that exemption does not reach; the clippy gate over the goldens
// themselves (`scripts/check_clippy_codec.sh`) holds them to everything else.
#![allow(clippy::wrong_self_convention)]

use crate::codec::SceCursor;

#[rustfmt::skip]
#[path = "../../../../tests/forge/expected/views/codec_chain_flag_entry.rs"]
mod codec_chain_flag_entry;
#[rustfmt::skip]
#[path = "../../../../tests/forge/expected/views/codec_chain_prev_carrier.rs"]
mod codec_chain_prev_carrier;
#[rustfmt::skip]
#[path = "../../../../tests/forge/expected/views/codec_chain_prev_entry.rs"]
mod codec_chain_prev_entry;
#[rustfmt::skip]
#[path = "../../../../tests/forge/expected/views/codec_chain_prev_gated.rs"]
mod codec_chain_prev_gated;
#[rustfmt::skip]
#[path = "../../../../tests/forge/expected/views/codec_repeat_basic.rs"]
mod codec_repeat_basic;
#[rustfmt::skip]
#[path = "../../../../tests/forge/expected/views/codec_repeat_elem.rs"]
mod codec_repeat_elem;
#[rustfmt::skip]
#[path = "../../../../tests/forge/expected/views/codec_repeat_flag_carrier.rs"]
mod codec_repeat_flag_carrier;
#[rustfmt::skip]
#[path = "../../../../tests/forge/expected/views/codec_tlv_chain_basic.rs"]
mod codec_tlv_chain_basic;
#[rustfmt::skip]
#[path = "../../../../tests/forge/expected/views/codec_tlv_entry.rs"]
mod codec_tlv_entry;
#[rustfmt::skip]
#[path = "../../../../tests/forge/expected/views/codec_until_eof_basic.rs"]
mod codec_until_eof_basic;

use codec_chain_prev_carrier::{CodecChainPrevCarrier, CodecChainPrevCarrierView};
use codec_chain_prev_gated::{CodecChainPrevGated, CodecChainPrevGatedView};
use codec_repeat_basic::{CodecRepeatBasic, CodecRepeatBasicView};
use codec_repeat_flag_carrier::{CodecRepeatFlagCarrier, CodecRepeatFlagCarrierView};
use codec_tlv_chain_basic::{CodecTlvChainBasic, CodecTlvChainBasicView};
use codec_until_eof_basic::{CodecUntilEofBasic, CodecUntilEofBasicView};

/// A small xorshift generator: the frames are a function of the seed alone.
struct Rng(u64);

impl Rng {
    fn draw(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.draw() % n as u64) as usize
    }
}

/// Fill `buf` with a frame of up to forty bytes and return its length. Three
/// distributions, so that the frames a codec accepts are not only the rare ones
/// whose every byte happens to be small: any byte, a byte below eight, and a byte
/// below three (the values flags and short counts are made of).
fn frame(rng: &mut Rng, buf: &mut [u8; 64]) -> usize {
    let len = rng.below(41);
    let mode = rng.below(3);
    for byte in buf[..len].iter_mut() {
        *byte = match mode {
            0 => rng.draw() as u8,
            1 => (rng.draw() % 8) as u8,
            _ => (rng.draw() % 3) as u8,
        };
    }
    len
}

/// Ask the eager decode and the view of `bytes`, and require the same answer: the
/// same error, or the same value, the same bytes consumed and, from the view, a
/// builder equal to the eager value. Returns 1 for a frame both accept.
macro_rules! agree {
    ($eager:ty, $view:ty, $bytes:expr) => {{
        let bytes: &[u8] = $bytes;
        let mut eager_cursor = SceCursor::new(bytes);
        let eager = <$eager>::decode(&mut eager_cursor);
        let mut view_cursor = SceCursor::new(bytes);
        let view = <$view>::decode(&mut view_cursor);
        match (eager, view) {
            (Ok(eager), Ok(view)) => {
                let mut built = <$eager>::new();
                view.into_builder(&mut built)
                    .expect("a view that decoded fills a builder of the same bounds");
                assert_eq!(built, eager, "the view's builder, for the frame {bytes:?}");
                assert_eq!(
                    view_cursor.remaining(),
                    eager_cursor.remaining(),
                    "the bytes left, for the frame {bytes:?}"
                );
                1u32
            }
            (Err(a), Err(b)) => {
                assert_eq!(a, b, "the error, for the frame {bytes:?}");
                0u32
            }
            (eager, view) => panic!(
                "the eager decode and the view disagree on the frame {bytes:?}: eager {} and view {}",
                if eager.is_ok() { "accepts" } else { "refuses" },
                if view.is_ok() { "accepts" } else { "refuses" },
            ),
        }
    }};
}

#[test]
fn a_view_answers_as_the_eager_decode_does_for_every_kind_of_list() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut buf = [0u8; 64];
    // One count of accepted frames for each codec, in the order below.
    let mut accepted = [0u32; 6];
    for _ in 0..40_000 {
        let len = frame(&mut rng, &mut buf);
        let bytes = &buf[..len];
        accepted[0] += agree!(CodecRepeatBasic, CodecRepeatBasicView, bytes);
        accepted[1] += agree!(CodecUntilEofBasic, CodecUntilEofBasicView, bytes);
        accepted[2] += agree!(CodecRepeatFlagCarrier, CodecRepeatFlagCarrierView, bytes);
        accepted[3] += agree!(CodecTlvChainBasic, CodecTlvChainBasicView, bytes);
        accepted[4] += agree!(CodecChainPrevCarrier, CodecChainPrevCarrierView, bytes);
        accepted[5] += agree!(CodecChainPrevGated, CodecChainPrevGatedView, bytes);
    }
    // A comparison that only ever saw refusals would pass for any view at all.
    for (codec, count) in accepted.iter().enumerate() {
        assert!(
            *count >= 25,
            "codec {codec} accepted {count} frames: too few for the comparison to say anything"
        );
    }
}
