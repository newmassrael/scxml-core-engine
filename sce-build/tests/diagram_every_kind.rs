// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Every kind of document `sce-codegen diagram` is given is drawn.
//!
//! # What this holds
//!
//! A kind that is not a statechart is drawn as its own picture beside the
//! table of every value it states. Which kinds have a picture, and what each
//! picture is called, is a decision; this test is where it is written down,
//! on the real binary, for every kind the parser knows:
//!
//! - the kinds listed here are exactly `ForgeKind::ALL_ATTR_NAMES`, so a kind
//!   added to the model fails this test until someone has said what its
//!   picture is — or that it has none, and why;
//! - each kind's example is drawn in both page languages, every file
//!   written is well-formed SVG, the files are the pictures this table
//!   names followed by the field table, and a second run writes the same
//!   bytes (`--assert-unchanged` agrees).

use sce_build::forge::model::ForgeKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// What each kind is drawn as, besides its field table: the files written
/// before `fields-<n>.svg`, in order. Empty for a kind whose reading is the
/// pseudocode page and the table alone.
const PICTURES: &[(&str, &[&str])] = &[
    // An algorithm is steps in order. Its reading is the pseudocode page,
    // where every step has its words, and the field table; a flowchart of
    // them is a second layout for one kind and says nothing the page does
    // not.
    ("algorithm", &[]),
    ("bounded-collection", &["slots.svg"]),
    ("buffer-pool", &["slots.svg"]),
    ("codec", &["layout.svg"]),
    ("condition", &["dataflow.svg"]),
    ("enum", &["mapping.svg"]),
    ("event-schema", &["mapping.svg"]),
    ("filter", &["dataflow.svg"]),
    ("interpolation", &["curve.svg"]),
    ("link", &["structure.svg"]),
    ("lookup", &["mapping.svg"]),
    ("observer", &["thresholds.svg"]),
    // A procedure is the statechart figure of its states.
    ("procedure", &["document.svg"]),
    ("timer", &["timeline.svg"]),
    ("transform", &["dataflow.svg"]),
    ("validator", &["dataflow.svg"]),
    ("worker", &["structure.svg"]),
];

fn scratch(label: &str) -> PathBuf {
    let dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

fn example(kind: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("kind-examples")
        .join(format!("{kind}.scxml"))
}

fn diagram(doc: &Path, out: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .args(["--error-format", "json", "diagram"])
        .arg(doc)
        .arg("-o")
        .arg(out)
        .args(extra)
        .output()
        .expect("run sce-codegen diagram")
}

fn written(run: &Output) -> Vec<PathBuf> {
    String::from_utf8(run.stdout.clone())
        .expect("utf-8")
        .lines()
        .map(PathBuf::from)
        .collect()
}

fn names(files: &[PathBuf]) -> Vec<String> {
    files
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect()
}

/// The kinds named here are the kinds the parser knows, once each, apart
/// from the statechart, which is drawn as its figures.
#[test]
fn the_table_names_every_kind_the_parser_knows() {
    let mut listed: Vec<&str> = PICTURES.iter().map(|(k, _)| *k).collect();
    listed.push("statechart");
    listed.sort_unstable();
    let mut known: Vec<&str> = ForgeKind::ALL_ATTR_NAMES.to_vec();
    known.sort_unstable();
    assert_eq!(
        listed, known,
        "a kind added to the model needs its picture decided here"
    );
}

/// Each example is drawn in both languages: the pictures this table names,
/// then the field table; every file is well-formed SVG; a second run is
/// the same bytes.
#[test]
fn every_kind_example_is_drawn_as_its_pictures_and_its_table() {
    let dir = scratch("diagram-every-kind");
    for (kind, pictures) in PICTURES {
        for lexicon in ["en", "ko"] {
            let out = dir.join(format!("{kind}-{lexicon}"));
            let run = diagram(&example(kind), &out, &["--lexicon", lexicon]);
            assert!(
                run.status.success(),
                "{kind} ({lexicon}): {}",
                String::from_utf8_lossy(&run.stderr)
            );
            let files = written(&run);
            let got = names(&files);
            let (leading, tables) = got.split_at(pictures.len().min(got.len()));
            assert_eq!(leading, *pictures, "{kind} ({lexicon}): {got:?}");
            assert!(
                !tables.is_empty(),
                "{kind}: the field table is always written"
            );
            for (n, name) in tables.iter().enumerate() {
                assert_eq!(*name, format!("fields-{}.svg", n + 1), "{kind}: {got:?}");
            }
            for path in &files {
                let svg = std::fs::read_to_string(path).expect("read svg");
                roxmltree::Document::parse(&svg).unwrap_or_else(|e| panic!("{kind} {path:?}: {e}"));
            }

            let again = diagram(
                &example(kind),
                &out,
                &["--lexicon", lexicon, "--assert-unchanged"],
            );
            assert!(
                again.status.success(),
                "{kind} ({lexicon}) is not the same bytes twice: {}",
                String::from_utf8_lossy(&again.stderr)
            );
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The statechart example is still drawn as its figures: the document's
/// own figure first, and no field table.
#[test]
fn the_statechart_example_is_still_drawn_as_figures() {
    let dir = scratch("diagram-every-kind-statechart");
    let out = dir.join("figures");
    let run = diagram(&example("statechart"), &out, &[]);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let got = names(&written(&run));
    assert_eq!(
        got.first().map(String::as_str),
        Some("document.svg"),
        "{got:?}"
    );
    assert!(
        got.iter().all(|n| !n.starts_with("fields-")),
        "a statechart has no field table: {got:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
