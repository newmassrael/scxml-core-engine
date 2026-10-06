// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The browser shell: who may call, what comes back, and what stays unreachable.

use std::net::IpAddr;
use std::path::PathBuf;

use sce_app_core::{ConnectionStore, NoRenderer, WorkStore};
use sce_web_shell::address::check_bind;
use sce_web_shell::assets::Assets;
use sce_web_shell::{token, Reply, Shell};
use serde_json::{json, Value};

const TOKEN: &str = "0123456789abcdef0123456789abcdef";

/// A scratch folder that is gone when the test is.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Scratch {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("shell-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn shell(scratch: &Scratch, assets: Option<Assets>) -> Shell {
    Shell::new(
        WorkStore::at(scratch.0.join("works")),
        Box::new(NoRenderer),
        TOKEN.to_string(),
        assets,
    )
}

fn post(shell: &Shell, auth: Option<&str>, body: &Value) -> Reply {
    shell.handle("POST", "/api/call", auth, body.to_string().as_bytes())
}

fn bearer() -> String {
    format!("Bearer {TOKEN}")
}

fn json_of(reply: &Reply) -> Value {
    serde_json::from_slice(&reply.body).expect("the body is JSON")
}

#[test]
fn a_call_without_the_token_is_refused_before_it_is_read() {
    let scratch = Scratch::new("no-token");
    let shell = shell(&scratch, None);
    let body = json!({"name": "list_works"});
    for auth in [None, Some("Bearer wrong"), Some("Basic abc"), Some(TOKEN)] {
        let reply = post(&shell, auth, &body);
        assert_eq!(reply.status, 401, "{auth:?}");
        assert_eq!(json_of(&reply)["kind"], "unauthorized");
    }
}

#[test]
fn a_call_with_the_token_reaches_the_command_layer() {
    let scratch = Scratch::new("call");
    let shell = shell(&scratch, None);
    let created = post(
        &shell,
        Some(&bearer()),
        &json!({"name": "create_work", "args": {"title": "Door lock"}}),
    );
    assert_eq!(
        created.status,
        200,
        "{}",
        String::from_utf8_lossy(&created.body)
    );
    let listed = post(&shell, Some(&bearer()), &json!({"name": "list_works"}));
    assert_eq!(listed.status, 200);
    assert_eq!(json_of(&listed)["works"][0]["title"], "Door lock");
}

#[test]
fn a_stale_save_travels_as_409_with_both_revisions() {
    let scratch = Scratch::new("conflict");
    let shell = shell(&scratch, None);
    let auth = bearer();
    let work = json_of(&post(
        &shell,
        Some(&auth),
        &json!({"name": "create_work", "args": {"title": "Pump"}}),
    ));
    let id = work["id"].as_str().unwrap();
    let first = json_of(&post(
        &shell,
        Some(&auth),
        &json!({"name": "save_source", "args": {"id": id, "text": "one"}}),
    ));
    let first = first["revision"].as_str().unwrap().to_string();
    let second = post(
        &shell,
        Some(&auth),
        &json!({"name": "save_source", "args": {"id": id, "text": "two", "base": first}}),
    );
    assert_eq!(second.status, 200);

    let stale = post(
        &shell,
        Some(&auth),
        &json!({"name": "save_source", "args": {"id": id, "text": "mine", "base": first}}),
    );
    assert_eq!(stale.status, 409);
    let error = json_of(&stale);
    assert_eq!(error["kind"], "conflict");
    assert_eq!(error["detail"]["base"], first);
    assert_ne!(error["detail"]["current"], first);
}

