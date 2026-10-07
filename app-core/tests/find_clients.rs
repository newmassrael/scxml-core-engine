// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Choosing which Claude Code a connection runs.
//!
//! A connection names a program the application runs, so naming one is a way to make the
//! application run a file, and a command that took any path would give that to whoever can send
//! it. So a program is chosen among the ones the application found and asked (`find_clients`),
//! and a save that names another is refused whatever that file says of itself. Only the desktop
//! window may ask or save, because asking starts the programs. What a connection names is then
//! what runs for it: the generation, and the status the screen shows.
//!
//! Unix only: the stand-ins are shell scripts.

#![cfg(unix)]

mod common;

use std::path::{Path, PathBuf};
use std::time::Duration;

use sce_app_core::claude_code::{AuthorServer, Search};
use sce_app_core::directory::{ClaudeLaunch, Connections};
use sce_app_core::requests::Pin;
use sce_app_core::{
    call_in, CommandError, ConnectionId, ConnectionStore, Context, Entrance, Policy, Revision,
    WorkStore,
};
use serde_json::{json, Value};

use common::{scratch, write_program, FakeRenderer};

const SUBSCRIPTION: &str =
    r#"{"loggedIn":true,"authMethod":"claude.ai","apiProvider":"firstParty"}"#;

/// A program called `claude` in a folder of its own, saying `says` to `--version` and
/// `status` to `auth status`.
fn client(label: &str, says: &str, status: &str) -> PathBuf {
    let program = scratch(label).join("claude");
    write_program(
        &program,
        &format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo '{says}'; exit 0; fi\n\
             echo '{status}'\nexit 0\n"
        ),
    );
    program
}

fn dir_of(program: &Path) -> &Path {
    program.parent().unwrap()
}

struct Rig {
    works: WorkStore,
    settings: ConnectionStore,
    policy: Policy,
    search: Search,
}

impl Rig {
    fn new(label: &str, found_in: &[&Path]) -> Rig {
        Rig {
            works: WorkStore::at(scratch(&format!("{label}-works"))),
            settings: ConnectionStore::at(scratch(&format!("{label}-settings"))),
            policy: Policy::shipped(),
            search: Search {
                path: found_in.iter().map(|dir| dir.to_path_buf()).collect(),
                known: vec![],
                timeout: Duration::from_secs(5),
            },
        }
    }

    fn ask(&self, entrance: Entrance, name: &str, args: Value) -> Result<Value, CommandError> {
        let context = Context::new(&self.works, &FakeRenderer, &self.policy, entrance)
            .with_connections(Some(&self.settings))
            .with_search(Some(&self.search));
        call_in(&context, name, args)
    }

    fn desktop(&self, name: &str, args: Value) -> Value {
        self.ask(Entrance::Desktop, name, args)
            .unwrap_or_else(|e| panic!("{name} was refused: {e:?}"))
    }
}

fn claude_json(executable: Option<&Path>) -> Value {
    let mut connection = json!({
        "id": "claude", "adapter": "claude-code", "model": "opus", "auth": "official-login",
    });
    if let Some(path) = executable {
        connection["executable"] = json!(path.display().to_string());
    }
    json!({ "connection": connection })
}

#[test]
fn the_desktop_is_told_which_clients_the_application_found() {
    let first = client("fc-first", "2.1.291 (Claude Code)", SUBSCRIPTION);
    let second = client("fc-second", "2.1.280 (Claude Code)", SUBSCRIPTION);
    let rig = Rig::new("fc-found", &[dir_of(&first), dir_of(&second)]);

    let said = rig.desktop("find_clients", json!({}));

    assert_eq!(
        said,
        json!({
            "claude": [
                { "path": first, "version": "2.1.291", "found": "search-path" },
                { "path": second, "version": "2.1.280", "found": "search-path" },
            ],
            // Nothing of Codex is on this search path, and that is said and not left out.
            "codex": [],
        })
    );
}

#[test]
fn only_the_desktop_may_ask_because_asking_starts_the_programs() {
    let program = client("fc-entrances", "2.1.291 (Claude Code)", SUBSCRIPTION);
    let rig = Rig::new("fc-entrances", &[dir_of(&program)]);

    for entrance in [Entrance::Browser, Entrance::Tool] {
        let refused = rig.ask(entrance, "find_clients", json!({})).unwrap_err();
        assert_eq!(refused.kind, "not-allowed-here", "{entrance:?}");
    }
}

