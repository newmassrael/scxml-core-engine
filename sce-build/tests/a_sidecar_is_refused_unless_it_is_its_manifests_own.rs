// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A sidecar carries the sentences a manifest deliberately does not, so
//! everything about whether it belongs to THAT manifest has to be
//! checked before a reviewer reads one of its sentences next to a
//! design.
//!
//! Requirement-closure RFC §7a. Each case below is a sidecar that would
//! otherwise "work": it parses, it has sentences, and every one of them
//! is about something else. The refusals are the point, so each is
//! exercised with a non-empty case rather than asserted about in prose.

use sce_build::requirement_manifest::RequirementManifest;
use sce_build::requirement_sidecar::{Gap, RequirementSidecar, SidecarError};

/// A manifest with two requirements. Deliberately a fictional document:
/// naming a real standard in a fixture is what the no-named-standard
/// gate exists to keep out of executable positions.
const MANIFEST: &str = r#"{
  "doc_id": "example-relay-spec",
  "rev": "D3",
  "extraction": {
    "ids": "native",
    "trace": "none",
    "modality_convention": "english-modal-verbs",
    "method": "ai-pass-1"
  },
  "sections": [{ "id": "3.3", "title": "Emergency mode" }],
  "requirements": [
    { "id": "REQ-042", "section": "3.3", "at": { "page": 42 } },
    { "id": "REQ-043", "section": "3.3", "at": { "page": 42 } }
  ]
}"#;

fn manifest() -> RequirementManifest {
    RequirementManifest::from_json(MANIFEST, "manifest under test").expect("the fixture loads")
}

fn sidecar(doc_id: &str, rev: &str, entries: &[(&str, &str)]) -> String {
    let text: Vec<String> = entries
        .iter()
        .map(|(id, sentence)| format!("    {id:?}: {sentence:?}"))
        .collect();
    format!(
        "{{\n  \"doc_id\": {doc_id:?},\n  \"rev\": {rev:?},\n  \"text\": {{\n{}\n  }}\n}}",
        text.join(",\n")
    )
}

#[test]
fn a_sidecar_about_another_document_or_revision_is_refused() {
    const FLOOR: usize = 4;
    let manifest = manifest();
    let sentence = "Holding the start button for 3 seconds enters emergency mode.";

    // (what the sidecar says, why it must be refused, a predicate on the error)
    #[allow(clippy::type_complexity)]
    let cases: Vec<(&str, String, fn(&SidecarError) -> bool)> = vec![
        (
            "sentences from a different document",
            sidecar("example-brake-spec", "D3", &[("REQ-042", sentence)]),
            |e| matches!(e, SidecarError::DifferentDocument { .. }),
        ),
        (
            "the right document at a different revision",
            sidecar("example-relay-spec", "D4", &[("REQ-042", sentence)]),
            |e| matches!(e, SidecarError::DifferentRevision { .. }),
        ),
        (
            "no sentences at all, which would report every requirement as \
             having none",
            sidecar("example-relay-spec", "D3", &[]),
            |e| matches!(e, SidecarError::Empty { .. }),
        ),
        (
            "a field the format does not define, which is how a sentence \
             ends up somewhere nothing reads it",
            r#"{ "doc_id": "example-relay-spec", "rev": "D3",
                 "text": { "REQ-042": "x" }, "notes": "stray" }"#
                .to_string(),
            |e| matches!(e, SidecarError::Parse { .. }),
        ),
    ];

    let mut examined = 0usize;
    let mut accepted: Vec<&str> = Vec::new();
    for (why, raw, is_expected) in &cases {
        examined += 1;
        match RequirementSidecar::from_json(raw, "sidecar under test", &manifest) {
            Ok(_) => accepted.push(why),
            Err(error) if is_expected(&error) => {}
            Err(other) => panic!("{why}: refused, but for the wrong reason: {other}"),
        }
    }

    println!("offered the loader {examined} sidecar(s) that must be refused");
    assert!(
        accepted.is_empty(),
        "a sidecar was accepted whose sentences are not about this manifest, \
         so a reviewer would read them beside a design they never described:\n  {}",
        accepted.join("\n  ")
    );
    assert!(
        examined >= FLOOR,
        "only {examined} refusal(s) exercised; floor {FLOOR}"
    );
}

#[test]
fn an_incomplete_sidecar_is_usable_and_says_where_it_is_silent() {
    let manifest = manifest();
    let raw = sidecar(
        "example-relay-spec",
        "D3",
        &[
            (
                "REQ-042",
                "Holding the start button for 3 seconds enters emergency mode.",
            ),
            (
                "REQ-999",
                "A sentence for an id this manifest never listed.",
            ),
        ],
    );

    let loaded = RequirementSidecar::from_json(&raw, "sidecar under test", &manifest)
        .expect("a sidecar for this document and revision is usable, however partial");

    assert_eq!(
        loaded.sentence("REQ-042"),
        Some("Holding the start button for 3 seconds enters emergency mode."),
        "the sentence a reviewer checks against the page must come back verbatim"
    );

    let gaps = loaded.gaps(&manifest);
    println!(
        "reported {} gap(s) between sidecar and manifest",
        gaps.len()
    );

    assert!(
        gaps.contains(&Gap::NoSentence {
            id: "REQ-043".into()
        }),
        "REQ-043 is listed and has no sentence; block A must print that absence \
         in words, because a blank line reads as a sentence nobody needed to \
         check. Gaps reported: {gaps:?}"
    );
    assert!(
        gaps.contains(&Gap::SentenceForUnlistedId {
            id: "REQ-999".into()
        }),
        "a sentence was extracted for an id the manifest does not list — either \
         a mistyped id or a requirement nobody registered, and dropping it \
         silently hides both. Gaps reported: {gaps:?}"
    );
    assert_eq!(gaps.len(), 2, "no third gap was expected: {gaps:?}");
}
