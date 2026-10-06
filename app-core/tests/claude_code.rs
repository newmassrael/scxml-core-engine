// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The generator that runs Claude Code: what it is given, what it is allowed, what it answers.
//!
//! The `claude` here is a script, because what is held is this side of the pipe: the command
//! line the client is started with (the tools it may use are the whole of its power), what it
//! is told, what is read back from what it says, and what happens when it is stopped, slow,
//! missing or wrong. That the real client writes good models is not a thing a test can hold;
//! that it cannot write to a work, run a command or read the owner's other files is.
//!
//! Unix only: the stand-in is a shell script.

#![cfg(unix)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use sce_app_core::claude_code::{AuthorServer, ClaudeCode, ClaudeCodeConfig, ALLOWED_TOOLS};
use sce_app_core::runner::{Cancel, GenerateError, Generator, Job};
use sce_app_core::{ModelFiles, Revision, WorkId};
use serde_json::{json, Value};

/// A stand-in for `claude`, and where it wrote down what it was given.
struct Fake {
    binary: PathBuf,
    record: PathBuf,
}

impl Fake {
    /// A script that records its command line, its folder, what it was told on standard
    /// input and the author server's configuration, and then does `behaviour`.
    fn new(label: &str, behaviour: &str) -> Fake {
        let dir = common::scratch(label);
        let record = dir.join("record");
        fs::create_dir_all(&record).unwrap();
        let binary = dir.join("claude");
        let script = format!(
            "#!/bin/sh\n\
             if [ \"$1\" = \"--version\" ]; then echo \"2.1.289 (Claude Code)\"; exit 0; fi\n\
             R='{record}'\n\
             for a in \"$@\"; do printf '%s\\0' \"$a\"; done > \"$R/argv\"\n\
             pwd > \"$R/cwd\"\n\
             cat > \"$R/stdin\"\n\
             cp mcp.json \"$R/mcp.json\"\n\
             {behaviour}\n",
            record = record.display()
        );
        common::write_program(&binary, &script);
        Fake { binary, record }
    }

    /// A stand-in that answers `answer`, as the client's JSON result.
    fn answering(label: &str, answer: &Value) -> Fake {
        let fake = Fake::new(label, "cat \"$R/answer.json\"");
        fs::write(fake.record.join("answer.json"), answer.to_string()).unwrap();
        fake
    }

    /// Every argument, in order, an empty one included: each was written with a NUL after it.
    fn argv(&self) -> Vec<String> {
        let bytes = fs::read(self.record.join("argv")).unwrap();
        let bytes = bytes.strip_suffix(&[0]).unwrap_or(&bytes);
        bytes
            .split(|b| *b == 0)
            .map(|part| String::from_utf8(part.to_vec()).unwrap())
            .collect()
    }

    /// The value that follows `flag` on the command line.
    fn value_of(&self, flag: &str) -> Option<String> {
        let argv = self.argv();
        let at = argv.iter().position(|a| a == flag)?;
        argv.get(at + 1).cloned()
    }

    fn read(&self, name: &str) -> String {
        fs::read_to_string(self.record.join(name)).unwrap()
    }
}

fn author() -> AuthorServer {
    AuthorServer {
        command: PathBuf::from("/opt/sce/bin/sce-author-mcp"),
        args: vec!["--stdio".to_string()],
        env: vec![
            ("SCE_WORK".to_string(), "/opt/sce/bin/sce-work".to_string()),
            ("SCE_WORKS_DIR".to_string(), "/home/owner/works".to_string()),
        ],
    }
}

fn client(fake: &Fake) -> ClaudeCode {
    ClaudeCode::new(fake.binary.clone(), author(), ClaudeCodeConfig::default())
}

fn job() -> Job {
    Job {
        work: WorkId::parse("door-lock-3f2a1b9c").unwrap(),
        title: "Door lock".to_string(),
        request: "req-0123456789ab".to_string(),
        attempt: 1,
        source: "The door opens when the card matches.".to_string(),
        source_revision: Revision::of(b"text"),
        answers: Default::default(),
        previous: None,
        refusal: None,
    }
}

fn single(model: &str) -> Value {
    json!({
        "model": {"documents": [{"name": "model.scxml", "text": model}]},
        "requirements": {"manifest_text": "{\"doc_id\":\"door\",\"rev\":\"1\"}\n"},
    })
}

