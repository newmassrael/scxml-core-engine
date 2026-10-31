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

use sce_forge_runtime::codec::{CodecError, Measured, SceCursor, SourceWrite, VecSink};

/// What an encode answered, as one type whichever codec gave it, so that one
/// sweep holds every codec to the same laws: a codec whose encode is total
/// answers with no error of its own (over a `VecSink` the error type is
/// uninhabited, and its `measure` returns the witness itself), and one that can
/// refuse answers with a `CodecError` (docs/adr/0065).
trait Answer<T> {
    fn answer(self) -> Result<T, CodecError>;
}

impl<T> Answer<T> for Result<T, CodecError> {
    fn answer(self) -> Result<T, CodecError> {
        self
    }
}

impl<T> Answer<T> for Result<T, core::convert::Infallible> {
    fn answer(self) -> Result<T, CodecError> {
        self.map_err(CodecError::from)
    }
}

impl<'s, C: SourceWrite<S>, S: ?Sized> Answer<Measured<'s, C, S>> for Measured<'s, C, S> {
    fn answer(self) -> Result<Measured<'s, C, S>, CodecError> {
        Ok(self)
    }
}

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
                let verdict =
                    <$ty>::encode_source(&value, &mut VecSink::new(&mut by_source)).answer();
                // The method is the function, applied to the value.
                let mut by_method = Vec::new();
                assert_eq!(
                    value.encode(&mut VecSink::new(&mut by_method)).answer(),
                    verdict,
                    "encode answers another verdict than encode_source, frame {frame:02x?}"
                );
                assert_eq!(by_method, by_source, "encode writes other bytes, frame {frame:02x?}");
                // The length found with nothing written is the length the encode
                // writes, the window holds the bytes the encode writes, and a
                // value the encode refuses has no length (docs/adr/0061). The
                // window is filled twice with different bytes, so a byte left
                // unwritten cannot pass for one that matched.
                match <$ty>::measure(&value).answer() {
                    Ok(measured) => {
                        assert!(verdict.is_ok(), "measure admits a value encode refuses, frame {frame:02x?}");
                        assert_eq!(measured.len(), by_source.len(), "measure counts other bytes, frame {frame:02x?}");
                        for fill in [0x00u8, 0xFF] {
                            let mut window = vec![fill; by_source.len()];
                            <$ty>::measure(&value).answer().unwrap().write_into(&mut window);
                            assert_eq!(window, by_source, "the window holds other bytes, frame {frame:02x?}");
                        }
                    }
                    Err(refusal) => assert_eq!(
                        Err(refusal),
                        verdict,
                        "measure answers another refusal than encode, frame {frame:02x?}"
                    ),
                }
                // The one-pass window (docs/adr/0061): the bytes the encode
                // writes and their count in a window that is large enough, the
                // refusal the encode gives, and an overflow in a window a byte
                // short. It leaves a prefix on an error, so nothing is claimed
                // of what it holds then.
                let mut roomy = vec![0xA5u8; by_source.len() + 8];
                let windowed = <$ty>::write_window(&value, &mut roomy);
                assert_eq!(
                    windowed.map(|_| ()),
                    verdict,
                    "write_window answers another verdict, frame {frame:02x?}"
                );
                if verdict.is_ok() {
                    assert_eq!(windowed, Ok(by_source.len()), "write_window counts other bytes, frame {frame:02x?}");
                    assert_eq!(&roomy[..by_source.len()], &by_source[..], "write_window holds other bytes, frame {frame:02x?}");
                    if !by_source.is_empty() {
                        let mut tight = vec![0u8; by_source.len() - 1];
                        assert_eq!(
                            <$ty>::write_window(&value, &mut tight),
                            Err(::sce_forge_runtime::codec::CodecError::BufferOverflow),
                            "write_window fits a window a byte short, frame {frame:02x?}"
                        );
                    }
                }
                if verdict.is_ok() {
                    // The bytes are a frame, and a fixed point of the encode.
                    let again = <$ty>::decode(&mut SceCursor::new(&by_source)).unwrap_or_else(|e| {
                        panic!("the encode writes undecodable bytes for frame {frame:02x?}: {e:?}")
                    });
                    let mut by_again = Vec::new();
                    <$ty>::encode_source(&again, &mut VecSink::new(&mut by_again))
                        .answer()
                        .unwrap_or_else(|e| {
                            panic!("the encode refuses its own frame, from frame {frame:02x?}: {e:?}")
                        });
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
                    let from_view =
                        <$ty>::encode_source(&viewed, &mut VecSink::new(&mut by_view)).answer();
                    assert_eq!(
                        from_view, verdict,
                        "a view answers another verdict for frame {frame:02x?}"
                    );
                    assert_eq!(
                        by_view, by_source,
                        "a view writes other bytes for frame {frame:02x?}"
                    );
                    assert_eq!(
                        <$ty>::measure(&viewed).answer().map(|m| m.len()),
                        <$ty>::measure(&value).answer().map(|m| m.len()),
                        "a view is another length for frame {frame:02x?}"
                    );
                )?
                $(
                    let owned: $owned = value.clone().try_into_owned().unwrap_or_else(|e| {
                        panic!("the owned form refuses frame {frame:02x?}: {e:?}")
                    });
                    let mut by_owned = Vec::new();
                    let from_owned =
                        <$ty>::encode_source(&owned, &mut VecSink::new(&mut by_owned)).answer();
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
            .answer()
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
    use sce_forge_runtime::codec::{Bounded, BoundedIter, VecSink};

    struct Seq(u16);

    impl CodecRepeatElemSource for Seq {
        fn seq(&self) -> u16 {
            self.0
        }
    }

    /// The sender's own list, held within the codec's bound of 32 entries.
    struct Fragments(Bounded<Vec<u16>, 32>);

    impl CodecRepeatBasicSource for Fragments {
        fn frags(
            &self,
        ) -> BoundedIter<impl Iterator<Item = impl CodecRepeatElemSource> + Clone, 32> {
            self.0.iter().map(|&s| Seq(s))
        }
    }

    #[test]
    fn a_foreign_source_is_encoded_with_its_count_derived() {
        let frags = Bounded::new(vec![1, 2, 3]).expect("within 32");
        let mut bytes = Vec::new();
        CodecRepeatBasic::encode_source(&Fragments(frags), &mut VecSink::new(&mut bytes)).unwrap();
        assert_eq!(bytes, [3, 0, 1, 0, 2, 0, 3]);
    }

    // The list past the bound has no source at all, so the encode, which used to
    // refuse it, has nothing to refuse (docs/adr/0065 §6 R2).
    #[test]
    fn a_list_past_its_bound_has_no_source() {
        assert!(Bounded::<Vec<u16>, 32>::new(vec![0; 32]).is_some());
        assert!(Bounded::<Vec<u16>, 32>::new(vec![0; 33]).is_none());
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
        CodecVariantDispatchUnknownTag,
    };
    use super::codecs::codec_variant_session_close::CodecVariantSessionCloseParts;
    use super::codecs::codec_variant_session_open::CodecVariantSessionOpenParts;
    use super::codecs::codec_zenoh_locator::CodecZenohLocatorParts;
    use sce_forge_runtime::codec::{Bounded, SceCursor, VecSink};

    #[test]
    fn a_list_held_as_a_slice_is_written_with_its_count_derived() {
        let entries = [
            CodecRepeatElemParts { seq: 1 },
            CodecRepeatElemParts { seq: 2 },
            CodecRepeatElemParts { seq: 3 },
        ];
        let mut bytes = Vec::new();
        CodecRepeatBasic::encode_source(
            &CodecRepeatBasicParts {
                frags: (&entries).into(),
            },
            &mut VecSink::new(&mut bytes),
        )
        .unwrap();
        assert_eq!(bytes, [3, 0, 1, 0, 2, 0, 3]);
    }

    // A slice past the bound is not a list the parts can hold; an array within it
    // converts with no check, and a slice is checked once (docs/adr/0065 §6 R2).
    #[test]
    fn a_slice_past_the_bound_is_no_list_of_the_parts() {
        let entries = [CodecRepeatElemParts { seq: 0 }; 33];
        assert!(Bounded::<&[CodecRepeatElemParts], 32>::new(&entries[..32]).is_some());
        assert!(Bounded::<&[CodecRepeatElemParts], 32>::new(&entries[..]).is_none());
    }

    // The window encoder (docs/adr/0061) on the types a caller already has and no
    // heap: count, reserve exactly that, write.
    #[test]
    fn a_slice_is_counted_then_written_into_a_window_of_exactly_that_size() {
        let entries = [
            CodecRepeatElemParts { seq: 1 },
            CodecRepeatElemParts { seq: 2 },
            CodecRepeatElemParts { seq: 3 },
        ];
        let source = CodecRepeatBasicParts {
            frags: (&entries).into(),
        };
        let measured = CodecRepeatBasic::measure(&source);
        assert_eq!(measured.len(), 7);
        let mut slot = [0xEEu8; 9];
        measured.write_into(&mut slot[1..8]);
        assert_eq!(slot, [0xEE, 3, 0, 1, 0, 2, 0, 3, 0xEE]);
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
                tag: CodecVariantDispatchUnknownTag::new(9).expect("no arm names 9"),
                body: &close
            }),
            [9, 4]
        );
        // It holds no tag an arm names, which a decode would read back as that
        // arm (docs/adr/0065 §6 R3).
        assert_eq!(CodecVariantDispatchUnknownTag::new(1), None);
        assert_eq!(CodecVariantDispatchUnknownTag::new(2), None);
    }

    // A value's tag is read from the arm it holds, through the interface, so each
    // form answers from the table the encode writes from, and a decoded frame
    // gives back the tag it was read with.
    #[test]
    fn a_value_says_the_tag_of_the_arm_it_holds() {
        let open = CodecVariantSessionOpenParts { version: 7 };
        let close = CodecVariantSessionCloseParts { reason: 4 };
        let tag_of = |body| CodecVariantDispatch::tag_of(&CodecVariantDispatchParts { body });
        assert_eq!(
            tag_of(CodecVariantDispatchPartsBody::CodecVariantSessionOpen(
                &open
            )),
            1
        );
        assert_eq!(
            tag_of(CodecVariantDispatchPartsBody::CodecVariantSessionClose(
                &close
            )),
            2
        );
        assert_eq!(
            tag_of(CodecVariantDispatchPartsBody::Default {
                tag: CodecVariantDispatchUnknownTag::new(9).expect("no arm names 9"),
                body: &close
            }),
            9
        );
        for frame in [[1u8, 0, 7].as_slice(), &[2, 4], &[9, 4]] {
            let decoded =
                CodecVariantDispatch::decode(&mut SceCursor::new(frame)).expect("a frame");
            assert_eq!(CodecVariantDispatch::tag_of(&decoded), frame[0]);
        }
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

// A default arm's tag as a value of its tag space (docs/adr/0065 §6 R3): the set
// `new` admits is the set the runtime is told, and two codecs whose arms name the
// same tags under the same bits convert their tags with nothing to check.
// A flag input the value says (docs/adr/0065 §6 R4a): the codec reads it off the
// value, a parent derives the flag it binds to the input from the body, and the
// refusal stays only where a value handed from elsewhere can disagree.
mod said_inputs {
    use super::codecs::codec_chain_prev_carrier::{
        CodecChainPrevCarrier, CodecChainPrevCarrierParts,
    };
    use super::codecs::codec_chain_prev_entry::CodecChainPrevEntryParts;
    use super::codecs::codec_zenoh_decl_token::{CodecZenohDeclToken, CodecZenohDeclTokenParts};
    use super::codecs::codec_zenoh_interest_body::{
        CodecZenohInterestBody, CodecZenohInterestBodyParts,
    };
    use super::codecs::codec_zenoh_open_body::{CodecZenohOpenBody, CodecZenohOpenBodyParts};
    use super::codecs::codec_zenoh_wireexpr::{CodecZenohWireexpr, CodecZenohWireexprParts};
    use sce_forge_runtime::codec::{CodecError, VecSink};

    fn wireexpr(suffix: Option<&str>) -> CodecZenohWireexprParts<'_> {
        CodecZenohWireexprParts { id: 7, suffix }
    }

    #[test]
    fn an_input_is_set_exactly_when_the_one_field_it_gates_is_there() {
        assert!(CodecZenohWireexpr::input_n(&wireexpr(Some("a"))));
        assert!(!CodecZenohWireexpr::input_n(&wireexpr(None)));
        // `open_body` gates its cookie on `!A`: the input is set where it is absent.
        let open = |cookie| CodecZenohOpenBodyParts {
            lease: 1,
            initial_sn: 2,
            cookie,
        };
        assert!(CodecZenohOpenBody::input_a(&open(None)));
        assert!(!CodecZenohOpenBody::input_a(&open(Some(&[1u8][..]))));
    }

    #[test]
    fn an_input_handed_on_to_a_body_is_the_input_the_body_says() {
        for suffix in [Some("a"), None] {
            let body = wireexpr(suffix);
            assert_eq!(
                CodecZenohDeclToken::input_n(&CodecZenohDeclTokenParts {
                    id: 1,
                    wireexpr: &body,
                }),
                suffix.is_some()
            );
        }
    }

    #[test]
    fn a_flag_bound_to_a_body_is_written_as_the_body_says_it() {
        let encode = |parts: &CodecZenohInterestBodyParts<'_>| {
            let mut bytes = Vec::new();
            let Ok(()) =
                CodecZenohInterestBody::encode_source(parts, &mut VecSink::new(&mut bytes));
            bytes
        };
        // The value presents `N` clear; the body has a suffix, so the frame sets it.
        let with = wireexpr(Some("a"));
        assert_eq!(
            encode(&CodecZenohInterestBodyParts {
                header: 0x00,
                keyexpr: Some(&with),
            }),
            [0x30, 0x07, 0x01, b'a']
        );
        // The value presents `N` set; the body has none, so the frame clears it.
        let without = wireexpr(None);
        assert_eq!(
            encode(&CodecZenohInterestBodyParts {
                header: 0x20,
                keyexpr: Some(&without),
            }),
            [0x10, 0x07]
        );
        // No body says it: the frame keeps the bit the value presents.
        assert_eq!(
            encode(&CodecZenohInterestBodyParts {
                header: 0x20,
                keyexpr: None,
            }),
            [0x20]
        );
    }

    #[test]
    fn an_entry_that_says_its_input_otherwise_than_its_chain_is_refused_before_a_byte() {
        let entries = [CodecChainPrevEntryParts {
            header: 0x01,
            short_value: Some(5),
            long_value: None,
            tag: Some(9),
        }];
        let carrier = |head| CodecChainPrevCarrierParts {
            head,
            entries: (&entries).into(),
        };
        let mut bytes = Vec::new();
        assert_eq!(
            CodecChainPrevCarrier::encode_source(&carrier(0x00), &mut VecSink::new(&mut bytes)),
            Err(CodecError::PresentIfMismatch)
        );
        assert!(bytes.is_empty(), "a refusal leaves the sink untouched");
        CodecChainPrevCarrier::encode_source(&carrier(0x01), &mut VecSink::new(&mut bytes))
            .expect("the entry says the input the chain hands it");
        assert_eq!(bytes, [0x01, 0x01, 5, 9]);
    }
}

