// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Tree-wide hygiene gate: committed source files must not carry
// roadmap markers ("Phase N", "Phase A3", "phase-b", ...) or
// references to gitignored design documents (`claudedocs/`). The
// commit-audit hook checks diffs only; this test pins the whole
// tree so a marker cannot survive through an unaudited path (merge,
// generated-tree refresh, vendored snippet, fixture authoring).
//
// Allowed citation forms are unaffected and never match here:
// W3C SCXML sections ("W3C SCXML 3.13"), ledger citations in the
// namespaced sigil form, and external SCE Protocol-Synthesis RFC
// sections ("RFC §synth-5-B", "item A3"). English uses of the bare word
// "phase" ("two-phase commit") also do not match — the pattern
// requires a token suffix (a digit-led token, a single letter, or
// a letter+digit token after the separator).

use std::fs;
use std::path::{Path, PathBuf};

/// Directory names never scanned anywhere in the tree: third-party
/// code, build artifacts, gitignored working docs.
const EXCLUDED_DIR_NAMES: &[&str] = &[
    ".git",
    ".claude",
    "build",
    "target",
    "vendor",
    "third_party",
    "doom_wasm",
    "node_modules",
    "claudedocs",
];

/// Repo-relative path prefixes never scanned. Precise prefixes, not
/// names — authored fixture trees like `tests/forge/resources/` stay
/// under the gate.
const EXCLUDED_PATH_PARTS: &[&str] = &[
    "resources/",               // W3C corpus mirror (third-party TXML/SCXML)
    "embed/",                   // gitignored mirror artifact of canonical sce/
    "tools/mnemosyne-adoption", // takes claudedocs paths as data input
    "docs/sce-ledger",          // ledger configs document their regeneration source
];

/// File extensions under the gate.
const SCANNED_EXTENSIONS: &[&str] = &[
    "rs", "h", "hpp", "c", "cc", "cpp", "go", "kt", "py", "jinja2", "sh", "yml", "yaml", "toml",
    "cmake", "scxml", "xsd",
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// Tracked files only — `git ls-files` is the enumeration source so
/// gitignored artifacts (generated probe sources, embed/ mirror,
/// build trees) never enter the gate, and an untracked scratch file
/// cannot red CI.
fn tracked_files(root: &Path) -> Vec<PathBuf> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-z"])
        .output()
        .expect("git ls-files runs");
    assert!(out.status.success(), "git ls-files must succeed");
    String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(|p| root.join(p))
        .collect()
}

fn is_scanned_file(path: &Path) -> bool {
    if path.file_name().is_some_and(|n| n == "CMakeLists.txt") {
        return true;
    }
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| SCANNED_EXTENSIONS.contains(&e))
}

fn is_excluded(rel: &str) -> bool {
    if EXCLUDED_PATH_PARTS.iter().any(|p| rel.starts_with(p)) {
        return true;
    }
    rel.split('/')
        .any(|seg| EXCLUDED_DIR_NAMES.contains(&seg) || seg.starts_with("build_"))
}

#[test]
fn tree_carries_no_roadmap_markers_or_gitignored_doc_references() {
    let root = repo_root();
    let files: Vec<PathBuf> = tracked_files(&root)
        .into_iter()
        .filter(|p| is_scanned_file(p))
        .collect();
    assert!(
        files.len() > 500,
        "gate scanned only {} tracked files — enumeration is broken",
        files.len()
    );

    // "phase" + separator + (digit-led token | single letter | letter+digit
    // token). Catches "Phase N", "Phase A3", "phase-x", "phase_b"
    // shapes while passing "phase shift" / "two-phase".
    let phase_marker = regex::Regex::new(r"(?i)\bphase[ _-]([a-z]?[0-9][a-z0-9.]*|[a-z])\b")
        .expect("phase-marker regex compiles");
    // Internal chain codenames: Greek-suffixed item subdivisions
    // ("B5-ν", "C13-α", "B7-η'"), Axis-N programs, decision-register
    // keys ("Q-Outbox-8", "Q-C10-β-3", "Q-A4" — `\b` keeps the genuine
    // upstream-RFC "OQ-Wnn" labels out since `O` is a word char),
    // and "<letter><digit> lock-in" decision ids ("T3 lock-in").
    // Bare item numbers ("item B7", "C13") stay legal — only the
    // memory-only subdivision/decision shapes are banned.
    let chain_label = regex::Regex::new(
        r"\b[A-Z][0-9]+-[\u{3b1}-\u{3c9}]|\bAxis-[0-9]|\bQ-[A-Z\u{3b1}-\u{3c9}§][A-Za-z0-9§.\u{3b1}-\u{3c9}-]*\b|\b[A-Z][0-9]+ lock-in\b",
    )
    .expect("chain-label regex compiles");
    // Assembled at runtime so this file's own source does not match
    // itself in other tools' greps.
    let gitignored_doc = format!("{}{}/", "claude", "docs");

    let self_name = "roadmap_marker_gate.rs";
    let mut violations = Vec::new();

    for file in &files {
        if file.file_name().is_some_and(|n| n == self_name) {
            continue;
        }
        let rel = file.strip_prefix(&root).unwrap_or(file);
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if is_excluded(&rel_str) {
            continue;
        }
        let Ok(bytes) = fs::read(file) else { continue };
        let text = String::from_utf8_lossy(&bytes);
        for (idx, line) in text.lines().enumerate() {
            if line.contains(&gitignored_doc)
                || phase_marker.is_match(line)
                || chain_label.is_match(line)
            {
                violations.push(format!("{}:{}: {}", rel_str, idx + 1, line.trim()));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "roadmap markers / gitignored-doc references found in the tree \
         ({} site(s)). Rewrite each as a self-contained present-tense \
         fact (see CLAUDE.md \"Code Comments\"):\n{}",
        violations.len(),
        violations.join("\n")
    );
}
