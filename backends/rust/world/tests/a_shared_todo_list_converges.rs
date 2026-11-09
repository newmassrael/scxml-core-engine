// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A shared to-do list: three people, `alice`, `bob` and `carol`, each with a copy (the
// fixture `todo_peer.scxml`), every copy a member of one group its changes go to and come
// from (`#people`, `delivery: causal`, docs/adr/0085 section 3.2). They change each other's
// to-dos at the same time — one retitles what another ticks off — on links that delay,
// reorder, repeat and cut what they carry, and the question is the one a shared list
// answers: do the three copies end with the same list, and is it the list the changes meant.
//
// A to-do is a row of fields and each field is its own register, so what is held here
// that a sheet of cells does not show is that two changes to DIFFERENT fields of one
// to-do both stay, where a to-do kept whole would keep one and lose the other.
//
// Equality of the copies says they agree, not that they are right, so each run is also
// held to an oracle that does not read the copies: of every write a change made, the ones no
// change wrote over are the field's values, and the latest is the one a reader takes.

mod common;

// Named here and not among what `common` hands every test: a test file that did not
// use it would be told it imports something it does not use.
use common::todo_wide_sender::TodoWideSenderPolicy as WideSender;
use common::*;
use sce_rust_world::Outcome;

#[test]
fn changes_to_different_fields_of_one_todo_are_both_kept() {
    let mut world = todo_world(1, 20);
    todo_title(&mut world, "alice", 7, "buy milk");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);

    // At the same moment: alice retitles it, bob ticks it off and carol gives it a day.
    todo_title(&mut world, "alice", 7, "buy oat milk");
    todo_set(&mut world, "bob", 7, DONE, 1);
    todo_set(&mut world, "carol", 7, DUE, 20_000);
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);

    let list = agreed_todos(&mut world);
    assert_eq!(list[&(7, TITLE)].text.as_deref(), Some("buy oat milk"));
    assert_eq!(
        list[&(7, DONE)].value,
        1,
        "bob's tick survived alice's retitle"
    );
    assert_eq!(list[&(7, DUE)].value, 20_000);
    assert_eq!(list, todo_oracle(&world));
    assert!(world.host_errors().is_empty());
}

/// Two people retitle a to-do at the same moment, neither having seen the other's: both
/// titles stay, on every copy, and a reader that needs one takes the later stamp.
#[test]
fn two_titles_written_at_the_same_instant_are_both_kept_everywhere() {
    let mut world = todo_world(1, 20);
    todo_title(&mut world, "alice", 3, "call dentist");
    todo_title(&mut world, "bob", 3, "call the dentist");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);

    let list = agreed_todos(&mut world);
    let title = &list[&(3, TITLE)];
    let texts: Vec<_> = title.live.iter().map(|held| held.text.as_deref()).collect();
    assert_eq!(
        texts.len(),
        2,
        "neither saw the other's title, so neither wrote over it: {texts:?}"
    );
    assert!(texts.contains(&Some("call dentist")));
    assert!(texts.contains(&Some("call the dentist")));
    // The stamps tie on time and counter, so the node id decides which a single reader
    // takes: bob's. And the text is the text that write carried, not the other's.
    assert_eq!(title.text.as_deref(), Some("call the dentist"));
    assert_eq!(list, todo_oracle(&world));
}

/// A person who has seen both titles and writes a third writes over both of them.
#[test]
fn a_title_written_after_seeing_both_replaces_both() {
    let mut world = todo_world(1, 20);
    todo_title(&mut world, "alice", 3, "call dentist");
    todo_title(&mut world, "bob", 3, "call the dentist");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    assert_eq!(agreed_todos(&mut world)[&(3, TITLE)].live.len(), 2);

    todo_title(&mut world, "carol", 3, "call Dr Han");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);

    let list = agreed_todos(&mut world);
    let title = &list[&(3, TITLE)];
    assert_eq!(title.live.len(), 1, "both were seen, so both were replaced");
    assert_eq!(title.text.as_deref(), Some("call Dr Han"));
    assert_eq!(list, todo_oracle(&world));
}

/// Changes made at the same moment on two sides of a cut are both kept when the links are
/// back, however long each side was apart: the values nobody wrote over stay.
#[test]
fn values_changed_on_both_sides_of_a_cut_link_are_both_kept() {
    let mut world = todo_world(5, 20);
    todo_set(&mut world, "alice", 1, DUE, 10);
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    world.partition("alice", "bob");
    world.partition("alice", "carol");
    todo_set(&mut world, "alice", 1, DUE, 20);
    todo_set(&mut world, "bob", 1, DUE, 30);
    world.advance_ms(500);
    assert_eq!(todos_of(&mut world, "alice")[&(1, DUE)].value, 20);

    world.heal("alice", "bob");
    world.heal("alice", "carol");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);

    let list = agreed_todos(&mut world);
    let mut values: Vec<_> = list[&(1, DUE)].live.iter().map(|held| held.value).collect();
    values.sort_unstable();
    assert_eq!(
        values,
        [20, 30],
        "the first value was written over by both, and the two writes did not see each other"
    );
    assert_eq!(list, todo_oracle(&world));
    assert!(world.host_errors().is_empty());
}

