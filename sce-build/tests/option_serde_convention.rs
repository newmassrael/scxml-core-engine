// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Drift guard for the `Option<T>` / `is [not] none` template-consumption
// convention documented on `sce-build/src/model.rs::DoneDataParam`.
//
// Minijinja's `is none` / `is not none` tests distinguish JSON `null` from
// `undefined`, but `serde`'s `skip_serializing_if = "Option::is_none"`
// erases the key entirely when the value is `None`. A template guard on
// an `is not none`-probed field whose Rust backing carries that attribute
// therefore reads `undefined` — which minijinja treats as truthy through
// `is not none` — and silently emits the wrong branch.
//
// This test fails when any Rust field name probed by a jinja template
// under `is [not] none` carries `skip_serializing_if = "Option::is_none"`
// in a file that is NOT documented as wire-format-only.

use regex::Regex;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Recursively collects paths matching `ext` under `dir`.
fn collect_with_ext(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("read_dir {}: {}", dir.display(), e));
    for entry in entries {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.is_dir() {
            collect_with_ext(&path, ext, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some(ext) {
            out.push(path);
        }
    }
}

/// Files whose `Option<T>` fields serialize to an external wire format
/// independent of template consumption. The P2 convention (no
/// `skip_serializing_if = "Option::is_none"` on fields probed by
/// `is [not] none`) does NOT apply to these files — wire-format stability
/// (smaller JSON, stable contracts with SDK consumers) wins.
///
/// Keep this list short. A struct in one of these files whose name
/// collides with a template-consumed field (e.g. `ForgeField.expr` vs
/// `DoneDataParam.expr`) is a lexical coincidence, not a drift violation.
/// Adding a file here is only appropriate when its structs are never fed
/// to a minijinja template as context.
const WIRE_FORMAT_EXEMPT: &[&str] = &[
    "conformance.rs",
    "forge/diagnostic.rs",
    "forge/model.rs",
    "mesh/deploy.rs",
    // NL→IR Mapping Roadmap Items 1/5/6 metadata family. These
    // structs flow through wire formats (the `sce-codegen
    // requirements` / `sce-codegen unresolved` NDJSON reports and
    // the diagnostic `spec_provenance` field) but are never fed
    // into a minijinja codegen template — the codegen never reads
    // them, so the template convention does not apply.
    "provenance.rs",
];

fn is_wire_format_exempt(rs_file: &Path, repo_root: &Path) -> bool {
    let rel = rs_file.strip_prefix(repo_root).unwrap_or(rs_file);
    let rel_str = rel.to_string_lossy().replace('\\', "/");
    WIRE_FORMAT_EXEMPT
        .iter()
        .any(|suffix| rel_str.ends_with(suffix))
}

#[test]
fn option_is_none_fields_must_not_skip_serializing() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .expect("workspace root is the parent of sce-build")
        .to_path_buf();
    let templates_dir = repo_root.join("tools/codegen/templates");
    let src_dir = manifest_dir.join("src");

    assert!(
        templates_dir.is_dir(),
        "templates directory missing: {}",
        templates_dir.display()
    );
    assert!(
        src_dir.is_dir(),
        "src directory missing: {}",
        src_dir.display()
    );

    // Step 1: collect every `\.X is [not] none` probe from every template.
    let mut jinja_files = Vec::new();
    collect_with_ext(&templates_dir, "jinja2", &mut jinja_files);
    assert!(
        !jinja_files.is_empty(),
        "no jinja2 templates under {} — test wiring or repo layout changed",
        templates_dir.display()
    );

    let none_probe =
        Regex::new(r"\.([A-Za-z_][A-Za-z0-9_]*)\s+is\s+(?:not\s+)?none").expect("regex compiles");
    let mut probed: BTreeSet<String> = BTreeSet::new();
    for file in &jinja_files {
        let content = fs::read_to_string(file).expect("read template");
        for caps in none_probe.captures_iter(&content) {
            probed.insert(caps[1].to_string());
        }
    }
    assert!(
        !probed.is_empty(),
        "no `.x is [not] none` probes found in {} — regex drifted?",
        templates_dir.display()
    );

    // Step 2: walk sce-build/src and flag any `pub <X>: Option<...>` whose
    // preceding attribute block contains `skip_serializing_if = "Option::is_none"`.
    let mut rs_files = Vec::new();
    collect_with_ext(&src_dir, "rs", &mut rs_files);

    // A serde attribute applies only to the field whose declaration it
    // immediately precedes. Walk UP from the `pub X:` line and stop at
    // the first line that is not an attribute — a blank line, another
    // field declaration, or any non-`#[...]` line terminates the block.
    let skip_attr_literal = r#"skip_serializing_if = "Option::is_none""#;

    fn attribute_block_contains(lines: &[&str], field_line: usize, needle: &str) -> bool {
        let mut j = field_line;
        while j > 0 {
            j -= 1;
            let trimmed = lines[j].trim_start();
            if !trimmed.starts_with("#[") {
                return false;
            }
            if lines[j].contains(needle) {
                return true;
            }
        }
        false
    }

    let mut violations: Vec<String> = Vec::new();

    for field in &probed {
        let field_regex = Regex::new(&format!(
            r"^\s*pub\s+{}\s*:\s*Option\b",
            regex::escape(field)
        ))
        .expect("field regex compiles");

        for rs_file in &rs_files {
            if is_wire_format_exempt(rs_file, &repo_root) {
                continue;
            }
            let content = fs::read_to_string(rs_file).expect("read rust source");
            let lines: Vec<&str> = content.lines().collect();
            for (idx, line) in lines.iter().enumerate() {
                if !field_regex.is_match(line) {
                    continue;
                }
                if attribute_block_contains(&lines, idx, skip_attr_literal) {
                    let rel = rs_file.strip_prefix(&repo_root).unwrap_or(rs_file);
                    violations.push(format!(
                        "{}:{}: field `{}` is probed via `is [not] none` in a jinja \
                         template but carries `#[serde(skip_serializing_if = \
                         \"Option::is_none\")]`. That attribute drops the JSON key \
                         entirely when None, so the template guard reads `undefined` \
                         — which minijinja treats as truthy through `is not none` — \
                         and mis-renders. Remove the attribute, or (if this struct \
                         is never fed to a template) add the file to \
                         `WIRE_FORMAT_EXEMPT` in this test with a one-line rationale.",
                        rel.display(),
                        idx + 1,
                        field
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Option<T> / `is [not] none` template convention violated. See \
         `sce-build/src/model.rs` DoneDataParam docstring for the canonical rule. \
         Misaligned fields:\n  {}",
        violations.join("\n  ")
    );
}
