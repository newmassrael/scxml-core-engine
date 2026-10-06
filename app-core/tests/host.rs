// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A shell that hosts the application's own executor: what it needs to find, and what it does.
//!
//! The settings are read from names the environment gives (`SCE_CLAUDE`, `SCE_AUTHOR_MCP`, ...),
//! and a shell that cannot find what it needs says so in words and hosts nothing: a person who
//! has no Claude Code installed has an application that shows "no AI connected" and works as it
//! always did. A host that was started takes the requests the owner makes, and leaves when it is
//! dropped, killing a client at work, so that closing the window does not leave one running.

mod common;

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use sce_app_core::host::{start, start_with, HostSettings, NotHosted};
use sce_app_core::installed::Installed;
use sce_app_core::requests::{Inputs, Pin, State};
use sce_app_core::{
    AdapterKind, AuthSource, Connection, ConnectionId, ConnectionStore, Limits, ManualClock,
    Policy, Registration, Saved, WorkId, WorkStore,
};
use serde_json::json;

use common::FakeRenderer;

fn lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
    let map: HashMap<String, OsString> = pairs
        .iter()
        .map(|(k, v)| (k.to_string(), OsString::from(v)))
        .collect();
    move |key| map.get(key).cloned()
}

#[test]
fn the_settings_are_read_from_the_names_the_environment_gives() {
    let settings = HostSettings::from_lookup(
        "desktop",
        lookup(&[
            ("SCE_CLAUDE", "/opt/claude/bin/claude"),
            ("SCE_AUTHOR_MCP", "/opt/sce/bin/sce-author-mcp"),
            ("SCE_WORK", "/opt/sce/bin/sce-work"),
            ("SCE_CODEGEN", "/opt/sce/bin/sce-codegen"),
            ("SCE_CLAUDE_MODEL", "sonnet"),
            ("SCE_CLAUDE_BUDGET_USD", "2.5"),
        ]),
    );

    assert_eq!(settings.name, "desktop");
    assert!(settings.enabled);
    assert_eq!(
        settings.claude,
        Some(PathBuf::from("/opt/claude/bin/claude"))
    );
    assert_eq!(
        settings.author,
        Some(PathBuf::from("/opt/sce/bin/sce-author-mcp"))
    );
    assert_eq!(settings.work, Some(PathBuf::from("/opt/sce/bin/sce-work")));
    assert_eq!(
        settings.codegen,
        Some(PathBuf::from("/opt/sce/bin/sce-codegen"))
    );
    assert_eq!(settings.config.model.as_deref(), Some("sonnet"));
    assert_eq!(settings.config.max_budget_usd, Some(2.5));
}

#[test]
fn what_an_installer_carried_is_used_where_the_environment_named_nothing() {
    let bundle = Installed {
        author: Some(PathBuf::from("/opt/app/sce-author/bin/sce-author-mcp")),
        work: Some(PathBuf::from("/opt/app/sce-author/bin/sce-work")),
        codegen: Some(PathBuf::from("/opt/app/sce-author/bin/sce-codegen")),
    };

    let settings = HostSettings::from_lookup(
        "desktop",
        lookup(&[("SCE_WORK", "/home/dev/target/debug/sce-work")]),
    )
    .with_bundle(&bundle);

    assert_eq!(settings.author, bundle.author);
    assert_eq!(settings.codegen, bundle.codegen);
    // A developer's word is the developer's: the variable that was set still wins.
    assert_eq!(
        settings.work,
        Some(PathBuf::from("/home/dev/target/debug/sce-work"))
    );
    // A bundle that carries nothing changes nothing.
    let same = HostSettings::from_lookup("desktop", lookup(&[])).with_bundle(&Installed::default());
    assert_eq!(same.author, None);
}

#[test]
fn nothing_is_set_by_default_and_the_executor_is_on() {
    let settings = HostSettings::from_lookup("desktop", lookup(&[]));

    assert!(settings.enabled);
    assert_eq!(settings.claude, None);
    assert_eq!(settings.config.model, None);
    assert_eq!(settings.config.max_budget_usd, None);
}

