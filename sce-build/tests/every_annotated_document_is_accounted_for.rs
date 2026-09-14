// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Every document that carries `sce:req` is accounted for: a manifest governs
// it, or this repository says why none does.
//
// WHAT WAS MISSING — measured 2026-09-14.
//
// Three committed documents carry `sce:req`. One of them is governed by
// `iso13400_2_nl_socket_handling.manifest.json`, and
// `iso13400_requirement_closure.rs` checks that pair thoroughly. The other two
// carry synthetic ids that name no specification. Nothing recorded which was
// which, and — the part that matters — nothing would have noticed a FOURTH
// document arriving with a mistyped id and no manifest at all. Every existing
// requirement test names its fixture explicitly; not one enumerates the tree
// (`git ls-files` appears in none of them).
//
// So the hole was never "an id fails to resolve" — the governed pair is
// already held to that. It was "an annotated document nobody classified", and
// a classification is not a property of the document: a document does not know
// it is a fixture. That is why the registry is a repository file
// (`tests/requirements/governance.json`) rather than an attribute the document
// carries, which would also have widened the accepted subset every backend
// reads for a fact no backend needs.
//
// ⚠ WHY THIS DOES NOT SIMPLY REFUSE UNRESOLVED IDS. The governed document
// cites `DoIP-134` where its manifest carries `3.DoIP-134`, deliberately: ISO
// 13400-2 itself spells the id both ways, and a test asserts the mismatch is
// reported. A gate that refused every unresolved id would delete that fixture's
// subject. The registry names such an id with its reason instead — and
// `an_unresolved_id_that_resolves_is_a_retired_fixture` fails when the reason
// stops being true, so the carve-out cannot outlive what it describes.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// The annotation this gate is about, as it appears in a document.
///
/// The `=` is load-bearing. `sce:req` alone is a PREFIX of
/// `sce:requires-parent-flags`, an unrelated forge attribute carried by
/// fourteen documents under `tests/forge/resources/`; a substring sweep for
/// the bare token reports seventeen annotated documents where there are
/// three, and a registry built on that count would have listed fourteen
/// fixtures that carry no annotation at all.
const ANNOTATION: &str = "sce:req=";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    #[serde(rename = "_comment", default)]
    _comment: Vec<String>,
    documents: Vec<Entry>,
}

/// One classified document.
///
/// `deny_unknown_fields` is the schema: a misspelled key is refused here
/// rather than ignored into a silent exemption, which is why this file carries
/// no companion `*.schema.json` — a second description of one shape is a
/// second answer to the same question.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    document: String,
    /// The manifest that governs it. Exactly one of this and `fixture`.
    #[serde(default)]
    governed_by: Option<String>,
    /// Why nothing governs it. Exactly one of this and `governed_by`.
    #[serde(default)]
    fixture: Option<String>,
    /// Ids this document cites that its manifest deliberately does not carry.
    #[serde(default)]
    unresolved: Vec<Unresolved>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Unresolved {
    id: String,
    why: String,
}

#[derive(Deserialize)]
struct Manifest {
    requirements: Vec<ManifestRequirement>,
}

#[derive(Deserialize)]
struct ManifestRequirement {
    id: String,
}