/// What `claude -p --output-format json` prints when it succeeded.
fn answered(structured: Value) -> Value {
    json!({
        "type": "result", "subtype": "success", "is_error": false,
        "result": structured.to_string(), "structured_output": structured,
        "num_turns": 7, "total_cost_usd": 0.4,
    })
}

fn generate(client: &ClaudeCode, job: &Job) -> Result<sce_app_core::runner::Draft, GenerateError> {
    client.generate(job, &Cancel::new())
}

#[test]
fn the_client_is_started_with_only_the_tools_the_task_needs() {
    let fake = Fake::answering("claude-args", &answered(single("<scxml/>")));

    generate(&client(&fake), &job()).unwrap();

    let argv = fake.argv();
    for flag in [
        "-p",
        "--strict-mcp-config",
        "--no-session-persistence",
        "--disable-slash-commands",
    ] {
        assert!(argv.iter().any(|a| a == flag), "{flag} in {argv:?}");
    }
    assert_eq!(fake.value_of("--output-format").as_deref(), Some("json"));
    // No built-in tool at all: no shell, no files, no web. What the client can do is what
    // the author server offers, and of that only what the task needs.
    assert_eq!(fake.value_of("--tools").as_deref(), Some(""));
    assert_eq!(fake.value_of("--setting-sources").as_deref(), Some(""));
    assert_eq!(
        fake.value_of("--permission-mode").as_deref(),
        Some("dontAsk")
    );
    let allowed = fake.value_of("--allowedTools").expect("a list of tools");
    assert_eq!(allowed, ALLOWED_TOOLS.join(","));
    for tool in allowed.split(',') {
        assert!(tool.starts_with("mcp__sce-author__"), "{tool}");
    }
    // Reading a work is allowed, and writing to one is not: the application saves what the
    // client answers, so a specification that tells the client to save elsewhere cannot.
    assert!(allowed.contains("works_read"));
    for withheld in [
        "works_save_model",
        "works_save_requirements",
        "works_begin_generation",
        "works_finish_generation",
        "works_fail_generation",
        "scxml_accept",
        "works_list",
    ] {
        assert!(
            !allowed.contains(withheld),
            "{withheld} must not be allowed"
        );
    }
    assert!(fake.value_of("--model").is_none(), "its own default model");
    assert_eq!(fake.value_of("--max-turns").as_deref(), Some("60"));
}

#[test]
fn what_the_client_must_answer_is_a_schema_and_the_application_is_what_saves() {
    let fake = Fake::answering("claude-schema", &answered(single("<scxml/>")));

    generate(&client(&fake), &job()).unwrap();

    let schema: Value = serde_json::from_str(&fake.value_of("--json-schema").unwrap()).unwrap();
    assert_eq!(schema["required"], json!(["model", "requirements"]));
    assert_eq!(
        schema["properties"]["requirements"]["required"],
        json!(["manifest_text"])
    );
    assert_eq!(
        schema["properties"]["model"]["properties"]["documents"]["minItems"],
        json!(1)
    );
}

#[test]
fn a_configured_model_and_budget_reach_the_command_line() {
    let fake = Fake::answering("claude-config", &answered(single("<scxml/>")));
    let config = ClaudeCodeConfig {
        model: Some("opus".to_string()),
        max_turns: 12,
        max_budget_usd: Some(2.5),
        ..ClaudeCodeConfig::default()
    };

    generate(
        &ClaudeCode::new(fake.binary.clone(), author(), config),
        &job(),
    )
    .unwrap();

    assert_eq!(fake.value_of("--model").as_deref(), Some("opus"));
    assert_eq!(fake.value_of("--max-turns").as_deref(), Some("12"));
    assert_eq!(fake.value_of("--max-budget-usd").as_deref(), Some("2.5"));
}