/// A person who has read a title and types over it means to replace it, even when their
/// own clock is behind the one that wrote it.
#[test]
fn a_title_written_after_seeing_another_wins_whatever_the_clock_says() {
    let mut world = todo_world(1, 20);
    todo_edit_at(&mut world, "alice", 1, TITLE, 0, "draft", 1_000);
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    assert_eq!(
        todos_of(&mut world, "bob")[&(1, TITLE)].text.as_deref(),
        Some("draft"),
        "bob saw the title"
    );

    todo_edit_at(&mut world, "bob", 1, TITLE, 0, "final", 10);
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);

    let list = agreed_todos(&mut world);
    assert_eq!(list[&(1, TITLE)].text.as_deref(), Some("final"));
    assert_eq!(list, todo_oracle(&world));
}

#[test]
fn changes_made_offline_are_merged_when_the_link_is_back() {
    let mut world = todo_world(5, 20);
    world.partition("alice", "bob");
    world.partition("alice", "carol");
    todo_title(&mut world, "alice", 1, "pack bags");
    todo_set(&mut world, "alice", 1, DUE, 100);
    world.advance_ms(1_000);
    // Bob, meanwhile, ticks off the same to-do and writes another.
    todo_set(&mut world, "bob", 1, DONE, 1);
    todo_title(&mut world, "bob", 2, "book taxi");
    world.advance_ms(500);
    // Alice holds only her own while she is cut off.
    assert_eq!(todos_of(&mut world, "alice").len(), 2);

    world.heal("alice", "bob");
    world.heal("alice", "carol");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);

    let list = agreed_todos(&mut world);
    assert_eq!(list.len(), 4);
    assert_eq!(list[&(1, TITLE)].text.as_deref(), Some("pack bags"));
    assert_eq!(list[&(1, DUE)].value, 100);
    assert_eq!(list[&(1, DONE)].value, 1);
    assert_eq!(list[&(2, TITLE)].text.as_deref(), Some("book taxi"));
    assert_eq!(list, todo_oracle(&world));
    assert!(world.host_errors().is_empty());
}

/// A title is counted in UTF-8 bytes and held to the 32 its schema declares, whatever
/// the script: eleven Hangul syllables are 33 bytes and do not fit, ten are 30 and do.
#[test]
fn a_title_in_another_script_is_held_to_its_bytes_and_kept_whole() {
    let fits = "\u{d560}\u{c77c}".repeat(5);
    assert_eq!(fits.len(), 30);
    let too_long = "\u{d560}".repeat(11);
    assert_eq!(too_long.len(), 33);

    let mut world = todo_world(1, 20);
    todo_title(&mut world, "alice", 1, &fits);
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    todo_title(&mut world, "bob", 2, &too_long);
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);

    let list = agreed_todos(&mut world);
    assert_eq!(list[&(1, TITLE)].text.as_deref(), Some(fits.as_str()));
    assert!(
        !list.contains_key(&(2, TITLE)),
        "a title past its bound is refused whole and writes nothing"
    );
    assert_eq!(list, todo_oracle(&world));
    assert_eq!(
        todo_variables_of(&mut world, "bob")["refusals"],
        1,
        "the refusal is counted where the title was typed"
    );
}

