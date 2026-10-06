// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the screen says of Claude Code: is it installed, who is signed in, and is that a way the
//! build uses.
//!
//! `read_claude_status` starts the client to ask it, so it is the desktop application's and no
//! other entrance's (a command that starts a program is a way to run one for whoever can send
//! it). It asks with the options a generation runs with, so that the login it shows is the login
//! a generation uses; it says who is signed in and how that is billed, and names an environment
//! variable only when the client said that one decided it, because that is a name and not a
//! secret. What it does not say is the person's account: the client's answer carries an email
//! and an organization, and neither is kept or shown. The client here is a script.
//!
//! Unix only: the stand-in is a shell script.

#![cfg(unix)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use sce_app_core::claude_code::account_from_status;
use sce_app_core::{
    call_in, ConnectionStore, Context, Entrance, Observed, Policy, Route, WorkStore,
};
use serde_json::{json, Value};

use common::{scratch, FakeRenderer};

const SUBSCRIPTION: &str = r#"{"loggedIn":true,"authMethod":"claude.ai","apiProvider":"firstParty","subscriptionType":"max","email":"person@example.test","orgId":"org-123","orgName":"Example Org"}"#;
const API_KEY: &str = r#"{"loggedIn":true,"authMethod":"api_key","apiKeySource":"ANTHROPIC_API_KEY","apiProvider":"firstParty"}"#;
const SIGNED_OUT: &str = r#"{"loggedIn":false,"authMethod":"none","apiProvider":"firstParty"}"#;
const BEDROCK: &str = r#"{"loggedIn":true,"authMethod":"third_party","apiProvider":"bedrock"}"#;

/// A stand-in for `claude` that answers `--version` and `auth status`, and records what it was
/// asked.
struct Fake {
    binary: PathBuf,
    record: PathBuf,
}

