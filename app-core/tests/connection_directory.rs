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
use std::path::PathBuf;
use std::time::Duration;

use sce_app_core::claude_code::{
    observe_auth, observed_from_status, AuthorServer, ClaudeCodeConfig,
};
use sce_app_core::directory::{ClaudeLaunch, Connections};
use sce_app_core::local::LocalConfig;
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
        common::write_program(&binary, &script);
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
        self.directory_with(policy, None)
    }

    /// The same, for a host whose environment set `max_budget_usd` (`SCE_CLAUDE_BUDGET_USD`).
    fn directory_with(&self, policy: Policy, max_budget_usd: Option<f64>) -> Connections {
        Connections::new(
            self.settings.clone(),
            policy,
            Some(ClaudeLaunch {
                binary: self.fake.binary.clone(),
                author: author(),
                max_budget_usd,
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
            ..Limits::default()
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
fn what_the_environment_may_spend_bounds_a_run_made_for_a_connection_too() {
    // `SCE_CLAUDE_BUDGET_USD` is the person's limit on what the application spends, and a
    // connection has no word for money: a run made for one is as bounded as one that is not.
    let rig = Rig::new("dir-budget", SUBSCRIPTION);
    let pin = rig.pin(&claude());

    let bounded = rig
        .directory_with(Policy::shipped(), Some(2.5))
        .claude_for(&pin)
        .unwrap();
    let unbounded = rig.directory(Policy::shipped()).claude_for(&pin).unwrap();

    assert_eq!(bounded.config().max_budget_usd, Some(2.5));
    assert_eq!(unbounded.config().max_budget_usd, None);
    // The limit changes what a run may spend and nothing it asked for: the model and the turns
    // are the connection's.
    assert_eq!(bounded.config().model.as_deref(), Some("opus"));
    assert_eq!(bounded.config().max_turns, 40);
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
fn a_program_that_is_gone_is_said_to_be_gone_and_not_to_be_a_client_that_could_not_be_asked() {
    let rig = Rig::new("dir-gone", SUBSCRIPTION);
    let pin = rig.pin(&claude());
    // The host found it, and the person has removed it since.
    std::fs::remove_file(&rig.fake.binary).unwrap();

    let said = refused(rig.directory(Policy::shipped()).claude_for(&pin));

    assert!(said.contains("Claude Code was not found"), "{said}");
    assert!(!said.contains("could not be asked"), "{said}");
}

#[test]
fn nobody_signed_in_says_how_to_sign_in() {
    let rig = Rig::new("dir-signed-out", SIGNED_OUT);
    let pin = rig.pin(&claude());

    let said = refused(rig.directory(Policy::shipped()).claude_for(&pin));

    assert!(said.contains("claude auth login"), "{said}");
}

#[test]
fn what_a_person_is_told_of_a_request_that_waits_carries_nothing_the_client_printed() {
    // A reason is said where the works folder is, which is shared and moved: it is a sentence of
    // the application's, and not a path or a line the client wrote.
    let rig = Rig::new(
        "dir-printed",
        "/home/coin/.secret-place printed this, and is not JSON",
    );
    let pin = rig.pin(&claude());

    let said = refused(rig.directory(Policy::shipped()).claude_for(&pin));

    assert!(
        said.contains("could not be asked who is signed in"),
        "{said}"
    );
    for leaked in ["/home/coin", "secret-place", "printed this"] {
        assert!(!said.contains(leaked), "{said}");
    }
}

#[test]
fn a_route_a_release_switched_off_is_not_run_and_says_so() {
    let rig = Rig::new("dir-switched-off", SUBSCRIPTION);
    let pin = rig.pin(&claude());
    let policy = Policy::shipped().with_switched_off([Route::ClaudeOfficialLogin]);

    let said = refused(rig.directory(policy).claude_for(&pin));

    assert!(said.contains("switched off"), "{said}");
}

/// A connection to a model server on this computer, named as a person names one.
fn local() -> Connection {
    Connection {
        id: ConnectionId::parse("pc2").unwrap(),
        adapter: AdapterKind::Local,
        display_name: Some("pc2 (tunnel)".to_string()),
        executable: None,
        model: Some("qwen3-coder:30b".to_string()),
        auth: AuthSource::NoAuth,
        server_url: Some("http://127.0.0.1:11434/v1".to_string()),
        limits: Limits {
            turns: Some(30),
            seconds: Some(900),
            ..Limits::default()
        },
    }
}

#[test]
fn a_connection_to_a_model_server_is_run_with_its_model_its_address_and_its_limits() {
    let rig = Rig::new("dir-local", SUBSCRIPTION);
    let pin = rig.pin(&local());
    let directory = rig.directory(Policy::shipped());

    let server = directory.local_for(&pin).unwrap();

    assert_eq!(server.config().model, "qwen3-coder:30b");
    assert_eq!(server.config().max_turns, 30);
    assert_eq!(server.config().timeout, Duration::from_secs(900));
    assert!(server.endpoint().is_loopback());
    // And it is what the runner is given for the request, whichever client is or is not installed.
    assert_eq!(directory.generator_for(&pin).unwrap().kind(), "local");
}

#[test]
fn what_a_connection_does_not_limit_is_left_to_the_defaults_of_a_model_server_run() {
    let rig = Rig::new("dir-local-defaults", SUBSCRIPTION);
    let mut open = local();
    open.limits = Limits::default();
    let pin = rig.pin(&open);

    let server = rig.directory(Policy::shipped()).local_for(&pin).unwrap();

    let defaults = LocalConfig::for_model("qwen3-coder:30b");
    assert_eq!(server.config().max_turns, defaults.max_turns);
    assert_eq!(server.config().timeout, defaults.timeout);
    // Nothing is assumed of a model nobody described: the context is not known, and the
    // conversation is begun again only when the server refuses it.
    assert_eq!(server.config().handoffs, defaults.handoffs);
    assert_eq!(server.config().handoff_percent, defaults.handoff_percent);
    assert_eq!(server.config().context_tokens, None);
}

#[test]
fn what_a_connection_says_of_the_models_context_is_what_decides_when_to_begin_again() {
    let rig = Rig::new("dir-local-context", SUBSCRIPTION);
    let mut described = local();
    described.limits = Limits {
        handoffs: Some(4),
        context_tokens: Some(131_072),
        handoff_percent: Some(70),
        ..Limits::default()
    };
    let pin = rig.pin(&described);

    let server = rig.directory(Policy::shipped()).local_for(&pin).unwrap();

    assert_eq!(server.config().handoffs, 4);
    assert_eq!(server.config().context_tokens, Some(131_072));
    assert_eq!(server.config().handoff_percent, 70);
}

#[test]
fn a_model_server_is_run_where_no_client_is_installed_when_the_authoring_server_is_known() {
    let rig = Rig::new("dir-local-alone", SUBSCRIPTION);
    let pin = rig.pin(&local());
    let alone = Connections::new(rig.settings.clone(), Policy::shipped(), None);

    let without = refused(alone.local_for(&pin));
    let with = alone.with_author(author()).local_for(&pin);

    assert!(
        without.contains("authoring server was not found"),
        "{without}"
    );
    assert!(with.is_ok());
}

#[test]
fn an_address_over_https_is_one_a_model_server_is_run_at() {
    let rig = Rig::new("dir-local-https", SUBSCRIPTION);
    let mut remote = local();
    remote.server_url = Some("https://models.example.com/v1".to_string());
    let pin = rig.pin(&remote);

    let server = rig.directory(Policy::shipped()).local_for(&pin).unwrap();

    assert!(server.endpoint().is_tls());
    assert!(!server.endpoint().is_loopback());
}

#[test]
fn a_model_server_with_no_model_chosen_waits_and_says_which_without_saying_where_it_is() {
    let rig = Rig::new("dir-local-no-model", SUBSCRIPTION);
    let mut open = local();
    open.model = None;
    let pin = rig.pin(&open);

    let said = refused(rig.directory(Policy::shipped()).local_for(&pin));

    assert!(said.contains("no model is chosen"), "{said}");
    assert!(said.contains("pc2 (tunnel)"), "{said}");
    assert!(!said.contains("127.0.0.1"), "{said}");
}

#[test]
fn a_model_server_that_wants_a_key_waits_because_no_place_keeps_one_yet() {
    let rig = Rig::new("dir-local-key", SUBSCRIPTION);
    let mut keyed = local();
    keyed.auth = AuthSource::ServerKey;
    let pin = rig.pin(&keyed);

    let said = refused(rig.directory(Policy::shipped()).local_for(&pin));

    assert!(said.contains("wants a key"), "{said}");
    assert!(said.contains("no place to keep one"), "{said}");
}

#[test]
fn an_address_this_build_cannot_reach_is_said_without_the_address() {
    let rig = Rig::new("dir-local-address", SUBSCRIPTION);
    let mut bad = local();
    // Held to its characters by the settings, and to the rest of the form here.
    bad.server_url = Some("http://127.0.0.1:99999/v1".to_string());
    let pin = rig.pin(&bad);

    let said = refused(rig.directory(Policy::shipped()).local_for(&pin));

    assert!(said.contains("not one this build can reach"), "{said}");
    assert!(
        said.contains("the port is a number from 1 to 65535"),
        "{said}"
    );
    assert!(!said.contains("127.0.0.1"), "{said}");
}

#[test]
fn a_route_the_build_switched_off_is_not_run_for_a_model_server_either() {
    let rig = Rig::new("dir-local-off", SUBSCRIPTION);
    let pin = rig.pin(&local());
    let policy = Policy::shipped().with_switched_off([Route::LocalServer]);

    let said = refused(rig.directory(policy).local_for(&pin));

    assert!(said.contains("switched off"), "{said}");
}

#[test]
fn a_request_is_written_by_the_adapter_of_its_own_connection_and_by_no_other() {
    let rig = Rig::new("dir-adapters", SUBSCRIPTION);
    let for_server = rig.pin(&local());
    let for_claude = rig.pin(&claude());
    let directory = rig.directory(Policy::shipped());

    let claude_said = refused(directory.claude_for(&for_server));
    let server_said = refused(directory.local_for(&for_claude));
    let codex_said = refused(directory.codex_for(&for_server));

    assert!(
        claude_said.contains("made for a `local` connection"),
        "{claude_said}"
    );
    assert!(
        claude_said.contains("`claude-code` writes"),
        "{claude_said}"
    );
    assert!(
        server_said.contains("made for a `claude-code` connection"),
        "{server_said}"
    );
    assert!(server_said.contains("`local` writes"), "{server_said}");
    assert!(codex_said.contains("`codex` writes"), "{codex_said}");
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
