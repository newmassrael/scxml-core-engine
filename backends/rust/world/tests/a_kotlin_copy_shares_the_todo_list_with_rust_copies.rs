// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The shared to-do list of `a_shared_todo_list_converges.rs`, with some of its three copies
// the generated Kotlin machine, each running in a JVM of its own and driven by the world
// over the step protocol (`sce_rust_world::external`): two people on Android, or one on
// Android and one on the web, change each other's to-dos at the same time, and the question
// is the one a shared list answers — do the copies end with one list, and is it the list
// the changes meant.
//
// The traffic between the copies is the real traffic: a Kotlin copy's `<send>` leaves the
// machine as the engine's host send, goes through the Mesh endpoint the world holds for it
// and over the simulated network, and what a Rust copy sends reaches the Kotlin one the
// same way. The oracle is the one the all-Rust tests use, and it reads the network, not
// the copies, so it judges the mix.
//
// These tests need a JVM and the step server's build (`./gradlew
// :sce-kotlin-world:installDist`); without them they say so and pass, unless
// `SCE_REQUIRE_TOOLS` is set, which fails them.

mod common;

use common::kotlin::{on_the_wire, Cast, Mix, ACTORS, MIXES};
use common::resync::{assert_converged, digest_of, MINUTE};
use common::*;
use sce_rust_world::{Outcome, World};

/// How many seeds an afternoon is run for in each mix. A Rust afternoon runs forty; each
/// step of a Kotlin copy is a round trip to its process.
const AFTERNOONS: u64 = 40;

/// Take the cast of every actor, or end the test as it is told to when the run has no JVM.
macro_rules! cast {
    ($names:expr) => {
        match Cast::of($names) {
            Some(cast) => cast,
            None => return,
        }
    };
}

#[test]
fn changes_to_different_fields_of_one_todo_are_both_kept_with_a_kotlin_person() {
    let cast = cast!(&["alice", "bob"]);
    for kotlin in [&["alice"][..], &["bob"][..], &["alice", "bob"][..]] {
        let mix = cast.mix(kotlin);
        let mut world = mix.world(1, 20);
        todo_title(&mut world, "alice", 7, "buy milk");
        assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

        // At the same moment: alice retitles it, bob ticks it off and gives it a day.
        todo_title(&mut world, "alice", 7, "buy oat milk");
        todo_set(&mut world, "bob", 7, DONE, 1);
        todo_set(&mut world, "bob", 7, DUE, 20_000);
        assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

        let list = agreed_todos(&mut world);
        let said = mix.describe();
        assert_eq!(
            list[&(7, TITLE)].text.as_deref(),
            Some("buy oat milk"),
            "{said}"
        );
        assert_eq!(
            list[&(7, DONE)].value,
            1,
            "{said}: bob's tick survived alice's retitle"
        );
        assert_eq!(list[&(7, DUE)].value, 20_000, "{said}");
        assert_eq!(list, todo_oracle(&world), "{said}");
        assert!(world.host_errors().is_empty(), "{said}");
    }
}

/// Two people retitle a to-do at the same moment, neither having seen the other's: both
/// titles stay, on every copy, whichever of them is Kotlin, and a person who has seen both
/// and writes a third writes over both.
#[test]
fn two_titles_written_at_the_same_instant_are_both_kept_in_every_mix() {
    let cast = cast!(&["alice", "bob", "server"]);
    for kotlin in MIXES {
        let mix = cast.mix(kotlin);
        let said = mix.describe();
        let mut world = mix.world(1, 20);
        todo_title(&mut world, "alice", 3, "call dentist");
        todo_title(&mut world, "bob", 3, "call the dentist");
        assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet, "{said}");

        let list = agreed_todos(&mut world);
        let title = &list[&(3, TITLE)];
        let mut texts: Vec<_> = title
            .live
            .iter()
            .map(|held| held.text.clone().expect("a title has a text"))
            .collect();
        texts.sort();
        assert_eq!(
            texts,
            ["call dentist", "call the dentist"],
            "{said}: neither saw the other's title, so neither wrote over it"
        );
        // The stamps tie on time and counter, so the node id decides which a single reader
        // takes: bob's.
        assert_eq!(title.text.as_deref(), Some("call the dentist"), "{said}");
        assert_eq!(list, todo_oracle(&world), "{said}");

        todo_title(&mut world, "alice", 3, "call Dr Han");
        assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet, "{said}");
        let list = agreed_todos(&mut world);
        assert_eq!(
            list[&(3, TITLE)].live.len(),
            1,
            "{said}: both were seen, so both were replaced"
        );
        assert_eq!(
            list[&(3, TITLE)].text.as_deref(),
            Some("call Dr Han"),
            "{said}"
        );
        assert_eq!(list, todo_oracle(&world), "{said}");
        assert!(world.host_errors().is_empty(), "{said}");
    }
}

