// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Which documents this repository AUTHORS, derived rather than listed.
//!
//! Two sweeps ask it — the expression-refusal sweep and the lint sweep —
//! and before 2026-09-22 each named its own directories: one saw 56 of
//! the 560 tracked statecharts and the other 309, with no reason given
//! for the rest. What the narrower one missed was a real defect
//! (`tests/integration/test_thermostat.scxml`, six undeclared functions),
//! so the lists were not merely incomplete: they were the reason nobody
//! had looked.
//!
//! Two exclusions, each a property of what the document IS:
//!
//!   * the W3C corpus under `resources/`, fetched by
//!     `resources/download-tests.py` against `resources/manifest.xml` —
//!     not this repository's to write, and four of its tests write an
//!     illegal expression on purpose (309, 343, 344, 457);
//!   * a FRAGMENT, meaning a document another tracked document pulls in
//!     with `<xi:include>` or `<sce:use>`. It is a piece of a machine
//!     rather than one, so the design-time lints — unreachable state,
//!     inconsistent event handling — describe what being a fragment
//!     means. Ten of them sit under `tests/parsing/fixtures`.
//!
//! A forge document is not a statechart and is left out by asking the
//! generator's own `detect_kind`, not by a path convention.

// Every test binary that declares `mod common` compiles this file, and
// each sweep asks for one of the two populations — the same reason
// `repository` carries this attribute.
#![allow(dead_code)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The repository root, from this crate's manifest directory.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// Every tracked statechart outside the W3C corpus, fragments included.
///
/// A fragment still carries expressions, so the refusal sweep reads it;
/// [`authored_machines`] is the narrower population for the sweeps that
/// judge a document as a machine.
pub fn authored_statecharts() -> Vec<String> {
    let root = repo_root();
    super::repository::paths_git_tracks(&["*.scxml"])
        .into_iter()
        .filter(|rel| !rel.starts_with("resources/"))
        .filter(|rel| {
            let Ok(text) = std::fs::read_to_string(root.join(rel)) else {
                return false;
            };
            // A document that does not parse carries no kind either; it
            // stays in, and whatever reads it reports its own refusal.
            matches!(
                sce_build::forge::parser::detect_kind(&text),
                Ok(None) | Ok(Some(sce_build::forge::model::ForgeKind::Statechart)) | Err(_)
            )
        })
        .collect()
}

/// Every tracked document some other tracked document pulls in.
///
/// Both inclusion forms are read, because both produce a file that is a
/// piece rather than a machine: W3C `<xi:include href>` and SCE's own
/// `<sce:use href>`. The href is resolved against the including
/// document's directory, which is how the expander resolves it.
pub fn included_documents() -> BTreeSet<String> {
    let root = repo_root();
    let mut included = BTreeSet::new();
    for rel in super::repository::paths_git_tracks(&["*.scxml"]) {
        let Ok(text) = std::fs::read_to_string(root.join(&rel)) else {
            continue;
        };
        let dir = Path::new(&rel)
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        for href in hrefs(&text) {
            // `#fragment` alone names a part of the SAME document.
            let path = href.split('#').next().unwrap_or_default();
            if path.is_empty() {
                continue;
            }
            let joined = dir.join(path);
            // A normalised, repo-relative spelling, so the result
            // compares against what `git ls-files` prints.
            if let Some(normalised) = normalise(&joined) {
                included.insert(normalised);
            }
        }
    }
    included
}

/// Every authored statechart that is a machine in its own right.
pub fn authored_machines() -> Vec<String> {
    let fragments = included_documents();
    authored_statecharts()
        .into_iter()
        .filter(|rel| !fragments.contains(rel))
        .collect()
}

/// `href="..."` values on the two inclusion elements, in document order.
///
/// Read with a scan rather than an XML parse because the caller's
/// question is "which files does the tree point at", and a document that
/// does not parse still points at files — several of the fixtures this
/// exists for are deliberately malformed.
fn hrefs(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (idx, _) in text.match_indices("href=") {
        let rest = &text[idx + "href=".len()..];
        let Some(quote) = rest.chars().next() else {
            continue;
        };
        if quote != '"' && quote != '\'' {
            continue;
        }
        let Some(end) = rest[1..].find(quote) else {
            continue;
        };
        out.push(rest[1..=end].to_string());
    }
    out
}

/// `a/b/../c.scxml` → `a/c.scxml`, and `None` for a path that climbs out
/// of the repository.
fn normalise(path: &Path) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            std::path::Component::ParentDir => {
                parts.pop()?;
            }
            std::path::Component::CurDir => {}
            _ => return None,
        }
    }
    Some(parts.join("/"))
}
