// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `scripts/generated_diff.sh`, the generated diff of a change (ADR 0097,
// section 6, and its amendment of 2026-11-16, item 6): every tree two commits
// declare, materialized at each, and what differs, one patch per tree with
// paths of the repository and a summary of the files and lines.
//
// The script is run in a repository of its own, with a generator of the
// fixture's in place of `sce-codegen`: what is under test is how the script
// prepares the two sides, which trees it compares and what it writes, not
// what the real generator writes, which `a_declared_tree_reproduces_its_
// committed_tree` holds. The fixture generator writes, for a tree, the text
// of its manifest as `gen/<tree>/out.txt`, and refuses a manifest that reads
// `refuse`. It is committed with the trees, as a checkout's own build output
// would be found in each worktree of it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::{tempdir, TempDir};

/// The fixture's generator, answering the two commands the script asks.
const GENERATOR: &str = r#"#!/usr/bin/env bash
set -eu
case "$1" in
    verify-generator) exit 0 ;;
    materialize)
        root="${SCE_WORKSPACE_ROOT:-$PWD}"
        manifest="$root/tools/codegen/trees/$2.toml"
        if [ "$(cat "$manifest")" = refuse ]; then
            echo "materialize $2: refused" >&2
            exit 1
        fi
        mkdir -p "$4/gen/$2"
        cp "$manifest" "$4/gen/$2/out.txt"
        ;;
    *) exit 64 ;;
esac
"#;

/// The files of this checkout the script needs to run in a repository of its
/// own: itself, and the locator it resolves the generator through.
const SCRIPTS: &[&str] = &["scripts/generated_diff.sh", "scripts/lib/sce_codegen.sh"];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// `command`, kept from the configuration of the machine and of any
/// repository a hook may have run this test from.
fn hermetic(mut command: Command) -> Command {
    command
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("SCE_WORKSPACE_ROOT");
    command
}

/// `git <args>` in `dir`, which must succeed; its standard output, trimmed.
fn git(dir: &Path, args: &[&str]) -> String {
    let mut command = hermetic(Command::new("git"));
    command
        .args([
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(dir);
    let out = command.output().expect("run git");
    assert!(
        out.status.success(),
        "git {args:?} in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .expect("git writes UTF-8 here")
        .trim()
        .to_string()
}

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("chmod");
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) {}

/// A repository holding the script, its locator and the fixture generator.
fn fixture() -> TempDir {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    for script in SCRIPTS {
        let to = root.join(script);
        fs::create_dir_all(to.parent().expect("a script has a directory"))
            .expect("create the script's directory");
        fs::copy(repo_root().join(script), &to).expect("copy the script");
    }
    let generator = root.join("target/debug/sce-codegen");
    fs::create_dir_all(generator.parent().expect("the generator has a directory"))
        .expect("create the generator's directory");
    fs::write(&generator, GENERATOR).expect("write the generator");
    make_executable(&generator);
    git(root, &["init", "-q"]);
    dir
}

/// Commit `trees`, each a name and its manifest's text, as every tree the
/// repository declares; the commit's id.
fn commit_trees(root: &Path, trees: &[(&str, &str)]) -> String {
    let dir = root.join("tools/codegen/trees");
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("remove the earlier trees");
    }
    fs::create_dir_all(&dir).expect("create the tree directory");
    for (name, text) in trees {
        fs::write(dir.join(format!("{name}.toml")), text).expect("write a manifest");
    }
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "trees"]);
    git(root, &["rev-parse", "HEAD"])
}