/// Changes made offline are merged when the link is back, and changes made at the same
/// moment on two sides of a cut link are both kept, whichever side is Kotlin.
#[test]
fn changes_made_offline_by_either_side_are_merged_in_every_mix() {
    let cast = cast!(&["alice", "bob", "server"]);
    for kotlin in MIXES {
        let mix = cast.mix(kotlin);
        let said = mix.describe();

        let mut world = mix.world(5, 20);
        world.partition("alice", "server");
        todo_title(&mut world, "alice", 1, "pack bags");
        todo_set(&mut world, "alice", 1, DUE, 100);
        assert_eq!(world.run_until_quiet(500), Outcome::Quiet, "{said}");
        world.advance_ms(500);
        // Bob, meanwhile, ticks off the same to-do and writes another.
        todo_set(&mut world, "bob", 1, DONE, 1);
        todo_title(&mut world, "bob", 2, "book taxi");
        assert_eq!(world.run_until_quiet(500), Outcome::Quiet, "{said}");
        // Alice holds only her own while she is cut off.
        assert_eq!(todos_of(&mut world, "alice").len(), 2, "{said}");

        world.heal("alice", "server");
        assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet, "{said}");
        let list = agreed_todos(&mut world);
        assert_eq!(list.len(), 4, "{said}");
        assert_eq!(
            list[&(1, TITLE)].text.as_deref(),
            Some("pack bags"),
            "{said}"
        );
        assert_eq!(list[&(1, DUE)].value, 100, "{said}");
        assert_eq!(list[&(1, DONE)].value, 1, "{said}");
        assert_eq!(
            list[&(2, TITLE)].text.as_deref(),
            Some("book taxi"),
            "{said}"
        );
        assert_eq!(list, todo_oracle(&world), "{said}");
        assert!(world.host_errors().is_empty(), "{said}");

        let mut world = mix.world(5, 20);
        todo_set(&mut world, "alice", 1, DUE, 10);
        assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet, "{said}");
        world.partition("alice", "server");
        todo_set(&mut world, "alice", 1, DUE, 20);
        todo_set(&mut world, "bob", 1, DUE, 30);
        assert_eq!(world.run_until_quiet(500), Outcome::Quiet, "{said}");
        assert_eq!(todos_of(&mut world, "alice")[&(1, DUE)].value, 20, "{said}");

        world.heal("alice", "server");
        assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet, "{said}");
        let list = agreed_todos(&mut world);
        let mut values: Vec<_> = list[&(1, DUE)].live.iter().map(|held| held.value).collect();
        values.sort_unstable();
        assert_eq!(
            values,
            [20, 30],
            "{said}: the first value was written over by both, and the two writes did not see each other"
        );
        assert_eq!(list, todo_oracle(&world), "{said}");
        assert!(world.host_errors().is_empty(), "{said}");
    }
}

/// A title is counted in UTF-8 bytes and held to the 32 its schema declares, whatever the
/// script and whichever language holds it: eleven Hangul syllables are 33 bytes and do not
/// fit, ten are 30 and do.
#[test]
fn a_title_in_another_script_is_held_to_its_bytes_by_a_kotlin_copy() {
    let cast = cast!(&["alice", "bob"]);
    let fits = "\u{d560}\u{c77c}".repeat(5);
    let too_long = "\u{d560}".repeat(11);
    for kotlin in [&["alice"][..], &["bob"][..]] {
        let mix = cast.mix(kotlin);
        let said = mix.describe();
        let mut world = mix.world(1, 20);
        todo_title(&mut world, "alice", 1, &fits);
        assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet, "{said}");
        todo_title(&mut world, "bob", 2, &too_long);
        assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet, "{said}");

        let list = agreed_todos(&mut world);
        assert_eq!(
            list[&(1, TITLE)].text.as_deref(),
            Some(fits.as_str()),
            "{said}"
        );
        assert!(
            !list.contains_key(&(2, TITLE)),
            "{said}: a title past its bound is refused whole and writes nothing"
        );
        assert_eq!(list, todo_oracle(&world), "{said}");
        assert_eq!(
            todo_variables_of(&mut world, "bob")["refusals"],
            1,
            "{said}: the refusal is counted where the title was typed"
        );
    }
}

/// An afternoon of changes to a few to-dos at random times, links cut and restored, delays
/// spread out and envelopes delivered twice, with Kotlin and Rust copies in every mix: for
/// every seed the three copies end alike and as the oracle says.
#[test]
fn a_random_afternoon_with_kotlin_and_rust_copies_ends_with_one_list() {
    let cast = cast!(&["alice", "bob", "server"]);
    for kotlin in MIXES {
        let mix = cast.mix(kotlin);
        for seed in 0..AFTERNOONS {
            let mut world = mix.world(seed, 5);
            todo_afternoon(&mut world, seed, false, |_, _| {
                unreachable!("an afternoon without restarts kills no copy")
            });
        }
    }
}

