// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

/// The library-relative name an `sce:std/...` path denotes, `.` and `..`
/// folded lexically; `None` for a path that is not one, or that leaves the
/// library root.
fn folded_name(path: &Path) -> Option<String> {
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
    Some(parts.join("/"))
}

/// The standard document at `path` (an `sce:std/...` path, `.` and `..`
/// folded lexically), or `None` when the library has no such document.
pub fn lookup(path: &Path) -> Option<&'static str> {
    let name = folded_name(path)?;
    EMBEDDED_STDLIB
        .iter()
        .find(|(entry, _)| *entry == name)
        .map(|(_, content)| *content)
}

/// One spelling for the document an import resolved to (`resolve`), so two
/// documents that import it by different `src` — `hlc_timestamp.scxml`
/// from beside it, `sce:std/merge/hlc_timestamp.scxml` from anywhere —
/// compare equal. A standard document is its folded `sce:std/...` path; a
/// file is its canonical path, or its path as written when the file cannot
/// be canonicalized (the import's own read reports that).
pub fn identity(path: &Path) -> String {
    if let Some(name) = folded_name(path) {
        return format!("{SCHEME}{name}");
    }
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}

/// Every standard document, as `(sce:std path, contents)`, sorted by path.
pub fn documents() -> impl Iterator<Item = (String, &'static str)> {
    EMBEDDED_STDLIB
        .iter()
        .map(|(name, content)| (format!("{SCHEME}{name}"), *content))
}

/// The namespace SCE's own elements are in (`sce:import` among them).
const SCE_NAMESPACE: &str = "http://sce.dev/ext";

/// The namespace of the SCXML elements a document is written in (`<invoke>` among them).
const SCXML_NAMESPACE: &str = "http://www.w3.org/2005/07/scxml";

/// The `src` of every `<sce:import>` a document names, and of every
/// `<invoke>` that names a standard statechart as its child (`sce:std/...`),
/// in document order: the documents a machine generated from it is made of.
///
/// Read from its elements, never from its text: a comment or a string that
/// mentions a standard name imports nothing. A document that is not
/// well-formed names none here; its own parse reports why. An `<invoke src>`
/// that names a file is not a standard document and is left to the reader of
/// that file.
pub fn import_sources(content: &str) -> Vec<String> {
    let Ok(document) = roxmltree::Document::parse(content) else {
        return Vec::new();
    };
    document
        .descendants()
        .filter(|node| node.is_element())
        .filter_map(|node| {
            let name = node.tag_name();
            let src = node.attribute("src")?;
            let imported = name.name() == "import" && name.namespace() == Some(SCE_NAMESPACE);
            let invoked = name.name() == "invoke"
                && name.namespace() == Some(SCXML_NAMESPACE)
                && src.starts_with(SCHEME);
            (imported || invoked).then(|| src.to_string())
        })
        .collect()
}

/// The standard documents `seeds` name and every standard document they
/// import, transitively — each once, as `(sce:std path, contents)`, sorted
/// by path.
///
/// This is the part of the library a document generated from `seeds`
/// depends on. A seed or an import the library has no document for is left
/// out: the import's own read refuses it. A cycle ends where it began.
pub fn closure(
    seeds: impl IntoIterator<Item = PathBuf>,
) -> std::collections::BTreeMap<String, &'static str> {
    let mut found = std::collections::BTreeMap::new();
    let mut pending: Vec<PathBuf> = seeds.into_iter().collect();
    while let Some(path) = pending.pop() {
        let Some(name) = folded_name(&path) else {
            continue;
        };
        let key = format!("{SCHEME}{name}");
        if found.contains_key(&key) {
            continue;
        }
        let Some(content) = lookup(&path) else {
            continue;
        };
        found.insert(key.clone(), content);
        let base = Path::new(&key).parent().unwrap_or(Path::new(SCHEME));
        pending.extend(
            import_sources(content)
                .iter()
                .map(|src| resolve(base, src))
                .filter(|resolved| names_standard(resolved)),
        );
    }
    found
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
    fn a_standard_statechart_a_document_invokes_is_one_of_its_sources() {
        // The child a parent invokes by its library name is part of what the
        // parent's machine is made of, and so is every standard document the
        // child imports; a child named by a file is not a standard document.
        let parent = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="s">
  <sce:import kind="event-schema" src="sce:std/shared/record_register.scxml" as="Register"/>
  <state id="s">
    <invoke type="scxml" id="list" src="sce:std/shared/record_list.scxml"/>
    <invoke type="scxml" id="other" src="worker.scxml"/>
  </state>
</scxml>"#;
        assert_eq!(
            import_sources(parent),
            [
                "sce:std/shared/record_register.scxml",
                "sce:std/shared/record_list.scxml"
            ]
        );
        let reached = closure(import_sources(parent).into_iter().map(PathBuf::from));
        for name in [
            "sce:std/shared/record_list.scxml",
            "sce:std/shared/record_write.scxml",
            "sce:std/merge/mv_value.scxml",
        ] {
            assert!(reached.contains_key(name), "{name} is not reached");
        }
    }

    #[test]
    fn a_path_on_disk_is_resolved_as_before() {
        assert_eq!(
            resolve(Path::new("/doc/dir"), "schema.scxml"),
            PathBuf::from("/doc/dir/schema.scxml")
        );
    }
}
