// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A shell that hosts the executor on a computer that has Codex, with or without Claude Code.
//!
//! The executor used to start only where Claude Code was found, so a computer with Codex and no
//! Claude could not run a request made for a Codex connection, whatever the person chose. What is
//! held here is that either client is enough for a shell that has the person's settings (the
//! requests are run for a connection, and a connection says which client and which credential),
//! that a shell without them still needs Claude Code (it runs the requests nobody chose a
//! connection for, and Codex has no default credential to run those with), and that what the
//! screen is told of a client is the one the executor reports itself as. The clients here are
//! scripts.
//!
//! Unix only: the stand-ins are shell scripts.

#![cfg(unix)]

mod common;

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use sce_app_core::host::{start, start_with, HostSettings, NotHosted};
use sce_app_core::requests::{Inputs, Pin, State};
use sce_app_core::{
    AdapterKind, AuthSource, Connection, ConnectionId, ConnectionStore, Limits, ManualClock,
    Policy, Registration, Saved, WorkId, WorkStore,
};

use common::FakeRenderer;

fn lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
    let map: HashMap<String, OsString> = pairs
        .iter()
        .map(|(k, v)| (k.to_string(), OsString::from(v)))
        .collect();
    move |key| map.get(key).cloned()
}

fn store(label: &str) -> Arc<WorkStore<Arc<ManualClock>>> {
    let clock = Arc::new(ManualClock::at(1_791_190_800));
    Arc::new(WorkStore::with_clock(common::scratch(label), clock))
}

/// A `codex` that says what it is and signs in as a ChatGPT plan, a `claude` that says what it
/// is, and a launcher for the authoring server that is ready.
struct Programs {
    codex: PathBuf,
    claude: PathBuf,
    author: PathBuf,
}

fn programs(label: &str) -> Programs {
    let dir = common::scratch(label);
    let codex = dir.join("codex");
    common::write_program(
        &codex,
        "#!/bin/sh\n\
         if [ \"$1\" = \"--version\" ]; then echo 'codex-cli 0.159.1'; exit 0; fi\n\
         if [ \"$1\" = \"features\" ]; then printf 'shell_tool  stable  true\\n'; exit 0; fi\n\
         if [ \"$1\" = \"login\" ]; then echo 'Logged in using ChatGPT'; exit 0; fi\n\
         cat > /dev/null\nexit 1\n",
    );
    let claude = dir.join("claude");
    common::write_program(
        &claude,
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo '2.1.289 (Claude Code)'; exit 0; fi\n\
         cat > /dev/null\nexit 1\n",
    );
    let author = dir.join("sce-author-mcp");
    common::write_program(&author, "#!/bin/sh\nexit 0\n");
    Programs {
        codex,
        claude,
        author,
    }
}

fn settings(author: &Path, claude: Option<&PathBuf>, codex: Option<&PathBuf>) -> HostSettings {
    let mut settings = HostSettings::from_lookup("desktop", lookup(&[]));
    settings.claude = Some(
        claude
            .cloned()
            .unwrap_or_else(|| PathBuf::from("/nowhere/claude")),
    );
    settings.codex = codex.cloned();
    settings.author = Some(author.to_path_buf());
    settings.retry = Duration::from_millis(50);
    settings
}

/// A Codex connection that signs in with the official client's login, and a pin to it.
fn pinned_to_codex(label: &str) -> (ConnectionStore, Pin) {
    let settings = ConnectionStore::at(common::scratch(label));
    let connection = Connection {
        id: ConnectionId::parse("gpt").unwrap(),
        adapter: AdapterKind::Codex,
        display_name: None,
        executable: None,
        model: Some("gpt-test".to_string()),
        auth: AuthSource::OfficialLogin,
        server_url: None,
        limits: Limits::default(),
    };
    let revision = match settings.save(&connection, None).unwrap() {
        Saved::Saved { revision, .. } | Saved::Unchanged { revision } => revision,
    };
    let pin = Pin {
        connection: connection.id,
        revision,
        adapter: connection.adapter,
        model: connection.model,
        limits: connection.limits,
    };
    (settings, pin)
}

