// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A shared to-do list: two people, `alice` and `bob`, each with a copy
// (`machines/todo_peer.scxml`), and the hub both send to. They change each other's
// to-dos at the same time — one retitles what the other ticks off — on links that
// delay, reorder, repeat and cut what they carry, and the question is the one a
// shared list answers: do the three copies end with the same list, and is it the list
// the changes meant.
//
// A to-do is a row of fields and each field is its own register, so what is held here
// that a sheet of cells does not show is that two changes to DIFFERENT fields of one
// to-do both stay, where a to-do kept whole would keep one and lose the other.
//
// Equality of the copies says they agree, not that they are right, so each run is also
// held to an oracle that does not read the copies: of every write the people sent, the
// one with the latest stamp is the field's value, and its text is the title.

mod common;

use common::*;
use sce_rust_world::{Outcome, Rng};

#[test]
fn changes_to_different_fields_of_one_todo_are_both_kept() {
    let mut world = todo_world(1, 20);
    todo_title(&mut world, "alice", 7, "buy milk");
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    // At the same moment: alice retitles it, bob ticks it off and gives it a day.
    todo_title(&mut world, "alice", 7, "buy oat milk");
    todo_set(&mut world, "bob", 7, DONE, 1);
    todo_set(&mut world, "bob", 7, DUE, 20_000);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

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

#[test]
fn two_titles_written_at_the_same_instant_leave_one_everywhere() {
    let mut world = todo_world(1, 20);
    todo_title(&mut world, "alice", 3, "call dentist");
    todo_title(&mut world, "bob", 3, "call the dentist");
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let list = agreed_todos(&mut world);
    // The stamps tie on time and counter, so the node id decides: bob's. And the text
    // is the text that write carried, not the other's.
    assert_eq!(list[&(3, TITLE)].text.as_deref(), Some("call the dentist"));
    assert_eq!(list, todo_oracle(&world));
}

/// A person who has read a title and types over it means to replace it, even when their
/// own clock is behind the one that wrote it.
#[test]
fn a_title_written_after_seeing_another_wins_whatever_the_clock_says() {
    let mut world = todo_world(1, 20);
    todo_edit_at(&mut world, "alice", 1, TITLE, 0, "draft", 1_000);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    assert_eq!(
        todos_of(&mut world, "bob")[&(1, TITLE)].text.as_deref(),
        Some("draft"),
        "bob saw the title"
    );

    todo_edit_at(&mut world, "bob", 1, TITLE, 0, "final", 10);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let list = agreed_todos(&mut world);
    assert_eq!(list[&(1, TITLE)].text.as_deref(), Some("final"));
    assert_eq!(list, todo_oracle(&world));
}

#[test]
fn changes_made_offline_are_merged_when_the_link_is_back() {
    let mut world = todo_world(5, 20);
    world.partition("alice", "server");
    todo_title(&mut world, "alice", 1, "pack bags");
    todo_set(&mut world, "alice", 1, DUE, 100);
    assert_eq!(world.run_until_quiet(500), Outcome::Quiet);
    world.advance_ms(500);
    // Bob, meanwhile, ticks off the same to-do and writes another.
    todo_set(&mut world, "bob", 1, DONE, 1);
    todo_title(&mut world, "bob", 2, "book taxi");
    assert_eq!(world.run_until_quiet(500), Outcome::Quiet);
    // Alice holds only her own while she is cut off.
    assert_eq!(todos_of(&mut world, "alice").len(), 2);

    world.heal("alice", "server");
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
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    todo_title(&mut world, "bob", 2, &too_long);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

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

/// A write that reaches the hub over the network with a text past its bound — sent by a
/// copy of the app that holds titles to 64 bytes, not 32 — is refused whole there: no
/// cell is merged, nothing is passed on, and the copies stay alike. A refusal in part
/// would leave the hub with a field its text is missing from, and the others without it.
#[test]
fn a_write_whose_text_does_not_fit_changes_nothing_at_the_hub() {
    let mut world = sce_rust_world::World::new(1);
    world.add("alice", WideSender::new(), person("alice", false));
    world.add("bob", TodoPeer::new(), person("bob", false));
    world.add("server", TodoPeer::new(), hub(false));
    world.link("alice", "server", 20);
    world.link("bob", "server", 20);
    world.raise("bob", "todo.join", r#"{"node":"2","hub":false}"#);
    world.raise("server", "todo.join", r#"{"node":"100","hub":true}"#);

    todo_title(&mut world, "bob", 1, "kept");
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    // 33 bytes: it fits the sender's bound and not the list's.
    let too_long = "x".repeat(33);
    world.raise(
        "alice",
        "todo.shout",
        &format!(r#"{{"id":"9","text":"{too_long}"}}"#),
    );
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    assert!(
        sent_to_the_hub(&world)
            .iter()
            .any(|(event, data)| event == "todo.write" && data["text"] == too_long),
        "the wide sender did send it, so the refusal below is the hub's"
    );

    let server = todos_of(&mut world, "server");
    assert_eq!(server.len(), 1, "no cell of the refused write was merged");
    assert_eq!(server[&(1, TITLE)].text.as_deref(), Some("kept"));
    assert_eq!(todos_of(&mut world, "bob"), server);
    assert_eq!(todo_variables_of(&mut world, "server")["refusals"], 1);
}

/// An afternoon of changes to a few to-dos at random times, links cut and restored,
/// delays spread out and envelopes delivered twice: for every seed the three copies end
/// alike and as the oracle says.
#[test]
fn a_random_afternoon_of_changes_ends_with_one_list() {
    const TITLES: [&str; 5] = ["milk", "post office", "\u{d560}\u{c77c}", "tax", "gym at 7"];
    for seed in 0..40 {
        let mut rng = Rng::new(seed);
        let mut world = todo_world(seed, 5);
        world.jitter("alice", "server", 40);
        world.jitter("bob", "server", 40);
        world.reorder("alice", "server");
        world.reorder("bob", "server");
        world.degrade("alice", "server", 0, 20);
        world.degrade("bob", "server", 0, 20);
        let mut cut = [false, false];
        for _ in 0..60 {
            let who = if rng.below(2) == 0 { "alice" } else { "bob" };
            let id = 1 + rng.below(4);
            match rng.below(4) {
                0 => {
                    let title = TITLES[usize::try_from(rng.below(5)).unwrap()];
                    todo_title(&mut world, who, id, title);
                }
                1 => todo_set(
                    &mut world,
                    who,
                    id,
                    DONE,
                    i64::try_from(rng.below(2)).unwrap(),
                ),
                2 => todo_set(
                    &mut world,
                    who,
                    id,
                    DUE,
                    i64::try_from(rng.below(365)).unwrap(),
                ),
                _ => todo_set(
                    &mut world,
                    who,
                    id,
                    REMOVED,
                    i64::try_from(rng.below(2)).unwrap(),
                ),
            }
            if rng.below(6) == 0 {
                let which = usize::from(rng.below(2) == 1);
                let name = ["alice", "bob"][which];
                if cut[which] {
                    world.heal(name, "server");
                } else {
                    world.partition(name, "server");
                }
                cut[which] = !cut[which];
            }
            world.run_until_quiet(i64::try_from(rng.below(60)).unwrap() + 1);
        }
        for (which, name) in ["alice", "bob"].into_iter().enumerate() {
            if cut[which] {
                world.heal(name, "server");
            }
        }
        assert_eq!(
            world.run_until_quiet(600_000),
            Outcome::Quiet,
            "seed {seed}"
        );

        let list = agreed_todos(&mut world);
        assert_eq!(list, todo_oracle(&world), "seed {seed}");
        assert!(world.host_errors().is_empty(), "seed {seed}");
        assert!(!list.is_empty(), "seed {seed}");
        for who in ["alice", "bob", "server"] {
            assert_eq!(
                todo_variables_of(&mut world, who)["refusals"],
                0,
                "seed {seed}: {who} refused a write"
            );
        }
    }
}