#[test]
fn a_connection_may_name_a_client_the_application_found() {
    let program = client("fc-name", "2.1.291 (Claude Code)", SUBSCRIPTION);
    let rig = Rig::new("fc-name", &[dir_of(&program)]);

    rig.desktop("save_connection", claude_json(Some(&program)));

    let kept = rig.desktop("read_connection", json!({ "id": "claude" }));
    assert_eq!(
        kept["connection"]["connection"]["executable"],
        json!(program.display().to_string())
    );
}

#[test]
fn a_program_the_application_did_not_find_is_refused_whatever_it_says_of_itself() {
    let found = client("fc-found-one", "2.1.291 (Claude Code)", SUBSCRIPTION);
    // Says exactly what Claude Code says, and is in no place the application looks.
    let elsewhere = client("fc-elsewhere", "2.1.291 (Claude Code)", SUBSCRIPTION);
    let rig = Rig::new("fc-unfound", &[dir_of(&found)]);

    let refused = rig
        .ask(
            Entrance::Desktop,
            "save_connection",
            claude_json(Some(&elsewhere)),
        )
        .unwrap_err();

    assert_eq!(refused.kind, "bad-connection");
    assert!(refused.message.contains("found"), "{}", refused.message);
    // And nothing was kept.
    let kept = rig.desktop("read_connection", json!({ "id": "claude" }));
    assert_eq!(kept["connection"], Value::Null);
}

#[test]
fn a_program_that_is_found_and_stops_saying_it_is_claude_code_is_not_kept() {
    let program = client("fc-changes", "2.1.291 (Claude Code)", SUBSCRIPTION);
    let rig = Rig::new("fc-changes", &[dir_of(&program)]);
    // Replaced, between the asking and the saving, by something else.
    write_program(
        &program,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'not it'; exit 0; fi\nexit 1\n",
    );

    let refused = rig
        .ask(
            Entrance::Desktop,
            "save_connection",
            claude_json(Some(&program)),
        )
        .unwrap_err();

    assert_eq!(refused.kind, "bad-connection");
}

#[test]
fn a_relative_name_and_a_program_for_a_kind_of_connection_with_none_are_refused() {
    let program = client("fc-shapes", "2.1.291 (Claude Code)", SUBSCRIPTION);
    let rig = Rig::new("fc-shapes", &[dir_of(&program)]);

    let relative = rig
        .ask(
            Entrance::Desktop,
            "save_connection",
            json!({ "connection": {
                "id": "claude", "adapter": "claude-code", "auth": "official-login",
                "executable": "claude",
            } }),
        )
        .unwrap_err();
    // A connection to a server of the person's runs no program of the application's to name.
    let local = rig
        .ask(
            Entrance::Desktop,
            "save_connection",
            json!({ "connection": {
                "id": "pc2", "adapter": "local", "auth": "none",
                "display_name": "pc2 (tunnel)", "server_url": "http://127.0.0.1:11434/v1",
                "executable": program.display().to_string(),
            } }),
        )
        .unwrap_err();

    assert_eq!(relative.kind, "bad-connection");
    assert_eq!(local.kind, "bad-connection");
}

/// A program called `codex` in a folder of its own, saying `says` to `--version`.
fn codex_client(label: &str, says: &str) -> PathBuf {
    let program = scratch(label).join("codex");
    write_program(
        &program,
        &format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo '{says}'; exit 0; fi\nexit 1\n"
        ),
    );
    program
}

fn codex_json(executable: &Path) -> Value {
    json!({ "connection": {
        "id": "gpt", "adapter": "codex", "model": "gpt-test", "auth": "env-api-key",
        "executable": executable.display().to_string(),
    } })
}

#[test]
fn a_codex_the_application_found_is_offered_beside_claude_code() {
    let claude = client("fc-both-claude", "2.1.291 (Claude Code)", SUBSCRIPTION);
    let codex = codex_client("fc-both-codex", "codex-cli 0.159.0");
    let rig = Rig::new("fc-both", &[dir_of(&claude), dir_of(&codex)]);

    let said = rig.desktop("find_clients", json!({}));

    assert_eq!(
        said["claude"],
        json!([{ "path": claude, "version": "2.1.291", "found": "search-path" }])
    );
    assert_eq!(
        said["codex"],
        json!([{ "path": codex, "version": "0.159.0", "found": "search-path" }])
    );
}

