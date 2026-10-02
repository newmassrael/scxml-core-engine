// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The works folder, through its public interface.
//!
//! The cases are the ways a save can go wrong that nothing about the happy path
//! shows: a stale base, a save that never read, two saves from one base, a file
//! changed by hand, a log cut short by a crash, an id that climbs out of the
//! folder.

mod common;

use std::fs;
use std::sync::{Arc, Barrier};
use std::thread;

use sce_app_core::clock::utc_timestamp;
use sce_app_core::{FixedClock, Revision, Saved, StoreError, WorkId, WorkStore, MAX_SOURCE_BYTES};

fn store(label: &str) -> WorkStore<FixedClock> {
    WorkStore::with_clock(
        common::scratch(label),
        FixedClock("2026-10-03T09:00:00Z".to_string()),
    )
}

fn saved_revision(outcome: Saved) -> Revision {
    match outcome {
        Saved::Saved { revision, .. } => revision,
        other => panic!("expected a new revision, got {other:?}"),
    }
}

// -- what a revision is ------------------------------------------------------

#[test]
fn a_revision_is_the_sha256_of_the_exact_bytes() {
    // The FIPS 180 test vector for "abc".
    assert_eq!(
        Revision::of(b"abc").as_str(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_ne!(Revision::of(b"a\n"), Revision::of(b"a\r\n"));
}

#[test]
fn only_sixty_four_lowercase_hex_characters_are_a_revision() {
    let good = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    assert!(Revision::parse(good).is_ok());
    for bad in [
        "",
        "abc",
        &good.to_uppercase(),
        &good[..63],
        &format!("{good}0"),
        &good.replace('b', "g"),
    ] {
        assert!(Revision::parse(bad).is_err(), "{bad:?} was accepted");
    }
    assert_eq!(Revision::parse(good).unwrap().short(), "ba7816bf8f01");
}

#[test]
fn the_clock_writes_utc_timestamps_across_the_calendar() {
    for (seconds, text) in [
        (0, "1970-01-01T00:00:00Z"),
        (951_782_400, "2000-02-29T00:00:00Z"),
        (1_000_000_000, "2001-09-09T01:46:40Z"),
        (1_782_982_800, "2026-07-02T09:00:00Z"),
        (4_102_444_800, "2100-01-01T00:00:00Z"),
        (4_107_542_399, "2100-02-28T23:59:59Z"),
    ] {
        assert_eq!(utc_timestamp(seconds), text, "{seconds}");
    }
}

// -- works -------------------------------------------------------------------

#[test]
fn a_work_is_created_listed_and_read_back() {
    let store = store("create");
    let work = store.create_work("  Door controller  ").unwrap();
    assert_eq!(work.title, "Door controller");
    assert!(work.id.as_str().starts_with("door-controller-"));
    assert_eq!(work.created_at, "2026-10-03T09:00:00Z");

    let listing = store.list_works().unwrap();
    assert_eq!(listing.works, vec![work.clone()]);
    assert!(listing.unreadable.is_empty());
    assert_eq!(store.read_work(&work.id).unwrap(), work);
}

#[test]
fn a_title_in_another_script_keeps_its_title_and_gets_a_plain_folder() {
    let store = store("script");
    // Hangul syllables, written as escapes so the source stays ASCII; what is
    // measured is that the title survives and the folder name does not follow it.
    let title = "\u{C790}\u{B3D9}\u{BB38} \u{C81C}\u{C5B4}\u{AE30}";
    assert!(
        title.chars().any(|c| c.len_utf8() == 3),
        "the fixture must hold a multi-byte scalar, or this test passes on ASCII"
    );
    let work = store.create_work(title).unwrap();
    assert_eq!(work.title, title);
    assert!(
        work.id.as_str().starts_with("work-"),
        "the folder name is {}",
        work.id
    );
    assert!(work.id.as_str().is_ascii());
    assert!(store.root().join(work.id.as_str()).is_dir());
}

#[test]
fn two_works_with_one_title_get_two_folders() {
    let store = store("same-title");
    let a = store.create_work("Retry client").unwrap();
    let b = store.create_work("Retry client").unwrap();
    assert_ne!(a.id, b.id);
    assert_eq!(store.list_works().unwrap().works.len(), 2);
}

#[test]
fn an_empty_root_lists_nothing_and_does_not_create_the_folder() {
    let root = common::scratch("absent").join("never-made");
    let listing = WorkStore::at(&root).list_works().unwrap();
    assert!(listing.works.is_empty() && listing.unreadable.is_empty());
    assert!(!root.exists());
}

#[test]
fn a_title_that_cannot_name_a_work_is_refused() {
    let store = store("titles");
    for title in ["", "   ", "line\nbreak", &"x".repeat(201)] {
        let error = store.create_work(title).unwrap_err();
        assert_eq!(error.kind(), "invalid-title", "{title:?}");
    }
    assert!(store.list_works().unwrap().works.is_empty());
}

#[test]
fn an_id_that_could_leave_the_root_names_no_work() {
    for id in [
        "",
        "..",
        "../escape",
        "a/b",
        "a\\b",
        ".hidden",
        "UPPER",
        "with space",
        "-leading",
        &"a".repeat(81),
    ] {
        let error = WorkId::parse(id).unwrap_err();
        assert_eq!(error.kind(), "invalid-id", "{id:?}");
    }
    assert!(WorkId::parse("door-1a2b3c4d").is_ok());
}

#[test]
fn a_damaged_work_is_listed_as_unreadable_and_not_dropped() {
    let store = store("damaged");
    let good = store.create_work("Good").unwrap();
    let bad = store.create_work("Bad").unwrap();
    fs::write(
        store.root().join(bad.id.as_str()).join("work.json"),
        "{ not json",
    )
    .unwrap();
    fs::create_dir(store.root().join("empty-folder")).unwrap();
    fs::write(store.root().join("README.txt"), "not a work").unwrap();
    fs::create_dir(store.root().join("Not A Work")).unwrap();

    let listing = store.list_works().unwrap();
    assert_eq!(listing.works, vec![good]);
    let unreadable: Vec<&str> = listing.unreadable.iter().map(|u| u.id.as_str()).collect();
    assert_eq!(unreadable.len(), 2, "{unreadable:?}");
    assert!(unreadable.contains(&bad.id.as_str()));
    assert!(unreadable.contains(&"empty-folder"));
}

#[test]
fn a_work_file_that_names_another_folder_is_refused() {
    let store = store("moved");
    let a = store.create_work("A").unwrap();
    let b = store.create_work("B").unwrap();
    fs::copy(
        store.root().join(a.id.as_str()).join("work.json"),
        store.root().join(b.id.as_str()).join("work.json"),
    )
    .unwrap();
    let error = store.read_work(&b.id).unwrap_err();
    assert_eq!(error.kind(), "corrupt");
}

#[test]
fn an_unknown_work_is_not_found() {
    let store = store("unknown");
    let id = WorkId::parse("nothing-here").unwrap();
    for error in [
        store.read_work(&id).unwrap_err(),
        store.head(&id).unwrap_err(),
        store.read_source(&id, None).unwrap_err(),
        store.save_source(&id, "x", None).unwrap_err(),
        store.history(&id).unwrap_err(),
    ] {
        assert_eq!(error.kind(), "not-found");
    }
}

// -- saves -------------------------------------------------------------------

#[test]
fn a_new_work_has_no_text_and_the_first_save_makes_the_first_revision() {
    let store = store("first");
    let work = store.create_work("Door").unwrap();
    assert_eq!(store.head(&work.id).unwrap(), None);
    assert_eq!(store.read_source(&work.id, None).unwrap(), None);
    assert!(store.history(&work.id).unwrap().is_empty());

    let first = saved_revision(
        store
            .save_source(&work.id, "The door opens.", None)
            .unwrap(),
    );
    assert_eq!(first, Revision::of(b"The door opens."));
    let read = store.read_source(&work.id, None).unwrap().unwrap();
    assert_eq!(
        (read.revision, read.text),
        (first.clone(), "The door opens.".to_string())
    );
    assert_eq!(store.head(&work.id).unwrap(), Some(first));
}

#[test]
fn each_save_names_its_parent_and_every_revision_stays_readable() {
    let store = store("chain");
    let work = store.create_work("Door").unwrap();
    let one = saved_revision(store.save_source(&work.id, "one", None).unwrap());
    let two = saved_revision(store.save_source(&work.id, "two", Some(&one)).unwrap());
    let three = saved_revision(store.save_source(&work.id, "three", Some(&two)).unwrap());

    let history = store.history(&work.id).unwrap();
    let chain: Vec<(Revision, Option<Revision>)> = history
        .iter()
        .map(|e| (e.revision.clone(), e.parent.clone()))
        .collect();
    assert_eq!(
        chain,
        vec![
            (one.clone(), None),
            (two.clone(), Some(one.clone())),
            (three.clone(), Some(two.clone())),
        ]
    );
    assert!(history.iter().all(|e| e.saved_at == "2026-10-03T09:00:00Z"));
    for (revision, text) in [(&one, "one"), (&two, "two"), (&three, "three")] {
        let read = store
            .read_source(&work.id, Some(revision))
            .unwrap()
            .unwrap();
        assert_eq!(read.text, text);
    }
}

#[test]
fn saving_the_current_text_again_writes_nothing() {
    let store = store("unchanged");
    let work = store.create_work("Door").unwrap();
    let one = saved_revision(store.save_source(&work.id, "same", None).unwrap());
    let again = store.save_source(&work.id, "same", Some(&one)).unwrap();
    assert_eq!(again, Saved::Unchanged { revision: one });
    assert_eq!(store.history(&work.id).unwrap().len(), 1);
}

#[test]
fn a_stale_base_is_refused_and_nothing_is_written() {
    let store = store("stale");
    let work = store.create_work("Door").unwrap();
    let one = saved_revision(store.save_source(&work.id, "one", None).unwrap());
    let two = saved_revision(store.save_source(&work.id, "two", Some(&one)).unwrap());

    let error = store
        .save_source(&work.id, "three", Some(&one))
        .unwrap_err();
    match error {
        StoreError::Conflict { base, current } => {
            assert_eq!(base, Some(one));
            assert_eq!(current, Some(two.clone()));
        }
        other => panic!("expected a conflict, got {other:?}"),
    }
    assert_eq!(store.head(&work.id).unwrap(), Some(two));
    assert_eq!(store.history(&work.id).unwrap().len(), 2);
    assert!(!store
        .root()
        .join(work.id.as_str())
        .join("source")
        .join(format!("{}.txt", Revision::of(b"three")))
        .exists());
}

#[test]
fn a_save_that_never_read_the_work_is_refused() {
    let store = store("unread");
    let work = store.create_work("Door").unwrap();
    let one = saved_revision(store.save_source(&work.id, "one", None).unwrap());
    // No base, but the work has text: the caller has not seen what it would replace.
    let error = store.save_source(&work.id, "other", None).unwrap_err();
    assert_eq!(error.kind(), "conflict");
    // And a base for a work with no text names a revision that does not exist.
    let empty = store.create_work("Empty").unwrap();
    let error = store.save_source(&empty.id, "x", Some(&one)).unwrap_err();
    assert_eq!(error.kind(), "conflict");
}

#[test]
fn a_text_larger_than_a_source_may_be_is_refused() {
    let store = store("large");
    let work = store.create_work("Door").unwrap();
    let error = store
        .save_source(&work.id, &"x".repeat(MAX_SOURCE_BYTES + 1), None)
        .unwrap_err();
    assert_eq!(error.kind(), "too-large");
    assert_eq!(store.head(&work.id).unwrap(), None);
}

#[test]
fn line_endings_are_kept_as_saved() {
    let store = store("endings");
    let work = store.create_work("Door").unwrap();
    let lf = saved_revision(store.save_source(&work.id, "a\nb\n", None).unwrap());
    let crlf = saved_revision(
        store
            .save_source(&work.id, "a\r\nb\r\n", Some(&lf))
            .unwrap(),
    );
    assert_ne!(lf, crlf);
    let read = store.read_source(&work.id, Some(&crlf)).unwrap().unwrap();
    assert_eq!(read.text, "a\r\nb\r\n");
}

// -- damage ------------------------------------------------------------------

#[test]
fn a_stored_text_edited_by_hand_is_refused_not_handed_over() {
    let store = store("edited");
    let work = store.create_work("Door").unwrap();
    let one = saved_revision(store.save_source(&work.id, "one", None).unwrap());
    fs::write(
        store
            .root()
            .join(work.id.as_str())
            .join("source")
            .join(format!("{one}.txt")),
        "one, changed",
    )
    .unwrap();
    let error = store.read_source(&work.id, None).unwrap_err();
    assert_eq!(error.kind(), "corrupt");
    let error = store.read_source(&work.id, Some(&one)).unwrap_err();
    assert_eq!(error.kind(), "corrupt");
}

#[test]
fn a_damaged_text_is_replaced_when_the_same_text_is_saved_again() {
    let store = store("heal");
    let work = store.create_work("Door").unwrap();
    let one = saved_revision(store.save_source(&work.id, "one", None).unwrap());
    let two = saved_revision(store.save_source(&work.id, "two", Some(&one)).unwrap());
    let path = store
        .root()
        .join(work.id.as_str())
        .join("source")
        .join(format!("{one}.txt"));
    fs::write(&path, "damaged").unwrap();
    // "one" is saved again, from the current revision: the file under its name is
    // rewritten, because the pointer is about to name it.
    let again = saved_revision(store.save_source(&work.id, "one", Some(&two)).unwrap());
    assert_eq!(again, one);
    assert_eq!(fs::read_to_string(&path).unwrap(), "one");
    assert_eq!(
        store.read_source(&work.id, None).unwrap().unwrap().text,
        "one"
    );
}

#[test]
fn a_pointer_to_a_missing_text_is_corrupt_and_a_named_missing_revision_is_not_found() {
    let store = store("missing");
    let work = store.create_work("Door").unwrap();
    let one = saved_revision(store.save_source(&work.id, "one", None).unwrap());
    let path = store
        .root()
        .join(work.id.as_str())
        .join("source")
        .join(format!("{one}.txt"));
    fs::remove_file(&path).unwrap();
    assert_eq!(
        store.read_source(&work.id, None).unwrap_err().kind(),
        "corrupt"
    );
    assert_eq!(
        store
            .read_source(&work.id, Some(&Revision::of(b"never saved")))
            .unwrap_err()
            .kind(),
        "not-found"
    );
}

#[test]
fn a_pointer_that_is_not_a_revision_is_corrupt() {
    let store = store("bad-head");
    let work = store.create_work("Door").unwrap();
    fs::write(
        store.root().join(work.id.as_str()).join("source.head"),
        "not a digest\n",
    )
    .unwrap();
    assert_eq!(store.head(&work.id).unwrap_err().kind(), "corrupt");
    assert_eq!(
        store.save_source(&work.id, "x", None).unwrap_err().kind(),
        "corrupt"
    );
}

#[test]
fn a_log_cut_short_by_a_crash_does_not_lose_the_next_save() {
    let store = store("torn");
    let work = store.create_work("Door").unwrap();
    let one = saved_revision(store.save_source(&work.id, "one", None).unwrap());
    let log = store.root().join(work.id.as_str()).join("source.log");

    // A save that died while logging: the line is partial and has no newline,
    // and the pointer never moved.
    let mut text = fs::read_to_string(&log).unwrap();
    text.push_str("{\"revision\":\"deadbeef");
    fs::write(&log, text).unwrap();
    assert_eq!(
        store.history(&work.id).unwrap().len(),
        1,
        "the fragment was read as a save"
    );
    assert_eq!(store.head(&work.id).unwrap(), Some(one.clone()));

    // The next save must not be glued onto the fragment.
    let two = saved_revision(store.save_source(&work.id, "two", Some(&one)).unwrap());
    let history = store.history(&work.id).unwrap();
    assert_eq!(
        history
            .iter()
            .map(|e| e.revision.clone())
            .collect::<Vec<_>>(),
        vec![one, two]
    );
}

#[test]
fn a_bad_line_in_the_middle_of_the_log_is_damage_and_says_so() {
    let store = store("bad-log");
    let work = store.create_work("Door").unwrap();
    let one = saved_revision(store.save_source(&work.id, "one", None).unwrap());
    saved_revision(store.save_source(&work.id, "two", Some(&one)).unwrap());
    let log = store.root().join(work.id.as_str()).join("source.log");
    let mut lines: Vec<String> = fs::read_to_string(&log)
        .unwrap()
        .lines()
        .map(String::from)
        .collect();
    lines[0] = "garbage".to_string();
    fs::write(&log, lines.join("\n") + "\n").unwrap();
    assert_eq!(store.history(&work.id).unwrap_err().kind(), "corrupt");
}

// -- two saves from one base -------------------------------------------------

#[test]
fn of_many_saves_from_one_base_exactly_one_wins() {
    const WRITERS: usize = 12;
    let store = Arc::new(store("race"));
    let work = store.create_work("Door").unwrap();
    let base = saved_revision(store.save_source(&work.id, "base", None).unwrap());

    let barrier = Arc::new(Barrier::new(WRITERS));
    let handles: Vec<_> = (0..WRITERS)
        .map(|n| {
            let (store, barrier, id, base) = (
                store.clone(),
                barrier.clone(),
                work.id.clone(),
                base.clone(),
            );
            thread::spawn(move || {
                barrier.wait();
                store.save_source(&id, &format!("writer {n}"), Some(&base))
            })
        })
        .collect();
    let outcomes: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    let winners: Vec<&Saved> = outcomes.iter().filter_map(|o| o.as_ref().ok()).collect();
    assert_eq!(winners.len(), 1, "{outcomes:?}");
    for outcome in &outcomes {
        if let Err(error) = outcome {
            assert_eq!(error.kind(), "conflict", "{error}");
        }
    }
    let history = store.history(&work.id).unwrap();
    assert_eq!(history.len(), 2, "the base and the one winner");
    let head = store.head(&work.id).unwrap().unwrap();
    assert_eq!(head, history[1].revision);
    assert_eq!(history[1].parent, Some(base));
}

#[test]
fn writers_that_each_read_first_all_land_in_one_chain() {
    const WRITERS: usize = 6;
    let store = Arc::new(store("chain-race"));
    let work = store.create_work("Door").unwrap();
    store.save_source(&work.id, "start", None).unwrap();

    let handles: Vec<_> = (0..WRITERS)
        .map(|n| {
            let (store, id) = (store.clone(), work.id.clone());
            thread::spawn(move || loop {
                // The loop a caller runs on a conflict: read what is there, save from it.
                let read = store.read_source(&id, None).unwrap().unwrap();
                let text = format!("{}\nwriter {n}", read.text);
                match store.save_source(&id, &text, Some(&read.revision)) {
                    Ok(_) => return,
                    Err(StoreError::Conflict { .. }) => continue,
                    Err(other) => panic!("{other}"),
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }

    let history = store.history(&work.id).unwrap();
    assert_eq!(history.len(), WRITERS + 1);
    for pair in history.windows(2) {
        assert_eq!(
            pair[1].parent.as_ref(),
            Some(&pair[0].revision),
            "the chain forked"
        );
    }
    let last = store.read_source(&work.id, None).unwrap().unwrap();
    for n in 0..WRITERS {
        assert!(
            last.text.contains(&format!("writer {n}")),
            "writer {n} was lost"
        );
    }
}
