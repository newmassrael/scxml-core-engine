// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A storing decode reads a frame past a capacity in one pass (docs/adr/0077
//! §7, amendment of 2026-11-05): a field past what the profile holds is read on
//! and not held, and the decode answers `CapacityExceeded` only once the frame
//! is read whole and every refusal of its own has had its turn.
//!
//! Each kind of site a capacity can stop is decoded at a profile that holds two
//! entries of a list and two bytes of a byte string or text (`Tight`), over
//! every combination of how much the frame carries (within the capacity, at it,
//! past it) and how the frame is made (well formed, malformed after the point the
//! capacity is met, malformed before it):
//!
//! - a counted list (`codec_repeat_basic`);
//! - a chain whose kinds of entry decide a group's arm (`codec_chain_has_envelope`,
//!   `has(2)`), past its capacity with and without an entry of the kind;
//! - two lists of one count (`codec_repeat_unconditional_count`);
//! - text a copying profile stores (`codec_nested_body`'s locators);
//! - an embedded body past its capacity (`codec_nested_parent`);
//! - a variant's arm holding bytes (`codec_zenoh_ext_entry`'s zbuf arm, as the
//!   entries of `codec_chain_has_envelope`).
//!
//! Each case states the verdict it expects, and the view of the codec, which
//! stores nothing and cannot run out, is the oracle the expectation is checked
//! against: a frame the view accepts is `CapacityExceeded` past the capacity and
//! accepted within it, and a frame the view refuses gets the view's error. After
//! every decode, refused or not, what the value holds is checked against what its
//! types say: a list inside its bounds, a chain the kinds its arm names, two lists
//! of one count.
#![cfg(feature = "alloc")]

mod codecs {
    pub mod codec_repeat_elem {
        include!(concat!(env!("OUT_DIR"), "/codec_repeat_elem.rs"));
    }
    pub mod codec_repeat_basic {
        include!(concat!(env!("OUT_DIR"), "/codec_repeat_basic.rs"));
    }
    pub mod codec_repeat_unconditional_count {
        include!(concat!(
            env!("OUT_DIR"),
            "/codec_repeat_unconditional_count.rs"
        ));
    }
    pub mod codec_zenoh_locator {
        include!(concat!(env!("OUT_DIR"), "/codec_zenoh_locator.rs"));
    }
    pub mod codec_nested_body {
        include!(concat!(env!("OUT_DIR"), "/codec_nested_body.rs"));
    }
    pub mod codec_nested_parent {
        include!(concat!(env!("OUT_DIR"), "/codec_nested_parent.rs"));
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
    pub mod codec_chain_has_slice {
        include!(concat!(env!("OUT_DIR"), "/codec_chain_has_slice.rs"));
    }
    pub mod codec_chain_has_envelope {
        include!(concat!(env!("OUT_DIR"), "/codec_chain_has_envelope.rs"));
    }
}

use codecs::codec_chain_has_envelope::{
    CodecChainHasEnvelopeExtensionsHas0x2 as Envelope, CodecChainHasEnvelopeOwned,
    CodecChainHasEnvelopeView,
};
use codecs::codec_nested_body::{CodecNestedBodyOwned, CodecNestedBodyView};
use codecs::codec_nested_parent::{CodecNestedParentOwned, CodecNestedParentView};
use codecs::codec_repeat_basic::{CodecRepeatBasicOwned, CodecRepeatBasicView};
use codecs::codec_repeat_unconditional_count::{
    CodecRepeatUnconditionalCountOptionsH as Paired, CodecRepeatUnconditionalCountOwned,
    CodecRepeatUnconditionalCountView,
};
use sce_forge_runtime::codec::{
    CodecError, Copying, HoldsBytes, HoldsList, HoldsStr, InlineBytes, InlineStr, SceCursor,
};

/// A profile that holds two entries of every list and two bytes of every byte
/// string or text: below every bound the fixtures state.
#[derive(Debug, Clone, PartialEq)]
struct Tight;

impl<Tag> HoldsList<Tag> for Tight {
    type List<T>
        = sce_forge_runtime::heapless::Vec<T, 2>
    where
        T: Default + core::fmt::Debug + Clone + PartialEq;
}
impl<Tag> HoldsStr<Tag> for Tight {
    type Str = InlineStr<2>;
}
impl<Tag> HoldsBytes<Tag> for Tight {
    type Bytes = InlineBytes<2>;
}
impl Copying for Tight {}

/// How much of a site the frame carries against `Tight`.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Room {
    Within,
    At,
    Past,
}

