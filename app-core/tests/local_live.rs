// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The whole chain once, for real, with a model server: the owner asks for a model, a runner takes
//! the request, a model on a real server writes it through the real authoring server and the real
//! product, and the core accepts it and publishes it.
//!
//! Ignored by default, because it runs a model: it takes minutes of the server's time, and it
//! needs a server (Ollama, llama.cpp's server, vLLM, LM Studio: anything that speaks Chat
//! Completions with tools), `python3`, and the product's generator built
//! (`cargo build -p sce-build --features cli --bin sce-codegen`). The other tests hold every side
//! of this pipe against a stand-in; what only this one holds is that a model, and not a script,
//! calls the tools with arguments the authoring server accepts, and writes a draft in the form the
//! application takes.
//!
//! The server is named by `SCE_LOCAL_URL` (`http://127.0.0.1:11434/v1`) and the model by
//! `SCE_LOCAL_MODEL`; both are required, and a key the server wants by `SCE_LOCAL_KEY`.
//!
//! ```text
//! SCE_LOCAL_URL=http://127.0.0.1:11434/v1 SCE_LOCAL_MODEL=qwen3-coder:30b \
//!   cargo test -p sce-app-core --features cli --test local_live -- --ignored --nocapture
//! ```

#![cfg(unix)]

mod common;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use sce_app_core::claude_code::AuthorServer;
use sce_app_core::host::{start_with, HostSettings};
use sce_app_core::http_client::Endpoint;
use sce_app_core::local::{list_models, Local, LocalConfig, Step};
use sce_app_core::requests::{Inputs, State};
use sce_app_core::runner::{Generator, Outcome, Runner, RunnerConfig};
use sce_app_core::{
    call_in, ConnectionStore, Context, Entrance, Policy, Product, Registration, SceCodegen,
    SystemClock, WorkStore,
};
use serde_json::{json, Value};

const SPECIFICATION: &str = "\
A door controller for a storage room.

The door is closed until somebody presents a card. A card that is on the list of cards opens \
the door; a card that is not on the list does nothing, and the controller counts three \
misses in a row, after which it ignores every card for one minute.

An open door closes by itself five seconds after it opened. Pressing the close button on the \
inside closes an open door at once.
";

/// The start of `text`, on one line.
fn clipped(text: &str, limit: usize) -> String {
    let one_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() <= limit {
        one_line
    } else {
        format!("{}...", one_line.chars().take(limit).collect::<String>())
    }
}

fn required(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} names the server and is required"))
}

