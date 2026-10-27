// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The source interface of a codec (docs/adr/0058): `{Codec}Source`, and
// `{Codec}::encode_source`, the one encode that reads it. A codec's `encode` is
// that function, applied to itself.
//
// What is asserted is what the encode owes, for every value a decode produces:
//
// 1. A refusal is made before the first byte: the sink an encode refused is
//    empty.
// 2. The bytes it writes are a frame: they decode, and the value they decode to
//    encodes to the same bytes (a decoded frame that said a number or a flag
//    the content does not is written as what the content says).
// 3. Every form of the value says the same: the builder, its view and its owned
//    form write the same bytes or answer the same refusal.
// 4. A frame the document gives (a test vector, or the wire it describes) is
//    reproduced byte for byte: what the encode derives from the content is what
//    a conforming sender writes.
//
// The frames are not hand-picked and neither is the list of codecs. `build.rs`
// generates one sweep for each codec of the catalog, from the frames its document
// holds as test vectors, the frame its default value encodes to, and seeded random
// frames; for each frame the eager decode accepts, every prefix of it and every
// one-byte change of it, so the values a damaged frame still decodes to are
// checked too.
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

/// Decode `frame` and, when it decodes, hold its encode to the laws above.
/// Returns whether the frame decoded. `$canonical` is whether the frame is one a
/// document gives, which the encode must reproduce.
///
/// `[$view]` and `[$owned]` are the codec's view and owned form when it has
/// them: a form is another representation of the value, so the frame decodes to
/// it exactly when it decodes to the builder, and the encode of it is the
/// builder's.
macro_rules! agree {
    ($ty:ty, $frame:expr, $canonical:expr, [$($view:ty)?], [$($owned:ty)?]) => {{
        let frame: &[u8] = $frame;
        match <$ty>::decode(&mut SceCursor::new(frame)) {
            Ok(value) => {
                let mut by_source = Vec::new();
                let verdict = <$ty>::encode_source(&value, &mut VecSink::new(&mut by_source));
                // The method is the function, applied to the value.
                let mut by_method = Vec::new();
                assert_eq!(
                    value.encode(&mut VecSink::new(&mut by_method)),
                    verdict,
                    "encode answers another verdict than encode_source, frame {frame:02x?}"
                );
                assert_eq!(by_method, by_source, "encode writes other bytes, frame {frame:02x?}");
                if verdict.is_ok() {
                    // The bytes are a frame, and a fixed point of the encode.
                    let again = <$ty>::decode(&mut SceCursor::new(&by_source)).unwrap_or_else(|e| {
                        panic!("the encode writes undecodable bytes for frame {frame:02x?}: {e:?}")
                    });
                    let mut by_again = Vec::new();
                    <$ty>::encode_source(&again, &mut VecSink::new(&mut by_again)).unwrap_or_else(
                        |e| panic!("the encode refuses its own frame, from frame {frame:02x?}: {e:?}"),
                    );
                    assert_eq!(
                        by_again, by_source,
                        "the encode does not keep its own frame, from frame {frame:02x?}"
                    );
                    if $canonical {
                        assert_eq!(
                            by_source, frame,
                            "the encode does not reproduce a frame its document gives"
                        );
                    }
                } else {
                    assert!(
                        by_source.is_empty(),
                        "a refused encode wrote {by_source:02x?} for frame {frame:02x?}"
                    );
                }
                $(
                    let viewed = <$view>::decode(&mut SceCursor::new(frame)).unwrap_or_else(|e| {
                        panic!("the view refuses frame {frame:02x?} that decode accepts: {e:?}")
                    });
                    let mut by_view = Vec::new();
                    let from_view = <$ty>::encode_source(&viewed, &mut VecSink::new(&mut by_view));
                    assert_eq!(
                        from_view, verdict,
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
                        from_owned, verdict,
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
        // The frames the document gives are frames of it: each one decodes, and
        // is the frame the encode writes for what it says.
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
            let decoded = agree!($ty, frame, index < given, [$($view)?], [$($owned)?]);
            assert!(
                decoded || index >= given,
                "a seed frame is not a frame of the codec: {frame:02x?}"
            );
            if decoded {
                accepted += 1;
                for cut in 0..frame.len() {
                    agree!($ty, &frame[..cut], false, [$($view)?], [$($owned)?]);
                }
                for at in 0..frame.len() {
                    for &b in pool {
                        let mut changed = frame.clone();
                        changed[at] = b;
                        agree!($ty, &changed, false, [$($view)?], [$($owned)?]);
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

// The ready-made source (ADR 0058 §4): `{Codec}Parts` holds what a value says in
// the types a caller already has, lists as slices and bodies as references, and
// is encoded by the same function as every other source.
mod parts {
    use super::codecs::codec_embed_basic::{CodecEmbedBasic, CodecEmbedBasicParts};
    use super::codecs::codec_repeat_basic::{CodecRepeatBasic, CodecRepeatBasicParts};
    use super::codecs::codec_repeat_elem::CodecRepeatElemParts;
    use super::codecs::codec_variant_dispatch::{
        CodecVariantDispatch, CodecVariantDispatchParts, CodecVariantDispatchPartsBody,
    };
    use super::codecs::codec_variant_session_close::CodecVariantSessionCloseParts;
    use super::codecs::codec_variant_session_open::CodecVariantSessionOpenParts;
    use super::codecs::codec_zenoh_locator::CodecZenohLocatorParts;
    use sce_forge_runtime::codec::{CodecError, SceCursor, VecSink};

    #[test]
    fn a_list_held_as_a_slice_is_written_with_its_count_derived() {
        let entries = [
            CodecRepeatElemParts { seq: 1 },
            CodecRepeatElemParts { seq: 2 },
            CodecRepeatElemParts { seq: 3 },
        ];
        let mut bytes = Vec::new();
        CodecRepeatBasic::encode_source(
            &CodecRepeatBasicParts { frags: &entries },
            &mut VecSink::new(&mut bytes),
        )
        .unwrap();
        assert_eq!(bytes, [3, 0, 1, 0, 2, 0, 3]);
    }

    #[test]
    fn a_slice_past_the_bound_is_refused_before_the_first_byte() {
        let entries = [CodecRepeatElemParts { seq: 0 }; 33];
        let mut bytes = Vec::new();
        let refused = CodecRepeatBasic::encode_source(
            &CodecRepeatBasicParts { frags: &entries },
            &mut VecSink::new(&mut bytes),
        );
        assert_eq!(refused, Err(CodecError::TooManyElements));
        assert!(bytes.is_empty(), "the sink was written to: {bytes:02x?}");
    }

    #[test]
    fn a_variant_arm_is_written_with_its_tag_derived() {
        let open = CodecVariantSessionOpenParts { version: 7 };
        let close = CodecVariantSessionCloseParts { reason: 4 };
        let write = |body| {
            let mut bytes = Vec::new();
            CodecVariantDispatch::encode_source(
                &CodecVariantDispatchParts { body },
                &mut VecSink::new(&mut bytes),
            )
            .unwrap();
            bytes
        };
        assert_eq!(
            write(CodecVariantDispatchPartsBody::CodecVariantSessionOpen(
                &open
            )),
            [1, 0, 7]
        );
        assert_eq!(
            write(CodecVariantDispatchPartsBody::CodecVariantSessionClose(
                &close
            )),
            [2, 4]
        );
        // The default arm keeps the tag the grammar could not name.
        assert_eq!(
            write(CodecVariantDispatchPartsBody::Default {
                tag: 9,
                body: &close
            }),
            [9, 4]
        );
    }

    #[test]
    fn an_embedded_body_is_written_and_read_back() {
        let locator = CodecZenohLocatorParts { locator: "tcp/a:1" };
        let source = CodecEmbedBasicParts {
            tag: 5,
            locator: &locator,
        };
        let mut bytes = Vec::new();
        CodecEmbedBasic::encode_source(&source, &mut VecSink::new(&mut bytes)).unwrap();
        let decoded = CodecEmbedBasic::decode(&mut SceCursor::new(&bytes)).unwrap();
        assert_eq!(decoded.tag, 5);
        let mut again = Vec::new();
        decoded.encode(&mut VecSink::new(&mut again)).unwrap();
        assert_eq!(again, bytes);
    }
}