fn repo_root() -> PathBuf {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .expect("git rev-parse");
    PathBuf::from(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn registry(root: &Path) -> Registry {
    let path = root.join("tests/requirements/governance.json");
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

/// Every tracked `*.scxml` that carries the annotation, repo-relative.
///
/// Enumerated from the tree, because the arrival this gate exists for is a
/// document that does not exist yet — which is exactly what a list written
/// over today's tree cannot name.
fn annotated_documents(root: &Path) -> Vec<String> {
    let out = std::process::Command::new("git")
        .current_dir(root)
        .args(["ls-files", "*.scxml"])
        .output()
        .expect("git ls-files");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .filter(|rel| !rel.is_empty())
        .filter(|rel| {
            fs::read_to_string(root.join(rel))
                .map(|text| text.contains(ANNOTATION))
                .unwrap_or(false)
        })
        .map(str::to_string)
        .collect()
}

/// The ids a document cites, flattened — one attribute may list several.
fn cited_ids(text: &str) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    let mut rest = text;
    while let Some(at) = rest.find(ANNOTATION) {
        rest = &rest[at + ANNOTATION.len()..];
        let Some(quote) = rest.strip_prefix('"') else {
            continue;
        };
        let Some(end) = quote.find('"') else { break };
        ids.extend(quote[..end].split_whitespace().map(str::to_string));
        rest = &quote[end..];
    }
    ids
}

fn manifest_ids(root: &Path, rel: &str) -> Option<BTreeSet<String>> {
    let text = fs::read_to_string(root.join(rel)).ok()?;
    let parsed: Manifest =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse manifest {rel}: {e}"));
    Some(parsed.requirements.into_iter().map(|r| r.id).collect())
}

/// The floors, asserted once and shared.
///
/// Three separate ones, because the three ways this sweep can go quiet are
/// different failures: the enumeration stops finding documents, the registry
/// stops classifying any of them as governed, or the manifest set empties. Any
/// one of those makes every assertion below true of nothing.
fn measured_population(root: &Path) -> (Vec<String>, Registry) {
    let documents = annotated_documents(root);
    let reg = registry(root);

    assert!(
        documents.len() >= 3,
        "found {} document(s) carrying `{ANNOTATION}`; this tree carries three. \
         An enumeration that stopped reaching the tree would make every \
         assertion in this file pass by examining nothing",
        documents.len()
    );
    assert!(
        reg.documents.iter().any(|e| e.governed_by.is_some()),
        "no registry entry names a manifest, so nothing here checks an id \
         against a requirement set"
    );
    (documents, reg)
}

#[test]
fn every_annotated_document_is_classified() {
    let root = repo_root();
    let (documents, reg) = measured_population(&root);

    let classified: BTreeSet<&str> = reg.documents.iter().map(|e| e.document.as_str()).collect();
    let unclassified: Vec<&String> = documents
        .iter()
        .filter(|rel| !classified.contains(rel.as_str()))
        .collect();

    assert!(
        unclassified.is_empty(),
        "document(s) carry `{ANNOTATION}` and tests/requirements/governance.json \
         says nothing about them: {unclassified:?}\n\
         Add an entry naming the manifest that governs each one, or say why none \
         does. An annotated document nobody classified is an annotation that \
         asserts nothing — which is the state fourteen forge resources were \
         mistaken for while this gate was being written."
    );
}

#[test]
fn every_entry_is_exactly_one_kind_and_still_describes_the_tree() {
    let root = repo_root();
    let (documents, reg) = measured_population(&root);
    let annotated: BTreeSet<&str> = documents.iter().map(String::as_str).collect();

    let mut wrong: Vec<String> = Vec::new();
    for e in &reg.documents {
        match (&e.governed_by, &e.fixture) {
            (Some(_), Some(_)) => wrong.push(format!(
                "{}: names a manifest AND declares itself a fixture",
                e.document
            )),
            (None, None) => wrong.push(format!(
                "{}: neither `governed_by` nor `fixture`",
                e.document
            )),
            _ => {}
        }
        if !annotated.contains(e.document.as_str()) {
            wrong.push(format!(
                "{}: classified here but carries no `{ANNOTATION}` — the entry \
                 outlived its document",
                e.document
            ));
        }
        if e.fixture.is_some() && !e.unresolved.is_empty() {
            wrong.push(format!(
                "{}: a fixture governed by no manifest cannot have ids its \
                 manifest does not carry",
                e.document
            ));
        }
        if let Some(m) = &e.governed_by {
            if !root.join(m).is_file() {
                wrong.push(format!(
                    "{}: names a manifest that is not there: {m}",
                    e.document
                ));
            }
        }
    }

    assert!(
        wrong.is_empty(),
        "tests/requirements/governance.json disagrees with the tree:\n  {}",
        wrong.join("\n  ")
    );
}

#[test]
fn every_cited_id_in_a_governed_document_resolves() {
    let root = repo_root();
    let (_, reg) = measured_population(&root);

    let mut unresolved: Vec<String> = Vec::new();
    let mut checked = 0usize;
    for e in &reg.documents {
        let Some(rel) = &e.governed_by else { continue };
        let Some(known) = manifest_ids(&root, rel) else {
            continue; // the previous test reports a missing manifest
        };
        let text = match fs::read_to_string(root.join(&e.document)) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let excused: BTreeSet<&str> = e.unresolved.iter().map(|u| u.id.as_str()).collect();
        for id in cited_ids(&text) {
            checked += 1;
            if !known.contains(&id) && !excused.contains(id.as_str()) {
                unresolved.push(format!(
                    "{}: cites {id:?}, {rel} does not carry it",
                    e.document
                ));
            }
        }
    }

    // An arity floor on the CITATIONS, not just the documents: a governed
    // document whose annotations stopped parsing would leave this test
    // comparing an empty set against a manifest and reporting nothing wrong.
    assert!(
        checked >= 10,
        "only {checked} citation(s) were compared against a manifest; the \
         governed document carries nineteen, so the reader is not reaching them"
    );
    assert!(
        unresolved.is_empty(),
        "requirement id(s) cited by a governed document that its manifest does \
         not carry:\n  {}\n\
         Either the id is mistyped, or the manifest is missing an entry, or the \
         mismatch is deliberate — in which case name it under `unresolved` with \
         the reason, the way `DoIP-134` is.",
        unresolved.join("\n  ")
    );
}

#[test]
fn an_unresolved_id_that_resolves_is_a_retired_fixture() {
    let root = repo_root();
    let (_, reg) = measured_population(&root);

    let mut retired: Vec<String> = Vec::new();
    let mut declared: BTreeMap<&str, usize> = BTreeMap::new();
    for e in &reg.documents {
        let Some(rel) = &e.governed_by else { continue };
        let Some(known) = manifest_ids(&root, rel) else {
            continue;
        };
        declared.insert(e.document.as_str(), e.unresolved.len());
        for u in &e.unresolved {
            assert!(
                !u.why.trim().is_empty(),
                "{}: `unresolved` id {:?} carries no reason",
                e.document,
                u.id
            );
            if known.contains(&u.id) {
                retired.push(format!(
                    "{}: {:?} is named as unresolvable, but {rel} now carries it — {}",
                    e.document, u.id, u.why
                ));
            }
        }
    }

    assert!(
        retired.is_empty(),
        "a citation declared unresolvable now resolves, so whatever the \
         declaration protects is no longer being measured:\n  {}\n\
         Delete the entry AND the assertion that depended on the mismatch — a \
         fixture whose subject has gone is a test that reports success for \
         having nothing to say.",
        retired.join("\n  ")
    );

    // The declaration set is small and load-bearing; a silent drop to zero
    // would mean the carve-out was deleted rather than retired deliberately.
    assert!(
        declared.values().sum::<usize>() >= 1,
        "no governed document declares an unresolvable citation. If the \
         `DoIP-134` spelling mismatch was resolved upstream, retire this floor \
         together with the fixture that asserts it"
    );
}
