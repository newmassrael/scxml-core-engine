// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Which AI a person connects the workbench to, kept apart from the works.
//!
//! A connection is the settings of one way to reach a model: which client, which model,
//! where its credentials come from. It is kept in the person's own settings folder and not in
//! the works folder, because a works folder is shared and moved, and nobody's account is
//! part of a work. A request will pin a connection by its id and the revision it had when the
//! request was made, so a revision is the digest of its bytes, an earlier one stays readable,
//! and a save that does not know what it replaces is refused. No secret is kept here: a field
//! that could hold one is not accepted, and a file that has one is not read.

mod common;

use std::fs;
use std::sync::{Arc, Barrier};
use std::thread;

use sce_app_core::{
    AdapterKind, AuthSource, Connection, ConnectionId, ConnectionStore, Limits, Revision, Saved,
    StoreError,
};

fn id(text: &str) -> ConnectionId {
    ConnectionId::parse(text).expect("a valid connection id")
}

fn claude(name: &str) -> Connection {
    Connection {
        id: id(name),
        adapter: AdapterKind::ClaudeCode,
        display_name: None,
        executable: None,
        model: Some("opus".to_string()),
        auth: AuthSource::OfficialLogin,
        server_url: None,
        limits: Limits::default(),
    }
}

fn local(name: &str) -> Connection {
    Connection {
        id: id(name),
        adapter: AdapterKind::Local,
        display_name: Some("pc2 (tunnel)".to_string()),
        executable: None,
        model: Some("qwen3-coder:30b".to_string()),
        auth: AuthSource::NoAuth,
        server_url: Some("http://127.0.0.1:11434/v1".to_string()),
        limits: Limits::default(),
    }
}

/// A connection of `adapter` with everything that adapter needs, and `auth` as asked.
fn shaped(adapter: AdapterKind, auth: AuthSource) -> Connection {
    let mut connection = match adapter {
        AdapterKind::Local => local("shaped"),
        _ => claude("shaped"),
    };
    connection.adapter = adapter;
    connection.auth = auth;
    connection
}

fn store(label: &str) -> ConnectionStore {
    ConnectionStore::at(common::scratch(label))
}

fn saved_revision(saved: Saved) -> Revision {
    match saved {
        Saved::Saved { revision, .. } | Saved::Unchanged { revision } => revision,
    }
}

fn refusal(connection: &Connection) -> String {
    let store = store("connections-refusal");
    let error = store
        .save(connection, None)
        .expect_err("a connection like this is refused");
    assert_eq!(error.kind(), "bad-connection", "{error}");
    error.to_string()
}

fn accepted(connection: &Connection) {
    let store = store("connections-accepted");
    store
        .save(connection, None)
        .unwrap_or_else(|e| panic!("this connection is accepted: {e}"));
}

#[test]
fn a_saved_connection_reads_back_under_the_digest_of_its_stored_bytes() {
    let store = store("connections-digest");

    let saved = store.save(&claude("main"), None).unwrap();

    let Saved::Saved { revision, parent } = saved else {
        panic!("a first save is a new revision");
    };
    assert_eq!(parent, None);
    let read = store.read(&id("main"), None).unwrap().expect("it is there");
    assert_eq!(read.revision, revision);
    assert_eq!(read.connection, claude("main"));
    let stored = fs::read(
        store
            .root()
            .join("connections")
            .join("main")
            .join(format!("{revision}.json")),
    )
    .unwrap();
    assert_eq!(Revision::of(&stored), revision);
}

#[test]
fn saving_the_same_connection_again_writes_nothing() {
    let store = store("connections-unchanged");
    let first = saved_revision(store.save(&claude("main"), None).unwrap());
    let before = fs::read_dir(store.root().join("connections").join("main"))
        .unwrap()
        .count();

    let again = store.save(&claude("main"), Some(&first)).unwrap();

    assert_eq!(
        again,
        Saved::Unchanged {
            revision: first.clone()
        }
    );
    let after = fs::read_dir(store.root().join("connections").join("main"))
        .unwrap()
        .count();
    assert_eq!(before, after);
}