#[test]
#[ignore = "runs a model on a real server: minutes of its time"]
fn a_model_on_a_real_server_writes_a_model_the_core_accepts_and_publishes() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let codegen = std::env::var_os("SCE_CODEGEN")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("target/debug/sce-codegen"));
    assert!(
        codegen.is_file(),
        "{} is not built: cargo build -p sce-build --features cli --bin sce-codegen",
        codegen.display()
    );
    let address = required("SCE_LOCAL_URL");
    let model = required("SCE_LOCAL_MODEL");
    let key = std::env::var("SCE_LOCAL_KEY").ok();
    let endpoint = Endpoint::parse(&address).expect("an address the application can reach");

    // What a registration screen would do first: ask the server which models it has.
    let listed = list_models(&endpoint, key.as_deref()).expect("the server lists its models");
    println!("the server lists: {listed:?}");
    assert!(
        listed.contains(&model),
        "the server does not list {model}: {listed:?}"
    );

    let works = common::scratch("local-live").join("works");
    let store = Arc::new(WorkStore::with_clock(&works, SystemClock));
    let work = store.create_work("Storage room door").unwrap().id;
    let source = store.save_source(&work, SPECIFICATION, None).unwrap();
    let head = match source {
        sce_app_core::Saved::Saved { revision, .. }
        | sce_app_core::Saved::Unchanged { revision } => revision,
    };
    let request = store
        .register_request(
            &work,
            Registration {
                key: "press-1",
                origin: "gui",
                expect: Inputs {
                    source: head,
                    answers: None,
                },
                supersede: false,
            },
        )
        .unwrap()
        .request
        .id;

    let author = AuthorServer {
        command: PathBuf::from("python3"),
        args: vec!["-m".to_string(), "sce_author.mcp".to_string()],
        env: vec![
            (
                "SCE_WORK".to_string(),
                env!("CARGO_BIN_EXE_sce-work").to_string(),
            ),
            ("SCE_WORKS_DIR".to_string(), works.display().to_string()),
            ("SCE_CODEGEN".to_string(), codegen.display().to_string()),
            (
                "PYTHONPATH".to_string(),
                repo.join("tools/authoring").display().to_string(),
            ),
        ],
    };
    let config = LocalConfig {
        timeout: Duration::from_secs(25 * 60),
        ..LocalConfig::for_model(model)
    };
    let began = Instant::now();
    let mut generator = Local::new(endpoint, author, config).with_trace(move |step| {
        let at = began.elapsed().as_secs();
        match step {
            Step::Asked { turn, messages } => {
                println!("[{at:>4}s] asks (turn {turn}, {messages} messages)")
            }
            Step::Said { words, calls, .. } => {
                for (name, arguments) in calls {
                    println!("[{at:>4}s]   calls {name} {}", clipped(arguments, 200));
                }
                if !words.trim().is_empty() {
                    // What the model said in full: a message that is not a draft is why a run fails.
                    println!(
                        "[{at:>4}s]   says ({} chars):\n{words}\n[end of what it said]",
                        words.len()
                    );
                }
            }
            Step::Tool {
                name,
                failed,
                words,
            } => {
                println!(
                    "[{at:>4}s]   {name} gave {} chars{}",
                    words.chars().count(),
                    if *failed { " (an error)" } else { "" }
                );
                // What a tool that failed said is why the model's next call differs, or does not.
                if *failed {
                    println!("[{at:>4}s]     {}", clipped(words, 600));
                }
            }
            Step::NotTheDraft { why } => println!("[{at:>4}s]   not the draft: {why}"),
        }
    });
    if let Some(key) = key {
        generator = generator.with_bearer(key);
    }
    println!("instructions: {:?}", generator.instructions());

    let mut runner_config = RunnerConfig::named("live");
    runner_config.repairs = 1;
    let runner = Runner::new(
        Arc::clone(&store),
        Arc::new(SceCodegen::at(&codegen)),
        Arc::new(generator),
        runner_config,
    );

    let outcome = runner.run_once().unwrap();
    println!("outcome after {:?}: {outcome:?}", began.elapsed());

    let Outcome::Completed { bundle, .. } = outcome else {
        let seen = store.read_request(&work, &request).unwrap();
        panic!(
            "the model did not write a model the core accepts: {outcome:?}; the request is {:?}",
            seen.request
        );
    };
    let published = store.read_bundle(&work, None).unwrap().expect("a bundle");
    assert_eq!(published.revision, bundle);
    let model = store.read_model(&work, None).unwrap().expect("a model");
    println!("--- model ---\n{}\n--- end ---", model.text);
    let list = store
        .read_requirements(&work, None)
        .unwrap()
        .expect("a requirement list");
    println!("--- requirements ---\n{}\n--- end ---", list.text);
    assert!(model.text.contains("<scxml") || model.text.contains("documents"));
    assert_eq!(published.bundle.checks[0].verdict, "accepted");
}

