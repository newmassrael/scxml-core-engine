// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The connections as commands, and who may change them.
//!
//! A connection names a program the application runs, so a command that writes one is a way to
//! make the application run something. The desktop application is the person at the keyboard;
//! the browser shell is a development tool a token reaches over a network; the tool entrance
//! is what the authoring MCP uses, and an AI must not be able to change which AI it runs on.
//! So only the desktop writes settings. Every entrance can read them, the browser shell and
//! the desktop alike, and an entrance with no settings folder says so.

mod common;

use sce_app_core::{
    call, call_in, CommandError, Connection, ConnectionStore, Context, Entrance, Policy, Route,
    WorkStore, COMMANDS, COMMAND_SET_VERSION,
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

    fn context(&self, entrance: Entrance) -> Context<'_, sce_app_core::SystemClock> {
        Context {
            works: &self.works,
            product: &FakeRenderer,
            connections: Some(&self.connections),
            policy: &self.policy,
            entrance,
        }
    }

    fn ask(&self, entrance: Entrance, name: &str, args: Value) -> Result<Value, CommandError> {
        call_in(&self.context(entrance), name, args)
    }

    fn desktop(&self, name: &str, args: Value) -> Value {
        self.ask(Entrance::Desktop, name, args)
            .unwrap_or_else(|e| panic!("{name} was refused: {e:?}"))
    }
}

fn claude_json(id: &str) -> Value {
    json!({ "id": id, "adapter": "claude-code", "model": "opus", "auth": "official-login" })
}

fn local_json(id: &str) -> Value {
    json!({
        "id": id, "adapter": "local", "display_name": "pc2 (tunnel)", "model": "qwen3-coder:30b",
        "auth": "none", "server_url": "http://127.0.0.1:11434/v1"
    })
}

fn revision_of(saved: &Value) -> String {
    saved["revision"].as_str().expect("a revision").to_string()
}

#[test]
fn describe_says_which_entrance_it_is_and_what_it_may_do() {
    let rig = Rig::new("cc-describe");

    let desktop = rig.desktop("describe", json!({}));
    let browser = rig.ask(Entrance::Browser, "describe", json!({})).unwrap();
    let tool = call(&rig.works, &FakeRenderer, "describe", json!({})).unwrap();

    assert_eq!(desktop["command_set_version"], json!(COMMAND_SET_VERSION));
    assert_eq!(
        [&desktop, &browser, &tool].map(|d| d["entrance"].clone()),
        [json!("desktop"), json!("browser"), json!("tool")]
    );
    assert_eq!(
        [&desktop, &browser, &tool].map(|d| (d["settings"].clone(), d["writes_settings"].clone())),
        [
            (json!(true), json!(true)),
            (json!(true), json!(false)),
            (json!(false), json!(false))
        ]
    );
    // The window of an application that found no settings folder cannot change any.
    let bare = call_in(
        &Context {
            works: &rig.works,
            product: &FakeRenderer,
            connections: None,
            policy: &rig.policy,
            entrance: Entrance::Desktop,
        },
        "describe",
        json!({}),
    )
    .unwrap();
    assert_eq!(
        (bare["settings"].clone(), bare["writes_settings"].clone()),
        (json!(false), json!(false))
    );
    for name in [
        "list_connections",
        "read_connection",
        "read_auth_policy",
        "save_connection",
        "delete_connection",
        "set_default_connection",
    ] {
        assert!(COMMANDS.contains(&name), "{name} is a command");
    }
}

