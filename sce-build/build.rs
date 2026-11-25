// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Embeds the digest of the generator's sources and its Jinja2 template
//! tree into the binary.
//!
//! # Template registry
//!
//! The template set has exactly one source of truth: the on-disk tree at
//! `tools/codegen/templates/`. This script walks it and emits an
//! `include_str!` registry so the in-memory template path (WASM, and any
//! caller without a filesystem) cannot carry a hand-maintained copy of
//! that list. A hand-maintained copy is not a hypothetical failure: the
//! `_macros/` family added by the synth 5-O round was never added to the previous
//! hardcoded list, and because `include_str!` only ever names files that
//! *do* exist, nothing rejected the omission at compile time — the
//! templates rendered fine natively and failed only in the browser.
//!
//! Deriving the list removes the ability to state it twice. What it
//! cannot check is that a template's `{% import %}` / `{% include %}`
//! targets resolve, since minijinja binds those at render time; that is
//! the job of `template_registry::tests` in the library crate.
//!
//! # Generator identity
//!
//! The crate version is frozen at `0.1.0` under the pre-1.0 no-versioning
//! rule, so it identifies nothing. The binary names itself by the digest
//! `src/generator_witness.rs` takes of the sources it is compiled from
//! (`SCE_GENERATOR_SOURCE_DIGEST`), and by nothing else this script reads:
//! no commit, no clock, no host name, no environment variable. A build
//! output may carry only what is derived from the inputs that made it, so
//! two builds of the same sources write the same binary whatever commit,
//! worktree or machine they ran at, and a build system that compares
//! artifacts by content keeps the work downstream of a generator that did
//! not change.
//!
//! The commit stood here until 2026-11. It made every commit a new binary
//! (and re-ran this script, and recompiled the crate) when no source of the
//! generator had moved, and it was the wrong identity besides: a build
//! compiles the tree in front of it, uncommitted edits included, so two
//! binaries could agree on the commit and differ in every line the
//! generator runs. `sce-build/tests/generator_source_witness.rs` holds this
//! script to what it may hand the compiler.

use std::path::{Path, PathBuf};

// The witness algorithm is included rather than imported: a build script
// is its own crate and cannot depend on the library it builds. Including
// the one implementation is what makes the two halves of the check —
// embedded here, recomputed by `sce-codegen verify-generator` — provably
// the same function rather than two spellings that must be kept in step.
//
// Wrapped in a module rather than included flat so the file keeps its own
// imports and its `//!` header, and so neither side has to avoid names the
// other already uses.
mod generator_witness {
    include!("src/generator_witness.rs");
}
use generator_witness::{digest_hex, witness_paths, DIGEST_UNAVAILABLE, RUN_TIME_TREE};

fn main() {
    emit_source_digest();
    emit_template_registry();
    emit_stdlib_registry();
}

/// Workspace root, one level above this crate.
fn workspace_root() -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by cargo"))
        .join("..")
}

/// Embed the content witness described in `src/generator_witness.rs`.
///
/// Absence is a supported build shape, not a failure: a vendored crate has
/// no workspace `Cargo.lock` beside it, and refusing to compile there would
/// break consumers to serve a check that only this repository's build
/// system runs. The sentinel is a distinct value rather than a plausible
/// digest, so `verify-generator` reports "cannot be checked" instead of
/// reporting drift it never measured.
fn emit_source_digest() {
    let root = workspace_root();
    match digest_hex(&root) {
        Ok(digest) => {
            println!("cargo:rustc-env=SCE_GENERATOR_SOURCE_DIGEST={digest}");
            // One watch per file. A watch on the directory would move only
            // when a file is added or removed, leaving the embedded digest
            // describing a tree that had since been edited — and a stale
            // digest makes the check refuse a generator that is current,
            // which is the failure the reverted mtime attempt shipped.
            match witness_paths(&root) {
                Ok(paths) => {
                    for path in paths {
                        watch(&path);
                    }
                }
                Err(e) => panic!("witness digest succeeded but its path list did not: {e}"),
            }
        }
        Err(e) => {
            println!("cargo:rustc-env=SCE_GENERATOR_SOURCE_DIGEST={DIGEST_UNAVAILABLE}");
            println!("cargo:warning=sce-build: generator source witness unavailable ({e}); `sce-codegen verify-generator` cannot check this binary");
        }
    }
}

