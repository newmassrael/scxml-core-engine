// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A work keeps the lineage its requirement list was built against, and does not lose it.
//!
//! The lineage is what makes an id mean one requirement across the revisions of a work's
//! text: it remembers which ids were issued and which were retired, so that a retired id is
//! never issued again (`docs/adr/0011-a-work-keeps-its-requirement-lineage-with-its-requirement-list.md`).
//! A work that hands a client back only a manifest and a sidecar cannot carry that memory, and
//! measured on three revisions of a three-sentence text the third gave the sentence dropped in
//! the second its id again. These tests hold that the memory is kept with the list, in the
//! record the list is already saved in, and that no save loses it:
//!
//! * a list with a lineage reads back with it, byte for byte, and a list without one is the
//!   bytes it always was;
//! * a save without a lineage onto a list that has one is refused, by a direct save and by a
//!   published candidate alike, and leaves the work as it was;
//! * the refusal is only ever about losing one: a first list, a list onto one without, and a
//!   list with another lineage are saved as ever.

mod common;

use std::sync::Arc;

use sce_app_core::bundle::{BundleCheck, CheckedBy};
use sce_app_core::requests::Inputs;
use sce_app_core::{
    call, CandidateWrite, CommandError, FixedClock, ManualClock, Registration, Requirements,
    Revision, StoreError, WorkId, WorkStore,
};
use serde_json::{json, Value};

use common::FakeRenderer;

const MANIFEST: &str = "{\"doc_id\":\"blind\",\"rev\":\"1\",\
                        \"requirements\":[{\"id\":\"R1\"},{\"id\":\"R2\"}]}\n";
const LINEAGE: &str = "{\"lineage\":\"sce-requirement-lineage\",\"v\":1,\"doc_id\":\"blind\",\
                       \"next\":3,\"revisions\":[],\"requirements\":[]}\n";
const NEXT_LINEAGE: &str = "{\"lineage\":\"sce-requirement-lineage\",\"v\":1,\"doc_id\":\"blind\",\
                            \"next\":4,\"revisions\":[],\"requirements\":[]}\n";

fn bare() -> String {
    Requirements::new(MANIFEST.to_string(), None)
        .unwrap()
        .stored_text()
}

fn held(lineage: &str) -> String {
    Requirements::new(MANIFEST.to_string(), None)
        .unwrap()
        .with_lineage(Some(lineage.to_string()))
        .unwrap()
        .stored_text()
}