/// What the screen does, by the commands it sends, and then what the application's own host does
/// with the request: the server is asked what it is (`read_server_status`), kept as the default
/// connection, a model is asked for with that connection, and a host that has no AI client at all
/// writes it with the model on the real server. Everything between the screen and the model is the
/// real thing (the commands, the settings folder, the host, the directory, the adapter, the
/// authoring server, the product); what is not here is the window.
#[test]
#[ignore = "runs a model on a real server: minutes of its time"]
fn a_server_registered_through_the_commands_is_written_for_by_a_host_with_no_client() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let codegen = std::env::var_os("SCE_CODEGEN")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("target/debug/sce-codegen"));
    assert!(codegen.is_file(), "{} is not built", codegen.display());
    let address = required("SCE_LOCAL_URL");
    let model = required("SCE_LOCAL_MODEL");

    let folder = common::scratch("local-live-host");
    let store = Arc::new(WorkStore::with_clock(folder.join("works"), SystemClock));
    let settings_store = ConnectionStore::at(folder.join("settings"));
    let product: Arc<dyn Product> = Arc::new(SceCodegen::at(&codegen));
    let policy = Policy::shipped();
    let ask = |name: &str, args: Value| {
        let context = Context::new(&*store, &*product, &policy, Entrance::Desktop)
            .with_connections(Some(&settings_store));
        call_in(&context, name, args)
    };

    // The person types the address and presses check.
    let probed = ask("read_server_status", json!({ "server_url": address })).expect("an answer");
    println!("read_server_status: {probed}");
    assert_eq!(probed["server"]["state"], "listed", "{probed}");
    assert!(
        probed["server"]["models"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m == &json!(model)),
        "the server does not list {model}: {probed}"
    );

    // And saves it, as the default.
    let saved = ask(
        "save_connection",
        json!({ "connection": {
            "id": "server", "adapter": "local", "display_name": "live test server",
            "model": model, "auth": "none", "server_url": address,
        } }),
    )
    .expect("saved");
    let revision = saved["revision"].clone();
    ask(
        "set_default_connection",
        json!({ "id": "server", "expect": null }),
    )
    .expect("made the default");

    // Asks for a model with it.
    let work = store.create_work("Storage room door").unwrap().id;
    let head = match store.save_source(&work, SPECIFICATION, None).unwrap() {
        sce_app_core::Saved::Saved { revision, .. }
        | sce_app_core::Saved::Unchanged { revision } => revision,
    };
    ask(
        "request_generation",
        json!({
            "id": work.as_str(), "key": "press-1", "origin": "gui",
            "expect": { "source": head, "answers": null }, "supersede": false,
            "connection": { "id": "server", "revision": revision },
        }),
    )
    .expect("a request");
    let request = store.list_requests(&work).unwrap()[0].request.id.clone();

    // A host that has no Claude Code and no Codex, only the server that was just registered.
    let mut shell = HostSettings::from_lookup("desktop", |_| None);
    shell.claude = Some(PathBuf::from("/nowhere/claude"));
    shell.codex = Some(PathBuf::from("/nowhere/codex"));
    shell.author = Some(repo.join("scripts/sce_author_mcp.sh"));
    shell.work = Some(PathBuf::from(env!("CARGO_BIN_EXE_sce-work")));
    shell.codegen = Some(codegen.clone());
    let host = start_with(
        Arc::clone(&store),
        Arc::clone(&product),
        shell,
        Some((settings_store.clone(), Policy::shipped())),
    );
    println!("host: not hosted = {:?}", host.not_hosted());
    assert_eq!(host.not_hosted(), None, "the host did not start");

    let began = Instant::now();
    let limit = began + Duration::from_secs(25 * 60);
    let mut last = None;
    let state = loop {
        let seen = store.read_request(&work, &request).unwrap();
        if last != Some(seen.request.state) {
            println!(
                "[{:>4}s] request is {:?}",
                began.elapsed().as_secs(),
                seen.request.state
            );
            last = Some(seen.request.state);
        }
        if matches!(
            seen.request.state,
            State::Completed | State::Failed | State::Cancelled
        ) {
            break seen;
        }
        assert!(
            Instant::now() < limit,
            "the request did not finish in time: {:?}",
            seen.request
        );
        std::thread::sleep(Duration::from_secs(2));
    };
    drop(host);
    println!(
        "outcome after {:?}: {:?}",
        began.elapsed(),
        state.request.outcome
    );
    assert_eq!(state.request.state, State::Completed, "{:?}", state.request);

    let published = store.read_bundle(&work, None).unwrap().expect("a bundle");
    let model_text = store.read_model(&work, None).unwrap().expect("a model");
    println!("--- model ---\n{}\n--- end ---", model_text.text);
    assert_eq!(published.bundle.checks[0].verdict, "accepted");
}
