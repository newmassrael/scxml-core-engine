// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A work's model: the chain kept beside the text, and what the commands say of it.
//!
//! The model is saved by the same code as the text (one compare-and-swap, one
//! lock, one log format), so what is held here is what is NEW about it: it
//! says which text it was written for, a model kept for a later text is an entry
//! of its history, and the screen's "is this about the text I see" is one word
//! (`standing`) every shell reads the same way.

mod common;

use sce_app_core::{
    call, CommandError, Revision, Saved, StoreError, WorkId, WorkStore, MAX_MODEL_BYTES,
};
use serde_json::{json, Value};

use common::{scratch, FakeRenderer};

const MODEL: &str = "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" version=\"1.0\"/>";

struct Fixture {
    store: WorkStore,
    id: WorkId,
    first: Revision,
}

/// A work whose first text is saved.
fn work(label: &str) -> Fixture {
    let store = WorkStore::at(scratch(label));
    let id = store.create_work("Door lock").unwrap().id;
    let Saved::Saved { revision, .. } = store.save_source(&id, "The lock opens.", None).unwrap()
    else {
        panic!("the first text is saved");
    };
    Fixture {
        store,
        id,
        first: revision,
    }
}

fn saved_revision(saved: Saved) -> Revision {
    match saved {
        Saved::Saved { revision, .. } | Saved::Unchanged { revision } => revision,
    }
}

fn run(f: &Fixture, name: &str, args: Value) -> Result<Value, CommandError> {
    call(&f.store, &FakeRenderer, name, args)
}

/// A work with no model has none to read, and the text and the model are
/// separate chains: saving one moves nothing of the other.
#[test]
fn a_work_has_no_model_until_one_is_saved_and_the_chains_are_separate() {
    let f = work("models-separate");
    assert_eq!(f.store.model_head(&f.id).unwrap(), None);
    assert!(f.store.read_model(&f.id, None).unwrap().is_none());
    assert!(f.store.model_history(&f.id).unwrap().is_empty());

    let model = f
        .store
        .save_model(&f.id, MODEL, None, Some(&f.first))
        .unwrap();
    let revision = saved_revision(model);
    assert_eq!(f.store.model_head(&f.id).unwrap(), Some(revision.clone()));
    assert_eq!(
        f.store.head(&f.id).unwrap(),
        Some(f.first.clone()),
        "the text did not move"
    );
    assert_eq!(f.store.history(&f.id).unwrap().len(), 1);

    let read = f.store.read_model(&f.id, None).unwrap().unwrap();
    assert_eq!(read.revision, revision);
    assert_eq!(read.text, MODEL);
    assert_eq!(read.written_for, Some(f.first.clone()));
}

/// A model is saved from the model the writer read, as a text is: a stale base
/// is refused and nothing is written.
#[test]
fn a_model_saved_from_a_stale_base_is_refused() {
    let f = work("models-conflict");
    let first = saved_revision(
        f.store
            .save_model(&f.id, MODEL, None, Some(&f.first))
            .unwrap(),
    );
    // A writer that never read the model has not seen what it would replace.
    match f.store.save_model(&f.id, "<scxml/>", None, Some(&f.first)) {
        Err(StoreError::Conflict {
            base: None,
            current,
        }) => assert_eq!(current, Some(first.clone())),
        other => panic!("{other:?}"),
    }
    assert_eq!(f.store.model_head(&f.id).unwrap(), Some(first));
}

/// A model that says it was written for a text nobody saved is about nothing.
#[test]
fn a_model_for_a_text_that_was_never_saved_is_refused() {
    let f = work("models-unknown-source");
    let nobody = Revision::of(b"a text this work never held");
    match f.store.save_model(&f.id, MODEL, None, Some(&nobody)) {
        Err(StoreError::NotFound { what }) => assert!(what.contains("written_for"), "{what}"),
        other => panic!("{other:?}"),
    }
    assert_eq!(
        f.store.model_head(&f.id).unwrap(),
        None,
        "nothing was written"
    );
}

/// The same model kept for a LATER text is a new entry of its history, not an
/// `Unchanged`: the text moved on, the writer read it again, and the model is
/// now the model of the new text. The same model for the SAME text is
/// unchanged.
#[test]
fn a_model_kept_for_a_later_text_is_a_new_entry_and_for_the_same_text_is_not() {
    let f = work("models-kept");
    let model = saved_revision(
        f.store
            .save_model(&f.id, MODEL, None, Some(&f.first))
            .unwrap(),
    );

    assert!(matches!(
        f.store
            .save_model(&f.id, MODEL, Some(&model), Some(&f.first))
            .unwrap(),
        Saved::Unchanged { .. }
    ));
    assert_eq!(f.store.model_history(&f.id).unwrap().len(), 1);

    let second = saved_revision(
        f.store
            .save_source(
                &f.id,
                "The lock opens. Three misses lock it.",
                Some(&f.first),
            )
            .unwrap(),
    );
    let kept = f
        .store
        .save_model(&f.id, MODEL, Some(&model), Some(&second))
        .unwrap();
    assert!(matches!(kept, Saved::Saved { .. }), "{kept:?}");

    let history = f.store.model_history(&f.id).unwrap();
    assert_eq!(history.len(), 2, "{history:?}");
    assert_eq!(
        history[0].revision, history[1].revision,
        "the same model both times"
    );
    assert_eq!(history[0].written_for, Some(f.first.clone()));
    assert_eq!(history[1].written_for, Some(second.clone()));
    assert_eq!(history[1].parent, Some(model.clone()));
    let read = f.store.read_model(&f.id, None).unwrap().unwrap();
    assert_eq!(
        read.written_for,
        Some(second),
        "what is read is the latest claim"
    );
}

