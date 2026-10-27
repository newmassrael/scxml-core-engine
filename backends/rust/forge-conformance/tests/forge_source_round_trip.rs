// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The source interface of a codec (docs/adr/0058): `{Codec}Source`, and
// `{Codec}::encode_source`, the one encode that reads it.
//
// What is asserted is law 2 of the ADR, conservation: for every value a decode
// produces, `encode_source` writes the bytes `encode` writes, or refuses as it
// does. A decoded value is the case the two must agree on, because the count, the
// length or the flag `encode` writes from what the struct holds is, in a decoded
// value, what the content says, and `encode_source` computes it from the content.
//
// The frames are not hand-picked and neither is the list of codecs. `build.rs`
// generates one sweep for each codec of the catalog whose generated file holds a
// source interface; each sweep decodes the frames its document holds as test
// vectors, the frame its default value encodes to, and seeded random frames, and
// for each frame the eager decode accepts, every prefix of it and every one-byte
// change of it, so the values a damaged frame still decodes to are compared too.
//
// Gated on `alloc` like the other round-trip tests.
#![cfg(feature = "alloc")]

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

/// Decode `frame` and, when it decodes, encode the value both ways and assert
/// that they agree. Returns whether the frame decoded.
macro_rules! agree {
    ($ty:ty, $frame:expr) => {{
        let frame: &[u8] = $frame;
        match <$ty>::decode(&mut SceCursor::new(frame)) {
            Ok(value) => {
                let mut by_encode = Vec::new();
                let old = value.encode(&mut VecSink::new(&mut by_encode));
                let mut by_source = Vec::new();
                let new = <$ty>::encode_source(&value, &mut VecSink::new(&mut by_source));
                assert_eq!(
                    new, old,
                    "encode_source answers another verdict for frame {frame:02x?}"
                );
                if by_source != by_encode {
                    // The frame was not canonical: it says a number or a flag the
                    // content does not (a length beside an absent field, a gate
                    // flag against the field it gates). `encode_source` writes
                    // what the content says, so its bytes must be a frame that
                    // `encode` reproduces from the value they decode to.
                    assert!(
                        old.is_ok(),
                        "encode_source writes bytes where encode refuses, frame {frame:02x?}"
                    );
                    let again = <$ty>::decode(&mut SceCursor::new(&by_source)).unwrap_or_else(
                        |e| panic!("encode_source writes undecodable bytes for frame {frame:02x?}: {e:?}"),
                    );
                    let mut by_again = Vec::new();
                    again
                        .encode(&mut VecSink::new(&mut by_again))
                        .unwrap_or_else(|e| {
                            panic!("encode refuses the canonical form of frame {frame:02x?}: {e:?}")
                        });
                    assert_eq!(
                        by_again, by_source,
                        "encode_source writes a form encode does not keep for frame {frame:02x?}"
                    );
                }
                true
            }
            Err(_) => false,
        }
    }};
}

/// The sweep of one codec: `seeds`, the frame its default value encodes to, and
/// seeded random frames drawn from `pool`; for each the decode accepts, its
/// prefixes and its one-byte changes. Returns how many frames were accepted.
macro_rules! sweep {
    ($ty:ty, $pool:expr, $seeds:expr) => {{
        let pool: &[u8] = $pool;
        let mut seeds: Vec<Vec<u8>> = $seeds;
        let mut by_default = Vec::new();
        if <$ty>::default()
            .encode(&mut VecSink::new(&mut by_default))
            .is_ok()
        {
            seeds.push(by_default);
        }
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        let random = (0..300).map(|_| {
            let len = rng.below(17);
            (0..len)
                .map(|_| pool[rng.below(pool.len())])
                .collect::<Vec<u8>>()
        });
        let frames: Vec<Vec<u8>> = seeds.into_iter().chain(random).collect();
        let mut accepted = 0usize;
        for frame in &frames {
            if agree!($ty, frame) {
                accepted += 1;
                for cut in 0..frame.len() {
                    agree!($ty, &frame[..cut]);
                }
                for at in 0..frame.len() {
                    for &b in pool {
                        let mut changed = frame.clone();
                        changed[at] = b;
                        agree!($ty, &changed);
                    }
                }
            }
        }
        accepted
    }};
}

/// One generated test: the codec `$name` as the module `build.rs` included it.
macro_rules! sweep_codec {
    ($name:ident, $ty:ty, $pool:expr, $seeds:expr) => {
        #[test]
        #[allow(non_snake_case)]
        fn $name() {
            let accepted = sweep!($ty, $pool, $seeds);
            assert!(
                accepted > 0,
                "no frame of {} was accepted, so nothing was compared",
                stringify!($name)
            );
        }
    };
}

include!(concat!(env!("OUT_DIR"), "/source_sweeps.rs"));
