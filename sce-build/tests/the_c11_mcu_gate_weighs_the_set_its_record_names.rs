// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The `c11-mcu` gate weighs and runs the set of machines docs/adr/0110 names
//! (D3), and its budget holds a figure for each of them and for nothing else.
//!
//! Three places name the set: the record, the gate's `C11_MCU_SET`, and the
//! budget's `<MACHINE>_<FIGURE>` lines. A machine the record names and the gate
//! does not is a claim nothing measures; one the gate weighs and the budget does
//! not is a figure no budget holds (the gate refuses it as unmeasured, which is
//! loud, but only once it runs); one the budget holds and the gate no longer
//! weighs is a figure nothing reads. Each is read here from the file itself.
//!
//! The one scenario the gate replays that no other lane does, the footprint
//! probe's (`backends/c/mcu/scenarios/plain_footprint_probe.json`), is held to the
//! vocabulary and to its machine as every committed scenario is, so the table the
//! gate generates from it is one the generator accepts.

use std::collections::BTreeSet;
use std::path::PathBuf;

const GATE: &str = include_str!("../../scripts/gates/c11-mcu.sh");
const BUDGET: &str = include_str!("../../backends/c/mcu/footprint.budget");
const PROBE_SCENARIO: &str =
    include_str!("../../backends/c/mcu/scenarios/plain_footprint_probe.json");
const RECORD: &str = include_str!(
    "../../docs/adr/0110-a-c11-machine-core-is-freestanding-and-a-gate-weighs-and-runs-it-on-an-mcu.md"
);

/// The four figures the budget holds for each machine.
const FIGURES: &[&str] = &["TEXT", "RODATA", "DATA", "BSS"];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// The entries of the gate's `C11_MCU_SET=( ... )`, each split on `|`.
fn gate_entries() -> Vec<Vec<String>> {
    let mut inside = false;
    let mut entries = Vec::new();
    for line in GATE.lines() {
        let line = line.trim();
        if line == "C11_MCU_SET=(" {
            inside = true;
            continue;
        }
        if inside && line == ")" {
            break;
        }
        if inside {
            let entry = line.trim_matches('"');
            entries.push(entry.split('|').map(str::to_string).collect());
        }
    }
    entries
}

/// The machines the record's D3 names: the first backticked name of each bullet
/// of the list that follows "The set,", by its last path segment.
fn record_set() -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut inside = false;
    for line in RECORD.lines() {
        if line.starts_with("The set,") {
            inside = true;
            continue;
        }
        if !inside {
            continue;
        }
        if let Some(bullet) = line.strip_prefix("- ") {
            let name = bullet
                .split('`')
                .nth(1)
                .unwrap_or_else(|| panic!("a bullet of D3's set names no machine: {line}"));
            names.insert(name.rsplit('/').next().unwrap_or(name).to_string());
        } else if line.starts_with(char::is_alphabetic) && !names.is_empty() {
            break;
        }
    }
    names
}

/// The machines the budget holds a figure for, lower-cased, each with the
/// figures it holds.
fn budget_set() -> Vec<(String, String)> {
    BUDGET
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .filter_map(|(name, _)| {
            FIGURES.iter().find_map(|figure| {
                name.strip_suffix(&format!("_{figure}"))
                    .map(|machine| (machine.to_lowercase(), (*figure).to_string()))
            })
        })
        .collect()
}

#[test]
fn the_gate_the_budget_and_the_record_name_one_set() {
    let record = record_set();
    assert_eq!(
        record.len(),
        6,
        "D3 of docs/adr/0110 names six units after \"The set,\"; the reader found {record:?}"
    );
    let gate: BTreeSet<String> = gate_entries()
        .iter()
        .map(|entry| entry[0].clone())
        .collect();
    assert_eq!(
        gate, record,
        "scripts/gates/c11-mcu.sh's C11_MCU_SET and the set docs/adr/0110 D3 names differ"
    );

    let figures = budget_set();
    let budgeted: BTreeSet<String> = figures.iter().map(|(m, _)| m.clone()).collect();
    assert_eq!(
        budgeted, gate,
        "backends/c/mcu/footprint.budget holds figures for another set than the gate weighs"
    );
    for machine in &budgeted {
        for figure in FIGURES {
            assert!(
                figures
                    .iter()
                    .any(|(m, f)| m == machine && f.as_str() == *figure),
                "the budget holds no {figure} of {machine}"
            );
        }
    }
}

#[test]
fn every_document_and_scenario_the_gate_names_is_in_the_tree() {
    let entries = gate_entries();
    assert!(entries.len() >= 6, "the gate's set read as {entries:?}");
    for entry in &entries {
        assert_eq!(
            entry.len(),
            4,
            "an entry is name|kind|document|scenario: {entry:?}"
        );
        let kind = entry[1].as_str();
        assert!(
            kind == "machine" || kind == "codec",
            "an entry's kind is `machine` or `codec`: {entry:?}"
        );
        assert!(
            repo_root().join(&entry[2]).is_file(),
            "{} names the document {}, which is not in the tree",
            entry[0],
            entry[2]
        );
        if kind == "machine" {
            assert!(
                repo_root().join(&entry[3]).is_file(),
                "{} names the scenario {}, which is not in the tree",
                entry[0],
                entry[3]
            );
        } else {
            assert!(
                entry[3].is_empty(),
                "a codec replays no scenario: {entry:?}"
            );
        }
    }
}

#[test]
fn the_footprint_probes_scenario_is_usable_against_its_machine() {
    use sce_build::scenario_replay::{machine_problems, problems, MachineFacts};
    let document = repo_root().join("sce-build/tests/fixtures/no_std/plain_footprint_probe.scxml");
    let model = sce_build::parser::SCXMLParser::new()
        .parse_file(document.to_str().expect("a UTF-8 path"))
        .expect("the probe parses");
    let facts = MachineFacts::of(&model, "plain_footprint_probe").expect("its facts read");
    let scenario: serde_json::Value =
        serde_json::from_str(PROBE_SCENARIO).expect("the scenario is JSON");
    let mut found = problems(&scenario);
    found.extend(machine_problems(&scenario, &facts));
    assert!(
        found.is_empty(),
        "the footprint probe's scenario has problems: {:?}",
        found
            .iter()
            .map(|p| format!("{} {}: {}", p.path, p.code, p.detail))
            .collect::<Vec<_>>()
    );
}