#[test]
fn the_executor_can_be_turned_off_and_an_empty_value_is_no_value() {
    for off in ["off", "OFF", "0", "false", "no"] {
        let settings = HostSettings::from_lookup("desktop", lookup(&[("SCE_EXECUTOR", off)]));
        assert!(!settings.enabled, "{off}");
    }
    let empty = HostSettings::from_lookup(
        "desktop",
        lookup(&[("SCE_CLAUDE", ""), ("SCE_CLAUDE_BUDGET_USD", "")]),
    );
    assert_eq!(empty.claude, None);
    assert_eq!(empty.config.max_budget_usd, None);
}

#[test]
fn a_budget_that_is_not_a_positive_number_is_not_a_budget() {
    for bad in ["many", "-1", "0", "nan", "inf"] {
        let settings =
            HostSettings::from_lookup("desktop", lookup(&[("SCE_CLAUDE_BUDGET_USD", bad)]));
        assert_eq!(settings.config.max_budget_usd, None, "{bad}");
    }
}

fn store(label: &str) -> (Arc<ManualClock>, Arc<WorkStore<Arc<ManualClock>>>) {
    let clock = Arc::new(ManualClock::at(1_791_190_800));
    let store = Arc::new(WorkStore::with_clock(
        common::scratch(label),
        Arc::clone(&clock),
    ));
    (clock, store)
}

#[test]
fn a_shell_that_is_told_not_to_host_hosts_nothing_and_says_why() {
    let (_, store) = store("host-off");
    let mut settings = HostSettings::from_lookup("desktop", lookup(&[]));
    settings.enabled = false;

    let host = start(Arc::clone(&store), Arc::new(FakeRenderer), settings);

    assert_eq!(host.not_hosted(), Some(NotHosted::Off));
    assert!(NotHosted::Off.to_string().contains("SCE_EXECUTOR"));
    assert_eq!(host.client_version(), None);
}

#[test]
fn a_shell_the_owner_turned_off_does_not_go_looking_again() {
    let (_, store) = store("host-off-stays");
    let mut settings = HostSettings::from_lookup("desktop", lookup(&[]));
    settings.enabled = false;
    settings.retry = Duration::from_millis(20);

    let host = start(Arc::clone(&store), Arc::new(FakeRenderer), settings);
    std::thread::sleep(Duration::from_millis(300));

    // It is the owner's word and not something that can come right while the window is open.
    assert_eq!(host.not_hosted(), Some(NotHosted::Off));
    assert!(!store.host_status().unwrap().hosts[0].host.hosting);
}

#[test]
fn a_shell_that_is_dropped_while_it_waits_to_look_again_stops_at_once() {
    let (_, store) = store("host-drop-waiting");
    let mut settings = HostSettings::from_lookup("desktop", lookup(&[]));
    settings.claude = Some(PathBuf::from("/nowhere/claude"));
    settings.author = Some(PathBuf::from("/bin/sh"));
    settings.retry = Duration::from_secs(60);
    let host = start(store, Arc::new(FakeRenderer), settings);

    let started = Instant::now();
    drop(host);

    assert!(
        started.elapsed() < Duration::from_secs(5),
        "dropping the host waited for its next look"
    );
}

#[test]
fn what_a_shell_cannot_host_is_said_where_the_owner_looks_and_not_only_to_the_terminal() {
    let (_, store) = store("host-says-why");
    let mut settings = HostSettings::from_lookup("desktop", lookup(&[]));
    settings.claude = Some(PathBuf::from("/nowhere/claude"));
    settings.author = Some(PathBuf::from("/bin/sh"));

    let host = start(Arc::clone(&store), Arc::new(FakeRenderer), settings);

    // The word is in the works folder at once, for a screen to read: a message on the standard
    // error of a program started from a menu is one nobody reads.
    let listing = store.host_status().unwrap();
    assert_eq!(listing.hosts.len(), 1);
    let said = &listing.hosts[0];
    assert_eq!(said.host.name, "desktop");
    assert!(!said.host.hosting);
    let reason = said.host.reason.as_deref().expect("a reason");
    assert!(reason.contains("/nowhere/claude"), "{reason}");
    assert!(reason.contains("SCE_CLAUDE"), "{reason}");
    assert!(said.live);
    drop(host);
}