mod unknown_tags {
    use super::codecs::codec_init_syn_envelope::CodecInitSynEnvelopeUnknownTag;
    use super::codecs::codec_origin_choice::CodecOriginChoiceUnknownTag;
    use super::codecs::codec_transport_envelope::CodecTransportEnvelopeUnknownTag;
    use super::codecs::codec_variant_dispatch::CodecVariantDispatchUnknownTag;
    use super::codecs::codec_zenoh_declaration::CodecZenohDeclarationUnknownTag;
    use super::codecs::codec_zenoh_ext_entry::CodecZenohExtEntryUnknownTag;
    use super::codecs::codec_zenoh_network_envelope::CodecZenohNetworkEnvelopeUnknownTag;
    use super::codecs::codec_zenoh_oam::CodecZenohOamUnknownTag;
    use super::codecs::codec_zenoh_push::CodecZenohPushUnknownTag;
    use super::codecs::codec_zenoh_push_body::CodecZenohPushBodyUnknownTag;
    use super::codecs::codec_zenoh_request::CodecZenohRequestUnknownTag;
    use super::codecs::codec_zenoh_response::CodecZenohResponseUnknownTag;
    use super::codecs::codec_zenoh_value_entry::CodecZenohValueEntryUnknownTag;
    use sce_forge_runtime::codec::UnknownTag;

