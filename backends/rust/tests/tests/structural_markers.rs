// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//!
//! `test189` is the queue's other half: W3C 6.2.4 puts a `<send>` on the
//! session's EXTERNAL queue unless it names `#_internal`, which is the internal
//! queue a `<raise>` uses. So of its two sends only the one with no target is
//! drivable from outside.

use sce_rust_tests::generated::test189::Test189Event;
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

/// A `<send>` reaches the internal queue only through `#_internal`, so of
/// `test189`'s two sends `event1` (`target="#_internal"`) is an owned internal
/// signal and `event2` (no target: the external queue) is not. Before the
/// parser recorded a send to `#_internal` with the `<raise>`s, both were
/// members, and `event1` was forgeable from outside in exactly the way a
/// `<raise>`d event is not.
#[test]
fn a_send_to_the_internal_queue_is_not_drivable_from_outside_and_a_plain_send_is() {
    assert_eq!(
        Test189Event::EXTERNALLY_DRIVABLE_EVENTS,
        [Test189Event::Event2].as_slice(),
    );
    assert!(!Test189Event::EXTERNALLY_DRIVABLE_EVENTS.contains(&Test189Event::Event1));
}
