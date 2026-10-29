// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a running machine keeps between microsteps that Appendix D's procedures
//! read and write: its configuration and what each `<history>` recorded.
//!
//! Both are sets of states, kept by the engine in a [`RunStore`] of the
//! document's size as bit vectors ([`state_set`](super::state_set)) and read
//! through a [`RunStoreMut`], which is the same type for every machine. So the
//! code that adds a state to a configuration or keeps a history's record is
//! compiled once for every program (docs/adr/0064), and a generated machine
//! keeps neither.

use crate::helpers::state_set::{state_words, StateSet, StateSetRef, Word};
use crate::helpers::tables::{HistoryId, StateId};

/// The words the history records of a document with `states` states and
/// `histories` histories take: one set of its states for each. The generator
/// writes a run store's size as a call of this function, so the rule is
/// stated once.
pub const fn record_words(states: usize, histories: usize) -> usize {
    state_words(states) * histories
}

/// A machine's configuration and history records, of the sizes its document
/// needs: `WORDS` the words of one set of its states
/// ([`state_words`](super::state_set::state_words)), `RECORD_WORDS` the words
/// of its `HISTORIES` histories' records ([`record_words`]). The generated
/// policy spells its own as `StatePolicy::RunStore`.
#[derive(Debug, Clone)]
pub struct RunStore<const WORDS: usize, const RECORD_WORDS: usize, const HISTORIES: usize> {
    configuration: [Word; WORDS],
    records: [Word; RECORD_WORDS],
    recorded: [bool; HISTORIES],
}

impl<const WORDS: usize, const RECORD_WORDS: usize, const HISTORIES: usize>
    RunStore<WORDS, RECORD_WORDS, HISTORIES>
{
    /// An empty configuration and no history ever recorded.
    pub const fn new() -> Self {
        Self {
            configuration: [0; WORDS],
            records: [0; RECORD_WORDS],
            recorded: [false; HISTORIES],
        }
    }
}

impl<const WORDS: usize, const RECORD_WORDS: usize, const HISTORIES: usize> Default
    for RunStore<WORDS, RECORD_WORDS, HISTORIES>
{
    fn default() -> Self {
        Self::new()
    }
}

/// A [`RunStore`] of any size, as the engine reads and writes it.
pub trait RunStorage {
    /// An empty store, as a constant, for an engine built in a constant
    /// expression.
    const EMPTY: Self;

    /// The store, to read.
    fn view(&self) -> RunStoreRef<'_>;

    /// The store, to read and write.
    fn view_mut(&mut self) -> RunStoreMut<'_>;
}

impl<const WORDS: usize, const RECORD_WORDS: usize, const HISTORIES: usize> RunStorage
    for RunStore<WORDS, RECORD_WORDS, HISTORIES>
{
    const EMPTY: Self = Self::new();

    #[inline]
    fn view(&self) -> RunStoreRef<'_> {
        RunStoreRef::new(&self.configuration, &self.records, &self.recorded)
    }

    #[inline]
    fn view_mut(&mut self) -> RunStoreMut<'_> {
        RunStoreMut::new(
            &mut self.configuration,
            &mut self.records,
            &mut self.recorded,
        )
    }
}

/// A [`RunStore`], to read.
#[derive(Debug, Clone, Copy)]
pub struct RunStoreRef<'a> {
    configuration: &'a [Word],
    records: &'a [Word],
    recorded: &'a [bool],
}

impl<'a> RunStoreRef<'a> {
    /// A store kept in slices of the caller's own: the configuration's words,
    /// one set of as many words for each history after another, and whether
    /// each history ever recorded.
    #[inline]
    pub fn new(configuration: &'a [Word], records: &'a [Word], recorded: &'a [bool]) -> Self {
        Self {
            configuration,
            records,
            recorded,
        }
    }

    /// The active states.
    #[inline]
    pub fn configuration(self) -> StateSetRef<'a> {
        StateSetRef::new(self.configuration)
    }

    /// Whether `state` is active.
    #[inline]
    pub fn is_active(self, state: StateId) -> bool {
        self.configuration().contains(state)
    }

    /// What `history` recorded when its parent was last exited; `None` before
    /// that ever happened.
    #[inline]
    pub fn history_value(self, history: HistoryId) -> Option<StateSetRef<'a>> {
        if !*self.recorded.get(history.index())? {
            return None;
        }
        let words = self.configuration.len();
        let start = history.index() * words;
        Some(StateSetRef::new(&self.records[start..start + words]))
    }
}

/// A [`RunStore`], to read and write.
#[derive(Debug)]
pub struct RunStoreMut<'a> {
    configuration: &'a mut [Word],
    records: &'a mut [Word],
    recorded: &'a mut [bool],
}

impl<'a> RunStoreMut<'a> {
    /// A store kept in slices of the caller's own, as [`RunStoreRef::new`]
    /// reads them.
    #[inline]
    pub fn new(
        configuration: &'a mut [Word],
        records: &'a mut [Word],
        recorded: &'a mut [bool],
    ) -> Self {
        Self {
            configuration,
            records,
            recorded,
        }
    }

    /// The store, to read.
    #[inline]
    pub fn as_ref(&self) -> RunStoreRef<'_> {
        RunStoreRef::new(self.configuration, self.records, self.recorded)
    }

    /// The configuration, to change.
    #[inline]
    pub fn configuration(&mut self) -> StateSet<'_> {
        StateSet::new(self.configuration)
    }

    /// Add `state` to the configuration; `false` when it was already there.
    #[inline]
    pub fn add(&mut self, state: StateId) -> bool {
        self.configuration().insert(state)
    }

    /// Remove `state` from the configuration.
    #[inline]
    pub fn remove(&mut self, state: StateId) {
        self.configuration().remove(state);
    }

    /// Make the configuration the states `states` yields.
    pub fn set_configuration(&mut self, states: impl IntoIterator<Item = StateId>) {
        let mut configuration = self.configuration();
        configuration.clear();
        for state in states {
            configuration.insert(state);
        }
    }

    /// `history`'s record, emptied and marked recorded — what the caller adds
    /// to it is what `history` recorded — and the configuration to read what
    /// it records from.
    pub fn record_history(&mut self, history: HistoryId) -> (StateSetRef<'_>, StateSet<'_>) {
        let words = self.configuration.len();
        self.recorded[history.index()] = true;
        let start = history.index() * words;
        let mut record = StateSet::new(&mut self.records[start..start + words]);
        record.clear();
        (StateSetRef::new(self.configuration), record)
    }

    /// Forget what `history` recorded, as before its parent was ever exited.
    #[inline]
    pub fn forget_history(&mut self, history: HistoryId) {
        self.recorded[history.index()] = false;
    }
}
