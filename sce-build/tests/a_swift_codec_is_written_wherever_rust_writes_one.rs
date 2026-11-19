// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Swift writes every codec of the repository that Rust, the reference, writes.
//!
//! The codec matrix says Swift ships a codec whole (`codegen_matrix::template_ships`), so the
//! conformance harness schedules every codec fixture on Swift and refuses one Swift does not write
//! as a defect. What Swift's codec still refuses by name (`forge::generator::swift_codec`) is a
//! shape no codec of the repository holds, as the C++ printer's refusals are
//! (`a_cpp_codec_is_printed_from_its_plan_or_refused_by_name.rs`). This walks every codec document
//! of the repository, generates it for Rust and for Swift, and holds each one Rust writes to being
//! written on Swift, but those [`REFUSED`] names: a codec only an MCU-class backend writes, which
//! Swift refuses as every backend that is not one does. A change that makes Swift refuse a codec
//! Rust writes, or write one it refused, is seen here.

use std::path::{Path, PathBuf};

use sce_build::compile_forge_file;
use sce_build::generator::Language;
use sce_build::ForgeCompileOptions;

/// The directories of the repository that hold codec documents, from `sce-build`: the resources,
/// the Rust conformance fixtures, the conformance contexts, the standard library, the test fixtures
/// and the kind examples.
const CODEC_ROOTS: &[&str] = &[
    "../tests/forge/resources",
    "../backends/rust/forge-conformance/fixtures",
    "../tests/forge/conformance",
    "../stdlib",
    "tests/fixtures",
    "kind-examples",
];

/// The codecs of the repository Rust writes and Swift does not, by path from `sce-build`, each
/// with what its refusal names, in order: the CBOR map of every constructor of the language, which
/// Rust, the reference, prints and Swift does not yet (docs/adr/0101, Landing order steps 4a and
/// 4g), and a field aligned to a DMA burst, which only an MCU-class backend writes
/// (`codegen/mcu-class-kind-on-non-mcu-language`).
const REFUSED: &[(&str, &str)] = &[
    (
        "../tests/forge/conformance/codec_cbor_composed/codec_cbor_composed.scxml",
        "CBOR entry 'small' holds `int8`, a type of the language whose int8 the printer does not \
         lower in a CBOR codec yet",
    ),
    (
        "../tests/forge/resources/codec_dma_aligned_basic.scxml",
        "CodegenMcuClassKindOnNonMcuLanguage",
    ),
];

/// The fewest codecs the walk compares: a walk that stopped reading a root would compare fewer
/// and pass on less.
const MIN_COMPARED: usize = 150;

/// Every document under `dir` that declares itself a codec, by path, in order.
fn codec_documents_under(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            codec_documents_under(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "scxml")
            && std::fs::read_to_string(&path).is_ok_and(|text| text.contains("sce:kind=\"codec\""))
        {
            out.push(path);
        }
    }
}

#[test]
fn swift_writes_every_codec_of_the_repository_that_rust_writes() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut paths = Vec::new();
    for root in CODEC_ROOTS {
        codec_documents_under(&manifest.join(root), &mut paths);
    }
    paths.sort();
    let options = ForgeCompileOptions::default();
    let mut compared = 0usize;
    // Each codec Rust writes and Swift refuses, with the refusal as its diagnostic prints.
    let mut refused: Vec<(String, String)> = Vec::new();
    for path in &paths {
        // A document Rust does not write (one that holds what no backend accepts, or that needs
        // a deployment to be written) is not a codec Swift owes.
        if compile_forge_file(path, Language::Rust, &[], &options).is_err() {
            continue;
        }
        compared += 1;
        if let Err(refusal) = compile_forge_file(path, Language::Swift, &[], &options) {
            let shown = path
                .strip_prefix(manifest)
                .unwrap_or(path)
                .display()
                .to_string();
            refused.push((shown, format!("{refusal:?}")));
        }
    }
    eprintln!(
        "{} codec document(s) read, {compared} written by Rust, {} refused by Swift:",
        paths.len(),
        refused.len()
    );
    for (shown, refusal) in &refused {
        eprintln!("  {shown}: {refusal}");
    }
    assert!(
        compared >= MIN_COMPARED,
        "{compared} codecs compared, fewer than {MIN_COMPARED}: a root was not read"
    );
    let refused_paths: Vec<&str> = refused.iter().map(|(shown, _)| shown.as_str()).collect();
    let expected_paths: Vec<&str> = REFUSED.iter().map(|(shown, _)| *shown).collect();
    assert_eq!(
        refused_paths, expected_paths,
        "Swift refuses exactly the codecs Rust writes that REFUSED names; a change that refuses \
         another, or writes one of them, names it there"
    );
    for ((shown, refusal), (_, names)) in refused.iter().zip(REFUSED) {
        assert!(
            refusal.contains(names),
            "{shown} is refused by `{names}`: {refusal}"
        );
    }
}