#[test]
fn a_save_from_a_stale_base_conflicts_and_writes_nothing() {
    let store = store("connections-stale");
    let first = saved_revision(store.save(&claude("main"), None).unwrap());
    let mut changed = claude("main");
    changed.model = Some("sonnet".to_string());
    let second = saved_revision(store.save(&changed, Some(&first)).unwrap());
    let mut other = claude("main");
    other.model = Some("haiku".to_string());

    let error = store.save(&other, Some(&first)).unwrap_err();

    match error {
        StoreError::Conflict { base, current } => {
            assert_eq!(base, Some(first));
            assert_eq!(current, Some(second.clone()));
        }
        other => panic!("expected a conflict, got {other}"),
    }
    let now = store.read(&id("main"), None).unwrap().unwrap();
    assert_eq!(now.revision, second);
    assert_eq!(now.connection, changed);
}

#[test]
fn a_save_that_does_not_name_what_it_replaces_cannot_replace_a_connection() {
    let store = store("connections-blind");
    let first = saved_revision(store.save(&claude("main"), None).unwrap());
    let mut changed = claude("main");
    changed.model = Some("sonnet".to_string());

    let error = store.save(&changed, None).unwrap_err();

    assert_eq!(error.kind(), "conflict");
    assert_eq!(
        store.read(&id("main"), None).unwrap().unwrap().revision,
        first
    );
}

#[test]
fn a_new_connection_cannot_claim_to_replace_a_revision() {
    let store = store("connections-phantom");

    let error = store
        .save(&claude("main"), Some(&Revision::of(b"nothing")))
        .unwrap_err();

    assert_eq!(error.kind(), "conflict");
    assert!(store.read(&id("main"), None).unwrap().is_none());
}

#[test]
fn an_earlier_revision_stays_readable_after_a_newer_save() {
    let store = store("connections-earlier");
    let first = saved_revision(store.save(&claude("main"), None).unwrap());
    let mut changed = claude("main");
    changed.model = Some("sonnet".to_string());
    let second = saved_revision(store.save(&changed, Some(&first)).unwrap());

    let old = store.read(&id("main"), Some(&first)).unwrap().unwrap();
    let current = store.read(&id("main"), None).unwrap().unwrap();

    assert_eq!(old.connection, claude("main"));
    assert_eq!(old.revision, first);
    assert_eq!(current.revision, second);
    assert_ne!(first, second);
}

#[test]
fn a_connection_nobody_saved_reads_as_none_and_a_revision_nobody_kept_is_not_found() {
    let store = store("connections-absent");
    store.save(&claude("main"), None).unwrap();

    assert!(store.read(&id("never"), None).unwrap().is_none());
    let error = store
        .read(&id("main"), Some(&Revision::of(b"never kept")))
        .unwrap_err();
    assert_eq!(error.kind(), "not-found");
}

#[test]
fn each_adapter_accepts_only_the_credential_sources_it_has() {
    use AdapterKind::{ClaudeCode, Codex, Local};
    use AuthSource::{AppStore, EnvApiKey, NoAuth, OfficialLogin, ServerKey};
    let table = [
        (ClaudeCode, OfficialLogin, true),
        (ClaudeCode, AppStore, false),
        (ClaudeCode, EnvApiKey, false),
        (ClaudeCode, ServerKey, false),
        (ClaudeCode, NoAuth, false),
        (Codex, AppStore, true),
        (Codex, EnvApiKey, true),
        (Codex, OfficialLogin, true),
        (Codex, ServerKey, false),
        (Codex, NoAuth, false),
        (Local, NoAuth, true),
        (Local, ServerKey, true),
        (Local, OfficialLogin, false),
        (Local, AppStore, false),
        (Local, EnvApiKey, false),
    ];

    for (adapter, auth, ok) in table {
        let connection = shaped(adapter, auth);
        if ok {
            accepted(&connection);
        } else {
            let said = refusal(&connection);
            assert!(
                said.contains("credential"),
                "{adapter:?} with {auth:?} is refused for its credential source: {said}"
            );
        }
    }
}