#[test]
fn the_author_server_is_the_one_the_host_names_and_the_only_one() {
    let fake = Fake::answering("claude-mcp", &answered(single("<scxml/>")));

    generate(&client(&fake), &job()).unwrap();

    let config: Value = serde_json::from_str(&fake.read("mcp.json")).unwrap();
    assert_eq!(
        config,
        json!({"mcpServers": {"sce-author": {
            "command": "/opt/sce/bin/sce-author-mcp",
            "args": ["--stdio"],
            "env": {"SCE_WORK": "/opt/sce/bin/sce-work", "SCE_WORKS_DIR": "/home/owner/works"},
        }}})
    );
    let given = fake.value_of("--mcp-config").unwrap();
    assert!(given.ends_with("mcp.json"), "{given}");
}

#[test]
fn it_is_told_which_work_to_write_for_on_standard_input_and_not_on_the_command_line() {
    let fake = Fake::answering("claude-prompt", &answered(single("<scxml/>")));

    generate(&client(&fake), &job()).unwrap();

    let told = fake.read("stdin");
    assert!(told.contains("door-lock-3f2a1b9c"), "{told}");
    assert!(told.contains("Door lock"), "{told}");
    assert!(told.contains("works_read"), "{told}");
    assert!(!told.contains("refused"), "nothing was refused yet: {told}");
    // A prompt can be long and is the owner's: it is not in a process listing.
    assert!(
        fake.argv().iter().all(|a| !a.contains("The door opens")),
        "the specification is read by the client with works_read, not handed on a command line"
    );
}

#[test]
fn a_draft_the_core_refused_is_put_to_the_client_in_the_products_words() {
    let fake = Fake::answering("claude-refusal", &answered(single("<scxml/>")));
    let mut job = job();
    job.refusal = Some(
        "the candidate was refused by model\n- validation/invalid-reference (line 3): no target"
            .to_string(),
    );

    generate(&client(&fake), &job).unwrap();

    let told = fake.read("stdin");
    assert!(
        told.contains("validation/invalid-reference (line 3)"),
        "{told}"
    );
}

#[test]
fn the_client_runs_in_a_folder_of_its_own_that_is_gone_when_it_is_done() {
    let fake = Fake::answering("claude-cwd", &answered(single("<scxml/>")));

    generate(&client(&fake), &job()).unwrap();

    let ran_in = PathBuf::from(fake.read("cwd").trim());
    assert!(!ran_in.exists(), "{} was left behind", ran_in.display());
    assert_ne!(ran_in, std::env::current_dir().unwrap());
    // It is not a folder of the owner's: nothing the client could read there is theirs.
    assert!(!ran_in.starts_with(env!("CARGO_MANIFEST_DIR")));
}

#[test]
fn the_folder_is_gone_when_the_run_failed_too() {
    let fake = Fake::new("claude-cwd-failed", "echo boom >&2; exit 3");

    let failed = generate(&client(&fake), &job());

    assert!(matches!(failed, Err(GenerateError::Failed(_))));
    let ran_in = PathBuf::from(fake.read("cwd").trim());
    assert!(!ran_in.exists(), "{} was left behind", ran_in.display());
}

#[test]
fn one_document_is_the_model_and_the_manifest_is_the_list() {
    let answer = json!({
        "model": {"documents": [{"name": "door.scxml", "text": "<scxml><!-- door --></scxml>"}]},
        "requirements": {
            "manifest_text": "{\"doc_id\":\"door\",\"rev\":\"1\"}\n",
            "sidecar_text": "{\"doc_id\":\"door\",\"rev\":\"1\",\"text\":{}}\n",
        },
    });
    let fake = Fake::answering("claude-single", &answered(answer));

    let draft = generate(&client(&fake), &job()).unwrap();

    assert_eq!(
        draft.model,
        ModelFiles::single("<scxml><!-- door --></scxml>")
    );
    assert_eq!(
        draft.requirements.manifest,
        "{\"doc_id\":\"door\",\"rev\":\"1\"}\n"
    );
    assert_eq!(
        draft.requirements.sidecar.as_deref(),
        Some("{\"doc_id\":\"door\",\"rev\":\"1\",\"text\":{}}\n")
    );
}