/// SCE's standard algorithm library, relative to the workspace root.
/// Embedded for the reason the templates are: an `sce:std/...` import
/// names a document of the library this generator was built with, and
/// must resolve to the same bytes on every machine — a search path on
/// disk would let two machines resolve one name to two documents.
const STDLIB_ROOT: &str = "stdlib";

/// The template tree is the witness's run-time tree
/// ([`generator_witness::RUN_TIME_TREE`]): the directory the witness leaves
/// out because a native run reads it from disk. Named once, there, so the
/// tree embedded here and the tree the witness exempts cannot be two.
fn emit_template_registry() {
    emit_embedded_tree(
        RUN_TIME_TREE,
        "jinja2",
        "EMBEDDED_TEMPLATES",
        "embedded_templates.rs",
    );
}

fn emit_stdlib_registry() {
    emit_embedded_tree(
        STDLIB_ROOT,
        "scxml",
        "EMBEDDED_STDLIB",
        "embedded_stdlib.rs",
    );
}

/// Walk the tree at `tree_root`, a path relative to the workspace root, and
/// write `$OUT_DIR/<file>`, a static `<static_name>` listing every
/// `.<extension>` file in it.
///
/// Entries are `(name, contents)` where `name` is the path relative to
/// the tree root with `/` separators — the same name the filesystem
/// loader derives, so a caller can swap sources without renaming.
/// The list is sorted so the emitted file is a function of the tree
/// alone and not of `read_dir` order.
fn emit_embedded_tree(tree_root: &str, extension: &str, static_name: &str, file: &str) {
    let workspace = workspace_root();
    // Canonicalize so the emitted `include_str!` paths are absolute:
    // the generated file lives in OUT_DIR, where a relative path would
    // resolve against the wrong directory.
    let root = workspace
        .join(tree_root)
        .canonicalize()
        .unwrap_or_else(|e| {
            // The tree is part of this repository, so absence means a
            // broken checkout rather than a supported build shape. Failing
            // here names the missing path; emitting an empty registry
            // instead would defer the failure to whatever consumer happened
            // to ask for an entry first.
            panic!(
                "embedded tree not found at {}/{tree_root}: {e}",
                workspace.display()
            )
        });

    let mut entries = Vec::new();
    collect_tree(&root, &root, extension, &mut entries);
    entries.sort();

    // Watch the tree root so a file added or removed re-runs this
    // script; `include_str!` already forces a rebuild when the contents
    // of an *already listed* file change.
    watch(&root);

    let mut out = format!(
        "// @generated by sce-build/build.rs from {} — do not edit.\n\
         pub(crate) static {static_name}: &[(&str, &str)] = &[\n",
        tree_root
    );
    for (name, path) in &entries {
        out.push_str(&format!(
            "    ({:?}, include_str!({:?})),\n",
            name,
            path.display().to_string()
        ));
    }
    out.push_str("];\n");

    let dest = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is set by cargo")).join(file);
    std::fs::write(&dest, out).unwrap_or_else(|e| panic!("Cannot write {}: {e}", dest.display()));
}

/// Recursively collect `(relative_name, absolute_path)` for every
/// `.<extension>` file under `current`.
fn collect_tree(root: &Path, current: &Path, extension: &str, out: &mut Vec<(String, PathBuf)>) {
    let entries = std::fs::read_dir(current)
        .unwrap_or_else(|e| panic!("Cannot read {}: {e}", current.display()));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|e| panic!("Dir entry error under {}: {e}", current.display()))
            .path();
        if path.is_dir() {
            // Watched per directory too, so a file added in a subdirectory
            // re-runs this script.
            watch(&path);
            collect_tree(root, &path, extension, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some(extension) {
            let rel = path
                .strip_prefix(root)
                .unwrap_or_else(|e| panic!("Path error for {}: {e}", path.display()));
            out.push((rel.to_string_lossy().replace('\\', "/"), path));
        }
    }
}

/// Ask cargo to re-run this script when `path` changes.
///
/// ⚠ `path` must exist. Cargo reads a watched path that does not as
/// changed on every build, so the script re-runs each time and everything
/// compiled from its output is rebuilt after it. Measured 2026-09-24: a
/// watch on the git branch ref (gone with the commit stamp) named a file
/// absent from every linked worktree, so each
/// cargo invocation in one recompiled this crate, and each throwaway crate
/// `forge_conformance` compiles against the forge runtime re-ran that
/// runtime's build script behind it — one every four and a half minutes,
/// in series on their shared target directory.
fn watch(path: &Path) {
    println!("cargo:rerun-if-changed={}", path.display());
}
