// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An acceptance names the list it was taken against by the words behind its
//! ids as well as by the manifest.
//!
//! A manifest is coordinates only (an id, a section, a modality): the
//! sentences live in a sidecar that is not committed. So two lists of one
//! shape and different sentences have one manifest digest, and a revision of
//! one read as a revision of the other (review of 2026-10-09: a delta built
//! from another specification belonged to an acceptance because the shape
//! was the same). The record therefore pins the sidecar's digest beside the
//! manifest's, and never the sentences, so it can be committed where the
//! sidecar is not.
//!
//! What these cases hold:
//!
//! * the digest is over the sidecar's bytes, and no sentence reaches the
//!   record;
//! * two lists of one shape differ in the pin the words add, and share the
//!   manifest's;
//! * a record taken without the words is the bytes it was before the field
//!   existed, and one with them reads back unchanged;
//! * a sidecar that is not the manifest's own is refused, and so is a
//!   manifest that is not the one the record pins;
//! * `accept --sidecar` writes what the library writes, and refuses what the
//!   library refuses.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sce_build::acceptance_record::{AcceptanceRecord, RecordError};
use sha2::{Digest, Sha256};

const CODEGEN: &str = env!("CARGO_BIN_EXE_sce-codegen");

/// A fictional document: a fixture that names a real standard is what the
/// no-named-standard gate keeps out of executable positions.
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

const DESIGN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" name="door" initial="shut">
  <state id="shut"><transition event="open" target="done"/></state>
  <final id="done"/>
</scxml>
"#;

const FIRST_SENTENCE: &str = "Holding the start button for 3 seconds enters emergency mode.";
const OTHER_SENTENCE: &str = "Pulling the release lever for 5 seconds leaves emergency mode.";

fn sidecar(doc_id: &str, rev: &str, first: &str, second: &str) -> String {
    format!(
        "{{\n  \"doc_id\": {doc_id:?},\n  \"rev\": {rev:?},\n  \"text\": {{\n    \
         \"REQ-042\": {first:?},\n    \"REQ-043\": {second:?}\n  }}\n}}\n"
    )
}

