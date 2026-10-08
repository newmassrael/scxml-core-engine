// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The table of `sce_rust_scenarios::machines` against the scenarios it is for,
// and every scenario of it replayed on the host this runs on — the answer the
// wasm32 build of the same table is held to.

use sce_rust_scenarios::machines::{run, scenario_text, NAMES};
use std::collections::BTreeSet;
use std::path::Path;

fn scenario_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../sce-build/tests/fixtures/static_datamodel/scenarios")
}

/// A scenario file the table does not name is a scenario this target never
/// replays, and a name the directory does not hold is a row that reads nothing.
/// The directory is the list; the table is checked against it, not the other way.
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
    let unnamed: Vec<&String> = on_disk.difference(&named).collect();
    let unread: Vec<&String> = named.difference(&on_disk).collect();
    assert!(
        unnamed.is_empty() && unread.is_empty(),
        "scenario files the table does not name: {unnamed:?}; rows with no file: {unread:?}"
    );
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
