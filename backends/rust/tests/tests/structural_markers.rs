// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! Runtime proof for the statechart structural markers (SCE-002):
//! `{Machine}State`'s `Default` / `#[default]` and the
//! `{Machine}Event::EXTERNALLY_DRIVABLE_EVENTS` associated const.
//!
//! The `rust_derive_ssot` tests string-check the *emitted* markers and
//! the full-suite compile proves they *compile*; this drives them at
//! RUNTIME so the byte-golden trap (a string that looks right but
//! behaves wrong, `feedback_byte_goldens_not_compile`) cannot hide.
//!
//! `test399` is a deliberately adversarial fixture: it `<raise>`s
//! `foo` / `bar` / `foos` / `foo.zoo`, wildcard-matches `foo.*` and `*`,
//! and `<send>`s + triggers `timeout`. So the only externally-drivable
//! event is `Timeout` — one machine that exercises raise-exclusion,
//! wildcard-exclusion, AND send-inclusion (a `<send>` event, unlike a
//! `<raise>`, is legitimately external).

use sce_rust_tests::generated::test399::{Test399Event, Test399State};

/// `<scxml initial="s0">` with `s0 initial="s01"` resolves to the deep
/// initial `S01`; `State::default()` must be that same state (the marker
/// and `initial_state()` share one computation).
#[test]
fn state_default_is_the_initial_state() {
    assert_eq!(Test399State::default(), Test399State::S01);
}

/// The drivable const holds only non-raised concrete triggers. `timeout`
/// (a `<send>` + `<transition>` event) is the sole member; every
/// `<raise>`d event and wildcard descriptor is excluded.
#[test]
fn externally_drivable_const_holds_only_non_raised_concrete_triggers() {
    assert_eq!(
        Test399Event::EXTERNALLY_DRIVABLE_EVENTS,
        [Test399Event::Timeout].as_slice(),
    );
    // Membership is the exact shape a name-parsing consumer keys off.
    assert!(Test399Event::EXTERNALLY_DRIVABLE_EVENTS.contains(&Test399Event::Timeout));
    // `Foo` is `<raise>`d (internal signal) → never externally forgeable.
    assert!(!Test399Event::EXTERNALLY_DRIVABLE_EVENTS.contains(&Test399Event::Foo));
    // The eventless `Null` sentinel is never a member.
    assert!(!Test399Event::EXTERNALLY_DRIVABLE_EVENTS.contains(&Test399Event::Null));
}
