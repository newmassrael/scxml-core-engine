// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A shell that hosts the executor for a model server the person runs, on a computer that has no
//! AI client installed.
//!
//! A model server is no program to find: it is found when the person registers one (a connection
//! that names it), and the executor then needs the authoring server and nothing else. What is held
//! is that such a shell hosts once a server is registered and not before (and says so, where the
//! screen reads it, as the server it runs for), that a request made for the server's connection is
//! written by the server, and that a shell that has neither a client nor a server still says where
//! it looked. The model server and the authoring server are stand-ins (`common::model_server`).
//!
//! Unix only: the authoring server stand-in is a shell script.

#![cfg(unix)]

mod common;

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use sce_app_core::codex::AUTHOR_TOOLS;
use sce_app_core::host::{start_with, HostSettings, NotHosted};
use sce_app_core::requests::{Inputs, Pin, State};
use sce_app_core::{
    AdapterKind, AuthSource, Connection, ConnectionId, ConnectionStore, Limits, ManualClock,
    Policy, Registration, Saved, WorkId, WorkStore,
};

use common::model_server::{authoring_server, chat_server, draft, says};
use common::FakeRenderer;

fn lookup(_: &str) -> Option<OsString> {
    None
}

fn store(label: &str) -> Arc<WorkStore<Arc<ManualClock>>> {
    let clock = Arc::new(ManualClock::at(1_791_190_800));
    Arc::new(WorkStore::with_clock(common::scratch(label), clock))
}

/// The shell's settings: no Claude Code and no Codex, and a launcher for the authoring server that
/// is ready (and speaks the protocol, for the run that is made for a server).
fn settings(label: &str) -> HostSettings {
    let mut settings = HostSettings::from_lookup("desktop", lookup);
    settings.claude = Some(PathBuf::from("/nowhere/claude"));
    settings.author = Some(authoring_server(&common::scratch(label), &AUTHOR_TOOLS).command);
    settings.retry = Duration::from_millis(50);
    settings
}

/// A connection to a model server at `address`.
fn server(address: &str) -> Connection {
    Connection {
        id: ConnectionId::parse("pc2").unwrap(),
        adapter: AdapterKind::Local,
        display_name: Some("pc2 (tunnel)".to_string()),
        executable: None,
        model: Some("qwen-test".to_string()),
        auth: AuthSource::NoAuth,
        server_url: Some(address.to_string()),
        limits: Limits::default(),
    }
}

fn register(settings: &ConnectionStore, connection: &Connection) -> Pin {
    let revision = match settings.save(connection, None).unwrap() {
        Saved::Saved { revision, .. } | Saved::Unchanged { revision } => revision,
    };
    Pin {
        connection: connection.id.clone(),
        revision,
        adapter: connection.adapter,
        model: connection.model.clone(),
        limits: connection.limits.clone(),
    }
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
fn a_computer_with_no_client_hosts_once_a_model_server_is_registered_and_not_before() {
    let store = store("hostl-late");
    let connections = ConnectionStore::at(common::scratch("hostl-late-settings"));

    let host = start_with(
        Arc::clone(&store),
        Arc::new(FakeRenderer),
        settings("hostl-late-bin"),
        Some((connections.clone(), Policy::shipped())),
    );

    // Nothing is registered: it says where it looked, and that a server is a way out of it.
    let Some(NotHosted::NoClient(tried)) = host.not_hosted() else {
        panic!("expected NoClient, got {:?}", host.not_hosted());
    };
    assert!(tried.contains("/nowhere/claude"), "{tried}");
    assert!(tried.contains("none is registered"), "{tried}");
    assert!(NotHosted::NoClient(tried)
        .to_string()
        .contains("register a model server"));

    register(&connections, &server("http://127.0.0.1:1/v1"));

    within_ten_seconds(
        "the host did not start once a server was registered",
        || host.not_hosted().is_none(),
    );
    // There is no client, so there is no version of one; and it says what it runs for.
    assert_eq!(host.client_version(), None);
    within_ten_seconds("the host did not say it hosts", || {
        store.host_status().unwrap().hosts[0].host.hosting
    });
    within_ten_seconds("the executor did not report itself", || {
        !store.adapter_status().unwrap().adapters.is_empty()
    });
    assert_eq!(
        store.adapter_status().unwrap().adapters[0].adapter.kind,
        "local"
    );
    drop(host);
}

#[test]
fn a_request_made_for_a_model_server_is_written_by_it_and_completes() {
    let store = store("hostl-run");
    let id = store.create_work("Door lock").unwrap().id;
    store.save_source(&id, "The lock opens.", None).unwrap();
    let model = chat_server(vec![says(&draft("<scxml><!-- door --></scxml>"))]);
    let connections = ConnectionStore::at(common::scratch("hostl-run-settings"));
    let pin = register(&connections, &server(&model.address));
    let request = ask_for(&store, &id, pin);

    let host = start_with(
        Arc::clone(&store),
        Arc::new(FakeRenderer),
        settings("hostl-run-bin"),
        Some((connections, Policy::shipped())),
    );

    within_ten_seconds("the request was not run", || {
        store.read_request(&id, &request).unwrap().state == State::Completed
    });
    drop(host);
    // The server was asked, for the model the connection names, and by no other.
    let asked = model.requests();
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].body["model"], "qwen-test");
    assert!(
        asked[0].head.starts_with("POST /v1/chat/completions"),
        "{}",
        asked[0].head
    );
    // What it wrote is what the work holds.
    let written = store.read_model(&id, None).unwrap().expect("a model");
    assert!(written.text.contains("door"), "{}", written.text);
}
