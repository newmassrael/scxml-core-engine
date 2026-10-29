// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A set of states of one document, as a bit vector over [`StateId`]: bit `i`
//! of the vector is state `i`.
//!
//! Every set Appendix D's procedures keep is a set of states of one finite
//! document — the configuration, what a `<history>` recorded, the states a
//! microstep exits and the states it enters — so each is a bit vector of the
//! document's size. Membership, adding and removing are one bit each; a set
//! holds each state once by what it is; and because a state's number is its
//! position in document order, reading a set in ascending order is reading it
//! in entry order, descending in exit order, and a subtree is one range of
//! bits. No set is ever sorted.

use crate::helpers::tables::StateId;

/// One word of a set: 32 states.
pub type Word = u32;

const WORD_BITS: usize = Word::BITS as usize;

/// The words a set of the states of a document with `states` states takes.
pub const fn state_words(states: usize) -> usize {
    states.div_ceil(WORD_BITS)
}

#[inline]
fn word_and_bit(state: StateId) -> (usize, Word) {
    let index = state.index();
    (index / WORD_BITS, 1 << (index % WORD_BITS))
}

/// A set of states, to read.
#[derive(Debug, Clone, Copy)]
pub struct StateSetRef<'a> {
    words: &'a [Word],
}

impl<'a> StateSetRef<'a> {
    /// The set the bits of `words` hold.
    #[inline]
    pub fn new(words: &'a [Word]) -> Self {
        Self { words }
    }

    /// Its words.
    #[inline]
    pub fn words(self) -> &'a [Word] {
        self.words
    }

    /// Whether it holds `state`.
    #[inline]
    pub fn contains(self, state: StateId) -> bool {
        let (word, bit) = word_and_bit(state);
        self.words.get(word).is_some_and(|&w| w & bit != 0)
    }

    /// Whether it holds no state.
    #[inline]
    pub fn is_empty(self) -> bool {
        self.words.iter().all(|&w| w == 0)
    }

    /// How many states it holds.
    #[inline]
    pub fn len(self) -> usize {
        self.words.iter().map(|w| w.count_ones() as usize).sum()
    }

    /// The first state it holds at `from` or after, in document order.
    pub fn first_from(self, from: StateId) -> Option<StateId> {
        let mut word = from.index() / WORD_BITS;
        let mut bits = *self.words.get(word)? & (Word::MAX << (from.index() % WORD_BITS));
        loop {
            if bits != 0 {
                return Some(StateId(
                    (word * WORD_BITS + bits.trailing_zeros() as usize) as u16,
                ));
            }
            word += 1;
            bits = *self.words.get(word)?;
        }
    }

    /// The last state it holds at `to` or before, in document order.
    pub fn last_to(self, to: StateId) -> Option<StateId> {
        let mut word = to.index() / WORD_BITS;
        let shift = WORD_BITS - 1 - to.index() % WORD_BITS;
        let mut bits = match self.words.get(word) {
            Some(&w) => w & (Word::MAX >> shift),
            None => {
                word = self.words.len().checked_sub(1)?;
                self.words[word]
            }
        };
        loop {
            if bits != 0 {
                let top = WORD_BITS - 1 - bits.leading_zeros() as usize;
                return Some(StateId((word * WORD_BITS + top) as u16));
            }
            word = word.checked_sub(1)?;
            bits = self.words[word];
        }
    }

    /// Whether it holds a state of the range `first..=last`.
    #[inline]
    pub fn meets(self, first: StateId, last: StateId) -> bool {
        self.first_from(first).is_some_and(|state| state <= last)
    }

    /// The last state it holds, in document order.
    #[inline]
    pub fn last(self) -> Option<StateId> {
        self.last_to(StateId(u16::MAX))
    }

    /// Its states in document order, which is entry order.
    #[inline]
    pub fn iter(self) -> Ascending<'a> {
        self.iter_from(StateId(0))
    }

    /// Its states at `from` or after, in document order.
    #[inline]
    pub fn iter_from(self, from: StateId) -> Ascending<'a> {
        Ascending {
            set: self,
            next: Some(from),
        }
    }

    /// Its states in reverse document order, which is exit order.
    #[inline]
    pub fn iter_rev(self) -> Descending<'a> {
        Descending {
            set: self,
            next: StateId(u16::MAX),
            done: false,
        }
    }
}

/// The states of a [`StateSetRef`] in document order.
#[derive(Debug, Clone)]
pub struct Ascending<'a> {
    set: StateSetRef<'a>,
    next: Option<StateId>,
}

impl Iterator for Ascending<'_> {
    type Item = StateId;

    #[inline]
    fn next(&mut self) -> Option<StateId> {
        let found = self.set.first_from(self.next?)?;
        self.next = found.0.checked_add(1).map(StateId);
        Some(found)
    }
}

/// The states of a [`StateSetRef`] in reverse document order.
#[derive(Debug, Clone)]
pub struct Descending<'a> {
    set: StateSetRef<'a>,
    next: StateId,
    done: bool,
}

impl Iterator for Descending<'_> {
    type Item = StateId;

    #[inline]
    fn next(&mut self) -> Option<StateId> {
        if self.done {
            return None;
        }
        let Some(found) = self.set.last_to(self.next) else {
            self.done = true;
            return None;
        };
        match found.0.checked_sub(1) {
            Some(next) => self.next = StateId(next),
            None => self.done = true,
        }
        Some(found)
    }
}

/// A set of states, to read and write.
#[derive(Debug)]
pub struct StateSet<'a> {
    words: &'a mut [Word],
}

impl<'a> StateSet<'a> {
    /// The set the bits of `words` hold.
    #[inline]
    pub fn new(words: &'a mut [Word]) -> Self {
        Self { words }
    }

    /// The set, to read.
    #[inline]
    pub fn as_ref(&self) -> StateSetRef<'_> {
        StateSetRef { words: self.words }
    }

    /// The same set, for a call that returns it.
    #[inline]
    pub fn reborrow(&mut self) -> StateSet<'_> {
        StateSet {
            words: &mut *self.words,
        }
    }

    /// Whether it holds `state`.
    #[inline]
    pub fn contains(&self, state: StateId) -> bool {
        self.as_ref().contains(state)
    }

    /// Add `state`; `false` when it was held already.
    #[inline]
    pub fn insert(&mut self, state: StateId) -> bool {
        let (word, bit) = word_and_bit(state);
        let Some(w) = self.words.get_mut(word) else {
            set_too_small()
        };
        let added = *w & bit == 0;
        *w |= bit;
        added
    }

    /// Remove `state`.
    #[inline]
    pub fn remove(&mut self, state: StateId) {
        let (word, bit) = word_and_bit(state);
        if let Some(w) = self.words.get_mut(word) {
            *w &= !bit;
        }
    }

    /// Remove every state.
    #[inline]
    pub fn clear(&mut self) {
        self.words.fill(0);
    }

    /// Make it hold exactly the states `other` holds.
    #[inline]
    pub fn copy_from(&mut self, other: StateSetRef<'_>) {
        self.words.copy_from_slice(other.words);
    }
}

/// A state past the end of a set: the generator sized the set below the
/// document. No formatted message, so a bare-metal image does not carry the
/// formatting machinery for a branch a correct generator never reaches.
#[cold]
#[inline(never)]
fn set_too_small() -> ! {
    panic!("a state set is full: sce-codegen sized it below the document's states")
}