/// The script, from `root`, comparing `base` with `head` into `out`.
fn render(root: &Path, out: &Path, base: &str, head: &str) -> Output {
    let mut command = hermetic(Command::new("bash"));
    command
        .arg("scripts/generated_diff.sh")
        .arg("--out")
        .arg(out)
        .args([base, head])
        .current_dir(root);
    command.output().expect("run scripts/generated_diff.sh")
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// A changed tree, an unchanged one and a tree only the head declares, the
/// head a commit the checkout does not hold, so both sides are worktrees: the
/// changed and the added tree each get a patch whose paths are the
/// repository's, which applies onto the base's output and gives the head's;
/// the unchanged tree gets none and is named as such; a tree a later commit
/// declares is in neither side.
#[test]
fn each_tree_that_differs_gets_a_patch_with_the_repositorys_paths() {
    let repo = fixture();
    let root = repo.path();
    let base = commit_trees(root, &[("alpha", "one\n"), ("beta", "same\n")]);
    let head = commit_trees(
        root,
        &[("alpha", "two\n"), ("beta", "same\n"), ("gamma", "new\n")],
    );
    commit_trees(
        root,
        &[
            ("alpha", "two\n"),
            ("beta", "same\n"),
            ("gamma", "new\n"),
            ("later", "x\n"),
        ],
    );
    let out_dir = tempdir().expect("tempdir");
    let out = out_dir.path();

    let run = render(root, out, &base, &head);
    assert!(
        run.status.success(),
        "the render fails: {}",
        String::from_utf8_lossy(&run.stderr)
    );

    let alpha = read(&out.join("alpha.patch"));
    assert!(
        alpha.contains("diff --git a/gen/alpha/out.txt b/gen/alpha/out.txt"),
        "a patch names the repository's paths, not the sides': {alpha}"
    );
    assert!(alpha.contains("-one") && alpha.contains("+two"), "{alpha}");
    let gamma = read(&out.join("gamma.patch"));
    assert!(
        gamma.contains("+++ b/gen/gamma/out.txt") && gamma.contains("+new"),
        "a tree only the head declares is all added: {gamma}"
    );
    assert!(
        !out.join("beta.patch").exists(),
        "a tree that does not differ gets no patch"
    );
    assert!(
        !out.join("later.patch").exists(),
        "a tree neither commit declares is not compared"
    );

    let summary = read(&out.join("summary.md"));
    assert!(
        summary.contains(&format!("Base `{base}`, head `{head}`.")),
        "{summary}"
    );
    assert!(summary.contains("| alpha | 1 | 1 | 1 |"), "{summary}");
    assert!(summary.contains("| gamma | 1 | 1 | 0 |"), "{summary}");
    assert!(summary.contains("No difference: beta."), "{summary}");
    assert!(!summary.contains("later"), "{summary}");

    // The patches take the base's output to the head's.
    let applied = tempdir().expect("tempdir");
    fs::create_dir_all(applied.path().join("gen/alpha")).expect("create the base's output");
    fs::write(applied.path().join("gen/alpha/out.txt"), "one\n").expect("write the base's output");
    for patch in ["alpha.patch", "gamma.patch"] {
        git(
            applied.path(),
            &["apply", out.join(patch).to_str().expect("a UTF-8 path")],
        );
    }
    assert_eq!(read(&applied.path().join("gen/alpha/out.txt")), "two\n");
    assert_eq!(read(&applied.path().join("gen/gamma/out.txt")), "new\n");
}

/// A tree the head cannot materialize fails the render, which writes the
/// summary first and names the tree and its log there: a missing patch must
/// not read as "no difference". The head is the checkout's own HEAD here, so
/// the head side is the checkout.
#[test]
fn a_tree_the_head_cannot_materialize_fails_the_render() {
    let repo = fixture();
    let root = repo.path();
    let base = commit_trees(root, &[("alpha", "one\n")]);
    let head = commit_trees(root, &[("alpha", "refuse")]);
    let out_dir = tempdir().expect("tempdir");
    let out = out_dir.path();

    let run = render(root, out, &base, &head);
    assert!(
        !run.status.success(),
        "a head that does not materialize is a failed render"
    );
    let summary = read(&out.join("summary.md"));
    assert!(
        summary.contains("Not materialized at the head: `alpha`"),
        "{summary}"
    );
    assert!(read(&out.join("head-alpha.log")).contains("refused"));
    assert!(!out.join("alpha.patch").exists());
}

/// A tree the base cannot materialize is named, and the render goes on with
/// the others and succeeds: an older commit is not the change's fault.
#[test]
fn a_tree_the_base_cannot_materialize_is_named_and_the_render_goes_on() {
    let repo = fixture();
    let root = repo.path();
    let base = commit_trees(root, &[("alpha", "refuse"), ("beta", "one\n")]);
    let head = commit_trees(root, &[("alpha", "fine\n"), ("beta", "two\n")]);
    let out_dir = tempdir().expect("tempdir");
    let out = out_dir.path();

    let run = render(root, out, &base, &head);
    assert!(
        run.status.success(),
        "the render fails: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let summary = read(&out.join("summary.md"));
    assert!(
        summary.contains("Not materialized at the base: `alpha`"),
        "{summary}"
    );
    assert!(!out.join("alpha.patch").exists());
    assert!(read(&out.join("beta.patch")).contains("+two"));
}