#[test]
fn a_program_that_answers_to_codex_and_says_anything_else_is_not_offered() {
    let impostor = codex_client("fc-codex-impostor", "hello 1.0");
    let rig = Rig::new("fc-codex-impostor", &[dir_of(&impostor)]);

    let said = rig.desktop("find_clients", json!({}));

    assert_eq!(said["codex"], json!([]));
}

#[test]
fn a_connection_to_codex_may_name_a_codex_the_application_found() {
    let codex = codex_client("fc-codex-name", "codex-cli 0.159.0");
    let rig = Rig::new("fc-codex-name", &[dir_of(&codex)]);

    rig.desktop("save_connection", codex_json(&codex));

    let kept = rig.desktop("read_connection", json!({ "id": "gpt" }));
    assert_eq!(
        kept["connection"]["connection"]["executable"],
        json!(codex.display().to_string())
    );
}

#[test]
fn a_connection_names_a_program_of_its_own_kind_and_not_the_others() {
    let claude = client("fc-kinds-claude", "2.1.291 (Claude Code)", SUBSCRIPTION);
    let codex = codex_client("fc-kinds-codex", "codex-cli 0.159.0");
    let rig = Rig::new("fc-kinds", &[dir_of(&claude), dir_of(&codex)]);

    // Both were found, and each is the other kind's program, not this one's.
    let codex_names_claude = rig
        .ask(Entrance::Desktop, "save_connection", codex_json(&claude))
        .unwrap_err();
    let claude_names_codex = rig
        .ask(
            Entrance::Desktop,
            "save_connection",
            claude_json(Some(&codex)),
        )
        .unwrap_err();

    assert_eq!(codex_names_claude.kind, "bad-connection");
    assert_eq!(claude_names_codex.kind, "bad-connection");
}

#[test]
fn a_codex_the_application_did_not_find_is_refused_whatever_it_says_of_itself() {
    let found = codex_client("fc-codex-found", "codex-cli 0.159.0");
    let elsewhere = codex_client("fc-codex-elsewhere", "codex-cli 0.159.0");
    let rig = Rig::new("fc-codex-unfound", &[dir_of(&found)]);

    let refused = rig
        .ask(Entrance::Desktop, "save_connection", codex_json(&elsewhere))
        .unwrap_err();

    assert_eq!(refused.kind, "bad-connection");
    let kept = rig.desktop("read_connection", json!({ "id": "gpt" }));
    assert_eq!(kept["connection"], Value::Null);
}

#[test]
fn the_codex_status_the_screen_shows_is_of_the_program_the_default_connection_names() {
    let automatic = codex_client("fc-codex-auto", "codex-cli 0.158.0");
    let chosen = codex_client("fc-codex-chosen", "codex-cli 0.159.0");
    let rig = Rig::new("fc-codex-status", &[dir_of(&automatic), dir_of(&chosen)]);
    rig.desktop("save_connection", codex_json(&chosen));
    rig.desktop(
        "set_default_connection",
        json!({ "id": "gpt", "expect": null }),
    );

    let said = rig.desktop("read_codex_status", json!({}));

    // The one the connection names answers, though the first one found is another.
    assert_eq!(said["codex"]["client"]["version"], "0.159.0");
    assert_eq!(
        said["codex"]["client"]["path"],
        json!(chosen.display().to_string())
    );
}

#[test]
fn another_entrance_cannot_save_a_program_at_all() {
    let program = client("fc-others", "2.1.291 (Claude Code)", SUBSCRIPTION);
    let rig = Rig::new("fc-others", &[dir_of(&program)]);

    let refused = rig
        .ask(
            Entrance::Browser,
            "save_connection",
            claude_json(Some(&program)),
        )
        .unwrap_err();

    assert_eq!(refused.kind, "not-allowed-here");
}

