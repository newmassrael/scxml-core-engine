// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A request names the connection it was made for, and only that connection's executor takes it.
//!
//! When a person presses the button the request is made against one connection, in the state it
//! was in then: a later change of the settings moves no request that was already made. The core
//! copies from the connection what a run needs to be the same run (which client, which model,
//! what it may spend) and nothing that says where or as whom: the works folder is shared, and a
//! server's address, a name for it and the path of a program are not for whoever it is shared
//! with. An executor takes a request by offering the connection it runs for, and a request that
//! is for another is not given to it, so that a person who chose one AI is never answered by
//! another.

mod common;

use sce_app_core::requests::Request;
use sce_app_core::{
    call, call_in, CommandError, ConnectionStore, Context, Entrance, Policy, SystemClock, WorkStore,
};
use serde_json::{json, Value};

use common::FakeRenderer;

struct Rig {
    works: WorkStore,
    connections: ConnectionStore,
    policy: Policy,
}

impl Rig {
    fn new(label: &str) -> Rig {
        Rig {
            works: WorkStore::at(common::scratch(&format!("{label}-works"))),
            connections: ConnectionStore::at(common::scratch(&format!("{label}-settings"))),
            policy: Policy::shipped(),
        }
    }

    fn context(&self) -> Context<'_, SystemClock> {
        Context {
            works: &self.works,
            product: &FakeRenderer,
            connections: Some(&self.connections),
            policy: &self.policy,
            entrance: Entrance::Desktop,
        }
    }

    fn ask(&self, name: &str, args: Value) -> Result<Value, CommandError> {
        call_in(&self.context(), name, args)
    }

    fn ok(&self, name: &str, args: Value) -> Value {
        self.ask(name, args)
            .unwrap_or_else(|e| panic!("{name} was refused: {e:?}"))
    }

    /// A work with a text, and the revision of that text.
    fn work(&self) -> (String, String) {
        let work = self.ok("create_work", json!({ "title": "Door lock" }));
        let id = work["id"].as_str().unwrap().to_string();
        let saved = self.ok(
            "save_source",
            json!({ "id": id, "text": "The lock opens when the code matches." }),
        );
        (id, saved["revision"].as_str().unwrap().to_string())
    }

    fn connection(&self, connection: Value) -> String {
        let saved = self.ok("save_connection", json!({ "connection": connection }));
        saved["revision"].as_str().unwrap().to_string()
    }

    fn request(
        &self,
        work: &(String, String),
        key: &str,
        connection: Option<Value>,
    ) -> Result<Value, CommandError> {
        let mut args = json!({
            "id": work.0, "key": key, "origin": "gui",
            "expect": { "source": work.1, "answers": null },
        });
        if let Some(connection) = connection {
            args["connection"] = connection;
        }
        self.ask("request_generation", args)
    }
}

fn claude() -> Value {
    json!({
        "id": "main", "adapter": "claude-code", "model": "opus", "auth": "official-login",
        "limits": { "turns": 40, "seconds": 600 }
    })
}

fn pc2() -> Value {
    json!({
        "id": "pc2", "adapter": "local", "display_name": "pc2 (tunnel)", "model": "qwen3-coder:30b",
        "auth": "none", "server_url": "http://127.0.0.1:11434/v1"
    })
}

fn reference(id: &str, revision: &str) -> Value {
    json!({ "id": id, "revision": revision })
}

#[test]
fn a_request_made_for_a_connection_records_what_a_run_needs_of_it() {
    let rig = Rig::new("pin-record");
    let revision = rig.connection(claude());
    let work = rig.work();

    let made = rig
        .request(&work, "press-1", Some(reference("main", &revision)))
        .unwrap();

    assert_eq!(
        made["request"]["pin"],
        json!({
            "connection": "main", "revision": revision, "adapter": "claude-code",
            "model": "opus", "limits": { "turns": 40, "seconds": 600 }
        })
    );
}