/// A change that reaches a copy with a text past what a copy keeps — sent by a copy of the app
/// that holds titles to 128 bytes, not 96 — is refused whole there: no operation of it is
/// taken in, and the copies stay alike. A refusal in part would leave a copy with a field its
/// text is missing from. The wide sender is the third member of the group.
#[test]
fn a_change_whose_text_does_not_fit_changes_nothing_where_it_arrives() {
    let mut world = group_world_of(1, 20, &COPIES, TRIO, |world, who, router| {
        if who == "alice" {
            world.add(who, WideSender::new(), router);
        } else {
            world.add(who, TodoPeer::new(), router);
        }
    });
    world.raise("bob", "todo.join", r#"{"node":"2"}"#);
    world.raise("carol", "todo.join", r#"{"node":"3"}"#);
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);

    todo_title(&mut world, "bob", 1, "kept");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);

    // 97 bytes: it fits the sender's bound and not the list's.
    let too_long = "x".repeat(97);
    world.raise(
        "alice",
        "todo.shout",
        &format!(r#"{{"id":9,"text":"{too_long}"}}"#),
    );
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    assert!(
        changes_sent(&world)
            .iter()
            .any(|change| change["ops"][0]["text"] == too_long),
        "the wide sender did send it, so the refusal below is the copies'"
    );

    let carol = todos_of(&mut world, "carol");
    assert_eq!(
        carol.len(),
        1,
        "no operation of the refused change was taken in"
    );
    assert_eq!(carol[&(1, TITLE)].text.as_deref(), Some("kept"));
    assert_eq!(todos_of(&mut world, "bob"), carol);
    for who in ["bob", "carol"] {
        assert_eq!(todo_variables_of(&mut world, who)["refusals"], 1, "{who}");
    }
}

/// A copy killed and started again from what it saved of itself, its journal intact: its
/// causal layer goes on where it was.
///
/// What it saves of itself after the restart is what it saved before: every variable,
/// not only the ones a test reads, so a restore that drops one is said where it drops it.
fn restart(world: &mut sce_rust_world::World, who: &str) {
    let before = todo_variables_of(world, who);
    restart_without_looking(world, who);
    assert_eq!(
        todo_variables_of(world, who),
        before,
        "{who} did not come back as it was"
    );
}

fn restart_without_looking(world: &mut sce_rust_world::World, who: &str) {
    world.restart::<TodoPeer>(who, |engine, clock| {
        let saved = engine.save_at(0).expect("a running machine saves");
        sce_rust_runtime::Engine::<TodoPeer>::restore_with(TodoPeer::new(), &saved, clock, 0)
            .expect("what a machine saved restores")
    });
}

/// A person's copy restarted in the middle of an afternoon holds what it held, and
/// the three copies still end alike: the registers and the values come back from the
/// saved state.
#[test]
fn a_copy_restarted_midway_holds_what_it_held_and_the_list_still_converges() {
    let mut world = todo_world(1, 20);
    todo_title(&mut world, "alice", 4, "water plants");
    todo_set(&mut world, "bob", 4, DONE, 1);
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    // Alice and bob each write over the title at the same moment, so a field holds two
    // values when bob is killed.
    todo_title(&mut world, "alice", 4, "water the plants");
    todo_title(&mut world, "bob", 4, "water ferns");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);
    let before = todos_of(&mut world, "bob");
    assert_eq!(before[&(4, TITLE)].live.len(), 2);

    restart(&mut world, "bob");
    assert_eq!(
        todos_of(&mut world, "bob"),
        before,
        "bob came back as he was"
    );

    // After the restart bob has seen both titles, so what he writes replaces both, and
    // alice, who changes the due day at the same time, keeps hers.
    todo_title(&mut world, "bob", 4, "water everything");
    todo_set(&mut world, "alice", 4, DUE, 6);
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);

    let list = agreed_todos(&mut world);
    assert_eq!(list[&(4, TITLE)].live.len(), 1);
    assert_eq!(list[&(4, TITLE)].text.as_deref(), Some("water everything"));
    assert_eq!(list[&(4, DUE)].value, 6);
    assert_eq!(list, todo_oracle(&world));
    assert!(world.host_errors().is_empty());
}

/// A copy killed and started again is still a member of its group: what either other person
/// writes next reaches it, and what it writes reaches them.
#[test]
fn a_copy_restarted_midway_still_sends_and_takes_in_through_its_group() {
    let mut world = todo_world(1, 20);
    todo_title(&mut world, "alice", 5, "before");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);

    restart(&mut world, "carol");

    todo_title(&mut world, "bob", 5, "after");
    todo_set(&mut world, "alice", 5, DUE, 9);
    todo_set(&mut world, "carol", 6, DONE, 1);
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);

    let list = agreed_todos(&mut world);
    assert_eq!(list[&(5, TITLE)].text.as_deref(), Some("after"));
    assert_eq!(list[&(5, DUE)].value, 9);
    assert_eq!(list[&(6, DONE)].value, 1);
    assert_eq!(list, todo_oracle(&world));
}

/// A copy killed with an edit it had not read loses that edit and nothing else: the
/// edit was never taken, so no copy was sent it.
#[test]
fn an_edit_a_killed_copy_had_not_read_is_lost_and_nothing_else() {
    let mut world = todo_world(1, 20);
    todo_title(&mut world, "alice", 2, "kept");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);

    todo_title(&mut world, "alice", 2, "never read");
    restart(&mut world, "alice");
    assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet);

    let list = agreed_todos(&mut world);
    assert_eq!(list[&(2, TITLE)].text.as_deref(), Some("kept"));
    assert_eq!(list[&(2, TITLE)].live.len(), 1);
    assert_eq!(list, todo_oracle(&world));
}

/// An afternoon of changes to a few to-dos at random times, links cut and restored,
/// delays spread out and envelopes delivered twice: for every seed the three copies end
/// alike and as the oracle says.
#[test]
fn a_random_afternoon_of_changes_ends_with_one_list() {
    an_afternoon(false);
}

/// The same afternoon with copies killed and started again from their saved states.
#[test]
fn a_random_afternoon_with_restarts_ends_with_one_list() {
    an_afternoon(true);
}

fn an_afternoon(restarts: bool) {
    for seed in 0..40 {
        let mut world = todo_world(seed, 5);
        todo_afternoon(&mut world, seed, restarts, restart);
    }
}
