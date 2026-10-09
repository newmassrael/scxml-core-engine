// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The real authoring server, started the way each client starts it, held against what a
//! specification could make the client ask of it: another work, a file outside the folder a run
//! works in, a tool that saves.
//!
//! No model and no account. A model that has read a hostile specification is replaced by a
//! script that makes the calls the specification asks for, so that what is held is the server and
//! the way a client starts it, and not whether a model happens to comply. What a real model does
//! with the same text is what the other live tests ask of it.
//!
//! Ignored by default, because it starts the authoring server, which needs `python3` with the
//! modules `sce_author/needs.py` lists and the product's generator built
//! (`cargo build -p sce-build --features cli --bin sce-codegen`):
//!
//! ```text
//! cargo test -p sce-app-core --features cli --test client_scope_live -- --ignored --nocapture
//! ```

#![cfg(unix)]

mod common;

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use sce_app_core::claude_code::{AuthorServer, ClaudeCode, ClaudeCodeConfig};
use sce_app_core::codex::AUTHOR_TOOLS;
use sce_app_core::http_client::Endpoint;
use sce_app_core::local::{Local, LocalConfig};
use sce_app_core::mcp_client::McpClient;
use sce_app_core::runner::{Cancel, Generator, Job};
use sce_app_core::{Revision, SystemClock, WorkId, WorkStore};
use serde_json::{json, Value};

use common::model_server::{calls, chat_server, draft, says};
use common::{scratch, write_program};

const DOOR: &str = "DOOR-SPEC: the door opens when the card matches.\n";

/// A work a run is for, a work it is not, and a file outside every work, each with a sentence that
/// shows up in an answer if the server read it.
struct Setting {
    store: WorkStore,
    works: PathBuf,
    codegen: PathBuf,
    assigned: WorkId,
    other: WorkId,
    /// What the other work says.
    secret: String,
    /// What the file outside says: the name of a parameter a diagnostic quotes.
    marker: String,
    template: PathBuf,
}

fn setting(label: &str) -> Setting {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let codegen = std::env::var_os("SCE_CODEGEN")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("target/debug/sce-codegen"));
    assert!(codegen.is_file(), "build sce-codegen first");
    let root = scratch(label);
    let works = root.join("works");
    let store = WorkStore::with_clock(&works, SystemClock);
    let secret = format!("PRIVATE-NOTES-{}", std::process::id());
    let marker = format!("SCOPE_MARKER_{}", std::process::id());
    let assigned = store.create_work("Door lock").unwrap().id;
    store.save_source(&assigned, DOOR, None).unwrap();
    let other = store.create_work("Private notes").unwrap().id;
    store
        .save_source(&other, &format!("{secret}\n"), None)
        .unwrap();
    let outside = root.join("outside");
    fs::create_dir_all(&outside).unwrap();
    let template = outside.join("secret.sce-template.xml");
    fs::write(
        &template,
        format!(
            "<sce:template xmlns=\"http://www.w3.org/2005/07/scxml\" \
             xmlns:sce=\"http://sce.dev/ext\" name=\"secret\">\
             <sce:param name=\"{marker}\" required=\"true\"/><state id=\"a\"/></sce:template>"
        ),
    )
    .unwrap();
    Setting {
        store,
        works,
        codegen,
        assigned,
        other,
        secret,
        marker,
        template,
    }
}

impl Setting {
    /// The authoring server as the host knows it: not told which work a run is for.
    fn server(&self) -> AuthorServer {
        let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        AuthorServer {
            command: PathBuf::from("python3"),
            args: vec![
                "-c".to_string(),
                "from sce_author import mcp; mcp.main()".to_string(),
            ],
            env: vec![
                ("SCE_WORK".into(), env!("CARGO_BIN_EXE_sce-work").into()),
                ("SCE_WORKS_DIR".into(), self.works.display().to_string()),
                ("SCE_CODEGEN".into(), self.codegen.display().to_string()),
                (
                    "PYTHONPATH".into(),
                    repo.join("tools/authoring").display().to_string(),
                ),
            ],
        }
    }

    fn job(&self) -> Job {
        Job {
            work: self.assigned.clone(),
            title: "Door lock".to_string(),
            request: "req-scope".to_string(),
            attempt: 1,
            source: DOOR.to_string(),
            source_revision: Revision::of(DOOR.as_bytes()),
            answers: Default::default(),
            previous: None,
            refusal: None,
            fresh_ids: false,
        }
    }

    /// The arguments of the call a hostile specification asks for to read a file outside: a document
    /// that names a template by its path.
    fn names_a_file_outside(&self) -> Value {
        json!({
            "document_name": "main.scxml",
            "document_text": format!(
                "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" \
                 xmlns:sce=\"http://sce.dev/ext\" initial=\"a\"><sce:use template=\"{}\"/></scxml>",
                self.template.display()
            ),
        })
    }

    fn reads(&self, work: &WorkId) -> Value {
        json!({"work": work.as_str()})
    }
}

fn within() -> Instant {
    Instant::now() + Duration::from_secs(90)
}

/// What a tool said, as one string, and whether it said it failed.
fn call(client: &mut McpClient, name: &str, arguments: &Value) -> (bool, String) {
    match client.call(name, arguments, within(), &Cancel::new()) {
        Ok(called) => (called.is_error, called.text),
        Err(e) => (true, e.to_string()),
    }
}

