// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A set of codec documents is checked whole, and nothing is written
//! (docs/adr/0078 §3).
//!
//! A downstream consumer moved its SCE pin and regenerated its codecs: its tool
//! wrote thirteen generated files, then stopped at the first refused document,
//! and left a tree that was half one pin and half the other. Generation stops at
//! the first refusal, so a consumer learned of each refused document in its own
//! run. `check_forge_files` answers every document's verdict in one call, in
//! the order given, before anything is written.

use std::path::PathBuf;

use sce_build::generator::Language;

const PASSES: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="passes">
  <datamodel>
    <sce:field id="len" sce:type="uint64" sce:byte="0" sce:bit-size="vle"/>
    <sce:field id="body" sce:type="bytes" sce:byte="1" sce:bit-size="length-ref"
               sce:length-field="len" sce:max-size="16"/>
  </datamodel>
</scxml>
"#;

/// A length-ref field without its bound (docs/adr/0077 §5).
const UNBOUNDED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="unbounded">
  <datamodel>
    <sce:field id="len" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:field id="body" sce:type="bytes" sce:byte="1" sce:bit-size="length-ref"
               sce:length-field="len"/>
  </datamodel>
</scxml>
"#;

/// A declaration that the encode can refuse, of a codec whose encode cannot
/// (docs/adr/0065 §3).
const STALE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:encode-may-fail="true" sce:default-endian="big" name="stale">
  <datamodel>
    <sce:field id="tag" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
  </datamodel>
</scxml>
"#;

#[test]
fn every_document_is_answered_in_order_and_nothing_is_written() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut paths: Vec<PathBuf> = Vec::new();
    for (name, text) in [
        ("unbounded", UNBOUNDED),
        ("passes", PASSES),
        ("stale", STALE),
    ] {
        let path = dir.path().join(format!("{name}.scxml"));
        std::fs::write(&path, text).expect("write document");
        paths.push(path);
    }
    let before: Vec<_> = std::fs::read_dir(dir.path())
        .expect("read dir")
        .map(|e| e.expect("entry").file_name())
        .collect();

    let verdicts = sce_build::check_forge_files(
        &paths,
        Language::Rust,
        &[],
        &sce_build::ForgeCompileOptions::default(),
    );

    assert_eq!(
        verdicts.iter().map(|v| v.path.clone()).collect::<Vec<_>>(),
        paths,
        "one verdict per document, in the order given"
    );
    let codes = |i: usize| -> Vec<String> {
        verdicts[i]
            .diagnostics
            .iter()
            .map(|d| d.code.as_str().to_string())
            .collect()
    };
    assert!(
        !verdicts[0].passed(),
        "a field without its bound is refused"
    );
    assert!(
        codes(0).iter().any(|c| c == "validation/missing-attribute"),
        "{:?}",
        codes(0)
    );
    assert!(verdicts[1].passed(), "{:?}", codes(1));
    assert!(!verdicts[2].passed(), "a stale declaration is refused");
    assert!(
        codes(2)
            .iter()
            .any(|c| c == "codec/encode-may-fail-without-refusal"),
        "{:?}",
        codes(2)
    );

    let after: Vec<_> = std::fs::read_dir(dir.path())
        .expect("read dir")
        .map(|e| e.expect("entry").file_name())
        .collect();
    assert_eq!(before, after, "checking writes nothing");
}