#[test]
fn the_connections_a_person_made_are_listed_with_the_default() {
    let rig = Rig::new("cc-list");
    let empty = rig.desktop("list_connections", json!({}));
    assert_eq!(
        empty,
        json!({ "connections": [], "unreadable": [], "default": null })
    );

    rig.desktop(
        "save_connection",
        json!({ "connection": local_json("pc2") }),
    );
    let saved = rig.desktop(
        "save_connection",
        json!({ "connection": claude_json("main") }),
    );
    assert_eq!(saved["outcome"], json!("saved"));
    rig.desktop(
        "set_default_connection",
        json!({ "id": "main", "expect": null }),
    );

    let listed = rig.desktop("list_connections", json!({}));

    let ids: Vec<&str> = listed["connections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["connection"]["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec!["main", "pc2"]);
    assert_eq!(listed["default"], json!("main"));
    assert_eq!(listed["connections"][0]["revision"], saved["revision"]);
    assert_eq!(listed["unreadable"], json!([]));
}

#[test]
fn a_connection_reads_by_id_and_by_an_earlier_revision() {
    let rig = Rig::new("cc-read");
    let first = rig.desktop(
        "save_connection",
        json!({ "connection": claude_json("main") }),
    );
    let mut changed = claude_json("main");
    changed["model"] = json!("sonnet");
    rig.desktop(
        "save_connection",
        json!({ "connection": changed, "base": first["revision"] }),
    );

    let now = rig.desktop("read_connection", json!({ "id": "main" }));
    let before = rig.desktop(
        "read_connection",
        json!({ "id": "main", "revision": first["revision"] }),
    );
    let none = rig.desktop("read_connection", json!({ "id": "other" }));

    assert_eq!(now["connection"]["connection"]["model"], json!("sonnet"));
    assert_eq!(before["connection"]["connection"]["model"], json!("opus"));
    assert_eq!(before["connection"]["revision"], first["revision"]);
    assert_eq!(none, json!({ "connection": null }));
    let refused = rig
        .ask(
            Entrance::Desktop,
            "read_connection",
            json!({ "id": "Not An Id" }),
        )
        .unwrap_err();
    assert_eq!(refused.kind, "bad-connection");
}

#[test]
fn only_the_desktop_changes_settings_and_the_others_read_them() {
    let rig = Rig::new("cc-entrances");
    let first = rig.desktop(
        "save_connection",
        json!({ "connection": claude_json("main") }),
    );
    let write_calls = [
        (
            "save_connection",
            json!({ "connection": claude_json("other") }),
        ),
        (
            "delete_connection",
            json!({ "id": "main", "base": first["revision"] }),
        ),
        (
            "set_default_connection",
            json!({ "id": "main", "expect": null }),
        ),
    ];

    for entrance in [Entrance::Browser, Entrance::Tool] {
        for (name, args) in &write_calls {
            let refused = rig.ask(entrance, name, args.clone()).unwrap_err();
            assert_eq!(refused.kind, "not-allowed-here", "{name} from {entrance:?}");
        }
        // Whoever has a settings folder may read it; only changing it is the desktop's.
        let listed = rig.ask(entrance, "list_connections", json!({}));
        assert!(listed.is_ok(), "{entrance:?} reads settings: {listed:?}");
    }

    let listed = rig.desktop("list_connections", json!({}));
    assert_eq!(listed["connections"].as_array().unwrap().len(), 1);
    assert_eq!(listed["default"], json!(null));
}

#[test]
fn an_entrance_with_no_settings_folder_says_so() {
    let rig = Rig::new("cc-no-settings");

    let refused = call(&rig.works, &FakeRenderer, "list_connections", json!({})).unwrap_err();

    assert_eq!(refused.kind, "no-settings");
}

#[test]
fn a_command_cannot_carry_a_program_to_run() {
    let rig = Rig::new("cc-executable");
    let mut connection = claude_json("main");
    connection["executable"] = json!("/tmp/anything");

    let refused = rig
        .ask(
            Entrance::Desktop,
            "save_connection",
            json!({ "connection": connection }),
        )
        .unwrap_err();

    // A program is chosen in a window the person sees and is asked what it is before it is
    // kept: a command that takes a path would be a way to make the application run it.
    assert_eq!(refused.kind, "bad-connection");
    assert!(
        refused.message.contains("executable"),
        "{}",
        refused.message
    );
    assert!(rig.connections.list().unwrap().connections.is_empty());
}