    macro_rules! assert_space_is_what_new_admits {
        ($($tag:ty),+ $(,)?) => {$(
            for t in 0..=u8::MAX {
                assert_eq!(
                    <$tag>::new(t).is_some(),
                    <$tag as UnknownTag>::SPACE.admits(u64::from(t)),
                    "{} at {t}",
                    stringify!($tag)
                );
            }
        )+};
    }

    #[test]
    fn the_space_each_codec_states_is_the_set_its_constructor_admits() {
        assert_space_is_what_new_admits!(
            CodecInitSynEnvelopeUnknownTag,
            CodecOriginChoiceUnknownTag,
            CodecTransportEnvelopeUnknownTag,
            CodecVariantDispatchUnknownTag,
            CodecZenohDeclarationUnknownTag,
            CodecZenohExtEntryUnknownTag,
            CodecZenohNetworkEnvelopeUnknownTag,
            CodecZenohOamUnknownTag,
            CodecZenohPushUnknownTag,
            CodecZenohPushBodyUnknownTag,
            CodecZenohRequestUnknownTag,
            CodecZenohResponseUnknownTag,
            CodecZenohValueEntryUnknownTag,
        );
    }

    #[test]
    fn codecs_whose_arms_name_the_same_tags_convert_their_unknown_tag_both_ways() {
        // An extension entry and a value entry dispatch on the same two bits and
        // name arms 0, 1 and 2; their arms hold different bodies. Each holds 3 alone.
        let ext = CodecZenohExtEntryUnknownTag::new(3).expect("no arm names 3");
        let value = CodecZenohValueEntryUnknownTag::from_unknown(ext);
        assert_eq!(value.get(), 3);
        assert_eq!(CodecZenohExtEntryUnknownTag::from_unknown(value), ext);
        assert_eq!(CodecZenohOamUnknownTag::from_unknown(ext).get(), 3);
    }
}
