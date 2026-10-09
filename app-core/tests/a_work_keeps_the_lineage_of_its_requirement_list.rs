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
//! record the list is already saved in, and that no save loses it or puts another's in its place:
//!
//! * a list with a lineage reads back with it, byte for byte, and a list without one is the
//!   bytes it always was;
//! * a save without a lineage onto a list that has one is refused, by a direct save and by a
//!   published candidate alike, and leaves the work as it was;
//! * a lineage that is not one, that is not the lineage of the list it is saved with, or that
//!   does not continue the work's is refused the same way. Measured (review of 2026-10-09): the
//!   store refused only a lineage that was LOST, so a model that built its list without the
//!   lineage it was given got a fresh one, which the store kept, and the retired id was issued
//!   again to another requirement. The lineages come from the shared cases
//!   (`sce-build/tests/fixtures/revision_judgment/cases.json`), built by the authoring package
//!   from real lists, so a test does not make up hashes that happen to agree.

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

const CASES: &str = include_str!("../../sce-build/tests/fixtures/revision_judgment/cases.json");

/// A manifest and a lineage that name themselves and nothing else: enough for the record to be
/// read and written, which is all the tests that never go through a save ask of them.
const MANIFEST: &str = "{\"doc_id\":\"blind\",\"rev\":\"1\",\
                        \"requirements\":[{\"id\":\"R1\"},{\"id\":\"R2\"}]}\n";
const LINEAGE: &str = "{\"lineage\":\"sce-requirement-lineage\",\"v\":1,\"doc_id\":\"blind\",\
                       \"next\":3,\"revisions\":[],\"requirements\":[]}\n";

fn cases() -> Value {
    serde_json::from_str(CASES).expect("the cases are JSON")
}

/// The texts of one call of `scxml_requirement_set`: `first`, `second` and `third` are three
/// revisions of one specification, each continuing the one before; `again` is the first read
/// into another list, `reworded` and `back` are the first with a requirement reworded.
struct Three {
    manifest: String,
    sidecar: String,
    lineage: String,
}

fn three(name: &str) -> Three {
    let all = cases();
    let list = &all["lists"][name];
    assert!(list.is_object(), "the cases have no list called {name}");
    let text = |key: &str| list[key].as_str().expect("a text").to_string();
    Three {
        manifest: text("manifest_text"),
        sidecar: text("sidecar_text"),
        lineage: text("lineage_text"),
    }
}

/// The list `name` as the store keeps it, with the lineage that was built with it.
fn held(name: &str) -> String {
    let list = three(name);
    Requirements::new(list.manifest, Some(list.sidecar))
        .unwrap()
        .with_lineage(Some(list.lineage))
        .unwrap()
        .stored_text()
}

/// The same list without its lineage.
fn bare_of(name: &str) -> String {
    let list = three(name);
    Requirements::new(list.manifest, Some(list.sidecar))
        .unwrap()
        .stored_text()
}

/// The list `name` with a lineage it was not built with.
fn with_lineage(name: &str, lineage: &str) -> String {
    let list = three(name);
    Requirements::new(list.manifest, Some(list.sidecar))
        .unwrap()
        .with_lineage(Some(lineage.to_string()))
        .unwrap()
        .stored_text()
}

/// A list that names a lineage and nothing else: the shape of the record, for the tests of it.
fn bare() -> String {
    Requirements::new(MANIFEST.to_string(), None)
        .unwrap()
        .stored_text()
}

fn shaped(lineage: &str) -> String {
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
        .save_requirements(&id, &held("first"), None, source.as_ref())
        .unwrap();
    let read = store.read_requirements(&id, None).unwrap().expect("a list");
    let list = Requirements::parse(&read.text).unwrap();
    let first = three("first");
    assert_eq!(list.lineage.as_deref(), Some(first.lineage.as_str()));
    assert_eq!(list.manifest, first.manifest);
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
    let with = held("first");
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
            .save_requirements(&id, &held("first"), None, source.as_ref())
            .unwrap(),
    );
    let error = store
        .save_requirements(&id, &bare_of("first"), Some(&first), source.as_ref())
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
fn a_lineage_can_be_gained_and_then_continued() {
    let (store, id) = work("lineage-gained-then-continued");
    let source = store.head(&id).unwrap();
    // A first list needs none, and a list without one can be replaced by another.
    let first = saved_revision(
        store
            .save_requirements(&id, &bare_of("first"), None, source.as_ref())
            .unwrap(),
    );
    // A lineage can be gained: it is judged on its own, as the list's.
    let second = saved_revision(
        store
            .save_requirements(&id, &held("first"), Some(&first), source.as_ref())
            .unwrap(),
    );
    // And continued, step by step.
    let third = saved_revision(
        store
            .save_requirements(&id, &held("second"), Some(&second), source.as_ref())
            .unwrap(),
    );
    store
        .save_requirements(&id, &held("third"), Some(&third), source.as_ref())
        .unwrap();
}

