// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A forge document handed to the statechart entry must be told so.
//
// SCE_ERROR_CONTRACT.md §4.1 makes `sce:kind` on the `<scxml>` root the
// routing key, and `Pipeline`'s own doc comment says the router "is the
// single source of truth for routing — the CLI dispatches on it, and any
// future embedding API must too". The CLI did. The library entries — the
// ones a downstream `build.rs` calls — did not.
//
// A forge document wears an `<scxml>` root, so the parser's
// `WrongRootElement` guard cannot see it; it parsed to a stateless model
// and the author was told `No state nodes found in SCXML document`. That
// message names the one thing that is not wrong: an `algorithm` document
// is not supposed to have states. The repair it implies — add a `<state>`
// — makes the document worse.
//
// The guard is at the parser rather than at each entry so that the file,
// string and WASM routes cannot disagree about it.

use sce_build::forge::error::{ForgeError, ValidationError};
use sce_build::forge::model::ForgeKind;
use sce_build::{compile_scxml_lang_typed, find_template_dir_for, generator::Language, Pipeline};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// Real forge fixtures, one per kind the corpus offers here, so the guard
/// is exercised against documents the forge pipeline genuinely accepts
/// rather than against a hand-written stub that might be wrong twice.
const FORGE_FIXTURES: &[(&str, ForgeKind)] = &[
    ("algorithm_crc16", ForgeKind::Algorithm),
    ("algorithm_bytes_equal", ForgeKind::Algorithm),
];

#[test]
fn a_forge_document_is_refused_by_name_not_by_its_missing_states() {
    let root = repo_root();
    let template_dir = find_template_dir_for(Language::Rust);

    for (stem, expected_kind) in FORGE_FIXTURES {
        let path = root.join(format!("tests/forge/resources/{stem}.scxml"));
        assert!(path.exists(), "fixture {} is missing", path.display());

        let Err(err) = compile_scxml_lang_typed(
            path.to_str().expect("fixture path is utf-8"),
            &template_dir,
            Language::Rust,
        ) else {
            panic!("{stem}: a forge document must not compile as a statechart");
        };

        match &err.error {
            ForgeError::Validation(v) => match &**v {
                ValidationError::WrongPipeline { kind, pipeline } => {
                    assert_eq!(kind, expected_kind, "{stem}: wrong kind reported");
                    assert_eq!(
                        pipeline,
                        &Pipeline::Scxml,
                        "{stem}: the refusing pipeline is the SCXML one, not the \
                         one the document belongs to",
                    );
                }
                other => panic!("{stem}: expected WrongPipeline, got {other:?}"),
            },
            other => panic!("{stem}: expected a validation error, got {other:?}"),
        }

        // The message is the part an author reads. It must not be the
        // old one, and it must name the pipeline that refused.
        let text = err.error.to_string();
        assert!(
            !text.contains("No state nodes found"),
            "{stem}: still reports the misleading empty-document message: {text}",
        );
        assert!(
            text.contains("SCXML pipeline"),
            "{stem}: message does not name the refusing pipeline: {text}",
        );
    }
}

#[test]
fn a_statechart_still_compiles_through_the_same_entry() {
    // The guard keys on a forge kind, so `sce:kind` absent and
    // `sce:kind="statechart"` must both pass straight through. Without
    // this the fix would read as working while having closed the door
    // on every real caller.
    let template_dir = find_template_dir_for(Language::Rust);
    let dir = tempfile::tempdir().expect("tempdir");

    for (name, kind_attr) in [("plain", ""), ("declared", r#" sce:kind="statechart""#)] {
        let path = dir.path().join(format!("{name}.scxml"));
        std::fs::write(
            &path,
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" name="{name}" initial="go"{kind_attr}>
  <state id="go">
    <transition event="t" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#
            ),
        )
        .expect("write fixture");

        let out = compile_scxml_lang_typed(
            path.to_str().expect("utf-8 path"),
            &template_dir,
            Language::Rust,
        )
        .unwrap_or_else(|e| panic!("{name}: statechart must still compile: {}", e.error));
        assert_eq!(out.files.len(), 1, "{name}: one Rust artifact expected");
    }
}