#[test]
fn a_shell_with_no_claude_hosts_nothing_and_says_what_to_install() {
    let (_, store) = store("host-no-claude");
    let mut settings = HostSettings::from_lookup("desktop", lookup(&[]));
    settings.claude = Some(PathBuf::from("/nowhere/claude"));
    settings.author = Some(PathBuf::from("/bin/sh"));

    let host = start(store, Arc::new(FakeRenderer), settings);

    let Some(NotHosted::NoClient(tried)) = host.not_hosted() else {
        panic!("expected NoClient, got {:?}", host.not_hosted());
    };
    assert!(tried.contains("/nowhere/claude"), "{tried}");
    assert!(NotHosted::NoClient(tried)
        .to_string()
        .contains("SCE_CLAUDE"));
}

#[cfg(unix)]
mod hosting {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    /// A `claude` that answers with a draft, and a launcher for the authoring server that is
    /// never run (the stand-in does not start it).
    fn installed(label: &str) -> (PathBuf, PathBuf) {
        let dir = common::scratch(label);
        let answer = json!({
            "type": "result", "subtype": "success", "is_error": false,
            "structured_output": {
                "model": {"documents": [{"name": "m.scxml", "text": "<scxml><!-- hosted --></scxml>"}]},
                "requirements": {"manifest_text": "{\"doc_id\":\"door\",\"rev\":\"1\"}\n"},
            },
        });
        fs::write(dir.join("answer.json"), answer.to_string()).unwrap();
        let claude = dir.join("claude");
        common::write_program(
            &claude,
            &format!(
                "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo \"2.1.289 (Claude Code)\"; exit 0; fi\ncat > /dev/null\ncat '{}'\n",
                dir.join("answer.json").display()
            ),
        );
        let author = dir.join("sce-author-mcp");
        common::write_program(&author, "#!/bin/sh\nexit 0\n");
        (claude, author)
    }

