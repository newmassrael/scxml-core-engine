// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The request and adapter commands, as JSON in and JSON out.
//!
//! The store's own tests hold what a request does. What is held here is what a caller that
//! is not Rust sees: the words of each answer (a request says its state as the clock reads
//! it AND as it was last written, because a lease that ran out is an interrupted request that
//! nobody has written down), the facts each refusal carries, and that an argument that nothing
//! reads is refused.

mod common;

use std::sync::Arc;

use sce_app_core::{call, CommandError, ManualClock, WorkStore};
use serde_json::{json, Value};

use common::FakeRenderer;

const T0: u64 = 1_791_190_800;

struct Fixture {
    clock: Arc<ManualClock>,
    store: WorkStore<Arc<ManualClock>>,
    work: String,
    source: String,
}

fn fixture(label: &str) -> Fixture {
    let clock = Arc::new(ManualClock::at(T0));
    let store = WorkStore::with_clock(common::scratch(label), Arc::clone(&clock));
    let work = call(
        &store,
        &FakeRenderer,
        "create_work",
        json!({"title": "Door lock"}),
    )
    .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let source = call(
        &store,
        &FakeRenderer,
        "save_source",
        json!({"id": work, "text": "The lock opens."}),
    )
    .unwrap()["revision"]
        .as_str()
        .unwrap()
        .to_string();
    Fixture {
        clock,
        store,
        work,
        source,
    }
}

impl Fixture {
    fn run(&self, name: &str, args: Value) -> Value {
        call(&self.store, &FakeRenderer, name, args)
            .unwrap_or_else(|e| panic!("{name} was refused: {e:?}"))
    }

    fn refuse(&self, name: &str, args: Value) -> CommandError {
        call(&self.store, &FakeRenderer, name, args).expect_err("the command should be refused")
    }

    fn generate(&self, key: &str) -> Value {
        self.run(
            "request_generation",
            json!({
                "id": self.work,
                "key": key,
                "origin": "gui",
                "expect": {"source": self.source},
            }),
        )
    }

    fn request_id(&self, key: &str) -> String {
        self.generate(key)["request"]["id"]
            .as_str()
            .unwrap()
            .to_string()
    }

    fn claim(&self, request: &str, holder: &str) -> Value {
        self.run(
            "claim_request",
            json!({"id": self.work, "request": request, "holder": holder}),
        )
    }
}

#[test]
fn a_request_is_said_in_the_words_a_caller_branches_on() {
    let f = fixture("request-commands-shape");

    let made = f.generate("press-1");

    assert_eq!(made["created"], json!(true));
    let request = &made["request"];
    assert_eq!(request["work"], json!(f.work));
    assert_eq!(request["state"], json!("queued"));
    assert_eq!(request["stored_state"], json!("queued"));
    assert_eq!(request["attempt"], json!(0));
    assert_eq!(request["seq"], json!(1));
    assert_eq!(request["key"], json!("press-1"));
    assert_eq!(request["origin"], json!("gui"));
    assert_eq!(
        request["inputs"],
        json!({"source": f.source, "answers": null})
    );
    assert_eq!(request["created_at"], json!("2026-10-05T09:00:00Z"));
    assert_eq!(request["lease"], Value::Null);
    assert_eq!(request["ended_at"], Value::Null);
    assert_eq!(request["note"], Value::Null);
}