#[test]
fn the_status_the_screen_shows_is_of_the_program_the_connection_names() {
    let automatic = client("fc-auto", "2.1.291 (Claude Code)", SUBSCRIPTION);
    let chosen = client(
        "fc-chosen",
        "2.1.280 (Claude Code)",
        r#"{"loggedIn":false,"authMethod":"none","apiProvider":"firstParty"}"#,
    );
    let rig = Rig::new("fc-status", &[dir_of(&automatic), dir_of(&chosen)]);
    rig.desktop("save_connection", claude_json(Some(&chosen)));
    rig.desktop(
        "set_default_connection",
        json!({ "id": "claude", "expect": null }),
    );

    let said = rig.desktop("read_claude_status", json!({}));

    // The program the connection names answers, and says nobody is signed in to it, though the
    // first one found would say somebody is.
    assert_eq!(said["claude"]["client"]["version"], "2.1.280");
    assert_eq!(
        said["claude"]["client"]["path"],
        json!(chosen.display().to_string())
    );
    assert_eq!(said["claude"]["account"]["state"], "signed-out");
}

const SIGNED_OUT: &str = r#"{"loggedIn":false,"authMethod":"none","apiProvider":"firstParty"}"#;

#[test]
fn the_status_is_of_the_connection_the_screen_asks_about_whether_or_not_it_is_the_default() {
    let automatic = client("fc-ask-auto", "2.1.291 (Claude Code)", SUBSCRIPTION);
    let chosen = client("fc-ask-chosen", "2.1.280 (Claude Code)", SIGNED_OUT);
    let codex = codex_client("fc-ask-codex", "codex-cli 0.159.0");
    let rig = Rig::new(
        "fc-ask",
        &[dir_of(&automatic), dir_of(&chosen), dir_of(&codex)],
    );
    // The program is kept for Claude Code and the connection is not made the default: the
    // default is the one to Codex, as it is when the program button of a Claude Code that nobody
    // is signed in to is pressed.
    rig.desktop("save_connection", codex_json(&codex));
    rig.desktop(
        "set_default_connection",
        json!({ "id": "gpt", "expect": null }),
    );
    rig.desktop("save_connection", claude_json(Some(&chosen)));

    let asked = rig.desktop("read_claude_status", json!({ "connection": "claude" }));
    let not_asked = rig.desktop("read_claude_status", json!({}));

    // The program the connection names answers, as it does for a generation made for it.
    assert_eq!(asked["claude"]["client"]["version"], "2.1.280");
    assert_eq!(asked["claude"]["account"]["state"], "signed-out");
    // Asked about no connection, it is what it was: the default is not one to Claude Code, so
    // the application's own choice answers.
    assert_eq!(not_asked["claude"]["client"]["version"], "2.1.291");
}

#[test]
fn the_codex_status_is_of_the_connection_the_screen_asks_about_whether_or_not_it_is_the_default() {
    let automatic = codex_client("fc-ask-codex-auto", "codex-cli 0.158.0");
    let chosen = codex_client("fc-ask-codex-chosen", "codex-cli 0.159.0");
    let claude = client("fc-ask-claude", "2.1.291 (Claude Code)", SUBSCRIPTION);
    let rig = Rig::new(
        "fc-ask-codex",
        &[dir_of(&automatic), dir_of(&chosen), dir_of(&claude)],
    );
    rig.desktop("save_connection", claude_json(Some(&claude)));
    rig.desktop(
        "set_default_connection",
        json!({ "id": "claude", "expect": null }),
    );
    rig.desktop("save_connection", codex_json(&chosen));

    let asked = rig.desktop("read_codex_status", json!({ "connection": "gpt" }));
    let not_asked = rig.desktop("read_codex_status", json!({}));

    assert_eq!(asked["codex"]["client"]["version"], "0.159.0");
    assert_eq!(not_asked["codex"]["client"]["version"], "0.158.0");
}

