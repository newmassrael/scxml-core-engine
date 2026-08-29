// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The C surface and the header that declares it are held to each other.
//!
//! `sce-build/src/ffi.rs` exports the entry points; C++ reaches them
//! through `sce/include/scripting/SceLowering.h`, which is hand written.
//! Two hand-written halves of one contract drift, and this drift is the
//! expensive kind: a header promising a function the library no longer
//! exports compiles cleanly and fails at link, and one whose SIGNATURE
//! has quietly changed compiles cleanly, links cleanly, and is undefined
//! behaviour at run time.
//!
//! So the gate is name-for-name in both directions. Not "the header
//! mentions each name" — this repository has already paid for a check
//! that read prose rather than structure, on 2026-08-29, in the ledger
//! gate two files over. An entry point added to Rust and not declared
//! is as red as a declaration with nothing behind it.
//!
//! ## What this does NOT check
//!
//! Argument types. Reading C declarations well enough to compare them
//! against `extern "C" fn` signatures means parsing C, and a half-parser
//! that silently skipped what it could not read would be worse than
//! this: it would report agreement it never established. What actually
//! compares the signatures is the C++ compiler, on every build that
//! includes the header — `LuaEngine.cpp` does, and calls three of them.

use std::path::{Path, PathBuf};

const SURFACE: &str = "sce-build/src/ffi.rs";
const HEADER: &str = "sce/include/scripting/SceLowering.h";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    let path = repo_root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {rel}: {e}"))
}

/// The names Rust exports with C linkage, in declaration order.
fn exported_names(src: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in src.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("pub unsafe extern \"C\" fn ") else {
            continue;
        };
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        assert!(
            !name.is_empty(),
            "an `extern \"C\" fn` in {SURFACE} has no readable name: {line}"
        );
        names.push(name);
    }
    names
}

/// The names the header declares, in declaration order.
///
/// A declaration is a line ending in `);` that names an identifier
/// followed by `(`. Comments are cut first — this file's own prose says
/// `sce_lower_free` several times, and a scanner that counted those
/// would agree with anything.
fn declared_names(src: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_block_comment = false;
    for raw in src.lines() {
        let mut line = raw.trim();
        if in_block_comment {
            match line.find("*/") {
                Some(end) => {
                    line = line[end + 2..].trim();
                    in_block_comment = false;
                }
                None => continue,
            }
        }
        if let Some(start) = line.find("/*") {
            in_block_comment = !line[start..].contains("*/");
            line = line[..start].trim();
        }
        if let Some(start) = line.find("//") {
            line = line[..start].trim();
        }
        if !line.ends_with(");") {
            continue;
        }
        let Some(open) = line.find('(') else { continue };
        let name: String = line[..open]
            .chars()
            .rev()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        if name.starts_with("sce_") {
            names.push(name);
        }
    }
    names
}

#[test]
fn ffi_header_matches_surface() {
    let exported = exported_names(&read(SURFACE));
    let declared = declared_names(&read(HEADER));

    assert!(
        exported.len() >= 4,
        "{SURFACE} exports {} entry point(s). The decision this surface \
         carries names FOUR lowering functions plus a scope handle; a \
         surface this small means the module has been gutted, and an \
         empty-against-empty comparison below would still pass.",
        exported.len()
    );

    let mut missing: Vec<&String> = exported.iter().filter(|n| !declared.contains(n)).collect();
    missing.sort();
    assert!(
        missing.is_empty(),
        "{SURFACE} exports {missing:?}, which {HEADER} does not declare. \
         C++ cannot call what the header does not name, so the surface \
         grew a function nothing can reach."
    );

    let mut orphaned: Vec<&String> = declared.iter().filter(|n| !exported.contains(n)).collect();
    orphaned.sort();
    assert!(
        orphaned.is_empty(),
        "{HEADER} declares {orphaned:?}, which {SURFACE} does not export. \
         That is a clean compile and a link error — or, if some other \
         library happens to define the symbol, something far worse."
    );
}