#[test]
fn a_request_is_taken_kept_and_finished_through_the_commands() {
    let f = fixture("request-commands-run");
    let id = f.request_id("press-1");

    let taken = f.claim(&id, "adapter-a");
    assert_eq!(taken["request"]["state"], json!("running"));
    assert_eq!(taken["request"]["attempt"], json!(1));
    assert_eq!(
        taken["request"]["lease"],
        json!({
            "holder": "adapter-a",
            "attempt": 1,
            "granted_at": "2026-10-05T09:00:00Z",
            "expires_at": "2026-10-05T09:01:00Z",
        })
    );

    f.clock.advance(30);
    let kept = f.run(
        "heartbeat_request",
        json!({"id": f.work, "request": id, "holder": "adapter-a", "attempt": 1, "ttl_seconds": 120}),
    );
    assert_eq!(
        kept["request"]["lease"]["expires_at"],
        json!("2026-10-05T09:02:30Z")
    );

    let done = f.run(
        "complete_request",
        json!({"id": f.work, "request": id, "holder": "adapter-a", "attempt": 1}),
    );
    assert_eq!(done["request"]["state"], json!("completed"));
    assert_eq!(done["request"]["ended_at"], json!("2026-10-05T09:00:30Z"));
    let read = f.run("read_request", json!({"id": f.work, "request": id}));
    assert_eq!(read["request"], done["request"]);
}

#[test]
fn a_lease_that_ran_out_is_interrupted_by_the_clock_and_running_as_written() {
    let f = fixture("request-commands-expiry");
    let id = f.request_id("press-1");
    f.run(
        "claim_request",
        json!({"id": f.work, "request": id, "holder": "adapter-a", "ttl_seconds": 30}),
    );

    f.clock.advance(31);
    let read = f.run("read_request", json!({"id": f.work, "request": id}));

    assert_eq!(read["request"]["state"], json!("interrupted"));
    assert_eq!(read["request"]["stored_state"], json!("running"));
    let resumed = f.run(
        "claim_request",
        json!({"id": f.work, "request": id, "holder": "adapter-b", "resume": true}),
    );
    assert_eq!(resumed["request"]["attempt"], json!(2));
    assert_eq!(resumed["request"]["state"], json!("running"));
}

#[test]
fn requests_are_listed_newest_first() {
    let f = fixture("request-commands-list");
    let first = f.request_id("press-1");
    f.run("cancel_request", json!({"id": f.work, "request": first}));
    let second = f.request_id("press-2");

    let listed = f.run("list_requests", json!({"id": f.work}));

    let ids: Vec<&str> = listed["requests"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec![second.as_str(), first.as_str()]);
}

#[test]
fn a_failure_is_said_with_its_reason() {
    let f = fixture("request-commands-fail");
    let id = f.request_id("press-1");
    f.claim(&id, "adapter-a");

    let failed = f.run(
        "fail_request",
        json!({"id": f.work, "request": id, "holder": "adapter-a", "attempt": 1, "reason": "SCE refused the model"}),
    );

    assert_eq!(failed["request"]["state"], json!("failed"));
    assert_eq!(failed["request"]["note"], json!("SCE refused the model"));
}

#[test]
fn a_registration_sent_again_is_the_request_it_repeats() {
    let f = fixture("request-commands-again");
    let first = f.generate("press-1");

    let again = f.generate("press-1");

    assert_eq!(first["created"], json!(true));
    assert_eq!(again["created"], json!(false));
    assert_eq!(again["request"]["id"], first["request"]["id"]);
}

#[test]
fn each_refusal_carries_what_a_caller_acts_on() {
    let f = fixture("request-commands-refusals");
    let id = f.request_id("press-1");

    let open = f.refuse(
        "request_generation",
        json!({"id": f.work, "key": "press-2", "origin": "gui", "expect": {"source": f.source}}),
    );
    assert_eq!(open.kind, "active-request");
    assert_eq!(open.detail, json!({"request": id, "state": "queued"}));

    f.claim(&id, "adapter-a");
    let held = f.refuse(
        "claim_request",
        json!({"id": f.work, "request": id, "holder": "adapter-b", "resume": true}),
    );
    assert_eq!(held.kind, "request-held");
    assert_eq!(held.detail["holder"], json!("adapter-a"));
    assert_eq!(held.detail["until"], json!("2026-10-05T09:01:00Z"));

    let stranger = f.refuse(
        "heartbeat_request",
        json!({"id": f.work, "request": id, "holder": "adapter-b", "attempt": 1}),
    );
    assert_eq!(stranger.kind, "not-holder");
    assert_eq!(stranger.detail["holder"], json!("adapter-a"));
    assert_eq!(stranger.detail["attempt"], json!(1));

    f.run("cancel_request", json!({"id": f.work, "request": id}));
    let ended = f.refuse(
        "heartbeat_request",
        json!({"id": f.work, "request": id, "holder": "adapter-a", "attempt": 1}),
    );
    assert_eq!(ended.kind, "request-ended");
    assert_eq!(ended.detail, json!({"request": id, "state": "cancelled"}));
}

