// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A host that is running while the clients on the computer change.
//!
//! A person installs a client while the window is open: the screen shows it installed and signed
//! in the next time it asks, and a request made for it has to run without the window being closed
//! and opened again. The host used to look for its clients until it found one and then to keep
//! what it found, so a computer that had Codex first never ran for Claude Code installed later
//! (the screen said signed in, and the request waited until the application was restarted). What
//! is held here is that each client is looked for again while the host runs, in both directions
//! (the second one to arrive, and the one that goes away), and that what the host says it is
//! follows what it finds. The clients are scripts.
//!
//! Unix only: the stand-ins are shell scripts.

#![cfg(unix)]

mod common;

use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use sce_app_core::host::{start_with, HostSettings};
use sce_app_core::requests::{Inputs, Pin, State};
use sce_app_core::{
    AdapterKind, AuthSource, Connection, ConnectionId, ConnectionStore, Limits, ManualClock,
    Policy, Registration, Saved, WorkId, WorkStore,
};
use serde_json::json;

use common::FakeRenderer;

type Works = Arc<WorkStore<Arc<ManualClock>>>;

fn lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
    let map: HashMap<String, OsString> = pairs
        .iter()
        .map(|(k, v)| (k.to_string(), OsString::from(v)))
        .collect();
    move |key| map.get(key).cloned()
}

fn works(label: &str) -> Works {
    let clock = Arc::new(ManualClock::at(1_791_190_800));
    Arc::new(WorkStore::with_clock(common::scratch(label), clock))
}

/// The programs of one computer: a Claude Code that is signed in and writes a model, a Codex that
/// says what it is, and the authoring server's launcher.
struct Programs {
    claude: PathBuf,
    codex: PathBuf,
    author: PathBuf,
}

fn programs(label: &str) -> Programs {
    let dir = common::scratch(label);
    let answer = json!({
        "type": "result", "subtype": "success", "is_error": false,
        "structured_output": {
            "model": {"documents": [{"name": "m.scxml", "text": "<scxml><!-- late --></scxml>"}]},
            "requirements": {"manifest_text": "{\"doc_id\":\"door\",\"rev\":\"1\"}\n"},
        },
    });
    fs::write(dir.join("answer.json"), answer.to_string()).unwrap();
    let claude = dir.join("claude");
    common::write_program(
        &claude,
        &format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo '2.1.289 (Claude Code)'; exit 0; fi\n\
             if [ \"$3\" = \"auth\" ]; then echo '{{\"loggedIn\":true,\"authMethod\":\"claude.ai\",\"apiProvider\":\"firstParty\"}}'; exit 0; fi\n\
             cat > /dev/null\ncat '{}'\n",
            dir.join("answer.json").display()
        ),
    );
    let codex = dir.join("codex");
    common::write_program(
        &codex,
        "#!/bin/sh\n\
         if [ \"$1\" = \"--version\" ]; then echo 'codex-cli 0.159.0'; exit 0; fi\n\
         if [ \"$1\" = \"features\" ]; then printf 'shell_tool  stable  true\\n'; exit 0; fi\n\
         if [ \"$1\" = \"login\" ]; then echo 'Logged in using ChatGPT'; exit 0; fi\n\
         cat > /dev/null\nexit 1\n",
    );
    let author = dir.join("sce-author-mcp");
    common::write_program(&author, "#!/bin/sh\nexit 0\n");
    Programs {
        claude,
        codex,
        author,
    }
}

/// Take a program away, as the person who has not installed it yet; the returned path puts it back.
fn away(program: &Path) -> PathBuf {
    let held = program.with_extension("away");
    fs::rename(program, &held).unwrap();
    held
}

fn settings(programs: &Programs) -> HostSettings {
    let mut settings = HostSettings::from_lookup("desktop", lookup(&[]));
    settings.claude = Some(programs.claude.clone());
    settings.codex = Some(programs.codex.clone());
    settings.author = Some(programs.author.clone());
    settings.retry = Duration::from_millis(50);
    settings
}

/// A connection of `adapter` that signs in with the official client's login, and a pin to it.
fn pinned(label: &str, adapter: AdapterKind) -> (ConnectionStore, Pin) {
    let store = ConnectionStore::at(common::scratch(label));
    let connection = Connection {
        id: ConnectionId::parse("main").unwrap(),
        adapter,
        display_name: None,
        executable: None,
        model: Some("opus".to_string()),
        auth: AuthSource::OfficialLogin,
        server_url: None,
        limits: Limits::default(),
    };
    let revision = match store.save(&connection, None).unwrap() {
        Saved::Saved { revision, .. } | Saved::Unchanged { revision } => revision,
    };
    let pin = Pin {
        connection: connection.id,
        revision,
        adapter: connection.adapter,
        model: connection.model,
        limits: connection.limits,
    };
    (store, pin)
}