#[test]
fn a_command_refuses_a_field_it_does_not_know() {
    let rig = Rig::new("cc-unknown-field");
    let mut connection = claude_json("main");
    connection["api_key"] = json!("sk-secret");

    let refused = rig
        .ask(
            Entrance::Desktop,
            "save_connection",
            json!({ "connection": connection }),
        )
        .unwrap_err();

    assert_eq!(refused.kind, "bad-request");
    assert!(rig.connections.list().unwrap().connections.is_empty());
}

#[test]
fn deleting_and_choosing_a_default_are_checked_against_what_they_replace() {
    let rig = Rig::new("cc-delete");
    let first = rig.desktop(
        "save_connection",
        json!({ "connection": claude_json("main") }),
    );
    let mut changed = claude_json("main");
    changed["model"] = json!("sonnet");
    let second = rig.desktop(
        "save_connection",
        json!({ "connection": changed, "base": first["revision"] }),
    );

    let stale = rig
        .ask(
            Entrance::Desktop,
            "delete_connection",
            json!({ "id": "main", "base": first["revision"] }),
        )
        .unwrap_err();
    assert_eq!(stale.kind, "conflict");

    rig.desktop(
        "set_default_connection",
        json!({ "id": "main", "expect": null }),
    );
    let moved = rig
        .ask(
            Entrance::Desktop,
            "set_default_connection",
            json!({ "id": null, "expect": null }),
        )
        .unwrap_err();
    assert_eq!(moved.kind, "moved");

    let deleted = rig.desktop(
        "delete_connection",
        json!({ "id": "main", "base": revision_of(&second) }),
    );
    assert_eq!(deleted, json!({ "deleted": "main" }));
    let listed = rig.desktop("list_connections", json!({}));
    assert_eq!(listed["connections"], json!([]));
    assert_eq!(listed["default"], json!(null));
}

#[test]
fn the_policy_is_read_with_the_decision_each_route_gets() {
    let rig = Rig::new("cc-policy");

    let policy = rig
        .ask(Entrance::Browser, "read_auth_policy", json!({}))
        .unwrap();

    let routes = policy["routes"].as_array().unwrap();
    assert_eq!(routes.len(), Route::ALL.len());
    let find = |word: &str| {
        routes
            .iter()
            .find(|r| r["route"] == json!(word))
            .unwrap_or_else(|| panic!("{word} is a route"))
            .clone()
    };
    assert_eq!(find("claude-api-key")["status"], json!("allowed"));
    assert_eq!(
        find("claude-official-login")["decision"],
        json!({ "decision": "use", "status": "conditional" })
    );
    assert_eq!(
        find("unlisted")["decision"],
        json!({ "decision": "refuse", "status": "unconfirmed", "reason": "unconfirmed" })
    );
    assert_eq!(policy["switched_off"], json!([]));
}

#[test]
fn a_route_a_release_switched_off_is_read_as_switched_off() {
    let mut rig = Rig::new("cc-policy-off");
    rig.policy = Policy::shipped().with_switched_off([Route::ClaudeOfficialLogin]);

    let policy = rig
        .ask(Entrance::Desktop, "read_auth_policy", json!({}))
        .unwrap();

    let login = policy["routes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["route"] == json!("claude-official-login"))
        .unwrap();
    assert_eq!(
        login["decision"],
        json!({ "decision": "refuse", "status": "conditional", "reason": "switched-off" })
    );
    assert_eq!(policy["switched_off"], json!(["claude-official-login"]));
}

#[test]
fn what_a_command_saves_is_what_the_store_reads_back() {
    let rig = Rig::new("cc-roundtrip");

    rig.desktop(
        "save_connection",
        json!({ "connection": local_json("pc2") }),
    );

    let stored = rig.connections.list().unwrap().connections;
    let expected: Connection = serde_json::from_value(local_json("pc2")).unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].connection, expected);
}
