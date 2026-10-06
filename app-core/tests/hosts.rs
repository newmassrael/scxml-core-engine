// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Whether a shell hosts an executor, and if it does not, why.
//!
//! An executor is hosted by a shell (the desktop application, the browser shell) when it can
//! find Claude Code and the authoring server, and not otherwise. When it is not, the owner has
//! to be told what to do about it where they look, which is the screen: a message on the
//! standard error of a program that was started from a menu is a message nobody reads. A shell
//! says what it is doing the way an adapter does, by reporting, and the record is there for as
//! long as its last report is recent, so that a shell that has gone is not read as one that is
//! hosting.

mod common;

use std::sync::Arc;

use sce_app_core::{HostReport, HostWaiting, ManualClock, WorkStore, ADAPTER_LIVE_SECONDS};

const T0: u64 = 1_791_190_800;

fn waiting(request: &str, reason: &str) -> HostWaiting {
    HostWaiting {
        work: "door".to_string(),
        request: request.to_string(),
        connection: "claude".to_string(),
        reason: reason.to_string(),
    }
}

fn store(label: &str) -> (Arc<ManualClock>, WorkStore<Arc<ManualClock>>) {
    let clock = Arc::new(ManualClock::at(T0));
    let store = WorkStore::with_clock(common::scratch(label), Arc::clone(&clock));
    (clock, store)
}

fn hosting(name: &str) -> HostReport<'_> {
    HostReport {
        name,
        hosting: true,
        reason: None,
        client_version: Some("2.1.289"),
        waiting: &[],
    }
}

fn not_hosting<'a>(name: &'a str, reason: &'a str) -> HostReport<'a> {
    HostReport {
        name,
        hosting: false,
        reason: Some(reason),
        client_version: None,
        waiting: &[],
    }
}

#[test]
fn a_shell_says_which_requests_it_could_not_run_and_why() {
    let (_, store) = store("hosts-waiting");
    let left = [
        waiting("req-1", "nobody is signed in to Claude Code"),
        waiting(
            "req-2",
            "this build has no adapter for `codex` connections yet",
        ),
    ];

    store
        .report_host(HostReport {
            waiting: &left,
            ..hosting("desktop")
        })
        .unwrap();

    let listing = store.host_status().unwrap();
    assert_eq!(listing.hosts[0].host.waiting, left);
}

#[test]
fn what_a_shell_waits_for_is_what_it_said_last() {
    let (_, store) = store("hosts-waiting-replaced");
    store
        .report_host(HostReport {
            waiting: &[waiting("req-1", "nobody is signed in")],
            ..hosting("desktop")
        })
        .unwrap();

    store.report_host(hosting("desktop")).unwrap();

    assert!(store.host_status().unwrap().hosts[0]
        .host
        .waiting
        .is_empty());
}

#[test]
fn a_record_from_before_a_shell_said_what_it_waited_for_says_nothing_is() {
    let (_, store) = store("hosts-waiting-older");
    store.report_host(hosting("desktop")).unwrap();
    let path = store.root().join(".sce-hosts").join("desktop.json");
    let mut record: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    record.as_object_mut().unwrap().remove("waiting");
    std::fs::write(&path, record.to_string()).unwrap();

    let listing = store.host_status().unwrap();

    assert_eq!(listing.hosts.len(), 1);
    assert!(listing.hosts[0].host.waiting.is_empty());
}

#[test]
fn what_a_shell_waits_for_is_a_few_short_sentences_and_not_a_log() {
    let (_, store) = store("hosts-waiting-bad");
    let many: Vec<HostWaiting> = (0..21)
        .map(|i| waiting(&format!("req-{i}"), "no"))
        .collect();
    for left in [
        vec![waiting("req-1", "")],
        vec![waiting("req-1", &"x".repeat(501))],
        vec![waiting("", "nobody is signed in")],
        vec![HostWaiting {
            connection: "../x".to_string(),
            ..waiting("req-1", "nobody is signed in")
        }],
        many,
    ] {
        let refused = store
            .report_host(HostReport {
                waiting: &left,
                ..hosting("desktop")
            })
            .unwrap_err();
        assert_eq!(refused.kind(), "bad-host", "{left:?}");
    }
    assert!(store.host_status().unwrap().hosts.is_empty());
}