fn ask_for(works: &WorkStore<Arc<ManualClock>>, id: &WorkId, pin: Pin) -> String {
    works
        .register_request_for(
            id,
            Registration {
                key: "press-1",
                origin: "gui",
                expect: Inputs {
                    source: works.head(id).unwrap().unwrap(),
                    answers: None,
                },
                supersede: false,
            },
            Some(pin),
        )
        .unwrap()
        .request
        .id
}

fn within_ten_seconds(what: &str, until: impl Fn() -> bool) {
    let limit = Instant::now() + Duration::from_secs(10);
    while !until() {
        assert!(Instant::now() < limit, "{what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Why the host says the oldest request it left is waiting, when it left one.
fn waiting_reason(works: &WorkStore<Arc<ManualClock>>) -> Option<String> {
    works.host_status().unwrap().hosts[0]
        .host
        .waiting
        .first()
        .map(|w| w.reason.clone())
}

#[test]
fn a_host_that_started_with_codex_runs_for_claude_code_installed_after_it() {
    let works = works("hcc-claude-later");
    let id = works.create_work("Door lock").unwrap().id;
    works.save_source(&id, "The lock opens.", None).unwrap();
    let programs = programs("hcc-claude-later-bin");
    let back = away(&programs.claude);
    let (connections, pin) = pinned("hcc-claude-later-settings", AdapterKind::ClaudeCode);
    let request = ask_for(&works, &id, pin);

    let host = start_with(
        Arc::clone(&works),
        Arc::new(FakeRenderer),
        settings(&programs),
        Some((connections, Policy::shipped())),
    );

    // Hosting for Codex, and the request made for Claude Code waits, saying why.
    assert_eq!(host.not_hosted(), None);
    assert_eq!(host.client_version().as_deref(), Some("0.159.0"));
    within_ten_seconds("the request was not said to wait", || {
        waiting_reason(&works).is_some()
    });
    let reason = waiting_reason(&works).unwrap();
    assert!(reason.contains("Claude Code was not found"), "{reason}");

    // The person installs it while the window is open.
    fs::rename(&back, &programs.claude).unwrap();

    within_ten_seconds(
        "the request was not run once Claude Code was installed",
        || works.read_request(&id, &request).unwrap().state == State::Completed,
    );
    // And the host says it is Claude Code that it runs for now, where the screen reads it.
    within_ten_seconds("the host did not say which client it is", || {
        host.client_version().as_deref() == Some("2.1.289")
    });
    within_ten_seconds("the adapter was not reported as Claude Code", || {
        works
            .adapter_status()
            .unwrap()
            .adapters
            .iter()
            .any(|a| a.adapter.kind == "claude-code")
    });
    drop(host);
}

#[test]
fn a_host_that_started_with_claude_code_looks_for_codex_again() {
    let works = works("hcc-codex-later");
    let id = works.create_work("Door lock").unwrap().id;
    works.save_source(&id, "The lock opens.", None).unwrap();
    let programs = programs("hcc-codex-later-bin");
    let back = away(&programs.codex);
    let (connections, pin) = pinned("hcc-codex-later-settings", AdapterKind::Codex);
    ask_for(&works, &id, pin);

    let host = start_with(
        Arc::clone(&works),
        Arc::new(FakeRenderer),
        settings(&programs),
        Some((connections, Policy::shipped())),
    );

    assert_eq!(host.client_version().as_deref(), Some("2.1.289"));
    within_ten_seconds("the request was not said to wait", || {
        waiting_reason(&works).is_some()
    });
    let reason = waiting_reason(&works).unwrap();
    assert!(reason.contains("Codex was not found"), "{reason}");

    fs::rename(&back, &programs.codex).unwrap();

    // Codex is judged as Codex now: this build has verified none, and says so, and not that it
    // was not found.
    within_ten_seconds("the reason did not follow the install", || {
        waiting_reason(&works).is_some_and(|r| r.contains("has not been verified"))
    });
    drop(host);
}

#[test]
fn a_client_that_goes_away_while_the_host_runs_is_said_to_be_gone() {
    let works = works("hcc-gone");
    let id = works.create_work("Door lock").unwrap().id;
    works.save_source(&id, "The lock opens.", None).unwrap();
    let programs = programs("hcc-gone-bin");
    let (connections, pin) = pinned("hcc-gone-settings", AdapterKind::ClaudeCode);
    let host = start_with(
        Arc::clone(&works),
        Arc::new(FakeRenderer),
        settings(&programs),
        Some((connections, Policy::shipped())),
    );
    assert_eq!(host.client_version().as_deref(), Some("2.1.289"));
    let _held = away(&programs.claude);

    // A request made now is not started on a program that is not there: it waits, and says so.
    ask_for(&works, &id, pin);

    within_ten_seconds(
        "the request was not said to wait for a program that is gone",
        || waiting_reason(&works).is_some_and(|r| r.contains("Claude Code was not found")),
    );
    drop(host);
}
