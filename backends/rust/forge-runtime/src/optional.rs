// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A value that may be absent (docs/adr/0055), as generated Rust holds it:
//! `core`'s `Option<T>`, absent as `None` and present as `Some(v)`.
//!
//! The type itself needs nothing of this module. What is here is the few
//! operations generated code spells through one function each, so that every
//! machine and algorithm reads, bounds and logs an optional value the same way:
//!
//! - [`proven`]: the value of an optional the build has proven present
//!   (`x.value` under a `x !== null` test). The build refuses every read it
//!   has not proven, so the absent arm is one no run reaches, written once
//!   here rather than at every read.
//! - [`bounded`]: an optional text or byte string held to the bound of the
//!   place it is written to; an absent value has no bytes and always fits.
//! - [`logged`]: the form a `<log>` prints, an absent value as `null`, as
//!   every engine logs one.
//!
//! Everything is `core` only, with no allocator: the bare-metal profile
//! holds an optional value as the standard profile does.

use core::fmt;

use crate::algorithm::AlgorithmError;

/// The value `value` holds, where the build has proven it present.
///
/// A document reads `x.value` only where a test (`x !== null`) proves `x`
/// present, and the build refuses every other read
/// (`expression/optional-read-unproven`), so no run reaches the absent arm:
/// it is unreachable by construction, not by a check made at run time.
#[inline]
#[track_caller]
pub fn proven<T>(value: Option<T>) -> T {
    match value {
        Some(present) => present,
        None => unreachable!(
            "an optional value read where the build proved it present was absent \
             (docs/adr/0055, section 3)"
        ),
    }
}

/// `value` handed back when it is absent, or present and no longer than
/// `capacity` bytes — the bound of the variable or the field it is written
/// to — and a capacity failure otherwise, as
/// [`crate::algorithm::bounded`] answers for a value that is always present.
pub fn bounded<S: AsRef<[u8]>>(
    value: Option<S>,
    capacity: u32,
) -> Result<Option<S>, AlgorithmError> {
    match value {
        Some(present) => crate::algorithm::bounded(present, capacity).map(Some),
        None => Ok(None),
    }
}

/// What a `<log>` prints for an optional value: a present one as its value
/// prints, an absent one as `null`.
pub fn logged<T: fmt::Debug>(value: &Option<T>) -> Logged<'_, T> {
    Logged(value)
}

/// The printed form [`logged`] answers.
pub struct Logged<'a, T>(&'a Option<T>);

impl<T: fmt::Debug> fmt::Debug for Logged<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(present) => present.fmt(f),
            None => f.write_str("null"),
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::{bounded, logged, proven};
    use crate::algorithm::AlgorithmError;
    use std::format;

    #[test]
    fn a_proven_read_is_the_value_held() {
        assert_eq!(proven(Some(12_i64)), 12);
        assert_eq!(proven(Some("soon")), "soon");
    }

    #[test]
    #[should_panic(expected = "proved it present was absent")]
    fn a_proven_read_of_an_absent_value_is_unreachable() {
        proven::<i64>(None);
    }

    #[test]
    fn an_absent_text_fits_every_bound_and_a_present_one_is_held_to_it() {
        assert_eq!(bounded::<&str>(None, 0), Ok(None));
        assert_eq!(bounded(Some("soon"), 4), Ok(Some("soon")));
        assert_eq!(
            bounded(Some("later"), 4),
            Err(AlgorithmError::CapacityExceeded)
        );
    }

    #[test]
    fn an_absent_value_is_logged_as_null() {
        assert_eq!(format!("{:?}", logged::<i64>(&None)), "null");
        assert_eq!(format!("{:?}", logged(&Some(7_i64))), "7");
        assert_eq!(format!("{:?}", logged(&Some("next"))), "\"next\"");
    }
}