#[test]
fn the_status_can_be_asked_of_a_program_not_kept_yet_and_of_the_applications_own_choice() {
    // The first one found is the application's own choice; the connection names the second.
    let first = client("fc-draft-first", "2.1.291 (Claude Code)", SUBSCRIPTION);
    let second = client("fc-draft-second", "2.1.280 (Claude Code)", SIGNED_OUT);
    let rig = Rig::new("fc-draft", &[dir_of(&first), dir_of(&second)]);
    rig.desktop("save_connection", claude_json(Some(&second)));
    rig.desktop(
        "set_default_connection",
        json!({ "id": "claude", "expect": null }),
    );

    let kept = rig.desktop("read_claude_status", json!({ "connection": "claude" }));
    let drafted = rig.desktop(
        "read_claude_status",
        json!({ "connection": "claude", "executable": first }),
    );
    let automatic = rig.desktop(
        "read_claude_status",
        json!({ "connection": "claude", "executable": null }),
    );

    // Asked about the connection, the program it names answers, as it did.
    assert_eq!(kept["claude"]["client"]["version"], "2.1.280");
    assert_eq!(kept["claude"]["account"]["state"], "signed-out");
    // Asked about a program that is about to be kept, that program answers, and not the one that
    // is kept: the login a screen shows is the login of the program that it would save.
    assert_eq!(drafted["claude"]["client"]["version"], "2.1.291");
    assert_eq!(drafted["claude"]["account"]["state"], "signed-in");
    // And asked about no program, the application's own choice answers, which the kept one does not.
    assert_eq!(automatic["claude"]["client"]["version"], "2.1.291");
}

#[test]
fn a_program_the_application_did_not_find_is_not_asked_whatever_it_says_of_itself() {
    let found = client("fc-ask-found", "2.1.291 (Claude Code)", SUBSCRIPTION);
    let elsewhere = client("fc-ask-elsewhere", "2.1.291 (Claude Code)", SUBSCRIPTION);
    let codex_found = codex_client("fc-ask-codex-found", "codex-cli 0.159.0");
    let codex_elsewhere = codex_client("fc-ask-codex-elsewhere", "codex-cli 0.159.0");
    let rig = Rig::new("fc-ask-unfound", &[dir_of(&found), dir_of(&codex_found)]);

    let claude = rig
        .ask(
            Entrance::Desktop,
            "read_claude_status",
            json!({ "executable": elsewhere }),
        )
        .unwrap_err();
    let codex = rig
        .ask(
            Entrance::Desktop,
            "read_codex_status",
            json!({ "executable": codex_elsewhere }),
        )
        .unwrap_err();
    // One of the other client's is not one of this client's.
    let crossed = rig
        .ask(
            Entrance::Desktop,
            "read_claude_status",
            json!({ "executable": codex_found }),
        )
        .unwrap_err();

    // A command that started whatever path it was given would be a way to run any file.
    for refused in [&claude, &codex, &crossed] {
        assert_eq!(refused.kind, "bad-connection");
        assert!(refused.message.contains("found"), "{}", refused.message);
    }
}

#[test]
fn the_codex_status_can_be_asked_of_a_program_not_kept_yet_as_well() {
    let first = codex_client("fc-draft-codex-first", "codex-cli 0.158.0");
    let second = codex_client("fc-draft-codex-second", "codex-cli 0.159.0");
    let claude = client(
        "fc-draft-codex-claude",
        "2.1.291 (Claude Code)",
        SUBSCRIPTION,
    );
    let rig = Rig::new(
        "fc-draft-codex",
        &[dir_of(&first), dir_of(&second), dir_of(&claude)],
    );
    rig.desktop("save_connection", codex_json(&second));

    let kept = rig.desktop("read_codex_status", json!({ "connection": "gpt" }));
    let drafted = rig.desktop(
        "read_codex_status",
        json!({ "connection": "gpt", "executable": first }),
    );

    assert_eq!(kept["codex"]["client"]["version"], "0.159.0");
    assert_eq!(drafted["codex"]["client"]["version"], "0.158.0");
}

#[test]
fn a_connection_that_names_no_program_is_not_given_the_one_the_default_names() {
    let automatic = client("fc-none-auto", "2.1.291 (Claude Code)", SUBSCRIPTION);
    let other = client("fc-none-other", "2.1.280 (Claude Code)", SIGNED_OUT);
    let rig = Rig::new("fc-none", &[dir_of(&automatic), dir_of(&other)]);
    // The default is another connection to Claude Code, and it names a program; the one the screen
    // is about names none.
    rig.desktop(
        "save_connection",
        json!({ "connection": {
            "id": "main", "adapter": "claude-code", "model": "opus", "auth": "official-login",
            "executable": other.display().to_string(),
        } }),
    );
    rig.desktop(
        "set_default_connection",
        json!({ "id": "main", "expect": null }),
    );
    rig.desktop("save_connection", claude_json(None));

    let asked = rig.desktop("read_claude_status", json!({ "connection": "claude" }));

    // The application's own choice answers for it, and not the default's program.
    assert_eq!(asked["claude"]["client"]["version"], "2.1.291");
}

