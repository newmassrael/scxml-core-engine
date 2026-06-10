// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Drift guard: the `src/helpers/mod.rs` module index marks `!no_std`-gated
// helpers as "(std-only)" plain code spans, because an intra-doc link to a
// cfg-gated target cannot resolve in the no_std docs profile. A plain span
// never breaks, so the rustdoc broken-link gate (Stage 2c, both profiles)
// can only catch the gated→linked direction — it is blind to a module that
// is later un-gated while its index entry still claims "(std-only)", and to
// a module missing from the index entirely. This test covers those
// directions: it parses the index bullets and the `pub mod` gating (outer
// `#[cfg]` attribute in mod.rs, or inner `#![cfg]` in the module file, e.g.
// `url_encoding`) and asserts the two stay in lockstep, both ways.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const MOD_RS: &str = include_str!("../src/helpers/mod.rs");
const NOT_NO_STD_OUTER: &str = "#[cfg(not(feature = \"no_std\"))]";
const NOT_NO_STD_INNER: &str = "#![cfg(not(feature = \"no_std\"))]";

/// Parse the module index bullets: `//! - [`name`]: ...` (linked = present
/// in both profiles) vs `//! - `name` (std-only): ...` (plain span =
/// `!no_std`-gated). Any other bullet form fails loudly so a new entry
/// must pick one of the two recognized shapes.
fn parse_index() -> (BTreeSet<String>, BTreeSet<String>) {
    let mut linked = BTreeSet::new();
    let mut std_only = BTreeSet::new();
    for line in MOD_RS.lines() {
        let Some(rest) = line.trim_start().strip_prefix("//! - ") else {
            continue;
        };
        if let Some(rest) = rest.strip_prefix("[`") {
            let name = rest
                .split('`')
                .next()
                .expect("split yields at least one element");
            assert!(
                linked.insert(name.to_string()),
                "duplicate index entry: {name}"
            );
        } else if let Some(rest) = rest.strip_prefix('`') {
            let (name, tail) = rest
                .split_once('`')
                .expect("unterminated code span in index bullet");
            assert!(
                tail.trim_start().starts_with("(std-only):"),
                "plain-span index entry `{name}` must carry the (std-only) marker; \
                 write it as [`{name}`] if the module is available under no_std"
            );
            assert!(
                std_only.insert(name.to_string()),
                "duplicate index entry: {name}"
            );
        } else {
            panic!("unrecognized index bullet form: {line}");
        }
    }
    (linked, std_only)
}

/// Collect `pub mod name;` declarations and whether an outer
/// `#[cfg(not(feature = "no_std"))]` guards each one. Comment and blank
/// lines may sit between the attribute and the declaration; any other
/// intervening item resets the pending attribute.
fn parse_decls() -> Vec<(String, bool)> {
    let mut decls = Vec::new();
    let mut pending_gate = false;
    for line in MOD_RS.lines() {
        let t = line.trim();
        if t == NOT_NO_STD_OUTER {
            pending_gate = true;
        } else if let Some(rest) = t.strip_prefix("pub mod ") {
            let name = rest.trim_end_matches(';');
            decls.push((name.to_string(), pending_gate));
            pending_gate = false;
        } else if !(t.is_empty() || t.starts_with("//")) {
            pending_gate = false;
        }
    }
    decls
}

/// Whole-module inner gate (`#![cfg(...)]` at the top of the module file),
/// the `url_encoding` pattern.
fn module_inner_gated(name: &str) -> bool {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/helpers")
        .join(format!("{name}.rs"));
    let source = fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {} (directory-form module? extend this guard): {e}",
            path.display()
        )
    });
    source.lines().any(|l| l.trim() == NOT_NO_STD_INNER)
}

#[test]
fn helpers_index_matches_no_std_gating() {
    let (linked, std_only) = parse_index();
    let decls = parse_decls();
    assert!(!decls.is_empty(), "parser found no pub mod declarations");
    assert!(!linked.is_empty(), "parser found no linked index entries");
    assert!(
        !std_only.is_empty(),
        "parser found no (std-only) index entries"
    );

    let declared: BTreeSet<String> = decls.iter().map(|(n, _)| n.clone()).collect();
    let indexed: BTreeSet<String> = linked.union(&std_only).cloned().collect();
    assert_eq!(
        declared, indexed,
        "module index out of sync with pub mod declarations"
    );

    let gated: BTreeSet<String> = decls
        .iter()
        .filter(|(name, outer)| *outer || module_inner_gated(name))
        .map(|(n, _)| n.clone())
        .collect();
    assert_eq!(
        std_only, gated,
        "(std-only) markers out of sync with cfg(not(feature = \"no_std\")) gating"
    );
}
