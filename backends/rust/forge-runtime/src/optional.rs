// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A value that may be absent (docs/adr/0055), as generated Rust holds it:
//! `core`'s `Option<T>`, absent as `None` and present as `Some(v)`.
//!
//! The type itself needs nothing of this module. What is here is the few
//! operations generated code spells through one function each, so that every
//! machine and algorithm reads and logs an optional value the same way:
//!
//! - [`proven`]: the value of an optional the build has proven present
//!   (`x.value` under a `x !== null` test). The build refuses every read it
//!   has not proven, so the absent arm is one no run reaches, written once
//!   here rather than at every read.
//! - [`proven_ref`] and [`proven_mut`]: the same read of a record held in an
//!   optional (`x.value.f`, docs/adr/0055, step 9), borrowed and not moved out:
//!   a record that owns text is not `Copy`, and the optional stays where it is
//!   held, so a field is read through [`proven_ref`] and written through
//!   [`proven_mut`].
//! - [`logged`]: the form a `<log>` prints, an absent value as `null`, as
//!   every engine logs one.
//!
//! Everything is `core` only, with no allocator: the bare-metal profile
//! holds an optional value as the standard profile does.

use core::fmt;

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

/// The value `value` holds, borrowed, where the build has proven it present:
/// [`proven`] for an optional whose present value is a record, which a read
/// takes a field of and never moves out of the place that holds it.
///
/// The build refuses a read of `x.value.f` that no test proves
/// (`expression/optional-read-unproven`), so the absent arm is one no run
/// reaches.
#[inline]
#[track_caller]
pub fn proven_ref<T>(value: &Option<T>) -> &T {
    match value {
        Some(present) => present,
        None => unreachable!(
            "an optional record read where the build proved it present was absent \
             (docs/adr/0055, section 3)"
        ),
    }
}

/// The value `value` holds, borrowed mutably, where the build has proven it
/// present: what a write of a field of the present record goes through
/// (`x.value.f = e`). A field is written only where a test proves the record
/// present (`expression/unsupported-construct`, "a write to … where no test
/// proves … present"), so the absent arm is one no run reaches.
#[inline]
#[track_caller]
pub fn proven_mut<T>(value: &mut Option<T>) -> &mut T {
    match value {
        Some(present) => present,
        None => unreachable!(
            "an optional record written where the build proved it present was absent \
             (docs/adr/0055, section 3)"
        ),
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

    use super::{logged, proven, proven_mut, proven_ref};
    use std::format;
    use std::string::{String, ToString};

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

    /// A record that owns text, as a machine holds one: not `Copy`.
    #[derive(Debug, Clone, PartialEq)]
    struct Item {
        title: String,
        count: i32,
    }

    #[test]
    fn a_proven_record_is_read_in_place_and_not_moved_out() {
        let held = Some(Item {
            title: "alpha".to_string(),
            count: 2,
        });
        assert_eq!(proven_ref(&held).count, 2);
        assert_eq!(proven_ref(&held).title, "alpha");
        // Read twice, and still held: the read borrowed it.
        assert_eq!(proven_ref(&held).count, 2);
        assert!(held.is_some());
    }

    #[test]
    fn a_field_of_a_proven_record_is_written_in_place() {
        let mut held = Some(Item {
            title: "alpha".to_string(),
            count: 2,
        });
        proven_mut(&mut held).count = 7;
        proven_mut(&mut held).title = "bumped".to_string();
        assert_eq!(
            held,
            Some(Item {
                title: "bumped".to_string(),
                count: 7
            })
        );
    }

    #[test]
    #[should_panic(expected = "read where the build proved it present was absent")]
    fn a_proven_read_of_an_absent_record_is_unreachable() {
        proven_ref::<Item>(&None);
    }

    #[test]
    #[should_panic(expected = "written where the build proved it present was absent")]
    fn a_proven_write_of_an_absent_record_is_unreachable() {
        let mut held: Option<Item> = None;
        proven_mut(&mut held).count = 1;
    }

    #[test]
    fn an_absent_value_is_logged_as_null() {
        assert_eq!(format!("{:?}", logged::<i64>(&None)), "null");
        assert_eq!(format!("{:?}", logged(&Some(7_i64))), "7");
        assert_eq!(format!("{:?}", logged(&Some("next"))), "\"next\"");
    }
}
