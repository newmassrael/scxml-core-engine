// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Where a machine keeps the `<donedata>` of its top-level `<final>`, chosen by
//! its policy.
//!
//! A child machine that reaches a top-level `<final>` carrying `<donedata>`
//! keeps the evaluated payload until its parent lifts it onto
//! `done.invoke.<id>`. Only such a document writes the slot, and every other
//! machine reads it as the empty string. Under `--no-default-features` the slot
//! is a string of the runtime's bound, so a document with no `<donedata>` on a
//! top-level `<final>` paid for one it can never fill.
//! [`StatePolicy::DoneData`](crate::StatePolicy::DoneData) is therefore the
//! document's choice: [`SceString`] when it has such a `<donedata>`, and the
//! zero-size [`NoDoneData`] when it has none.

use crate::SceString;

/// The slot the payload of a top-level `<final>`'s `<donedata>` waits in for
/// the parent: [`SceString`] when the document has one, [`NoDoneData`] when it
/// has none.
pub trait DoneDataStorage {
    /// The empty slot, as a constant: the engine is built in a constant
    /// expression (`Engine::new_const`) as well as at run time.
    const EMPTY: Self;

    /// Keep `data`, the payload evaluated on entering the top-level `<final>`.
    fn stash(&mut self, data: SceString);

    /// The payload kept, or the empty string when none was.
    fn read(&self) -> &str;
}

impl DoneDataStorage for SceString {
    const EMPTY: Self = Self::new();

    #[inline]
    fn stash(&mut self, data: SceString) {
        *self = data;
    }

    #[inline]
    fn read(&self) -> &str {
        self.as_str()
    }
}

/// The slot of a document none of whose top-level `<final>`s carries
/// `<donedata>`. It holds nothing, is zero-size, and reads as the empty
/// string, which is what a parent lifts onto `done.invoke.<id>` for a child
/// that had no `<donedata>`.
///
/// Keeping a payload in it stops the program, with a message that says the
/// machine keeps no slot. Only the entry of a top-level `<final>` with
/// `<donedata>` keeps one, and the generator chooses this type only for a
/// document that has none, so a generated machine never reaches it; a payload
/// dropped in silence would be data the parent never receives, which nothing
/// could see.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoDoneData;

impl DoneDataStorage for NoDoneData {
    const EMPTY: Self = NoDoneData;

    fn stash(&mut self, _data: SceString) {
        no_donedata_slot()
    }

    #[inline]
    fn read(&self) -> &str {
        ""
    }
}

/// The one way a payload reaches a slot that is not there: stop, and say why.
#[cold]
#[inline(never)]
fn no_donedata_slot() -> ! {
    panic!(
        "a <donedata> payload was kept on a machine whose document has none on a top-level \
         <final>: its policy names `NoDoneData`, which the generator chooses only for such a \
         document, so the machine keeps no slot for one. A policy that keeps one names \
         `SceString`."
    )
}