/// The same afternoons with copies, Kotlin and Rust, killed and started again from their
/// saved states: a Kotlin copy comes back through the `restore` its generator wrote.
#[test]
fn a_random_afternoon_with_restarts_of_kotlin_and_rust_copies_ends_with_one_list() {
    let cast = cast!(&["alice", "bob", "server"]);
    for kotlin in MIXES {
        let mix = cast.mix(kotlin);
        for seed in 0..AFTERNOONS {
            let mut world = mix.world(seed, 5);
            todo_afternoon(&mut world, seed, true, |world, who| mix.restart(world, who));
        }
    }
}

/// A Kotlin copy is the same machine as a Rust one: given the same afternoon, it puts the
/// same envelopes on the network at the same times, to the same peers, with the same
/// events and data, and they meet the same fates, and each copy ends holding the very text
/// a Rust copy saves of itself, every variable of it. Nothing in a run says which language
/// a copy is written in but how it was added.
#[test]
fn a_kotlin_copy_sends_and_holds_what_a_rust_copy_does() {
    let cast = cast!(&["alice", "bob", "server"]);
    let nothing: &[&'static str] = &[];
    let rust_mix = cast.mix(nothing);
    for seed in 0..AFTERNOONS {
        let mut rust = rust_mix.world(seed, 5);
        todo_afternoon(&mut rust, seed, false, |_, _| unreachable!("no restarts"));
        let wire = on_the_wire(&rust);
        assert!(!wire.is_empty(), "seed {seed}");
        let held: Vec<String> = ACTORS
            .iter()
            .map(|who| rust_mix.save(&mut rust, who))
            .collect();
        drop(rust);

        for kotlin in MIXES {
            let mix = cast.mix(kotlin);
            let said = format!("seed {seed}, {}", mix.describe());
            let mut world = mix.world(seed, 5);
            todo_afternoon(&mut world, seed, false, |_, _| unreachable!("no restarts"));
            let carried = on_the_wire(&world);
            assert_eq!(carried.len(), wire.len(), "{said}");
            for (n, (got, want)) in carried.iter().zip(&wire).enumerate() {
                assert_eq!(got, want, "{said}: line {n}");
            }
            for (who, want) in ACTORS.iter().zip(&held) {
                assert_eq!(
                    &mix.save(&mut world, who),
                    want,
                    "{said}: {who} saves other text than a Rust copy does"
                );
            }
        }
    }
}

/// A run is a function of its seed, with Kotlin copies as without: the same world run
/// twice, in processes that are asked to start over, is the same run byte for byte, since
/// the child reads no clock and draws no number of its own.
#[test]
fn a_run_with_kotlin_copies_is_the_same_run_each_time() {
    let cast = cast!(&["alice", "bob", "server"]);
    let mix = cast.mix(&["alice", "bob", "server"]);
    let run = |seed| {
        let mut world = mix.world(seed, 5);
        todo_afternoon(&mut world, seed, true, |world, who| mix.restart(world, who));
        world.trace()
    };
    for seed in 0..3 {
        assert_eq!(run(seed), run(seed), "seed {seed}");
    }
}

/// A Kotlin copy killed and started again from what it saved of itself holds what it held:
/// every variable, so a restore that drops one is said where it drops it. The registers, the
/// writes seen and the tombstones all come back, and the list still converges.
#[test]
fn a_kotlin_copy_restarted_midway_holds_what_it_held_and_the_list_still_converges() {
    let cast = cast!(&["alice", "bob", "server"]);
    for kotlin in MIXES {
        let mix = cast.mix(kotlin);
        let said = mix.describe();
        let mut world = mix.world(1, 20);
        todo_title(&mut world, "alice", 4, "water plants");
        todo_set(&mut world, "bob", 4, DONE, 1);
        assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet, "{said}");
        // Alice and bob each write over the title at the same moment, so a field holds
        // two values when a copy is killed.
        todo_title(&mut world, "alice", 4, "water the plants");
        todo_title(&mut world, "bob", 4, "water ferns");
        assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet, "{said}");
        assert_eq!(
            todos_of(&mut world, "bob")[&(4, TITLE)].live.len(),
            2,
            "{said}"
        );

        for who in ["alice", "bob", "server"] {
            let before = todo_variables_of(&mut world, who);
            mix.restart(&mut world, who);
            assert_eq!(
                todo_variables_of(&mut world, who),
                before,
                "{said}: {who} did not come back as it was"
            );
        }

        // After the restarts bob has seen both titles, so what he writes replaces both, and
        // alice, who changes the due day at the same time, keeps hers.
        todo_title(&mut world, "bob", 4, "water everything");
        todo_set(&mut world, "alice", 4, DUE, 6);
        assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet, "{said}");
        let list = agreed_todos(&mut world);
        assert_eq!(list[&(4, TITLE)].live.len(), 1, "{said}");
        assert_eq!(
            list[&(4, TITLE)].text.as_deref(),
            Some("water everything"),
            "{said}"
        );
        assert_eq!(list[&(4, DUE)].value, 6, "{said}");
        assert_eq!(list, todo_oracle(&world), "{said}");
        assert_eq!(
            todo_variables_of(&mut world, "server")["hub"],
            true,
            "{said}: the hub came back as the hub"
        );
        assert!(world.host_errors().is_empty(), "{said}");
    }
}