#[test]
fn several_documents_that_name_each_other_are_a_set_with_the_entry_it_names() {
    let answer = json!({
        "model": {
            "documents": [
                {"name": "schema.scxml", "text": "<scxml><!-- schema --></scxml>"},
                {"name": "gate.scxml", "text": "<scxml><!-- gate --></scxml>"},
            ],
            "entry": "gate.scxml",
        },
        "requirements": {"manifest_text": "{\"doc_id\":\"gate\"}\n"},
    });
    let fake = Fake::answering("claude-set", &answered(answer));

    let draft = generate(&client(&fake), &job()).unwrap();

    assert_eq!(draft.model.entry_text(), "<scxml><!-- gate --></scxml>");
    assert_eq!(draft.model.documents().len(), 2);
    assert_eq!(draft.model.entry_name(), Some("gate.scxml"));
}

#[test]
fn an_answer_read_from_the_result_text_when_the_client_gave_no_structured_output() {
    let structured = single("<scxml><!-- from text --></scxml>");
    let answer = json!({
        "type": "result", "subtype": "success", "is_error": false,
        "result": structured.to_string(),
    });
    let fake = Fake::answering("claude-text", &answer);

    let draft = generate(&client(&fake), &job()).unwrap();

    assert!(draft.model.entry_text().contains("from text"));
}

#[test]
fn a_draft_that_is_not_one_is_unusable_and_says_what_is_wrong() {
    for (label, answer, wanted) in [
        (
            "claude-no-model",
            answered(json!({"requirements": {"manifest_text": "{}"}})),
            "model",
        ),
        (
            "claude-empty-documents",
            answered(json!({"model": {"documents": []}, "requirements": {"manifest_text": "{}"}})),
            "documents",
        ),
        (
            "claude-bad-manifest",
            answered(json!({
                "model": {"documents": [{"name": "m.scxml", "text": "<scxml/>"}]},
                "requirements": {"manifest_text": "not json"},
            })),
            "manifest",
        ),
        (
            "claude-bad-name",
            answered(json!({
                "model": {"documents": [
                    {"name": "../escape.scxml", "text": "<scxml/>"},
                    {"name": "b.scxml", "text": "<scxml/>"},
                ]},
                "requirements": {"manifest_text": "{}"},
            })),
            "name",
        ),
    ] {
        let fake = Fake::answering(label, &answer);

        let result = generate(&client(&fake), &job());

        match result {
            Err(GenerateError::Unusable(wrong)) => {
                assert!(wrong.contains(wanted), "{label}: {wrong}");
            }
            other => panic!("{label}: expected an unusable draft, got {other:?}"),
        }
    }
}

#[test]
fn a_client_that_says_it_failed_is_a_failure_in_its_own_words() {
    let answer = json!({
        "type": "result", "subtype": "error_max_turns", "is_error": true,
        "result": "Reached the turn limit",
    });
    let fake = Fake::answering("claude-error", &answer);

    let failed = generate(&client(&fake), &job());

    let Err(GenerateError::Failed(reason)) = failed else {
        panic!("expected a failure, got {failed:?}");
    };
    assert!(reason.contains("error_max_turns"), "{reason}");
    assert!(reason.contains("Reached the turn limit"), "{reason}");
}

#[test]
fn something_that_is_not_json_is_a_failure_that_shows_what_it_was() {
    let fake = Fake::new("claude-garbage", "echo 'Please run /login'");

    let failed = generate(&client(&fake), &job());

    let Err(GenerateError::Failed(reason)) = failed else {
        panic!("expected a failure, got {failed:?}");
    };
    assert!(reason.contains("not JSON"), "{reason}");
    assert!(reason.contains("Please run /login"), "{reason}");
}

#[test]
fn a_client_that_stops_with_a_status_says_what_it_said() {
    let fake = Fake::new("claude-status", "echo 'Invalid API key' >&2; exit 3");

    let failed = generate(&client(&fake), &job());

    let Err(GenerateError::Failed(reason)) = failed else {
        panic!("expected a failure, got {failed:?}");
    };
    assert!(reason.contains("Invalid API key"), "{reason}");
    assert!(reason.contains('3'), "{reason}");
}

#[test]
fn a_client_that_is_not_there_is_a_failure_that_says_so() {
    let gone = ClaudeCode::new(
        PathBuf::from("/nowhere/claude"),
        author(),
        ClaudeCodeConfig::default(),
    );

    assert_eq!(gone.version(), None);
    let failed = generate(&gone, &job());

    let Err(GenerateError::Failed(reason)) = failed else {
        panic!("expected a failure, got {failed:?}");
    };
    assert!(reason.contains("could not be started"), "{reason}");
    assert!(reason.contains("/nowhere/claude"), "{reason}");
}

