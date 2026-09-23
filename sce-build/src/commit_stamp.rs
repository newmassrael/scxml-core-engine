// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//
// Which commit `HEAD` names, and which paths a build has to watch to see
// that answer change. The module's documentation lives on its `mod`
// declaration in `lib.rs`, because `sce-build/build.rs` `include!`s this
// file and an inner doc comment cannot survive that expansion.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The commit `HEAD` names in the checkout holding `dir`, abbreviated to
/// twelve hex digits.
///
/// `None` when `dir` is not in a git checkout (a vendored crate, a release
/// tarball, a fetched dependency) or when `HEAD` names no commit yet.
pub fn head_commit(dir: &Path) -> Option<String> {
    git(dir, &["rev-parse", "--short=12", "HEAD"])?
        .into_iter()
        .next()
        .filter(|commit| !commit.is_empty())
}

/// Every path whose change can move what [`head_commit`] answers for
/// `dir`, or `None` when `dir` is not in a git checkout.
///
/// Every path returned exists, and that is the property this function is
/// for. Cargo reads a `rerun-if-changed` path that does not exist as
/// changed on every build, so a single absent path re-runs the build
/// script each time — and recompiles everything built from its output
/// after it.
pub fn head_watch_paths(dir: &Path) -> Option<Vec<PathBuf>> {
    let [git_dir, common_dir] =
        git_paths::<2>(dir, &["rev-parse", "--git-dir", "--git-common-dir"])?;
    let head = git_dir.join("HEAD");

    let reftable = common_dir.join("reftable");
    if reftable.is_dir() {
        // The reftable backend keeps every ref in a stack of tables that
        // each ref update rewrites. `HEAD` is among them: the file of that
        // name is a placeholder that never moves, so its content cannot be
        // followed. A linked worktree keeps its own refs — its `HEAD` among
        // them — in a stack of its own, beside the shared one.
        let own = git_dir.join("reftable");
        let mut paths = vec![head, reftable];
        if own.is_dir() && !paths.contains(&own) {
            paths.push(own);
        }
        return Some(paths);
    }

    // A detached `HEAD` holds the commit itself, so the file is the whole
    // answer then. A symbolic one also moves when a branch is checked out,
    // and the commit it names moves with the branch.
    let mut paths = vec![head];
    if let Some(reference) = symbolic_target(&paths[0]) {
        paths.extend(files_backend_ref_paths(dir, &reference)?);
    }
    Some(paths)
}

/// Where the files backend keeps `reference`: its loose file, and the
/// `packed-refs` file that holds it once it is packed.
///
/// Both are resolved by git rather than joined onto the git directory,
/// because the answer depends on the checkout's shape. In a linked
/// worktree `HEAD` is private to the worktree, while branch refs and
/// `packed-refs` live in the common directory every worktree shares —
/// so `<git-dir>/refs/heads/<branch>` names a file that does not exist.
fn files_backend_ref_paths(dir: &Path, reference: &str) -> Option<Vec<PathBuf>> {
    let [loose, packed, refs] = git_paths::<3>(
        dir,
        &[
            "rev-parse",
            "--git-path",
            reference,
            "--git-path",
            "packed-refs",
            "--git-path",
            "refs",
        ],
    )?;

    let mut paths = Vec::new();
    // A packed ref has no loose file, and the ref's next update writes one.
    // The nearest directory that exists is where that file will appear, and
    // watching it is what sees it appear; watching the absent file instead
    // would re-run the build every time until it did.
    if let Some(existing) = loose
        .ancestors()
        .take_while(|ancestor| ancestor.starts_with(&refs))
        .find(|ancestor| ancestor.exists())
    {
        paths.push(existing.to_path_buf());
    }
    // A ref that has never been packed has no `packed-refs` to read, and
    // packing it later deletes the loose file watched above — a change
    // cargo sees — so there is nothing to watch here until then.
    if packed.is_file() {
        paths.push(packed);
    }
    Some(paths)
}

/// The ref a symbolic `HEAD` names (`refs/heads/main`), or `None` when
/// `HEAD` is detached and holds a commit instead.
fn symbolic_target(head: &Path) -> Option<String> {
    let content = std::fs::read_to_string(head).ok()?;
    content.strip_prefix("ref: ").map(|r| r.trim().to_string())
}

