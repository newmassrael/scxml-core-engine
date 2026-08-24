// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D: a `<parallel>` is not a transition domain -- Rust AOT
// path.
//
// `getTransitionDomain` sends an external transition to `findLCCA`, which
// filters the proper ancestors with `isCompoundStateOrScxmlElement`. A
// `<parallel>` is neither, so an external transition written on a REGION ROOT
// has the document root as its domain: every region exits and re-enters, and a
// sibling region's transition on the same event is preempted because the two
// exit sets intersect and the sibling's source is not a descendant of this
// one's.
//
// The engine answered the enclosing `<parallel>` here instead, because both
// the runtime's `handle_hierarchical_transition` and the generated conflict
// resolver asked for a plain lowest-common-ancestor -- the first common
// ancestor, whatever its kind. That is the `findLCA` the appendix
// distinguishes from `findLCCA`, and the difference is invisible until a
// `<parallel>` sits between the source and the first compound `<state>` above
// it, which is exactly a region root.
//
// Measured 2026-08-25 on `examples/ai_loop/ai_loop.scxml`: the Kotlin engine,
// the only one implementing the filter, ended `session.lost` in
// `[alive, restarting]` where C++, Rust and Go ended in
// `[rebuilding, restarting]`. That document was then repaired to say
// `type="internal"`, which is what its three region-root transitions meant --
// and that repair is why this fixture exists rather than the ai_loop suite:
// with the document fixed, no committed document reaches the external form.
//
// Sibling of the C++ drivers `ParallelRegionRootExternalDomainTest.cpp`
// (Interpreter) and `ParallelRegionRootExternalDomainAotTest.cpp` (AOT), which
// pin the same two clauses against the same document.
//
// Fixture: tests/integration/parallel_region_root_external_domain.scxml
// (not under `integration_resources/`: a stem there is a seven-channel
// contract enforced by `integration_stem_registration.rs`, and the Go engine
// has not been repaired yet.)
//
// Regeneration (after fixture or template edit):
//   scripts/regen_parallel_region_root_external_domain.sh

use sce_rust_tests::integration::parallel_region_root_external_domain::{
    ParallelRegionRootExternalDomainEvent as Event,
    ParallelRegionRootExternalDomainPolicy as Policy,
    ParallelRegionRootExternalDomainState as State,
};

/// The whole configuration, sorted, rather than a handful of membership
/// questions.
///
/// The way this defect presents is an ILLEGAL configuration -- two children of
/// the same compound region active at once -- and every individual
/// "is this state active" call answers `true` to that.
fn configuration(engine: &sce_rust_runtime::Engine<Policy>) -> Vec<String> {
    let mut names: Vec<String> = engine
        .get_active_states()
        .iter()
        .map(|s| format!("{s:?}"))
        .collect();
    names.sort();
    names
}

fn started() -> sce_rust_runtime::Engine<Policy> {
    let mut engine = sce_rust_runtime::Engine::new(Policy::default());
    engine.initialize();
    engine
}

// The clause itself.
#[test]
fn an_external_region_root_transition_exits_every_region() {
    let mut engine = started();
    assert_eq!(
        configuration(&engine),
        vec!["Alive", "Drive", "Run", "Watch", "Working"],
        "precondition: the fixture is supposed to start with both regions at \
         their defaults, so nothing below is testing what it claims"
    );

    engine.raise_external(Event::Restart, "", "");
    engine.step();

    assert_eq!(
        configuration(&engine),
        vec!["Alive", "Drive", "Restarting", "Run", "Watch"],
        "an external transition on a region root has the DOCUMENT ROOT as its \
         domain (Appendix D `findLCCA` filters `<parallel>` out of the \
         candidate ancestors), so every region exits and re-enters, `watch` is \
         back at its default, and `watch`'s own transition on the same event \
         is preempted as conflicting"
    );
}

// The contrast, and the reason the ai_loop document is spelled the way it is.
// A test that only pinned the external case would pass just as well on an
// engine that sent EVERY region-root transition to the document root.
#[test]
fn an_internal_region_root_transition_leaves_the_other_region() {
    let mut engine = started();

    engine.raise_external(Event::Hold, "", "");
    engine.step();

    assert_eq!(
        configuration(&engine),
        vec!["Drive", "Paused", "Rebuilding", "Run", "Watch"],
        "an internal region-root transition has the region as its domain \
         (source compound, target its descendant), so the sibling region never \
         exits and answers the event itself"
    );
}

// Guards the enum spelling the two assertions above depend on: a renamed
// variant would otherwise turn a real divergence into a passing string
// comparison against the new name.
#[test]
fn the_states_the_assertions_name_are_the_documents_states() {
    for (state, spelled) in [
        (State::Alive, "Alive"),
        (State::Drive, "Drive"),
        (State::Paused, "Paused"),
        (State::Rebuilding, "Rebuilding"),
        (State::Restarting, "Restarting"),
        (State::Run, "Run"),
        (State::Watch, "Watch"),
        (State::Working, "Working"),
    ] {
        assert_eq!(format!("{state:?}"), spelled);
    }
}
