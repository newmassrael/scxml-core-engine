// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a running machine keeps between microsteps that Appendix D's procedures
//! read and write: its configuration and what each `<history>` recorded.
//!
//! Both are numbers ([`StateId`]), kept by the engine in a [`RunStore`] of the
//! document's size and read through a [`RunStoreMut`], which is the same type
//! for every machine. So the code that adds a state to a configuration or keeps
//! a history's record is compiled once for every program (docs/adr/0064), and a
//! generated machine keeps neither.

use crate::helpers::tables::{DocumentTables, HistoryId, StateId};

/// A machine's configuration and history records, of the sizes its document
/// needs: `CONFIGURATION` states at once, `RECORDS` states over all its
/// histories' records, `HISTORIES` histories. The generated policy spells its
/// own as `StatePolicy::RunStore`.
#[derive(Debug, Clone)]
pub struct RunStore<const CONFIGURATION: usize, const RECORDS: usize, const HISTORIES: usize> {
    configuration: [StateId; CONFIGURATION],
    configuration_len: u16,
    records: [StateId; RECORDS],
    record_lens: [Option<u16>; HISTORIES],
}

impl<const CONFIGURATION: usize, const RECORDS: usize, const HISTORIES: usize>
    RunStore<CONFIGURATION, RECORDS, HISTORIES>
{
    /// An empty configuration and no history ever recorded.
    pub const fn new() -> Self {
        Self {
            configuration: [StateId(0); CONFIGURATION],
            configuration_len: 0,
            records: [StateId(0); RECORDS],
            record_lens: [None; HISTORIES],
        }
    }
}

impl<const CONFIGURATION: usize, const RECORDS: usize, const HISTORIES: usize> Default
    for RunStore<CONFIGURATION, RECORDS, HISTORIES>
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

impl<const CONFIGURATION: usize, const RECORDS: usize, const HISTORIES: usize> RunStorage
    for RunStore<CONFIGURATION, RECORDS, HISTORIES>
{
    const EMPTY: Self = Self::new();

    #[inline]
    fn view(&self) -> RunStoreRef<'_> {
        RunStoreRef {
            configuration: &self.configuration[..usize::from(self.configuration_len)],
            records: &self.records,
            record_lens: &self.record_lens,
        }
    }

    #[inline]
    fn view_mut(&mut self) -> RunStoreMut<'_> {
        RunStoreMut {
            configuration: &mut self.configuration,
            configuration_len: &mut self.configuration_len,
            records: &mut self.records,
            record_lens: &mut self.record_lens,
        }
    }
}

/// A [`RunStore`], to read.
#[derive(Debug, Clone, Copy)]
pub struct RunStoreRef<'a> {
    configuration: &'a [StateId],
    records: &'a [StateId],
    record_lens: &'a [Option<u16>],
}

impl<'a> RunStoreRef<'a> {
    /// A store kept in slices of the caller's own: the active states, the
    /// record slots of every history at the spans the tables give them, and
    /// how many states each history recorded, `None` before it ever did.
    pub fn new(
        configuration: &'a [StateId],
        records: &'a [StateId],
        record_lens: &'a [Option<u16>],
    ) -> Self {
        Self {
            configuration,
            records,
            record_lens,
        }
    }

    /// The active states, in the order they were added.
    #[inline]
    pub fn configuration(self) -> &'a [StateId] {
        self.configuration
    }

    /// Whether `state` is active.
    #[inline]
    pub fn is_active(self, state: StateId) -> bool {
        self.configuration.contains(&state)
    }

    /// What `history` recorded when its parent was last exited; `None` before
    /// that ever happened.
    pub fn history_value(
        self,
        tables: &DocumentTables,
        history: HistoryId,
    ) -> Option<&'a [StateId]> {
        let len = (*self.record_lens.get(history.index())?)?;
        let start = usize::from(tables.history(history).record.start);
        Some(&self.records[start..start + usize::from(len)])
    }
}

/// A [`RunStore`], to read and write.
#[derive(Debug)]
pub struct RunStoreMut<'a> {
    configuration: &'a mut [StateId],
    configuration_len: &'a mut u16,
    records: &'a mut [StateId],
    record_lens: &'a mut [Option<u16>],
}

impl<'a> RunStoreMut<'a> {
    /// A store kept in slices of the caller's own, as [`RunStoreRef::new`]
    /// reads them; the configuration is the first `configuration_len` slots of
    /// `configuration`.
    pub fn new(
        configuration: &'a mut [StateId],
        configuration_len: &'a mut u16,
        records: &'a mut [StateId],
        record_lens: &'a mut [Option<u16>],
    ) -> Self {
        Self {
            configuration,
            configuration_len,
            records,
            record_lens,
        }
    }

    /// The store, to read.
    #[inline]
    pub fn as_ref(&self) -> RunStoreRef<'_> {
        RunStoreRef {
            configuration: &self.configuration[..usize::from(*self.configuration_len)],
            records: self.records,
            record_lens: self.record_lens,
        }
    }

    /// Add `state` to the configuration; `false` when it was already there.
    pub fn add(&mut self, state: StateId) -> bool {
        if self.as_ref().is_active(state) {
            return false;
        }
        let len = usize::from(*self.configuration_len);
        let Some(slot) = self.configuration.get_mut(len) else {
            store_too_small()
        };
        *slot = state;
        *self.configuration_len += 1;
        true
    }

    /// Remove `state` from the configuration, keeping the order of the rest.
    pub fn remove(&mut self, state: StateId) {
        let len = usize::from(*self.configuration_len);
        if let Some(at) = self.configuration[..len].iter().position(|&s| s == state) {
            self.configuration.copy_within(at + 1..len, at);
            *self.configuration_len -= 1;
        }
    }

    /// Make the configuration `states`, in their order.
    pub fn set_configuration(&mut self, states: &[StateId]) {
        *self.configuration_len = 0;
        for &state in states {
            self.add(state);
        }
    }

    /// Keep `states` as what `history` recorded.
    pub fn record_history(
        &mut self,
        tables: &DocumentTables,
        history: HistoryId,
        states: &[StateId],
    ) {
        self.record_history_from(tables, history, states.iter().copied());
    }

    /// Keep the states `states` yields as what `history` recorded.
    pub fn record_history_from(
        &mut self,
        tables: &DocumentTables,
        history: HistoryId,
        states: impl IntoIterator<Item = StateId>,
    ) {
        let span = tables.history(history).record;
        let slots = &mut self.records[usize::from(span.start)..][..usize::from(span.len)];
        let mut len = 0;
        for state in states {
            let Some(slot) = slots.get_mut(len) else {
                store_too_small()
            };
            *slot = state;
            len += 1;
        }
        self.record_lens[history.index()] = Some(len as u16);
    }

    /// Forget what `history` recorded, as before its parent was ever exited.
    pub fn forget_history(&mut self, history: HistoryId) {
        self.record_lens[history.index()] = None;
    }
}

/// A configuration or a history record past the room the generator gave it.
#[cold]
#[inline(never)]
fn store_too_small() -> ! {
    panic!("a machine's run store is full: sce-codegen sized it too small for its document")
}
