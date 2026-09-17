// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A string `lookup`'s MISS DEFAULT is a member of the emitted enum.
//
// Found the same way the procedure donedata and payload defects were, and
// failing the same way: the document was accepted with exit 0 and the
// EMITTED C++ DID NOT COMPILE. `LookupModel::unique_values()` built the
// variant list from `entries` alone, while every backend emits the miss arm
// as `return <Enum>::<default>` — so a default that is not also an entry
// value names a member the enum does not have.
//
// ⚠ It stayed invisible because every fixture's default ALSO appeared as an
// entry: `lookup_gear_position` defaults to `NEUTRAL`, which is key 2. The
// variant was always supplied by accident. The shape that exposes it is the
// ordinary one — "everything listed is supported, anything else is not" —
// where the default is deliberately a value no key maps to.
//
// Measured 2026-09-17 on a service-capability table: every declared key
// answers "supported" and the default is the one answer no key carries.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

static SCRATCH: AtomicU64 = AtomicU64::new(0);

struct Out(PathBuf);

impl Out {
    fn new(label: &str) -> Self {
        let id = SCRATCH.fetch_add(1, Ordering::SeqCst);
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("{label}-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        Out(dir)
    }

    fn emitted(&self, ext: &str) -> String {
        let mut all = String::new();
        for entry in std::fs::read_dir(&self.0).expect("read scratch dir") {
            let path = entry.expect("dir entry").path();
            if path.extension().is_some_and(|e| e == ext) {
                all.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
            }
        }
        all
    }
}

impl Drop for Out {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A capability table: every declared key answers the same value, and the
/// default is the one answer no key carries.
const DEFAULT_NOT_AN_ENTRY: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="lookup" name="capability" version="1.0">
  <datamodel>
    <data id="key" sce:type="uint8" sce:direction="in"/>
    <data id="support" sce:type="string" sce:direction="out"/>
    <data id="mapping" sce:default="NotSupported">
      <sce:entry key="1" value="Support"/>
      <sce:entry key="2" value="Support"/>
    </data>
  </datamodel>
</scxml>
"#;

fn generate(lang: &str, ext: &str, label: &str) -> String {
    let out = Out::new(label);
    let path = out.0.join("capability.scxml");
    std::fs::write(&path, DEFAULT_NOT_AN_ENTRY).expect("write document");
    let run = Command::new(sce_codegen_bin())
        .args([
            "generate",
            path.to_str().unwrap(),
            "-l",
            lang,
            "-o",
            out.0.to_str().unwrap(),
        ])
        .current_dir(repo_root())
        .output()
        .expect("spawn sce-codegen");
    assert_eq!(
        run.status.code(),
        Some(0),
        "{lang}: generation refused:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    out.emitted(ext)
}

/// The miss default appears in the emitted variant list.
///
/// The assertion is on the DECLARATION rather than on a compile, because a
/// unit test cannot run six toolchains — but the declaration is exactly what
/// was missing, and the miss arm that names it is asserted alongside so the
/// two cannot drift apart.
#[test]
fn a_lookup_miss_default_is_declared_as_a_variant() {
    for (lang, ext, decl) in [
        ("cpp", "h", "NotSupported"),
        ("rust", "rs", "NotSupported"),
        ("kotlin", "kt", "NotSupported"),
        ("go", "go", "NotSupported"),
    ] {
        let src = generate(lang, ext, &format!("lookup-default-{lang}"));
        assert!(
            src.contains(decl),
            "{lang}: the miss default is never declared:\n{src}"
        );
        // And it is reachable: something must RETURN it, or the declaration
        // would satisfy this check while the miss arm named something else.
        assert!(
            src.matches(decl).count() >= 2,
            "{lang}: `{decl}` appears once — declared or returned, not both:\n{src}"
        );
    }
}

/// ⚠ And the fix must not duplicate a default that IS an entry value.
///
/// `lookup_gear_position` is exactly that shape, and a naive append would
/// emit `enum { …, NEUTRAL, NEUTRAL }`. This is the guard that keeps the
/// repair from breaking the case that hid the defect.
#[test]
fn a_default_that_is_also_an_entry_is_not_duplicated() {
    let doc = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="lookup" name="gearsel" version="1.0">
  <datamodel>
    <data id="raw" sce:type="uint8" sce:direction="in"/>
    <data id="gear" sce:type="string" sce:direction="out"/>
    <data id="mapping" sce:default="NEUTRAL">
      <sce:entry key="0" value="PARK"/>
      <sce:entry key="1" value="NEUTRAL"/>
    </data>
  </datamodel>
</scxml>
"#;
    let out = Out::new("lookup-default-dup");
    let path = out.0.join("gearsel.scxml");
    std::fs::write(&path, doc).expect("write document");
    let run = Command::new(sce_codegen_bin())
        .args([
            "generate",
            path.to_str().unwrap(),
            "-l",
            "cpp",
            "-o",
            out.0.to_str().unwrap(),
        ])
        .current_dir(repo_root())
        .output()
        .expect("spawn sce-codegen");
    assert_eq!(run.status.code(), Some(0), "generation refused");

    let src = out.emitted("h");
    let decl = src
        .lines()
        .find(|l| l.contains("enum class"))
        .unwrap_or_default();
    assert_eq!(
        decl.matches("NEUTRAL").count(),
        1,
        "the default is declared twice: {decl}"
    );
}