#[test]
fn each_refusal_travels_under_its_own_status() {
    let scratch = Scratch::new("statuses");
    let shell = shell(&scratch, None);
    let auth = bearer();
    let cases = [
        (json!({"name": "nope"}), 404, "unknown-command"),
        (
            json!({"name": "read_work", "args": {"id": "absent"}}),
            404,
            "not-found",
        ),
        (
            json!({"name": "read_work", "args": {"id": "../x"}}),
            400,
            "invalid-id",
        ),
        (
            json!({"name": "create_work", "args": {"title": ""}}),
            400,
            "invalid-title",
        ),
        (
            json!({"name": "list_works", "args": {"extra": 1}}),
            400,
            "bad-request",
        ),
        (
            json!({"name": "list_works", "extra": 1}),
            400,
            "bad-request",
        ),
    ];
    for (body, status, kind) in cases {
        let reply = post(&shell, Some(&auth), &body);
        assert_eq!(
            (reply.status, json_of(&reply)["kind"].clone()),
            (status, json!(kind)),
            "{body}"
        );
    }
    let not_json = shell.handle("POST", "/api/call", Some(&auth), b"{");
    assert_eq!(not_json.status, 400);
}

#[test]
fn only_post_reaches_the_command_endpoint_and_nothing_else_is_an_endpoint() {
    let scratch = Scratch::new("methods");
    let shell = shell(&scratch, None);
    let auth = bearer();
    assert_eq!(
        shell.handle("GET", "/api/call", Some(&auth), b"").status,
        405
    );
    assert_eq!(
        shell
            .handle("POST", "/api/other", Some(&auth), b"{}")
            .status,
        404
    );
    assert_eq!(shell.handle("DELETE", "/", None, b"").status, 405);
}

#[test]
fn the_screen_is_served_and_cannot_be_left() {
    let scratch = Scratch::new("assets");
    let ui = scratch.0.join("ui");
    std::fs::create_dir_all(ui.join("assets")).unwrap();
    std::fs::write(ui.join("index.html"), "<!doctype html><title>x</title>").unwrap();
    std::fs::write(ui.join("assets/app.js"), "export {}").unwrap();
    std::fs::write(scratch.0.join("secret.txt"), "not the screen").unwrap();
    let shell = shell(&scratch, Some(Assets::new(&ui).unwrap()));

    let home = shell.handle("GET", "/", None, b"");
    assert_eq!(
        (home.status, home.content_type),
        (200, "text/html; charset=utf-8")
    );
    let script = shell.handle("GET", "/assets/app.js?v=1", None, b"");
    assert_eq!(
        (script.status, script.content_type),
        (200, "text/javascript; charset=utf-8")
    );

    for escape in [
        "/../secret.txt",
        "/assets/../../secret.txt",
        "/%2e%2e/secret.txt",
        "/..%2fsecret.txt",
        "/assets\\..\\..\\secret.txt",
        "/missing.js",
    ] {
        let reply = shell.handle("GET", escape, None, b"");
        assert_eq!(reply.status, 404, "{escape}");
    }
}

#[test]
fn a_link_out_of_the_screen_folder_is_not_followed() {
    let scratch = Scratch::new("symlink");
    let ui = scratch.0.join("ui");
    std::fs::create_dir_all(&ui).unwrap();
    std::fs::write(scratch.0.join("secret.txt"), "not the screen").unwrap();
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(scratch.0.join("secret.txt"), ui.join("leak.txt")).unwrap();
        let shell = shell(&scratch, Some(Assets::new(&ui).unwrap()));
        assert_eq!(shell.handle("GET", "/leak.txt", None, b"").status, 404);
    }
}

#[test]
fn without_a_screen_a_get_says_so() {
    let scratch = Scratch::new("no-ui");
    let shell = shell(&scratch, None);
    let reply = shell.handle("GET", "/", None, b"");
    assert_eq!(reply.status, 404);
    assert!(json_of(&reply)["message"]
        .as_str()
        .unwrap()
        .contains("--ui"));
}

#[test]
fn only_loopback_and_tailnet_addresses_may_be_listened_on() {
    for ok in [
        "127.0.0.1",
        "::1",
        "100.64.0.1",
        "100.101.102.103",
        "100.127.255.254",
        "fd7a:115c:a1e0::1",
    ] {
        assert!(check_bind(ok.parse::<IpAddr>().unwrap()).is_ok(), "{ok}");
    }
    for refused in [
        "0.0.0.0",
        "::",
        "192.168.1.5",
        "10.0.0.1",
        "100.63.255.255",
        "100.128.0.0",
        "8.8.8.8",
        "fd7a:115c:a1e1::1",
    ] {
        let reason = check_bind(refused.parse::<IpAddr>().unwrap()).expect_err(refused);
        assert!(reason.contains("ssh -L"), "{refused}: {reason}");
    }
}

