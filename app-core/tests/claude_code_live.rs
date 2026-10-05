// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The whole chain once, for real: the owner asks for a model, a runner takes the request, the
//! real Claude Code writes it through the real authoring server and the real product, and the
//! core accepts it and publishes it.
//!
//! Ignored by default, because it runs a model: it takes minutes and costs money, and it needs
//! `claude` on the search path (signed in), `python3`, and the product's generator built
//! (`cargo build -p sce-build --features cli --bin sce-codegen`). The other tests hold every
//! side of this pipe against a stand-in; what only this one holds is that the flags, the tool
//! names and the answer's form mean to the real client what this side says they mean.
//!
//! ```text
//! cargo test -p sce-app-core --features cli --test claude_code_live -- --ignored --nocapture
//! ```

#![cfg(unix)]

mod common;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use sce_app_core::claude_code::{AuthorServer, ClaudeCode, ClaudeCodeConfig};
use sce_app_core::requests::Inputs;
use sce_app_core::runner::{Generator, Outcome, Runner, RunnerConfig};
use sce_app_core::{Registration, SceCodegen, SystemClock, WorkStore};

const SPECIFICATION: &str = "\
A door controller for a storage room.

The door is closed until somebody presents a card. A card that is on the list of cards opens \
the door; a card that is not on the list does nothing, and the controller counts three \
misses in a row, after which it ignores every card for one minute.

An open door closes by itself five seconds after it opened. Pressing the close button on the \
inside closes an open door at once.
";

#[test]
#[ignore = "runs the real claude: minutes and money"]
fn the_real_client_writes_a_model_the_core_accepts_and_publishes() {
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

    let works = common::scratch("claude-live").join("works");
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
    let config = ClaudeCodeConfig {
        model: std::env::var("SCE_LIVE_MODEL").ok(),
        max_budget_usd: Some(5.0),
        timeout: Duration::from_secs(25 * 60),
        ..ClaudeCodeConfig::default()
    };
    let client = ClaudeCode::find(author, config).expect("claude on the search path");
    println!("client version: {:?}", client.version());

    let mut runner_config = RunnerConfig::named("live");
    runner_config.repairs = 1;
    let runner = Runner::new(
        Arc::clone(&store),
        Arc::new(SceCodegen::at(&codegen)),
        Arc::new(client),
        runner_config,
    );

    let outcome = runner.run_once().unwrap();
    println!("outcome: {outcome:?}");

    let Outcome::Completed { bundle, .. } = outcome else {
        let seen = store.read_request(&work, &request).unwrap();
        panic!(
            "the client did not write a model the core accepts: {outcome:?}; the request is {:?}",
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