#[test]
fn a_registration_about_a_text_that_moved_says_what_moved_and_where_the_work_is() {
    let f = fixture("request-commands-moved");
    let newer = f.run(
        "save_source",
        json!({"id": f.work, "text": "The lock opens twice.", "base": f.source}),
    )["revision"]
        .clone();

    let moved = f.refuse(
        "request_generation",
        json!({"id": f.work, "key": "press-1", "origin": "gui", "expect": {"source": f.source}}),
    );

    assert_eq!(moved.kind, "moved");
    assert_eq!(moved.detail["moved"], json!(["source"]));
    assert_eq!(
        moved.detail["current"],
        json!({"source": newer, "answers": null})
    );
}

#[test]
fn an_argument_that_nothing_reads_or_a_shape_that_is_not_one_is_refused() {
    let f = fixture("request-commands-bad");
    let id = f.request_id("press-1");

    for (name, args) in [
        (
            "read_request",
            json!({"id": f.work, "request": id, "extra": 1}),
        ),
        ("claim_request", json!({"id": f.work, "request": id})),
        (
            "heartbeat_request",
            json!({"id": f.work, "request": id, "holder": "a", "attempt": "one"}),
        ),
        (
            "request_generation",
            json!({"id": f.work, "key": "k", "origin": "gui", "expect": {"source": "not-a-digest"}}),
        ),
        ("list_requests", json!({})),
    ] {
        let error = f.refuse(name, args);
        assert_eq!(error.kind, "bad-request", "{name}");
    }
}

#[test]
fn a_request_that_is_not_there_is_not_found_in_the_same_word_as_everything_else() {
    let f = fixture("request-commands-missing");

    let error = f.refuse(
        "read_request",
        json!({"id": f.work, "request": "req-0123456789ab"}),
    );

    assert_eq!(error.kind, "not-found");
}

#[test]
fn an_adapter_reports_and_is_read_back_as_there_for_as_long_as_it_is_recent() {
    let f = fixture("request-commands-adapters");
    assert_eq!(
        f.run("read_adapter_status", json!({})),
        json!({"adapters": [], "unreadable": []})
    );

    let reported = f.run(
        "report_adapter",
        json!({"name": "desktop", "kind": "claude-code", "capabilities": ["generate", "cancel"], "version": "2.1"}),
    );

    assert_eq!(
        reported["adapter"],
        json!({
            "name": "desktop",
            "kind": "claude-code",
            "capabilities": ["generate", "cancel"],
            "version": "2.1",
            "seen_at": "2026-10-05T09:00:00Z",
            "live": true,
        })
    );
    f.clock.advance(90);
    let status = f.run("read_adapter_status", json!({}));
    assert_eq!(status["adapters"][0]["live"], json!(false));
    assert_eq!(status["adapters"][0]["name"], json!("desktop"));
    let error = f.refuse(
        "report_adapter",
        json!({"name": "../x", "kind": "claude-code", "capabilities": []}),
    );
    assert_eq!(error.kind, "bad-adapter");
}

#[test]
fn the_new_commands_are_listed_so_a_screen_can_tell_a_core_that_has_them() {
    let f = fixture("request-commands-describe");

    let described = f.run("describe", json!({}));

    let commands: Vec<&str> = described["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    for name in [
        "request_generation",
        "read_request",
        "list_requests",
        "claim_request",
        "heartbeat_request",
        "complete_request",
        "fail_request",
        "cancel_request",
        "report_adapter",
        "read_adapter_status",
    ] {
        assert!(commands.contains(&name), "{name}");
    }
}