#[test]
fn nothing_that_says_where_or_as_whom_is_copied_into_the_works_folder() {
    let rig = Rig::new("pin-nothing-else");
    let revision = rig.connection(pc2());
    let work = rig.work();

    let made = rig
        .request(&work, "press-1", Some(reference("pc2", &revision)))
        .unwrap();

    assert_eq!(made["request"]["pin"]["adapter"], json!("local"));
    // The answer a screen reads and every file the works folder holds: neither names the
    // server, what the person calls it, or a program.
    let mut seen = vec![made.to_string()];
    for entry in walk(rig.works.root()) {
        seen.push(std::fs::read_to_string(&entry).unwrap_or_default());
    }
    for text in seen {
        for secret in [
            "127.0.0.1",
            "pc2 (tunnel)",
            "11434",
            "server_url",
            "executable",
        ] {
            assert!(
                !text.contains(secret),
                "`{secret}` reached the works folder"
            );
        }
    }
}

fn walk(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let mut folders = vec![root.to_path_buf()];
    while let Some(folder) = folders.pop() {
        for entry in std::fs::read_dir(&folder).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                folders.push(path);
            } else {
                files.push(path);
            }
        }
    }
    files
}

#[test]
fn a_request_for_a_revision_that_is_no_longer_current_is_refused_as_moved() {
    let rig = Rig::new("pin-moved");
    let first = rig.connection(claude());
    let mut changed = claude();
    changed["model"] = json!("sonnet");
    let saved = rig.ok(
        "save_connection",
        json!({ "connection": changed, "base": first }),
    );
    let work = rig.work();

    let refused = rig
        .request(&work, "press-1", Some(reference("main", &first)))
        .unwrap_err();

    assert_eq!(refused.kind, "moved");
    assert_eq!(
        refused.detail["current"],
        json!({ "connection": { "id": "main", "revision": saved["revision"] } })
    );
    // Nothing was registered.
    assert_eq!(
        rig.ok("list_requests", json!({ "id": work.0 }))["requests"],
        json!([])
    );
}

#[test]
fn a_request_for_a_connection_that_is_not_there_is_not_found() {
    let rig = Rig::new("pin-absent");
    let work = rig.work();

    let refused = rig
        .request(&work, "press-1", Some(reference("nobody", &"a".repeat(64))))
        .unwrap_err();

    assert_eq!(refused.kind, "not-found");
}

#[test]
fn an_entrance_with_no_settings_cannot_pin_a_connection() {
    let rig = Rig::new("pin-no-settings");
    let work = rig.work();

    let refused = call(
        &rig.works,
        &FakeRenderer,
        "request_generation",
        json!({
            "id": work.0, "key": "press-1", "origin": "mcp",
            "expect": { "source": work.1, "answers": null },
            "connection": reference("main", &"a".repeat(64)),
        }),
    )
    .unwrap_err();

    assert_eq!(refused.kind, "no-settings");
}

#[test]
fn the_same_press_sent_again_is_the_request_it_made_whatever_connection_it_names() {
    let rig = Rig::new("pin-idempotent");
    let main = rig.connection(claude());
    let other = rig.connection(pc2());
    let work = rig.work();
    let first = rig
        .request(&work, "press-1", Some(reference("main", &main)))
        .unwrap();

    let again = rig
        .request(&work, "press-1", Some(reference("pc2", &other)))
        .unwrap();

    assert_eq!(again["created"], json!(false));
    assert_eq!(again["request"]["id"], first["request"]["id"]);
    // What comes back is the request as it was made: it is the screen that shows it.
    assert_eq!(again["request"]["pin"]["connection"], json!("main"));
}

#[test]
fn a_press_sent_again_after_the_connection_changed_is_still_the_request_it_made() {
    let rig = Rig::new("pin-idempotent-after-change");
    let first_revision = rig.connection(claude());
    let work = rig.work();
    let first = rig
        .request(&work, "press-1", Some(reference("main", &first_revision)))
        .unwrap();
    let mut changed = claude();
    changed["model"] = json!("sonnet");
    rig.ok(
        "save_connection",
        json!({ "connection": changed, "base": first_revision }),
    );

    // The connection is not an input of the key: the same press is the same request, and is
    // not refused for a connection that moved after it was made.
    let again = rig
        .request(&work, "press-1", Some(reference("main", &first_revision)))
        .unwrap();

    assert_eq!(again["created"], json!(false));
    assert_eq!(again["request"]["id"], first["request"]["id"]);
    assert_eq!(again["request"]["pin"]["model"], json!("opus"));
}

#[test]
fn a_request_made_without_a_connection_has_none() {
    let rig = Rig::new("pin-none");
    let work = rig.work();

    let made = rig.request(&work, "press-1", None).unwrap();

    assert_eq!(made["request"]["pin"], json!(null));
}

