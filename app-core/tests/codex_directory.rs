// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Running a request made for a connection to Codex: when it can be run, and what is said when
//! it cannot.
//!
//! A request pinned to a Codex connection waits, with a reason, until three things are true: the
//! Codex this computer has is one a person verified, the credential the connection chose is
//! there (a key in the environment, or a login that Codex reports), and that way of signing in is
//! one the build uses. What is said of a request that waits is kept in the works folder, which is
//! shared, so it names no path and nothing Codex printed. The client here is a script: it
//! answers `--version`, `features list` and `login status`, and records what it was asked and in
//! what environment.
//!
//! Unix only: the stand-in is a shell script.

#![cfg(unix)]

mod common;

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use sce_app_core::claude_code::AuthorServer;
use sce_app_core::codex::{observed_from_login_status, CodexLaunch};
use sce_app_core::codex_support::Support;
use sce_app_core::directory::Connections;
use sce_app_core::requests::Pin;
use sce_app_core::runner::{Cancel, Directory, GenerateError, Generator, Job, Unrunnable};
use sce_app_core::{
    AdapterKind, AuthSource, Connection, ConnectionId, ConnectionStore, Limits, Observed, Policy,
    Revision, Route, WorkId,
};

const VERSION: &str = "0.159.0";
const APP_HOME: &str = "/app/data/codex-home";

/// A stand-in for `codex` that answers `--version`, `features list` and `login status`.
struct Fake {
    binary: PathBuf,
    record: PathBuf,
}

impl Fake {
    fn new(label: &str, login: &str) -> Fake {
        let dir = common::scratch(label);
        let record = dir.join("record");
        fs::create_dir_all(&record).unwrap();
        fs::write(record.join("login.txt"), login).unwrap();
        let binary = dir.join("codex");
        let script = format!(
            "#!/bin/sh\n\
             R='{record}'\n\
             if [ \"$1\" = \"--version\" ]; then echo 'codex-cli {VERSION}'; exit 0; fi\n\
             if [ \"$1\" = \"features\" ]; then printf 'shell_tool  stable  true\\n'; exit 0; fi\n\
             if [ \"$1\" = \"login\" ] && [ \"$2\" = \"status\" ]; then\n\
               env > \"$R/login-env\"\n\
               cat \"$R/login.txt\"\n\
               exit 0\n\
             fi\n\
             for a in \"$@\"; do printf '%s\\0' \"$a\"; done > \"$R/argv\"\n\
             cat > /dev/null\n\
             out=''; prev=''\n\
             for a in \"$@\"; do if [ \"$prev\" = '-o' ]; then out=\"$a\"; fi; prev=\"$a\"; done\n\
             if [ -f \"$R/answer.json\" ] && [ -n \"$out\" ]; then cp \"$R/answer.json\" \"$out\"; fi\n\
             if [ -f \"$R/slow\" ]; then exec sleep 60; fi\n\
             exit 0\n",
            record = record.display()
        );
        common::write_program(&binary, &script);
        Fake { binary, record }
    }

    fn login_env(&self, name: &str) -> Option<String> {
        fs::read_to_string(self.record.join("login-env"))
            .ok()?
            .lines()
            .filter_map(|line| line.split_once('='))
            .find(|(k, _)| *k == name)
            .map(|(_, v)| v.to_string())
    }

    fn asked_for_login(&self) -> bool {
        self.record.join("login-env").exists()
    }

    /// What the client was started with for a run, one argument at a time.
    fn argv(&self) -> Vec<String> {
        let bytes = fs::read(self.record.join("argv")).expect("a run was started");
        let bytes = bytes.strip_suffix(&[0]).unwrap_or(&bytes);
        bytes
            .split(|b| *b == 0)
            .map(|part| String::from_utf8(part.to_vec()).unwrap())
            .collect()
    }

    /// The client answers a run with `text` as its last message.
    fn will_answer(&self, text: &str) {
        let answer = serde_json::json!({
            "model": { "documents": [{ "name": "m.scxml", "text": text }] },
            "requirements": { "manifest_text": "{}\n" },
        });
        fs::write(self.record.join("answer.json"), answer.to_string()).unwrap();
    }

    /// The client takes longer than any limit a test gives it.
    fn will_be_slow(&self) {
        fs::write(self.record.join("slow"), "").unwrap();
    }
}

fn job() -> Job {
    Job {
        work: WorkId::parse("door-lock").unwrap(),
        title: "Door lock".to_string(),
        request: "req-7".to_string(),
        attempt: 1,
        source: "The lock opens when the code matches.".to_string(),
        source_revision: Revision::of(b"The lock opens when the code matches."),
        answers: Default::default(),
        previous: None,
        refusal: None,
    }
}

