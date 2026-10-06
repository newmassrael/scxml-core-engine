// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the screen says of Codex: is it installed, did this build verify that version, and who is
//! signed in by each of the three ways a connection can take its credential.
//!
//! `read_codex_status` starts the client to ask it, so it is the desktop application's and no
//! other entrance's. Four things are kept apart because a screen acts on each differently: the
//! program being there, this build having verified that version (until it has, a generation
//! never runs, whoever is signed in), who is signed in by each source (asked the way a generation
//! is run, so the login shown is the login a generation uses), and whether the build uses that
//! way of signing in. A client that could not be asked is `unknown` and is said so, and never read
//! as nobody. Nothing of a credential is read into the answer: a key's variable is named and its
//! value is not. The client here is a script.
//!
//! Unix only: the stand-in is a shell script.

#![cfg(unix)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use sce_app_core::claude_code::Search;
use sce_app_core::codex::instructions_of;
use sce_app_core::codex_status;
use sce_app_core::codex_support::Support;
use sce_app_core::{call_in, ConnectionStore, Context, Entrance, Policy, Route, WorkStore};
use serde_json::{json, Value};

use common::{scratch, FakeRenderer};

const VERSION: &str = "0.159.0";
const APP_HOME: &str = "/app/data/codex-home";

const CHATGPT: &str = "Logged in using ChatGPT";
const API_KEY: &str = "Logged in using an API key - sk-...abcd";
const SIGNED_OUT: &str = "Not logged in";

/// A stand-in for `codex` that answers `--version`, `features list` and `login status`, and
/// records the home and the credentials it was started with whenever it is asked who is signed in.
struct Fake {
    binary: PathBuf,
    record: PathBuf,
}

impl Fake {
    /// `official` is what it says when started with no home of its own, and `store` what it says
    /// when started with one.
    fn new(label: &str, official: &str, store: &str) -> Fake {
        let dir = scratch(label);
        let record = dir.join("record");
        fs::create_dir_all(&record).unwrap();
        fs::write(record.join("official.txt"), official).unwrap();
        fs::write(record.join("store.txt"), store).unwrap();
        fs::write(
            record.join("features.txt"),
            "shell_tool  stable  true\nfast_mode  stable  true\n",
        )
        .unwrap();
        let binary = dir.join("codex");
        let script = format!(
            "#!/bin/sh\n\
             R='{record}'\n\
             if [ \"$1\" = \"--version\" ]; then echo 'codex-cli {VERSION}'; exit 0; fi\n\
             if [ \"$1\" = \"features\" ]; then cat \"$R/features.txt\"; exit 0; fi\n\
             if [ \"$1\" = \"login\" ]; then\n\
               printf 'home=%s openai=%s codex=%s\\n' \"$CODEX_HOME\" \"$OPENAI_API_KEY\" \"$CODEX_API_KEY\" >> \"$R/logins\"\n\
               if [ -n \"$CODEX_HOME\" ]; then text=$(cat \"$R/store.txt\"); else text=$(cat \"$R/official.txt\"); fi\n\
               echo \"$text\"\n\
               case \"$text\" in *'Not logged in'*) exit 1;; esac\n\
               exit 0\n\
             fi\n\
             exit 1\n",
            record = record.display()
        );
        common::write_program(&binary, &script);
        Fake { binary, record }
    }

    /// How many times it was asked who is signed in.
    fn logins(&self) -> Vec<String> {
        fs::read_to_string(self.record.join("logins"))
            .map(|text| text.lines().map(str::to_string).collect())
            .unwrap_or_default()
    }

    fn will_list(&self, features: &str) {
        fs::write(self.record.join("features.txt"), features).unwrap();
    }
}

/// A list that verifies this stand-in's version, with `shell_tool` switched off and `fast_mode`
/// reviewed.
fn verified() -> Support {
    let lists = |verified: Value| {
        Support::from_json(
            &json!({
                "verified": verified,
                "disabled_features": ["shell_tool"],
                "reviewed_features": ["fast_mode"],
            })
            .to_string(),
        )
        .unwrap()
    };
    let instructions = instructions_of(&lists(json!([])));
    lists(json!([{
        "os": std::env::consts::OS, "version": VERSION, "instructions": instructions
    }]))
}

fn environment(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    let mut environment = vec![("PATH".to_string(), "/usr/bin:/bin".to_string())];
    environment.extend(pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())));
    environment
}

fn nowhere() -> Search {
    Search {
        path: vec![],
        known: vec![],
        timeout: Duration::from_secs(15),
    }
}

