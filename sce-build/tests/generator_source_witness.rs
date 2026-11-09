// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// End-to-end probe for `sce-codegen verify-generator`.
//
// The unit tests beside `generator_witness.rs` pin the digest's algebra.
// This target pins the thing that actually decides whether the check is
// worth having: that the *real binary*, invoked the way a build system
// invokes it, says yes to a tree it was built from and no to one it was
// not.
//
// That distinction is not academic here. The previous attempt at this
// check passed a two-way probe locally and still had to be reverted the
// same day, because the probe never crossed the boundary the check lives
// on: the tree arrives on a build machine through `rsync -rlpgoD
// --checksum`, without `-t`, so every file's mtime is rewritten and every
// source looks newer than the binary. A timestamp comparison refused every
// remote configure for a generator that was current.
//
// So the probe below moves the tree instead of editing it in place: it
// copies the witness set to a different prefix — which is what the
// transfer does — and asserts the verdict does not change. Then it edits
// one byte and asserts it does. A check that only ever ran against the
// checkout it was built in could pass the second half and still be the
// same defect.

use std::path::{Path, PathBuf};
use std::process::Command;

mod common;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

struct Verdict {
    status: i32,
    stderr: String,
}

impl Verdict {
    /// The `code` of the single NDJSON record, or `None` on success.
    fn code(&self) -> Option<String> {
        let line = self.stderr.lines().find(|l| l.starts_with('{'))?;
        let key = "\"code\":\"";
        let start = line.find(key)? + key.len();
        let rest = &line[start..];
        Some(rest[..rest.find('"')?].to_string())
    }
}