fn author() -> AuthorServer {
    AuthorServer {
        command: PathBuf::from("/nowhere/sce-author-mcp"),
        args: vec![],
        env: vec![],
    }
}

fn codex_connection(auth: AuthSource) -> Connection {
    Connection {
        id: ConnectionId::parse("gpt").unwrap(),
        adapter: AdapterKind::Codex,
        display_name: None,
        executable: None,
        model: Some("gpt-test".to_string()),
        auth,
        server_url: None,
        limits: Limits {
            turns: Some(12),
            seconds: Some(900),
        },
    }
}

/// A list that verifies this stand-in, as the generator a directory makes would be started.
fn verified(fake: &Fake) -> Support {
    // The instructions are the generator's own: ask one for them, then name them.
    let probe = sce_app_core::codex::Codex::new(
        fake.binary.clone(),
        author(),
        sce_app_core::codex::CodexConfig::default(),
        AuthSource::AppStore,
        PathBuf::from(APP_HOME),
        Support::from_json(r#"{"verified":[],"disabled_features":["shell_tool"]}"#).unwrap(),
    );
    let instructions = probe.instructions().unwrap();
    Support::from_json(
        &serde_json::json!({
            "verified": [{
                "os": std::env::consts::OS, "version": VERSION, "instructions": instructions
            }],
            "disabled_features": ["shell_tool"],
        })
        .to_string(),
    )
    .unwrap()
}

struct Rig {
    settings: ConnectionStore,
    fake: Fake,
    /// The application's own home for the client, not there yet: the application makes it.
    home: PathBuf,
}

impl Rig {
    fn new(label: &str, login: &str) -> Rig {
        Rig {
            settings: ConnectionStore::at(common::scratch(&format!("{label}-settings"))),
            fake: Fake::new(label, login),
            home: common::scratch(&format!("{label}-home")).join("codex-home"),
        }
    }

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

    fn launch(&self, support: Support, environment: &[(&str, &str)]) -> CodexLaunch {
        CodexLaunch {
            binary: self.fake.binary.clone(),
            author: author(),
            app_home: self.home.clone(),
            support,
            environment: environment
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    fn directory(&self, launch: Option<CodexLaunch>) -> Connections {
        let directory = Connections::new(self.settings.clone(), Policy::shipped(), None);
        match launch {
            Some(launch) => directory.with_codex(launch),
            None => directory,
        }
    }
}

fn refused(result: Result<impl Sized, Unrunnable>) -> String {
    match result {
        Ok(_) => panic!("it should not be runnable"),
        Err(why) => why.reason,
    }
}

const PATH: (&str, &str) = ("PATH", "/usr/bin:/bin");

#[test]
fn what_codex_says_of_who_is_signed_in_is_read_as_a_kind_of_credential() {
    for (said, succeeded, observed) in [
        (
            "Logged in using ChatGPT",
            true,
            Some(Observed::Subscription),
        ),
        (
            "Logged in using an API key - sk-...abcd",
            true,
            Some(Observed::ApiKey),
        ),
        // It fails when nobody is signed in, and says so.
        ("Not logged in", false, None),
        ("Not logged in\n", false, None),
        // A way of being signed in that the table does not name is one, and is not guessed at.
        (
            "Logged in using an access token",
            true,
            Some(Observed::Other),
        ),
        // What is printed before it is not what it says: a path has the words of a login's name.
        (
            "WARNING: no aliases under \"/home/chatgpt/api key\"\nLogged in using an access token",
            true,
            Some(Observed::Other),
        ),
    ] {
        assert_eq!(
            observed_from_login_status(said, succeeded),
            Ok(observed),
            "{said}"
        );
    }
}

#[test]
fn what_codex_says_that_is_not_a_login_is_not_read_as_one() {
    for (said, succeeded) in [
        ("something else entirely", true),
        (
            "Error loading configuration: CODEX_HOME points to \"/x\", but that path does not exist",
            false,
        ),
        // The words of a login's name are in a path as well.
        (
            "Error loading configuration: CODEX_HOME points to \"/home/chatgpt/api key\", but that path does not exist",
            false,
        ),
        ("", false),
        // It said somebody is signed in and failed: nothing is known.
        ("Logged in using ChatGPT", false),
    ] {
        assert!(
            observed_from_login_status(said, succeeded).is_err(),
            "{said:?} was read as a login"
        );
    }
}

#[test]
fn a_codex_that_could_not_say_who_is_signed_in_waits_and_says_to_ask_again() {
    let rig = Rig::new("cdir-login-error", "unused");
    // The client fails the way it does for a home that is not there.
    common::write_program(
        &rig.fake.binary,
        &format!(
            "#!/bin/sh\n\
             if [ \"$1\" = \"--version\" ]; then echo 'codex-cli {VERSION}'; exit 0; fi\n\
             if [ \"$1\" = \"features\" ]; then printf 'shell_tool  stable  true\\n'; exit 0; fi\n\
             echo 'Error loading configuration: CODEX_HOME points to \"/x\", but that path does not exist' >&2\n\
             exit 1\n"
        ),
    );
    let pin = rig.pin(&codex_connection(AuthSource::OfficialLogin));
    let launch = rig.launch(verified(&rig.fake), &[PATH]);

    let said = refused(rig.directory(Some(launch)).generator_for(&pin));

    assert!(
        said.contains("could not be asked who is signed in"),
        "{said}"
    );
}

#[test]
fn a_verified_codex_with_a_login_the_build_uses_is_run_with_the_model_and_time_the_request_pinned()
{
    let rig = Rig::new("cdir-ok", "Logged in using ChatGPT");
    let pin = rig.pin(&codex_connection(AuthSource::OfficialLogin));
    let launch = rig.launch(verified(&rig.fake), &[PATH]);

    let generator = rig.directory(Some(launch)).generator_for(&pin).unwrap();

    assert_eq!(generator.kind(), "codex");
}

#[test]
fn the_model_a_request_pinned_is_the_one_the_run_asks_for() {
    let rig = Rig::new("cdir-model", "Logged in using ChatGPT");
    rig.fake.will_answer("<scxml/>");
    let mut connection = codex_connection(AuthSource::OfficialLogin);
    connection.model = Some("gpt-pinned".to_string());
    let pin = rig.pin(&connection);
    // The connection is changed after the request was made: the request is about what it was.
    let mut changed = connection.clone();
    changed.model = Some("gpt-later".to_string());
    rig.settings.save(&changed, Some(&pin.revision)).unwrap();
    let launch = rig.launch(verified(&rig.fake), &[PATH]);
    let generator = rig.directory(Some(launch)).generator_for(&pin).unwrap();

    generator.generate(&job(), &Cancel::new()).unwrap();

    let args = rig.fake.argv();
    let at = args
        .iter()
        .position(|a| a == "-m")
        .expect("a model was asked for");
    assert_eq!(args[at + 1], "gpt-pinned");
}

#[test]
fn the_time_a_request_pinned_is_how_long_the_run_may_take() {
    let rig = Rig::new("cdir-time", "Logged in using ChatGPT");
    rig.fake.will_be_slow();
    let mut connection = codex_connection(AuthSource::OfficialLogin);
    connection.limits.seconds = Some(1);
    let pin = rig.pin(&connection);
    let launch = rig.launch(verified(&rig.fake), &[PATH]);
    let generator = rig.directory(Some(launch)).generator_for(&pin).unwrap();

    let begun = std::time::Instant::now();
    let refused = generator.generate(&job(), &Cancel::new()).unwrap_err();

    assert!(matches!(refused, GenerateError::Failed(_)), "{refused:?}");
    assert!(
        begun.elapsed() < Duration::from_secs(20),
        "{:?}",
        begun.elapsed()
    );
}

#[test]
fn what_was_found_of_codex_is_not_asked_again_for_every_look() {
    let rig = Rig::new("cdir-cache", "Logged in using ChatGPT");
    let pin = rig.pin(&codex_connection(AuthSource::OfficialLogin));
    let launch = rig.launch(verified(&rig.fake), &[PATH]);
    let directory = rig.directory(Some(launch));

    directory.generator_for(&pin).unwrap();
    fs::remove_file(rig.fake.record.join("login-env")).unwrap();
    directory.generator_for(&pin).unwrap();

    // The second look was made of what the first found: nobody was asked again.
    assert!(!rig.fake.asked_for_login());
}

#[test]
fn a_codex_nobody_verified_waits_and_says_so_without_naming_a_path() {
    let rig = Rig::new("cdir-unverified", "Logged in using ChatGPT");
    let pin = rig.pin(&codex_connection(AuthSource::OfficialLogin));
    // A list that verifies nothing, spelled out: the shipped one verifies a version, and this
    // stand-in says that version.
    let nothing_verified =
        Support::from_json(r#"{"verified":[],"disabled_features":["shell_tool"]}"#).unwrap();
    let launch = rig.launch(nothing_verified, &[PATH]);

    let said = refused(rig.directory(Some(launch)).generator_for(&pin));

    assert!(said.contains("has not been verified"), "{said}");
    assert!(said.contains("another connection"), "{said}");
    assert!(!said.contains(rig.fake.binary.to_str().unwrap()), "{said}");
}

#[test]
fn nobody_signed_in_to_codex_waits_and_says_to_sign_in() {
    let rig = Rig::new("cdir-signed-out", "Not logged in");
    let pin = rig.pin(&codex_connection(AuthSource::OfficialLogin));
    let launch = rig.launch(verified(&rig.fake), &[PATH]);

    let said = refused(rig.directory(Some(launch)).generator_for(&pin));

    assert!(said.contains("signed in to Codex"), "{said}");
}

#[test]
fn a_login_the_build_does_not_use_waits_and_says_which_way_it_is() {
    let rig = Rig::new("cdir-token", "Logged in using an access token");
    let pin = rig.pin(&codex_connection(AuthSource::OfficialLogin));
    let launch = rig.launch(verified(&rig.fake), &[PATH]);

    let said = refused(rig.directory(Some(launch)).generator_for(&pin));

    assert!(said.contains("not one this build uses"), "{said}");
}

#[test]
fn the_stored_login_of_the_application_is_asked_of_in_the_home_it_is_kept_in() {
    let rig = Rig::new("cdir-home", "Logged in using an API key - sk-...abcd");
    let pin = rig.pin(&codex_connection(AuthSource::AppStore));
    let launch = rig.launch(verified(&rig.fake), &[PATH, ("OPENAI_API_KEY", "sk-other")]);

    rig.directory(Some(launch)).generator_for(&pin).unwrap();

    assert_eq!(
        rig.fake.login_env("CODEX_HOME").as_deref(),
        rig.home.to_str()
    );
    // The home was made for it: the client does not start in one that is not there.
    assert!(rig.home.is_dir());
    // And a key that was only in the shell did not decide who is signed in.
    assert_eq!(rig.fake.login_env("OPENAI_API_KEY"), None);
}

#[test]
fn the_login_of_the_official_client_is_asked_of_where_the_client_keeps_it() {
    let rig = Rig::new("cdir-official", "Logged in using ChatGPT");
    let pin = rig.pin(&codex_connection(AuthSource::OfficialLogin));
    let launch = rig.launch(verified(&rig.fake), &[PATH]);

    rig.directory(Some(launch)).generator_for(&pin).unwrap();

    assert_eq!(rig.fake.login_env("CODEX_HOME"), None);
}

#[test]
fn a_key_from_the_environment_needs_no_login_and_is_run() {
    // The key is the credential, whatever a stored login says: nothing is asked of the login.
    let rig = Rig::new("cdir-key", "Not logged in");
    let pin = rig.pin(&codex_connection(AuthSource::EnvApiKey));
    let launch = rig.launch(
        verified(&rig.fake),
        &[PATH, ("CODEX_API_KEY", "sk-codex-secret")],
    );

    let generator = rig.directory(Some(launch)).generator_for(&pin).unwrap();

    assert_eq!(generator.kind(), "codex");
    assert!(!rig.fake.asked_for_login());
}

#[test]
fn a_connection_that_chose_a_key_the_environment_lacks_waits_and_says_what_to_set() {
    let rig = Rig::new("cdir-key-missing", "Logged in using ChatGPT");
    let pin = rig.pin(&codex_connection(AuthSource::EnvApiKey));
    let launch = rig.launch(verified(&rig.fake), &[PATH, ("OPENAI_API_KEY", "sk-other")]);

    let said = refused(rig.directory(Some(launch)).generator_for(&pin));

    assert!(said.contains("CODEX_API_KEY"), "{said}");
    assert!(!said.contains("sk-other"), "{said}");
}

#[test]
fn a_computer_with_no_codex_waits_and_says_so() {
    let rig = Rig::new("cdir-none", "Logged in using ChatGPT");
    let pin = rig.pin(&codex_connection(AuthSource::OfficialLogin));

    let said = refused(rig.directory(None).generator_for(&pin));

    assert!(said.contains("Codex was not found"), "{said}");
}

#[test]
fn a_route_a_release_switched_off_waits_and_says_so() {
    let rig = Rig::new("cdir-off", "Logged in using ChatGPT");
    let pin = rig.pin(&codex_connection(AuthSource::OfficialLogin));
    let launch = rig.launch(verified(&rig.fake), &[PATH]);
    let directory = Connections::new(
        rig.settings.clone(),
        Policy::shipped().with_switched_off([Route::CodexCliChatGptLogin]),
        None,
    )
    .with_codex(launch);

    let said = refused(directory.generator_for(&pin));

    assert!(said.contains("switched off"), "{said}");
}