fn read(
    named: Option<&Path>,
    policy: &Policy,
    support: &Support,
    environment: &[(String, String)],
) -> Value {
    serde_json::to_value(codex_status::read(
        named,
        policy,
        support,
        Path::new(APP_HOME),
        environment,
        &nowhere(),
    ))
    .unwrap()
}

fn account<'a>(status: &'a Value, source: &str) -> &'a Value {
    status["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["source"] == source)
        .unwrap_or_else(|| panic!("no account for {source}: {status}"))
}

#[test]
fn it_says_which_codex_is_there_and_that_this_build_verified_it() {
    let fake = Fake::new("cst-installed", CHATGPT, SIGNED_OUT);

    let status = read(
        Some(&fake.binary),
        &Policy::shipped(),
        &verified(),
        &environment(&[]),
    );

    assert_eq!(status["client"]["state"], "installed");
    assert_eq!(status["client"]["version"], VERSION);
    assert_eq!(
        status["client"]["path"],
        fake.binary.display().to_string().as_str()
    );
    assert_eq!(status["support"]["state"], "verified");
}

#[test]
fn a_version_this_build_did_not_verify_is_said_so_and_accounts_are_still_shown() {
    let fake = Fake::new("cst-unverified", CHATGPT, SIGNED_OUT);

    let status = read(
        Some(&fake.binary),
        &Policy::shipped(),
        &Support::shipped(),
        &environment(&[]),
    );

    assert_eq!(status["support"]["state"], "unverified");
    let reason = status["support"]["reason"].as_str().unwrap();
    assert!(reason.contains("has not been verified"), "{reason}");
    assert!(
        !reason.contains(&fake.binary.display().to_string()),
        "{reason}"
    );
    // Who is signed in is a separate question and is still answered.
    assert_eq!(account(&status, "official-login")["state"], "signed-in");
}

#[test]
fn a_feature_that_is_on_and_that_nobody_looked_at_is_said_by_name() {
    let fake = Fake::new("cst-feature", CHATGPT, SIGNED_OUT);
    fake.will_list("shell_tool  stable  true\nunified_exec  stable  true\n");

    let status = read(
        Some(&fake.binary),
        &Policy::shipped(),
        &verified(),
        &environment(&[]),
    );

    assert_eq!(status["support"]["state"], "unverified");
    let reason = status["support"]["reason"].as_str().unwrap();
    assert!(reason.contains("unified_exec"), "{reason}");
}

#[test]
fn a_client_that_would_not_list_its_features_is_unknown_and_not_unverified() {
    let fake = Fake::new("cst-no-features", CHATGPT, SIGNED_OUT);
    // `features` exits with an error: what is on is not known, so nothing is said of the version.
    common::write_program(
        &fake.binary,
        &format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'codex-cli {VERSION}'; exit 0; fi\n\
             if [ \"$1\" = \"features\" ]; then echo 'boom' >&2; exit 2; fi\n\
             echo '{CHATGPT}'\n"
        ),
    );

    let status = read(
        Some(&fake.binary),
        &Policy::shipped(),
        &verified(),
        &environment(&[]),
    );

    assert_eq!(status["support"]["state"], "unknown");
    assert!(status["support"]["reason"]
        .as_str()
        .unwrap()
        .contains("features"));
}

#[test]
fn a_variable_that_is_empty_is_not_a_key() {
    let fake = Fake::new("cst-empty-key", CHATGPT, CHATGPT);

    let status = read(
        Some(&fake.binary),
        &Policy::shipped(),
        &verified(),
        &environment(&[("CODEX_API_KEY", "")]),
    );

    // A generation would find no key to run on, so the screen does not say there is one.
    assert_eq!(account(&status, "env-api-key")["state"], "signed-out");
}

#[test]
fn each_source_is_asked_the_way_a_generation_runs_it() {
    let fake = Fake::new("cst-sources", CHATGPT, API_KEY);

    let status = read(
        Some(&fake.binary),
        &Policy::shipped(),
        &verified(),
        &environment(&[("OPENAI_API_KEY", "sk-openai-secret")]),
    );

    // The official client's login is asked with no home of its own, the application's with the
    // application's, and neither with the key that was only in the person's shell.
    let asked = fake.logins();
    assert_eq!(asked.len(), 2, "{asked:?}");
    assert!(
        asked.contains(&"home= openai= codex=".to_string()),
        "{asked:?}"
    );
    assert!(
        asked.contains(&format!("home={APP_HOME} openai= codex=")),
        "{asked:?}"
    );
    let official = account(&status, "official-login");
    assert_eq!(official["state"], "signed-in");
    assert_eq!(official["billing"], "subscription");
    assert_eq!(official["route"], "codex-cli-chat-gpt-login");
    assert_eq!(official["usable"], true);
    let stored = account(&status, "app-store");
    assert_eq!(stored["state"], "signed-in");
    assert_eq!(stored["billing"], "usage");
    assert_eq!(stored["route"], "codex-cli-api-key");
}

