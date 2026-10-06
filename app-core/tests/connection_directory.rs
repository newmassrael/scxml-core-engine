// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Running for the connection a request was made for: who is signed in, and whether that is a way
//! the build uses.
//!
//! The directory reads the connection at the revision the request pinned, asks the client who is
//! signed in (with the options a generation runs with, so that what the screen shows is what a
//! generation uses), judges the way of signing in by the build's table, and builds the generator
//! with the model and the limits the request pinned. What it cannot run it says why, in words a
//! person acts on. The client here is a script, because what is held is what is asked of it and
//! what is done with the answer.
//!
//! Unix only: the stand-in is a shell script.

#![cfg(unix)]

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::Duration;

use sce_app_core::claude_code::{
    observe_auth, observed_from_status, AuthorServer, ClaudeCodeConfig,
};
use sce_app_core::directory::{ClaudeLaunch, Connections};
use sce_app_core::requests::Pin;
use sce_app_core::runner::{Directory, Unrunnable};
use sce_app_core::{
    AdapterKind, AuthSource, Connection, ConnectionId, ConnectionStore, Limits, Observed, Policy,
    Route,
};

/// A stand-in for `claude` that answers `--version` and `auth status`, and records what it was
/// asked.
struct Fake {
    binary: PathBuf,
    record: PathBuf,
}

impl Fake {
    fn new(label: &str, status: &str, exit: i32) -> Fake {
        let dir = common::scratch(label);
        let record = dir.join("record");
        fs::create_dir_all(&record).unwrap();
        fs::write(record.join("status.json"), status).unwrap();
        let binary = dir.join("claude");
        let script = format!(
            "#!/bin/sh\n\
             R='{record}'\n\
             if [ \"$1\" = \"--version\" ]; then echo \"2.1.290 (Claude Code)\"; exit 0; fi\n\
             for a in \"$@\"; do printf '%s\\0' \"$a\"; done > \"$R/argv\"\n\
             cat \"$R/status.json\"\n\
             exit {exit}\n",
            record = record.display()
        );
        fs::write(&binary, script).unwrap();
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
        Fake { binary, record }
    }

    fn argv(&self) -> Vec<String> {
        let bytes = fs::read(self.record.join("argv")).unwrap();
        let bytes = bytes.strip_suffix(&[0]).unwrap_or(&bytes);
        bytes
            .split(|b| *b == 0)
            .map(|part| String::from_utf8(part.to_vec()).unwrap())
            .collect()
    }
}

const SUBSCRIPTION: &str =
    r#"{"loggedIn":true,"authMethod":"claude.ai","apiProvider":"firstParty"}"#;
const API_KEY: &str = r#"{"loggedIn":true,"authMethod":"api_key","apiKeySource":"ANTHROPIC_API_KEY","apiProvider":"firstParty"}"#;
const SIGNED_OUT: &str = r#"{"loggedIn":false,"authMethod":"none","apiProvider":"firstParty"}"#;

#[test]
fn what_a_client_says_of_who_is_signed_in_is_read_as_a_kind_of_credential() {
    let table = [
        (SUBSCRIPTION, Some(Observed::Subscription)),
        (API_KEY, Some(Observed::ApiKey)),
        (
            r#"{"loggedIn":true,"authMethod":"api_key_helper","apiProvider":"firstParty"}"#,
            Some(Observed::ApiKey),
        ),
        (SIGNED_OUT, None),
        // A cloud provider's credential is not a login the client tracks: it says so by the
        // provider, whatever it says of the login.
        (
            r#"{"loggedIn":false,"authMethod":"none","apiProvider":"bedrock"}"#,
            Some(Observed::CloudProvider),
        ),
        (
            r#"{"loggedIn":true,"authMethod":"claude.ai","apiProvider":"vertex"}"#,
            Some(Observed::CloudProvider),
        ),
        // Anything the table does not name is not guessed at.
        (
            r#"{"loggedIn":true,"authMethod":"oauth_token","apiProvider":"firstParty"}"#,
            Some(Observed::Other),
        ),
    ];

    for (said, observed) in table {
        assert_eq!(observed_from_status(said), Ok(observed), "{said}");
    }
}