fn verify_generator(root: &Path) -> Verdict {
    let out = Command::new(sce_codegen_bin())
        .arg("verify-generator")
        .arg("--root")
        .arg(root)
        .arg("--error-format=json")
        .output()
        .expect("sce-codegen runs");
    Verdict {
        status: out.status.code().expect("no signal"),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// Copy the witness set — and only the witness set — into `dest`.
///
/// Deliberately not a copy of the whole repository: a witness that
/// silently depended on a file outside its declared set would still pass
/// against a full copy. Reproducing exactly what
/// `generator_witness::WITNESS_FILES` and `WITNESS_TREES` name is what
/// makes a green verdict here evidence that the declared set is the real
/// one.
fn stage_witness_set(dest: &Path) {
    let src = repo_root();
    for file in sce_build::generator_witness::WITNESS_FILES {
        let to = dest.join(file);
        std::fs::create_dir_all(to.parent().expect("witness member has a parent"))
            .expect("create parent");
        std::fs::copy(src.join(file), &to).unwrap_or_else(|e| panic!("copy {file}: {e}"));
    }
    for tree in sce_build::generator_witness::WITNESS_TREES {
        copy_tree(&src.join(tree), &dest.join(tree));
    }
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("create dir");
    for entry in std::fs::read_dir(from).expect("read dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        let target = to.join(entry.file_name());
        if entry.file_type().expect("file type").is_dir() {
            copy_tree(&path, &target);
        } else {
            std::fs::copy(&path, &target).expect("copy file");
        }
    }
}

/// The binary under test was built by cargo from this checkout moments
/// ago, so it must agree with it. A red here means the embedded digest is
/// not being refreshed when sources change — the failure mode that makes
/// the whole check refuse correct builds.
#[test]
fn the_binary_agrees_with_the_tree_it_was_built_from() {
    let verdict = verify_generator(&repo_root());
    assert_eq!(
        verdict.status, 0,
        "a freshly built generator was called stale: {}",
        verdict.stderr
    );
    assert!(
        verdict.stderr.is_empty(),
        "success must be silent — this runs on every configure: {}",
        verdict.stderr
    );
}

/// The boundary the previous attempt died on, stated as a test.
///
/// Same bytes, different prefix, no mtime relationship preserved — which
/// is what arrives on a build machine. The verdict must be identical to
/// the one taken in the original checkout.
#[test]
fn the_same_bytes_at_a_different_prefix_are_still_fresh() {
    let scratch = tempfile::tempdir().expect("scratch dir");
    stage_witness_set(scratch.path());

    let verdict = verify_generator(scratch.path());
    assert_eq!(
        verdict.status, 0,
        "the witness read something outside its declared set, or read the \
         checkout location into the digest: {}",
        verdict.stderr
    );
}

/// The other direction: one byte of one generator source, changed after
/// the binary was built. This is the actual field failure — a tree whose
/// sources are current, generating with a binary that predates them.
#[test]
fn a_source_edit_the_binary_predates_is_refused() {
    let scratch = tempfile::tempdir().expect("scratch dir");
    stage_witness_set(scratch.path());
    assert_eq!(
        verify_generator(scratch.path()).status,
        0,
        "the staged copy must start out fresh, or the edit below proves nothing"
    );

    let edited = scratch.path().join("sce-build/src/generator_witness.rs");
    let mut bytes = std::fs::read(&edited).expect("read a staged source");
    bytes.extend_from_slice(b"\n// one byte the binary never saw\n");
    std::fs::write(&edited, &bytes).expect("write a staged source");

    let verdict = verify_generator(scratch.path());
    assert_eq!(
        verdict.status, 20,
        "an edited generator source did not refuse: {}",
        verdict.stderr
    );
    assert_eq!(
        verdict.code().as_deref(),
        Some("cli/generator-source-drift"),
        "wrong code on stderr: {}",
        verdict.stderr
    );
    assert!(
        verdict
            .stderr
            .contains("cargo build --bin sce-codegen --features cli -p sce-build"),
        "the refusal must carry the command that repairs it: {}",
        verdict.stderr
    );
}

/// Deletion rather than edit. A source file that vanishes is the direction
/// an edit-only probe cannot reach, and it is the one a partial transfer
/// produces.
#[test]
fn a_deleted_source_is_refused() {
    let scratch = tempfile::tempdir().expect("scratch dir");
    stage_witness_set(scratch.path());
    std::fs::remove_file(scratch.path().join("sce-build/src/generator_witness.rs"))
        .expect("remove a staged source");

    let verdict = verify_generator(scratch.path());
    assert_eq!(
        verdict.status, 20,
        "a deleted generator source did not refuse: {}",
        verdict.stderr
    );
    assert_eq!(
        verdict.code().as_deref(),
        Some("cli/generator-source-drift")
    );
}

/// A tree with no witness in it is not a tree that disagrees. Reporting it
/// as drift would send the reader to rebuild against something that was
/// never the problem — the misattribution this repository has paid for
/// three times in gates that could not run.
#[test]
fn a_tree_without_the_witness_set_is_unverifiable_not_stale() {
    let scratch = tempfile::tempdir().expect("scratch dir");
    stage_witness_set(scratch.path());
    std::fs::remove_file(scratch.path().join("Cargo.lock")).expect("remove the lock");

    let verdict = verify_generator(scratch.path());
    assert_eq!(
        verdict.status, 20,
        "a tree missing a witness member was accepted: {}",
        verdict.stderr
    );
    assert_eq!(
        verdict.code().as_deref(),
        Some("cli/generator-source-unverifiable"),
        "an unreadable tree was reported as drift: {}",
        verdict.stderr
    );
    assert!(
        verdict.stderr.contains("Cargo.lock"),
        "the refusal must name the member it could not read: {}",
        verdict.stderr
    );
}

// ── The declared set against what the compiler read ───────────────────
//
// The tests above stage exactly the declared set and ask whether the
// verdict holds there, which proves the digest reads nothing outside it.
// They cannot prove the converse: that every file the binary compiles in
// is inside it. A file pulled in by `include_str!` from outside
// `sce-build/src` and `stdlib` (the Rust and Go harnesses `w3c_suite.rs`
// writes beside the W3C machines, a runtime's `policy` source, a schema)
// changes what the generator emits without moving the digest, so
// `verify-generator` calls a stale binary current and an action key built
// on the digest (ADR 0097, answer 1) names two generators alike.
//
// The compiler knows the set exactly. Cargo writes it next to the binary
// as a Make-format dep-info file (`<bin>.d`): every source file rustc read
// for the binary and for the local packages it links, every file a macro
// included, and every file a local build script watches. The test reads
// that list and requires each local member of it to be a witness member.

/// The paths a Make-format dep-info file declares as the prerequisites of
/// its one target. Cargo escapes a space inside a path as `\ `.
fn dep_info_prerequisites(text: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for line in text.lines() {
        let Some((_, prerequisites)) = line.split_once(": ") else {
            continue;
        };
        let mut current = String::new();
        let mut chars = prerequisites.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '\\' if chars.peek() == Some(&' ') => {
                    current.push(' ');
                    chars.next();
                }
                ' ' => {
                    if !current.is_empty() {
                        paths.push(PathBuf::from(std::mem::take(&mut current)));
                    }
                }
                _ => current.push(c),
            }
        }
        if !current.is_empty() {
            paths.push(PathBuf::from(current));
        }
    }
    paths
}

