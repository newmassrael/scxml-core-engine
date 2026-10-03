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

use sce_app_core::{
    call, CommandError, FigureRenderer, FixedClock, NoRenderer, WorkStore, COMMANDS,
    COMMAND_SET_VERSION,
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

fn refusal(store: &WorkStore<FixedClock>, name: &str, args: Value) -> Value {
    refusal_by(store, &FakeRenderer, name, args)
}

fn refusal_by(
    store: &WorkStore<FixedClock>,
    renderer: &dyn FigureRenderer,
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