    fn within_ten_seconds(what: &str, until: impl Fn() -> bool) {
        let limit = Instant::now() + Duration::from_secs(10);
        while !until() {
            assert!(Instant::now() < limit, "{what}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn a_host_takes_the_request_the_owner_makes_and_leaves_when_it_is_dropped() {
        let (_, store) = store("host-run");
        let id = store.create_work("Door lock").unwrap().id;
        store.save_source(&id, "The lock opens.", None).unwrap();
        let (claude, author) = installed("host-run-bin");
        let mut settings = HostSettings::from_lookup("desktop", lookup(&[]));
        settings.claude = Some(claude);
        settings.author = Some(author);

        let host = start(Arc::clone(&store), Arc::new(FakeRenderer), settings);
        assert_eq!(host.not_hosted(), None);
        assert_eq!(host.client_version().as_deref(), Some("2.1.289"));
        // The shell says it hosts, and which client, where the screen reads it.
        let said = &store.host_status().unwrap().hosts[0];
        assert!(said.host.hosting);
        assert_eq!(said.host.client_version.as_deref(), Some("2.1.289"));
        let request = ask(&store, &id);
        within_ten_seconds("the request was not taken", || {
            store.read_request(&id, &request).unwrap().state == State::Completed
        });

        let model = store.read_model(&id, None).unwrap().expect("a model");
        assert!(model.text.contains("hosted"));
        let status = store.adapter_status().unwrap();
        assert_eq!(status.adapters.len(), 1);
        assert_eq!(status.adapters[0].adapter.name, "desktop");
        assert_eq!(status.adapters[0].adapter.kind, "claude-code");
        let started = Instant::now();
        drop(host);
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "dropping the host waited for the thread"
        );
    }

    #[test]
    fn a_shell_with_no_authoring_server_hosts_nothing_and_says_what_to_set() {
        let (_, store) = store("host-no-author");
        let (claude, _) = installed("host-no-author-bin");
        let mut settings = HostSettings::from_lookup("desktop", lookup(&[]));
        settings.claude = Some(claude);
        settings.author = Some(PathBuf::from("/nowhere/sce-author-mcp"));

        let host = start(store, Arc::new(FakeRenderer), settings);

        let Some(NotHosted::NoAuthorServer(tried)) = host.not_hosted() else {
            panic!("expected NoAuthorServer, got {:?}", host.not_hosted());
        };
        assert!(tried.contains("/nowhere/sce-author-mcp"), "{tried}");
        assert!(NotHosted::NoAuthorServer(tried)
            .to_string()
            .contains("SCE_AUTHOR_MCP"));
    }

    #[test]
    fn an_authoring_server_that_would_not_start_says_why_before_the_owner_finds_out_by_waiting() {
        let (_, store) = store("host-not-ready");
        let (claude, _) = installed("host-not-ready-bin");
        let dir = common::scratch("host-not-ready-launcher");
        let launcher = dir.join("sce-author-mcp");
        // What a launcher says when Python has no PyYAML, to the standard error, and stops.
        fs::write(
            &launcher,
            "#!/bin/sh\n[ \"$1\" = \"--check\" ] || exit 0\necho 'PyYAML is not installed for this Python: pip install pyyaml' >&2\nexit 1\n",
        )
        .unwrap();
        fs::set_permissions(&launcher, fs::Permissions::from_mode(0o755)).unwrap();
        let mut settings = HostSettings::from_lookup("desktop", lookup(&[]));
        settings.claude = Some(claude);
        settings.author = Some(launcher);

        let host = start(Arc::clone(&store), Arc::new(FakeRenderer), settings);

        let Some(NotHosted::AuthorServerNotReady(said)) = host.not_hosted() else {
            panic!("expected AuthorServerNotReady, got {:?}", host.not_hosted());
        };
        assert!(said.contains("pip install pyyaml"), "{said}");
        // And it is where the screen reads it.
        let reason = store.host_status().unwrap().hosts[0]
            .host
            .reason
            .clone()
            .unwrap();
        assert!(reason.contains("pip install pyyaml"), "{reason}");
        assert!(reason.contains("cannot start"), "{reason}");
    }

    #[test]
    fn the_server_is_asked_with_the_environment_the_client_will_give_it() {
        let (_, store) = store("host-check-env");
        let (claude, _) = installed("host-check-env-bin");
        let dir = common::scratch("host-check-env-launcher");
        let launcher = dir.join("sce-author-mcp");
        // Ready only when it is told where the works folder is: the folder of this store.
        fs::write(
            &launcher,
            "#!/bin/sh\n[ \"$1\" = \"--check\" ] || exit 0\n[ -n \"$SCE_WORKS_DIR\" ] || { echo 'no works folder' >&2; exit 1; }\nexit 0\n",
        )
        .unwrap();
        fs::set_permissions(&launcher, fs::Permissions::from_mode(0o755)).unwrap();
        let mut settings = HostSettings::from_lookup("desktop", lookup(&[]));
        settings.claude = Some(claude);
        settings.author = Some(launcher);

        let host = start(Arc::clone(&store), Arc::new(FakeRenderer), settings);

        assert_eq!(host.not_hosted(), None);
    }

    /// The same `claude`, and signed in by a subscription: it answers `auth status` as well. What it
    /// is asked there is how the host knows who is signed in.
    fn installed_signed_in(label: &str) -> (PathBuf, PathBuf) {
        installed_saying(
            label,
            r#"{"loggedIn":true,"authMethod":"claude.ai","apiProvider":"firstParty"}"#,
        )
    }

    /// The same `claude`, answering `auth status` with `status`.
    fn installed_saying(label: &str, status: &str) -> (PathBuf, PathBuf) {
        let (claude, author) = installed(label);
        let dir = claude.parent().unwrap().to_path_buf();
        common::write_program(
            &claude,
            &format!(
                "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo \"2.1.289 (Claude Code)\"; exit 0; fi\n\
                 if [ \"$3\" = \"auth\" ]; then echo '{status}'; exit 0; fi\n\
                 cat > /dev/null\ncat '{}'\n",
                dir.join("answer.json").display()
            ),
        );
        (claude, author)
    }

    #[test]
    fn a_host_says_which_request_it_could_not_run_and_why() {
        let (_, store) = store("host-waiting");
        let id = store.create_work("Door lock").unwrap().id;
        store.save_source(&id, "The lock opens.", None).unwrap();
        let (claude, author) = installed_saying(
            "host-waiting-bin",
            r#"{"loggedIn":false,"authMethod":"none","apiProvider":"firstParty"}"#,
        );
        let (settings_store, pin) = pinned("host-waiting-settings");
        let request = ask_for(&store, &id, Some(pin));
        let mut settings = HostSettings::from_lookup("desktop", lookup(&[]));
        settings.claude = Some(claude.clone());
        settings.author = Some(author);

        let host = start_with(
            Arc::clone(&store),
            Arc::new(FakeRenderer),
            settings,
            Some((settings_store, Policy::shipped())),
        );

        within_ten_seconds("the host did not say what it waits for", || {
            !store.host_status().unwrap().hosts[0]
                .host
                .waiting
                .is_empty()
        });
        let said = store.host_status().unwrap().hosts[0].host.waiting.clone();
        assert_eq!(said.len(), 1);
        assert_eq!(said[0].request, request);
        assert_eq!(said[0].work, id.as_str());
        assert_eq!(said[0].connection, "main");
        assert!(
            said[0].reason.contains("claude auth login"),
            "{}",
            said[0].reason
        );
        // The works folder is shared: it carries a sentence, and nothing of this computer.
        assert!(
            !said[0]
                .reason
                .contains(claude.parent().unwrap().to_str().unwrap()),
            "{}",
            said[0].reason
        );
        // And the request is still there to be taken, by this host once somebody signs in or by
        // another executor: it was not failed.
        assert_eq!(
            store.read_request(&id, &request).unwrap().state,
            State::Queued
        );
        drop(host);
    }

    #[test]
    fn a_host_stops_saying_it_waits_for_a_request_that_was_called_off() {
        let (_, store) = store("host-waiting-cleared");
        let id = store.create_work("Door lock").unwrap().id;
        store.save_source(&id, "The lock opens.", None).unwrap();
        let (claude, author) = installed_saying(
            "host-waiting-cleared-bin",
            r#"{"loggedIn":false,"authMethod":"none","apiProvider":"firstParty"}"#,
        );
        let (settings_store, pin) = pinned("host-waiting-cleared-settings");
        let request = ask_for(&store, &id, Some(pin));
        let mut settings = HostSettings::from_lookup("desktop", lookup(&[]));
        settings.claude = Some(claude);
        settings.author = Some(author);
        let host = start_with(
            Arc::clone(&store),
            Arc::new(FakeRenderer),
            settings,
            Some((settings_store, Policy::shipped())),
        );
        within_ten_seconds("the host did not say what it waits for", || {
            !store.host_status().unwrap().hosts[0]
                .host
                .waiting
                .is_empty()
        });

        store.cancel_request(&id, &request).unwrap();

        within_ten_seconds("the host went on saying it waits", || {
            store.host_status().unwrap().hosts[0]
                .host
                .waiting
                .is_empty()
        });
        drop(host);
    }

    /// A connection that is saved, and the pin a request made for it carries.
    fn pinned(label: &str) -> (ConnectionStore, Pin) {
        let settings = ConnectionStore::at(common::scratch(label));
        let connection = Connection {
            id: ConnectionId::parse("main").unwrap(),
            adapter: AdapterKind::ClaudeCode,
            display_name: None,
            executable: None,
            model: Some("opus".to_string()),
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

    fn ask_for(store: &WorkStore<Arc<ManualClock>>, id: &WorkId, pin: Option<Pin>) -> String {
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
                pin,
            )
            .unwrap()
            .request
            .id
    }

    #[test]
    fn a_host_that_has_the_settings_runs_a_request_made_for_a_connection() {
        let (_, store) = store("host-pinned");
        let id = store.create_work("Door lock").unwrap().id;
        store.save_source(&id, "The lock opens.", None).unwrap();
        let (claude, author) = installed_signed_in("host-pinned-bin");
        let (settings_store, pin) = pinned("host-pinned-settings");
        let request = ask_for(&store, &id, Some(pin));
        let mut settings = HostSettings::from_lookup("desktop", lookup(&[]));
        settings.claude = Some(claude);
        settings.author = Some(author);

        let host = start_with(
            Arc::clone(&store),
            Arc::new(FakeRenderer),
            settings,
            Some((settings_store, Policy::shipped())),
        );

        assert_eq!(host.not_hosted(), None);
        within_ten_seconds("the pinned request was not run", || {
            store.read_request(&id, &request).unwrap().state == State::Completed
        });
        assert!(store
            .read_model(&id, None)
            .unwrap()
            .unwrap()
            .text
            .contains("hosted"));
    }

    #[test]
    fn a_host_that_has_no_settings_leaves_a_request_made_for_a_connection_alone() {
        let (_, store) = store("host-pinned-alone");
        let id = store.create_work("Door lock").unwrap().id;
        store.save_source(&id, "The lock opens.", None).unwrap();
        let (claude, author) = installed_signed_in("host-pinned-alone-bin");
        let (_, pin) = pinned("host-pinned-alone-settings");
        let request = ask_for(&store, &id, Some(pin));
        let mut settings = HostSettings::from_lookup("desktop", lookup(&[]));
        settings.claude = Some(claude);
        settings.author = Some(author);

        let host = start(Arc::clone(&store), Arc::new(FakeRenderer), settings);
        std::thread::sleep(Duration::from_millis(700));

        // The executor is there, and the request is not its to take: it was made for a
        // connection, and an executor that runs for none does not answer it.
        assert_eq!(host.not_hosted(), None);
        let view = store.read_request(&id, &request).unwrap();
        assert_eq!((view.state, view.request.attempt), (State::Queued, 0));
    }

    #[test]
    fn a_host_that_has_the_settings_leaves_a_request_nobody_chose_a_connection_for() {
        let (_, store) = store("host-unpinned-left");
        let id = store.create_work("Door lock").unwrap().id;
        store.save_source(&id, "The lock opens.", None).unwrap();
        let (claude, author) = installed_signed_in("host-unpinned-left-bin");
        let (settings_store, _) = pinned("host-unpinned-left-settings");
        let request = ask(&store, &id);
        let mut settings = HostSettings::from_lookup("desktop", lookup(&[]));
        settings.claude = Some(claude);
        settings.author = Some(author);

        let host = start_with(
            Arc::clone(&store),
            Arc::new(FakeRenderer),
            settings,
            Some((settings_store, Policy::shipped())),
        );
        std::thread::sleep(Duration::from_millis(700));

        // The executor is there, and which AI such a request was meant for is not for it to
        // guess: an authoring client of the person's own takes it, or the person chooses.
        assert_eq!(host.not_hosted(), None);
        let view = store.read_request(&id, &request).unwrap();
        assert_eq!((view.state, view.request.attempt), (State::Queued, 0));
        assert!(store.host_status().unwrap().hosts[0]
            .host
            .waiting
            .is_empty());
    }

    #[test]
    fn a_host_started_before_claude_was_installed_hosts_once_it_is() {
        let (_, store) = store("host-late");
        let id = store.create_work("Door lock").unwrap().id;
        store.save_source(&id, "The lock opens.", None).unwrap();
        let (claude, author) = installed_signed_in("host-late-bin");
        // Not there yet: it is put where the host looks after the window is open.
        let later = claude.with_extension("later");
        fs::rename(&claude, &later).unwrap();
        let (settings_store, pin) = pinned("host-late-settings");
        let request = ask_for(&store, &id, Some(pin));
        let mut settings = HostSettings::from_lookup("desktop", lookup(&[]));
        settings.claude = Some(claude.clone());
        settings.author = Some(author);
        settings.retry = Duration::from_millis(50);

        let host = start_with(
            Arc::clone(&store),
            Arc::new(FakeRenderer),
            settings,
            Some((settings_store, Policy::shipped())),
        );
        assert!(matches!(host.not_hosted(), Some(NotHosted::NoClient(_))));
        assert!(!store.host_status().unwrap().hosts[0].host.hosting);
        fs::rename(&later, &claude).unwrap();

        within_ten_seconds("the host did not start once Claude was installed", || {
            host.not_hosted().is_none()
        });
        within_ten_seconds("the request was not run once it did", || {
            store.read_request(&id, &request).unwrap().state == State::Completed
        });
        // And where the screen reads, it hosts now, and says which client.
        within_ten_seconds("the host did not say it hosts", || {
            store.host_status().unwrap().hosts[0].host.hosting
        });
        assert_eq!(host.client_version().as_deref(), Some("2.1.289"));
        drop(host);
    }

    /// A `claude` signed in by a subscription that writes the arguments of the run that asked it
    /// for a draft, one to a line, to `run-argv` beside it.
    fn installed_recording(label: &str) -> (PathBuf, PathBuf, PathBuf) {
        let (claude, author) = installed(label);
        let dir = claude.parent().unwrap().to_path_buf();
        let record = dir.join("run-argv");
        common::write_program(
            &claude,
            &format!(
                "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo \"2.1.289 (Claude Code)\"; exit 0; fi\n\
                 if [ \"$3\" = \"auth\" ]; then echo '{{\"loggedIn\":true,\"authMethod\":\"claude.ai\",\"apiProvider\":\"firstParty\"}}'; exit 0; fi\n\
                 for a in \"$@\"; do printf '%s\\n' \"$a\"; done > '{}'\ncat > /dev/null\ncat '{}'\n",
                record.display(),
                dir.join("answer.json").display()
            ),
        );
        (claude, author, record)
    }

    #[test]
    fn what_the_environment_may_spend_bounds_the_run_made_for_a_connection() {
        let (_, store) = store("host-budget");
        let id = store.create_work("Door lock").unwrap().id;
        store.save_source(&id, "The lock opens.", None).unwrap();
        let (claude, author, record) = installed_recording("host-budget-bin");
        let (settings_store, pin) = pinned("host-budget-settings");
        let request = ask_for(&store, &id, Some(pin));
        let mut settings =
            HostSettings::from_lookup("desktop", lookup(&[("SCE_CLAUDE_BUDGET_USD", "2.5")]));
        settings.claude = Some(claude);
        settings.author = Some(author);

        let host = start_with(
            Arc::clone(&store),
            Arc::new(FakeRenderer),
            settings,
            Some((settings_store, Policy::shipped())),
        );
        within_ten_seconds("the pinned request was not run", || {
            store.read_request(&id, &request).unwrap().state == State::Completed
        });
        drop(host);

        let argv = fs::read_to_string(&record).unwrap();
        let args: Vec<&str> = argv.lines().collect();
        let at = args.iter().position(|a| *a == "--max-budget-usd");
        assert_eq!(at.map(|i| args[i + 1]), Some("2.5"), "{args:?}");
    }

    fn ask(store: &WorkStore<Arc<ManualClock>>, id: &WorkId) -> String {
        store
            .register_request(
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
            )
            .unwrap()
            .request
            .id
    }
}