fn kind_of(error: StoreError) -> (&'static str, Value) {
    match error {
        StoreError::Refused { kind, detail, .. } => (kind, detail),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn work(label: &str) -> (WorkStore<FixedClock>, WorkId) {
    let store = WorkStore::with_clock(
        common::scratch(label),
        FixedClock("2026-10-08T09:00:00Z".to_string()),
    );
    let id = store.create_work("Window blind").unwrap().id;
    store
        .save_source(&id, "The blind rises. The blind stops at the top.", None)
        .unwrap();
    (store, id)
}

fn saved_revision(saved: sce_app_core::Saved) -> Revision {
    match saved {
        sce_app_core::Saved::Saved { revision, .. }
        | sce_app_core::Saved::Unchanged { revision } => revision,
    }
}

// -- the record -------------------------------------------------------------

#[test]
fn a_list_with_a_lineage_reads_back_with_it_byte_for_byte() {
    let (store, id) = work("lineage-record");
    let source = store.head(&id).unwrap();
    store
        .save_requirements(&id, &held(LINEAGE), None, source.as_ref())
        .unwrap();
    let read = store.read_requirements(&id, None).unwrap().expect("a list");
    let list = Requirements::parse(&read.text).unwrap();
    assert_eq!(list.lineage.as_deref(), Some(LINEAGE));
    assert_eq!(list.manifest, MANIFEST);
}

#[test]
fn a_list_without_a_lineage_is_the_bytes_it_always_was() {
    // Written out here, field by field, as the store wrote a list before lineages: a digest
    // already kept, and an acceptance that pins one, name these bytes. A comparison of the
    // code with itself would hold nothing.
    let before = format!(
        "{{\n  \"format\": \"sce-requirements\",\n  \"v\": 1,\n  \"manifest\": {},\n  \
         \"sidecar\": null\n}}\n",
        serde_json::to_string(MANIFEST).unwrap()
    );
    assert_eq!(bare(), before);
}

/// What a build that predates lineages reads a list as: the same shape, and strict about it.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct AnOlderBuildsList {
    format: String,
    v: u32,
    manifest: String,
    sidecar: Option<String>,
}

#[test]
fn a_build_that_predates_lineages_refuses_a_list_that_has_one_and_does_not_read_it_without() {
    // Emulated: the older shape is written out above, not run. It refuses on an unknown field,
    // which is what `deny_unknown_fields` made the record's reader do before lineages, so a list
    // with a lineage is not read by it as a list with none, and its lineage lost at the next save.
    let with = held(LINEAGE);
    assert!(
        serde_json::from_str::<AnOlderBuildsList>(&with).is_err(),
        "an older build read a list with a lineage"
    );
    // The control: the same reader takes a list without one.
    assert!(serde_json::from_str::<AnOlderBuildsList>(&bare()).is_ok());
}

// -- a direct save ----------------------------------------------------------

#[test]
fn a_list_without_a_lineage_cannot_replace_one_with_it() {
    let (store, id) = work("lineage-dropped-direct");
    let source = store.head(&id).unwrap();
    let first = saved_revision(
        store
            .save_requirements(&id, &held(LINEAGE), None, source.as_ref())
            .unwrap(),
    );
    let error = store
        .save_requirements(&id, &bare(), Some(&first), source.as_ref())
        .unwrap_err();
    let (kind, detail) = kind_of(error);
    assert_eq!(kind, "lineage-dropped");
    assert_eq!(detail["revision"], json!(first));
    // Nothing was written: the list the work has is the list it had.
    let now = store.read_requirements(&id, None).unwrap().unwrap();
    assert_eq!(now.revision, first);
    assert!(Requirements::parse(&now.text).unwrap().lineage.is_some());
}

#[test]
fn the_refusal_is_only_about_losing_a_lineage() {
    let (store, id) = work("lineage-not-dropped");
    let source = store.head(&id).unwrap();
    // A first list needs none, and a list without one can be replaced by another.
    let first = saved_revision(
        store
            .save_requirements(&id, &bare(), None, source.as_ref())
            .unwrap(),
    );
    // A lineage can be gained.
    let second = saved_revision(
        store
            .save_requirements(&id, &held(LINEAGE), Some(&first), source.as_ref())
            .unwrap(),
    );
    // And another lineage replaces it: that it follows the one it replaces is the authoring
    // package's to judge, not the store's.
    store
        .save_requirements(&id, &held(NEXT_LINEAGE), Some(&second), source.as_ref())
        .unwrap();
}

#[test]
fn a_stale_base_is_a_conflict_before_it_is_a_lost_lineage() {
    let (store, id) = work("lineage-conflict-first");
    let source = store.head(&id).unwrap();
    store
        .save_requirements(&id, &held(LINEAGE), None, source.as_ref())
        .unwrap();
    // The writer read no list at all (base None) and writes one without a lineage: it has not
    // seen what it would lose, and is told to read again before it is told anything else.
    let error = store
        .save_requirements(&id, &bare(), None, source.as_ref())
        .unwrap_err();
    assert!(matches!(error, StoreError::Conflict { .. }), "{error:?}");
}

// -- a published candidate --------------------------------------------------

struct Generation {
    store: WorkStore<Arc<ManualClock>>,
    id: WorkId,
}

fn generation(label: &str) -> Generation {
    let clock = Arc::new(ManualClock::at(1_791_190_800));
    let store = WorkStore::with_clock(common::scratch(label), clock);
    let id = store.create_work("Window blind").unwrap().id;
    store
        .save_source(&id, "The blind rises. The blind stops at the top.", None)
        .unwrap();
    Generation { store, id }
}

fn core_check() -> BundleCheck {
    BundleCheck {
        by: CheckedBy::Core,
        name: "model".to_string(),
        verdict: "accepted".to_string(),
        generator: Some("sce-codegen 0".to_string()),
        digest: Some(Revision::of(b"record").to_string()),
        subject: None,
    }
}

impl Generation {
    fn running(&self, key: &str) -> String {
        let source = self.store.head(&self.id).unwrap().expect("a text");
        let made = self
            .store
            .register_request(
                &self.id,
                Registration {
                    key,
                    origin: "gui",
                    expect: Inputs {
                        source,
                        answers: None,
                    },
                    supersede: false,
                },
            )
            .unwrap();
        self.store
            .claim_request(&self.id, &made.request.id, "adapter-a", None, false)
            .unwrap();
        made.request.id
    }

    fn write(&self, request: &str, tag: &str, list: String) {
        self.store
            .save_candidate(
                &self.id,
                request,
                "adapter-a",
                1,
                CandidateWrite {
                    model: Some(format!("<scxml>{tag}</scxml>")),
                    requirements: Some(list),
                    instructions: None,
                },
            )
            .unwrap();
    }

    fn publish(&self, request: &str) -> Result<sce_app_core::Published, StoreError> {
        self.store
            .publish_candidate(&self.id, request, "adapter-a", 1, vec![core_check()])
    }
}

#[test]
fn a_candidate_that_loses_the_lineage_is_not_published_and_stays_the_executors() {
    let g = generation("lineage-dropped-bundle");
    let first = g.running("press-1");
    g.write(&first, "one", held(LINEAGE));
    g.publish(&first)
        .expect("the first generation is the work's");

    let second = g.running("press-2");
    g.write(&second, "two", bare());
    let (kind, _) = kind_of(g.publish(&second).unwrap_err());
    assert_eq!(kind, "lineage-dropped");
    // The work still has the generation it had, with its lineage.
    let list = g.store.read_requirements(&g.id, None).unwrap().unwrap();
    assert!(Requirements::parse(&list.text).unwrap().lineage.is_some());

    // The request is still the executor's: it writes the list again with the lineage, and
    // the same request is published.
    g.write(&second, "two", held(NEXT_LINEAGE));
    g.publish(&second).expect("with a lineage it is published");
    let list = g.store.read_requirements(&g.id, None).unwrap().unwrap();
    assert_eq!(
        Requirements::parse(&list.text).unwrap().lineage.as_deref(),
        Some(NEXT_LINEAGE)
    );
}

#[test]
fn a_work_whose_lists_never_had_a_lineage_publishes_as_ever() {
    let g = generation("lineage-never-had-one");
    let first = g.running("press-1");
    g.write(&first, "one", bare());
    g.publish(&first).unwrap();
    let second = g.running("press-2");
    g.write(&second, "two", bare());
    g.publish(&second).unwrap();
}

// -- the commands -----------------------------------------------------------

fn command(store: &WorkStore<FixedClock>, name: &str, args: Value) -> Result<Value, CommandError> {
    call(store, &FakeRenderer, name, args)
}

#[test]
fn the_commands_save_and_read_the_lineage_and_refuse_its_loss() {
    let (store, id) = work("lineage-commands");
    let source = store.head(&id).unwrap().unwrap();
    let saved = command(
        &store,
        "save_requirements",
        json!({"id": id, "manifest": MANIFEST, "lineage": LINEAGE, "written_for": source}),
    )
    .unwrap();
    let read = command(&store, "read_requirements", json!({"id": id})).unwrap();
    assert_eq!(read["requirements"]["lineage"], json!(LINEAGE), "{read}");

    let refused = command(
        &store,
        "save_requirements",
        json!({"id": id, "manifest": MANIFEST, "base": saved["revision"], "written_for": source}),
    )
    .unwrap_err();
    assert_eq!(refused.kind, "lineage-dropped", "{refused:?}");
}

#[test]
fn a_list_that_has_no_lineage_says_nothing_of_one() {
    // `lineage` is absent from the reply and not `null`: the reply of every list that never had
    // one is the reply it always was, so a screen written before lineages reads it unchanged.
    let (store, id) = work("lineage-reply-absent");
    command(
        &store,
        "save_requirements",
        json!({"id": id, "manifest": MANIFEST}),
    )
    .unwrap();
    let read = command(&store, "read_requirements", json!({"id": id})).unwrap();
    assert!(read["requirements"].get("lineage").is_none(), "{read}");
}

#[test]
fn what_is_not_a_lineage_is_refused_as_an_invalid_list() {
    let (store, id) = work("lineage-invalid");
    for bad in ["not json", "[]", "{}", "{\"lineage\":\"another-kind\"}"] {
        let refused = command(
            &store,
            "save_requirements",
            json!({"id": id, "manifest": MANIFEST, "lineage": bad}),
        )
        .unwrap_err();
        assert_eq!(refused.kind, "invalid-requirements", "{bad}: {refused:?}");
    }
}

#[test]
fn a_candidate_carries_its_lineage_and_a_lineage_belongs_to_a_manifest() {
    let g = generation("lineage-candidate-command");
    let request = g.running("press-1");
    let candidate = command_by(
        &g.store,
        "save_request_candidate",
        json!({"id": g.id, "request": request, "holder": "adapter-a", "attempt": 1,
               "manifest": MANIFEST, "lineage": LINEAGE}),
    )
    .unwrap();
    assert!(candidate["request"].is_object(), "{candidate}");
    let texts = g.store.read_candidate(&g.id, &request).unwrap();
    let (_, text) = texts.requirements.expect("the list was written");
    assert_eq!(
        Requirements::parse(&text).unwrap().lineage.as_deref(),
        Some(LINEAGE)
    );

    let refused = command_by(
        &g.store,
        "save_request_candidate",
        json!({"id": g.id, "request": request, "holder": "adapter-a", "attempt": 1,
               "lineage": LINEAGE}),
    )
    .unwrap_err();
    assert_eq!(refused.kind, "bad-request", "{refused:?}");
}

fn command_by(
    store: &WorkStore<Arc<ManualClock>>,
    name: &str,
    args: Value,
) -> Result<Value, CommandError> {
    call(store, &FakeRenderer, name, args)
}
