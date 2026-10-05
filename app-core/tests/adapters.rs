// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Which AI adapters are there.
//!
//! An adapter says it is there by reporting, and it is there for as long as its last report
//! is recent. That is soft state beside the works, not a fact about any work: the screen
//! reads it to say "no AI is connected" and to offer what the connected one can do, and it
//! must never say an adapter is connected because one was, once.

mod common;

use std::sync::Arc;

use sce_app_core::{AdapterReport, ManualClock, WorkStore, ADAPTER_LIVE_SECONDS};

const T0: u64 = 1_791_190_800;

fn store(label: &str) -> (Arc<ManualClock>, WorkStore<Arc<ManualClock>>) {
    let clock = Arc::new(ManualClock::at(T0));
    let store = WorkStore::with_clock(common::scratch(label), Arc::clone(&clock));
    (clock, store)
}

fn report<'a>(name: &'a str, capabilities: &'a [&'a str]) -> AdapterReport<'a> {
    AdapterReport {
        name,
        kind: "claude-code",
        capabilities: capabilities.iter().map(|c| c.to_string()).collect(),
        version: Some("2.1"),
    }
}

#[test]
fn nobody_is_connected_until_an_adapter_reports() {
    let (_, store) = store("adapters-none");

    let listing = store.adapter_status().unwrap();

    assert!(listing.adapters.is_empty());
    assert!(listing.unreadable.is_empty());
}

#[test]
fn an_adapter_that_reports_is_connected_and_says_what_it_can_do() {
    let (_, store) = store("adapters-report");

    let seen = store
        .report_adapter(report("desktop", &["generate", "cancel"]))
        .unwrap();

    assert!(seen.live);
    assert_eq!(seen.adapter.name, "desktop");
    assert_eq!(seen.adapter.kind, "claude-code");
    assert_eq!(seen.adapter.capabilities, vec!["generate", "cancel"]);
    assert_eq!(seen.adapter.seen_at, "2026-10-05T09:00:00Z");
    assert_eq!(store.adapter_status().unwrap().adapters, vec![seen]);
}

#[test]
fn an_adapter_is_connected_for_as_long_as_its_last_report_is_recent() {
    let (clock, store) = store("adapters-expiry");
    store
        .report_adapter(report("desktop", &["generate"]))
        .unwrap();

    clock.advance(ADAPTER_LIVE_SECONDS - 1);
    assert!(store.adapter_status().unwrap().adapters[0].live);
    clock.advance(1);

    let gone = &store.adapter_status().unwrap().adapters[0];
    assert!(!gone.live, "the record stays, and says nobody is there");
    assert_eq!(gone.adapter.seen_at, "2026-10-05T09:00:00Z");

    // Reporting again is being there again.
    store
        .report_adapter(report("desktop", &["generate"]))
        .unwrap();
    assert!(store.adapter_status().unwrap().adapters[0].live);
}

#[test]
fn a_report_replaces_what_the_adapter_said_before() {
    let (clock, store) = store("adapters-replace");
    store
        .report_adapter(report("desktop", &["generate", "cancel", "resume"]))
        .unwrap();
    clock.advance(10);

    store
        .report_adapter(report("desktop", &["generate"]))
        .unwrap();

    let listing = store.adapter_status().unwrap();
    assert_eq!(listing.adapters.len(), 1);
    assert_eq!(listing.adapters[0].adapter.capabilities, vec!["generate"]);
    assert_eq!(listing.adapters[0].adapter.seen_at, "2026-10-05T09:00:10Z");
}

#[test]
fn adapters_are_listed_by_name() {
    let (_, store) = store("adapters-order");
    for name in ["zebra", "alpha", "mid"] {
        store.report_adapter(report(name, &["generate"])).unwrap();
    }

    let names: Vec<String> = store
        .adapter_status()
        .unwrap()
        .adapters
        .into_iter()
        .map(|a| a.adapter.name)
        .collect();

    assert_eq!(names, vec!["alpha", "mid", "zebra"]);
}

#[test]
fn a_name_a_capability_or_a_kind_that_cannot_name_a_file_is_refused() {
    let (_, store) = store("adapters-names");

    for name in ["", "has space", "../x", "a/b", &"a".repeat(65)] {
        let error = store
            .report_adapter(report(name, &["generate"]))
            .unwrap_err();
        assert_eq!(error.kind(), "bad-adapter", "{name:?}");
    }
    let error = store
        .report_adapter(report("ok", &["no good"]))
        .unwrap_err();
    assert_eq!(error.kind(), "bad-adapter");
    let error = store
        .report_adapter(AdapterReport {
            kind: "",
            ..report("ok", &["generate"])
        })
        .unwrap_err();
    assert_eq!(error.kind(), "bad-adapter");
    let many: Vec<String> = (0..17).map(|i| format!("c{i}")).collect();
    let error = store
        .report_adapter(AdapterReport {
            capabilities: many,
            ..report("ok", &[])
        })
        .unwrap_err();
    assert_eq!(error.kind(), "bad-adapter");
    assert!(store.adapter_status().unwrap().adapters.is_empty());
}

#[test]
fn a_record_that_is_not_one_is_listed_as_unreadable_and_does_not_hide_the_others() {
    let (_, store) = store("adapters-corrupt");
    store.report_adapter(report("good", &["generate"])).unwrap();
    let folder = store.root().join(".sce-adapters");
    std::fs::write(folder.join("broken.json"), "{ not json").unwrap();

    let listing = store.adapter_status().unwrap();

    assert_eq!(listing.adapters.len(), 1);
    assert_eq!(listing.adapters[0].adapter.name, "good");
    assert_eq!(listing.unreadable.len(), 1);
    assert_eq!(listing.unreadable[0].id, "broken");
}

#[test]
fn the_adapters_folder_is_not_a_work_and_is_not_listed_as_one() {
    let (_, store) = store("adapters-not-a-work");
    store
        .report_adapter(report("desktop", &["generate"]))
        .unwrap();
    store.create_work("Door lock").unwrap();

    let listing = store.list_works().unwrap();

    assert_eq!(listing.works.len(), 1);
    assert!(listing.unreadable.is_empty());
}