/// Some to-dos written across the first half hour, a save of `who`'s copy, and then more
/// changes by both people in the minutes after it: the changes that copy would lose if it
/// came back from the save. The save is text, which is the same for either language.
fn saved_then_both_write_on(mix: &Mix<'_>, who: &str) -> (World, String) {
    let mut world = mix.world(7, 20);
    assert_eq!(world.run_until_quiet(1), Outcome::Quiet);
    todo_title(&mut world, "alice", 1, "pay rent");
    todo_set(&mut world, "bob", 1, DUE, 5);
    todo_set(&mut world, "alice", 2, DONE, 0);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    world.advance_ms(30 * MINUTE);

    let old = mix.save(&mut world, who);

    world.advance_ms(7 * MINUTE);
    todo_title(&mut world, "bob", 1, "pay the rent");
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    world.advance_ms(5 * MINUTE);
    todo_set(&mut world, "alice", 3, DUE, 9);
    todo_title(&mut world, "alice", 2, "buy bread");
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    world.advance_ms(11 * MINUTE);
    todo_set(&mut world, "bob", 3, DONE, 1);
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);
    (world, old)
}

/// A copy brought back from a save older than what the others have been sent holds less
/// than they do and stays that way, whichever copy it is and whichever language it is
/// written in: nothing the network does repairs it. Told that its link is back, it ends
/// with the list the others hold, and the digest each copy keeps, which every language
/// computes with its own checked integer arithmetic, is the digest of what it holds and
/// the same as the others'.
#[test]
fn a_copy_restored_from_an_old_save_is_repaired_by_resync_in_every_mix() {
    let cast = cast!(&["alice", "bob", "server"]);
    for kotlin in MIXES {
        let mix = cast.mix(kotlin);
        for lost in ["alice", "server"] {
            let said = format!("{}, {lost} restored", mix.describe());
            let (mut world, old) = saved_then_both_write_on(&mix, lost);
            let whole_list = assert_converged(&mut world, &format!("{said}: before the restore"));

            mix.restore_from(&mut world, lost, &old);
            let behind = todos_of(&mut world, lost);
            assert_ne!(
                behind, whole_list,
                "{said}: it came back without what came after"
            );
            // The links are up and the others believe it has everything they sent it, so
            // there is nothing left for the network to do.
            assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet, "{said}");
            assert_eq!(
                todos_of(&mut world, lost),
                behind,
                "{said}: nothing repaired it"
            );
            assert_ne!(
                digest_of(&mut world, lost),
                digest_of(
                    &mut world,
                    if lost == "server" { "alice" } else { "server" }
                ),
                "{said}"
            );

            world.raise(lost, "todo.resync", "");
            assert_eq!(world.run_until_quiet(60_000), Outcome::Quiet, "{said}");
            let list = assert_converged(&mut world, &format!("{said}: after the resync"));
            assert_eq!(list, whole_list, "{said}");
        }
    }
}

/// An edit a killed Kotlin copy had not read is lost, and nothing else: the edit was never
/// taken, so no copy was sent it.
#[test]
fn an_edit_a_killed_kotlin_copy_had_not_read_is_lost_and_nothing_else() {
    let cast = cast!(&["alice"]);
    let mix = cast.mix(&["alice"]);
    let mut world = mix.world(1, 20);
    todo_title(&mut world, "alice", 2, "kept");
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    todo_title(&mut world, "alice", 2, "never read");
    mix.restart(&mut world, "alice");
    assert_eq!(world.run_until_quiet(10_000), Outcome::Quiet);

    let list = agreed_todos(&mut world);
    assert_eq!(list[&(2, TITLE)].text.as_deref(), Some("kept"));
    assert_eq!(list[&(2, TITLE)].live.len(), 1);
    assert_eq!(list, todo_oracle(&world));
}
