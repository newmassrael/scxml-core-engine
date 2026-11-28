// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Regular expressions as I-Regexp (RFC 9485) writes them: read by its grammar, built into
//! Thompson's automaton over code points, and matched by simulating that automaton, in time
//! linear in the text and with no heap (docs/adr/0149, D1, D3 and D4).
//!
//! # What a text is tested for
//!
//! [`Regexp::matches`] is true when the whole text is in the language (RFC 9535, section
//! 2.4.6); [`Regexp::search`] when some part of it is, the text around that part being any
//! well-formed text (section 2.4.7). Both read UTF-8 by RFC 3629's grammar (section 4) as they
//! go, so a byte sequence that is not UTF-8 is in no language and neither test accepts it.
//!
//! # The automaton
//!
//! The reader ([`check`], [`compile`]) follows RFC 9485's ABNF (section 3) and its one
//! additional restriction (`[^]` is no class). Construction is Thompson's (1968): one state per
//! character, dot or class, one split per alternative, `?`, `*` and `+`, every counted
//! repetition expanded, and one accepting state. A class is one state whatever its ranges: its
//! code points are tested when a text is read, against the ranges the expression wrote and the
//! General_Category of the code point in the pinned Unicode data (`unicode/pin.toml`), so the
//! count of states reads the expression alone and is the same under every Unicode version.
//!
//! The simulation advances a set of states per code point (Thompson 1968; Cox 2009), the set a
//! sparse set (Briggs and Torczon 1993), each state entered at most once per code point: time
//! O(n * m) for a text of n code points and an automaton of m states, space O(m) words. Neither
//! the reader nor the simulation recurses or allocates; the storage is the caller's, sized by
//! two bounds: the states of the automaton (the bound a build names `regexp_nfa_states`) and the
//! bytes of the expression's text.
//!
//! ```
//! let mut space = sce_rust_regexp::Space::<64, 32>::new();
//! let mut slug = space.compile("[a-z0-9]+(-[a-z0-9]+)*").unwrap();
//! assert!(slug.matches(b"an-id-7"));
//! assert!(!slug.matches(b"An id"));
//! assert!(slug.search(b"An id"));
//! ```

#![no_std]
#![forbid(unsafe_code)]

mod nfa;
mod simulate;
mod syntax;
mod utf8;

pub use nfa::{Class, Range, State};
pub use simulate::{Meter, Regexp, Steps};
pub use syntax::Frame;

use nfa::Builder;

/// Why a text is not read as a regular expression, or not built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// The text is not an I-Regexp: RFC 9485's grammar stops at the byte `offset` of the text.
    Invalid {
        /// The byte of the text where the grammar stops.
        offset: usize,
        /// What the grammar found there.
        reason: Reason,
    },
    /// The text is an I-Regexp whose automaton has more states than the bound. `states` is the
    /// automaton's count, read from the expression without building it; past `u64::MAX` it
    /// stays there.
    TooLarge {
        /// The states the automaton would have, its accepting state among them.
        states: u64,
        /// The most states it may have.
        bound: usize,
    },
    /// The storage given to [`compile`] or [`check`] is smaller than the text needs.
    Storage(Shortage),
}

/// What RFC 9485's grammar found where it stops.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    /// The text ends where the grammar needs more: an open group, class, escape or quantifier.
    UnexpectedEnd,
    /// A character the grammar has no place for here: `)` with no group open, `]` or `}`
    /// outside a class or a quantifier, `[` inside a class (class subtraction), a `-` inside a
    /// class that neither ends a range nor stands last.
    Unexpected,
    /// A quantifier with nothing to repeat: at the start of a branch or a group, or after another
    /// quantifier (I-Regexp has no lazy or possessive form).
    NothingToRepeat,
    /// A backslash before a character that is no escape of I-Regexp: a multi-character escape
    /// (`\d`, `\s`, `\w`), a code point escape (`\u`), a backreference (`\1`).
    Escape,
    /// `\p{..}` or `\P{..}` naming no General Category value or group of RFC 9485's list: a
    /// Unicode block (`IsBasicLatin`), a script, `Cs`, `LC`.
    Category,
    /// A class with nothing in it: `[]`, and `[^]`, which RFC 9485 refuses by name.
    EmptyClass,
    /// A range of a class whose first character is past its last.
    RangeOutOfOrder,
    /// A range quantifier whose least count is more than its most.
    QuantifierOutOfOrder,
}

/// The part of the storage that was too small.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shortage {
    /// One frame is held for each group open at once; as many frames as the text has bytes
    /// never run short.
    Frames,
    /// One class for each class the text writes; as many as the text has bytes never run short.
    Classes,
    /// One range for each character or range a class writes; as many as the text has bytes never
    /// run short.
    Ranges,
    /// Five words for each state of the automaton.
    Scratch,
}