fn ask_for(store: &WorkStore<Arc<ManualClock>>, id: &WorkId, pin: Pin) -> String {
    store
        .register_request_for(
            id,
            Registration {
                key: "press-1",
                origin: "gui",
                expect: Inputs {
                    source: store.head(id).unwrap().unwrap(),
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

#[test]
fn the_program_that_is_codex_is_read_from_the_name_the_environment_gives() {
    let named =
        HostSettings::from_lookup("desktop", lookup(&[("SCE_CODEX", "/opt/codex/bin/codex")]));
    let empty = HostSettings::from_lookup("desktop", lookup(&[("SCE_CODEX", "")]));
    let unset = HostSettings::from_lookup("desktop", lookup(&[]));

    assert_eq!(named.codex, Some(PathBuf::from("/opt/codex/bin/codex")));
    assert_eq!(empty.codex, None);
    assert_eq!(unset.codex, None);
}

#[test]
fn a_computer_with_codex_and_no_claude_hosts_for_the_connections_it_has() {
    let store = store("hostx-codex-only");
    let programs = programs("hostx-codex-only-bin");
    let (connections, _) = pinned_to_codex("hostx-codex-only-settings");

    let host = start_with(
        Arc::clone(&store),
        Arc::new(FakeRenderer),
        settings(&programs.author, None, Some(&programs.codex)),
        Some((connections, Policy::shipped())),
    );

    assert_eq!(host.not_hosted(), None);
    // The executor is reported as the client there is, with that client's version.
    assert_eq!(host.client_version().as_deref(), Some("0.159.1"));
    let said = &store.host_status().unwrap().hosts[0];
    assert!(said.host.hosting);
    assert_eq!(said.host.client_version.as_deref(), Some("0.159.1"));
    // The runner says what it is on its own thread, a moment after the host is up.
    within_ten_seconds("the executor did not report itself", || {
        !store.adapter_status().unwrap().adapters.is_empty()
    });
    let adapters = store.adapter_status().unwrap();
    assert_eq!(adapters.adapters[0].adapter.kind, "codex");
}

#[test]
fn a_request_made_for_codex_waits_with_its_reason_where_the_screen_reads_it() {
    let store = store("hostx-waits");
    let id = store.create_work("Door lock").unwrap().id;
    store.save_source(&id, "The lock opens.", None).unwrap();
    let programs = programs("hostx-waits-bin");
    let (connections, pin) = pinned_to_codex("hostx-waits-settings");
    let request = ask_for(&store, &id, pin);

    let host = start_with(
        Arc::clone(&store),
        Arc::new(FakeRenderer),
        settings(&programs.author, None, Some(&programs.codex)),
        Some((connections, Policy::shipped())),
    );

    // This stand-in reports an unverified version, so the request is not run, and the
    // person is told so instead of waiting for an AI that will not come.
    within_ten_seconds("the request was not said to wait", || {
        !store.host_status().unwrap().hosts[0]
            .host
            .waiting
            .is_empty()
    });
    let waiting = &store.host_status().unwrap().hosts[0].host.waiting[0];
    assert_eq!(waiting.request, request);
    assert!(
        waiting.reason.contains("has not been verified"),
        "{}",
        waiting.reason
    );
    assert_eq!(
        store.read_request(&id, &request).unwrap().state,
        State::Queued
    );
    drop(host);
}

#[test]
fn a_computer_with_both_reports_as_claude_code_and_still_runs_for_codex() {
    let store = store("hostx-both");
    let id = store.create_work("Door lock").unwrap().id;
    store.save_source(&id, "The lock opens.", None).unwrap();
    let programs = programs("hostx-both-bin");
    let (connections, pin) = pinned_to_codex("hostx-both-settings");
    ask_for(&store, &id, pin);

    let host = start_with(
        Arc::clone(&store),
        Arc::new(FakeRenderer),
        settings(
            &programs.author,
            Some(&programs.claude),
            Some(&programs.codex),
        ),
        Some((connections, Policy::shipped())),
    );

    assert_eq!(host.not_hosted(), None);
    assert_eq!(host.client_version().as_deref(), Some("2.1.289"));
    within_ten_seconds("the executor did not report itself", || {
        !store.adapter_status().unwrap().adapters.is_empty()
    });
    assert_eq!(
        store.adapter_status().unwrap().adapters[0].adapter.kind,
        "claude-code"
    );
    // Codex is reached all the same: the request made for it is judged as Codex's, and is left
    // waiting for the reason that is Codex's (not for a Codex that "was not found").
    within_ten_seconds("the request made for Codex was not looked at", || {
        !store.host_status().unwrap().hosts[0]
            .host
            .waiting
            .is_empty()
    });
    let reason = store.host_status().unwrap().hosts[0].host.waiting[0]
        .reason
        .clone();
    assert!(reason.contains("has not been verified"), "{reason}");
    drop(host);
}

#[test]
fn a_shell_without_the_settings_still_needs_claude_code() {
    let store = store("hostx-no-settings");
    let programs = programs("hostx-no-settings-bin");

    let host = start(
        Arc::clone(&store),
        Arc::new(FakeRenderer),
        settings(&programs.author, None, Some(&programs.codex)),
    );

    // It runs what nobody chose a connection for, and Codex has no credential to run that with.
    let Some(NotHosted::NoClient(tried)) = host.not_hosted() else {
        panic!("expected NoClient, got {:?}", host.not_hosted());
    };
    assert!(tried.contains("/nowhere/claude"), "{tried}");
    assert!(!tried.contains("codex"), "{tried}");
}

#[test]
fn a_shell_with_neither_client_says_where_it_looked_for_each() {
    let store = store("hostx-neither");
    let programs = programs("hostx-neither-bin");
    let (connections, _) = pinned_to_codex("hostx-neither-settings");
    let mut shell = settings(&programs.author, None, None);
    shell.codex = Some(PathBuf::from("/nowhere/codex"));

    let host = start_with(
        store,
        Arc::new(FakeRenderer),
        shell,
        Some((connections, Policy::shipped())),
    );

    let Some(NotHosted::NoClient(tried)) = host.not_hosted() else {
        panic!("expected NoClient, got {:?}", host.not_hosted());
    };
    assert!(tried.contains("/nowhere/claude"), "{tried}");
    assert!(tried.contains("/nowhere/codex"), "{tried}");
    let said = NotHosted::NoClient(tried).to_string();
    assert!(said.contains("SCE_CLAUDE"), "{said}");
    assert!(said.contains("SCE_CODEX"), "{said}");
}

#[test]
fn a_program_that_does_not_say_it_is_codex_is_not_taken_for_it() {
    let store = store("hostx-not-codex");
    let programs = programs("hostx-not-codex-bin");
    let (connections, _) = pinned_to_codex("hostx-not-codex-settings");
    let impostor = programs.codex.with_file_name("not-codex");
    common::write_program(&impostor, "#!/bin/sh\necho 'hello 1.0'\n");

    let host = start_with(
        store,
        Arc::new(FakeRenderer),
        settings(&programs.author, None, Some(&impostor)),
        Some((connections, Policy::shipped())),
    );

    let Some(NotHosted::NoClient(tried)) = host.not_hosted() else {
        panic!("expected NoClient, got {:?}", host.not_hosted());
    };
    assert!(tried.contains("not-codex"), "{tried}");
}

#[test]
fn a_host_started_before_codex_was_installed_hosts_once_it_is() {
    let store = store("hostx-late");
    let programs = programs("hostx-late-bin");
    let later = programs.codex.with_extension("later");
    std::fs::rename(&programs.codex, &later).unwrap();
    let (connections, _) = pinned_to_codex("hostx-late-settings");

    let host = start_with(
        Arc::clone(&store),
        Arc::new(FakeRenderer),
        settings(&programs.author, None, Some(&programs.codex)),
        Some((connections, Policy::shipped())),
    );
    assert!(matches!(host.not_hosted(), Some(NotHosted::NoClient(_))));
    std::fs::rename(&later, &programs.codex).unwrap();

    within_ten_seconds("the host did not start once Codex was installed", || {
        host.not_hosted().is_none()
    });
    assert_eq!(host.client_version().as_deref(), Some("0.159.1"));
    drop(host);
}
