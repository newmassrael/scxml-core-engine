// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The read of a union's case that a test of its tag proved (docs/adr/0092,
//! section 6.1), as generated Rust spells it.
//!
//! A union's unit answers each case that holds a record by a total view,
//! `as_<case>(&self) -> Option<&R>`, `None` in every other case, so the unit
//! holds no panic and a crate that denies the panic-free lints compiles it as
//! it is. A generated body reads a field of a case only where a test of the
//! tag proves the value is in that case (`expression/union-read-unproven`
//! refuses every other read), and reads it through [`proven`] of the case's
//! view: the arm of the other cases is one no run reaches, written once here
//! rather than in every unit, as a proven optional is read through
//! [`crate::optional::proven`].
//!
//! Everything is `core` only, with no allocator.

/// The record the view `case` holds, where a test of the union's tag proved the
/// value is in that case.
///
/// The build refuses a read of a field of a case that no test proves
/// (`expression/union-read-unproven`), so no run reaches the other arm: it is
/// unreachable by construction, not by a check made at run time.
#[inline]
#[track_caller]
pub fn proven<T>(case: Option<T>) -> T {
    match case {
        Some(record) => record,
        None => unreachable!(
            "a union read as the case a test of its tag proved was in another case \
             (docs/adr/0092, section 6.1)"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::proven;

    #[test]
    fn a_proven_case_is_the_record_its_view_holds() {
        let radius = 7_u16;
        assert_eq!(*proven(Some(&radius)), 7);
        assert_eq!(proven(Some(3_i64)), 3);
    }

    #[test]
    #[should_panic(expected = "proved was in another case")]
    fn a_proven_read_of_another_case_is_unreachable() {
        proven::<&u16>(None);
    }
}
