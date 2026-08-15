// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.3 + Appendix D: a state entered only because the target lies
// inside it takes no default child — Rust AOT path.
//
// Appendix D asks two different questions with two different functions.
// `addDescendantStatesToEnter` gives a compound state its default child and is
// called for the transition's TARGET; `addAncestorStatesToEnter` walks the
// states between the target and the LCCA and adds them WITHOUT defaults,
// because the entry set already holds a descendant of each. Its one exception
// is a parallel ancestor, whose other regions do get theirs.
//
// Measured 2026-08-15 on the worked example `examples/ai_loop/ai_loop.scxml`,
// which is where this was found: answering a dialog from `paused` targets
// `judging`, and the configuration came back holding `priming` as well — whose
// `<onentry>` sends the opening prompt. The supervised session was
// re-introduced to itself every time a person answered. Both AOT engines,
// every W3C fixture green.
//
// The document is driven twice on purpose. `cross` enters the `<parallel>`
// itself, so `run` is a parallel ancestor and `drive`/`outer` are compound
// ones; `again` runs with the parallel already active, so only `outer` is
// entered. Those are different branches of the generated entry walk.
//
// Fixture: integration_resources/ancestor_entry_is_not_default_entry/ancestor_entry_is_not_default_entry.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_ancestor_entry_is_not_default_entry.sh

use sce_rust_tests::integration::ancestor_entry_is_not_default_entry::{
    AncestorEntryIsNotDefaultEntryEvent as Event, AncestorEntryIsNotDefaultEntryPolicy as Policy,
    AncestorEntryIsNotDefaultEntryState as State,
};

fn engine() -> sce_rust_runtime::Engine<Policy> {
    // The fixture counts entries with `<assign>`, so the policy takes an
    // engine — the same injection the other scripted integration fixtures use
    // on this channel.
    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> =
        std::sync::Arc::new(sce_rust_lua::LuaEngine::new());
    let mut e = sce_rust_runtime::Engine::new(Policy::new(script_engine));
    e.initialize();
    e
}

fn send(e: &mut sce_rust_runtime::Engine<Policy>, ev: Event) {
    e.raise_external(ev, "", "");
    e.step();
}

#[test]
fn an_ancestor_entered_on_the_way_to_a_target_takes_no_default_child() {
    let mut e = engine();

    let entry = e.get_active_states();
    assert!(
        entry.contains(&State::Away),
        "the run has to start OUTSIDE the `<parallel>` for the first pass to be testing \
         anything — a source already inside it leaves the ancestors active and the entry \
         chain never reaches their defaults; it came up as {entry:?}"
    );

    // Pass one: the parallel is not active, so `run` is entered as a parallel
    // ancestor and `drive` and `outer` as compound ones.
    send(&mut e, Event::Cross);

    let crossed = e.get_active_states();
    assert!(
        crossed.contains(&State::Chosen),
        "the transition named `chosen` and the machine did not enter it (active: {crossed:?})"
    );
    assert!(
        !crossed.contains(&State::ByDefault),
        "⚠ `outer` has two children active at once (active: {crossed:?}). `by_default` is what \
         `initial` names, and nothing targeted it — it was entered because the engine gave \
         `outer` its default child while entering `outer` merely as an ancestor of `chosen`"
    );
    assert!(
        crossed.contains(&State::Idle),
        "the region no entering state is inside must still be entered with its default \
         (active: {crossed:?}) — Appendix D's one exception for a parallel ancestor"
    );

    // Pass two: the parallel is already active now, so `run` and `drive` are
    // skipped and only `outer` is entered. That is a different branch of the
    // entry walk, and it is the one a running machine takes.
    send(&mut e, Event::Back);
    send(&mut e, Event::Again);

    let again = e.get_active_states();
    assert!(
        !again.contains(&State::ByDefault),
        "⚠ `outer` took its default child on the second pass (active: {again:?}), where the \
         `<parallel>` was already active and only `outer` itself was entered — the shape the \
         worked example hits every time a person answers a dialog"
    );

    send(&mut e, Event::Check);

    let settled = e.get_active_states();
    assert!(
        settled.contains(&State::Settled),
        "`check` did not carry the machine to `settled` (active: {settled:?}). The document \
         checks its four clauses in document order and lands each in a `<final>` of its own, \
         so the configuration above names which one broke: `failDefaulted` (a default nobody \
         targeted), `failLobbied` (`drive`'s default taken while it was only an ancestor), \
         `failIdled` (the untouched region did not get its default, or got it twice), \
         `failTargeted` (a pass never reached the target)"
    );
}