#[test]
fn a_local_connection_needs_a_server_and_a_name_and_the_others_refuse_a_server() {
    let mut no_server = local("pc2");
    no_server.server_url = None;
    assert!(refusal(&no_server).contains("server address"));

    for name in [None, Some(String::new()), Some("   ".to_string())] {
        let mut unnamed = local("pc2");
        unnamed.display_name = name;
        assert!(refusal(&unnamed).contains("display name"));
    }

    for adapter in [AdapterKind::ClaudeCode, AdapterKind::Codex] {
        let mut with_server = shaped(adapter, AuthSource::OfficialLogin);
        with_server.server_url = Some("http://127.0.0.1:11434/v1".to_string());
        assert!(refusal(&with_server).contains("server address"));
    }

    let mut named = claude("main");
    named.display_name = Some("My Claude".to_string());
    accepted(&named);
}

#[test]
fn a_server_address_that_could_carry_a_secret_is_refused() {
    for address in [
        "",
        "ftp://host/v1",
        "127.0.0.1:11434/v1",
        "http://",
        "http:///v1",
        "http://user:password@host/v1",
        "http://token@host/v1",
        "http://host/v1?key=abc",
        "http://host/v1#fragment",
        "http://host /v1",
        "http://host/v1\n",
    ] {
        let mut connection = local("pc2");
        connection.server_url = Some(address.to_string());
        assert!(
            refusal(&connection).contains("server address"),
            "`{address:?}` is refused as an address"
        );
    }
    for address in [
        "http://127.0.0.1:11434/v1",
        "https://models.example.com/v1",
        "http://[::1]:11434/v1",
        "http://localhost",
    ] {
        let mut connection = local("pc2");
        connection.server_url = Some(address.to_string());
        accepted(&connection);
    }
}

#[test]
fn a_connection_id_is_lowercase_letters_digits_and_hyphens() {
    for bad in [
        "",
        "Main",
        "-a",
        "a-",
        "a_b",
        "a b",
        "../x",
        "a/b",
        &"x".repeat(41),
    ] {
        let error = ConnectionId::parse(bad).unwrap_err();
        assert_eq!(error.kind(), "bad-connection", "`{bad}`");
    }
    for good in ["a", "0", "claude-1", "pc2-tunnel", &"x".repeat(40)] {
        assert_eq!(ConnectionId::parse(good).unwrap().as_str(), good);
    }
}

#[test]
fn limits_are_positive_and_bounded() {
    for (turns, seconds, ok) in [
        (Some(1), Some(1), true),
        (Some(500), Some(86_400), true),
        (None, None, true),
        (Some(0), None, false),
        (Some(501), None, false),
        (None, Some(0), false),
        (None, Some(86_401), false),
    ] {
        let mut connection = claude("main");
        connection.limits = Limits { turns, seconds };
        if ok {
            accepted(&connection);
        } else {
            assert!(refusal(&connection).contains("limit"));
        }
    }
}

#[test]
fn a_model_a_name_and_an_executable_must_be_text_a_person_can_read() {
    let mut connection = claude("main");
    connection.model = Some(String::new());
    assert!(refusal(&connection).contains("model"));
    connection.model = Some("x".repeat(201));
    assert!(refusal(&connection).contains("model"));
    connection.model = Some("opus\nsonnet".to_string());
    assert!(refusal(&connection).contains("model"));

    let mut connection = claude("main");
    connection.display_name = Some("x".repeat(81));
    assert!(refusal(&connection).contains("display name"));
    connection.display_name = Some("two\nlines".to_string());
    assert!(refusal(&connection).contains("display name"));

    let mut connection = claude("main");
    connection.executable = Some("claude".to_string());
    assert!(refusal(&connection).contains("executable"));
    connection.executable = Some("/usr/bin/cla\0ude".to_string());
    assert!(refusal(&connection).contains("executable"));
    connection.executable = Some("/usr/local/bin/claude".to_string());
    accepted(&connection);
}