#[test]
fn only_the_executor_of_the_pinned_connection_takes_a_request() {
    let rig = Rig::new("pin-claim");
    let main = rig.connection(claude());
    let other = rig.connection(pc2());
    let work = rig.work();
    let made = rig
        .request(&work, "press-1", Some(reference("main", &main)))
        .unwrap();
    let request = made["request"]["id"].as_str().unwrap().to_string();
    let claim = |connection: Option<Value>| {
        let mut args = json!({ "id": work.0, "request": request, "holder": "desktop" });
        if let Some(connection) = connection {
            args["connection"] = connection;
        }
        rig.ask("claim_request", args)
    };

    for (label, offered) in [
        ("none offered", None),
        ("another connection", Some(reference("pc2", &other))),
        (
            "an older revision of it",
            Some(reference("main", &"b".repeat(64))),
        ),
    ] {
        let refused = claim(offered).unwrap_err();
        assert_eq!(refused.kind, "wrong-connection", "{label}");
        assert_eq!(
            refused.detail["pinned"],
            json!({ "id": "main", "revision": main }),
            "{label}"
        );
    }
    // Refused, it is still nobody's.
    assert_eq!(
        rig.ok("read_request", json!({ "id": work.0, "request": request }))["request"]["state"],
        json!("queued")
    );

    let taken = claim(Some(reference("main", &main))).unwrap();
    assert_eq!(taken["request"]["state"], json!("running"));
}

#[test]
fn a_request_with_no_connection_is_taken_by_an_executor_that_offers_none() {
    let rig = Rig::new("pin-claim-none");
    let main = rig.connection(claude());
    let work = rig.work();
    let made = rig.request(&work, "press-1", None).unwrap();
    let request = made["request"]["id"].as_str().unwrap().to_string();

    let offered = rig
        .ask(
            "claim_request",
            json!({
                "id": work.0, "request": request, "holder": "desktop",
                "connection": reference("main", &main),
            }),
        )
        .unwrap_err();
    assert_eq!(offered.kind, "wrong-connection");
    assert_eq!(offered.detail["pinned"], json!(null));

    let taken = rig
        .ask(
            "claim_request",
            json!({ "id": work.0, "request": request, "holder": "mcp-1" }),
        )
        .unwrap();
    assert_eq!(taken["request"]["state"], json!("running"));
}

#[test]
fn taking_a_request_again_is_held_to_the_same_connection() {
    let rig = Rig::new("pin-resume");
    let main = rig.connection(claude());
    let other = rig.connection(pc2());
    let work = rig.work();
    let made = rig
        .request(&work, "press-1", Some(reference("main", &main)))
        .unwrap();
    let request = made["request"]["id"].as_str().unwrap().to_string();
    rig.ask(
        "claim_request",
        json!({
            "id": work.0, "request": request, "holder": "desktop", "ttl_seconds": 10,
            "connection": reference("main", &main),
        }),
    )
    .unwrap();
    let wrong = rig
        .ask(
            "claim_request",
            json!({
                "id": work.0, "request": request, "holder": "other", "resume": true,
                "connection": reference("pc2", &other),
            }),
        )
        .unwrap_err();

    // The request is held by the executor of its own connection, and an executor of another that
    // says it resumes is told it is not its to take, not that the request is held: the first is
    // what it can act on.
    assert_eq!(wrong.kind, "wrong-connection");
    let held = rig
        .ask(
            "claim_request",
            json!({
                "id": work.0, "request": request, "holder": "other", "resume": true,
                "connection": reference("main", &main),
            }),
        )
        .unwrap_err();
    assert_eq!(held.kind, "request-held");
}

#[test]
fn a_request_written_before_connections_existed_still_reads() {
    let legacy = r#"{
        "id": "req-1", "seq": 1, "key": "k", "origin": "gui",
        "inputs": { "source": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "answers": null },
        "created_at": "2026-10-05T09:00:00Z", "state": "queued", "attempt": 0
    }"#;

    let request: Request = serde_json::from_str(legacy).unwrap();

    assert!(request.pin.is_none());
    // And it is written back as it was: no pin, no new field in a file nobody changed.
    let written = serde_json::to_value(&request).unwrap();
    assert!(written.get("pin").is_none(), "{written}");
}