#[test]
fn nothing_is_said_of_a_shell_that_never_reported() {
    let (_, store) = store("hosts-none");

    let listing = store.host_status().unwrap();

    assert!(listing.hosts.is_empty());
    assert!(listing.unreadable.is_empty());
}

#[test]
fn a_shell_that_hosts_says_so_and_which_client() {
    let (_, store) = store("hosts-running");

    let seen = store.report_host(hosting("desktop")).unwrap();

    assert!(seen.live);
    assert!(seen.host.hosting);
    assert_eq!(seen.host.name, "desktop");
    assert_eq!(seen.host.client_version.as_deref(), Some("2.1.289"));
    assert_eq!(seen.host.reason, None);
    assert_eq!(seen.host.seen_at, "2026-10-05T09:00:00Z");
    assert_eq!(store.host_status().unwrap().hosts, vec![seen]);
}

#[test]
fn a_shell_that_does_not_host_says_why_in_words_the_owner_can_act_on() {
    let (_, store) = store("hosts-not-running");
    let why = "no Claude Code to write models with: install it, or set SCE_CLAUDE to its path";

    store.report_host(not_hosting("desktop", why)).unwrap();

    let listing = store.host_status().unwrap();
    assert_eq!(listing.hosts.len(), 1);
    assert!(!listing.hosts[0].host.hosting);
    assert_eq!(listing.hosts[0].host.reason.as_deref(), Some(why));
}

#[test]
fn what_a_shell_says_now_replaces_what_it_said_before() {
    let (clock, store) = store("hosts-replace");
    store
        .report_host(not_hosting("desktop", "no Claude Code"))
        .unwrap();
    clock.advance(30);

    store.report_host(hosting("desktop")).unwrap();

    let listing = store.host_status().unwrap();
    assert_eq!(listing.hosts.len(), 1);
    assert!(listing.hosts[0].host.hosting);
    assert_eq!(listing.hosts[0].host.reason, None);
    assert_eq!(listing.hosts[0].host.seen_at, "2026-10-05T09:00:30Z");
}

#[test]
fn a_shell_that_stopped_reporting_is_not_there_but_what_it_last_said_is_kept() {
    let (clock, store) = store("hosts-stale");
    store.report_host(hosting("desktop")).unwrap();

    clock.advance(ADAPTER_LIVE_SECONDS - 1);
    assert!(store.host_status().unwrap().hosts[0].live);
    clock.advance(1);
    let listing = store.host_status().unwrap();

    assert!(!listing.hosts[0].live, "a shell that went is not hosting");
    assert!(
        listing.hosts[0].host.hosting,
        "it said so, and that is on record"
    );
}

#[test]
fn shells_are_listed_by_name_and_each_is_one_record() {
    let (_, store) = store("hosts-two");
    store.report_host(hosting("web-shell")).unwrap();
    store
        .report_host(not_hosting("desktop", "no Claude Code"))
        .unwrap();

    let names: Vec<String> = store
        .host_status()
        .unwrap()
        .hosts
        .into_iter()
        .map(|h| h.host.name)
        .collect();

    assert_eq!(names, vec!["desktop", "web-shell"]);
}

#[test]
fn a_report_that_is_not_one_is_refused() {
    let (_, store) = store("hosts-bad");
    for report in [
        hosting("../x"),
        hosting(""),
        not_hosting("desktop", ""),
        not_hosting("desktop", &"x".repeat(2_001)),
        HostReport {
            name: "desktop",
            hosting: true,
            reason: None,
            client_version: Some(&"v".repeat(65)),
            waiting: &[],
        },
    ] {
        let refused = store.report_host(report.clone()).unwrap_err();
        assert_eq!(refused.kind(), "bad-host", "{report:?}");
    }
    assert!(store.host_status().unwrap().hosts.is_empty());
}

#[test]
fn a_record_that_cannot_be_read_is_listed_as_such_and_hides_no_other() {
    let (_, store) = store("hosts-unreadable");
    store.report_host(hosting("desktop")).unwrap();
    let folder = store.root().join(".sce-hosts");
    std::fs::write(folder.join("broken.json"), "not json").unwrap();
    // What a half-written replacement leaves beside a record is not a record.
    std::fs::write(folder.join("desktop.json.tmp"), "{").unwrap();

    let listing = store.host_status().unwrap();

    assert_eq!(listing.hosts.len(), 1);
    assert_eq!(listing.unreadable.len(), 1);
    assert_eq!(listing.unreadable[0].id, "broken");
}
