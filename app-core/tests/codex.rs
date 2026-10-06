// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Codex as a generator: how it is started, what it is given, what it is refused, and what
//! comes back.
//!
//! The client here is a script. Nothing of a person's account or of a model is used, and what is
//! held is what the application does: which arguments it starts the client with, what it puts in
//! its environment and what it keeps out, what it says on standard input, what it will not start
//! the client for (a version nobody verified, a feature nobody reviewed, a key the environment
//! does not have), and what it makes of the last message the client writes. That the arguments
//! keep a real Codex inside the tools it is allowed is not something a script can show: it is
//! what a person verifies against a real one before a version goes on the list.
//!
//! Unix only: the stand-in is a shell script.

#![cfg(unix)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use sce_app_core::claude_code::{AuthorServer, ALLOWED_TOOLS};
use sce_app_core::codex::{Codex, CodexConfig, AUTHOR_TOOLS};
use sce_app_core::codex_support::Support;
use sce_app_core::runner::{Cancel, GenerateError, Generator, Job};
use sce_app_core::{AuthSource, Revision, WorkId};
use serde_json::{json, Value};

const VERSION: &str = "0.159.0";
const APP_HOME: &str = "/app/data/codex-home";

/// What the stand-in lists as its features: some on that a list may say to switch off, one that
/// is off.
const FEATURES: &str = "\
shell_tool                               stable             true
unified_exec                             stable             true
fast_mode                                stable             true
code_mode                                under development  false
";

/// A stand-in for `codex`, and where it wrote down what it was given.
struct Fake {
    binary: PathBuf,
    record: PathBuf,
}

impl Fake {
    /// A script that answers `--version` and `features list`, and for `exec` records what it was
    /// given (its arguments, its folder, its environment, what it was told on standard input),
    /// writes `answer.json` to the file `-o` names, and then does `behaviour`.
    fn new(label: &str, behaviour: &str) -> Fake {
        let dir = common::scratch(label);
        let record = dir.join("record");
        fs::create_dir_all(&record).unwrap();
        fs::write(record.join("features.txt"), FEATURES).unwrap();
        let binary = dir.join("codex");
        let script = format!(
            "#!/bin/sh\n\
             R='{record}'\n\
             if [ \"$1\" = \"--version\" ]; then echo 'codex-cli {VERSION}'; exit 0; fi\n\
             if [ \"$1\" = \"features\" ]; then cat \"$R/features.txt\"; exit 0; fi\n\
             for a in \"$@\"; do printf '%s\\0' \"$a\"; done > \"$R/argv\"\n\
             pwd > \"$R/cwd\"\n\
             env > \"$R/env\"\n\
             cat > \"$R/stdin\"\n\
             out=''; prev=''\n\
             for a in \"$@\"; do if [ \"$prev\" = '-o' ]; then out=\"$a\"; fi; prev=\"$a\"; done\n\
             if [ -f \"$R/answer.json\" ] && [ -n \"$out\" ]; then cp \"$R/answer.json\" \"$out\"; fi\n\
             {behaviour}\n",
            record = record.display()
        );
        common::write_program(&binary, &script);
        Fake { binary, record }
    }

    fn answering(label: &str, answer: &Value) -> Fake {
        let fake = Fake::new(label, "exit 0");
        fs::write(fake.record.join("answer.json"), answer.to_string()).unwrap();
        fake
    }

    /// What `exec` was given, one argument at a time.
    fn argv(&self) -> Vec<String> {
        let bytes = fs::read(self.record.join("argv")).expect("exec was started");
        let bytes = bytes.strip_suffix(&[0]).unwrap_or(&bytes);
        bytes
            .split(|b| *b == 0)
            .map(|part| String::from_utf8(part.to_vec()).unwrap())
            .collect()
    }

    fn started(&self) -> bool {
        self.record.join("argv").exists()
    }