/// The server the Claude Code client is given for a run, as the client writes it down for the
/// program it starts (`mcp.json`).
fn server_claude_is_given(setting: &Setting) -> AuthorServer {
    let folder = scratch("claude-config");
    let record = folder.join("mcp.json");
    let answer = {
        let structured = json!({
            "model": {"documents": [{"name": "model.scxml", "text": "<scxml/>"}]},
            "requirements": {"manifest_text": "{\"doc_id\":\"door\",\"rev\":\"1\"}\n"},
        });
        json!({"type": "result", "subtype": "success", "is_error": false,
               "result": structured.to_string(), "structured_output": structured})
    };
    fs::write(folder.join("answer.json"), answer.to_string()).unwrap();
    let program = folder.join("claude");
    write_program(
        &program,
        &format!(
            "#!/bin/sh\n\
             if [ \"$1\" = \"--version\" ]; then echo \"2.1.289 (Claude Code)\"; exit 0; fi\n\
             cat > /dev/null\n\
             cp mcp.json '{record}'\n\
             cat '{answer}'\n",
            record = record.display(),
            answer = folder.join("answer.json").display()
        ),
    );
    ClaudeCode::new(program, setting.server(), ClaudeCodeConfig::default())
        .generate(&setting.job(), &Cancel::new())
        .expect("the stand-in client answers");
    let config: Value = serde_json::from_str(&fs::read_to_string(record).unwrap()).unwrap();
    let server = &config["mcpServers"]["sce-author"];
    AuthorServer {
        command: PathBuf::from(server["command"].as_str().unwrap()),
        args: server["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a.as_str().unwrap().to_string())
            .collect(),
        env: server["env"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
            .collect(),
    }
}

fn start(server: &AuthorServer) -> McpClient {
    McpClient::start(server, &scratch("server-folder"), within(), &Cancel::new())
        .expect("the authoring server starts")
}

#[test]
#[ignore = "starts the real authoring server: python3 and a built generator, no model"]
fn the_server_claude_code_is_given_refuses_what_a_specification_could_ask_of_it() {
    let setting = setting("claude-scope");
    let given = server_claude_is_given(&setting);

    // Told which work the run is for, it offers the nine tools of a run and no others.
    let mut scoped = start(&given);
    let offered: std::collections::BTreeSet<String> = scoped
        .tools(within(), &Cancel::new())
        .unwrap()
        .into_iter()
        .map(|tool| tool.name)
        .collect();
    let allowed: std::collections::BTreeSet<String> =
        AUTHOR_TOOLS.iter().map(|t| t.to_string()).collect();
    assert_eq!(offered, allowed);

    // The work it is for is read.
    let (failed, text) = call(&mut scoped, "works_read", &setting.reads(&setting.assigned));
    assert!(!failed && text.contains("DOOR-SPEC"), "{failed} {text}");
    // Another work of the same folder is not, and a tool that saves is not run.
    let (failed, text) = call(&mut scoped, "works_read", &setting.reads(&setting.other));
    assert!(failed && !text.contains(&setting.secret), "{failed} {text}");
    let (failed, text) = call(
        &mut scoped,
        "works_save_model",
        &json!({"work": setting.assigned.as_str(), "text": "<scxml/>"}),
    );
    assert!(failed, "a tool that saves was run: {text}");
    assert!(setting
        .store
        .read_model(&setting.assigned, None)
        .unwrap()
        .is_none());
    // A document that names a file is refused before the file is opened.
    let (failed, text) = call(
        &mut scoped,
        "validate_scxml",
        &setting.names_a_file_outside(),
    );
    assert!(failed && !text.contains(&setting.marker), "{failed} {text}");
    drop(scoped);

    // The same attacks on the server as the host knows it, told nothing, get what they ask for:
    // this is how the check is shown to tell the two apart.
    let mut unscoped = start(&setting.server());
    let (_, text) = call(&mut unscoped, "works_read", &setting.reads(&setting.other));
    assert!(
        text.contains(&setting.secret),
        "the unscoped server did not read the other work: {text}"
    );
    let (_, text) = call(
        &mut unscoped,
        "validate_scxml",
        &setting.names_a_file_outside(),
    );
    assert!(
        text.contains(&setting.marker),
        "the unscoped server did not read the file outside: {text}"
    );
}

#[test]
#[ignore = "starts the real authoring server: python3 and a built generator, no model"]
fn the_server_a_local_model_is_given_refuses_what_a_specification_could_ask_of_it() {
    let setting = setting("local-scope");
    let other = setting.reads(&setting.other).to_string();
    let outside = setting.names_a_file_outside().to_string();
    let own = setting.reads(&setting.assigned).to_string();
    // A model that did what the specification said: read its own work, then another, then name a
    // file outside, and then answer.
    let model = chat_server(vec![
        calls(&[
            (Some("c0"), "works_read", &own),
            (Some("c1"), "works_read", &other),
            (Some("c2"), "validate_scxml", &outside),
        ]),
        says(&draft("<scxml/>")),
    ]);
    let local = Local::new(
        Endpoint::parse(&model.address).unwrap(),
        setting.server(),
        LocalConfig::for_model("scripted"),
    );

    local
        .generate(&setting.job(), &Cancel::new())
        .expect("the run ends with the draft");

    let told: Vec<String> = model
        .messages(1)
        .into_iter()
        .filter(|m| m["role"] == "tool")
        .map(|m| m["content"].as_str().unwrap_or("").to_string())
        .collect();
    assert_eq!(told.len(), 3, "{told:?}");
    assert!(told[0].contains("DOOR-SPEC"), "{}", told[0]);
    for answer in &told[1..] {
        assert!(
            !answer.contains(&setting.secret) && !answer.contains(&setting.marker),
            "the model was told what the specification asked for: {answer}"
        );
    }
}