/// Each file of this checkout the compiler read to build `sce-codegen`, as a
/// path relative to the checkout. Files outside the checkout (the registry,
/// the toolchain) and files the build itself wrote (under the target
/// directory, wherever it is linked to) are not sources of this tree.
fn compiled_in_sources() -> Vec<PathBuf> {
    let bin = sce_codegen_bin();
    let dep_info = bin.with_extension("d");
    let text = std::fs::read_to_string(&dep_info).unwrap_or_else(|e| {
        panic!(
            "cargo writes the binary's dep-info at {}, and it could not be read: {e}",
            dep_info.display()
        )
    });
    let root = repo_root()
        .canonicalize()
        .expect("the checkout has a canonical path");
    let target = bin
        .parent()
        .and_then(Path::parent)
        .expect("the binary sits in <target>/<profile>")
        .canonicalize()
        .expect("the target directory has a canonical path");
    let mut sources: Vec<PathBuf> = dep_info_prerequisites(&text)
        .into_iter()
        .filter_map(|path| {
            // Cargo writes this file when it builds `sce-codegen` as a target
            // it was asked for, and not when it builds it for an integration
            // test, so in a target directory another tree built last it is
            // that tree's list. A file of the list that is not here says so;
            // judging this tree by it would judge some other tree.
            let canonical = path.canonicalize().unwrap_or_else(|e| {
                panic!(
                    "the dep-info at {} names {}, which is not on disk ({e}), so it was \
                     written for another tree. Build the generator as a target first \
                     (`cargo build --bin sce-codegen --features cli -p sce-build`, as \
                     scripts/gates/workspace-tests.sh does) and run this test again",
                    dep_info.display(),
                    path.display()
                )
            });
            // The build directory is the build's own output. A directory is
            // a watch on its listing (a build script re-runs when a file is
            // added to it), and the files it lists are what a witness covers.
            if canonical.starts_with(&target) || canonical.is_dir() {
                return None;
            }
            canonical.strip_prefix(&root).ok().map(Path::to_path_buf)
        })
        .collect();
    sources.sort();
    sources.dedup();
    sources
}