impl Fake {
    fn new(label: &str, status: &str, exit: i32) -> Fake {
        let dir = scratch(label);
        let record = dir.join("record");
        fs::create_dir_all(&record).unwrap();
        fs::write(record.join("status.json"), status).unwrap();
        let binary = dir.join("claude");
        let script = format!(
            "#!/bin/sh\n\
             R='{record}'\n\
             if [ \"$1\" = \"--version\" ]; then echo \"2.1.291 (Claude Code)\"; exit 0; fi\n\
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

/// Ask `read_claude_status` as `entrance`, with `claude` as the program the shell was told of.
fn ask(entrance: Entrance, claude: Option<&Path>, policy: &Policy) -> Result<Value, String> {
    let works = WorkStore::at(scratch("claude-status-works"));
    let settings = ConnectionStore::at(scratch("claude-status-settings"));
    let context = Context::new(&works, &FakeRenderer, policy, entrance)
        .with_connections(Some(&settings))
        .with_claude(claude);
    call_in(&context, "read_claude_status", json!({}))
        .map_err(|e| format!("{}: {}", e.kind, e.message))
}

fn desktop(claude: &Path) -> Value {
    ask(Entrance::Desktop, Some(claude), &Policy::shipped()).expect("an answer")["claude"].clone()
}

#[test]
fn what_the_client_says_is_read_as_a_credential_and_what_decided_it() {
    let account = account_from_status(SUBSCRIPTION).unwrap();
    assert_eq!(account.observed, Some(Observed::Subscription));
    assert_eq!(account.environment, None);

    let account = account_from_status(API_KEY).unwrap();
    assert_eq!(account.observed, Some(Observed::ApiKey));
    assert_eq!(account.environment.as_deref(), Some("ANTHROPIC_API_KEY"));

    let account = account_from_status(SIGNED_OUT).unwrap();
    assert_eq!(account.observed, None);

    // A cloud provider is named by the variable that selects it.
    let account = account_from_status(BEDROCK).unwrap();
    assert_eq!(account.observed, Some(Observed::CloudProvider));
    assert_eq!(
        account.environment.as_deref(),
        Some("CLAUDE_CODE_USE_BEDROCK")
    );
}

#[test]
fn a_source_that_is_not_the_name_of_a_variable_of_the_clients_is_not_shown() {
    // The client may name a helper program or a stored key: neither is a variable's name, and a
    // path or a helper's command line is not for the screen.
    for source in [
        "apiKeyHelper",
        "/home/person/bin/get-key",
        "/login managed key",
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY ",
        "anthropic_api_key",
    ] {
        let said = format!(
            r#"{{"loggedIn":true,"authMethod":"api_key","apiKeySource":"{source}","apiProvider":"firstParty"}}"#
        );
        let account = account_from_status(&said).unwrap();
        assert_eq!(account.observed, Some(Observed::ApiKey), "{source}");
        assert_eq!(account.environment, None, "{source}");
    }
}

#[test]
fn the_persons_account_is_not_in_what_is_read() {
    let account = account_from_status(SUBSCRIPTION).unwrap();
    let kept = format!("{account:?}");
    for secret in ["person@example.test", "org-123", "Example Org"] {
        assert!(!kept.contains(secret), "{kept}");
    }
}

#[test]
fn a_subscription_is_said_with_how_it_is_billed_and_what_the_build_does_with_it() {
    let fake = Fake::new("claude-status-subscription", SUBSCRIPTION, 0);

    let said = desktop(&fake.binary);

    assert_eq!(
        said["client"],
        json!({"state": "installed", "version": "2.1.291"})
    );
    assert_eq!(said["account"]["state"], "signed-in");
    assert_eq!(said["account"]["route"], "claude-official-login");
    assert_eq!(said["account"]["billing"], "subscription");
    assert_eq!(said["account"]["environment"], Value::Null);
    assert_eq!(said["account"]["usable"], true);
    assert_eq!(said["account"]["decision"]["decision"], "use");
    assert_eq!(said["account"]["decision"]["status"], "conditional");
}

#[test]
fn a_key_from_the_environment_is_said_to_be_used_instead_of_the_subscription() {
    let fake = Fake::new("claude-status-key", API_KEY, 0);

    let said = desktop(&fake.binary);

    assert_eq!(said["account"]["route"], "claude-api-key");
    assert_eq!(said["account"]["billing"], "usage");
    assert_eq!(said["account"]["environment"], "ANTHROPIC_API_KEY");
    assert_eq!(said["account"]["usable"], true);
}

#[test]
fn a_cloud_provider_is_billed_by_the_provider() {
    let fake = Fake::new("claude-status-cloud", BEDROCK, 0);

    let said = desktop(&fake.binary);

    assert_eq!(said["account"]["route"], "claude-cloud-provider");
    assert_eq!(said["account"]["billing"], "provider");
    assert_eq!(said["account"]["environment"], "CLAUDE_CODE_USE_BEDROCK");
}

#[test]
fn nobody_signed_in_is_said_with_the_commands_that_sign_in() {
    // The client fails when nobody is signed in, and still says so: what it printed counts.
    let fake = Fake::new("claude-status-out", SIGNED_OUT, 1);

    let said = desktop(&fake.binary);

    assert_eq!(said["client"]["state"], "installed");
    assert_eq!(said["account"], json!({"state": "signed-out"}));
    assert_eq!(
        said["sign_in"],
        json!([
            {"billing": "subscription", "command": "claude auth login"},
            {"billing": "usage", "command": "claude auth login --console"},
        ])
    );
}

#[test]
fn the_commands_that_sign_in_carry_no_path_and_no_variable() {
    let fake = Fake::new("claude-status-guidance", SIGNED_OUT, 1);

    let said = desktop(&fake.binary);

    for guidance in said["sign_in"].as_array().unwrap() {
        let command = guidance["command"].as_str().unwrap();
        assert!(command.starts_with("claude auth login"), "{command}");
        for forbidden in ["/", "\\", "$", "=", "HOME", "CONFIG"] {
            assert!(!command.contains(forbidden), "{command}");
        }
    }
}

#[test]
fn a_way_of_signing_in_the_build_does_not_use_is_said_and_not_offered() {
    // Something the table does not name: signed in, and not usable.
    let fake = Fake::new(
        "claude-status-other",
        r#"{"loggedIn":true,"authMethod":"oauth_token","apiProvider":"firstParty"}"#,
        0,
    );

    let said = desktop(&fake.binary);

    assert_eq!(said["account"]["state"], "signed-in");
    assert_eq!(said["account"]["route"], "unlisted");
    assert_eq!(said["account"]["usable"], false);
    assert_eq!(said["account"]["decision"]["decision"], "refuse");
    assert_eq!(said["account"]["decision"]["reason"], "unconfirmed");
}

#[test]
fn a_route_a_release_switched_off_is_signed_in_and_not_usable() {
    let fake = Fake::new("claude-status-off", SUBSCRIPTION, 0);
    let policy = Policy::shipped().with_switched_off([Route::ClaudeOfficialLogin]);

    let said = ask(Entrance::Desktop, Some(&fake.binary), &policy).unwrap()["claude"].clone();

    assert_eq!(said["account"]["usable"], false);
    assert_eq!(said["account"]["decision"]["reason"], "switched-off");
}

#[test]
fn the_client_is_asked_with_the_options_a_generation_runs_with() {
    let fake = Fake::new("claude-status-options", SUBSCRIPTION, 0);

    desktop(&fake.binary);

    assert_eq!(
        fake.argv(),
        ["--setting-sources", "", "auth", "status", "--json"]
    );
}

#[test]
fn a_program_that_is_not_there_is_a_client_to_install() {
    let said = ask(
        Entrance::Desktop,
        Some(Path::new("/nowhere/claude")),
        &Policy::shipped(),
    )
    .unwrap()["claude"]
        .clone();

    assert_eq!(said["client"]["state"], "missing");
    assert_eq!(said["account"]["state"], "unknown");
    assert!(said["sign_in"].as_array().unwrap().len() == 2);
}

#[test]
fn a_client_that_does_not_answer_json_is_not_a_login() {
    let fake = Fake::new("claude-status-garbled", "this is not json", 0);

    let said = desktop(&fake.binary);

    assert_eq!(said["client"]["state"], "installed");
    assert_eq!(said["account"]["state"], "unknown");
    assert!(said["account"]["reason"]
        .as_str()
        .unwrap()
        .contains("did not answer JSON"));
}

#[test]
fn what_starts_a_program_is_only_the_desktops() {
    let fake = Fake::new("claude-status-entrances", SUBSCRIPTION, 0);

    for entrance in [Entrance::Browser, Entrance::Tool] {
        let refused = ask(entrance, Some(&fake.binary), &Policy::shipped()).unwrap_err();
        assert!(refused.starts_with("not-allowed-here:"), "{refused}");
    }
    // And the client was not started for them.
    assert!(!fake.record.join("argv").exists());
}

#[test]
fn the_entrance_says_whether_it_may_start_a_program() {
    let policy = Policy::shipped();
    let works = WorkStore::at(scratch("claude-status-describe"));
    let said = |entrance| {
        let context = Context::new(&works, &FakeRenderer, &policy, entrance);
        call_in(&context, "describe", json!({})).unwrap()["starts_programs"].clone()
    };

    assert_eq!(said(Entrance::Desktop), json!(true));
    assert_eq!(said(Entrance::Browser), json!(false));
    assert_eq!(said(Entrance::Tool), json!(false));
}
