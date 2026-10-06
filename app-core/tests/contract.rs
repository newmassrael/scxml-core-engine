// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The command layer's replies, written down once.
//!
//! `contract/replies.json` is every command's answer and every refusal's shape,
//! produced by running the commands against a real works folder with a fixed
//! clock. Two readers hold it to account:
//!
//! - this test, which runs the commands again and fails if the replies differ from
//!   the file, so the core cannot change an answer without the file changing;
//! - the screen's test (`app/ui/test/contract.test.ts`), which feeds the same file
//!   to the screen's guards, so the screen cannot expect an answer the core does
//!   not give.
//!
//! A change to a reply is therefore visible in review as a change to one JSON
//! file, and a screen written against the old shape fails its own test.
//!
//! To accept a deliberate change: `SCE_UPDATE_CONTRACT=1 cargo test -p sce-app-core --test contract`.

mod common;

use sce_app_core::claude_status::{AccountState, Billing, ClaudeStatus, Client, SIGN_IN};
use sce_app_core::{
    call, call_in, CommandError, ConnectionStore, Context, Entrance, FixedClock, NoRenderer,
    Policy, Product, Route, WorkStore, COMMANDS, COMMAND_SET_VERSION,
};
use serde_json::{json, Map, Value};

use common::{FakeRenderer, RefusingRenderer};

const FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/contract/replies.json");

/// Replace `from` by `to` in every string of `value`: what varies between runs
/// (the scratch folder, the generated work id) becomes a name.
fn name_the_unstable(value: &mut Value, from: &str, to: &str) {
    match value {
        Value::String(s) => {
            if s.contains(from) {
                *s = s.replace(from, to);
            }
        }
        Value::Array(items) => items
            .iter_mut()
            .for_each(|v| name_the_unstable(v, from, to)),
        Value::Object(fields) => fields
            .values_mut()
            .for_each(|v| name_the_unstable(v, from, to)),
        _ => {}
    }
}

fn answer(store: &WorkStore<FixedClock>, name: &str, args: Value) -> Value {
    call(store, &FakeRenderer, name, args).unwrap_or_else(|e| panic!("{name} was refused: {e:?}"))
}

fn answer_by(
    store: &WorkStore<FixedClock>,
    renderer: &dyn Product,
    name: &str,
    args: Value,
) -> Value {
    call(store, renderer, name, args).unwrap_or_else(|e| panic!("{name} was refused: {e:?}"))
}

fn refusal(store: &WorkStore<FixedClock>, name: &str, args: Value) -> Value {
    refusal_by(store, &FakeRenderer, name, args)
}

fn refusal_by(
    store: &WorkStore<FixedClock>,
    renderer: &dyn Product,
    name: &str,
    args: Value,
) -> Value {
    let error: CommandError =
        call(store, renderer, name, args).expect_err("the command should be refused");
    serde_json::to_value(error).unwrap()
}