#[test]
fn the_list_gives_the_current_connections_by_id_and_names_what_it_could_not_read() {
    let store = store("connections-list");
    store.save(&local("b-local"), None).unwrap();
    store.save(&claude("a-claude"), None).unwrap();
    let folder = store.root().join("connections");
    // A head that names a revision nobody kept.
    fs::create_dir_all(folder.join("lost")).unwrap();
    fs::write(
        folder.join("lost").join("head"),
        format!("{}\n", Revision::of(b"gone")),
    )
    .unwrap();
    // A head that is not a revision at all.
    fs::create_dir_all(folder.join("garbled")).unwrap();
    fs::write(folder.join("garbled").join("head"), "not a revision\n").unwrap();

    let listing = store.list().unwrap();

    let ids: Vec<&str> = listing
        .connections
        .iter()
        .map(|c| c.connection.id.as_str())
        .collect();
    assert_eq!(ids, vec!["a-claude", "b-local"]);
    let unreadable: Vec<&str> = listing.unreadable.iter().map(|u| u.id.as_str()).collect();
    assert_eq!(unreadable, vec!["garbled", "lost"]);
}

#[test]
fn a_file_with_a_field_the_store_does_not_know_is_not_read() {
    let store = store("connections-unknown-field");
    store.save(&claude("main"), None).unwrap();
    let folder = store.root().join("connections").join("planted");
    fs::create_dir_all(&folder).unwrap();
    let bytes =
        br#"{"id":"planted","adapter":"claude-code","auth":"official-login","api_key":"sk-secret"}
"#;
    let revision = Revision::of(bytes);
    fs::write(folder.join(format!("{revision}.json")), bytes).unwrap();
    fs::write(folder.join("head"), format!("{revision}\n")).unwrap();

    let listing = store.list().unwrap();

    assert_eq!(listing.connections.len(), 1);
    assert_eq!(listing.unreadable.len(), 1);
    assert_eq!(listing.unreadable[0].id, "planted");
    assert!(store.read(&id("planted"), None).is_err());
}

#[test]
fn a_file_written_by_hand_is_held_to_the_rules_of_a_save() {
    let store = store("connections-by-hand");
    // Each is plain JSON of fields this type has, so only the rules of a save can refuse it.
    let planted = [
        (
            "userinfo",
            r#"{"id":"userinfo","adapter":"local","display_name":"pc2","auth":"none","server_url":"http://user:pw@host/v1"}"#,
        ),
        (
            "wrong-source",
            r#"{"id":"wrong-source","adapter":"claude-code","auth":"env-api-key"}"#,
        ),
        (
            "renamed",
            r#"{"id":"someone-else","adapter":"claude-code","auth":"official-login"}"#,
        ),
    ];
    for (name, json) in planted {
        let bytes = format!("{json}\n").into_bytes();
        let revision = Revision::of(&bytes);
        let folder = store.root().join("connections").join(name);
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join(format!("{revision}.json")), &bytes).unwrap();
        fs::write(folder.join("head"), format!("{revision}\n")).unwrap();

        let error = store.read(&id(name), None).unwrap_err();

        assert_eq!(error.kind(), "corrupt", "{name}: {error}");
    }

    let listing = store.list().unwrap();

    assert!(listing.connections.is_empty());
    assert_eq!(listing.unreadable.len(), 3);
}

#[test]
fn a_revision_file_whose_bytes_are_not_its_name_is_not_read() {
    let store = store("connections-tampered");
    let revision = saved_revision(store.save(&claude("main"), None).unwrap());
    let path = store
        .root()
        .join("connections")
        .join("main")
        .join(format!("{revision}.json"));
    let mut bytes = fs::read(&path).unwrap();
    bytes.extend_from_slice(b" ");
    fs::write(&path, bytes).unwrap();

    let error = store.read(&id("main"), None).unwrap_err();

    assert_eq!(error.kind(), "corrupt");
    assert_eq!(store.list().unwrap().unreadable.len(), 1);
}

