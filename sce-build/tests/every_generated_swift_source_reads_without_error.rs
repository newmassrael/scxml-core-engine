// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Swift output for every build of a product line is merged from the
//! sources its variants generate, read with tree-sitter-swift (ADR 0057 §10),
//! and a source the reader reads with an error is refused. So every Swift
//! source the generator wrote and the tree commits must read without one: a
//! template that writes what the reader cannot read fails here, where its
//! output is committed, and not in a consumer's build of a gated document.

use std::path::{Path, PathBuf};

use sce_build::variant_dialect::swift::unreadable;

/// The first line of every file the generator writes.
const GENERATED: &str = "// SCE-GENERATED — DO NOT EDIT";

/// Every `.swift` file under `dir`, SwiftPM's build output aside.
fn swift_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name != ".build") {
                swift_files(&path, out);
            }
        } else if path
            .extension()
            .is_some_and(|extension| extension == "swift")
        {
            out.push(path);
        }
    }
}

#[test]
fn every_generated_swift_source_reads_without_error() {
    let swift = Path::new(env!("CARGO_MANIFEST_DIR")).join("../backends/swift");
    let mut files = Vec::new();
    swift_files(&swift, &mut files);
    files.sort();
    let mut read = 0;
    let mut errors = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file).expect("a Swift source");
        if text.lines().next() != Some(GENERATED) {
            continue;
        }
        read += 1;
        if let Some(error) = unreadable(&text) {
            errors.push(format!("{}: {error}", file.display()));
        }
    }
    // Held to something: the tree commits hundreds of generated Swift sources.
    assert!(
        read > 100,
        "only {read} generated Swift sources found under {}",
        swift.display()
    );
    assert!(
        errors.is_empty(),
        "{} of {read} generated Swift sources read with an error:\n{}",
        errors.len(),
        errors.join("\n")
    );
}