/// What a text that is an I-Regexp asks of the storage that builds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Measure {
    /// The states of its automaton, its accepting state among them.
    pub states: u64,
    /// The classes it writes.
    pub classes: usize,
    /// The characters and ranges its classes write.
    pub ranges: usize,
    /// The most groups open at once.
    pub depth: usize,
}

/// The storage [`compile`] builds an automaton into, every part the caller's.
#[derive(Debug)]
pub struct Storage<'s> {
    /// The automaton's states. Its length is the bound of states: an automaton with more is
    /// refused as [`Error::TooLarge`].
    pub states: &'s mut [State],
    /// Five words for each element of `states`: the two sets and the stack the simulation keeps.
    pub scratch: &'s mut [u32],
    /// The classes, one for each class the text writes.
    pub classes: &'s mut [Class],
    /// The ranges of the classes.
    pub ranges: &'s mut [Range],
    /// One frame for each group open at once.
    pub frames: &'s mut [Frame],
}

/// Reads `regexp` by RFC 9485's grammar and counts the states of its automaton without building
/// it, refusing a text that is not an I-Regexp and one whose automaton has more than `bound`
/// states. It reads no Unicode data: a class counts one state whatever its ranges. Time is
/// linear in the text; `frames` holds the groups open at once.
pub fn check(regexp: &str, bound: usize, frames: &mut [Frame]) -> Result<Measure, Error> {
    let mut builder = Builder::new(Default::default(), Default::default(), Default::default());
    let read = syntax::read(regexp, &mut builder, frames)?;
    let built = builder.finish(read.whole);
    if built.len > bound as u64 {
        return Err(Error::TooLarge {
            states: built.len,
            bound,
        });
    }
    Ok(Measure {
        states: built.len,
        classes: built.class_len,
        ranges: built.range_len,
        depth: read.depth,
    })
}

/// Reads `regexp` by RFC 9485's grammar and builds its automaton into `storage`, ready to test
/// texts. The bound of states is the length of `storage.states`; the build takes time linear in
/// that bound and in the text.
pub fn compile<'s>(regexp: &str, storage: Storage<'s>) -> Result<Regexp<'s>, Error> {
    let Storage {
        states,
        scratch,
        classes,
        ranges,
        frames,
    } = storage;
    let mut builder = Builder::new(states, classes, ranges);
    let read = syntax::read(regexp, &mut builder, frames)?;
    let built = builder.finish(read.whole);
    if built.len > built.cap {
        return Err(Error::TooLarge {
            states: built.len,
            bound: built.cap as usize,
        });
    }
    if built.class_len > built.classes.len() {
        return Err(Error::Storage(Shortage::Classes));
    }
    if built.range_len > built.ranges.len() {
        return Err(Error::Storage(Shortage::Ranges));
    }
    let count = built.len as usize;
    if scratch.len() / 5 < count {
        return Err(Error::Storage(Shortage::Scratch));
    }
    let states: &'s [State] = built.states;
    let classes: &'s [Class] = built.classes;
    let ranges: &'s [Range] = built.ranges;
    Ok(Regexp::new(
        &states[..count],
        &classes[..built.class_len],
        ranges,
        built.start,
        built.accept,
        scratch,
    ))
}

/// Storage for one automaton of at most `STATES` states built from a text of at most `TEXT`
/// bytes, held inline: the `regexp[N]` of a machine is `Space<B, N>` for its bound of states
/// `B`.
#[derive(Debug)]
pub struct Space<const STATES: usize, const TEXT: usize> {
    states: [State; STATES],
    scratch: [[u32; 5]; STATES],
    classes: [Class; TEXT],
    ranges: [Range; TEXT],
    frames: [Frame; TEXT],
}

impl<const STATES: usize, const TEXT: usize> Space<STATES, TEXT> {
    /// Storage holding no automaton.
    pub const fn new() -> Self {
        Space {
            states: [State::VACANT; STATES],
            scratch: [[0; 5]; STATES],
            classes: [Class::VACANT; TEXT],
            ranges: [Range::VACANT; TEXT],
            frames: [Frame::VACANT; TEXT],
        }
    }

    /// [`compile`] into this storage: refused past `STATES` states, and never short of storage
    /// for a text of at most `TEXT` bytes.
    pub fn compile(&mut self, regexp: &str) -> Result<Regexp<'_>, Error> {
        compile(
            regexp,
            Storage {
                states: &mut self.states,
                scratch: self.scratch.as_flattened_mut(),
                classes: &mut self.classes,
                ranges: &mut self.ranges,
                frames: &mut self.frames,
            },
        )
    }
}

impl<const STATES: usize, const TEXT: usize> Default for Space<STATES, TEXT> {
    fn default() -> Self {
        Self::new()
    }
}