fn replies() -> Value {
    let root = common::scratch("contract");
    let store = WorkStore::with_clock(&root, FixedClock("2026-10-03T09:00:00Z".to_string()));
    let mut answers = Map::new();
    let mut refusals = Map::new();

    answers.insert("describe".into(), answer(&store, "describe", json!({})));
    answers.insert(
        "list_works_empty".into(),
        answer(&store, "list_works", json!({})),
    );
    let work = answer(&store, "create_work", json!({"title": "Door lock"}));
    let id = work["id"].as_str().expect("a work id").to_string();
    answers.insert("create_work".into(), work);
    answers.insert("list_works".into(), answer(&store, "list_works", json!({})));
    answers.insert(
        "read_work_no_text".into(),
        answer(&store, "read_work", json!({"id": id})),
    );
    answers.insert(
        "read_work_snapshot_empty".into(),
        answer(&store, "read_work_snapshot", json!({"id": id})),
    );
    answers.insert(
        "read_work_heads_empty".into(),
        answer(&store, "read_work_heads", json!({"id": id})),
    );
    answers.insert(
        "read_source_none".into(),
        answer(&store, "read_source", json!({"id": id})),
    );

    let first = answer(
        &store,
        "save_source",
        json!({"id": id, "text": "The lock opens when the code matches."}),
    );
    let first_revision = first["revision"].as_str().unwrap().to_string();
    answers.insert("save_source_first".into(), first);
    answers.insert(
        "save_source_unchanged".into(),
        answer(
            &store,
            "save_source",
            json!({"id": id, "text": "The lock opens when the code matches.", "base": first_revision}),
        ),
    );
    answers.insert(
        "save_source_next".into(),
        answer(
            &store,
            "save_source",
            json!({"id": id, "text": "The lock opens when the code matches. Three misses lock it.", "base": first_revision}),
        ),
    );
    answers.insert(
        "read_work".into(),
        answer(&store, "read_work", json!({"id": id})),
    );
    answers.insert(
        "read_source".into(),
        answer(&store, "read_source", json!({"id": id})),
    );
    answers.insert(
        "read_source_revision".into(),
        answer(
            &store,
            "read_source",
            json!({"id": id, "revision": first_revision}),
        ),
    );
    answers.insert(
        "history".into(),
        answer(&store, "history", json!({"id": id})),
    );

    // The model: none yet, then saved for the first text, then read as the text
    // moved on, then kept for the new one.
    answers.insert(
        "read_model_none".into(),
        answer(&store, "read_model", json!({"id": id})),
    );
    let model = "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" version=\"1.0\"/>";
    let model_first = answer(
        &store,
        "save_model",
        json!({"id": id, "text": model, "written_for": first_revision}),
    );
    let model_revision = model_first["revision"].as_str().unwrap().to_string();
    answers.insert("save_model_first".into(), model_first);
    answers.insert(
        "save_model_unchanged".into(),
        answer(
            &store,
            "save_model",
            json!({"id": id, "text": model, "base": model_revision, "written_for": first_revision}),
        ),
    );
    answers.insert(
        "read_model".into(),
        answer(&store, "read_model", json!({"id": id})),
    );
    answers.insert(
        "figures".into(),
        answer(
            &store,
            "figures",
            json!({"id": id, "page": "a4-portrait", "lexicon": "en", "min_pt": 7}),
        ),
    );
    let source_head = answer(&store, "read_work", json!({"id": id}))["head"]
        .as_str()
        .unwrap()
        .to_string();
    answers.insert(
        "review".into(),
        answer(&store, "review", json!({"id": id, "lexicon": "en"})),
    );
    answers.insert(
        "save_model_kept".into(),
        answer(
            &store,
            "save_model",
            json!({"id": id, "text": model, "base": model_revision, "written_for": source_head}),
        ),
    );
    answers.insert(
        "read_model_kept".into(),
        answer(&store, "read_model", json!({"id": id})),
    );
    answers.insert(
        "model_history".into(),
        answer(&store, "model_history", json!({"id": id})),
    );

    // The owner's answers: none yet, then saved, then read, then said again.
    answers.insert(
        "read_answers_none".into(),
        answer(&store, "read_answers", json!({"id": id})),
    );
    let answered = answer(
        &store,
        "save_answers",
        json!({"id": id, "answers": {"open-guard": "Any card on the list opens it."}}),
    );
    let answers_revision = answered["revision"].as_str().unwrap().to_string();
    answers.insert("save_answers_first".into(), answered);
    answers.insert(
        "save_answers_unchanged".into(),
        answer(
            &store,
            "save_answers",
            json!({"id": id, "answers": {"open-guard": "Any card on the list opens it."},
                   "base": answers_revision}),
        ),
    );
    answers.insert(
        "read_answers".into(),
        answer(&store, "read_answers", json!({"id": id})),
    );
    // The work as one state: this one has a text, a model and answers, and no
    // requirement list and no acceptance.
    answers.insert(
        "read_work_snapshot".into(),
        answer(&store, "read_work_snapshot", json!({"id": id})),
    );
    answers.insert(
        "read_work_heads".into(),
        answer(&store, "read_work_heads", json!({"id": id})),
    );
    refusals.insert(
        "invalid-answers".into(),
        refusal(
            &store,
            "save_answers",
            json!({"id": id, "answers": {"open-guard": "  "}, "base": answers_revision}),
        ),
    );

    // A removal, on a work of its own so the one above keeps its place in every
    // reply that follows.
    let other = answer(&store, "create_work", json!({"title": "Window blind"}));
    let other_id = other["id"].as_str().expect("a work id").to_string();
    // What SCE says of a model it refuses, and of one it accepts and cannot write
    // the page of, each on this work before it is removed.
    let refused_model = answer(
        &store,
        "save_model",
        json!({"id": other_id, "text": "<scxml>REFUSE</scxml>"}),
    );
    answers.insert(
        "review_refused".into(),
        answer(&store, "review", json!({"id": other_id})),
    );
    answer(
        &store,
        "save_model",
        json!({"id": other_id, "text": "<scxml>NOPAGE</scxml>",
               "base": refused_model["revision"]}),
    );
    let one_document = answer(&store, "read_model", json!({"id": other_id}));
    answers.insert(
        "review_no_page".into(),
        answer(&store, "review", json!({"id": other_id})),
    );
    // A model of several documents: a statechart and the event schema it imports.
    let model_set = answer(
        &store,
        "save_model",
        json!({"id": other_id, "entry": "door.scxml",
               "documents": [
                   {"name": "open.scxml", "text": "<scxml/>"},
                   {"name": "door.scxml", "text": "<scxml>NOPAGE</scxml>"}],
               "base": one_document["model"]["revision"]}),
    );
    answers.insert("save_model_set".into(), model_set);
    answers.insert(
        "read_model_set".into(),
        answer(&store, "read_model", json!({"id": other_id})),
    );
    answers.insert(
        "remove_work".into(),
        answer(&store, "remove_work", json!({"id": other_id})),
    );
    answers.insert(
        "list_works_after_removal".into(),
        answer(&store, "list_works", json!({})),
    );
    refusals.insert(
        "removed-work".into(),
        refusal(&store, "read_work", json!({"id": other_id})),
    );

    refusals.insert(
        "invalid-model".into(),
        refusal(
            &store,
            "save_model",
            json!({"id": id, "documents": [{"name": "../escape.scxml", "text": "x"}],
                   "base": model_revision}),
        ),
    );
    refusals.insert(
        "model-conflict".into(),
        refusal(
            &store,
            "save_model",
            json!({"id": id, "text": "<scxml/>", "base": null}),
        ),
    );
    refusals.insert(
        "model-for-an-unknown-text".into(),
        refusal(
            &store,
            "save_model",
            json!({"id": id, "text": "<scxml/>", "base": model_revision,
                   "written_for": "0000000000000000000000000000000000000000000000000000000000000000"}),
        ),
    );
    refusals.insert(
        "sce-refused".into(),
        refusal_by(&store, &RefusingRenderer, "figures", json!({"id": id})),
    );
    refusals.insert(
        "sce-unavailable".into(),
        refusal_by(&store, &NoRenderer, "figures", json!({"id": id})),
    );

    refusals.insert(
        "conflict".into(),
        refusal(
            &store,
            "save_source",
            json!({"id": id, "text": "mine", "base": first_revision}),
        ),
    );
    refusals.insert(
        "not-found".into(),
        refusal(&store, "read_work", json!({"id": "absent"})),
    );
    refusals.insert(
        "invalid-id".into(),
        refusal(&store, "read_work", json!({"id": "../x"})),
    );
    refusals.insert(
        "invalid-title".into(),
        refusal(&store, "create_work", json!({"title": ""})),
    );
    refusals.insert(
        "bad-request".into(),
        refusal(&store, "list_works", json!({"extra": 1})),
    );
    refusals.insert("unknown-command".into(), refusal(&store, "nope", json!({})));

    // What the owner accepts: a work of its own (a text, a design that leaves a question
    // open, the requirement list the text was read into), measured, accepted, and then
    // moved under the acceptance. Its replies carry the revisions the owner was shown.
    let accepted_work = answer(&store, "create_work", json!({"title": "Garage door"}));
    let accepted_id = accepted_work["id"].as_str().expect("a work id").to_string();
    let accepted_text = answer(
        &store,
        "save_source",
        json!({"id": accepted_id, "text": "The door opens for a listed card."}),
    );
    let accepted_head = accepted_text["revision"].as_str().unwrap().to_string();
    let accepted_model = answer(
        &store,
        "save_model",
        json!({"id": accepted_id, "text": "<scxml><!-- OPEN --></scxml>",
               "written_for": accepted_head}),
    );
    answers.insert(
        "read_requirements_none".into(),
        answer(&store, "read_requirements", json!({"id": accepted_id})),
    );
    answers.insert(
        "read_acceptance_none".into(),
        answer(&store, "read_acceptance", json!({"id": accepted_id})),
    );
    let manifest = "{\"doc_id\":\"door\",\"rev\":\"1\",\
                    \"requirements\":[{\"id\":\"R1\"},{\"id\":\"R2\"}]}\n";
    let sidecar = "{\"doc_id\":\"door\",\"rev\":\"1\",\"text\":{\"R1\":\"The door opens.\"}}\n";
    let listed = answer(
        &store,
        "save_requirements",
        json!({"id": accepted_id, "manifest": manifest, "sidecar": sidecar,
               "written_for": accepted_head}),
    );
    let list_revision = listed["revision"].as_str().unwrap().to_string();
    answers.insert("save_requirements_first".into(), listed);
    answers.insert(
        "save_requirements_unchanged".into(),
        answer(
            &store,
            "save_requirements",
            json!({"id": accepted_id, "manifest": manifest, "sidecar": sidecar,
                   "base": list_revision, "written_for": accepted_head}),
        ),
    );
    answers.insert(
        "read_requirements".into(),
        answer(&store, "read_requirements", json!({"id": accepted_id})),
    );
    let report = answer(&store, "requirements_report", json!({"id": accepted_id}));
    let shown = report["basis"].clone();
    answers.insert("requirements_report".into(), report);
    // What SCE says of the revisions that were read: measured, and nothing accepted yet.
    answers.insert(
        "read_judgment_unaccepted".into(),
        answer(
            &store,
            "read_judgment",
            json!({"id": accepted_id, "basis": shown}),
        ),
    );
    answers.insert(
        "accept".into(),
        answer(
            &store,
            "accept",
            json!({"id": accepted_id, "expect": shown}),
        ),
    );
    let held = answer(&store, "read_acceptance", json!({"id": accepted_id}));
    let acceptance_revision = held["acceptance"]["revision"].clone();
    answers.insert("read_acceptance".into(), held);
    // The acceptance holds for the revisions it was taken of.
    answers.insert(
        "read_judgment_accepted".into(),
        answer(
            &store,
            "read_judgment",
            json!({"id": accepted_id, "basis": shown, "acceptance": acceptance_revision}),
        ),
    );
    // SCE does not answer: the revisions are the caller's, and the acceptance and the
    // measure each say they were not answered.
    answers.insert(
        "read_judgment_unanswered".into(),
        answer_by(
            &store,
            &RefusingRenderer,
            "read_judgment",
            json!({"id": accepted_id, "basis": shown, "acceptance": acceptance_revision}),
        ),
    );
    // This one has a text, a model, a requirement list and an acceptance, and no answers.
    answers.insert(
        "read_work_snapshot_accepted".into(),
        answer(&store, "read_work_snapshot", json!({"id": accepted_id})),
    );
    answers.insert(
        "read_work_heads_accepted".into(),
        answer(&store, "read_work_heads", json!({"id": accepted_id})),
    );
    refusals.insert(
        "invalid-requirements".into(),
        refusal(
            &store,
            "save_requirements",
            json!({"id": accepted_id, "manifest": "not json", "base": list_revision}),
        ),
    );
    // The design moves under the acceptance, and the owner presses accept on the page
    // they were shown before it did.
    answer(
        &store,
        "save_model",
        json!({"id": accepted_id, "text": "<scxml><!-- OPEN, edited --></scxml>",
               "base": accepted_model["revision"], "written_for": accepted_head}),
    );
    let lapsed = answer(&store, "read_acceptance", json!({"id": accepted_id}));
    let lapsed_now = lapsed["now"].clone();
    answers.insert("read_acceptance_lapsed".into(), lapsed);
    // Asked of the design as it is now, the acceptance has lapsed; asked of the design it was
    // read as, it still holds, because the verdict is of the revisions named and not of the
    // work as it stands.
    answers.insert(
        "read_judgment_lapsed".into(),
        answer(
            &store,
            "read_judgment",
            json!({"id": accepted_id, "basis": lapsed_now, "acceptance": acceptance_revision}),
        ),
    );
    answers.insert(
        "read_judgment_of_an_earlier_state".into(),
        answer(
            &store,
            "read_judgment",
            json!({"id": accepted_id, "basis": shown, "acceptance": acceptance_revision}),
        ),
    );
    refusals.insert(
        "moved".into(),
        refusal(
            &store,
            "accept",
            json!({"id": accepted_id, "expect": shown}),
        ),
    );
    // The text moves on and the design and the list were written for the old one.
    answer(
        &store,
        "save_source",
        json!({"id": accepted_id, "text": "The door opens for a listed card, twice.",
               "base": accepted_head}),
    );
    let behind = answer(&store, "requirements_report", json!({"id": accepted_id}));
    let behind_shown = behind["basis"].clone();
    answers.insert("requirements_report_behind".into(), behind);
    refusals.insert(
        "not-current".into(),
        refusal(
            &store,
            "accept",
            json!({"id": accepted_id, "expect": behind_shown}),
        ),
    );
    refusals.insert(
        "sce-timeout".into(),
        refusal_by(
            &store,
            &RefusingRenderer,
            "requirements_report",
            json!({"id": accepted_id}),
        ),
    );

    // Requests, on the first work: its text and the owner's answers are what a request is
    // asked about. The clock does not move here, so no lease runs out.
    let about = json!({"source": source_head, "answers": answers_revision});
    let asked = json!({"id": id, "key": "press-1", "origin": "gui", "expect": about});
    let made = answer(&store, "request_generation", asked.clone());
    let request_id = made["request"]["id"].as_str().unwrap().to_string();
    answers.insert("request_generation".into(), made);
    answers.insert(
        "request_generation_again".into(),
        answer(&store, "request_generation", asked),
    );
    answers.insert(
        "read_request".into(),
        answer(
            &store,
            "read_request",
            json!({"id": id, "request": request_id}),
        ),
    );
    // What an executor looks at to find something to do: every work's open requests.
    answers.insert(
        "list_open_requests".into(),
        answer(&store, "list_open_requests", json!({})),
    );
    refusals.insert(
        "active-request".into(),
        refusal(
            &store,
            "request_generation",
            json!({"id": id, "key": "press-2", "origin": "gui", "expect": about}),
        ),
    );
    refusals.insert(
        "key-reused".into(),
        refusal(
            &store,
            "request_generation",
            json!({"id": id, "key": "press-1", "origin": "gui",
                   "expect": {"source": source_head}}),
        ),
    );
    answers.insert(
        "claim_request".into(),
        answer(
            &store,
            "claim_request",
            json!({"id": id, "request": request_id, "holder": "adapter-a"}),
        ),
    );
    refusals.insert(
        "request-held".into(),
        refusal(
            &store,
            "claim_request",
            json!({"id": id, "request": request_id, "holder": "adapter-b", "resume": true}),
        ),
    );
    refusals.insert(
        "not-holder".into(),
        refusal(
            &store,
            "heartbeat_request",
            json!({"id": id, "request": request_id, "holder": "adapter-b", "attempt": 1}),
        ),
    );
    refusals.insert(
        "bad-lease".into(),
        refusal(
            &store,
            "claim_request",
            json!({"id": id, "request": request_id, "holder": "adapter-a", "ttl_seconds": 5}),
        ),
    );
    answers.insert(
        "heartbeat_request".into(),
        answer(
            &store,
            "heartbeat_request",
            json!({"id": id, "request": request_id, "holder": "adapter-a", "attempt": 1,
                   "ttl_seconds": 120}),
        ),
    );
    answers.insert(
        "read_work_heads_requested".into(),
        answer(&store, "read_work_heads", json!({"id": id})),
    );
    // What the executor writes is the request's and not yet the work's; completing
    // publishes it as one bundle.
    let list = "{\"doc_id\":\"door\",\"rev\":\"2\",\"requirements\":[{\"id\":\"R1\"}]}\n";
    refusals.insert(
        "no-candidate".into(),
        refusal(
            &store,
            "complete_request",
            json!({"id": id, "request": request_id, "holder": "adapter-a", "attempt": 1}),
        ),
    );
    answers.insert(
        "save_request_candidate".into(),
        answer(
            &store,
            "save_request_candidate",
            json!({"id": id, "request": request_id, "holder": "adapter-a", "attempt": 1,
                   "text": "<scxml><!-- candidate --></scxml>", "manifest": list,
                   "instructions": "claude-code/0123456789ab"}),
        ),
    );
    answers.insert(
        "read_request_candidate".into(),
        answer(
            &store,
            "read_request_candidate",
            json!({"id": id, "request": request_id}),
        ),
    );
    let completed = answer(
        &store,
        "complete_request",
        json!({"id": id, "request": request_id, "holder": "adapter-a", "attempt": 1,
               "checks": [{"name": "decisions", "verdict": "accepted"}]}),
    );
    // A bundle is named by what it holds, and what it holds says the request that made
    // it, which is not the same in two runs.
    let bundle_revision = completed["bundle"].as_str().unwrap().to_string();
    answers.insert("complete_request".into(), completed);
    answers.insert(
        "read_bundle".into(),
        answer(&store, "read_bundle", json!({"id": id})),
    );
    answers.insert(
        "bundle_history".into(),
        answer(&store, "bundle_history", json!({"id": id})),
    );
    answers.insert(
        "read_work_heads_bundled".into(),
        answer(&store, "read_work_heads", json!({"id": id})),
    );
    refusals.insert(
        "bundled-work".into(),
        refusal(
            &store,
            "save_model",
            json!({"id": id, "text": "<scxml><!-- by hand --></scxml>"}),
        ),
    );
    refusals.insert(
        "request-ended".into(),
        refusal(
            &store,
            "heartbeat_request",
            json!({"id": id, "request": request_id, "holder": "adapter-a", "attempt": 1}),
        ),
    );
    // A request that could not be done, and one that was called off.
    let failing = answer(
        &store,
        "request_generation",
        json!({"id": id, "key": "press-3", "origin": "gui", "expect": about}),
    );
    let failed_id = failing["request"]["id"].as_str().unwrap().to_string();
    answer(
        &store,
        "claim_request",
        json!({"id": id, "request": failed_id, "holder": "adapter-a"}),
    );
    // A model the core refuses is not published, and the request is still the executor's.
    answer(
        &store,
        "save_request_candidate",
        json!({"id": id, "request": failed_id, "holder": "adapter-a", "attempt": 1,
               "text": "<scxml><!-- REFUSE --></scxml>", "manifest": list}),
    );
    refusals.insert(
        "check-refused".into(),
        refusal(
            &store,
            "complete_request",
            json!({"id": id, "request": failed_id, "holder": "adapter-a", "attempt": 1}),
        ),
    );
    answers.insert(
        "fail_request".into(),
        answer(
            &store,
            "fail_request",
            json!({"id": id, "request": failed_id, "holder": "adapter-a", "attempt": 1,
                   "reason": "SCE refused the model"}),
        ),
    );
    let calling_off = answer(
        &store,
        "request_generation",
        json!({"id": id, "key": "press-4", "origin": "gui", "expect": about}),
    );
    let cancelled_id = calling_off["request"]["id"].as_str().unwrap().to_string();
    answers.insert(
        "cancel_request".into(),
        answer(
            &store,
            "cancel_request",
            json!({"id": id, "request": cancelled_id}),
        ),
    );
    answers.insert(
        "list_requests".into(),
        answer(&store, "list_requests", json!({"id": id})),
    );

    // Adapters: none, then one that reported.
    answers.insert(
        "read_adapter_status_none".into(),
        answer(&store, "read_adapter_status", json!({})),
    );
    answers.insert(
        "report_adapter".into(),
        answer(
            &store,
            "report_adapter",
            json!({"name": "desktop", "kind": "claude-code",
                   "capabilities": ["generate", "cancel"], "version": "2.1"}),
        ),
    );
    answers.insert(
        "read_adapter_status".into(),
        answer(&store, "read_adapter_status", json!({})),
    );
    // Whether a shell hosts an executor, and why not: nothing yet, then a shell that could not.
    answers.insert(
        "read_host_status_none".into(),
        answer(&store, "read_host_status", json!({})),
    );
    store
        .report_host(sce_app_core::HostReport {
            name: "desktop",
            hosting: false,
            reason: Some("no Claude Code to write models with: install it, or set SCE_CLAUDE"),
            client_version: None,
            waiting: &[],
        })
        .unwrap();
    answers.insert(
        "read_host_status".into(),
        answer(&store, "read_host_status", json!({})),
    );
    // A shell that hosts, and the request it left queued because it could not run it.
    store
        .report_host(sce_app_core::HostReport {
            name: "web-shell",
            hosting: true,
            reason: None,
            client_version: Some("2.1.291"),
            waiting: &[sce_app_core::HostWaiting {
                work: id.clone(),
                request: request_id.clone(),
                connection: "claude".to_string(),
                reason: "nobody is signed in to Claude Code: run `claude auth login`".to_string(),
            }],
        })
        .unwrap();
    answers.insert(
        "read_host_status_waiting".into(),
        answer(&store, "read_host_status", json!({})),
    );
    refusals.insert(
        "bad-adapter".into(),
        refusal(
            &store,
            "report_adapter",
            json!({"name": "../x", "kind": "claude-code", "capabilities": []}),
        ),
    );

    // The settings a person keeps apart from the works, asked of the entrance that may change
    // them; the others are shown only by what they are refused.
    let settings = ConnectionStore::at(common::scratch("contract-settings"));
    let policy = Policy::shipped();
    let entrance = |entrance: Entrance, with_settings: bool| {
        Context::new(&store, &FakeRenderer, &policy, entrance)
            .with_connections(with_settings.then_some(&settings))
    };
    let desktop = entrance(Entrance::Desktop, true);
    let ask = |name: &str, args: Value| {
        call_in(&desktop, name, args).unwrap_or_else(|e| panic!("{name} was refused: {e:?}"))
    };
    let refuse = |context: &Context<'_, FixedClock>, name: &str, args: Value| {
        let error: CommandError =
            call_in(context, name, args).expect_err("the command should be refused");
        serde_json::to_value(error).unwrap()
    };
    let connection = |model: &str| json!({ "id": "main", "adapter": "claude-code", "model": model, "auth": "official-login" });
    answers.insert("describe_desktop".into(), ask("describe", json!({})));
    answers.insert(
        "list_connections_empty".into(),
        ask("list_connections", json!({})),
    );
    let first = ask(
        "save_connection",
        json!({ "connection": connection("opus") }),
    );
    let first_revision = first["revision"].clone();
    answers.insert("save_connection_first".into(), first);
    answers.insert(
        "save_connection_unchanged".into(),
        ask(
            "save_connection",
            json!({ "connection": connection("opus"), "base": first_revision }),
        ),
    );
    let next = ask(
        "save_connection",
        json!({ "connection": connection("sonnet"), "base": first_revision }),
    );
    let next_revision = next["revision"].clone();
    answers.insert("save_connection_next".into(), next);
    ask(
        "save_connection",
        json!({ "connection": {
            "id": "pc2", "adapter": "local", "display_name": "pc2 (tunnel)",
            "model": "qwen3-coder:30b", "auth": "none", "server_url": "http://127.0.0.1:11434/v1"
        } }),
    );
    answers.insert(
        "set_default_connection".into(),
        ask(
            "set_default_connection",
            json!({ "id": "main", "expect": null }),
        ),
    );
    answers.insert(
        "list_connections".into(),
        ask("list_connections", json!({})),
    );
    answers.insert(
        "read_connection".into(),
        ask("read_connection", json!({ "id": "main" })),
    );
    answers.insert(
        "read_connection_revision".into(),
        ask(
            "read_connection",
            json!({ "id": "main", "revision": first_revision }),
        ),
    );
    answers.insert(
        "read_connection_none".into(),
        ask("read_connection", json!({ "id": "nobody" })),
    );
    answers.insert(
        "read_auth_policy".into(),
        ask("read_auth_policy", json!({})),
    );
    // What a screen is told of Claude Code. A program that is not there is asked for real; the
    // shapes a client that is there gives are the core's own types, so that the file does not
    // depend on a script that only a Unix shell can run.
    let missing = Context::new(&store, &FakeRenderer, &policy, Entrance::Desktop)
        .with_connections(Some(&settings))
        .with_claude(Some(std::path::Path::new("/nowhere/claude")));
    answers.insert(
        "read_claude_status_missing".into(),
        call_in(&missing, "read_claude_status", json!({})).expect("an answer"),
    );
    let installed = |account: AccountState| {
        json!({ "claude": ClaudeStatus {
            client: Client::Installed { version: "2.1.291".to_string() },
            account,
            sign_in: SIGN_IN.to_vec(),
        } })
    };
    let signed_in = |route: Route, environment: Option<&str>| {
        let decision = policy.decide(route);
        installed(AccountState::SignedIn {
            route,
            billing: Billing::of(route),
            environment: environment.map(str::to_string),
            usable: matches!(decision, sce_app_core::Decision::Use { .. }),
            decision,
        })
    };
    answers.insert(
        "read_claude_status_subscription".into(),
        signed_in(Route::ClaudeOfficialLogin, None),
    );
    answers.insert(
        "read_claude_status_key_from_environment".into(),
        signed_in(Route::ClaudeApiKey, Some("ANTHROPIC_API_KEY")),
    );
    answers.insert(
        "read_claude_status_not_used".into(),
        signed_in(Route::Unlisted, None),
    );
    answers.insert(
        "read_claude_status_signed_out".into(),
        installed(AccountState::SignedOut),
    );
    answers.insert(
        "read_claude_status_unknown".into(),
        installed(AccountState::Unknown {
            reason: "`claude auth status` did not answer JSON".to_string(),
        }),
    );
    refusals.insert(
        "not-allowed-to-start-a-program".into(),
        refuse(
            &entrance(Entrance::Browser, true),
            "read_claude_status",
            json!({}),
        ),
    );
    refusals.insert(
        "connection-conflict".into(),
        refuse(
            &desktop,
            "delete_connection",
            json!({ "id": "main", "base": first_revision }),
        ),
    );
    refusals.insert(
        "connection-moved".into(),
        refuse(
            &desktop,
            "set_default_connection",
            json!({ "id": "pc2", "expect": null }),
        ),
    );
    refusals.insert(
        "bad-connection".into(),
        refuse(
            &desktop,
            "save_connection",
            json!({ "connection": {
                "id": "main", "adapter": "claude-code", "auth": "official-login",
                "executable": "/tmp/anything"
            } }),
        ),
    );
    refusals.insert(
        "not-allowed-here".into(),
        refuse(
            &entrance(Entrance::Browser, true),
            "save_connection",
            json!({ "connection": connection("opus") }),
        ),
    );
    refusals.insert(
        "no-settings".into(),
        refusal(&store, "list_connections", json!({})),
    );
    // A request made for a connection, and what is refused to a caller it is not for.
    let pinned_work = ask("create_work", json!({ "title": "Pinned" }));
    let pinned_id = pinned_work["id"].as_str().expect("a work id").to_string();
    let pinned_source = ask(
        "save_source",
        json!({ "id": pinned_id, "text": "The lock opens when the code matches." }),
    );
    let pinned = ask(
        "request_generation",
        json!({
            "id": pinned_id, "key": "press-pinned", "origin": "gui",
            "expect": { "source": pinned_source["revision"], "answers": null },
            "connection": { "id": "main", "revision": next_revision },
        }),
    );
    let pinned_request = pinned["request"]["id"]
        .as_str()
        .expect("a request id")
        .to_string();
    answers.insert("request_generation_pinned".into(), pinned);
    refusals.insert(
        "wrong-connection".into(),
        refuse(
            &desktop,
            "claim_request",
            json!({ "id": pinned_id, "request": pinned_request, "holder": "desktop" }),
        ),
    );
    refusals.insert(
        "request-connection-moved".into(),
        refuse(
            &desktop,
            "request_generation",
            json!({
                "id": pinned_id, "key": "press-stale", "origin": "gui", "supersede": true,
                "expect": { "source": pinned_source["revision"], "answers": null },
                "connection": { "id": "main", "revision": first_revision },
            }),
        ),
    );
    answers.insert(
        "delete_connection".into(),
        ask(
            "delete_connection",
            json!({ "id": "main", "base": next_revision }),
        ),
    );

    // A command without a written-down reply is a command the screen's test
    // cannot hold to account.
    for command in COMMANDS {
        assert!(
            answers
                .keys()
                .any(|name| name == command || name.starts_with(&format!("{command}_"))),
            "`{command}` has no reply in the contract file; add one to `replies()`"
        );
    }

    let mut document = json!({
        "about": "Every reply of the command layer, produced by tests/contract.rs and read by the screen's tests.",
        "command_set_version": COMMAND_SET_VERSION,
        "answers": answers,
        "refusals": refusals,
    });
    name_the_unstable(&mut document, &root.display().to_string(), "<root>");
    name_the_unstable(&mut document, &id, "<work-id>");
    name_the_unstable(&mut document, &other_id, "<removed-work-id>");
    name_the_unstable(&mut document, &accepted_id, "<accepted-work-id>");
    name_the_unstable(&mut document, &request_id, "<request-id>");
    name_the_unstable(&mut document, &failed_id, "<failed-request-id>");
    name_the_unstable(&mut document, &cancelled_id, "<cancelled-request-id>");
    name_the_unstable(&mut document, &pinned_request, "<pinned-request-id>");
    name_the_unstable(&mut document, &pinned_id, "<pinned-work-id>");
    // A revision stays the shape of one, so that the screen's guard reads the file as it
    // would read the core.
    name_the_unstable(&mut document, &bundle_revision, &"b".repeat(64));
    let _ = std::fs::remove_dir_all(&root);
    document
}

#[test]
fn the_replies_are_the_ones_written_down() {
    let mut now = serde_json::to_string_pretty(&replies()).unwrap();
    now.push('\n');

    if std::env::var_os("SCE_UPDATE_CONTRACT").is_some() {
        std::fs::create_dir_all(std::path::Path::new(FILE).parent().unwrap()).unwrap();
        std::fs::write(FILE, &now).unwrap();
        return;
    }
    let written = std::fs::read_to_string(FILE)
        .unwrap_or_else(|e| panic!("{FILE} cannot be read ({e}); run with SCE_UPDATE_CONTRACT=1"));
    assert!(
        written == now,
        "a command's reply changed. If that is intended, run \
         `SCE_UPDATE_CONTRACT=1 cargo test -p sce-app-core --test contract` and review the diff of \
         {FILE}; the screen's test reads it.\n--- written\n{written}\n--- now\n{now}"
    );
}
