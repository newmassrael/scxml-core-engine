// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Removing a work: it leaves the list and refuses every read and save, and its
//! files stay, so that taking it back is deleting one file.

mod common;

use sce_app_core::{call, Saved, StoreError, WorkStore};
use serde_json::json;

use common::{scratch, FakeRenderer};

const REMOVED_FILE: &str = "removed.json";

/// A removed work is gone from the list and from every command, and a work
/// beside it is untouched.
#[test]
fn a_removed_work_leaves_the_list_and_refuses_every_command() {
    let root = scratch("removal-gone");
    let store = WorkStore::at(&root);
    let keep = store.create_work("Door lock").unwrap();
    let gone = store.create_work("Window blind").unwrap();
    let Saved::Saved { revision, .. } = store.save_source(&gone.id, "It rolls.", None).unwrap()
    else {
        panic!("the first text is saved");
    };

    let removed = store.remove_work(&gone.id).unwrap();
    assert_eq!(removed, gone, "the answer is the work that was removed");

    let listing = store.list_works().unwrap();
    assert_eq!(listing.works, vec![keep.clone()]);
    assert!(
        listing.unreadable.is_empty(),
        "a removed work is not a damaged one"
    );

    for attempt in [
        store.read_work(&gone.id).map(|_| ()),
        store.head(&gone.id).map(|_| ()),
        store.read_source(&gone.id, None).map(|_| ()),
        store.read_model(&gone.id, None).map(|_| ()),
        store.history(&gone.id).map(|_| ()),
        store.model_history(&gone.id).map(|_| ()),
        store
            .save_source(&gone.id, "more", Some(&revision))
            .map(|_| ()),
        store
            .save_model(&gone.id, "<scxml/>", None, None)
            .map(|_| ()),
    ] {
        match attempt {
            Err(StoreError::NotFound { what }) => {
                assert!(what.contains("removed"), "{what}");
            }
            other => panic!("a removed work answered {other:?}"),
        }
    }
    store.read_work(&keep.id).expect("the other work is intact");
}

/// Removing keeps the files, and deleting the marker brings the work back whole.
#[test]
fn deleting_the_marker_restores_the_work_with_its_history() {
    let root = scratch("removal-restore");
    let store = WorkStore::at(&root);
    let work = store.create_work("Door lock").unwrap();
    let Saved::Saved { revision, .. } = store.save_source(&work.id, "It opens.", None).unwrap()
    else {
        panic!("the first text is saved");
    };
    store.remove_work(&work.id).unwrap();

    let marker = root.join(work.id.as_str()).join(REMOVED_FILE);
    assert!(marker.is_file(), "the removal is a file a person can find");
    let text = std::fs::read_to_string(&marker).unwrap();
    assert!(text.contains("sce-work-removed"), "{text}");
    assert!(
        root.join(work.id.as_str()).join("work.json").is_file(),
        "the work's own files were not touched"
    );

    std::fs::remove_file(&marker).unwrap();
    assert_eq!(store.read_work(&work.id).unwrap(), work);
    assert_eq!(store.head(&work.id).unwrap(), Some(revision.clone()));
    assert_eq!(store.history(&work.id).unwrap().len(), 1);
    assert_eq!(store.list_works().unwrap().works, vec![work]);
}

/// A second removal, and a removal of a work that never was, are refused rather
/// than reported as done.
#[test]
fn removing_twice_or_removing_nothing_is_refused() {
    let store = WorkStore::at(scratch("removal-twice"));
    let work = store.create_work("Door lock").unwrap();
    store.remove_work(&work.id).unwrap();
    match store.remove_work(&work.id) {
        Err(StoreError::NotFound { what }) => assert!(what.contains("removed"), "{what}"),
        other => panic!("{other:?}"),
    }
    let never = sce_app_core::WorkId::parse("never-made").unwrap();
    match store.remove_work(&never) {
        Err(StoreError::NotFound { what }) => assert!(!what.contains("removed"), "{what}"),
        other => panic!("{other:?}"),
    }
}

/// A title used again after a removal is a new work: the removed folder keeps
/// its id, and a new id is made beside it.
#[test]
fn a_title_used_again_after_a_removal_is_a_new_work() {
    let store = WorkStore::at(scratch("removal-reuse"));
    let first = store.create_work("Door lock").unwrap();
    store.remove_work(&first.id).unwrap();
    let second = store.create_work("Door lock").unwrap();
    assert_ne!(first.id, second.id);
    assert_eq!(store.list_works().unwrap().works, vec![second]);
}

/// The command is one more entrance onto the same store, and refuses an
/// argument it does not read like every other.
#[test]
fn the_command_removes_and_refuses_what_it_does_not_read() {
    let store = WorkStore::at(scratch("removal-command"));
    let work = store.create_work("Door lock").unwrap();

    let refused = call(
        &store,
        &FakeRenderer,
        "remove_work",
        json!({"id": work.id, "purge": true}),
    )
    .unwrap_err();
    assert_eq!(refused.kind, "bad-request");
    assert_eq!(
        store.list_works().unwrap().works.len(),
        1,
        "a refused command removed nothing"
    );

    let answer = call(&store, &FakeRenderer, "remove_work", json!({"id": work.id})).unwrap();
    assert_eq!(answer["removed"]["id"], work.id.as_str());
    assert_eq!(answer["removed"]["title"], "Door lock");

    let again = call(&store, &FakeRenderer, "remove_work", json!({"id": work.id})).unwrap_err();
    assert_eq!(again.kind, "not-found");
}

/// A save that was waiting for the work's lock when the removal took it finds the
/// work removed and writes nothing.
///
/// The test holds the work's lock the way a removal does, starts a save that
/// passes the first existence check and waits on the lock, places the marker, and
/// lets go. A store that checked only before the lock would write into the work.
#[test]
fn a_save_that_waited_out_a_removal_writes_nothing() {
    let root = scratch("removal-race");
    let store = WorkStore::at(&root);
    let work = store.create_work("Door lock").unwrap();
    let dir = root.join(work.id.as_str());

    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join(".lock"))
        .unwrap();
    lock.lock().unwrap();

    let saver = {
        let store = store.clone();
        let id = work.id.clone();
        std::thread::spawn(move || store.save_source(&id, "late", None))
    };
    // Long enough for the saver to be past its first check and polling the lock
    // (it polls every 20 ms).
    std::thread::sleep(std::time::Duration::from_millis(400));
    std::fs::write(dir.join(REMOVED_FILE), "{}\n").unwrap();
    lock.unlock().unwrap();

    match saver.join().unwrap() {
        Err(StoreError::NotFound { what }) => assert!(what.contains("removed"), "{what}"),
        other => panic!("{other:?}"),
    }
    assert!(
        !dir.join("source.head").exists(),
        "the late save wrote no pointer"
    );
    assert!(!dir.join("source").exists(), "the late save wrote no text");
}
