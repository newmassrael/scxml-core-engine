// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a host does with the duties of a machine (docs/adr/0038).
//!
//! A `<sce:action effect="duty">` is an operation the machine's correctness
//! depends on the host performing. The generated host interface of a machine
//! that states one carries a [`ActionRealization`] with no default, so the
//! author of a host says beside the code whether it performs them or only
//! simulates them (a recorder, a test double), and the machine refuses a host
//! that simulates unless it is built for one on purpose.

/// What a host does with a machine's duties.
///
/// Declared by the host, in its own type, as `const REALIZATION`. `Performs` is
/// a statement the host's author makes about the body of its methods, in a line
/// a reviewer can find; SCE cannot read the bodies, and an empty body under
/// `Performs` is that author's false statement and not an unwritten fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionRealization {
    /// The host performs the duties: its methods do what the operations'
    /// names say.
    Performs,
    /// The host does not perform them: it records, drops or fakes them. A
    /// machine with a duty refuses such a host unless it is built with
    /// `new_simulated`.
    Simulates,
}

impl ActionRealization {
    /// Whether this is [`ActionRealization::Performs`]. A `const fn`, so a
    /// generated machine can refuse a simulating host while it compiles.
    pub const fn is_performing(self) -> bool {
        matches!(self, Self::Performs)
    }
}
