// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! What a run tells the observer of each transition it takes (docs/adr/0089).
//!
//! A transition is named by the pair the emitted transition table names its
//! rows by (`sce:transition-table="emit"`, docs/SCE_TRANSITION_TABLE.md): the
//! state it is written in and its position among that state's own transitions
//! in document order. The set of pairs a run is told is therefore a set of rows
//! of the table, and the share of the table it holds is the run's transition
//! coverage, measured on the engine that ran and not predicted from the
//! document.
//!
//! The observer is told after the selection and before the microstep's first
//! exit, each transition of the optimal enabled set once and in selection order,
//! which is where the Python engine's `set_observer` tells its `"transition"`.
//! It reads and is never asked anything: a machine with one set takes the same
//! transitions and reaches the same configurations as one without. It runs on
//! the thread that drives the engine, inside the macrostep, and must not call
//! back into the engine; in the `std` profile it is a `'static + Send` closure,
//! which cannot hold the `&mut Engine` such a call would need.

/// One transition the engine took, as its row of the emitted transition table
/// names it.
///
/// `S` is the machine's own state type, the type the table's `from` holds, so a
/// pair compares with a row (`(row.from, row.index)`) without a conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransitionTaken<S> {
    /// The state the transition is written in: the `from` of its row.
    pub source: S,
    /// Its position among `source`'s own transitions in document order: the
    /// `index` of its row.
    pub index: u32,
}

/// What the engine holds for a set observer: any `FnMut + Send` closure in the
/// `std` profile, which boxes it.
#[cfg(feature = "std")]
pub(crate) type Observer<S> = Box<dyn FnMut(TransitionTaken<S>) + Send>;

/// What the engine holds for a set observer: a `fn` pointer in the bare-metal
/// profile, which has no allocator to box a closure in.
#[cfg(not(feature = "std"))]
pub(crate) type Observer<S> = fn(TransitionTaken<S>);