fn shell_with_settings(scratch: &Scratch) -> Shell {
    shell(scratch, None).with_settings(ConnectionStore::at(scratch.0.join("settings")))
}

#[test]
fn a_browser_reads_the_settings_and_is_refused_every_change_of_them() {
    let scratch = Scratch::new("settings");
    let shell = shell_with_settings(&scratch);

    let listed = post(
        &shell,
        Some(&bearer()),
        &json!({"name": "list_connections"}),
    );
    assert_eq!(listed.status, 200);
    assert_eq!(json_of(&listed)["connections"], json!([]));

    let connection = json!({"id": "main", "adapter": "claude-code", "auth": "official-login"});
    for (name, args) in [
        ("save_connection", json!({ "connection": connection })),
        (
            "delete_connection",
            json!({ "id": "main", "base": "0".repeat(64) }),
        ),
        (
            "set_default_connection",
            json!({ "id": "main", "expect": null }),
        ),
    ] {
        let refused = post(
            &shell,
            Some(&bearer()),
            &json!({ "name": name, "args": args }),
        );
        assert_eq!(refused.status, 403, "{name}");
        assert_eq!(json_of(&refused)["kind"], "not-allowed-here", "{name}");
    }
    // A shell a token reaches over a network changed nothing: the folder was not even made.
    assert!(!scratch.0.join("settings").exists());

    let bad = post(
        &shell,
        Some(&bearer()),
        &json!({"name": "read_connection", "args": {"id": "Not An Id"}}),
    );
    assert_eq!(bad.status, 400);
    assert_eq!(json_of(&bad)["kind"], "bad-connection");
}

#[test]
fn a_browser_shell_with_no_settings_folder_is_told_so() {
    let scratch = Scratch::new("no-settings");
    let shell = shell(&scratch, None);

    let reply = post(
        &shell,
        Some(&bearer()),
        &json!({"name": "list_connections"}),
    );

    assert_eq!(reply.status, 404);
    assert_eq!(json_of(&reply)["kind"], "no-settings");
}

#[test]
fn describe_says_it_is_the_browser_that_asked() {
    let scratch = Scratch::new("describe");
    let with = shell_with_settings(&scratch);
    let without = shell(&scratch, None);

    let described = json_of(&post(&with, Some(&bearer()), &json!({"name": "describe"})));
    let bare = json_of(&post(
        &without,
        Some(&bearer()),
        &json!({"name": "describe"}),
    ));

    assert_eq!(described["entrance"], "browser");
    assert_eq!(described["settings"], true);
    assert_eq!(described["writes_settings"], false);
    assert_eq!(described["starts_programs"], false);
    assert_eq!(bare["entrance"], "browser");
    assert_eq!(bare["settings"], false);
}

#[test]
fn the_browser_is_refused_a_command_that_would_start_the_persons_program() {
    let scratch = Scratch::new("starts-program");
    let with = shell_with_settings(&scratch);

    let reply = post(
        &with,
        Some(&bearer()),
        &json!({"name": "read_claude_status"}),
    );

    assert_eq!(reply.status, 403);
    assert_eq!(json_of(&reply)["kind"], "not-allowed-here");
}

#[test]
fn tokens_compare_whole_and_parse_from_the_header() {
    assert!(token::matches("abcd", "abcd"));
    assert!(!token::matches("abcd", "abce"));
    assert!(!token::matches("abcd", "abc"));
    assert!(!token::matches("abc", "abcd"));
    assert_eq!(token::bearer("Bearer t"), Some("t"));
    assert_eq!(token::bearer("bearer t"), Some("t"));
    assert_eq!(token::bearer("Bearer "), None);
    assert_eq!(token::bearer("Basic t"), None);
    assert_eq!(token::bearer("t"), None);
    let generated = token::generate().unwrap();
    assert_eq!(generated.len(), 64);
    assert_ne!(generated, token::generate().unwrap());
}
