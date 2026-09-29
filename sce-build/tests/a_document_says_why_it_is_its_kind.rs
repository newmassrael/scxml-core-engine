// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A document can say why it is the kind it declares, and the owner reads
//! the reason where they read the document.
//!
//! # What was wrong
//!
//! An author choosing a kind from a specification — a person, or a model
//! reaching SCE through the authoring MCP — stated the reason in the
//! conversation that produced the document and nowhere else. The owner
//! reviewing the pseudocode saw `transform adc_to_volts` and never the
//! clause it was chosen from, or the neighbour it was told apart from.
//!
//! `<sce:kind-basis>` puts the reason in the document
//! (docs/SCE_ACCEPTED_SUBSET.md §2.10.1). These cases hold it on the
//! binary an author's document goes through: `check` accepts it for a
//! forge kind and a statechart alike and says so on the manifest, `pseudo`
//! writes it under the head line, and a basis that contradicts the
//! document is refused in both pipelines with one code.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/kind_basis")
        .join(name)
}

fn run(args: &[&str], document: &Path) -> Output {
    Command::new(sce_codegen_bin())
        .args(["--error-format", "json"])
        .args(args)
        .arg(document)
        .output()
        .expect("run sce-codegen")
}

fn scratch(label: &str, body: &str) -> PathBuf {
    let dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    let path = dir.join(format!("{label}.scxml"));
    std::fs::write(&path, body).expect("write fixture");
    path
}

/// Accepted, and published on the manifest, for both pipelines.
#[test]
fn a_basis_is_accepted_and_published_for_every_pipeline() {
    for (file, kind) in [
        ("transform_with_basis.scxml", "transform"),
        ("statechart_with_basis.scxml", "statechart"),
    ] {
        let out = run(&["check", "--lint"], &fixture(file));
        assert!(
            out.status.success(),
            "{file}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let manifest: serde_json::Value =
            serde_json::from_slice(&out.stdout).expect("one manifest line");
        assert_eq!(
            manifest["document_kind"],
            serde_json::json!({"name": kind, "declared": true, "basis_recorded": true}),
            "{file}"
        );
    }
}

/// The page says why, directly under the line that names the kind, in the
/// order the document wrote it.
#[test]
fn the_page_says_why_under_the_head_line() {
    let out = run(&["pseudo"], &fixture("transform_with_basis.scxml"));
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let page = String::from_utf8(out.stdout).expect("utf-8");
    let lines: Vec<&str> = page.lines().collect();
    assert_eq!(
        lines[..7],
        [
            // A forge document is named by its file stem
            // (docs/SCE_ACCEPTED_SUBSET.md §2.1), not by `name=`.
            "transform transform_with_basis",
            "  kind-basis:",
            "    evidence the voltage is one formula over the current count",
            "      provenance SPEC-7@2#4.1",
            "    rather-than interpolation",
            "      reason the text gives a formula, not values at breakpoints",
            "  in count: uint16",
        ],
        "{page}"
    );

    let out = run(&["pseudo"], &fixture("statechart_with_basis.scxml"));
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let page = String::from_utf8(out.stdout).expect("utf-8");
    let lines: Vec<&str> = page.lines().collect();
    assert!(lines[0].starts_with("machine "), "{page}");
    assert_eq!(lines[1], "  kind-basis:", "{page}");
    assert_eq!(lines[4], "    rather-than procedure", "{page}");
}

/// A kind the specification leaves open is an open marker like any other:
/// the document checks, the strict build refuses it, the report lists it
/// under `<kind-basis>`, and the page shows it — in both pipelines.
#[test]
fn an_open_kind_is_refused_by_the_strict_build_and_listed() {
    let basis = r#"<sce:kind-basis sce:unresolved="kind"
      sce:unresolved-reason="the text never says what an unlisted input gives"
      sce:unresolved-candidates="transform">
    <sce:evidence>three listed inputs, each with its setting</sce:evidence>
  </sce:kind-basis>"#;
    let cases = [
        (
            "open_lookup",
            format!(
                r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="lookup" name="setting">
  {basis}
  <datamodel>
    <data id="input" sce:type="uint8" sce:direction="in"/>
    <data id="setting" sce:type="uint8" sce:direction="out"/>
    <data id="mapping" sce:default="0">
      <sce:entry key="1" value="10"/>
      <sce:entry key="2" value="20"/>
    </data>
  </datamodel>
</scxml>
"#
            ),
        ),
        (
            "open_statechart",
            format!(
                r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="statechart" version="1.0" initial="idle">
  {basis}
  <state id="idle"/>
</scxml>
"#
            ),
        ),
    ];
    for (label, body) in cases {
        let path = scratch(label, &body);
        let checked = run(&["check"], &path);
        assert!(
            checked.status.success(),
            "{label}: {}",
            String::from_utf8_lossy(&checked.stderr)
        );

        let strict = run(&["check", "--strict-unresolved"], &path);
        assert!(
            !strict.status.success(),
            "{label}: the strict build must refuse"
        );
        assert!(
            String::from_utf8_lossy(&strict.stderr).contains("validation/unresolved-placeholder"),
            "{label}: {}",
            String::from_utf8_lossy(&strict.stderr)
        );

        let listed = run(&["unresolved"], &path);
        assert!(listed.status.success(), "{label}");
        let record: serde_json::Value = serde_json::from_slice(
            listed
                .stdout
                .split(|b| *b == b'\n')
                .next()
                .expect("a record"),
        )
        .expect("NDJSON");
        assert_eq!(record["node_path"], "<kind-basis>", "{label}: {record}");
        assert_eq!(record["id"], "kind", "{label}: {record}");
        assert_eq!(
            record["candidates"],
            serde_json::json!(["transform"]),
            "{record}"
        );

        let page = run(&["pseudo"], &path);
        let page = String::from_utf8_lossy(&page.stdout);
        assert!(page.contains("    unresolved kind\n"), "{label}: {page}");
    }
}

/// Ruling out the document's own kind is a contradiction, refused by one
/// code in both pipelines, on the line that states it.
#[test]
fn a_basis_that_rules_out_its_own_kind_is_refused() {
    let cases = [
        (
            "own_transform",
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="transform" name="t">
  <sce:kind-basis>
    <sce:evidence>a formula</sce:evidence>
    <sce:rejected kind="transform">not a formula</sce:rejected>
  </sce:kind-basis>
  <datamodel>
    <data id="a" sce:type="int32" sce:direction="in"/>
    <data id="b" sce:type="int32" sce:direction="out" expr="a + 1"/>
  </datamodel>
</scxml>
"#,
        ),
        (
            "own_statechart",
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle">
  <sce:kind-basis>
    <sce:evidence>it waits for events</sce:evidence>
    <sce:rejected kind="statechart">it does not wait</sce:rejected>
  </sce:kind-basis>
  <state id="idle"/>
</scxml>
"#,
        ),
    ];
    for (label, body) in cases {
        let out = run(&["check"], &scratch(label, body));
        assert!(!out.status.success(), "{label} must be refused");
        let record: serde_json::Value = serde_json::from_slice(
            out.stderr
                .split(|b| *b == b'\n')
                .next()
                .expect("one record"),
        )
        .expect("the record is JSON");
        assert_eq!(
            record["code"], "validation/kind-basis-malformed",
            "{record}"
        );
        assert_eq!(record["location"]["line"], 5, "{record}");
    }
}