#[test]
fn a_stale_base_is_a_conflict_before_it_is_a_lost_lineage() {
    let (store, id) = work("lineage-conflict-first");
    let source = store.head(&id).unwrap();
    store
        .save_requirements(&id, &held("first"), None, source.as_ref())
        .unwrap();
    // The writer read no list at all (base None) and writes one without a lineage: it has not
    // seen what it would lose, and is told to read again before it is told anything else.
    let error = store
        .save_requirements(&id, &bare_of("first"), None, source.as_ref())
        .unwrap_err();
    assert!(matches!(error, StoreError::Conflict { .. }), "{error:?}");
}

// -- a lineage that is not the work's to keep -------------------------------

/// What the store says of each lineage the shared cases refuse to continue. The store judges a
/// lineage in the order a person would want to be told: is it a lineage at all, is it THIS
/// list's, does it continue the work's. So two of the cases are not told as "not continued":
/// `fewer revisions` and `a next that is behind` are not lineages (a requirement names a revision
/// that is gone; `next` is below an id that was issued), and `another specification` names
/// another document than the list's. The rest are the list's own and do not continue the work's.
fn refused_as(name: &str) -> &'static str {
    match name {
        "fewer revisions" | "a next that is behind" => "lineage-unusable",
        "another specification" => "lineage-of-another-list",
        _ => "lineage-not-continued",
    }
}

/// The history a case's lineage was made from: `third, a retired id live again` is the third
/// revision's lineage with one thing changed, and its list is the third's. A lineage that has a
/// list of its own in the cases is that list's.
fn list_of<'a>(lineage_name: &'a str, lists: &Value) -> &'a str {
    if lists[lineage_name].is_object() {
        return lineage_name;
    }
    lineage_name.split(',').next().expect("a name").trim()
}

