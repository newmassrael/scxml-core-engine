// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! SCE's standard algorithm library — the documents an `sce:std/...`
//! import names.
//!
//! General-purpose algorithms (date and time arithmetic, merge primitives) are
//! written once, as SCXML algorithm documents under the repository's
//! `stdlib/`, and every consumer imports the same document rather than
//! writing its own copy in each language:
//!
//! ```xml
//! <sce:import kind="algorithm" src="sce:std/time/days_from_civil.scxml" as="DaysFromCivil"/>
//! ```
//!
//! The documents are embedded in this binary by `build.rs` and are never
//! looked up on disk: a search path would let two machines resolve one name
//! to two documents, and "one implementation" is the reason the library
//! exists. So which version of a standard document a machine was generated
//! against is decided by the generator that generated it, and the tree is
//! part of the generator's source witness.
//!
//! A standard document may import another by a relative `src`, which
//! resolves inside the library exactly as a relative import between two
//! files resolves on disk.

use std::path::{Component, Path, PathBuf};

include!(concat!(env!("OUT_DIR"), "/embedded_stdlib.rs"));

/// The prefix an import `src` names the standard library by.
pub const SCHEME: &str = "sce:std/";

/// Whether `src` (or a path already resolved from one) names a standard
/// document.
pub fn names_standard(path: &Path) -> bool {
    path.to_str().is_some_and(|p| p.starts_with(SCHEME))
}

/// The path `src` resolves to, imported by a document in `base_dir`.
///
/// A standard document is named by its `sce:std/...` path wherever it is
/// imported from; anything else is resolved against the importing
/// document's directory — which, for an import made BY a standard
/// document, is itself an `sce:std/...` path. Every reader that turns an
/// import into a path goes through here, so a cycle key, a label and a
/// nested import's base agree on where an import points.
pub fn resolve(base_dir: &Path, src: &str) -> PathBuf {
    if src.starts_with(SCHEME) {
        PathBuf::from(src)
    } else {
        base_dir.join(src)
    }
}

/// The standard document at `path` (an `sce:std/...` path, `.` and `..`
/// folded lexically), or `None` when the library has no such document.
pub fn lookup(path: &Path) -> Option<&'static str> {
    let relative = path.to_str()?.strip_prefix(SCHEME)?;
    let mut parts: Vec<&str> = Vec::new();
    for component in Path::new(relative).components() {
        match component {
            Component::Normal(part) => parts.push(part.to_str()?),
            Component::CurDir => {}
            // `..` past the library root leaves it, and nothing outside it
            // is a standard document.
            Component::ParentDir => {
                parts.pop()?;
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    let name = parts.join("/");
    EMBEDDED_STDLIB
        .iter()
        .find(|(entry, _)| *entry == name)
        .map(|(_, content)| *content)
}

/// Every standard document, as `(sce:std path, contents)`, sorted by path.
pub fn documents() -> impl Iterator<Item = (String, &'static str)> {
    EMBEDDED_STDLIB
        .iter()
        .map(|(name, content)| (format!("{SCHEME}{name}"), *content))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_relative_import_inside_the_library_stays_in_it() {
        let base = resolve(Path::new("/anywhere"), "sce:std/time/days_from_civil.scxml");
        let sibling = resolve(base.parent().unwrap(), "../time/./days_from_civil.scxml");
        assert_eq!(
            sibling,
            PathBuf::from("sce:std/time/../time/./days_from_civil.scxml")
        );
        assert!(
            lookup(&sibling).is_some(),
            "lexical folding finds the same document"
        );
    }

    #[test]
    fn nothing_outside_the_library_is_a_standard_document() {
        assert!(lookup(Path::new("sce:std/../stdlib/time/days_from_civil.scxml")).is_none());
        assert!(lookup(Path::new("sce:std/time/no_such_document.scxml")).is_none());
        assert!(lookup(Path::new("time/days_from_civil.scxml")).is_none());
    }

    #[test]
    fn a_path_on_disk_is_resolved_as_before() {
        assert_eq!(
            resolve(Path::new("/doc/dir"), "schema.scxml"),
            PathBuf::from("/doc/dir/schema.scxml")
        );
    }
}
