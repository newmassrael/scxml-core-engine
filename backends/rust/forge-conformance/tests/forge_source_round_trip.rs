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
///
/// `[$view]` and `[$owned]` are the codec's view and owned form when it has
/// them: a form is another representation of the value, so the frame decodes to
/// it exactly when it decodes to the builder, and `encode_source` writes the
/// same bytes, or answers the same refusal, from each.
macro_rules! agree {
    ($ty:ty, $frame:expr, [$($view:ty)?], [$($owned:ty)?]) => {{
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
                $(
                    let viewed = <$view>::decode(&mut SceCursor::new(frame)).unwrap_or_else(|e| {
                        panic!("the view refuses frame {frame:02x?} that decode accepts: {e:?}")
                    });
                    let mut by_view = Vec::new();
                    let from_view = <$ty>::encode_source(&viewed, &mut VecSink::new(&mut by_view));
                    assert_eq!(
                        from_view, new,
                        "a view answers another verdict for frame {frame:02x?}"
                    );
                    assert_eq!(
                        by_view, by_source,
                        "a view writes other bytes for frame {frame:02x?}"
                    );
                )?
                $(
                    let owned: $owned = value.clone().try_into_owned().unwrap_or_else(|e| {
                        panic!("the owned form refuses frame {frame:02x?}: {e:?}")
                    });
                    let mut by_owned = Vec::new();
                    let from_owned =
                        <$ty>::encode_source(&owned, &mut VecSink::new(&mut by_owned));
                    assert_eq!(
                        from_owned, new,
                        "an owned form answers another verdict for frame {frame:02x?}"
                    );
                    assert_eq!(
                        by_owned, by_source,
                        "an owned form writes other bytes for frame {frame:02x?}"
                    );
                )?
                true
            }
            Err(_) => {
                $(
                    assert!(
                        <$view>::decode(&mut SceCursor::new(frame)).is_err(),
                        "the view accepts frame {frame:02x?} that decode refuses"
                    );
                )?
                false
            }
        }
    }};
}

/// The sweep of one codec: `seeds`, the frame its default value encodes to, and
/// seeded random frames drawn from `pool`; for each the decode accepts, its
/// prefixes and its one-byte changes. Returns how many frames were accepted.
macro_rules! sweep {
    ($ty:ty, $pool:expr, $seeds:expr, [$($view:ty)?], [$($owned:ty)?]) => {{
        let pool: &[u8] = $pool;
        let mut seeds: Vec<Vec<u8>> = $seeds;
        // The frames the document gives are frames of it: each one decodes.
        let given = seeds.len();
        let mut by_default = Vec::new();
        if <$ty>::default()
            .encode(&mut VecSink::new(&mut by_default))
            .is_ok()
        {
            seeds.push(by_default);
        }
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        // Short frames reach the small codecs; longer ones hold a list of
        // several entries.
        let random = (0..500).map(|n| {
            let len = if n < 300 {
                rng.below(17)
            } else {
                rng.below(41)
            };
            (0..len)
                .map(|_| pool[rng.below(pool.len())])
                .collect::<Vec<u8>>()
        });
        let frames: Vec<Vec<u8>> = seeds.into_iter().chain(random).collect();
        let mut accepted = 0usize;
        for (index, frame) in frames.iter().enumerate() {
            let decoded = agree!($ty, frame, [$($view)?], [$($owned)?]);
            assert!(
                decoded || index >= given,
                "a seed frame is not a frame of the codec: {frame:02x?}"
            );
            if decoded {
                accepted += 1;
                for cut in 0..frame.len() {
                    agree!($ty, &frame[..cut], [$($view)?], [$($owned)?]);
                }
                for at in 0..frame.len() {
                    for &b in pool {
                        let mut changed = frame.clone();
                        changed[at] = b;
                        agree!($ty, &changed, [$($view)?], [$($owned)?]);
                    }
                }
            }
        }
        accepted
    }};
}

/// One generated test: the codec `$name` as the module `build.rs` included it.
macro_rules! sweep_codec {
    ($name:ident, $ty:ty, $pool:expr, $seeds:expr, [$($view:ty)?], [$($owned:ty)?]) => {
        #[test]
        #[allow(non_snake_case)]
        fn $name() {
            let accepted = sweep!($ty, $pool, $seeds, [$($view)?], [$($owned)?]);
            assert!(
                accepted > 0,
                "no frame of {} was accepted, so nothing was compared",
                stringify!($name)
            );
        }
    };
}

include!(concat!(env!("OUT_DIR"), "/source_sweeps.rs"));

// A value that is not the codec's builder says what it holds through the same
// interface, and is encoded by the same function. This is the point of the
// interface: the sender keeps its entries in its own type and no builder is
// filled to send them.
mod foreign {
    use super::codecs::codec_repeat_basic::{CodecRepeatBasic, CodecRepeatBasicSource};
    use super::codecs::codec_repeat_elem::CodecRepeatElemSource;
    use sce_forge_runtime::codec::{CodecError, VecSink};

    struct Seq(u16);

    impl CodecRepeatElemSource for Seq {
        fn seq(&self) -> u16 {
            self.0
        }
    }

    struct Fragments(Vec<u16>);

    impl CodecRepeatBasicSource for Fragments {
        fn frags(&self) -> impl Iterator<Item = impl CodecRepeatElemSource> + Clone {
            self.0.iter().map(|&s| Seq(s))
        }
    }

    #[test]
    fn a_foreign_source_is_encoded_with_its_count_derived() {
        let mut bytes = Vec::new();
        CodecRepeatBasic::encode_source(&Fragments(vec![1, 2, 3]), &mut VecSink::new(&mut bytes))
            .unwrap();
        assert_eq!(bytes, [3, 0, 1, 0, 2, 0, 3]);
    }

    #[test]
    fn a_list_past_its_bound_is_refused_before_the_first_byte() {
        let mut bytes = Vec::new();
        let refused =
            CodecRepeatBasic::encode_source(&Fragments(vec![0; 33]), &mut VecSink::new(&mut bytes));
        assert_eq!(refused, Err(CodecError::TooManyElements));
        assert!(bytes.is_empty(), "the sink was written to: {bytes:02x?}");
    }
}
