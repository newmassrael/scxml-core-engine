// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Removing a spec: it leaves the list and refuses every read and save, and its
//! files stay, so that taking it back is deleting one file.

mod common;

use sce_specs::{call, Saved, SpecStore, StoreError};
use serde_json::json;

use common::{scratch, FakeRenderer};

const REMOVED_FILE: &str = "removed.json";

/// A removed spec is gone from the list and from every command, and a spec
/// beside it is untouched.
#[test]
fn a_removed_spec_leaves_the_list_and_refuses_every_command() {
    let root = scratch("removal-gone");
    let store = SpecStore::at(&root);
    let keep = store.create_spec("Door lock").unwrap();
    let gone = store.create_spec("Window blind").unwrap();
    let Saved::Saved { revision, .. } = store.save_source(&gone.id, "It rolls.", None).unwrap()
    else {
        panic!("the first text is saved");
    };

    let removed = store.remove_spec(&gone.id).unwrap();
    assert_eq!(removed, gone, "the answer is the spec that was removed");

    let listing = store.list_specs().unwrap();
    assert_eq!(listing.specs, vec![keep.clone()]);
    assert!(
        listing.unreadable.is_empty(),
        "a removed spec is not a damaged one"
    );

    for attempt in [
        store.read_spec(&gone.id).map(|_| ()),
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
            other => panic!("a removed spec answered {other:?}"),
        }
    }
    store.read_spec(&keep.id).expect("the other spec is intact");
}

/// Removing keeps the files, and deleting the marker brings the spec back whole.
#[test]
fn deleting_the_marker_restores_the_spec_with_its_history() {
    let root = scratch("removal-restore");
    let store = SpecStore::at(&root);
    let spec = store.create_spec("Door lock").unwrap();
    let Saved::Saved { revision, .. } = store.save_source(&spec.id, "It opens.", None).unwrap()
    else {
        panic!("the first text is saved");
    };
    store.remove_spec(&spec.id).unwrap();

    let marker = root.join(spec.id.as_str()).join(REMOVED_FILE);
    assert!(marker.is_file(), "the removal is a file a person can find");
    let text = std::fs::read_to_string(&marker).unwrap();
    assert!(text.contains("sce-spec-removed"), "{text}");
    assert!(
        root.join(spec.id.as_str()).join("spec.json").is_file(),
        "the spec's own files were not touched"
    );

    std::fs::remove_file(&marker).unwrap();
    assert_eq!(store.read_spec(&spec.id).unwrap(), spec);
    assert_eq!(store.head(&spec.id).unwrap(), Some(revision.clone()));
    assert_eq!(store.history(&spec.id).unwrap().len(), 1);
    assert_eq!(store.list_specs().unwrap().specs, vec![spec]);
}

/// A second removal, and a removal of a spec that never was, are refused rather
/// than reported as done.
#[test]
fn removing_twice_or_removing_nothing_is_refused() {
    let store = SpecStore::at(scratch("removal-twice"));
    let spec = store.create_spec("Door lock").unwrap();
    store.remove_spec(&spec.id).unwrap();
    match store.remove_spec(&spec.id) {
        Err(StoreError::NotFound { what }) => assert!(what.contains("removed"), "{what}"),
        other => panic!("{other:?}"),
    }
    let never = sce_specs::SpecId::parse("never-made").unwrap();
    match store.remove_spec(&never) {
        Err(StoreError::NotFound { what }) => assert!(!what.contains("removed"), "{what}"),
        other => panic!("{other:?}"),
    }
}

/// A title used again after a removal is a new spec: the removed folder keeps
/// its id, and a new id is made beside it.
#[test]
fn a_title_used_again_after_a_removal_is_a_new_spec() {
    let store = SpecStore::at(scratch("removal-reuse"));
    let first = store.create_spec("Door lock").unwrap();
    store.remove_spec(&first.id).unwrap();
    let second = store.create_spec("Door lock").unwrap();
    assert_ne!(first.id, second.id);
    assert_eq!(store.list_specs().unwrap().specs, vec![second]);
}

/// The command is one more entrance onto the same store, and refuses an
/// argument it does not read like every other.
#[test]
fn the_command_removes_and_refuses_what_it_does_not_read() {
    let store = SpecStore::at(scratch("removal-command"));
    let spec = store.create_spec("Door lock").unwrap();

    let refused = call(
        &store,
        &FakeRenderer,
        "remove_spec",
        json!({"id": spec.id, "purge": true}),
    )
    .unwrap_err();
    assert_eq!(refused.kind, "bad-request");
    assert_eq!(
        store.list_specs().unwrap().specs.len(),
        1,
        "a refused command removed nothing"
    );

    let answer = call(&store, &FakeRenderer, "remove_spec", json!({"id": spec.id})).unwrap();
    assert_eq!(answer["removed"]["id"], spec.id.as_str());
    assert_eq!(answer["removed"]["title"], "Door lock");

    let again = call(&store, &FakeRenderer, "remove_spec", json!({"id": spec.id})).unwrap_err();
    assert_eq!(again.kind, "not-found");
}

/// A save that was waiting for the spec's lock when the removal took it finds the
/// spec removed and writes nothing.
///
/// The test holds the spec's lock the way a removal does, starts a save that
/// passes the first existence check and waits on the lock, places the marker, and
/// lets go. A store that checked only before the lock would write into the spec.
#[test]
fn a_save_that_waited_out_a_removal_writes_nothing() {
    let root = scratch("removal-race");
    let store = SpecStore::at(&root);
    let spec = store.create_spec("Door lock").unwrap();
    let dir = root.join(spec.id.as_str());

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
        let id = spec.id.clone();
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