#[test]
fn something_that_is_not_what_the_client_says_is_not_read() {
    for said in ["", "not json", "[]", r#"{"authMethod":"claude.ai"}"#] {
        assert!(observed_from_status(said).is_err(), "`{said}`");
    }
}

#[test]
fn the_client_is_asked_with_the_options_a_generation_runs_with() {
    let fake = Fake::new("dir-argv", SUBSCRIPTION, 0);

    let observed = observe_auth(&fake.binary).unwrap();

    assert_eq!(observed, Some(Observed::Subscription));
    // The machine's settings are not read: a login a generation cannot see (a key a settings
    // file names) is not one the screen may show, and `--setting-sources ""` is what a
    // generation is run with.
    assert_eq!(
        fake.argv(),
        vec!["--setting-sources", "", "auth", "status", "--json"]
    );
}

#[test]
fn a_client_that_says_it_is_signed_out_by_failing_is_still_read() {
    // `claude auth status` fails when nobody is signed in, and still says so in JSON.
    let fake = Fake::new("dir-signed-out", SIGNED_OUT, 1);

    assert_eq!(observe_auth(&fake.binary), Ok(None));
}

#[test]
fn a_client_that_cannot_be_asked_is_not_a_login() {
    let missing = common::scratch("dir-missing").join("claude");
    assert!(observe_auth(&missing).is_err());
    let garbage = Fake::new("dir-garbage", "this is not json", 0);
    assert!(observe_auth(&garbage.binary).is_err());
}

struct Rig {
    settings: ConnectionStore,
    fake: Fake,
}

fn author() -> AuthorServer {
    AuthorServer {
        command: PathBuf::from("/nonexistent/sce-author-mcp"),
        args: Vec::new(),
        env: Vec::new(),
    }
}

impl Rig {
    fn new(label: &str, status: &str) -> Rig {
        Rig {
            settings: ConnectionStore::at(common::scratch(&format!("{label}-settings"))),
            fake: Fake::new(&format!("{label}-claude"), status, 0),
        }
    }

    fn directory(&self, policy: Policy) -> Connections {
        Connections::new(
            self.settings.clone(),
            policy,
            Some(ClaudeLaunch {
                binary: self.fake.binary.clone(),
                author: author(),
            }),
        )
    }

    /// Save `connection` and pin it as a request made for it would.
    fn pin(&self, connection: &Connection) -> Pin {
        let saved = self.settings.save(connection, None).unwrap();
        let revision = match saved {
            sce_app_core::Saved::Saved { revision, .. }
            | sce_app_core::Saved::Unchanged { revision } => revision,
        };
        Pin {
            connection: connection.id.clone(),
            revision,
            adapter: connection.adapter,
            model: connection.model.clone(),
            limits: connection.limits.clone(),
        }
    }
}

fn claude() -> Connection {
    Connection {
        id: ConnectionId::parse("main").unwrap(),
        adapter: AdapterKind::ClaudeCode,
        display_name: None,
        executable: None,
        model: Some("opus".to_string()),
        auth: AuthSource::OfficialLogin,
        server_url: None,
        limits: Limits {
            turns: Some(40),
            seconds: Some(600),
        },
    }
}

fn refused(result: Result<impl Sized, Unrunnable>) -> String {
    match result {
        Ok(_) => panic!("it should not be runnable"),
        Err(why) => why.reason,
    }
}

#[test]
fn a_connection_that_is_signed_in_is_run_with_the_model_and_the_limits_it_pinned() {
    let rig = Rig::new("dir-config", SUBSCRIPTION);
    let pin = rig.pin(&claude());

    let client = rig.directory(Policy::shipped()).claude_for(&pin).unwrap();

    let config: &ClaudeCodeConfig = client.config();
    assert_eq!(config.model.as_deref(), Some("opus"));
    assert_eq!(config.max_turns, 40);
    assert_eq!(config.timeout, Duration::from_secs(600));
    // And it is a generator the runner can take it with.
    assert!(rig.directory(Policy::shipped()).generator_for(&pin).is_ok());
}