/// `git <args>` output of exactly `N` lines, each read as a path.
///
/// Git prints a path relative to the directory it ran in when it can, and
/// it runs here in `dir`, so a relative line is joined onto `dir`.
fn git_paths<const N: usize>(dir: &Path, args: &[&str]) -> Option<[PathBuf; N]> {
    let paths: Vec<PathBuf> = git(dir, args)?
        .into_iter()
        .map(|line| {
            let path = PathBuf::from(line);
            if path.is_absolute() {
                path
            } else {
                dir.join(path)
            }
        })
        .collect();
    paths.try_into().ok()
}

/// Stdout of `git <args>` run in `dir`, one entry per line, or `None` when
/// git is missing or refuses — which outside a checkout it does.
fn git(dir: &Path, args: &[&str]) -> Option<Vec<String>> {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    Some(text.lines().map(|line| line.trim().to_string()).collect())
}

// ── tests ─────────────────────────────────────────────────────────────
//
// `#[cfg(test)]` is stripped before name resolution in a build script, so
// this block is invisible to the `include!` in `build.rs` and may use the
// crate's dev-dependencies freely.
#[cfg(test)]
mod tests {
    use super::*;

    /// Run git in `dir` as a fixture step. Identity, signing and hooks are
    /// pinned on the command line, so the fixture does not inherit them
    /// from whoever runs it.
    fn run(dir: &Path, args: &[&str]) -> String {
        let out = Command::new("git")
            .args([
                "-c",
                "user.name=sce fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "core.hooksPath=/dev/null",
            ])
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap_or_else(|e| panic!("git {args:?}: {e}"));
        assert!(out.status.success(), "git {args:?}: {out:?}");
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    }

    /// A repository at `<tmp>/main` with one commit on `main`, and a linked
    /// worktree at `<tmp>/linked` on a branch of its own.
    fn repository_with_linked_worktree(tmp: &Path) -> (PathBuf, PathBuf) {
        let main = tmp.join("main");
        std::fs::create_dir_all(&main).unwrap();
        run(&main, &["init", "-q", "-b", "main"]);
        run(&main, &["commit", "-q", "--allow-empty", "-m", "first"]);
        run(
            &main,
            &["worktree", "add", "-q", "-b", "topic", "../linked"],
        );
        (main, tmp.join("linked"))
    }

    /// Where git itself says `reference` is kept as a loose file.
    fn loose_ref(dir: &Path, reference: &str) -> PathBuf {
        let path = PathBuf::from(run(dir, &["rev-parse", "--git-path", reference]));
        if path.is_absolute() {
            path
        } else {
            dir.join(path)
        }
    }

    /// Whether a change at `path` is one cargo sees through `watched`: the
    /// path itself is watched, or it lies under a watched directory, which
    /// cargo scans.
    fn covered(path: &Path, watched: &[PathBuf]) -> bool {
        watched
            .iter()
            .any(|w| path == w || (w.is_dir() && path.starts_with(w)))
    }

    fn assert_all_exist(watched: &[PathBuf]) {
        for path in watched {
            assert!(
                path.exists(),
                "watched {} does not exist, so cargo would re-run the build \
                 script on every build; whole set: {watched:#?}",
                path.display()
            );
        }
    }

    /// The failure this module exists for. `git rev-parse --git-dir` in a
    /// linked worktree names the worktree's private directory, and branch
    /// refs and `packed-refs` are not kept there — so watches joined onto
    /// it named two absent files, and every cargo invocation in a worktree
    /// rebuilt `sce-build` and everything compiled against it.
    #[test]
    fn a_linked_worktree_watches_only_paths_that_exist() {
        let tmp = tempfile::tempdir().unwrap();
        let (_, linked) = repository_with_linked_worktree(tmp.path());

        let watched = head_watch_paths(&linked).expect("a linked worktree is a checkout");

        assert_all_exist(&watched);
        let branch = loose_ref(&linked, "refs/heads/topic");
        assert!(
            covered(&branch, &watched),
            "the checked-out branch's ref {} is not watched: {watched:#?}",
            branch.display()
        );
    }