#[test]
fn every_file_of_this_tree_the_binary_compiles_in_is_witnessed() {
    let root = repo_root();
    let witnessed: std::collections::BTreeSet<PathBuf> =
        sce_build::generator_witness::witness_paths(&root)
            .expect("the witness set of this checkout is readable")
            .into_iter()
            .map(|path| {
                path.strip_prefix(&root)
                    .expect("a witness member is under the checkout")
                    .to_path_buf()
            })
            .collect();
    let sources = compiled_in_sources();
    assert!(
        sources.iter().any(|p| p.starts_with("sce-build/src")),
        "the dep-info named no source of sce-build at all, so it is not the \
         binary's: {sources:?}"
    );

    // Two exemptions, each a file the binary reads that no generated file
    // depends on. The template tree is read by every run from disk and is an
    // input of each generation (`RUN_TIME_TREE`). The git HEAD and the ref it
    // names are watched by the build script for the commit stamp
    // (`GENERATOR_COMMIT`), a label of diagnostics and exports that no
    // generated file carries: the drift header leaves it out on purpose
    // (`forge::drift`), since it would re-stamp every committed tree on every
    // commit.
    let unwitnessed: Vec<&PathBuf> = sources
        .iter()
        .filter(|path| {
            !witnessed.contains(*path)
                && !path.starts_with(sce_build::generator_witness::RUN_TIME_TREE)
                && !path.starts_with(".git")
        })
        .collect();
    assert!(
        unwitnessed.is_empty(),
        "sce-codegen compiles in files its witness does not cover, so editing \
         one changes what it emits and leaves GENERATOR_SOURCE_DIGEST where it \
         was. Add each to generator_witness::WITNESS_FILES (or its directory to \
         WITNESS_TREES):\n  {}",
        unwitnessed
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

/// The parser above, on the two shapes cargo writes: a plain list, and a path
/// holding an escaped space. A parser that split on every space would turn the
/// second into two paths that do not exist and fail the test above for a
/// reason that is not the tree's.
#[test]
fn a_dep_info_line_is_read_path_by_path() {
    let text = "/t/debug/sce-codegen: /r/a.rs /r/dir\\ with\\ space/b.rs  /r/c.rs\n\n/r/a.rs:\n";
    assert_eq!(
        dep_info_prerequisites(text),
        vec![
            PathBuf::from("/r/a.rs"),
            PathBuf::from("/r/dir with space/b.rs"),
            PathBuf::from("/r/c.rs"),
        ]
    );
}

// ── The CMake half ────────────────────────────────────────────────────
//
// The tests above prove the binary reaches the right verdict. They say
// nothing about whether anything acts on it, and a check nobody runs is
// the same as no check — `cmake/SCEFindCodegen.cmake` is where it becomes
// a refusal.
//
// Driven with a stub generator rather than a mutated checkout, because the
// module deliberately points `--root` at this repository: making the real
// verdict flip would mean editing the tree the developer is working in.
// The stub decides the verdict; what is under test here is what CMake does
// with it. Composed with the tests above — which prove the real binary
// produces that verdict from a real tree — the chain is covered end to end.

fn cmake_bin() -> Option<PathBuf> {
    let probe = Command::new("cmake").arg("--version").output().ok()?;
    probe.status.success().then(|| PathBuf::from("cmake"))
}

/// Configure a throwaway project that includes the real module, with
/// `SCE_CODEGEN` pre-set to a stub exiting `code`. Returns CMake's own
/// exit status and combined output.
fn configure_with_stub_generator(dir: &Path, code: i32) -> (bool, String) {
    let stub = dir.join("stub-sce-codegen");
    common::executable::install_executable(
        &stub,
        format!(
            "#!/bin/sh\n\
             echo 'stub refusal: rebuild it with: cargo build --bin sce-codegen \
             --features cli -p sce-build' >&2\n\
             exit {code}\n"
        ),
    )
    .expect("install stub");

    std::fs::write(
        dir.join("CMakeLists.txt"),
        format!(
            "cmake_minimum_required(VERSION 3.16)\n\
             project(sce_witness_probe NONE)\n\
             set(SCE_CODEGEN \"{}\" CACHE FILEPATH \"\" FORCE)\n\
             include(\"{}/cmake/SCEFindCodegen.cmake\")\n",
            stub.display(),
            repo_root().display()
        ),
    )
    .expect("write CMakeLists");

    let out = Command::new(cmake_bin().expect("cmake probed"))
        .arg("-S")
        .arg(dir)
        .arg("-B")
        .arg(dir.join("b"))
        .output()
        .expect("cmake runs");
    let mut combined = String::from_utf8_lossy(&out.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.success(), combined)
}

#[test]
fn cmake_stops_when_the_generator_reports_it_is_stale() {
    let Some(_) = cmake_bin() else {
        // Loud rather than silent. A skip that reads like a pass is how a
        // gate ends up believed on a machine where it never ran.
        panic!("cmake is absent, so the CMake half of this witness went unchecked");
    };
    let scratch = tempfile::tempdir().expect("scratch dir");
    let (ok, output) = configure_with_stub_generator(scratch.path(), 20);

    assert!(
        !ok,
        "configure continued past a generator that reported itself stale:\n{output}"
    );
    assert!(
        output.contains("cargo build --bin sce-codegen"),
        "the refusal dropped the repair command on its way through CMake:\n{output}"
    );
}

#[test]
fn cmake_proceeds_when_the_generator_reports_it_is_current() {
    let Some(_) = cmake_bin() else {
        panic!("cmake is absent, so the CMake half of this witness went unchecked");
    };
    let scratch = tempfile::tempdir().expect("scratch dir");
    let (ok, output) = configure_with_stub_generator(scratch.path(), 0);

    assert!(
        ok,
        "configure refused a generator that reported itself current — the \
         false-refusal direction that got the previous attempt reverted:\n{output}"
    );
}

/// The omission that keeps the check from refusing builds that are
/// correct. A native `sce-codegen` reads the Jinja2 tree from disk per
/// run, so an edited template is already in effect for the binary CMake
/// invokes; demanding a rebuild for one would be a false refusal, which is
/// exactly what got the mtime attempt reverted — a `cargo fmt` there was
/// enough to stop the next C++ build.
#[test]
fn a_template_edit_does_not_demand_a_rebuild() {
    let scratch = tempfile::tempdir().expect("scratch dir");
    stage_witness_set(scratch.path());
    let templates = scratch.path().join("tools/codegen/templates");
    std::fs::create_dir_all(&templates).expect("create template dir");
    std::fs::write(
        templates.join("state_machine.jinja2"),
        b"edited after the build",
    )
    .expect("write a template");

    let verdict = verify_generator(scratch.path());
    assert_eq!(
        verdict.status, 0,
        "a template edit demanded a generator rebuild it does not need: {}",
        verdict.stderr
    );
}