/// How the frame is made.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Shape {
    WellFormed,
    /// Broken after the point where the capacity is met (or, within it, after the
    /// site).
    MalformedAfter,
    /// Broken before the point where the capacity is met.
    MalformedBefore,
}

use Room::{At, Past, Within};
use Shape::{MalformedAfter, MalformedBefore, WellFormed};

struct Case {
    room: Room,
    shape: Shape,
    frame: &'static [u8],
    expect: Result<(), CodecError>,
}

const fn case(
    room: Room,
    shape: Shape,
    frame: &'static [u8],
    expect: Result<(), CodecError>,
) -> Case {
    Case {
        room,
        shape,
        frame,
        expect,
    }
}

const OK: Result<(), CodecError> = Ok(());
const OVER: Result<(), CodecError> = Err(CodecError::CapacityExceeded);
const SHORT: Result<(), CodecError> = Err(CodecError::NeedMoreBytes);
const MANY: Result<(), CodecError> = Err(CodecError::TooManyElements);
const UTF8: Result<(), CodecError> = Err(CodecError::InvalidUtf8);

/// Each case of `site` decoded into a value of `T` at `Tight`, against the view
/// (`view`), and what the value holds afterwards checked by `holds`.
fn judge<T: Default>(
    site: &str,
    cases: &[Case],
    decode: impl Fn(&mut SceCursor<'static>, &mut T) -> Result<(), CodecError>,
    view: impl Fn(&mut SceCursor<'static>) -> Result<(), CodecError>,
    holds: impl Fn(&T, &str),
) {
    for room in [Within, At, Past] {
        for shape in [WellFormed, MalformedAfter, MalformedBefore] {
            assert!(
                cases.iter().any(|c| c.room == room && c.shape == shape),
                "{site}: no case of {room:?} x {shape:?}"
            );
        }
    }
    for c in cases {
        let what = format!(
            "{site}, {:?} x {:?}, frame {:02x?}",
            c.room, c.shape, c.frame
        );
        let viewed = view(&mut SceCursor::new(c.frame));
        match (c.shape, &viewed) {
            (WellFormed, Err(e)) => panic!("{what}: the view refuses a well-formed frame: {e:?}"),
            (MalformedAfter | MalformedBefore, Ok(())) => {
                panic!("{what}: the view accepts a malformed frame")
            }
            _ => {}
        }
        let oracle = match (viewed, c.room) {
            (Ok(()), Past) => OVER,
            (Ok(()), _) => OK,
            (Err(e), _) => Err(e),
        };
        assert_eq!(
            c.expect, oracle,
            "{what}: the case's verdict is not the view's"
        );
        let mut out = T::default();
        let stored = decode(&mut SceCursor::new(c.frame), &mut out);
        assert_eq!(stored, c.expect, "{what}");
        holds(&out, &what);
    }
}

#[test]
fn a_counted_list_past_its_capacity_is_read_whole() {
    let cases = [
        case(Within, WellFormed, &[1, 0, 1], OK),
        case(At, WellFormed, &[2, 0, 1, 0, 2], OK),
        case(Past, WellFormed, &[3, 0, 1, 0, 2, 0, 3], OVER),
        case(Within, MalformedAfter, &[1, 0], SHORT),
        case(At, MalformedAfter, &[2, 0, 1, 0], SHORT),
        // The third entry, past the capacity, is cut short: the frame's own
        // refusal, not the node's.
        case(Past, MalformedAfter, &[3, 0, 1, 0, 2, 0], SHORT),
        // A count past the bound of 32, refused before an entry is read.
        case(Within, MalformedBefore, &[33, 0, 1], MANY),
        case(At, MalformedBefore, &[33, 0, 1, 0, 2], MANY),
        case(Past, MalformedBefore, &[33, 0, 1, 0, 2, 0, 3], MANY),
    ];
    judge(
        "counted list",
        &cases,
        |c, out: &mut CodecRepeatBasicOwned<Tight>| {
            CodecRepeatBasicOwned::<Tight>::decode_into(c, out)
        },
        |c| CodecRepeatBasicView::decode(c).map(|_| ()),
        |out, what| assert!(out.frags.len() <= 2, "{what}: a list past its capacity"),
    );
}

/// What a value of `codec_chain_has_envelope` holds: the arm its chain's kinds
/// name, with a chain inside its bounds.
fn envelope_holds(out: &CodecChainHasEnvelopeOwned<Tight>, what: &str) {
    let has_2 = |header: u8| header & 0x0F == 2;
    match &out.extensions_has_0x2 {
        Envelope::Has0x2 { extensions, slices } => {
            assert!(
                extensions.inner().iter().any(|e| has_2(e.header)),
                "{what}: the arm of has(2) holds a chain with no entry of the kind"
            );
            assert!(
                (1..=2).contains(&extensions.len()),
                "{what}: a chain outside its bounds"
            );
            assert!(slices.len() <= 2, "{what}: a list past its capacity");
        }
        Envelope::NotHas0x2 { extensions, .. } => {
            if let Some(chain) = extensions {
                assert!(
                    !chain.inner().iter().any(|e| has_2(e.header)),
                    "{what}: the arm without has(2) holds an entry of the kind"
                );
                assert!(
                    (1..=2).contains(&chain.len()),
                    "{what}: a chain outside its bounds"
                );
            }
        }
    }
}

fn envelope_decode(
    c: &mut SceCursor<'static>,
    out: &mut CodecChainHasEnvelopeOwned<Tight>,
) -> Result<(), CodecError> {
    CodecChainHasEnvelopeOwned::<Tight>::decode_into(c, out)
}

fn envelope_view(c: &mut SceCursor<'static>) -> Result<(), CodecError> {
    CodecChainHasEnvelopeView::decode(c).map(|_| ())
}

#[test]
fn a_chain_whose_kinds_decide_an_arm_is_read_whole_past_its_capacity() {
    // Header 0x80: the chain is present. An entry with no content is its header:
    // the identifier in the low four bits, more to come in bit 7.
    let cases = [
        case(Within, WellFormed, &[0x80, 0x01, 1, 0xAA], OK),
        case(At, WellFormed, &[0x80, 0x81, 0x01, 1, 0xAA], OK),
        case(Past, WellFormed, &[0x80, 0x81, 0x81, 0x01, 1, 0xAA], OVER),
        case(Within, MalformedAfter, &[0x80, 0x01, 2, 0xAA], SHORT),
        case(At, MalformedAfter, &[0x80, 0x81, 0x01, 2, 0xAA], SHORT),
        // The chain is past its capacity and the payload after it cut short.
        case(
            Past,
            MalformedAfter,
            &[0x80, 0x81, 0x81, 0x01, 2, 0xAA],
            SHORT,
        ),
        // A chain that announces an entry and ends.
        case(Within, MalformedBefore, &[0x80], SHORT),
        case(At, MalformedBefore, &[0x80, 0x81], SHORT),
        case(Past, MalformedBefore, &[0x80, 0x81, 0x81], SHORT),
    ];
    judge(
        "chain with has(2)",
        &cases,
        envelope_decode,
        envelope_view,
        envelope_holds,
    );
}

#[test]
fn a_full_chain_keeps_an_entry_of_the_kind_its_gate_reads_and_its_arm() {
    // Three entries, 1, 3 and 2, past two held: the entry of kind 2 comes last and
    // takes the place of 3, so the arm of has(2) holds, and the fields after the
    // chain are read as has(2) says: a slice count of none.
    let cases = [
        case(Within, WellFormed, &[0x80, 0x02, 0], OK),
        case(At, WellFormed, &[0x80, 0x81, 0x02, 0], OK),
        case(Past, WellFormed, &[0x80, 0x81, 0x83, 0x02, 0], OVER),
        case(Within, MalformedAfter, &[0x80, 0x02, 1], SHORT),
        case(At, MalformedAfter, &[0x80, 0x81, 0x02, 1], SHORT),
        // Read as has(2), a slice is announced and missing: the frame's refusal.
        case(Past, MalformedAfter, &[0x80, 0x81, 0x83, 0x02, 1], SHORT),
        // A slice count past its bound of four.
        case(Within, MalformedBefore, &[0x80, 0x02, 5], MANY),
        case(At, MalformedBefore, &[0x80, 0x81, 0x02, 5], MANY),
        case(Past, MalformedBefore, &[0x80, 0x81, 0x83, 0x02, 5], MANY),
    ];
    judge(
        "chain keeping its gated kind",
        &cases,
        envelope_decode,
        envelope_view,
        envelope_holds,
    );
    // The arm of has(2) is the one held past the capacity.
    let mut out = CodecChainHasEnvelopeOwned::<Tight>::default();
    let frame: &'static [u8] = &[0x80, 0x81, 0x83, 0x02, 0];
    assert_eq!(envelope_decode(&mut SceCursor::new(frame), &mut out), OVER);
    assert!(matches!(out.extensions_has_0x2, Envelope::Has0x2 { .. }));
}

#[test]
fn two_lists_of_one_count_past_their_capacity_are_read_whole() {
    // Options 0x01: the second list is present; the count is the first's.
    let cases = [
        case(Within, WellFormed, &[1, 1, 0, 1, 0, 2], OK),
        case(At, WellFormed, &[1, 2, 0, 1, 0, 2, 0, 3, 0, 4], OK),
        case(
            Past,
            WellFormed,
            &[1, 3, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6],
            OVER,
        ),
        case(Within, MalformedAfter, &[1, 1, 0, 1, 0], SHORT),
        case(At, MalformedAfter, &[1, 2, 0, 1, 0, 2, 0, 3, 0], SHORT),
        case(
            Past,
            MalformedAfter,
            &[1, 3, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0],
            SHORT,
        ),
        // A count past the bound of 64.
        case(Within, MalformedBefore, &[1, 65, 0, 1], MANY),
        case(At, MalformedBefore, &[1, 65, 0, 1, 0, 2], MANY),
        case(Past, MalformedBefore, &[1, 65, 0, 1, 0, 2, 0, 3], MANY),
    ];
    judge(
        "two lists of one count",
        &cases,
        |c, out: &mut CodecRepeatUnconditionalCountOwned<Tight>| {
            CodecRepeatUnconditionalCountOwned::<Tight>::decode_into(c, out)
        },
        |c| CodecRepeatUnconditionalCountView::decode(c).map(|_| ()),
        |out, what| match &out.options_h {
            Paired::H { links_weights } => assert_eq!(
                links_weights.first().len(),
                links_weights.second().len(),
                "{what}: two lists of one count hold two counts"
            ),
            Paired::NotH { links } => {
                assert!(links.len() <= 2, "{what}: a list past its capacity")
            }
        },
    );
}

#[test]
fn text_past_what_a_profile_stores_is_read_whole() {
    // Two locators, each a length and its text; the text holds two bytes.
    let cases = [
        case(Within, WellFormed, &[2, 1, b'a', 1, b'b'], OK),
        case(At, WellFormed, &[2, 2, b'a', b'b', 1, b'c'], OK),
        case(Past, WellFormed, &[2, 3, b'a', b'b', b'c', 1, b'd'], OVER),
        case(Within, MalformedAfter, &[2, 1, b'a', 2, b'b'], SHORT),
        case(At, MalformedAfter, &[2, 2, b'a', b'b', 2, b'c'], SHORT),
        // The first text is past what the profile holds; the second is cut short.
        case(
            Past,
            MalformedAfter,
            &[2, 3, b'a', b'b', b'c', 2, b'd'],
            SHORT,
        ),
        // The first text is not UTF-8, before any is past the capacity.
        case(Within, MalformedBefore, &[2, 1, 0xFF, 1, b'b'], UTF8),
        case(At, MalformedBefore, &[2, 1, 0xFF, 2, b'a', b'b'], UTF8),
        case(
            Past,
            MalformedBefore,
            &[2, 1, 0xFF, 3, b'a', b'b', b'c'],
            UTF8,
        ),
    ];
    judge(
        "text a copying profile stores",
        &cases,
        |c, out: &mut CodecNestedBodyOwned<Tight>| {
            CodecNestedBodyOwned::<Tight>::decode_into(c, out)
        },
        |c| CodecNestedBodyView::decode(c).map(|_| ()),
        |out, what| assert!(out.locs.len() <= 2, "{what}: a list past its capacity"),
    );
}

#[test]
fn an_embedded_body_past_its_capacity_is_read_whole_and_its_holder_reads_on() {
    // No optional body; one body listed after the required one, of no locators.
    let cases = [
        case(Within, WellFormed, &[0, 1, 1, 1, b'a', 0], OK),
        case(At, WellFormed, &[0, 1, 2, 1, b'a', 1, b'b', 0], OK),
        case(
            Past,
            WellFormed,
            &[0, 1, 3, 1, b'a', 1, b'b', 1, b'c', 0],
            OVER,
        ),
        case(Within, MalformedAfter, &[0, 1, 1, 1, b'a'], SHORT),
        case(At, MalformedAfter, &[0, 1, 2, 1, b'a', 1, b'b'], SHORT),
        // The embedded body is past its capacity; the listed body after it is
        // missing.
        case(
            Past,
            MalformedAfter,
            &[0, 1, 3, 1, b'a', 1, b'b', 1, b'c'],
            SHORT,
        ),
        // The embedded body's count is past its bound of four.
        case(Within, MalformedBefore, &[0, 1, 5, 1, b'a'], MANY),
        case(At, MalformedBefore, &[0, 1, 5, 1, b'a', 1, b'b'], MANY),
        case(
            Past,
            MalformedBefore,
            &[0, 1, 5, 1, b'a', 1, b'b', 1, b'c'],
            MANY,
        ),
    ];
    judge(
        "embedded body",
        &cases,
        |c, out: &mut CodecNestedParentOwned<Tight>| {
            CodecNestedParentOwned::<Tight>::decode_into(c, out)
        },
        |c| CodecNestedParentView::decode(c).map(|_| ()),
        |out, what| {
            assert!(
                out.required_body.locs.len() <= 2,
                "{what}: a list past its capacity"
            );
            assert!(out.body_list.len() <= 2, "{what}: a list past its capacity");
        },
    );
}

#[test]
fn a_variant_arm_past_its_capacity_is_read_whole_and_its_holder_reads_on() {
    // An extension entry of kind 3 whose arm is a byte string (encoding 2, header
    // 0x43), the last of the chain, then the payload.
    let cases = [
        case(Within, WellFormed, &[0x80, 0x43, 1, 0x11, 1, 0xAA], OK),
        case(At, WellFormed, &[0x80, 0x43, 2, 0x11, 0x22, 1, 0xAA], OK),
        case(
            Past,
            WellFormed,
            &[0x80, 0x43, 3, 0x11, 0x22, 0x33, 1, 0xAA],
            OVER,
        ),
        case(
            Within,
            MalformedAfter,
            &[0x80, 0x43, 1, 0x11, 2, 0xAA],
            SHORT,
        ),
        case(
            At,
            MalformedAfter,
            &[0x80, 0x43, 2, 0x11, 0x22, 2, 0xAA],
            SHORT,
        ),
        // The arm's bytes are past what the profile holds; the payload after the
        // chain is cut short.
        case(
            Past,
            MalformedAfter,
            &[0x80, 0x43, 3, 0x11, 0x22, 0x33, 2, 0xAA],
            SHORT,
        ),
        // The arm's bytes end before their length.
        case(Within, MalformedBefore, &[0x80, 0x43, 1], SHORT),
        case(At, MalformedBefore, &[0x80, 0x43, 2, 0x11], SHORT),
        case(Past, MalformedBefore, &[0x80, 0x43, 3, 0x11], SHORT),
    ];
    judge(
        "variant arm",
        &cases,
        envelope_decode,
        envelope_view,
        envelope_holds,
    );
}