#[test]
fn a_connection_of_another_kind_is_refused_and_one_not_kept_yet_is_asked_as_none_was_named() {
    let automatic = client("fc-kind-auto", "2.1.291 (Claude Code)", SUBSCRIPTION);
    let chosen = client("fc-kind-chosen", "2.1.280 (Claude Code)", SIGNED_OUT);
    let codex = codex_client("fc-kind-codex", "codex-cli 0.159.0");
    let rig = Rig::new(
        "fc-kind",
        &[dir_of(&automatic), dir_of(&chosen), dir_of(&codex)],
    );
    rig.desktop("save_connection", codex_json(&codex));
    rig.desktop("save_connection", claude_json(Some(&chosen)));
    rig.desktop(
        "set_default_connection",
        json!({ "id": "claude", "expect": null }),
    );

    // A connection to Codex is not one whose Claude Code is being asked about.
    let refused = rig
        .ask(
            Entrance::Desktop,
            "read_claude_status",
            json!({ "connection": "gpt" }),
        )
        .unwrap_err();
    // One that is not kept (yet) names no program, so what the default names answers, as before.
    let not_kept = rig.desktop("read_claude_status", json!({ "connection": "nobody" }));

    assert_eq!(refused.kind, "bad-connection");
    assert!(refused.message.contains("gpt"), "{}", refused.message);
    assert_eq!(not_kept["claude"]["client"]["version"], "2.1.280");
}

fn pin_of(rig: &Rig, executable: Option<&Path>) -> Pin {
    let saved = rig.desktop("save_connection", claude_json(executable));
    let revision = Revision::parse(saved["revision"].as_str().unwrap()).unwrap();
    Pin {
        connection: ConnectionId::parse("claude").unwrap(),
        revision,
        adapter: sce_app_core::AdapterKind::ClaudeCode,
        model: Some("opus".to_string()),
        limits: sce_app_core::Limits::default(),
    }
}

fn directory(rig: &Rig, launch: &Path) -> Connections {
    Connections::new(
        ConnectionStore::at(rig.settings.root().to_path_buf()),
        Policy::shipped(),
        Some(ClaudeLaunch {
            binary: launch.to_path_buf(),
            author: AuthorServer {
                command: PathBuf::from("/nowhere/sce-author-mcp"),
                args: vec![],
                env: vec![],
            },
            max_budget_usd: None,
        }),
    )
}

#[test]
fn a_generation_runs_the_program_the_connection_names_and_not_the_one_the_host_found() {
    let launch = client("fc-run-launch", "2.1.291 (Claude Code)", SUBSCRIPTION);
    let chosen = client("fc-run-chosen", "2.1.280 (Claude Code)", SUBSCRIPTION);
    let rig = Rig::new("fc-run", &[dir_of(&launch), dir_of(&chosen)]);
    let pin = pin_of(&rig, Some(&chosen));

    let generator = directory(&rig, &launch).claude_for(&pin).unwrap();

    assert_eq!(generator.binary(), chosen);
}

#[test]
fn a_connection_that_names_none_runs_the_one_the_host_found() {
    let launch = client("fc-none-launch", "2.1.291 (Claude Code)", SUBSCRIPTION);
    let rig = Rig::new("fc-none", &[dir_of(&launch)]);
    let pin = pin_of(&rig, None);

    let generator = directory(&rig, &launch).claude_for(&pin).unwrap();

    assert_eq!(generator.binary(), launch);
}

#[test]
fn a_program_a_connection_names_that_is_gone_is_said_to_be_gone_and_not_run_as_another() {
    let launch = client("fc-gone-launch", "2.1.291 (Claude Code)", SUBSCRIPTION);
    let chosen = client("fc-gone-chosen", "2.1.280 (Claude Code)", SUBSCRIPTION);
    let rig = Rig::new("fc-gone", &[dir_of(&launch), dir_of(&chosen)]);
    let pin = pin_of(&rig, Some(&chosen));
    std::fs::remove_file(&chosen).unwrap();

    let why = directory(&rig, &launch)
        .claude_for(&pin)
        .unwrap_err()
        .reason;

    assert!(why.contains("choose it again"), "{why}");
    // A reason is said in the works folder: it carries no path of this computer.
    assert!(!why.contains(chosen.to_str().unwrap()), "{why}");
}