fn digest(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

fn root() -> tempfile::TempDir {
    let root = tempfile::TempDir::new().expect("tempdir");
    put(root.path(), "spec/manifest.json", MANIFEST);
    put(root.path(), "design/door.scxml", DESIGN);
    root
}

fn put(root: &Path, rel: &str, text: &str) -> PathBuf {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
    fs::write(&path, text).expect("write");
    path
}

fn taken(root: &Path) -> AcceptanceRecord {
    AcceptanceRecord::take(
        root,
        &root.join("design/door.scxml"),
        &root.join("spec/manifest.json"),
        "base",
    )
    .expect("the record is taken")
}

#[test]
fn the_digest_is_over_the_sidecars_bytes_and_no_sentence_reaches_the_record() {
    let root = root();
    let words = sidecar("example-relay-spec", "D3", FIRST_SENTENCE, FIRST_SENTENCE);
    let path = put(root.path(), "spec/words.json", &words);
    let record = taken(root.path())
        .with_words(&root.path().join("spec/manifest.json"), &path)
        .expect("the sidecar is the manifest's own");
    assert_eq!(
        record.manifest.sidecar_sha256.as_deref(),
        Some(digest(&words).as_str())
    );
    let json = record.to_json();
    assert!(
        !json.contains(FIRST_SENTENCE),
        "a sentence of the specification reached a record that is committed:\n{json}"
    );
    assert_eq!(
        AcceptanceRecord::from_json(&json).expect("the record reads back"),
        record
    );
}

#[test]
fn two_lists_of_one_shape_differ_in_the_words_and_not_in_the_manifest() {
    let root = root();
    let manifest = root.path().join("spec/manifest.json");
    let a = put(
        root.path(),
        "spec/a.json",
        &sidecar("example-relay-spec", "D3", FIRST_SENTENCE, FIRST_SENTENCE),
    );
    let b = put(
        root.path(),
        "spec/b.json",
        &sidecar("example-relay-spec", "D3", OTHER_SENTENCE, OTHER_SENTENCE),
    );
    let with_a = taken(root.path()).with_words(&manifest, &a).unwrap();
    let with_b = taken(root.path()).with_words(&manifest, &b).unwrap();
    assert_eq!(
        with_a.manifest.sha256, with_b.manifest.sha256,
        "the manifest is coordinates only, so it cannot tell these two lists apart"
    );
    assert_ne!(
        with_a.manifest.sidecar_sha256, with_b.manifest.sidecar_sha256,
        "the words are what tell them apart"
    );
}

#[test]
fn a_record_taken_without_the_words_is_the_bytes_it_was_before_the_field() {
    let root = root();
    let json = taken(root.path()).to_json();
    assert!(
        !json.contains("sidecar_sha256"),
        "a record with no words names a field it does not have:\n{json}"
    );
    let record = AcceptanceRecord::from_json(&json).expect("it reads back");
    assert_eq!(record.manifest.sidecar_sha256, None);
    assert_eq!(
        record.to_json(),
        json,
        "reading and writing it moved a byte"
    );
}

#[test]
fn a_sidecar_that_is_not_the_manifests_own_is_refused() {
    let root = root();
    let manifest = root.path().join("spec/manifest.json");
    for (why, text, kind) in [
        (
            "another document",
            sidecar("example-brake-spec", "D3", FIRST_SENTENCE, FIRST_SENTENCE),
            "different-document",
        ),
        (
            "another revision of this document",
            sidecar("example-relay-spec", "D4", FIRST_SENTENCE, FIRST_SENTENCE),
            "different-revision",
        ),
        (
            "no sentence at all",
            "{\"doc_id\": \"example-relay-spec\", \"rev\": \"D3\", \"text\": {}}\n".to_string(),
            "empty",
        ),
    ] {
        let path = put(root.path(), "spec/words.json", &text);
        match taken(root.path()).with_words(&manifest, &path) {
            Err(error @ RecordError::Words(_)) => assert_eq!(error.kind(), kind, "{why}"),
            other => panic!("{why}: expected the sidecar's own refusal, got {other:?}"),
        }
    }
}

#[test]
fn the_manifest_given_has_to_be_the_one_the_record_pins() {
    let root = root();
    let words = put(
        root.path(),
        "spec/words.json",
        &sidecar("example-relay-spec", "D3", FIRST_SENTENCE, FIRST_SENTENCE),
    );
    // The same list with one more requirement is another manifest.
    let other = put(
        root.path(),
        "spec/other.json",
        &MANIFEST.replace(
            "{ \"id\": \"REQ-043\", \"section\": \"3.3\", \"at\": { \"page\": 42 } }",
            "{ \"id\": \"REQ-043\", \"section\": \"3.3\", \"at\": { \"page\": 42 } },\n    \
             { \"id\": \"REQ-044\", \"section\": \"3.3\", \"at\": { \"page\": 43 } }",
        ),
    );
    match taken(root.path()).with_words(&other, &words) {
        Err(RecordError::Source { detail }) => {
            assert!(
                detail.contains("not the manifest this record pins"),
                "{detail}"
            )
        }
        other => panic!("a sidecar was tied to a manifest the record does not pin: {other:?}"),
    }
}

#[test]
fn a_digest_that_is_not_one_is_refused_when_a_record_is_read() {
    let root = root();
    let words = put(
        root.path(),
        "spec/words.json",
        &sidecar("example-relay-spec", "D3", FIRST_SENTENCE, FIRST_SENTENCE),
    );
    let json = taken(root.path())
        .with_words(&root.path().join("spec/manifest.json"), &words)
        .unwrap()
        .to_json();
    let pinned = digest(&fs::read_to_string(&words).unwrap());
    for bad in ["not-a-digest", &pinned.to_uppercase(), &pinned[..40]] {
        let altered = json.replace(&pinned, bad);
        assert!(
            matches!(
                AcceptanceRecord::from_json(&altered),
                Err(RecordError::Format { .. })
            ),
            "a words digest of `{bad}` was read as a record"
        );
    }
}

fn accept(root: &Path, sidecar: Option<&Path>) -> Output {
    let mut command = Command::new(CODEGEN);
    command
        .arg("--error-format=json")
        .arg("accept")
        .arg(root.join("design/door.scxml"))
        .arg("--manifest")
        .arg(root.join("spec/manifest.json"))
        .args(["--variant", "base", "--root"])
        .arg(root)
        .arg("--out")
        .arg(root.join("acceptance.json"))
        .current_dir(root);
    if let Some(sidecar) = sidecar {
        command.arg("--sidecar").arg(sidecar);
    }
    command.output().expect("spawn sce-codegen")
}

#[test]
fn accept_with_a_sidecar_writes_what_the_library_writes() {
    let root = root();
    let words = put(
        root.path(),
        "spec/words.json",
        &sidecar("example-relay-spec", "D3", FIRST_SENTENCE, OTHER_SENTENCE),
    );
    let out = accept(root.path(), Some(&words));
    assert!(
        out.status.success(),
        "`accept --sidecar` exited {:?}: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    let written = fs::read_to_string(root.path().join("acceptance.json")).unwrap();
    let expected = taken(root.path())
        .with_words(&root.path().join("spec/manifest.json"), &words)
        .unwrap()
        .to_json();
    assert_eq!(written, expected);
    assert!(written.contains("sidecar_sha256"), "{written}");
    // And without it, the record is the one the library takes without it.
    assert!(accept(root.path(), None).status.success());
    let bare = fs::read_to_string(root.path().join("acceptance.json")).unwrap();
    assert_eq!(bare, taken(root.path()).to_json());
}

#[test]
fn accept_refuses_a_sidecar_that_is_not_the_manifests_own_and_writes_nothing() {
    let root = root();
    let words = put(
        root.path(),
        "spec/words.json",
        &sidecar("example-brake-spec", "D3", FIRST_SENTENCE, FIRST_SENTENCE),
    );
    let out = accept(root.path(), Some(&words));
    assert!(
        !out.status.success(),
        "a sidecar of another document was accepted"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cli/closure-input-unusable")
            && stderr.contains("sentences from 'example-brake-spec'"),
        "the refusal does not say which door refused it, or why: {stderr}"
    );
    assert!(
        !root.path().join("acceptance.json").exists(),
        "a record was written for a refused sidecar"
    );
}
