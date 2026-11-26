// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Gated on `alloc` like the other generated-codec tests of this crate.
#![cfg(feature = "alloc")]

//! A chain inside a body a length bounds is shown to the hook of the view
//! that reads the frame, in its one read (docs/adr/0130).
//!
//! `fixtures/codec_chain_within_length.scxml` holds a VLE length, the body
//! `codec_tlv_chain_basic` read from the bytes the length gives it, and one byte
//! after the body. The body's chain is read while bytes remain, so it ends
//! where the length ends it. A view read with a hook hands the hook to every
//! body it reads, the body a length bounds among them, so the hook is shown
//! each entry of that chain right after it is read, and what it refuses is the
//! frame's verdict.

mod codecs {
    pub mod codec_tlv_entry {
        include!(concat!(env!("OUT_DIR"), "/codec_tlv_entry.rs"));
    }
    pub mod codec_tlv_chain_basic {
        include!(concat!(env!("OUT_DIR"), "/codec_tlv_chain_basic.rs"));
    }
    pub mod codec_chain_within_length {
        include!(concat!(env!("OUT_DIR"), "/codec_chain_within_length.rs"));
    }
}

use codecs::codec_chain_within_length::CodecChainWithinLengthView;
use sce_forge_runtime::codec::{ChainHook, ChainSite, CodecError, EntryRefused, SceCursor};

/// The chain of the bounded body, by the codec that declares it and its field.
const EXTENSIONS: ChainSite = ChainSite {
    codec: "codec_tlv_chain_basic",
    field: "extensions",
};

/// A length of 9; the body, its header byte 0x01 and two entries, (0x10 "ABC")
/// and (0x20 "X"); and the byte 0x7e after the body.
const FRAME: [u8; 11] = [
    0x09, 0x01, 0x10, 0x03, 0x41, 0x42, 0x43, 0x20, 0x01, 0x58, 0x7e,
];

/// The first entry of the chain, as the bytes it occupies.
const FIRST: [u8; 5] = [0x10, 0x03, 0x41, 0x42, 0x43];

/// The second entry of the chain, as the bytes it occupies.
const SECOND: [u8; 3] = [0x20, 0x01, 0x58];

/// Each entry shown, by its site; the entry whose first byte is `refuse` is
/// refused.
struct Seen {
    entries: Vec<(ChainSite, Vec<u8>)>,
    refuse: Option<u8>,
}

impl Seen {
    fn admitting() -> Self {
        Self {
            entries: Vec::new(),
            refuse: None,
        }
    }

    fn refusing(first: u8) -> Self {
        Self {
            entries: Vec::new(),
            refuse: Some(first),
        }
    }
}

impl ChainHook for Seen {
    fn entry(&mut self, site: ChainSite, entry: &[u8]) -> Result<(), EntryRefused> {
        self.entries.push((site, entry.to_vec()));
        match (self.refuse, entry.first()) {
            (Some(refused), Some(&first)) if refused == first => Err(EntryRefused),
            _ => Ok(()),
        }
    }
}

#[test]
fn each_entry_of_a_chain_within_a_length_is_shown_to_the_hook() {
    let mut hook = Seen::admitting();
    let mut view = CodecChainWithinLengthView::default();
    let mut cursor = SceCursor::new(&FRAME);
    assert_eq!(
        CodecChainWithinLengthView::decode_with(&mut cursor, &mut view, &mut hook),
        Ok(())
    );
    assert_eq!(
        hook.entries,
        vec![(EXTENSIONS, FIRST.to_vec()), (EXTENSIONS, SECOND.to_vec())]
    );
    // The chain ended where the length ended it, and the byte after the body
    // is read as the field it is.
    assert_eq!(view.total_length, 9);
    assert_eq!(view.inner.header_flags, 0x01);
    assert_eq!(view.trailer, 0x7e);
    assert_eq!(cursor.remaining(), 0);
}

#[test]
fn an_entry_refused_within_a_length_is_the_frames_verdict() {
    let mut hook = Seen::refusing(0x20);
    let mut view = CodecChainWithinLengthView::default();
    let mut cursor = SceCursor::new(&FRAME);
    assert_eq!(
        CodecChainWithinLengthView::decode_with(&mut cursor, &mut view, &mut hook),
        Err(CodecError::ChainEntryRefused)
    );
    // The entry refused was shown, and nothing after it: the hook ended the
    // chain there.
    assert_eq!(
        hook.entries,
        vec![(EXTENSIONS, FIRST.to_vec()), (EXTENSIONS, SECOND.to_vec())]
    );
    // A refused frame leaves the cursor where it stood.
    assert_eq!(cursor.remaining(), FRAME.len());
}

#[test]
fn a_read_with_a_hook_reads_the_frame_a_read_without_one_reads() {
    let mut hook = Seen::admitting();
    let mut hooked = CodecChainWithinLengthView::default();
    assert_eq!(
        CodecChainWithinLengthView::decode_with(
            &mut SceCursor::new(&FRAME),
            &mut hooked,
            &mut hook
        ),
        Ok(())
    );
    assert_eq!(
        CodecChainWithinLengthView::decode(&mut SceCursor::new(&FRAME)),
        Ok(hooked)
    );
}