    #[test]
    fn the_main_worktree_watches_only_paths_that_exist() {
        let tmp = tempfile::tempdir().unwrap();
        let (main, _) = repository_with_linked_worktree(tmp.path());

        let watched = head_watch_paths(&main).expect("the main worktree is a checkout");

        assert_all_exist(&watched);
        let branch = loose_ref(&main, "refs/heads/main");
        assert!(
            covered(&branch, &watched),
            "the checked-out branch's ref {} is not watched: {watched:#?}",
            branch.display()
        );
    }

    /// A packed branch has no loose file, and its next commit writes one.
    /// The watch set taken while it was packed has to see that file
    /// appear, or the embedded commit goes stale — and it must not name
    /// the absent file, or every build re-runs until the commit lands.
    #[test]
    fn a_packed_branch_sees_its_next_commit_without_watching_an_absent_file() {
        let tmp = tempfile::tempdir().unwrap();
        let (main, linked) = repository_with_linked_worktree(tmp.path());
        run(&main, &["pack-refs", "--all"]);
        let branch = loose_ref(&linked, "refs/heads/topic");
        assert!(
            !branch.exists(),
            "pack-refs left {} loose",
            branch.display()
        );

        let watched = head_watch_paths(&linked).expect("a linked worktree is a checkout");
        assert_all_exist(&watched);

        run(&linked, &["commit", "-q", "--allow-empty", "-m", "second"]);
        assert!(
            branch.exists(),
            "the commit did not write the loose ref this test watches for"
        );
        assert!(
            covered(&branch, &watched),
            "the commit wrote {} outside every watched path: {watched:#?}",
            branch.display()
        );
    }

    #[test]
    fn a_detached_head_is_watched_through_head_alone() {
        let tmp = tempfile::tempdir().unwrap();
        let (_, linked) = repository_with_linked_worktree(tmp.path());
        run(&linked, &["checkout", "-q", "--detach"]);

        let watched = head_watch_paths(&linked).expect("a linked worktree is a checkout");

        assert_eq!(
            watched.len(),
            1,
            "a detached HEAD watched more: {watched:#?}"
        );
        assert_eq!(watched[0].file_name().unwrap(), "HEAD");
        assert_all_exist(&watched);
    }

    /// The reftable backend keeps `HEAD` in its table stacks and leaves a
    /// placeholder file of that name, so the rule is to watch the stacks
    /// and never follow the placeholder.
    ///
    /// What this holds is the layout rule, not git's reftable writer: the
    /// stacks are made by hand in a files-backend repository, because the
    /// git a developer runs may predate reftable. A real one differs only in
    /// what the directories hold, and the rule reads nothing inside them.
    #[test]
    fn a_reftable_store_is_watched_as_its_table_stacks() {
        let tmp = tempfile::tempdir().unwrap();
        let (main, linked) = repository_with_linked_worktree(tmp.path());
        let common = main.join(".git");
        let own = common.join("worktrees/linked");
        assert!(
            own.join("HEAD").is_file(),
            "the fixture's worktree directory moved"
        );
        std::fs::create_dir(common.join("reftable")).unwrap();
        std::fs::create_dir(own.join("reftable")).unwrap();

        let watched = head_watch_paths(&linked).expect("a linked worktree is a checkout");

        let canonical = |paths: &[PathBuf]| -> Vec<PathBuf> {
            paths.iter().map(|p| p.canonicalize().unwrap()).collect()
        };
        assert_eq!(
            canonical(&watched),
            canonical(&[
                own.join("HEAD"),
                common.join("reftable"),
                own.join("reftable")
            ]),
            "a reftable store is its HEAD file and its two stacks, and nothing else"
        );
    }

    #[test]
    fn the_commit_is_the_one_head_names() {
        let tmp = tempfile::tempdir().unwrap();
        let (_, linked) = repository_with_linked_worktree(tmp.path());

        let expected = run(&linked, &["rev-parse", "--short=12", "HEAD"]);

        assert_eq!(head_commit(&linked).as_deref(), Some(expected.as_str()));
    }
}