#[test]
fn what_the_cases_refuse_to_continue_is_not_saved_and_what_they_accept_is() {
    let all = cases();
    let table = &all["lineages"];
    let mut wrong = Vec::new();
    let (mut accepted, mut refused) = (0, 0);
    for (at, case) in all["extends"].as_array().unwrap().iter().enumerate() {
        let name = case["name"].as_str().unwrap();
        let (previous, following) = (
            case["previous"].as_str().unwrap(),
            case["following"].as_str().unwrap(),
        );
        let (store, id) = work(&format!("lineage-extends-{at}"));
        let source = store.head(&id).unwrap();
        let first = saved_revision(
            store
                .save_requirements(
                    &id,
                    &held(list_of(previous, &all["lists"])),
                    None,
                    source.as_ref(),
                )
                .unwrap(),
        );
        let next = with_lineage(
            list_of(following, &all["lists"]),
            &serde_json::to_string(&table[following]).unwrap(),
        );
        let saved = store.save_requirements(&id, &next, Some(&first), source.as_ref());
        match (case["expect"] == "ok", saved) {
            (true, Ok(_)) => accepted += 1,
            (false, Err(StoreError::Refused { kind, .. })) => {
                refused += 1;
                if kind != refused_as(name) {
                    wrong.push(format!(
                        "{name}: refused as {kind}, not {}",
                        refused_as(name)
                    ));
                }
            }
            (true, Err(error)) => {
                wrong.push(format!("{name}: refused, but it continues: {error:?}"))
            }
            (false, Ok(_)) => wrong.push(format!("{name}: saved, but it does not continue")),
            (false, Err(other)) => wrong.push(format!("{name}: failed another way: {other:?}")),
        }
        // A refusal leaves the work with the list it had.
        let now = store.read_requirements(&id, None).unwrap().unwrap();
        if case["expect"] != "ok" && now.revision != first {
            wrong.push(format!(
                "{name}: the work moved though the save was refused"
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} case(s) wrong:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
    // The loop judged every case the file holds, and the file holds enough of both kinds for the
    // loop to say something: a file that lost its refusals would pass the loop above for nothing.
    let cases_that_continue = all["extends"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case["expect"] == "ok")
        .count();
    assert_eq!(
        (accepted, refused),
        (
            cases_that_continue,
            all["extends"].as_array().unwrap().len() - cases_that_continue
        )
    );
    assert!(
        accepted >= 6 && refused >= 12,
        "{accepted} accepted, {refused} refused"
    );
}

#[test]
fn a_model_that_built_its_list_without_the_lineage_it_was_given_is_refused() {
    // The review's case. The work holds the lineage of its first three revisions; the model is
    // asked for a fourth list, builds it without the lineage and so gets one that starts at
    // `R1` again, which names ITS list (the lineage is well formed and the list's own) and does
    // not continue the work's. Held by `reworded`: the first revision's list with a requirement
    // reworded, whose lineage has two revisions where the work's has three.
    let (store, id) = work("lineage-fresh-start");
    let source = store.head(&id).unwrap();
    let mut head = None;
    for step in ["first", "second", "third"] {
        head = Some(saved_revision(
            store
                .save_requirements(&id, &held(step), head.as_ref(), source.as_ref())
                .unwrap(),
        ));
    }
    let error = store
        .save_requirements(&id, &held("reworded"), head.as_ref(), source.as_ref())
        .unwrap_err();
    let (kind, detail) = kind_of(error);
    assert_eq!(kind, "lineage-not-continued");
    assert_eq!(detail["revision"], json!(head.as_ref().unwrap()));
    let now = store.read_requirements(&id, None).unwrap().unwrap();
    assert_eq!(Some(&now.revision), head.as_ref(), "the work moved");
}

#[test]
fn a_lineage_that_is_another_lists_is_refused_with_the_reason() {
    let (store, id) = work("lineage-of-another-list");
    let source = store.head(&id).unwrap();
    // The first revision's lineage beside the second revision's list: both well formed, and the
    // lineage names a manifest this list is not.
    let wrong = with_lineage("second", &three("first").lineage);
    let (kind, _) = kind_of(
        store
            .save_requirements(&id, &wrong, None, source.as_ref())
            .unwrap_err(),
    );
    assert_eq!(kind, "lineage-of-another-list");
    // The same list without its sidecar names a sidecar that was not given.
    let list = three("first");
    let without = Requirements::new(list.manifest, None)
        .unwrap()
        .with_lineage(Some(list.lineage))
        .unwrap()
        .stored_text();
    let error = store
        .save_requirements(&id, &without, None, source.as_ref())
        .unwrap_err();
    match error {
        StoreError::Refused { kind, message, .. } => {
            assert_eq!(kind, "lineage-of-another-list");
            assert!(message.contains("no sidecar was given"), "{message}");
        }
        other => panic!("{other:?}"),
    }
    assert!(store.read_requirements(&id, None).unwrap().is_none());
}

#[test]
fn what_names_itself_a_lineage_and_is_not_one_is_refused_when_it_is_saved() {
    let (store, id) = work("lineage-unusable");
    let source = store.head(&id).unwrap();
    // The record reads it (it names itself), and the store judges it: no revision, no lineage.
    let (kind, _) = kind_of(
        store
            .save_requirements(&id, &shaped(LINEAGE), None, source.as_ref())
            .unwrap_err(),
    );
    assert_eq!(kind, "lineage-unusable");
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
    g.write(&first, "one", held("first"));
    g.publish(&first)
        .expect("the first generation is the work's");

    let second = g.running("press-2");
    g.write(&second, "two", bare_of("second"));
    let (kind, _) = kind_of(g.publish(&second).unwrap_err());
    assert_eq!(kind, "lineage-dropped");
    // The work still has the generation it had, with its lineage.
    let list = g.store.read_requirements(&g.id, None).unwrap().unwrap();
    assert!(Requirements::parse(&list.text).unwrap().lineage.is_some());

    // The request is still the executor's: it writes the list again with the lineage, and
    // the same request is published.
    g.write(&second, "two", held("second"));
    g.publish(&second).expect("with a lineage it is published");
    let list = g.store.read_requirements(&g.id, None).unwrap().unwrap();
    assert_eq!(
        Requirements::parse(&list.text).unwrap().lineage.as_deref(),
        Some(three("second").lineage.as_str())
    );
}

#[test]
fn a_candidate_whose_lineage_does_not_continue_the_works_is_not_published() {
    // The application's own path, and the one the review reproduced: a model builds its list
    // without the lineage it was given, the lineage it gets starts again, and the store kept it.
    let g = generation("lineage-not-continued-bundle");
    for (key, step) in [
        ("press-1", "first"),
        ("press-2", "second"),
        ("press-3", "third"),
    ] {
        let request = g.running(key);
        g.write(&request, step, held(step));
        g.publish(&request).expect("each continues the one before");
    }
    let fourth = g.running("press-4");
    g.write(&fourth, "four", held("reworded"));
    let (kind, _) = kind_of(g.publish(&fourth).unwrap_err());
    assert_eq!(kind, "lineage-not-continued");
    let list = g.store.read_requirements(&g.id, None).unwrap().unwrap();
    assert_eq!(
        Requirements::parse(&list.text).unwrap().lineage.as_deref(),
        Some(three("third").lineage.as_str()),
        "the work kept the lineage it had"
    );
    // The request is still the executor's, and a list that continues is published.
    g.write(&fourth, "four", held("third"));
    g.publish(&fourth)
        .expect("a lineage that continues is the work's");
}

#[test]
fn a_candidate_with_the_lineage_of_another_list_is_not_published() {
    let g = generation("lineage-of-another-list-bundle");
    let first = g.running("press-1");
    g.write(
        &first,
        "one",
        with_lineage("second", &three("first").lineage),
    );
    let (kind, _) = kind_of(g.publish(&first).unwrap_err());
    assert_eq!(kind, "lineage-of-another-list");
    g.write(&first, "one", held("second"));
    g.publish(&first)
        .expect("the list with its own lineage is published");
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
    let first = three("first");
    let saved = command(
        &store,
        "save_requirements",
        json!({"id": id, "manifest": first.manifest, "sidecar": first.sidecar,
               "lineage": first.lineage, "written_for": source}),
    )
    .unwrap();
    let read = command(&store, "read_requirements", json!({"id": id})).unwrap();
    assert_eq!(
        read["requirements"]["lineage"],
        json!(first.lineage),
        "{read}"
    );

    let refused = command(
        &store,
        "save_requirements",
        json!({"id": id, "manifest": first.manifest, "sidecar": first.sidecar,
               "base": saved["revision"], "written_for": source}),
    )
    .unwrap_err();
    assert_eq!(refused.kind, "lineage-dropped", "{refused:?}");

    // A list that does not continue the work's is refused through the command in its own words.
    let reworded = three("reworded");
    let refused = command(
        &store,
        "save_requirements",
        json!({"id": id, "manifest": reworded.manifest, "sidecar": reworded.sidecar,
               "lineage": reworded.lineage, "base": saved["revision"], "written_for": source}),
    );
    // Two revisions where the work's has one is a continuation: it is the first revision's
    // lineage with a second revision after it.
    assert!(refused.is_ok(), "{refused:?}");
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

// -- a lineage the store cannot read ---------------------------------------

/// A work whose requirement list was saved twice: first the list `reworded then added`, which
/// used the ids R1 to R6, and then a list whose lineage this build cannot read (a newer version,
/// or damage), which names only R1 to R5. A save cannot make that state (the store judges a
/// lineage at the door), so it is written the way a store written by another build is: the
/// text under its own digest, its line in the log, and the pointer that makes it current.
/// Returns the store, the work, the source revision and the revision the work now holds.
fn work_whose_lineage_cannot_be_read(
    label: &str,
) -> (WorkStore<FixedClock>, WorkId, Revision, Revision) {
    let root = common::scratch(label);
    let store = WorkStore::with_clock(&root, FixedClock("2026-10-08T09:00:00Z".to_string()));
    let id = store.create_work("Window blind").unwrap().id;
    let source = saved_revision(
        store
            .save_source(&id, "The blind rises. The blind stops at the top.", None)
            .unwrap(),
    );
    let used = saved_revision(
        store
            .save_requirements(&id, &held("reworded then added"), None, Some(&source))
            .unwrap(),
    );
    let unreadable = {
        let list = three("first");
        Requirements::new(list.manifest, Some(list.sidecar))
            .unwrap()
            .with_lineage(Some(LINEAGE.to_string()))
            .unwrap()
            .stored_text()
    };
    let head = Revision::of(unreadable.as_bytes());
    let dir = root.join(id.as_str());
    std::fs::write(
        dir.join("requirements").join(format!("{head}.json")),
        &unreadable,
    )
    .unwrap();
    let line = json!({"revision": head, "parent": used, "saved_at": "2026-10-08T09:00:00Z",
                      "parent_at": 0});
    let log = dir.join("requirements.log");
    let mut text = std::fs::read_to_string(&log).unwrap();
    text.push_str(&format!("{line}\n"));
    std::fs::write(&log, text).unwrap();
    std::fs::write(dir.join("requirements.head"), format!("{head}\nlog 1\n")).unwrap();
    (store, id, source, head)
}

#[test]
fn the_planted_state_is_the_one_the_tests_below_mean() {
    // Without this the refusals below could be passing for a state that is not the one named: the
    // work holds the unreadable lineage, and the list it replaced is still kept.
    let (store, id, _, head) = work_whose_lineage_cannot_be_read("lineage-unreadable-planted");
    let (revision, text) = store
        .read_requirements(&id, None)
        .unwrap()
        .map(|read| (read.revision, read.text))
        .expect("the work has a list");
    assert_eq!(revision, head);
    let list = Requirements::parse(&text).unwrap();
    assert_eq!(list.lineage.as_deref(), Some(LINEAGE));
    assert!(
        sce_revision::parse(LINEAGE).is_err(),
        "the lineage is one this build reads"
    );
}

#[test]
fn a_lineage_that_numbers_from_an_id_the_work_already_issued_is_refused() {
    let (store, id, source, head) = work_whose_lineage_cannot_be_read("lineage-unreadable-reuse");
    // The first revision's lineage numbers on from R6, and the work's lists have used R6. The held
    // lineage cannot be read, so the new one is not compared with it, but it can be compared with
    // what the lists the work kept used.
    let error = store
        .save_requirements(&id, &held("first"), Some(&head), Some(&source))
        .unwrap_err();
    match error {
        StoreError::Refused {
            kind,
            message,
            detail,
        } => {
            assert_eq!(kind, "lineage-numbers-reused");
            assert_eq!(detail["floor"], json!(6), "{detail}");
            assert_eq!(detail["next"], json!(6), "{detail}");
            // It says what to do: a client cannot choose where the numbering starts any other way.
            assert!(message.contains("next_at_least"), "{message}");
            assert!(message.contains("7"), "{message}");
        }
        other => panic!("{other:?}"),
    }
    // Nothing was saved: the work still holds what it held.
    let held_now = store
        .read_requirements(&id, None)
        .unwrap()
        .unwrap()
        .revision;
    assert_eq!(held_now, head);
}

#[test]
fn a_lineage_that_numbers_past_every_id_the_work_issued_is_kept_though_the_held_one_is_unreadable()
{
    // Otherwise a work whose lineage a newer build wrote could never be saved by this one.
    let (store, id, source, head) = work_whose_lineage_cannot_be_read("lineage-unreadable-past");
    let saved = store
        .save_requirements(&id, &held("third"), Some(&head), Some(&source))
        .unwrap();
    assert!(
        matches!(saved, sce_app_core::Saved::Saved { .. }),
        "{saved:?}"
    );
}

#[test]
fn a_work_that_never_had_a_lineage_is_not_held_to_the_ids_of_its_lists() {
    // The weaker guarantee ADR 0011 names and ADR 0012 keeps: with no lineage held, what the lists
    // used before cannot be said to be retired or live, so the first lineage is judged on its own.
    let (store, id) = work("lineage-never-had");
    let source = store.head(&id).unwrap();
    let first = saved_revision(
        store
            .save_requirements(&id, &bare_of("reworded then added"), None, source.as_ref())
            .unwrap(),
    );
    let saved = store
        .save_requirements(&id, &held("first"), Some(&first), source.as_ref())
        .unwrap();
    assert!(
        matches!(saved, sce_app_core::Saved::Saved { .. }),
        "{saved:?}"
    );
}