/// A work whose text moved on from the one its model was written for, and whose model
/// is read again at the new text without changing: the save that would say so.
#[cfg(unix)]
struct Behind {
    f: Fixture,
    model: Revision,
    second: Revision,
}

#[cfg(unix)]
fn behind(label: &str) -> Behind {
    let f = work(label);
    let model = saved_revision(
        f.store
            .save_model(&f.id, MODEL, None, Some(&f.first))
            .unwrap(),
    );
    let second = saved_revision(
        f.store
            .save_source(
                &f.id,
                "The lock opens. Three misses lock it.",
                Some(&f.first),
            )
            .unwrap(),
    );
    Behind { f, model, second }
}

/// The same model kept for the new text is a save like any other, and a save whose
/// pointer cannot move did not take effect: the model is still the one for the
/// earlier text, which is what the screen says (`behind`), and the history has one entry.
#[cfg(unix)]
#[test]
fn a_model_kept_for_a_later_text_by_a_save_that_failed_is_still_behind() {
    let Behind { f, model, second } = behind("models-kept-failed");
    let dir = f.store.root().join(f.id.as_str());
    let Some(failed) = common::while_the_pointer_cannot_move(&dir, || {
        f.store
            .save_model(&f.id, MODEL, Some(&model), Some(&second))
    }) else {
        return;
    };
    assert_eq!(failed.unwrap_err().kind(), "io");

    let read = f.store.read_model(&f.id, None).unwrap().unwrap();
    assert_eq!(
        read.written_for,
        Some(f.first.clone()),
        "the claim of a save that did not take effect was read as the model's"
    );
    let standing = run(&f, "read_model", json!({"id": f.id.as_str()})).unwrap();
    assert_eq!(standing["standing"], "behind", "{standing}");
    assert_eq!(f.store.model_history(&f.id).unwrap().len(), 1);

    // Retried where it can be written, it takes effect, and only then is it current.
    let kept = f
        .store
        .save_model(&f.id, MODEL, Some(&model), Some(&second))
        .unwrap();
    assert!(matches!(kept, Saved::Saved { .. }), "{kept:?}");
    let read = f.store.read_model(&f.id, None).unwrap().unwrap();
    assert_eq!(read.written_for, Some(second));
    assert_eq!(f.store.model_history(&f.id).unwrap().len(), 2);
}

/// A failed save leaves its line in the log for good. A model saved after it descends
/// from the save that took effect, not from the failed one that has the same text.
#[cfg(unix)]
#[test]
fn a_model_saved_after_a_failed_save_of_the_same_model_follows_the_one_that_took_effect() {
    let Behind { f, model, second } = behind("models-after-failed");
    let dir = f.store.root().join(f.id.as_str());
    let Some(failed) = common::while_the_pointer_cannot_move(&dir, || {
        f.store
            .save_model(&f.id, MODEL, Some(&model), Some(&second))
    }) else {
        return;
    };
    assert_eq!(failed.unwrap_err().kind(), "io");

    let other = "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" version=\"1.0\" initial=\"a\"><state id=\"a\"/></scxml>";
    let next = saved_revision(
        f.store
            .save_model(&f.id, other, Some(&model), Some(&second))
            .unwrap(),
    );

    let history = f.store.model_history(&f.id).unwrap();
    assert_eq!(
        history
            .iter()
            .map(|entry| (entry.revision.clone(), entry.written_for.clone()))
            .collect::<Vec<_>>(),
        vec![(model, Some(f.first.clone())), (next, Some(second)),],
        "the failed save was listed as a save the next one followed"
    );
}

/// The `standing` word: `current` for the text as it is, `behind` once the
/// text moved on, `unstated` when the writer did not say.
#[test]
fn standing_says_whether_the_model_is_about_the_text_now() {
    let f = work("models-standing");
    let id = f.id.as_str();
    let standing =
        |f: &Fixture| run(f, "read_model", json!({"id": id})).unwrap()["standing"].clone();
    assert_eq!(standing(&f), Value::Null, "no model, no standing");

    f.store
        .save_model(&f.id, MODEL, None, Some(&f.first))
        .unwrap();
    assert_eq!(standing(&f), json!("current"));

    let head = f.store.head(&f.id).unwrap();
    f.store
        .save_source(
            &f.id,
            "The lock opens. Three misses lock it.",
            head.as_ref(),
        )
        .unwrap();
    assert_eq!(standing(&f), json!("behind"), "the text moved on");

    let model = f.store.model_head(&f.id).unwrap();
    f.store
        .save_model(&f.id, "<scxml/>", model.as_ref(), None)
        .unwrap();
    assert_eq!(standing(&f), json!("unstated"), "its writer did not say");
}

