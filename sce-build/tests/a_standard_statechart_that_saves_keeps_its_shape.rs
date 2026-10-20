// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A standard statechart that saves keeps its shape (docs/adr/0045, "The shape
// of a tree").
//
// An application invokes a standard statechart as a child session, and a
// format-2 saved state holds that child's own state, bound to the shape of
// the document it runs. A restore whose child's document changed shape is
// refused, so an edit of a standard statechart that moves its shape refuses the
// saved states of every application that runs it. Such a document may change
// only as docs/adr/0044 allows, which leaves the shape where it was, and this
// test is where an edit that does otherwise is seen: in the repository, not on
// the devices that hold the states.
//
// Every statechart of `stdlib/` (a `datamodel="sce-static"` root that names no
// `sce:kind`: the library's schemas, enums, codecs and algorithms are not
// sessions) is generated for Rust, and the shape it binds a saved state to is
// held to `fixtures/standard_saved_shapes.json`. A statechart that saves and is
// not pinned fails, so a new one is added with its shape; a pin whose document
// no longer saves, or no longer exists, fails as well.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// Every `.scxml` file under `dir`, recursively, as a path from the
/// repository root with `/` between its parts.
fn documents(root: &Path, dir: &Path, found: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            documents(root, &path, found);
        } else if path.extension().is_some_and(|e| e == "scxml") {
            let relative = path.strip_prefix(root).expect("under the root");
            let parts: Vec<String> = relative
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            found.push(parts.join("/"));
        }
    }
}

/// The start tag of the document's `<scxml>` root.
fn root_tag(text: &str) -> &str {
    let start = text.find("<scxml").expect("a document has an <scxml> root");
    let end = text[start..].find('>').expect("the root tag ends") + start;
    &text[start..=end]
}

/// Whether the document is a statechart of the static data model: its root
/// declares `datamodel="sce-static"` and names no `sce:kind`.
fn is_static_statechart(text: &str) -> bool {
    let tag = root_tag(text);
    tag.contains(r#"datamodel="sce-static""#) && !tag.contains("sce:kind=")
}

/// The shape the generated Rust machine binds its saved state to, or `None`
/// for a machine generated without the save API.
fn rust_shape(document: &str) -> Option<String> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "standard-shape-{}-{}",
        std::process::id(),
        document.replace('/', "_")
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    let run = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .args([
            "generate",
            document,
            "-l",
            "rust",
            "-o",
            dir.to_str().unwrap(),
            "--error-format=json",
        ])
        .current_dir(repo_root())
        .output()
        .expect("spawn sce-codegen");
    assert!(
        run.status.success(),
        "{document} does not generate:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let mut text = String::new();
    for entry in std::fs::read_dir(&dir).expect("read scratch dir") {
        let file = entry.expect("dir entry").path();
        if file.extension().is_some_and(|e| e == "rs") {
            text.push_str(&std::fs::read_to_string(&file).expect("read generated file"));
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    const MARKER: &str = "const SHAPE: &'static str = \"";
    let start = text.find(MARKER)? + MARKER.len();
    let len = text[start..].find('"').expect("the shape ends");
    Some(text[start..start + len].to_string())
}

/// The pins, by path.
fn pinned() -> BTreeMap<String, String> {
    let text = std::fs::read_to_string(
        repo_root().join("sce-build/tests/fixtures/standard_saved_shapes.json"),
    )
    .expect("read the pinned shapes");
    let value: serde_json::Value = serde_json::from_str(&text).expect("the pins are JSON");
    value["shapes"]
        .as_object()
        .expect("`shapes` is an object")
        .iter()
        .map(|(path, shape)| {
            (
                path.clone(),
                shape.as_str().expect("a shape is a text").to_string(),
            )
        })
        .collect()
}

/// The library holds well over a hundred documents; a walk that read fewer
/// stopped finding it, and would pass on nothing.
const MIN_STANDARD_DOCUMENTS: usize = 100;

#[test]
fn a_standard_statechart_that_saves_keeps_its_shape() {
    let root = repo_root();
    let mut found = Vec::new();
    documents(&root, &root.join("stdlib"), &mut found);
    found.sort();
    assert!(
        found.len() >= MIN_STANDARD_DOCUMENTS,
        "read only {} standard documents",
        found.len()
    );
    let pins = pinned();
    let mut saving = BTreeMap::new();
    for document in &found {
        let text = std::fs::read_to_string(root.join(document)).expect("read document");
        if !is_static_statechart(&text) {
            continue;
        }
        if let Some(shape) = rust_shape(document) {
            saving.insert(document.clone(), shape);
        }
    }
    for (document, shape) in &saving {
        assert_eq!(
            pins.get(document),
            Some(shape),
            "{document} saves, and its shape is not the one pinned in \
             fixtures/standard_saved_shapes.json. An edit that moves it refuses every saved \
             state of an application that runs it (docs/adr/0045); a new standard statechart \
             is pinned with its shape."
        );
    }
    for document in pins.keys() {
        assert!(
            saving.contains_key(document),
            "{document} is pinned but is not a standard statechart that saves"
        );
    }
}

#[test]
fn the_walk_tells_a_statechart_from_the_other_kinds_of_document() {
    // The control for `is_static_statechart`: a session is told from a schema,
    // and a statechart under another data model is not one a state is saved of.
    let statechart = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="sce-static" name="m">"#;
    assert!(is_static_statechart(statechart));
    assert!(!is_static_statechart(&statechart.replace(
        r#" name="m""#,
        r#" sce:kind="event-schema" name="m""#
    )));
    assert!(!is_static_statechart(
        &statechart.replace("sce-static", "ecmascript")
    ));
}