#[test]
fn the_instructions_are_named_by_what_the_client_is_told_and_allowed() {
    let fake = Fake::new("claude-instructions", "true");
    let named = client(&fake)
        .instructions()
        .expect("a version of its instructions");

    assert!(
        regex_like(&named),
        "`claude-code/` and twelve hex digits, got {named}"
    );
    // The same wording is the same version, wherever and however often it is asked.
    assert_eq!(named, client(&fake).instructions().unwrap());
    // The model and the budget are not instructions: they are how a run is bounded.
    let bounded = ClaudeCode::new(
        fake.binary.clone(),
        author(),
        ClaudeCodeConfig {
            model: Some("opus".to_string()),
            max_budget_usd: Some(1.0),
            ..ClaudeCodeConfig::default()
        },
    );
    assert_eq!(bounded.instructions().unwrap(), named);
}

fn regex_like(name: &str) -> bool {
    name.strip_prefix("claude-code/")
        .is_some_and(|digest| digest.len() == 12 && digest.bytes().all(|b| b.is_ascii_hexdigit()))
}

#[test]
fn the_version_is_what_the_client_says_it_is() {
    let fake = Fake::new("claude-version", "true");

    assert_eq!(client(&fake).version().as_deref(), Some("2.1.289"));
    assert_eq!(client(&fake).kind(), "claude-code");
}

#[test]
fn a_client_told_to_stop_is_killed_and_the_run_ends_at_once() {
    let fake = Fake::new("claude-cancel", "echo $$ > \"$R/pid\"; exec sleep 60");
    let cancel = Cancel::new();
    let client = Arc::new(client(&fake));

    let started = Instant::now();
    let outcome = thread::scope(|scope| {
        let running = scope.spawn(|| client.generate(&job(), &cancel));
        wait_for(&fake.record.join("pid"));
        cancel.cancel();
        running.join().unwrap()
    });

    assert_eq!(outcome.unwrap_err(), GenerateError::Cancelled);
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "it waited out the client"
    );
    #[cfg(target_os = "linux")]
    {
        let pid = fake.read("pid").trim().to_string();
        assert!(
            !Path::new(&format!("/proc/{pid}")).exists(),
            "the client {pid} is still running"
        );
    }
}

#[test]
fn a_client_that_takes_longer_than_its_time_is_stopped_and_the_run_failed() {
    let fake = Fake::new("claude-slow", "exec sleep 60");
    let config = ClaudeCodeConfig {
        timeout: Duration::from_millis(300),
        ..ClaudeCodeConfig::default()
    };
    let client = ClaudeCode::new(fake.binary.clone(), author(), config);

    let started = Instant::now();
    let failed = generate(&client, &job());

    let Err(GenerateError::Failed(reason)) = failed else {
        panic!("expected a failure, got {failed:?}");
    };
    assert!(reason.contains("longer than"), "{reason}");
    assert!(started.elapsed() < Duration::from_secs(20));
}

#[test]
fn a_great_deal_of_output_does_not_stop_the_client_from_finishing() {
    // More than a pipe holds: a reader that waits for the exit before it reads would hold the
    // client at its write and never see it end.
    let fake = Fake::new(
        "claude-flood",
        "i=0; while [ $i -lt 4000 ]; do echo \"0123456789012345678901234567890123456789012345678901234567890123\" >&2; i=$((i+1)); done; cat \"$R/answer.json\"",
    );
    fs::write(
        fake.record.join("answer.json"),
        answered(single("<scxml/>")).to_string(),
    )
    .unwrap();

    let draft = generate(&client(&fake), &job());

    assert!(draft.is_ok(), "{draft:?}");
}

/// Wait for a file the stand-in writes when it has started.
fn wait_for(path: &Path) {
    let limit = Instant::now() + Duration::from_secs(10);
    while !path.exists() {
        assert!(Instant::now() < limit, "{} never appeared", path.display());
        thread::sleep(Duration::from_millis(5));
    }
    // The script writes it, then replaces itself: a moment for the exec.
    thread::sleep(Duration::from_millis(50));
}
