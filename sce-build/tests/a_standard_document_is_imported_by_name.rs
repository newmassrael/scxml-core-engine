// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! SCE's standard algorithm library: a document imports a standard one by
//! its `sce:std/...` name, and that name resolves to the copy this
//! generator embeds — on every backend, with nothing copied beside the
//! importing document.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sce_build::compile_scxml_with_imports;
use sce_build::generator::Language;
use sce_build::ForgeCompileOptions;

const DAYS_FROM_CIVIL: &str = "sce:std/time/days_from_civil.scxml";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn walk(root: &Path, current: &Path, out: &mut BTreeMap<String, String>) {
    for entry in std::fs::read_dir(current).expect("read stdlib dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            walk(root, &path, out);
        } else if path.extension().is_some_and(|e| e == "scxml") {
            let rel = path.strip_prefix(root).expect("under root");
            out.insert(
                format!("sce:std/{}", rel.to_string_lossy().replace('\\', "/")),
                std::fs::read_to_string(&path).expect("read standard document"),
            );
        }
    }
}

/// The library a generator carries is the `stdlib/` tree it was built
/// from — every document, and nothing else. A floor, because an empty
/// library on both sides would agree.
#[test]
fn the_embedded_library_is_the_tree_on_disk() {
    let root = repo_root().join("stdlib");
    let mut on_disk = BTreeMap::new();
    walk(&root, &root, &mut on_disk);
    let embedded: BTreeMap<String, String> = sce_build::forge::stdlib::documents()
        .map(|(name, content)| (name, content.to_string()))
        .collect();
    assert!(
        on_disk.contains_key(DAYS_FROM_CIVIL),
        "the library holds days_from_civil"
    );
    assert_eq!(embedded, on_disk);
}

fn options_for(language: Language) -> ForgeCompileOptions {
    let mut options = ForgeCompileOptions::default();
    if matches!(language, Language::Go) {
        options.go_module_prefix = Some("example.com/consumer/generated".to_string());
    }
    options
}

/// A caller in `dir` that imports `src` and calls it.
fn caller(dir: &Path, src: &str) -> PathBuf {
    let path = dir.join("epoch_day_of.scxml");
    std::fs::write(
        &path,
        format!(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="epoch_day_of" version="1.0">
  <sce:import kind="algorithm" src="{src}" as="civil"/>
  <sce:signature>
    <sce:param name="year" type="int32"/>
    <sce:param name="month" type="uint8"/>
    <sce:param name="day" type="uint8"/>
    <sce:return type="int64"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="civil(year, month, day)"/>
  </sce:body>
</scxml>"#
        ),
    )
    .expect("write caller");
    path
}

/// A document imports `days_from_civil` by name, and the build set that
/// generates it lists the standard document by the same name: nothing is
/// staged beside the caller, on any backend.
#[test]
fn a_document_imports_a_standard_one_by_name_on_every_backend() {
    let dir = tempfile::tempdir().expect("tempdir");
    let caller = caller(dir.path(), DAYS_FROM_CIVIL);
    for &language in Language::ALL {
        let outputs = compile_scxml_with_imports(
            &[],
            &[Path::new(DAYS_FROM_CIVIL), caller.as_path()],
            &sce_build::find_template_dir_for(language),
            language,
            &options_for(language),
            None,
        )
        .unwrap_or_else(|e| {
            panic!(
                "{}: the build set did not generate: {e}",
                language.canonical_name()
            )
        });
        let generated: String = outputs
            .iter()
            .flat_map(|(_, out)| out.files.iter().map(|(_, text)| text.as_str()))
            .collect();
        for name in ["days_from_civil", "epoch_day_of"] {
            assert!(
                generated
                    .to_ascii_lowercase()
                    .replace('_', "")
                    .contains(&name.replace('_', "")),
                "{}: nothing named {name} was generated",
                language.canonical_name()
            );
        }
    }
}

/// Every standard document generates on every backend, named as a consumer
/// names it — the library is a promise to all six, so a document one
/// backend cannot generate is not one it may hold. Imports between standard
/// documents (`days_in_month` reads `is_leap_year` by a relative name)
/// resolve inside the library.
#[test]
fn every_standard_document_generates_on_every_backend() {
    let names: Vec<String> = sce_build::forge::stdlib::documents()
        .map(|(name, _)| name)
        .collect();
    assert!(
        names.len() >= 4,
        "the time documents are in the library: {names:?}"
    );
    let paths: Vec<&Path> = names.iter().map(Path::new).collect();
    for &language in Language::ALL {
        let outputs = compile_scxml_with_imports(
            &[],
            &paths,
            &sce_build::find_template_dir_for(language),
            language,
            &options_for(language),
            None,
        )
        .unwrap_or_else(|e| {
            panic!(
                "{}: the library did not generate: {e}",
                language.canonical_name()
            )
        });
        assert_eq!(
            outputs.len(),
            names.len(),
            "{}: one output per standard document",
            language.canonical_name()
        );
    }
}

/// `sce-codegen generate` takes a standard document by the name a consumer
/// imports it by, and writes a depfile a build system can keep fresh: the
/// standard name is in the generator, so it is not listed as a file — a
/// file that never exists reads as always out of date.
#[test]
fn the_cli_generates_a_standard_document_by_its_name() {
    let out = tempfile::tempdir().expect("tempdir");
    let depfile = out.path().join("days_in_month.d");
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .args([
            "generate",
            "sce:std/time/days_in_month.scxml",
            "-l",
            "rust",
            "-o",
        ])
        .arg(out.path())
        .arg("--write-deps")
        .arg(&depfile)
        .current_dir(repo_root())
        .output()
        .expect("sce-codegen runs");
    assert_eq!(
        run.status.code(),
        Some(0),
        "generation failed:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let written: Vec<String> = std::fs::read_dir(out.path())
        .expect("output dir")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        written.iter().any(|name| name.ends_with(".rs")),
        "no Rust source was written: {written:?}"
    );
    let deps = std::fs::read_to_string(&depfile).expect("depfile written");
    assert!(
        !deps.contains("sce:std"),
        "the depfile names a standard document:\n{deps}"
    );

    let missing = std::process::Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .args([
            "generate",
            "sce:std/time/no_such_day.scxml",
            "-l",
            "rust",
            "-o",
        ])
        .arg(out.path())
        .current_dir(repo_root())
        .output()
        .expect("sce-codegen runs");
    assert_ne!(
        missing.status.code(),
        Some(0),
        "an unknown standard document generated"
    );
    assert!(
        String::from_utf8_lossy(&missing.stderr).contains("standard library"),
        "the refusal names the library: {}",
        String::from_utf8_lossy(&missing.stderr)
    );
}

/// A standard name the library does not hold is refused as a missing
/// import, and the refusal says where it looked — so an author reading it
/// does not go looking for a file on disk.
#[test]
fn a_standard_name_the_library_lacks_is_refused_where_it_looked() {
    let dir = tempfile::tempdir().expect("tempdir");
    let caller = caller(dir.path(), "sce:std/time/days_from_civl.scxml");
    let result = compile_scxml_with_imports(
        &[],
        &[caller.as_path()],
        &sce_build::find_template_dir_for(Language::Rust),
        Language::Rust,
        &options_for(Language::Rust),
        None,
    );
    let text = match result {
        Ok(_) => panic!("an unknown standard document was accepted"),
        Err(err) => err.to_string(),
    };
    assert!(
        text.contains("days_from_civl") && text.contains("standard library"),
        "the refusal names the document and the library: {text}"
    );
}
