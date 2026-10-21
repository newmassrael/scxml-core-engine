// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The table of `sce_rust_scenarios::machines` against the scenarios it is for,
// and every scenario of it replayed on the host this runs on — the answer the
// wasm32 build of the same table is held to.
//
// The members a scenario may carry, and the rules between them, are the format's
// and not this driver's: `sce_build::scenario_replay` holds them, `sce-codegen
// scenarios` checks a file against them, and
// `sce-build/tests/every_replay_scenario_keeps_to_its_vocabulary.rs` holds every
// committed scenario to them.

use sce_rust_scenarios::machines::{run, scenario_text, NAMES, NOT_YET_LOWERED};
use std::collections::BTreeSet;
use std::path::Path;

fn scenario_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../sce-build/tests/fixtures/static_datamodel/scenarios")
}

/// A scenario file the table does not name is a scenario this target never
/// replays, and a name the directory does not hold is a row that reads nothing.
/// The directory is the list; the table is checked against it, not the other way.
/// A scenario whose machine Rust does not lower yet is named by `NOT_YET_LOWERED`
/// instead, with why, and never by both.
#[test]
fn the_table_names_every_scenario_file_and_only_those() {
    let on_disk: BTreeSet<String> = std::fs::read_dir(scenario_dir())
        .expect("the scenario directory is readable")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .map(|path| {
            path.file_stem()
                .expect("a file stem")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert!(
        on_disk.len() >= 40,
        "the scenario directory held {} files: a test that read none would pass",
        on_disk.len()
    );
    let named: BTreeSet<String> = NAMES.iter().map(|name| (*name).to_string()).collect();
    assert_eq!(
        named.len(),
        NAMES.len(),
        "a scenario is named by two rows of the table"
    );
    let pending: BTreeSet<String> = NOT_YET_LOWERED
        .iter()
        .map(|(name, _)| (*name).to_string())
        .collect();
    let both: Vec<&String> = named.intersection(&pending).collect();
    assert!(
        both.is_empty(),
        "scenarios that are rows and not yet lowered at once: {both:?}"
    );
    let accounted: BTreeSet<String> = named.union(&pending).cloned().collect();
    let unnamed: Vec<&String> = on_disk.difference(&accounted).collect();
    let unread: Vec<&String> = accounted.difference(&on_disk).collect();
    assert!(
        unnamed.is_empty() && unread.is_empty(),
        "scenario files the table neither names nor lists as not yet lowered: {unnamed:?}; \
         rows or entries with no file: {unread:?}"
    );
}

/// A scenario not yet lowered is no row: the table holds no text of it and refuses
/// to replay it, and the entry says why.
#[test]
fn a_scenario_not_yet_lowered_is_no_row_of_the_table() {
    for (name, why) in NOT_YET_LOWERED {
        assert!(
            scenario_text(name).is_none(),
            "`{name}` is a row of the table"
        );
        assert!(!why.trim().is_empty(), "`{name}` is listed with no reason");
        let refused = std::panic::catch_unwind(|| run(name));
        assert!(refused.is_err(), "`{name}` was replayed by the table");
    }
}

#[test]
fn every_scenario_text_is_the_file_it_is_named_for() {
    for name in NAMES {
        let on_disk = std::fs::read_to_string(scenario_dir().join(format!("{name}.json")))
            .expect("the scenario file is readable");
        assert_eq!(
            scenario_text(name),
            Some(on_disk.as_str()),
            "the text compiled in for `{name}` is not the file's"
        );
    }
}

/// One failure names every scenario that failed and the step, where a loop of
/// `run` would stop at the first.
#[test]
fn every_scenario_replays() {
    let mut failed = Vec::new();
    for name in NAMES {
        let outcome = std::panic::catch_unwind(|| run(name));
        if let Err(cause) = outcome {
            let message = cause
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| cause.downcast_ref::<&str>().map(|s| (*s).to_string()))
                .unwrap_or_else(|| "a panic with no message".to_string());
            failed.push(format!("{name}: {message}"));
        }
    }
    assert!(
        failed.is_empty(),
        "{} scenario(s) failed:\n{}",
        failed.len(),
        failed.join("\n")
    );
}

#[test]
#[should_panic(expected = "is not a scenario of this table")]
fn a_name_the_table_does_not_hold_is_refused() {
    run("no_such_scenario");
}