#[test]
fn deleting_takes_a_connection_out_of_the_list_and_keeps_its_revisions() {
    let store = store("connections-delete");
    let revision = saved_revision(store.save(&claude("main"), None).unwrap());
    store.save(&local("pc2"), None).unwrap();

    store.delete(&id("main"), &revision).unwrap();

    let listing = store.list().unwrap();
    assert_eq!(listing.connections.len(), 1);
    assert_eq!(listing.connections[0].connection.id.as_str(), "pc2");
    // What a delete leaves behind is not a connection, and it is not damage either.
    assert!(listing.unreadable.is_empty());
    assert!(store.read(&id("main"), None).unwrap().is_none());
    // A request made while it existed can still be run with the settings it was made with.
    let kept = store.read(&id("main"), Some(&revision)).unwrap().unwrap();
    assert_eq!(kept.connection, claude("main"));
}

#[test]
fn deleting_from_a_stale_base_conflicts_and_deletes_nothing() {
    let store = store("connections-delete-stale");
    let first = saved_revision(store.save(&claude("main"), None).unwrap());
    let mut changed = claude("main");
    changed.model = Some("sonnet".to_string());
    store.save(&changed, Some(&first)).unwrap();

    let error = store.delete(&id("main"), &first).unwrap_err();

    assert_eq!(error.kind(), "conflict");
    assert!(store.read(&id("main"), None).unwrap().is_some());
}

#[test]
fn deleting_a_connection_that_is_not_there_is_not_found() {
    let store = store("connections-delete-absent");

    let error = store
        .delete(&id("main"), &Revision::of(b"anything"))
        .unwrap_err();

    assert_eq!(error.kind(), "not-found");
}

#[test]
fn the_default_is_a_connection_that_exists() {
    let store = store("connections-default");
    assert_eq!(store.default_connection().unwrap(), None);

    let missing = store.set_default(Some(&id("main")), None).unwrap_err();
    assert_eq!(missing.kind(), "not-found");

    store.save(&claude("main"), None).unwrap();
    store.set_default(Some(&id("main")), None).unwrap();
    assert_eq!(store.default_connection().unwrap(), Some(id("main")));
}

#[test]
fn setting_the_default_from_a_stale_expectation_is_refused_as_moved() {
    let store = store("connections-default-moved");
    store.save(&claude("main"), None).unwrap();
    store.save(&local("pc2"), None).unwrap();
    store.set_default(Some(&id("main")), None).unwrap();

    // Another window made `pc2` the default while this one still showed none.
    store
        .set_default(Some(&id("pc2")), Some(&id("main")))
        .unwrap();
    let error = store
        .set_default(Some(&id("main")), Some(&id("main")))
        .unwrap_err();

    assert_eq!(error.kind(), "moved");
    assert_eq!(store.default_connection().unwrap(), Some(id("pc2")));
}

#[test]
fn clearing_the_default_and_deleting_the_default_both_leave_none() {
    let store = store("connections-default-cleared");
    let main = saved_revision(store.save(&claude("main"), None).unwrap());
    store.set_default(Some(&id("main")), None).unwrap();

    store.set_default(None, Some(&id("main"))).unwrap();
    assert_eq!(store.default_connection().unwrap(), None);

    store.set_default(Some(&id("main")), None).unwrap();
    store.delete(&id("main"), &main).unwrap();
    assert_eq!(store.default_connection().unwrap(), None);
}

#[test]
fn two_saves_from_the_same_base_cannot_both_win() {
    for round in 0..20 {
        let root = common::scratch(&format!("connections-race-{round}"));
        let start = Arc::new(Barrier::new(2));
        let racers: Vec<_> = ["opus", "sonnet"]
            .into_iter()
            .map(|model| {
                let root = root.clone();
                let start = Arc::clone(&start);
                thread::spawn(move || {
                    let store = ConnectionStore::at(root);
                    let mut connection = claude("main");
                    connection.model = Some(model.to_string());
                    start.wait();
                    store.save(&connection, None)
                })
            })
            .collect();

        let outcomes: Vec<_> = racers.into_iter().map(|r| r.join().unwrap()).collect();

        let wins = outcomes.iter().filter(|o| o.is_ok()).count();
        let conflicts = outcomes
            .iter()
            .filter(|o| matches!(o, Err(StoreError::Conflict { .. })))
            .count();
        assert_eq!((wins, conflicts), (1, 1), "round {round}: {outcomes:?}");
    }
}