#[test]
fn nobody_signed_in_is_a_state_and_is_not_read_as_not_knowing() {
    let fake = Fake::new("cst-signed-out", SIGNED_OUT, SIGNED_OUT);

    let status = read(
        Some(&fake.binary),
        &Policy::shipped(),
        &verified(),
        &environment(&[]),
    );

    assert_eq!(account(&status, "official-login")["state"], "signed-out");
    assert_eq!(account(&status, "app-store")["state"], "signed-out");
}

#[test]
fn a_key_in_the_environment_is_a_source_that_needs_no_login_and_names_its_variable() {
    let fake = Fake::new("cst-env-key", SIGNED_OUT, SIGNED_OUT);

    let status = read(
        Some(&fake.binary),
        &Policy::shipped(),
        &verified(),
        &environment(&[("CODEX_API_KEY", "sk-codex-secret")]),
    );

    let key = account(&status, "env-api-key");
    assert_eq!(key["state"], "signed-in");
    assert_eq!(key["billing"], "usage");
    assert_eq!(key["environment"], "CODEX_API_KEY");
    assert_eq!(key["usable"], true);
    // Nobody was asked about it, and its value is nowhere in the answer.
    assert!(
        !status.to_string().contains("sk-codex-secret"),
        "the key reached the answer: {status}"
    );
    assert_eq!(status["key_variable"], "CODEX_API_KEY");
}

#[test]
fn a_key_that_the_environment_does_not_have_is_signed_out_for_that_source() {
    let fake = Fake::new("cst-env-none", CHATGPT, CHATGPT);

    let status = read(
        Some(&fake.binary),
        &Policy::shipped(),
        &verified(),
        // A key for another purpose is not the one a connection asks for.
        &environment(&[("OPENAI_API_KEY", "sk-openai-secret")]),
    );

    assert_eq!(account(&status, "env-api-key")["state"], "signed-out");
    assert!(!status.to_string().contains("sk-openai-secret"));
}

#[test]
fn a_login_the_build_does_not_use_is_shown_as_the_login_it_is() {
    let fake = Fake::new(
        "cst-unlisted",
        "Logged in using an access token",
        SIGNED_OUT,
    );

    let status = read(
        Some(&fake.binary),
        &Policy::shipped(),
        &verified(),
        &environment(&[]),
    );

    let official = account(&status, "official-login");
    assert_eq!(official["state"], "signed-in");
    assert_eq!(official["usable"], false);
    assert_eq!(official["decision"]["decision"], "refuse");
    assert_eq!(official["decision"]["reason"], "unconfirmed");
}

#[test]
fn a_route_a_release_switched_off_is_not_usable_and_says_so() {
    let fake = Fake::new("cst-off", CHATGPT, SIGNED_OUT);
    let policy = Policy::shipped().with_switched_off([Route::CodexCliChatGptLogin]);

    let status = read(Some(&fake.binary), &policy, &verified(), &environment(&[]));

    let official = account(&status, "official-login");
    assert_eq!(official["usable"], false);
    assert_eq!(official["decision"]["reason"], "switched-off");
}

#[test]
fn no_codex_is_said_and_where_to_say_where_it_is() {
    let status = read(None, &Policy::shipped(), &verified(), &environment(&[]));

    assert_eq!(status["client"]["state"], "missing");
    assert_eq!(status["support"]["state"], "unknown");
    for source in ["official-login", "app-store", "env-api-key"] {
        let said = account(&status, source);
        // A key in the environment can be named without the client; the others cannot be asked.
        if source != "env-api-key" {
            assert_eq!(said["state"], "unknown", "{source}");
            assert!(
                said["reason"].as_str().unwrap().contains("SCE_CODEX"),
                "{said}"
            );
        }
    }
}

#[test]
fn a_file_that_does_not_say_it_is_codex_is_not_asked_who_is_signed_in() {
    let dir = scratch("cst-impostor");
    let impostor = dir.join("codex");
    common::write_program(&impostor, "#!/bin/sh\necho 'hello 1.0'\n");

    let status = read(
        Some(&impostor),
        &Policy::shipped(),
        &verified(),
        &environment(&[]),
    );

    assert_eq!(status["client"]["state"], "unverified");
    assert_eq!(account(&status, "official-login")["state"], "unknown");
}