/// A model larger than the store accepts is refused as a model.
#[test]
fn an_oversize_model_is_refused_as_a_model() {
    let f = work("models-too-large");
    match f
        .store
        .save_model(&f.id, &"x".repeat(MAX_MODEL_BYTES + 1), None, None)
    {
        Err(e @ StoreError::TooLarge { what: "model", .. }) => {
            assert!(e.to_string().contains("a model may hold"), "{e}");
        }
        other => panic!("{other:?}"),
    }
}

/// A revision of the model is read back only if its bytes hash to its name,
/// as a text's are.
#[test]
fn a_damaged_model_file_is_refused_not_handed_over() {
    let f = work("models-damaged");
    let model = saved_revision(f.store.save_model(&f.id, MODEL, None, None).unwrap());
    let file = f
        .store
        .root()
        .join(f.id.as_str())
        .join("model")
        .join(format!("{model}.scxml"));
    assert!(
        file.is_file(),
        "a model lives under model/ as <digest>.scxml"
    );
    std::fs::write(&file, "<scxml>edited by hand</scxml>").unwrap();
    match f.store.read_model(&f.id, None) {
        Err(StoreError::Corrupt { .. }) => {}
        other => panic!("{other:?}"),
    }
}

/// `figures` draws the model that is current, or the one named, with the
/// options it was given, and says how the model stands to the text.
#[test]
fn figures_draw_the_model_with_the_options_and_say_where_it_stands() {
    let f = work("models-figures");
    let id = f.id.as_str();
    match run(&f, "figures", json!({"id": id})) {
        Err(e) => assert_eq!(e.kind, "not-found", "{e:?}"),
        Ok(v) => panic!("a work with no model drew {v}"),
    }

    f.store
        .save_model(&f.id, MODEL, None, Some(&f.first))
        .unwrap();
    let drawn = run(
        &f,
        "figures",
        json!({"id": id, "page": "a3-landscape", "lexicon": "ko", "min_pt": 8}),
    )
    .unwrap();
    assert_eq!(drawn["standing"], "current");
    assert_eq!(drawn["generator"], "fake-sce 0");
    assert_eq!(drawn["model"]["written_for"], json!(f.first));
    let sheets = drawn["sheets"].as_array().unwrap();
    let names: Vec<&str> = sheets.iter().map(|s| s["name"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        ["picture.svg", "fields-1.svg"],
        "in the order SCE wrote them"
    );
    let asked = sheets[0]["svg"].as_str().unwrap();
    assert!(
        asked.contains("page=a3-landscape lexicon=ko min_pt=8"),
        "{asked}"
    );
    assert!(
        asked.contains("<scxml xmlns"),
        "the model reached the renderer: {asked}"
    );
    // The product names its figures by the document's name, so the work's own
    // name (its id without the hexadecimal suffix) is what it is handed.
    assert!(asked.contains("name=door-lock "), "{asked}");
}

/// What a work's id says of its name: the title's readable part, and not the
/// hexadecimal suffix that keeps two works of one title apart.
#[test]
fn a_works_slug_is_its_id_without_the_suffix() {
    let f = work("models-slug");
    assert_eq!(f.id.slug(), "door-lock");
    assert!(
        f.id.as_str().starts_with("door-lock-") && f.id.as_str().len() == "door-lock-".len() + 8
    );
    for (id, slug) in [
        ("door-lock-0123abcd", "door-lock"),
        ("work-ffffffff", "work"),
        ("by-hand", "by-hand"),
        ("plain", "plain"),
        ("x-0123abcg", "x-0123abcg"),
        ("x-0123abcde", "x-0123abcde"),
        ("-0123abcd", "-0123abcd"),
    ] {
        // `WorkId::parse` refuses an id that starts with a hyphen, so that one is
        // a refusal and not a slug.
        match WorkId::parse(id) {
            Ok(parsed) => assert_eq!(parsed.slug(), slug, "{id}"),
            Err(_) => assert!(id.starts_with('-'), "{id}"),
        }
    }
}

/// An argument nothing reads is a caller that thinks it said something.
#[test]
fn the_model_commands_refuse_an_argument_they_do_not_read() {
    let f = work("models-strict");
    for (name, args) in [
        (
            "save_model",
            json!({"id": f.id.as_str(), "text": "x", "extra": 1}),
        ),
        ("read_model", json!({"id": f.id.as_str(), "extra": 1})),
        ("model_history", json!({"id": f.id.as_str(), "extra": 1})),
        ("figures", json!({"id": f.id.as_str(), "extra": 1})),
    ] {
        let e = run(&f, name, args).unwrap_err();
        assert_eq!(e.kind, "bad-request", "{name}: {e:?}");
    }
}
