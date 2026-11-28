// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A document's generated code is a function of the document and of what its
// imports reach, and of nothing else. The route that brings it to the
// generator, `sce-codegen check|generate <doc>` alone or a document set that
// holds it beside other documents, does not change what it means: one route
// refusing it while the other accepts it, or the two printing it apart, would
// give one document two meanings.
//
// What a set says of itself is the one thing it adds, and it moves no
// meaning: which functions another document of the set calls (SCE_FORGE.md
// §4.12) decides only whether a Rust function a document of the set calls is
// hinted `#[inline]`, which the documents compared below are not.
//
// Measured 2026-11-23 before the element of a collection was read at its
// import: an algorithm that walks a bounded collection typed each item from
// the element's schema only when a document set held the element document
// too. Generated alone, the item was a record whose members were registered
// nowhere, so a misspelled field passed every backend, and the Go it printed
// read the field of an element Go presents through a method as a struct
// field, which does not compile.

use std::path::{Path, PathBuf};

use sce_build::forge::codegen_matrix::{emitting, emitting_in_part};
use sce_build::forge::model::ForgeKind;
use sce_build::generator::{GeneratedOutput, Language};
use sce_build::{CompileError, ForgeCompileOptions};

const GO_PREFIX: &str = "example.com/generated";

fn resource(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/forge/resources")
        .join(name)
}

fn options_for(lang: Language) -> ForgeCompileOptions {
    let mut options = ForgeCompileOptions::default();
    if matches!(lang, Language::Go) {
        options.go_module_prefix = Some(GO_PREFIX.to_string());
    }
    options
}

/// The document generated alone, as `sce-codegen generate <doc>` generates it.
fn alone(document: &Path, lang: Language) -> Result<GeneratedOutput, CompileError> {
    sce_build::compile_forge_file(document, lang, &[], &options_for(lang))
}

/// The document generated in the set `members`, the document among them, as
/// `sce-codegen orchestrate --forge ...` generates it.
fn in_set(
    document: &Path,
    members: &[PathBuf],
    lang: Language,
) -> Result<GeneratedOutput, CompileError> {
    let members: Vec<&Path> = members.iter().map(PathBuf::as_path).collect();
    let outputs = sce_build::compile_scxml_with_imports(
        &[],
        &members,
        &sce_build::find_template_dir_for(lang),
        lang,
        &options_for(lang),
        None,
    )?;
    let name = document
        .file_name()
        .and_then(|name| name.to_str())
        .expect("a document file name");
    Ok(outputs
        .into_iter()
        .find(|(basename, _)| basename == name)
        .unwrap_or_else(|| panic!("{name}: the set generated no output for it"))
        .1)
}

/// A document that walks a bounded collection, and the set an orchestrated
/// build generates it in.
struct Walker {
    document: &'static str,
    /// Its element, its collection and what else it imports; the set is these
    /// and the document itself, last.
    beside: &'static [&'static str],
    /// The backends the set generates on.
    langs: Vec<Language>,
}

fn walkers() -> Vec<Walker> {
    vec![
        Walker {
            document: "algorithm_bc_iter_minimal.scxml",
            beside: &["subscription_entry.scxml", "local_sub_table.scxml"],
            // A backend that writes an algorithm in part (Swift) lowers the
            // walk of a collection, so it is held to it too.
            langs: emitting_in_part(ForgeKind::Algorithm).collect(),
        },
        Walker {
            document: "algorithm_keyexpr_match_first.scxml",
            beside: &[
                "subscription_entry.scxml",
                "local_sub_table.scxml",
                "algorithm_keyexpr_intersect_exact.scxml",
            ],
            langs: emitting(ForgeKind::Algorithm).collect(),
        },
        Walker {
            document: "algorithm_keyexpr_field_match.scxml",
            beside: &[
                "keyexpr_entry.scxml",
                "local_keyexpr_table.scxml",
                "algorithm_bytes_equal.scxml",
            ],
            langs: emitting(ForgeKind::Algorithm).collect(),
        },
    ]
}

/// What a document set prints for each document that walks a collection, it
/// prints for the document alone, file for file, on every backend.
#[test]
fn a_document_that_walks_a_collection_prints_alike_alone_and_in_a_set() {
    for Walker {
        document,
        beside,
        langs,
    } in walkers()
    {
        let path = resource(document);
        let mut members: Vec<PathBuf> = beside.iter().copied().map(resource).collect();
        members.push(path.clone());
        assert!(!langs.is_empty(), "{document}: no backend to compare on");
        for lang in langs {
            let set = in_set(&path, &members, lang)
                .unwrap_or_else(|e| panic!("{document} ({lang:?}): the set refused it: {e}"));
            let single = alone(&path, lang)
                .unwrap_or_else(|e| panic!("{document} ({lang:?}): generated alone, refused: {e}"));
            assert_eq!(
                single.files, set.files,
                "{document} ({lang:?}): generated alone, it prints otherwise than in a set"
            );
        }
    }
}

/// The Go a document alone prints reads the element's field as the element
/// presents it, through its method: the line that did not compile while the
/// element was read only by a document set.
#[test]
fn a_document_alone_reads_the_field_of_a_printed_go_element_by_its_method() {
    let path = resource("algorithm_bc_iter_minimal.scxml");
    let single = alone(&path, Language::Go).expect("the Go of the document alone");
    let code: String = single
        .files
        .iter()
        .map(|(_, text)| text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        code.contains("entry.CallbackId()"),
        "the element's field is read through its method:\n{code}"
    );
}

/// The element a compile reads is an input of the compile, listed beside the
/// documents its imports reach, so a build that regenerates on a change of
/// any input regenerates on a change of the element.
#[test]
fn the_element_of_a_collection_is_an_input_of_the_document_that_walks_it() {
    let path = resource("algorithm_bc_iter_minimal.scxml");
    let single = alone(&path, Language::Rust).expect("the Rust of the document alone");
    let element = std::fs::canonicalize(resource("subscription_entry.scxml")).expect("resolves");
    let collection = std::fs::canonicalize(resource("local_sub_table.scxml")).expect("resolves");
    assert!(
        single.deps.contains(&element),
        "the element is an input: {:?}",
        single.deps
    );
    assert!(
        single.deps.contains(&collection),
        "the collection is an input: {:?}",
        single.deps
    );
}