#[test]
fn the_first_codex_the_search_finds_is_the_one_asked_when_none_was_named() {
    let fake = Fake::new("cst-search", CHATGPT, SIGNED_OUT);
    let search = Search {
        path: vec![fake.binary.parent().unwrap().to_path_buf()],
        known: vec![],
        timeout: Duration::from_secs(15),
    };

    let status = serde_json::to_value(codex_status::read(
        None,
        &Policy::shipped(),
        &verified(),
        Path::new(APP_HOME),
        &environment(&[]),
        &search,
    ))
    .unwrap();

    assert_eq!(status["client"]["state"], "installed");
    assert_eq!(
        status["client"]["path"],
        fake.binary.display().to_string().as_str()
    );
}

#[test]
fn the_commands_that_sign_in_are_fixed_words_and_the_stored_login_names_its_home() {
    let fake = Fake::new("cst-guidance", SIGNED_OUT, SIGNED_OUT);

    let status = read(
        Some(&fake.binary),
        &Policy::shipped(),
        &verified(),
        &environment(&[]),
    );

    let guidance = status["sign_in"].as_array().unwrap();
    let find = |source: &str, billing: &str| {
        guidance
            .iter()
            .find(|g| g["source"] == source && g["billing"] == billing)
            .unwrap_or_else(|| panic!("no guidance for {source}/{billing}: {status}"))
    };
    assert_eq!(
        find("official-login", "subscription")["command"],
        "codex login"
    );
    assert_eq!(
        find("official-login", "usage")["command"],
        "codex login --with-api-key"
    );
    assert_eq!(find("official-login", "subscription")["home"], Value::Null);
    // A login made anywhere else is not the one a generation uses, so the folder is named.
    assert_eq!(find("app-store", "subscription")["command"], "codex login");
    assert_eq!(find("app-store", "subscription")["home"], APP_HOME);
}

#[test]
fn the_answer_carries_no_path_but_the_one_program_that_was_asked() {
    let fake = Fake::new("cst-no-paths", CHATGPT, CHATGPT);

    let status = read(
        Some(&fake.binary),
        &Policy::shipped(),
        &Support::shipped(),
        &environment(&[("HOME", "/home/person")]),
    );

    let said = status.to_string();
    assert!(!said.contains("/home/person"), "{said}");
}

mod command {
    use super::*;

    fn ask(entrance: Entrance, codex: Option<&Path>) -> Result<Value, (String, String)> {
        let works = WorkStore::at(scratch("cst-cmd-works"));
        let settings = ConnectionStore::at(scratch("cst-cmd-settings"));
        let policy = Policy::shipped();
        let context = Context::new(&works, &FakeRenderer, &policy, entrance)
            .with_connections(Some(&settings))
            .with_codex(codex);
        call_in(&context, "read_codex_status", json!({})).map_err(|e| (e.kind, e.message))
    }

    #[test]
    fn only_the_desktop_application_may_start_a_program_to_ask_it_something() {
        let fake = Fake::new("cst-cmd-entrances", CHATGPT, SIGNED_OUT);

        for entrance in [Entrance::Browser, Entrance::Tool] {
            let (kind, _) = ask(entrance, Some(&fake.binary)).unwrap_err();
            assert_eq!(kind, "not-allowed-here", "{entrance:?}");
        }
        // And nothing was started for the ones that were refused.
        assert!(fake.logins().is_empty());
    }

    #[test]
    fn the_desktop_application_is_answered_with_what_it_asked_about() {
        let fake = Fake::new("cst-cmd-desktop", CHATGPT, SIGNED_OUT);

        let said = ask(Entrance::Desktop, Some(&fake.binary)).expect("an answer");

        assert_eq!(said["codex"]["client"]["state"], "installed");
        assert_eq!(said["codex"]["client"]["version"], VERSION);
        // This build has verified no version of Codex.
        assert_eq!(said["codex"]["support"]["state"], "unverified");
    }

    #[test]
    fn arguments_it_does_not_take_are_refused() {
        let works = WorkStore::at(scratch("cst-cmd-args-works"));
        let policy = Policy::shipped();
        let context = Context::new(&works, &FakeRenderer, &policy, Entrance::Desktop);

        let refused = call_in(&context, "read_codex_status", json!({ "path": "/bin/sh" }));

        assert!(refused.is_err());
    }
}