    fn env(&self) -> Vec<(String, String)> {
        fs::read_to_string(self.record.join("env"))
            .unwrap()
            .lines()
            .filter_map(|line| line.split_once('='))
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn env_of(&self, name: &str) -> Option<String> {
        self.env()
            .into_iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v)
    }
}

fn author() -> AuthorServer {
    AuthorServer {
        command: PathBuf::from("/opt/sce/bin/sce-author-mcp"),
        args: vec!["--stdio".to_string()],
        env: vec![
            ("SCE_WORKS_DIR".to_string(), "/works".to_string()),
            ("SCE_WORK".to_string(), "/opt/sce/bin/sce-work".to_string()),
        ],
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

fn answer(text: &str) -> Value {
    json!({
        "model": { "documents": [{ "name": "m.scxml", "text": text }] },
        "requirements": { "manifest_text": "{\"doc_id\":\"door\",\"rev\":\"1\"}\n" },
    })
}

/// A list that verifies `binary` as it is started with this configuration: the instructions are
/// the generator's own, so a change to what it is told or what it switches off needs another entry.
fn verified_for(fake: &Fake, auth: AuthSource, disabled: &[&str]) -> Support {
    let unverified = codex(fake, auth, support_with(&[], disabled));
    let instructions = unverified.instructions().expect("instructions are named");
    support_with(
        &[(std::env::consts::OS, VERSION, instructions.as_str())],
        disabled,
    )
}

fn support_with(verified: &[(&str, &str, &str)], disabled: &[&str]) -> Support {
    Support::from_json(
        &json!({
            "verified": verified.iter().map(|(os, version, instructions)| json!({
                "os": os, "version": version, "instructions": instructions
            })).collect::<Vec<_>>(),
            "disabled_features": disabled,
            "reviewed_features": ["fast_mode"],
        })
        .to_string(),
    )
    .unwrap()
}

fn codex(fake: &Fake, auth: AuthSource, support: Support) -> Codex {
    Codex::new(
        fake.binary.clone(),
        author(),
        CodexConfig {
            model: Some("gpt-test".to_string()),
            timeout: Duration::from_secs(20),
        },
        auth,
        PathBuf::from(APP_HOME),
        support,
    )
    .with_environment(vec![
        (
            "PATH".to_string(),
            std::env::var("PATH").unwrap_or_default(),
        ),
        ("HOME".to_string(), "/home/person".to_string()),
        ("OPENAI_API_KEY".to_string(), "sk-openai-secret".to_string()),
        ("CODEX_API_KEY".to_string(), "sk-codex-secret".to_string()),
        (
            "OPENAI_IDENTITY_TOKEN".to_string(),
            "identity-secret".to_string(),
        ),
    ])
}

fn verified(fake: &Fake) -> Codex {
    codex(
        fake,
        AuthSource::AppStore,
        verified_for(fake, AuthSource::AppStore, &["shell_tool", "unified_exec"]),
    )
}

fn generate(client: &Codex) -> Result<sce_app_core::runner::Draft, GenerateError> {
    client.generate(&job(), &Cancel::new())
}

fn after<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

#[test]
fn it_says_what_it_is_and_which_version() {
    let fake = Fake::new("codex-kind", "exit 0");
    let client = verified(&fake);

    assert_eq!(client.kind(), "codex");
    assert_eq!(client.version().as_deref(), Some(VERSION));
}

#[test]
fn a_program_that_will_not_say_its_version_has_none() {
    let client = Codex::new(
        PathBuf::from("/nowhere/codex"),
        author(),
        CodexConfig::default(),
        AuthSource::AppStore,
        PathBuf::from(APP_HOME),
        Support::shipped(),
    );

    assert_eq!(client.version(), None);
}

#[test]
fn a_codex_nobody_verified_is_not_started() {
    let fake = Fake::answering("codex-unverified", &answer("<scxml/>"));
    let client = codex(&fake, AuthSource::AppStore, Support::shipped());

    let refused = generate(&client).unwrap_err();

    let GenerateError::Failed(said) = refused else {
        panic!("expected a failure, got {refused:?}");
    };
    assert!(said.contains("has not been verified"), "{said}");
    assert!(said.contains("another connection"), "{said}");
    // Nothing was started for the run: it was refused before there was one.
    assert!(!fake.started());
}

#[test]
fn a_feature_that_is_on_and_that_nobody_switched_off_or_reviewed_is_not_run_beside() {
    let fake = Fake::answering("codex-feature", &answer("<scxml/>"));
    // Only the shell is switched off: `unified_exec` is on, and nobody said anything of it.
    let client = codex(
        &fake,
        AuthSource::AppStore,
        verified_for(&fake, AuthSource::AppStore, &["shell_tool"]),
    );

    let refused = generate(&client).unwrap_err();

    let GenerateError::Failed(said) = refused else {
        panic!("expected a failure, got {refused:?}");
    };
    assert!(said.contains("unified_exec"), "{said}");
    assert!(!fake.started());
}

#[test]
fn it_is_started_with_the_arguments_that_were_verified() {
    let fake = Fake::answering("codex-argv", &answer("<scxml/>"));
    let client = verified(&fake);

    generate(&client).unwrap();

    let args = fake.argv();
    assert_eq!(args[0], "exec");
    // One JSON document in one form, nothing kept, nothing read from the person's own settings,
    // no write to the disk, and no question put to a person who is not there.
    for flag in [
        "--json",
        "--ephemeral",
        "--ignore-user-config",
        "--skip-git-repo-check",
    ] {
        assert!(
            args.iter().any(|a| a == flag),
            "{flag} is missing: {args:?}"
        );
    }
    assert_eq!(after(&args, "-s"), Some("read-only"));
    assert!(
        args.windows(2)
            .any(|w| w[0] == "-c" && w[1] == "approval_policy=\"never\""),
        "{args:?}"
    );
    assert!(
        args.windows(2)
            .any(|w| w[0] == "-c" && w[1] == "web_search=\"disabled\""),
        "{args:?}"
    );
    // Where a login is kept is said, and said the way the sign-in check says it: the file the
    // person's settings are in is ignored by a run and read by the check, so both are told.
    assert!(
        args.windows(2)
            .any(|w| w[0] == "-c" && w[1] == sce_app_core::codex::auth_store_override()),
        "{args:?}"
    );
    assert_eq!(after(&args, "-m"), Some("gpt-test"));
    // The prompt is the last thing, and it is standard input.
    assert_eq!(args.last().map(String::as_str), Some("-"));
}

#[test]
fn the_features_that_are_switched_off_are_switched_off_on_every_run() {
    let fake = Fake::answering("codex-disable", &answer("<scxml/>"));
    let client = verified(&fake);

    generate(&client).unwrap();

    let args = fake.argv();
    let disabled: Vec<&str> = args
        .windows(2)
        .filter(|w| w[0] == "--disable")
        .map(|w| w[1].as_str())
        .collect();
    assert_eq!(disabled, vec!["shell_tool", "unified_exec"]);
}

#[test]
fn no_model_is_asked_for_when_the_connection_names_none() {
    let fake = Fake::answering("codex-no-model", &answer("<scxml/>"));
    let client = Codex::new(
        fake.binary.clone(),
        author(),
        CodexConfig {
            model: None,
            timeout: Duration::from_secs(20),
        },
        AuthSource::AppStore,
        PathBuf::from(APP_HOME),
        verified_for(&fake, AuthSource::AppStore, &["shell_tool", "unified_exec"]),
    )
    .with_environment(vec![(
        "PATH".to_string(),
        std::env::var("PATH").unwrap_or_default(),
    )]);

    generate(&client).unwrap();

    assert!(!fake.argv().iter().any(|a| a == "-m"), "{:?}", fake.argv());
}

#[test]
fn what_it_is_told_goes_in_on_standard_input_and_not_on_the_command_line() {
    let fake = Fake::answering("codex-stdin", &answer("<scxml/>"));
    let client = verified(&fake);

    generate(&client).unwrap();

    let told = fs::read_to_string(fake.record.join("stdin")).unwrap();
    assert!(told.contains("`door-lock`"), "{told}");
    assert!(told.contains("works_read"), "{told}");
    // Not the specification: the client reads it through the server.
    assert!(
        !told.contains("The lock opens when the code matches."),
        "{told}"
    );
    assert!(
        !fake.argv().iter().any(|a| a.contains("door-lock")),
        "{:?}",
        fake.argv()
    );
}

#[test]
fn only_the_authoring_server_is_reachable_and_only_by_the_tools_the_task_needs() {
    let fake = Fake::answering("codex-mcp", &answer("<scxml/>"));
    let client = verified(&fake);

    generate(&client).unwrap();

    let args = fake.argv();
    let overrides: Vec<&str> = args
        .windows(2)
        .filter(|w| w[0] == "-c")
        .map(|w| w[1].as_str())
        .collect();
    // Every server key names the one server, and the one server has these four keys.
    let servers: Vec<&&str> = overrides
        .iter()
        .filter(|o| o.starts_with("mcp_servers."))
        .collect();
    assert!(
        servers
            .iter()
            .all(|o| o.starts_with("mcp_servers.sce-author.")),
        "{servers:?}"
    );
    for key in ["command", "args", "env", "enabled_tools"] {
        assert!(
            servers
                .iter()
                .any(|o| o.starts_with(&format!("mcp_servers.sce-author.{key}="))),
            "{key} is missing: {servers:?}"
        );
    }
    let command = overrides
        .iter()
        .find(|o| o.starts_with("mcp_servers.sce-author.command="))
        .unwrap();
    assert_eq!(
        *command,
        "mcp_servers.sce-author.command=\"/opt/sce/bin/sce-author-mcp\""
    );
    let args_value = overrides
        .iter()
        .find(|o| o.starts_with("mcp_servers.sce-author.args="))
        .unwrap();
    assert_eq!(*args_value, "mcp_servers.sce-author.args=[\"--stdio\"]");
    let env = overrides
        .iter()
        .find(|o| o.starts_with("mcp_servers.sce-author.env="))
        .unwrap();
    assert!(env.contains("\"SCE_WORKS_DIR\"=\"/works\""), "{env}");
    // The tools are the nine the client of the other kind is allowed, by their own names.
    let tools = overrides
        .iter()
        .find(|o| o.starts_with("mcp_servers.sce-author.enabled_tools="))
        .unwrap();
    let listed = tools
        .trim_start_matches("mcp_servers.sce-author.enabled_tools=")
        .trim_matches(|c| c == '[' || c == ']');
    let names: Vec<&str> = listed
        .split(',')
        .map(|n| n.trim().trim_matches('"'))
        .collect();
    assert_eq!(names, AUTHOR_TOOLS);
}

#[test]
fn the_tools_it_may_use_are_the_ones_the_other_client_may() {
    // One list, two spellings: Claude Code names a tool by its server, Codex by its own name.
    assert_eq!(AUTHOR_TOOLS.len(), ALLOWED_TOOLS.len());
    for (bare, prefixed) in AUTHOR_TOOLS.iter().zip(ALLOWED_TOOLS.iter()) {
        assert_eq!(*prefixed, format!("mcp__sce-author__{bare}"));
    }
}

#[test]
fn the_run_is_in_a_folder_of_its_own_that_is_gone_when_it_is_done() {
    let fake = Fake::answering("codex-folder", &answer("<scxml/>"));
    let client = verified(&fake);

    generate(&client).unwrap();

    let cwd = fs::read_to_string(fake.record.join("cwd")).unwrap();
    let cwd = Path::new(cwd.trim());
    assert!(!cwd.as_os_str().is_empty());
    assert!(!cwd.exists(), "{} is still there", cwd.display());
    // And it is the folder the client was told to work in.
    let args = fake.argv();
    let told = after(&args, "-C").unwrap();
    assert_eq!(Path::new(told).file_name(), cwd.file_name());
}

#[test]
fn the_folder_is_gone_when_the_run_failed_too() {
    let fake = Fake::new("codex-folder-failed", "echo 'it broke' >&2; exit 3");
    let client = verified(&fake);

    let result = generate(&client);

    assert!(result.is_err());
    let cwd = fs::read_to_string(fake.record.join("cwd")).unwrap();
    assert!(!Path::new(cwd.trim()).exists());
}

#[test]
fn a_run_with_the_applications_stored_login_has_that_login_and_none_of_the_other_credentials() {
    let fake = Fake::answering("codex-env-store", &answer("<scxml/>"));
    let client = verified(&fake);

    generate(&client).unwrap();

    assert_eq!(fake.env_of("CODEX_HOME").as_deref(), Some(APP_HOME));
    for name in ["OPENAI_API_KEY", "CODEX_API_KEY", "OPENAI_IDENTITY_TOKEN"] {
        assert_eq!(fake.env_of(name), None, "{name} reached the client");
    }
    // What is not a credential is there.
    assert_eq!(fake.env_of("HOME").as_deref(), Some("/home/person"));
}

#[test]
fn a_run_with_a_key_from_the_environment_has_that_key_and_no_other_credential() {
    let fake = Fake::answering("codex-env-key", &answer("<scxml/>"));
    let client = codex(
        &fake,
        AuthSource::EnvApiKey,
        verified_for(
            &fake,
            AuthSource::EnvApiKey,
            &["shell_tool", "unified_exec"],
        ),
    );

    generate(&client).unwrap();

    assert_eq!(
        fake.env_of("CODEX_API_KEY").as_deref(),
        Some("sk-codex-secret")
    );
    assert_eq!(fake.env_of("OPENAI_API_KEY"), None);
    assert_eq!(fake.env_of("OPENAI_IDENTITY_TOKEN"), None);
    assert_eq!(fake.env_of("CODEX_HOME").as_deref(), Some(APP_HOME));
}

#[test]
fn a_connection_that_chose_a_key_the_environment_does_not_have_is_not_run_on_anything_else() {
    let fake = Fake::answering("codex-env-missing", &answer("<scxml/>"));
    let client = Codex::new(
        fake.binary.clone(),
        author(),
        CodexConfig::default(),
        AuthSource::EnvApiKey,
        PathBuf::from(APP_HOME),
        verified_for(
            &fake,
            AuthSource::EnvApiKey,
            &["shell_tool", "unified_exec"],
        ),
    )
    // A stored login is around, and a key for another purpose: neither is what was chosen.
    .with_environment(vec![
        (
            "PATH".to_string(),
            std::env::var("PATH").unwrap_or_default(),
        ),
        ("OPENAI_API_KEY".to_string(), "sk-openai-secret".to_string()),
    ]);

    let refused = generate(&client).unwrap_err();

    let GenerateError::Failed(said) = refused else {
        panic!("expected a failure, got {refused:?}");
    };
    assert!(said.contains("CODEX_API_KEY"), "{said}");
    assert!(!said.contains("sk-openai-secret"), "{said}");
    assert!(!fake.started());
}

#[test]
fn the_last_message_it_writes_is_the_draft() {
    let fake = Fake::answering("codex-draft", &answer("<scxml><!-- door --></scxml>"));
    let client = verified(&fake);

    let draft = generate(&client).unwrap();

    assert_eq!(draft.model.entry_text(), "<scxml><!-- door --></scxml>");
    assert!(draft.requirements.manifest.contains("door"));
}

#[test]
fn a_last_message_that_is_not_the_draft_is_unusable_and_says_what_is_wrong() {
    for (label, written) in [
        ("codex-not-json", "this is not JSON"),
        (
            "codex-no-model",
            r#"{"requirements": {"manifest_text": "{}"}}"#,
        ),
        (
            "codex-empty-documents",
            r#"{"model": {"documents": []}, "requirements": {"manifest_text": "{}"}}"#,
        ),
    ] {
        let fake = Fake::new(label, "exit 0");
        fs::write(fake.record.join("answer.json"), written).unwrap();
        let client = verified(&fake);

        let refused = generate(&client).unwrap_err();

        assert!(
            matches!(refused, GenerateError::Unusable(_)),
            "{label}: {refused:?}"
        );
    }
}

#[test]
fn a_client_that_wrote_no_last_message_has_not_answered() {
    let fake = Fake::new("codex-silent", "exit 0");
    let client = verified(&fake);

    let refused = generate(&client).unwrap_err();

    let GenerateError::Unusable(said) = refused else {
        panic!("expected an unusable reply, got {refused:?}");
    };
    assert!(said.contains("last message"), "{said}");
}

#[test]
fn a_client_that_stops_with_an_error_is_a_failure_that_shows_what_it_said() {
    let fake = Fake::new("codex-error", "echo 'not logged in' >&2; exit 3");
    let client = verified(&fake);

    let refused = generate(&client).unwrap_err();

    let GenerateError::Failed(said) = refused else {
        panic!("expected a failure, got {refused:?}");
    };
    assert!(said.contains("not logged in"), "{said}");
}

#[test]
fn a_client_told_to_stop_is_killed_and_the_run_ends_at_once() {
    let fake = Fake::new("codex-cancel", "touch \"$R/started\"; exec sleep 60");
    let client = verified(&fake);
    let cancel = Cancel::new();
    let stopper = {
        let cancel = cancel.clone();
        let started = fake.record.join("started");
        thread::spawn(move || {
            let limit = Instant::now() + Duration::from_secs(10);
            while !started.exists() && Instant::now() < limit {
                thread::sleep(Duration::from_millis(10));
            }
            cancel.cancel();
        })
    };

    let begun = Instant::now();
    let result = client.generate(&job(), &cancel);
    stopper.join().unwrap();

    assert!(
        matches!(result, Err(GenerateError::Cancelled)),
        "{result:?}"
    );
    assert!(
        begun.elapsed() < Duration::from_secs(15),
        "{:?}",
        begun.elapsed()
    );
}

#[test]
fn a_client_that_takes_longer_than_its_time_is_stopped_and_the_run_failed() {
    let fake = Fake::new("codex-slow", "exec sleep 60");
    let client = Codex::new(
        fake.binary.clone(),
        author(),
        CodexConfig {
            model: None,
            timeout: Duration::from_millis(400),
        },
        AuthSource::AppStore,
        PathBuf::from(APP_HOME),
        verified_for(&fake, AuthSource::AppStore, &["shell_tool", "unified_exec"]),
    )
    .with_environment(vec![(
        "PATH".to_string(),
        std::env::var("PATH").unwrap_or_default(),
    )]);

    let begun = Instant::now();
    let refused = generate(&client).unwrap_err();

    let GenerateError::Failed(said) = refused else {
        panic!("expected a failure, got {refused:?}");
    };
    // The limit is said as it was given, not rounded up to a minute.
    assert!(said.contains("longer than 400 milliseconds"), "{said}");
    assert!(begun.elapsed() < Duration::from_secs(15));
}

#[test]
fn the_instructions_are_named_by_what_the_client_is_told_and_what_is_switched_off() {
    let fake = Fake::new("codex-instructions", "exit 0");
    let one = codex(
        &fake,
        AuthSource::AppStore,
        support_with(&[], &["shell_tool"]),
    );
    let again = codex(
        &fake,
        AuthSource::AppStore,
        support_with(&[], &["shell_tool"]),
    );
    let other = codex(
        &fake,
        AuthSource::AppStore,
        support_with(&[], &["shell_tool", "unified_exec"]),
    );

    let name = one.instructions().unwrap();

    assert!(name.starts_with("codex/"), "{name}");
    assert_eq!(again.instructions(), one.instructions());
    // Another list of what is off is another verification.
    assert_ne!(other.instructions(), one.instructions());
}
