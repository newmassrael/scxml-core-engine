// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A requirement's fragment is what its behaviour depends on, not the
//! nodes that happen to carry its id.
//!
//! Requirement-closure RFC §7a.1 judges the acceptance report by
//! mutation: break a requirement and the page must move. A block chosen
//! by `sce:req` fails that test by construction — retiming a delay
//! changes a `<send>` that carries no id at all, so a page built from
//! id-selected nodes stays byte-identical while the machine's behaviour
//! changes.
//!
//! This file holds the closure to that standard against the pair in
//! `tests/fixtures/requirement_closure/`, which is a real standard's
//! subclause and a statechart written from it — neither shaped to this
//! code.

use std::path::{Path, PathBuf};

use sce_build::acceptance_report::{fragment, Relation};
use sce_build::parser::SCXMLParser;
use sce_build::requirement_manifest::RequirementManifest;

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("requirement_closure")
}

fn load_manifest() -> RequirementManifest {
    let path = fixture_dir().join("iso13400_2_nl_socket_handling.manifest.json");
    RequirementManifest::load(&path).expect("the committed manifest loads")
}

fn model() -> sce_build::model::SCXMLModel {
    let path = fixture_dir().join("doip_nl_connection_states.scxml");
    let raw = std::fs::read_to_string(&path).expect("the committed document is readable");
    SCXMLParser::new()
        .parse_string(&raw, &path.display().to_string())
        .unwrap_or_else(|e| panic!("the committed document must parse: {:?}", e.error))
}

#[test]
fn a_fragment_reaches_nodes_that_do_not_carry_the_id() {
    let manifest = load_manifest();
    let model = model();

    let mut annotated = 0usize;
    let mut pairs = 0usize;
    let mut off_id = 0usize;
    let mut empty: Vec<String> = Vec::new();
    let mut by_relation: std::collections::BTreeMap<Relation, usize> = Default::default();

    for entry in &manifest.requirements {
        let deps = fragment(&model, &entry.id);
        if deps.is_empty() {
            // Not every requirement in a standard's subclause is met by
            // this document; an unannotated one has no fragment and that
            // is not a defect. It is only counted, so the numbers below
            // are about the requirements this document does claim.
            continue;
        }
        annotated += 1;
        pairs += deps.len();
        for dep in &deps {
            *by_relation.entry(dep.relation).or_default() += 1;
            if dep.relation != Relation::Cited {
                off_id += 1;
            }
        }
        if deps.iter().all(|d| d.relation == Relation::Cited) {
            empty.push(entry.id.clone());
        }
    }

    println!(
        "{annotated} annotated requirement(s), {pairs} (requirement, dependency) pair(s), \
         {off_id} of them on nodes carrying no id"
    );
    for (relation, count) in &by_relation {
        println!("  {relation:?}: {count}");
    }
    println!(
        "{} requirement(s) whose fragment is only its own cited nodes",
        empty.len()
    );

    // Measured 2026-09-13 against this pair: 18 annotated requirements,
    // 62 pairs, 31 off-id, and every relation non-empty. The floors sit
    // just under those so a regression trips while an edit to the
    // fixture does not have to move them.
    const ANNOTATED_FLOOR: usize = 15;
    const PAIRS_FLOOR: usize = 55;
    const OFF_ID_FLOOR: usize = 28;

    assert!(
        annotated >= ANNOTATED_FLOOR,
        "only {annotated} requirement(s) annotated; floor {ANNOTATED_FLOOR}. A \
         shrinking population is how this file would keep passing while \
         measuring less and less"
    );
    assert!(
        pairs >= PAIRS_FLOOR,
        "only {pairs} (requirement, dependency) pair(s); floor {PAIRS_FLOOR}"
    );
    assert!(
        off_id >= OFF_ID_FLOOR,
        "only {off_id} dependency(ies) lie on nodes carrying no id; floor \
         {OFF_ID_FLOOR}. Were this zero the closure would be doing nothing an \
         `sce:req` selection does not, and a delay mutation would leave the \
         page byte-identical — which is exactly what RFC §7a.1 refuses"
    );

    // ⭐ The assertion that keeps the closure honest as it changes: every
    // relation it claims to walk must actually fire on this pair. A
    // relation that silently stops matching still leaves the totals above
    // looking healthy, because the other five cover for it.
    for relation in [
        Relation::Cited,
        Relation::ArmingSend,
        Relation::CancelOfArmingSend,
        Relation::TargetEntry,
        Relation::SourceExit,
        Relation::TransitionAction,
    ] {
        assert!(
            by_relation.get(&relation).copied().unwrap_or(0) > 0,
            "no dependency was found through {relation:?}, so that edge is \
             either unreachable or no longer matching. Counts: {by_relation:?}"
        );
    }
}
