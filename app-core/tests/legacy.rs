// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A works folder an OLDER BUILD wrote, read and saved into by this one.
//!
//! The pointer used to be the digest alone, and no log line said which of several saves
//! of one digest the pointer meant. This build writes the place of that save beside the
//! digest. A folder written before that has no places, and nothing in it can say which
//! line took effect -- so what is held here is what a build must do with a folder it
//! cannot be sure of: not read a save that failed as the one that took effect, not let
//! its own failed save become one, and settle the question with the first save that
//! works. A test built only from folders this build wrote would never meet any of it.

mod common;

use std::fs;

use sce_app_core::{call, Revision, Saved, WorkId, WorkStore};
use serde_json::json;

use common::{as_an_older_build_wrote_it, scratch, FakeRenderer};

const MODEL: &str = "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" version=\"1.0\"/>";

struct Older {
    store: WorkStore,
    id: WorkId,
    s1: Revision,
    s2: Revision,
    model: Revision,
}

impl Older {
    fn dir(&self) -> std::path::PathBuf {
        self.store.root().join(self.id.as_str())
    }

    fn pointer_lines(&self, stem: &str) -> usize {
        fs::read_to_string(self.dir().join(format!("{stem}.head")))
            .unwrap()
            .lines()
            .count()
    }

    fn standing(&self) -> String {
        let answer = call(
            &self.store,
            &FakeRenderer,
            "read_model",
            json!({"id": self.id.as_str()}),
        )
        .unwrap();
        answer["standing"].as_str().unwrap().to_string()
    }
}

fn saved(outcome: Saved) -> Revision {
    match outcome {
        Saved::Saved { revision, .. } | Saved::Unchanged { revision } => revision,
    }
}

/// A work whose model was written for its first text, whose text then moved on, all
/// written the way an older build wrote it.
fn older(label: &str) -> Older {
    let store = WorkStore::at(scratch(label));
    let id = store.create_work("Door lock").unwrap().id;
    let s1 = saved(store.save_source(&id, "The lock opens.", None).unwrap());
    let model = saved(store.save_model(&id, MODEL, None, Some(&s1)).unwrap());
    let s2 = saved(
        store
            .save_source(&id, "The lock opens. Three misses lock it.", Some(&s1))
            .unwrap(),
    );
    let work = Older {
        store,
        id,
        s1,
        s2,
        model,
    };
    as_an_older_build_wrote_it(&work.dir());
    work
}

/// The same model kept for the new text, by a save whose pointer cannot move. On older data
/// nothing may call it the model's claim, and the screen's word for it stays `behind`; the
/// save that works afterwards is the one that makes it `current`.
#[cfg(unix)]
#[test]
fn a_model_kept_for_a_newer_text_by_a_failed_save_stays_behind_on_older_data() {
    let w = older("legacy-failed-relink");
    assert_eq!(
        w.pointer_lines("model"),
        1,
        "the fixture is an older folder"
    );
    assert_eq!(w.standing(), "behind");

    let Some(failed) = common::while_the_pointer_cannot_move(&w.dir(), || {
        w.store
            .save_model(&w.id, MODEL, Some(&w.model), Some(&w.s2))
    }) else {
        return;
    };
    assert_eq!(failed.unwrap_err().kind(), "io");

    let read = w.store.read_model(&w.id, None).unwrap().unwrap();
    assert_eq!(
        read.written_for,
        Some(w.s1.clone()),
        "the claim of a save that did not take effect was read as the model's"
    );
    assert_eq!(w.standing(), "behind");
    assert_eq!(w.store.model_history(&w.id).unwrap().len(), 1);

    // Where it can be written, the same save takes effect -- it is not skipped as the
    // same claim -- and only then is the model the model of the new text.
    let kept = w
        .store
        .save_model(&w.id, MODEL, Some(&w.model), Some(&w.s2))
        .unwrap();
    assert!(matches!(kept, Saved::Saved { .. }), "{kept:?}");
    assert_eq!(w.standing(), "current");
    assert_eq!(w.store.model_history(&w.id).unwrap().len(), 2);
    assert_eq!(
        w.pointer_lines("model"),
        2,
        "the save that worked left the folder in the form this build writes"
    );
}

/// Two saves of one model that disagree about the text, and a pointer that is the digest
/// alone: which of them took effect cannot be told, because a save that failed leaves the
/// same line a save that worked does. So the model says nothing of its text -- not the
/// last line's claim -- and a save settles it. (A model kept for a later text by a save
/// that DID work reads the same until then: the price of not guessing.)
#[test]
fn two_older_saves_of_one_model_that_disagree_leave_its_text_unstated_until_it_is_saved() {
    let w = older("legacy-ambiguous");
    // The line an older build's failed re-link left behind: logged, pointer never moved.
    let log = w.dir().join("model.log");
    let mut text = fs::read_to_string(&log).unwrap();
    text.push_str(
        &json!({
            "revision": w.model,
            "parent": w.model,
            "saved_at": "2026-10-03T09:00:09Z",
            "written_for": w.s2,
        })
        .to_string(),
    );
    text.push('\n');
    fs::write(&log, text).unwrap();

    let read = w.store.read_model(&w.id, None).unwrap().unwrap();
    assert_eq!(
        read.written_for, None,
        "a claim nobody can vouch for was read"
    );
    assert_eq!(w.standing(), "unstated");

    let kept = w
        .store
        .save_model(&w.id, MODEL, Some(&w.model), Some(&w.s2))
        .unwrap();
    assert!(
        matches!(kept, Saved::Saved { .. }),
        "an unvouched claim made the save look like one already held: {kept:?}"
    );
    assert_eq!(w.standing(), "current");
    assert_eq!(w.pointer_lines("model"), 2);
}

/// One save of each, nothing in doubt: older data reads as it always did.
#[test]
fn older_data_with_nothing_in_doubt_reads_as_it_always_did() {
    let w = older("legacy-plain");
    let read = w.store.read_model(&w.id, None).unwrap().unwrap();
    assert_eq!(read.written_for, Some(w.s1.clone()));
    assert_eq!(w.standing(), "behind");
    let history = w.store.history(&w.id).unwrap();
    assert_eq!(
        history
            .iter()
            .map(|e| e.revision.clone())
            .collect::<Vec<_>>(),
        vec![w.s1.clone(), w.s2.clone()]
    );
}

/// The first save into older data brings it to the form this build writes, and what it
/// wrote is read exactly: the chain follows the save it followed, by place.
#[test]
fn the_first_save_into_older_data_brings_it_to_the_form_this_build_writes() {
    let w = older("legacy-upgrade");
    assert_eq!(w.pointer_lines("source"), 1);
    let s3 = saved(
        w.store
            .save_source(&w.id, "The lock opens after a code.", Some(&w.s2))
            .unwrap(),
    );
    assert_eq!(w.pointer_lines("source"), 2);
    let history = w.store.history(&w.id).unwrap();
    assert_eq!(
        history
            .iter()
            .map(|e| e.revision.clone())
            .collect::<Vec<_>>(),
        vec![w.s1.clone(), w.s2.clone(), s3]
    );
}