#[test]
fn what_a_connection_does_not_limit_is_left_to_the_clients_own_defaults() {
    let rig = Rig::new("dir-defaults", API_KEY);
    let mut open = claude();
    open.model = None;
    open.limits = Limits::default();
    let pin = rig.pin(&open);

    let client = rig.directory(Policy::shipped()).claude_for(&pin).unwrap();

    let defaults = ClaudeCodeConfig::default();
    assert_eq!(client.config().model, None);
    assert_eq!(client.config().max_turns, defaults.max_turns);
    assert_eq!(client.config().timeout, defaults.timeout);
}

#[test]
fn a_way_of_signing_in_the_build_uses_is_run_and_one_it_does_not_is_said_so() {
    let table = [
        (SUBSCRIPTION, Route::ClaudeOfficialLogin, true, ""),
        (API_KEY, Route::ClaudeApiKey, true, ""),
        (
            r#"{"loggedIn":false,"authMethod":"none","apiProvider":"bedrock"}"#,
            Route::ClaudeCloudProvider,
            true,
            "",
        ),
        (
            r#"{"loggedIn":true,"authMethod":"oauth_token","apiProvider":"firstParty"}"#,
            Route::Unlisted,
            false,
            "not one this build uses",
        ),
    ];

    for (status, route, runnable, words) in table {
        let rig = Rig::new("dir-routes", status);
        let pin = rig.pin(&claude());
        let result = rig.directory(Policy::shipped()).claude_for(&pin);
        assert_eq!(result.is_ok(), runnable, "{route:?}");
        if !runnable {
            let said = refused(result);
            assert!(said.contains(words), "{route:?}: {said}");
        }
    }
}

#[test]
fn nobody_signed_in_says_how_to_sign_in() {
    let rig = Rig::new("dir-signed-out", SIGNED_OUT);
    let pin = rig.pin(&claude());

    let said = refused(rig.directory(Policy::shipped()).claude_for(&pin));

    assert!(said.contains("claude auth login"), "{said}");
}

#[test]
fn a_route_a_release_switched_off_is_not_run_and_says_so() {
    let rig = Rig::new("dir-switched-off", SUBSCRIPTION);
    let pin = rig.pin(&claude());
    let policy = Policy::shipped().with_switched_off([Route::ClaudeOfficialLogin]);

    let said = refused(rig.directory(policy).claude_for(&pin));

    assert!(said.contains("switched off"), "{said}");
}

#[test]
fn a_kind_of_connection_the_build_has_no_adapter_for_is_not_run() {
    let rig = Rig::new("dir-adapters", SUBSCRIPTION);
    let mut codex = claude();
    codex.id = ConnectionId::parse("gpt").unwrap();
    codex.adapter = AdapterKind::Codex;
    codex.auth = AuthSource::EnvApiKey;
    let mut local = claude();
    local.id = ConnectionId::parse("pc2").unwrap();
    local.adapter = AdapterKind::Local;
    local.auth = AuthSource::NoAuth;
    local.display_name = Some("pc2 (tunnel)".to_string());
    local.server_url = Some("http://127.0.0.1:11434/v1".to_string());

    for connection in [codex, local] {
        let pin = rig.pin(&connection);
        let said = refused(rig.directory(Policy::shipped()).generator_for(&pin));
        assert!(
            said.contains(&format!("no adapter for `{}`", connection.adapter.word())),
            "{said}"
        );
    }
}

#[test]
fn settings_that_are_not_on_this_computer_are_said_to_be_missing() {
    let rig = Rig::new("dir-missing-settings", SUBSCRIPTION);
    let mut pin = rig.pin(&claude());
    // The request came with the works folder from another computer, and its revision is not here.
    pin.revision = sce_app_core::Revision::of(b"another computer");

    let said = refused(rig.directory(Policy::shipped()).claude_for(&pin));

    assert!(said.contains("not on this computer"), "{said}");
}

#[test]
fn no_client_to_run_is_said_so() {
    let rig = Rig::new("dir-no-client", SUBSCRIPTION);
    let pin = rig.pin(&claude());
    let directory = Connections::new(rig.settings.clone(), Policy::shipped(), None);

    let said = refused(directory.claude_for(&pin));

    assert!(said.contains("Claude Code was not found"), "{said}");
}
