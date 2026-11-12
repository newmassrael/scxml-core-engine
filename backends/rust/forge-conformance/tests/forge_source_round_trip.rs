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
/// `$wire` is the type the codec's wire is written through, `{Codec}Wire`.
/// `[$view]` and `[$owned]` are the codec's view and owned form when it has
/// them: a form is another representation of the value, so the frame decodes to
/// it exactly when it decodes to the builder, and the encode of it is the
/// builder's.
macro_rules! agree {
    ($ty:ty, $wire:ty, $frame:expr, $canonical:expr, [$($view:ty)?], [$($owned:ty)?]) => {{
        let frame: &[u8] = $frame;
        match <$ty>::decode(&mut SceCursor::new(frame)) {
            Ok(value) => {
                let mut by_source = Vec::new();
                let verdict =
                    <$wire>::encode_source(&value, &mut VecSink::new(&mut by_source)).answer();
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
                match <$wire>::measure(&value).answer() {
                    Ok(measured) => {
                        assert!(verdict.is_ok(), "measure admits a value encode refuses, frame {frame:02x?}");
                        assert_eq!(measured.len(), by_source.len(), "measure counts other bytes, frame {frame:02x?}");
                        for fill in [0x00u8, 0xFF] {
                            let mut window = vec![fill; by_source.len()];
                            <$wire>::measure(&value).answer().unwrap().write_into(&mut window);
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
                let windowed = <$wire>::write_window(&value, &mut roomy);
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
                            <$wire>::write_window(&value, &mut tight),
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
                    <$wire>::encode_source(&again, &mut VecSink::new(&mut by_again))
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
                        <$wire>::encode_source(&viewed, &mut VecSink::new(&mut by_view)).answer();
                    assert_eq!(
                        from_view, verdict,
                        "a view answers another verdict for frame {frame:02x?}"
                    );
                    assert_eq!(
                        by_view, by_source,
                        "a view writes other bytes for frame {frame:02x?}"
                    );
                    assert_eq!(
                        <$wire>::measure(&viewed).answer().map(|m| m.len()),
                        <$wire>::measure(&value).answer().map(|m| m.len()),
                        "a view is another length for frame {frame:02x?}"
                    );
                )?
                $(
                    let owned: $owned = value.clone().try_into_owned().unwrap_or_else(|e| {
                        panic!("the owned form refuses frame {frame:02x?}: {e:?}")
                    });
                    let mut by_owned = Vec::new();
                    let from_owned =
                        <$wire>::encode_source(&owned, &mut VecSink::new(&mut by_owned)).answer();
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
    ($ty:ty, $wire:ty, $pool:expr, $seeds:expr, [$($view:ty)?], [$($owned:ty)?]) => {{
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
            let decoded = agree!($ty, $wire, frame, index < given, [$($view)?], [$($owned)?]);
            assert!(
                decoded || index >= given,
                "a seed frame is not a frame of the codec: {frame:02x?}"
            );
            if decoded {
                accepted += 1;
                for cut in 0..frame.len() {
                    agree!($ty, $wire, &frame[..cut], false, [$($view)?], [$($owned)?]);
                }
                for at in 0..frame.len() {
                    for &b in pool {
                        let mut changed = frame.clone();
                        changed[at] = b;
                        agree!($ty, $wire, &changed, false, [$($view)?], [$($owned)?]);
                    }
                }
            }
        }
        accepted
    }};
}

/// One generated test: the codec `$name` as the module `build.rs` included it.
macro_rules! sweep_codec {
    ($name:ident, $ty:ty, $wire:ty, $pool:expr, $seeds:expr, [$($view:ty)?], [$($owned:ty)?]) => {
        #[test]
        #[allow(non_snake_case)]
        fn $name() {
            let accepted = sweep!($ty, $wire, $pool, $seeds, [$($view)?], [$($owned)?]);
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
    use super::codecs::codec_repeat_basic::{CodecRepeatBasicSource, CodecRepeatBasicWire};
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
        CodecRepeatBasicWire::encode_source(&Fragments(frags), &mut VecSink::new(&mut bytes))
            .unwrap();
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
    use super::codecs::codec_embed_basic::{
        CodecEmbedBasic, CodecEmbedBasicParts, CodecEmbedBasicWire,
    };
    use super::codecs::codec_repeat_basic::{CodecRepeatBasicParts, CodecRepeatBasicWire};
    use super::codecs::codec_repeat_elem::CodecRepeatElemParts;
    use super::codecs::codec_variant_dispatch::{
        CodecVariantDispatch, CodecVariantDispatchParts, CodecVariantDispatchPartsBody,
        CodecVariantDispatchUnknownTag, CodecVariantDispatchWire,
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
        CodecRepeatBasicWire::encode_source(
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
        let measured = CodecRepeatBasicWire::measure(&source);
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
            CodecVariantDispatchWire::encode_source(
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
        let tag_of = |body| CodecVariantDispatchWire::tag_of(&CodecVariantDispatchParts { body });
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
            assert_eq!(CodecVariantDispatchWire::tag_of(&decoded), frame[0]);
        }
    }

    #[test]
    fn an_embedded_body_is_written_and_read_back() {
        let locator = CodecZenohLocatorParts {
            locator: sce_forge_runtime::codec::Bounded::new("tcp/a:1")
                .expect("within the locator's bound"),
        };
        let source = CodecEmbedBasicParts {
            tag: 5,
            locator: &locator,
        };
        let mut bytes = Vec::new();
        CodecEmbedBasicWire::encode_source(&source, &mut VecSink::new(&mut bytes)).unwrap();
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
        CodecChainPrevCarrierEntriesLaw, CodecChainPrevCarrierParts, CodecChainPrevCarrierWire,
    };
    use super::codecs::codec_chain_prev_entry::{
        CodecChainPrevEntryAfterMarker, CodecChainPrevEntryParts,
    };
    use super::codecs::codec_zenoh_decl_token::{
        CodecZenohDeclTokenParts, CodecZenohDeclTokenWire,
    };
    use super::codecs::codec_zenoh_interest_body::{
        CodecZenohInterestBodyParts, CodecZenohInterestBodyWire,
    };
    use super::codecs::codec_zenoh_open_body::{CodecZenohOpenBodyParts, CodecZenohOpenBodyWire};
    use super::codecs::codec_zenoh_wireexpr::{CodecZenohWireexprParts, CodecZenohWireexprWire};
    use sce_forge_runtime::codec::{Bounded, Lawful, VecSink};

    fn wireexpr(suffix: Option<&str>) -> CodecZenohWireexprParts<'_> {
        CodecZenohWireexprParts {
            id: 7,
            suffix: suffix.map(|s| Bounded::new(s).expect("within the suffix's bound")),
        }
    }

    #[test]
    fn an_input_is_set_exactly_when_the_one_field_it_gates_is_there() {
        assert!(CodecZenohWireexprWire::input_n(&wireexpr(Some("a"))));
        assert!(!CodecZenohWireexprWire::input_n(&wireexpr(None)));
        // `open_body` gates its cookie on `!A`: the input is set where it is absent.
        let open = |cookie| CodecZenohOpenBodyParts {
            lease: 1,
            initial_sn: 2,
            cookie,
        };
        assert!(CodecZenohOpenBodyWire::input_a(&open(None)));
        assert!(!CodecZenohOpenBodyWire::input_a(&open(Some(
            (&[1u8]).into()
        ))));
    }

    #[test]
    fn an_input_handed_on_to_a_body_is_the_input_the_body_says() {
        for suffix in [Some("a"), None] {
            let body = wireexpr(suffix);
            assert_eq!(
                CodecZenohDeclTokenWire::input_n(&CodecZenohDeclTokenParts {
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
                CodecZenohInterestBodyWire::encode_source(parts, &mut VecSink::new(&mut bytes));
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

    // The chain hands every entry the flag `T` as its start and the input
    // `after_marker` from the entry before, so an entry that says either
    // otherwise has no chain to be in (docs/adr/0065 §6 R8): the refusal is the
    // list's constructor's, the frame's `T` is the start, whatever the carrier
    // the value presents holds, and the encode refuses nothing.
    #[test]
    fn an_entry_that_says_its_input_otherwise_than_its_chain_has_no_chain() {
        let entry = |after_marker| CodecChainPrevEntryParts {
            header: 0x01,
            after_marker,
            tag: Some(9),
        };
        let first = [entry(CodecChainPrevEntryAfterMarker::NotAfterMarker {
            short_value: 5,
        })];
        // The start is the flag `T` and nothing for the entry before.
        let list = |start: bool| Lawful::new((start, ()), (&first).into());
        assert!(
            list(false).is_none(),
            "an entry that says T set has no chain whose start is clear"
        );
        let after_nothing = [entry(CodecChainPrevEntryAfterMarker::AfterMarker {
            long_value: 0x1234,
        })];
        let after_nothing: Bounded<&[CodecChainPrevEntryParts], 4, 1> = (&after_nothing).into();
        assert!(
            Lawful::<_, CodecChainPrevCarrierEntriesLaw>::new((true, ()), after_nothing).is_none(),
            "a first entry that says it follows a marker has no chain"
        );
        let carrier = CodecChainPrevCarrierParts {
            head: 0x00,
            entries: list(true).expect("the entry says the start"),
        };
        let mut bytes = Vec::new();
        let Ok(()) =
            CodecChainPrevCarrierWire::encode_source(&carrier, &mut VecSink::new(&mut bytes));
        assert_eq!(bytes, [0x01, 0x01, 5, 9]);
    }
}

// A flag input every arm of a variant says (docs/adr/0065 §6 R4d), and the tag of
// the arm a body holds (`<sce:variant-dispatch>`): a parent writes both flags from
// the arm whatever its value states of them. The catalog's byte cases hold every
// frame, refusal and input of these codecs; a case states a value as its decode
// presents it, so none states a header that says otherwise than the arm, which is
// what this holds.
mod every_arm_says {
    use super::codecs::codec_every_arm_says::{CodecEveryArmSaysParts, CodecEveryArmSaysPartsBody};
    use super::codecs::codec_every_arm_says_far::CodecEveryArmSaysFarParts;
    use super::codecs::codec_every_arm_says_near::CodecEveryArmSaysNearParts;
    use super::codecs::codec_every_arm_says_parent::{
        CodecEveryArmSaysParentParts, CodecEveryArmSaysParentWire,
    };
    use sce_forge_runtime::codec::{Bounded, VecSink};

    fn near(suffix: Option<&[u8]>) -> CodecEveryArmSaysNearParts<'_> {
        CodecEveryArmSaysNearParts {
            id: 7,
            suffix: suffix.map(|s| Bounded::new(s).expect("within the suffix's bound")),
        }
    }

    fn far(route: Option<u16>) -> CodecEveryArmSaysFarParts {
        CodecEveryArmSaysFarParts { route }
    }

    fn written(value: &CodecEveryArmSaysParts<'_>) -> Vec<u8> {
        let parent = CodecEveryArmSaysParentParts {
            header: 0,
            body: value,
        };
        let mut bytes = Vec::new();
        let Ok(()) =
            CodecEveryArmSaysParentWire::encode_source(&parent, &mut VecSink::new(&mut bytes));
        bytes
    }

    #[test]
    fn a_parent_writes_the_flag_it_binds_as_the_arm_says_it() {
        // `N` is bit 5, as the arm says it, and `M` bit 6, the arm's tag; the
        // value states both clear.
        let with = near(Some(b"a"));
        let value = CodecEveryArmSaysParts {
            body: CodecEveryArmSaysPartsBody::CodecEveryArmSaysNear(&with),
        };
        assert_eq!(written(&value), [0x60, 7, 1, b'a']);

        let without = near(None);
        let value = CodecEveryArmSaysParts {
            body: CodecEveryArmSaysPartsBody::CodecEveryArmSaysNear(&without),
        };
        assert_eq!(written(&value), [0x40, 7]);

        let routed = far(Some(0x0102));
        let value = CodecEveryArmSaysParts {
            body: CodecEveryArmSaysPartsBody::CodecEveryArmSaysFar(&routed),
        };
        assert_eq!(written(&value), [0x00, 1, 2]);

        let bare = far(None);
        let value = CodecEveryArmSaysParts {
            body: CodecEveryArmSaysPartsBody::CodecEveryArmSaysFar(&bare),
        };
        assert_eq!(written(&value), [0x20]);
    }
}

// The fields one gate decides, held together (docs/adr/0065 §6 R4b): a value
// holds a side or none, the gate is derived from which, and a length stated
// while its content is absent is held on the side where it is.
mod groups {
    use super::codecs::codec_chain_flag_entry::{
        CodecChainFlagEntryParts, CodecChainFlagEntryWide, CodecChainFlagEntryWire,
    };
    use super::codecs::codec_init_syn_body::{
        CodecInitSynBody, CodecInitSynBodyParts, CodecInitSynBodyS, CodecInitSynBodyWire,
    };
    use super::codecs::codec_present_if_length_ref::{
        CodecPresentIfLengthRefFlagsHasPayload as HasPayload, CodecPresentIfLengthRefParts,
        CodecPresentIfLengthRefWire,
    };
    use super::codecs::codec_zenoh_value_slice::CodecZenohValueSliceParts;
    use super::codecs::codec_zenoh_value_zbuf::{
        CodecZenohValueZbufAfterShm as AfterShm, CodecZenohValueZbufParts, CodecZenohValueZbufWire,
    };
    use sce_forge_runtime::codec::{Bounded, SceCursor, VecSink};

    #[test]
    fn a_group_of_one_side_is_held_whole_or_not_at_all() {
        let syn = |s| CodecInitSynBodyParts { version: 1, s };
        let with = syn(Some(CodecInitSynBodyS {
            sn_res: 0x33,
            batch_size: 0xCAFE,
        }));
        assert!(CodecInitSynBodyWire::input_s(&with));
        assert!(!CodecInitSynBodyWire::input_s(&syn(None)));
        let mut bytes = Vec::new();
        let Ok(()) = CodecInitSynBodyWire::encode_source(&with, &mut VecSink::new(&mut bytes));
        assert_eq!(bytes, [1, 0x33, 0xCA, 0xFE]);
        let decoded = CodecInitSynBody::decode(&mut SceCursor::new(&bytes), 1).expect("a frame");
        assert_eq!(decoded.s, with.s);
        let decoded = CodecInitSynBody::decode(&mut SceCursor::new(&[1]), 0).expect("a frame");
        assert_eq!(decoded.s, None);
    }

    #[test]
    fn an_input_two_sides_decide_is_the_side_the_value_holds() {
        let entry = |wide| CodecChainFlagEntryParts { header: 0x01, wide };
        assert!(CodecChainFlagEntryWire::input_wide(&entry(
            CodecChainFlagEntryWide::Wide { long_value: 0x1234 }
        )));
        assert!(!CodecChainFlagEntryWire::input_wide(&entry(
            CodecChainFlagEntryWide::NotWide { short_value: 0x12 }
        )));
    }

    #[test]
    fn a_stated_length_is_held_on_the_side_without_its_content() {
        let encode = |after_shm| {
            let mut bytes = Vec::new();
            let Ok(()) = CodecZenohValueZbufWire::encode_source(
                &CodecZenohValueZbufParts { after_shm },
                &mut VecSink::new(&mut bytes),
            );
            bytes
        };
        // The protocol bounds the slices by nothing (docs/adr/0077 §7).
        let none: Bounded<&[CodecZenohValueSliceParts<'_>], { usize::MAX }> =
            Bounded::new(&[][..]).expect("no entries");
        // The sliced side states a length its bytes do not measure.
        let sliced = AfterShm::AfterShm {
            encoding: 0,
            slices: none,
            value_len: 1026,
        };
        assert!(CodecZenohValueZbufWire::input_after_shm(
            &CodecZenohValueZbufParts { after_shm: sliced }
        ));
        assert_eq!(encode(sliced), [0x82, 0x08, 0x00, 0x00]);
        // The plain side's length is its value's.
        let plain = AfterShm::NotAfterShm {
            value: (&[1u8, 2, 3]).into(),
        };
        assert!(!CodecZenohValueZbufWire::input_after_shm(
            &CodecZenohValueZbufParts { after_shm: plain }
        ));
        assert_eq!(encode(plain), [0x03, 1, 2, 3]);
    }

    // An input one arm of a variant reads (docs/adr/0065 §6 R4e): that arm says it,
    // and a value in another arm has no bit the input could change, so it says
    // nothing and the encode is handed nothing.
    #[test]
    fn an_input_one_arm_reads_is_said_by_that_arm_alone() {
        use super::codecs::codec_zenoh_ext_unit::CodecZenohExtUnitParts;
        use super::codecs::codec_zenoh_value_entry::{
            CodecZenohValueEntryParts, CodecZenohValueEntryPartsBody as Body,
            CodecZenohValueEntryWire,
        };
        let entry = |body| CodecZenohValueEntryParts { header: 0x00, body };

        let unit = CodecZenohExtUnitParts {};
        assert_eq!(
            CodecZenohValueEntryWire::input_after_shm(&entry(Body::CodecZenohExtUnit(&unit))),
            None
        );

        let plain = CodecZenohValueZbufParts {
            after_shm: AfterShm::NotAfterShm {
                value: (&[1u8]).into(),
            },
        };
        assert_eq!(
            CodecZenohValueEntryWire::input_after_shm(&entry(Body::CodecZenohValueZbuf(&plain))),
            Some(false)
        );

        let none: Bounded<&[CodecZenohValueSliceParts<'_>], { usize::MAX }> =
            Bounded::new(&[][..]).expect("no entries");
        let sliced = CodecZenohValueZbufParts {
            after_shm: AfterShm::AfterShm {
                encoding: 0,
                slices: none,
                value_len: 0,
            },
        };
        assert_eq!(
            CodecZenohValueEntryWire::input_after_shm(&entry(Body::CodecZenohValueZbuf(&sliced))),
            Some(true)
        );

        // The encode takes no input: the arm's own layout decides the bytes.
        let mut bytes = Vec::new();
        let Ok(()) = CodecZenohValueEntryWire::encode_source(
            &entry(Body::CodecZenohValueZbuf(&plain)),
            &mut VecSink::new(&mut bytes),
        );
        assert_eq!(bytes, [0x40, 0x01, 1]);
    }

    // A disjunction holds its fields in an arm for each assignment of its flags
    // that satisfies it, and none under the one that does not; the flags are
    // derived from the arm (docs/adr/0065 §6 R4c).
    #[test]
    fn a_disjunction_is_held_as_the_assignment_of_its_flags() {
        use super::codecs::codec_present_if_disjunction::{
            CodecPresentIfDisjunction, CodecPresentIfDisjunctionFlagsWantsAOrWantsB as Wants,
            CodecPresentIfDisjunctionParts, CodecPresentIfDisjunctionWire,
        };
        let encode = |held| {
            let mut bytes = Vec::new();
            let Ok(()) = CodecPresentIfDisjunctionWire::encode_source(
                &CodecPresentIfDisjunctionParts {
                    flags: 0x00,
                    flags_wants_a_or_wants_b: held,
                },
                &mut VecSink::new(&mut bytes),
            );
            bytes
        };
        let cases = [
            (Some(Wants::WantsA { seq: 0x0102 }), vec![0x01, 0x01, 0x02]),
            (Some(Wants::WantsB { seq: 0x0102 }), vec![0x02, 0x01, 0x02]),
            (
                Some(Wants::WantsAWantsB { seq: 0x0102 }),
                vec![0x03, 0x01, 0x02],
            ),
            (None, vec![0x00]),
        ];
        for (held, frame) in cases {
            assert_eq!(encode(held), frame);
            let decoded =
                CodecPresentIfDisjunction::decode(&mut SceCursor::new(&frame)).expect("a frame");
            assert_eq!(decoded.flags_wants_a_or_wants_b, held);
        }
    }

    #[test]
    fn a_flag_of_the_codec_is_derived_from_the_side_the_value_holds() {
        let encode = |flags_has_payload| {
            let mut bytes = Vec::new();
            CodecPresentIfLengthRefWire::encode_source(
                &CodecPresentIfLengthRefParts {
                    flags: 0x00,
                    flags_has_payload,
                },
                &mut VecSink::new(&mut bytes),
            )
            .expect("a payload within its length's bits");
            bytes
        };
        assert_eq!(
            encode(HasPayload::HasPayload {
                payload: (&[9u8, 8]).into()
            }),
            [0x01, 0x02, 9, 8]
        );
        assert_eq!(
            encode(HasPayload::NotHasPayload { payload_size: 5 }),
            [0x00, 0x05]
        );
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

// A gate on a chain's content whose named entry holds nothing but its
// identifier is a group of the chain (docs/adr/0065 §6 R6a): where the gate
// holds, the value holds the other entries and the named entry's place among
// them, and writes the named entry there; where it does not, the chain holds no
// entry of the kind. The fields the gate decides cannot disagree with the chain.
mod memberships {
    use super::codecs::codec_chain_has_marker::{
        CodecChainHasMarker, CodecChainHasMarkerExtensionsHas0x12 as Has0x12,
        CodecChainHasMarkerWire,
    };
    use sce_forge_runtime::codec::{SceCursor, VecSink};

    fn encode(value: &CodecChainHasMarker<'_>) -> Vec<u8> {
        let mut bytes = Vec::new();
        let Ok(()) = CodecChainHasMarkerWire::encode_source(value, &mut VecSink::new(&mut bytes));
        bytes
    }

    #[test]
    fn the_named_entry_is_held_at_its_place_and_written_back_there() {
        // E set; extension 1 with a successor, the marker with a successor,
        // extension 3, the last; no slices.
        let frame = [0x80, 0x81, 0x92, 0x03, 0x00];
        let value = CodecChainHasMarker::decode(&mut SceCursor::new(&frame)).expect("a frame");
        match &value.extensions_has_0x12 {
            Has0x12::Has0x12 { extensions, .. } => {
                assert_eq!(extensions.place(), 1);
                assert_eq!(extensions.rest().len(), 2);
            }
            Has0x12::NotHas0x12 { .. } => panic!("the chain holds the marker"),
        }
        assert_eq!(encode(&value), frame);
    }

    #[test]
    fn a_chain_without_the_named_entry_is_held_as_one_with_none_of_its_kind() {
        // E clear: no chain, so no marker, and the payload follows.
        let frame = [0x00, 0x01, 0xAA];
        let value = CodecChainHasMarker::decode(&mut SceCursor::new(&frame)).expect("a frame");
        match &value.extensions_has_0x12 {
            Has0x12::NotHas0x12 {
                extensions,
                payload,
            } => {
                assert!(extensions.is_none());
                assert_eq!(*payload, &[0xAA][..]);
            }
            Has0x12::Has0x12 { .. } => panic!("the chain is absent"),
        }
        assert_eq!(encode(&value), frame);
    }

    use super::codecs::codec_chain_has_tagged::{
        CodecChainHasTagged, CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide as Tagged,
        CodecChainHasTaggedWire,
    };

    fn encode_tagged(value: &CodecChainHasTagged) -> Vec<u8> {
        let mut bytes = Vec::new();
        let Ok(()) = CodecChainHasTaggedWire::encode_source(value, &mut VecSink::new(&mut bytes));
        bytes
    }

    #[test]
    fn the_gates_on_a_chains_entries_and_a_flag_hold_the_arm_their_values_select() {
        // Not wide; an entry of kind 9 with a successor, then one of kind 7:
        // `priority` follows for the 7, and no `checksum` for the 9. The arm
        // says `wide` is clear, so the header is written back with it clear.
        let frame = [0x00, 0x09, 0x01, 0xAA, 0x07, 0x00, 0xBB, 0x42];
        let value = CodecChainHasTagged::decode(&mut SceCursor::new(&frame)).expect("a frame");
        match &value.entries_has_0x7_has_0x9_header_wide {
            Tagged::Has0x7Has0x9 { entries, priority } => {
                assert_eq!(entries.len(), 2);
                assert_eq!(*priority, 0x42);
            }
            other => panic!("kinds 7 and 9, not wide: {other:?}"),
        }
        assert_eq!(encode_tagged(&value), frame);
    }

    #[test]
    fn a_flag_joined_with_a_chains_gate_is_the_arms_to_say() {
        // Wide, and an entry of kind 1 alone: `priority` follows for `wide`,
        // and `checksum` for the missing 9.
        let frame = [0x01, 0x01, 0x00, 0xCC, 0x05, 0x12, 0x34];
        let value = CodecChainHasTagged::decode(&mut SceCursor::new(&frame)).expect("a frame");
        match &value.entries_has_0x7_has_0x9_header_wide {
            Tagged::Wide {
                priority, checksum, ..
            } => assert_eq!((*priority, *checksum), (0x05, 0x1234)),
            other => panic!("neither kind, wide: {other:?}"),
        }
        assert_eq!(encode_tagged(&value), frame);
    }

    #[test]
    fn the_default_value_holds_the_chains_default_in_the_arm_it_selects() {
        // The chain's default is one default entry, of kind 0, so neither gate
        // holds and `wide` starts clear.
        let value = CodecChainHasTagged::default();
        assert!(matches!(
            value.entries_has_0x7_has_0x9_header_wide,
            Tagged::Neither { .. }
        ));
        let bytes = encode_tagged(&value);
        let decoded = CodecChainHasTagged::decode(&mut SceCursor::new(&bytes)).expect("a frame");
        assert_eq!(decoded, value);
    }
}

// Two lists that share one count: the gate decides the second whole, and the
// value holds the two as one value of one count, so the number written cannot
// disagree with either (docs/adr/0065 §6 R5).
mod shared_counts {
    use super::codecs::codec_repeat_elem::CodecRepeatElemParts;
    use super::codecs::codec_repeat_unconditional_count::{
        CodecRepeatUnconditionalCount, CodecRepeatUnconditionalCountOptionsH as Lists,
        CodecRepeatUnconditionalCountParts, CodecRepeatUnconditionalCountWire,
    };
    use sce_forge_runtime::codec::{Aligned, Bounded, SceCursor, VecSink};

    fn encode(value: &CodecRepeatUnconditionalCount) -> Vec<u8> {
        let mut bytes = Vec::new();
        let Ok(()) =
            CodecRepeatUnconditionalCountWire::encode_source(value, &mut VecSink::new(&mut bytes));
        bytes
    }

    #[test]
    fn two_lists_of_one_count_are_held_as_one_value_and_written_as_two_runs() {
        let frame = [0x01, 0x02, 0xCA, 0xFE, 0xBE, 0xEF, 0x11, 0x11, 0x22, 0x22];
        let value =
            CodecRepeatUnconditionalCount::decode(&mut SceCursor::new(&frame)).expect("a frame");
        match &value.options_h {
            Lists::H { links_weights } => {
                assert_eq!(links_weights.len(), 2);
                assert_eq!(links_weights.second()[1].seq, 0x2222);
            }
            Lists::NotH { .. } => panic!("the flag is set"),
        }
        assert_eq!(encode(&value), frame);
    }

    #[test]
    fn the_flag_and_the_count_are_the_arms_to_say() {
        let frame = [0x00, 0x01, 0xCA, 0xFE];
        let value =
            CodecRepeatUnconditionalCount::decode(&mut SceCursor::new(&frame)).expect("a frame");
        assert!(matches!(value.options_h, Lists::NotH { .. }));
        assert_eq!(encode(&value), frame);
        let default = CodecRepeatUnconditionalCount::default();
        assert_eq!(encode(&default), [0x00, 0x00]);
    }

    #[test]
    fn a_source_presents_two_lists_of_one_count_or_none() {
        let links = [
            CodecRepeatElemParts { seq: 1 },
            CodecRepeatElemParts { seq: 2 },
        ];
        let weights = [CodecRepeatElemParts { seq: 3 }];
        let of = |list| Bounded::<&[CodecRepeatElemParts], 64>::new(list).expect("within it");
        // Two lists of different counts are no value a source can present.
        assert!(Aligned::new(of(&links), of(&weights)).is_none());
        let aligned = Aligned::new(of(&links), of(&links)).expect("one count");
        let parts = CodecRepeatUnconditionalCountParts {
            options: 0,
            options_h: Lists::H {
                links_weights: aligned,
            },
        };
        let mut bytes = Vec::new();
        let Ok(()) =
            CodecRepeatUnconditionalCountWire::encode_source(&parts, &mut VecSink::new(&mut bytes));
        assert_eq!(bytes, [0x01, 0x02, 0, 1, 0, 2, 0, 1, 0, 2]);
    }
}

// A chain closed by an entry flag is one entry or more: every entry but the
// last has the flag set, so the grammar has no word for none. A frame that ends
// before the chain's first entry is short, gated or not (docs/adr/0065 §6 R7).
mod chain_ends {
    use super::codecs::codec_chain_depth_entry::CodecChainDepthEntryParts;
    use super::codecs::codec_chain_depth_plain::{CodecChainDepthPlain, CodecChainDepthPlainWire};
    use super::codecs::codec_chain_has_marker::CodecChainHasMarker;
    use sce_forge_runtime::codec::{Bounded, CodecError, SceCursor, VecSink};

    // The list of the chain is one entry or more where the value is made, so no
    // source presents it empty, and the default value holds one default entry:
    // it encodes to a frame its decode reads back.
    #[test]
    fn a_chain_closed_by_an_entry_flag_is_presented_with_an_entry() {
        let none: &[CodecChainDepthEntryParts] = &[];
        assert!(Bounded::<_, 8, 1>::new(none).is_none());
        let value = CodecChainDepthPlain::default();
        assert_eq!(value.extensions.len(), 1);
        let mut bytes = Vec::new();
        let Ok(()) = CodecChainDepthPlainWire::encode_source(&value, &mut VecSink::new(&mut bytes));
        // The header, and one entry: id 0, the last.
        assert_eq!(bytes, [0x00, 0x00]);
        let decoded = CodecChainDepthPlain::decode(&mut SceCursor::new(&bytes)).expect("a frame");
        assert_eq!(decoded.extensions.len(), 1);
    }

    #[test]
    fn a_frame_that_ends_before_a_chains_first_entry_is_short() {
        // The header and no entry of the chain that always follows it.
        assert_eq!(
            CodecChainDepthPlain::decode(&mut SceCursor::new(&[0x00])).err(),
            Some(CodecError::NeedMoreBytes)
        );
        // `E` set, and nothing after the header.
        assert_eq!(
            CodecChainHasMarker::decode(&mut SceCursor::new(&[0x80])).err(),
            Some(CodecError::NeedMoreBytes)
        );
    }
}
