// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! `tools/git-hooks/restricted-term-gate.sh` reports what it always reported,
//! and its cost follows the pattern count rather than the staged file count.
//!
//! The gate used to run one `grep` per staged file per pattern, each inside a
//! command substitution. A commit of 3,509 files spent about seventy minutes in
//! it on 2026-09-28. The repair runs one `grep` per pattern over the staged
//! blobs, so the first half of this suite holds the report to its old shape
//! and the second half counts the `grep` processes.
//!
//! The second half of the file holds what the gate added to the strict tier: a
//! loose one (case ignored, no word boundary) over the lines a commit adds, a
//! file of pattern numbers that leaves a blunt pattern out of it, and the same
//! list held to the commit message.
//!
//! The last part holds the same list to a range of commits, which is what a push
//! publishes: the lines each commit adds and its message, a commit made past the
//! commit hooks, the post-commit warning for such a commit, and the override
//! that leaves a line in a log outside the repository.
//!
//! Every run here reads a synthetic term list through `SCE_RESTRICTED_TERMS`.
//! The real list lives outside the repository and is never opened by a test:
//! reading it would copy the terms it exists to keep out.

#![cfg(unix)]

mod common;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// Two synthetic patterns: an anchored word and an unanchored token.
const TERMS: &str = "# synthetic list for this suite\nSCE_PROBE_TERM_[0-9]+\n\\bneedle\\b\n";

struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "sce-restricted-gate-{label}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("repo")).expect("create sandbox repository");
        let sandbox = Sandbox { dir };
        sandbox.git(&["init", "-q"]);
        sandbox
    }

    fn repo(&self) -> PathBuf {
        self.dir.join("repo")
    }

    fn terms(&self, contents: &str) -> PathBuf {
        let path = self.dir.join("terms.list");
        std::fs::write(&path, contents).expect("write synthetic term list");
        path
    }

    fn git(&self, args: &[&str]) {
        let out = Command::new("git")
            .arg("-C")
            .arg(self.repo())
            .args(args)
            .output()
            .expect("git runs");
        assert!(
            out.status.success(),
            "git {args:?} failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn write(&self, rel: &str, bytes: &[u8]) {
        let path = self.repo().join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).expect("create parent directory");
        std::fs::write(&path, bytes).expect("write sandbox file");
    }

    fn stage(&self, rel: &str, bytes: &[u8]) {
        self.write(rel, bytes);
        self.git(&["add", "--", rel]);
    }

    /// Source the gate and call it in the sandbox, the way `pre-commit` does: in a shell with
    /// `set -euo pipefail`, where a command of the gate that fails and is not tested ends it. It
    /// was once sourced without, and a gate that only worked unless its shell was strict passed
    /// here and refused the commit that mattered.
    fn run_gate(&self, terms: &Path, path_prefix: Option<&Path>) -> Output {
        let gate = common::repository::root().join("tools/git-hooks/restricted-term-gate.sh");
        let script = format!(
            "set -euo pipefail\nsource '{}'\nrestricted_term_gate_staged\n",
            gate.display()
        );
        let mut cmd = Command::new("bash");
        cmd.arg("-c")
            .arg(script)
            .current_dir(self.repo())
            .env("SCE_RESTRICTED_TERMS", terms)
            .env_remove("SCE_RESTRICTED_OVERRIDE");
        if let Some(prefix) = path_prefix {
            let path = std::env::var("PATH").unwrap_or_default();
            cmd.env("PATH", format!("{}:{path}", prefix.display()));
        }
        cmd.output().expect("bash runs the gate")
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// The lines that name a hit: `  <path>: line(s) <n,...> — term #<i>`.
fn hit_lines(out: &Output) -> Vec<String> {
    String::from_utf8_lossy(&out.stderr)
        .lines()
        .filter(|l| l.starts_with("  ") && l.contains(" — term #"))
        .map(str::to_string)
        .collect()
}

#[test]
fn the_report_names_each_staged_file_line_and_pattern_in_order() {
    let sb = Sandbox::new("report");
    let terms = sb.terms(TERMS);

    sb.stage("plain.txt", b"nothing here\n");
    sb.stage("both.txt", b"needle\nSCE_PROBE_TERM_5\n");
    sb.stage(
        "notes/a b:c.txt",
        b"one\nSCE_PROBE_TERM_1 here\nthree\nfour\nagain SCE_PROBE_TERM_22\n",
    );
    sb.stage("crlf.txt", b"a\r\nSCE_PROBE_TERM_9\r\n");
    // A binary blob is skipped even when it carries a pattern.
    sb.stage("bin.dat", b"\x00\x01SCE_PROBE_TERM_3\x00needle\n");
    // The STAGED blob is what is scanned: the working tree no longer carries
    // the term, and the gate must still see it.
    sb.stage("worktree_differs.txt", b"needle\n");
    sb.write("worktree_differs.txt", b"clean\n");
    // A symlink is published as its target text, and that text is scanned.
    std::os::unix::fs::symlink("needle", sb.repo().join("link")).expect("create symlink");
    sb.git(&["add", "--", "link"]);

    let out = sb.run_gate(&terms, None);
    assert!(!out.status.success(), "a staged hit must refuse the commit");
    assert_eq!(
        hit_lines(&out),
        vec![
            "  both.txt: line(s) 2 — term #0",
            "  both.txt: line(s) 1 — term #1",
            "  crlf.txt: line(s) 2 — term #0",
            "  link: line(s) 1 — term #1",
            "  notes/a b:c.txt: line(s) 2,5 — term #0",
            "  worktree_differs.txt: line(s) 1 — term #1",
        ],
        "stderr was:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The gate never prints what it matched.
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("SCE_PROBE_TERM") && !stderr.contains("needle"),
        "the report must not carry the matched text:\n{stderr}"
    );
}

/// `git diff --name-only` quotes a path that is not plain ASCII, and a quoted
/// path names no index entry. The gate once read that list, failed to find
/// the blob and skipped the file, so a term in such a file was never scanned.
#[test]
fn a_path_git_would_quote_is_still_scanned() {
    let sb = Sandbox::new("quoted");
    let terms = sb.terms(TERMS);
    sb.stage("naïve/über.txt", b"first\nneedle\n");

    let out = sb.run_gate(&terms, None);
    assert!(
        !out.status.success(),
        "a term in a non-ASCII path must refuse the commit:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        hit_lines(&out),
        vec!["  naïve/über.txt: line(s) 2 — term #1"],
        "stderr was:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn a_clean_or_empty_index_passes_and_an_unreadable_list_refuses() {
    let sb = Sandbox::new("verdicts");
    let terms = sb.terms(TERMS);

    let empty = sb.run_gate(&terms, None);
    assert!(
        empty.status.success(),
        "nothing staged is nothing to refuse"
    );

    sb.stage("clean.txt", b"no terms at all\n");
    let clean = sb.run_gate(&terms, None);
    assert!(
        clean.status.success(),
        "a clean index must pass:\n{}",
        String::from_utf8_lossy(&clean.stderr)
    );

    let missing = sb.run_gate(&sb.dir.join("no-such-list"), None);
    assert!(
        !missing.status.success(),
        "a list it cannot read must refuse (fail closed)"
    );

    let comments_only = sb.terms("# nothing but a comment\n\n");
    let hollow = sb.run_gate(&comments_only, None);
    assert!(
        !hollow.status.success(),
        "an empty list must refuse rather than pass"
    );
}

/// Stage `files` text files, run the gate with a `grep` that records each
/// call, and return how many times `grep` ran.
fn grep_calls_for(files: usize) -> usize {
    let sb = Sandbox::new(&format!("forks-{files}"));
    let terms = sb.terms(TERMS);
    for n in 0..files {
        sb.write(
            &format!("dir{}/f{n}.txt", n % 7),
            format!("line {n}\n").as_bytes(),
        );
    }
    sb.git(&["add", "-A"]);

    let real_grep = String::from_utf8(
        Command::new("bash")
            .args(["-c", "command -v grep"])
            .output()
            .expect("locate grep")
            .stdout,
    )
    .expect("grep path is UTF-8");
    let real_grep = real_grep.trim();
    assert!(!real_grep.is_empty(), "grep must be on PATH");

    let shim_dir = sb.dir.join("shim");
    std::fs::create_dir_all(&shim_dir).expect("create shim directory");
    let log = sb.dir.join("grep.calls");
    common::executable::install_executable(
        &shim_dir.join("grep"),
        format!(
            "#!/bin/sh\necho call >> '{}'\nexec '{real_grep}' \"$@\"\n",
            log.display()
        ),
    )
    .expect("install grep shim");

    let out = sb.run_gate(&terms, Some(&shim_dir));
    assert!(
        out.status.success(),
        "{files} clean files must pass:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::read_to_string(&log).map_or(0, |s| s.lines().count())
}

#[test]
fn grep_runs_once_per_pattern_however_many_files_are_staged() {
    let patterns = 2;
    let few = grep_calls_for(12);
    let many = grep_calls_for(60);

    // A floor, so a gate that scanned nothing cannot pass by running no grep.
    assert!(
        few >= patterns,
        "each pattern must run at least once; grep ran {few} times for {patterns} patterns"
    );
    assert_eq!(
        few, many,
        "grep ran {few} times for 12 staged files and {many} for 60: the gate's \
         cost must follow the pattern count, not the file count"
    );
}

/// The lines of the loose tier: `  <path>: line(s) <n,...> — loose term #<i>`.
fn loose_lines(out: &Output) -> Vec<String> {
    String::from_utf8_lossy(&out.stderr)
        .lines()
        .filter(|l| l.starts_with("  ") && l.contains(" — loose term #"))
        .map(str::to_string)
        .collect()
}

impl Sandbox {
    /// Commit what is staged, with an identity of its own and no hooks.
    fn commit(&self, message: &str) {
        self.git(&[
            "-c",
            "user.name=probe",
            "-c",
            "user.email=probe@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-q",
            "--no-verify",
            "-m",
            message,
        ]);
    }

    /// Source the gate and hold `message` to it, the way `commit-msg` does.
    fn run_message_gate(&self, terms: &Path, message: &str) -> Output {
        let gate = common::repository::root().join("tools/git-hooks/restricted-term-gate.sh");
        let file = self.dir.join("COMMIT_EDITMSG");
        std::fs::write(&file, message).expect("write the message");
        let script = format!(
            "set -euo pipefail\nsource '{}'\nrestricted_term_gate_message '{}'\n",
            gate.display(),
            file.display()
        );
        Command::new("bash")
            .arg("-c")
            .arg(script)
            .current_dir(self.repo())
            .env("SCE_RESTRICTED_TERMS", terms)
            .env_remove("SCE_RESTRICTED_OVERRIDE")
            .output()
            .expect("bash runs the gate")
    }
}

/// A submodule pointer is staged by the commit that moves a pin, and a repository that sets
/// `* text=auto` has an attribute that names it. The gate checked it out as an empty directory,
/// failed to write a blob over it, and under `set -e` ended there: the workbench could not commit
/// a pin until this was held (2026-10-17).
#[test]
fn a_staged_submodule_pointer_is_not_a_blob_to_scan_and_does_not_end_the_gate() {
    let sb = Sandbox::new("gitlink");
    let terms = sb.terms(TERMS);
    sb.write(".gitattributes", b"* text=auto\n");
    sb.git(&["add", ".gitattributes"]);
    sb.git(&[
        "update-index",
        "--add",
        "--cacheinfo",
        "160000,0123456789012345678901234567890123456789,vendored",
    ]);
    sb.stage("beside.txt", b"nothing restricted\n");

    let clean = sb.run_gate(&terms, None);
    assert!(
        clean.status.success(),
        "a pointer beside clean files must pass:\n{}",
        String::from_utf8_lossy(&clean.stderr)
    );

    sb.stage("beside.txt", b"a SCE_PROBE_TERM_8 line\n");
    let dirty = sb.run_gate(&terms, None);
    assert_eq!(
        hit_lines(&dirty),
        vec!["  beside.txt: line(s) 1 — term #0"],
        "the files beside a pointer are still scanned:\n{}",
        String::from_utf8_lossy(&dirty.stderr)
    );
}

#[test]
fn a_term_in_another_case_or_behind_a_letter_is_refused_on_a_line_the_commit_adds() {
    let sb = Sandbox::new("loose-added");
    let terms = sb.terms(TERMS);
    sb.stage("case.txt", b"fine\nNeedle\n");
    sb.stage("prefix.txt", b"mneedle\n");
    sb.stage("clean.txt", b"a nee dle line\n");

    let out = sb.run_gate(&terms, None);

    assert!(!out.status.success(), "a loose hit must refuse the commit");
    assert!(
        hit_lines(&out).is_empty(),
        "the strict tier names neither: {:?}",
        hit_lines(&out)
    );
    assert_eq!(
        loose_lines(&out),
        vec![
            "  case.txt: line(s) 2 — loose term #1",
            "  prefix.txt: line(s) 1 — loose term #1",
        ],
        "stderr was:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("eedle"),
        "the report must not carry the matched text:\n{stderr}"
    );
}

#[test]
fn a_line_that_matches_loosely_and_was_there_before_does_not_refuse_the_commit_that_did_not_write_it(
) {
    let sb = Sandbox::new("loose-old");
    let terms = sb.terms(TERMS);
    sb.stage("old.txt", b"Needle here\nother\nlast\n");
    sb.commit("the file as it was");

    // A line that does not match is changed beside the one that does.
    sb.stage("old.txt", b"Needle here\nother, changed\nlast\n");
    let beside = sb.run_gate(&terms, None);
    assert!(
        beside.status.success(),
        "an old line is not the commit's to answer for:\n{}",
        String::from_utf8_lossy(&beside.stderr)
    );

    // A line that matches is added to it.
    sb.stage(
        "old.txt",
        b"Needle here\nother, changed\nlast\nNEEDLE again\n",
    );
    let added = sb.run_gate(&terms, None);
    assert!(!added.status.success(), "an added line must be refused");
    assert_eq!(
        loose_lines(&added),
        vec!["  old.txt: line(s) 4 — loose term #1"],
        "stderr was:\n{}",
        String::from_utf8_lossy(&added.stderr)
    );
}

#[test]
fn a_line_the_strict_tier_named_is_not_named_again_by_the_loose_one() {
    let sb = Sandbox::new("loose-once");
    let terms = sb.terms(TERMS);
    sb.stage("both.txt", b"needle\nNeedle\n");

    let out = sb.run_gate(&terms, None);

    assert_eq!(hit_lines(&out), vec!["  both.txt: line(s) 1 — term #1"]);
    assert_eq!(
        loose_lines(&out),
        vec!["  both.txt: line(s) 2 — loose term #1"]
    );
}

#[test]
fn a_pattern_the_exclude_file_names_leaves_the_loose_tier_and_not_the_strict_one() {
    let sb = Sandbox::new("loose-exclude");
    let terms = sb.terms(TERMS);
    // Numbers and comments only: the file holds no term.
    std::fs::write(
        sb.dir.join("terms.list.loose-exclude"),
        "# too blunt for the loose tier\n1  # the second pattern\n",
    )
    .expect("write the exclude file");

    sb.stage("loose.txt", b"Needle\n");
    let loose = sb.run_gate(&terms, None);
    assert!(
        loose.status.success(),
        "an excluded pattern is not checked loosely:\n{}",
        String::from_utf8_lossy(&loose.stderr)
    );

    sb.stage("strict.txt", b"needle\n");
    let strict = sb.run_gate(&terms, None);
    assert!(!strict.status.success(), "the strict tier still applies");
    assert_eq!(
        hit_lines(&strict),
        vec!["  strict.txt: line(s) 1 — term #1"]
    );

    // Pattern 0 is not excluded, so it is still checked loosely.
    sb.stage("other.txt", b"sce_probe_term_4\n");
    let other = sb.run_gate(&terms, None);
    assert_eq!(
        loose_lines(&other),
        vec!["  other.txt: line(s) 1 — loose term #0"],
        "stderr was:\n{}",
        String::from_utf8_lossy(&other.stderr)
    );
}

/// A lockfile's `checksum = "<64 hex digits>"` line is written by a package manager, and a digest
/// is hex, so a term made of hex letters turns up inside one at random. Measured 2026-10-19: a
/// `Cargo.lock` line tripped a pattern of a few letters from `a` to `f` and the commit that locked
/// a dependency tree was refused. Only the loose tier, which has no word boundary, passes such a
/// line, and only when the line is nothing else; the strict tier and every other line are read as
/// before, in the staged scan and in the push-range scan alike.
#[test]
fn a_lockfile_checksum_line_is_not_read_loosely_and_nothing_else_is_spared() {
    let sb = Sandbox::new("loose-checksum");
    let terms = sb.terms("\\bfacade\n");
    let inside = format!("3facade{}", "0".repeat(57));
    assert_eq!(inside.len(), 64);
    let line = |digest: &str| format!("checksum = \"{digest}\"\n");

    sb.stage("Cargo.lock", line(&inside).as_bytes());
    let spared = sb.run_gate(&terms, None);
    assert!(
        spared.status.success(),
        "a digest that holds the term inside it is not a hit:\n{}",
        String::from_utf8_lossy(&spared.stderr)
    );

    sb.stage(
        "Cargo.lock",
        format!("checksum = \"{inside}\" xfacade\n").as_bytes(),
    );
    let extra = sb.run_gate(&terms, None);
    assert_eq!(
        loose_lines(&extra),
        vec!["  Cargo.lock: line(s) 1 — loose term #0"],
        "text after the digest is read: {}",
        String::from_utf8_lossy(&extra.stderr)
    );

    sb.stage("Cargo.lock", line(&inside[..63]).as_bytes());
    let short = sb.run_gate(&terms, None);
    assert_eq!(
        loose_lines(&short),
        vec!["  Cargo.lock: line(s) 1 — loose term #0"],
        "63 digits is not a checksum: {}",
        String::from_utf8_lossy(&short.stderr)
    );

    let leading = format!("facade{}", "0".repeat(58));
    sb.stage("Cargo.lock", line(&leading).as_bytes());
    let strict = sb.run_gate(&terms, None);
    assert_eq!(
        hit_lines(&strict),
        vec!["  Cargo.lock: line(s) 1 — term #0"],
        "the strict tier still reads a digest that begins with the term: {}",
        String::from_utf8_lossy(&strict.stderr)
    );

    // The push-range scan reads the lines a commit added, and spares the same line and no other.
    sb.stage("Cargo.lock", line(&inside).as_bytes());
    sb.commit("lock the tree");
    let range = sb.run_range(&terms, &["HEAD"]);
    assert!(
        range.status.success(),
        "the range scan spares the digest line:\n{}",
        String::from_utf8_lossy(&range.stderr)
    );
    sb.stage("note.txt", b"a word xfacade here\n");
    sb.commit("a note");
    let refused = sb.run_range(&terms, &["HEAD"]);
    assert!(
        !refused.status.success(),
        "the range scan still refuses an ordinary line:\n{}",
        String::from_utf8_lossy(&refused.stderr)
    );
}

#[test]
fn many_files_that_match_loosely_are_reported_in_bounded_time() {
    // `grep_runs_once_per_pattern_however_many_files_are_staged` stages clean
    // files, where the loose tier also runs; this one stages files that match
    // it, so the added-line bookkeeping runs over many of them.
    let sb = Sandbox::new("loose-cost");
    let terms = sb.terms(TERMS);
    for n in 0..80 {
        sb.write(&format!("d{}/f{n}.txt", n % 5), b"first\nNeedle\n");
    }
    sb.git(&["add", "-A"]);

    let began = std::time::Instant::now();
    let out = sb.run_gate(&terms, None);

    assert!(!out.status.success());
    assert_eq!(loose_lines(&out).len(), 80, "one line for each file");
    assert!(
        began.elapsed() < std::time::Duration::from_secs(30),
        "80 files took {:?}",
        began.elapsed()
    );
}

#[test]
fn a_message_is_held_to_the_list_strictly_and_loosely_and_never_prints_what_it_matched() {
    let sb = Sandbox::new("message");
    let terms = sb.terms(TERMS);

    let out = sb.run_message_gate(
        &terms,
        "subject\n\nthe body says Needle once\n# a comment says needle and git drops it\nSCE_PROBE_TERM_7 too\n",
    );

    assert!(!out.status.success(), "a message with a term is refused");
    assert_eq!(
        hit_lines(&out),
        vec!["  commit message: line(s) 5 — term #0"],
        "stderr was:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        loose_lines(&out),
        vec!["  commit message: line(s) 3 — loose term #1"],
        "the comment line is not scanned"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("eedle") && !stderr.contains("SCE_PROBE_TERM"),
        "the report must not carry the matched text:\n{stderr}"
    );
}

#[test]
fn a_clean_message_passes_and_a_list_that_cannot_be_read_refuses() {
    let sb = Sandbox::new("message-verdicts");
    let terms = sb.terms(TERMS);

    let clean = sb.run_message_gate(&terms, "subject\n\n- one plain line\n");
    assert!(
        clean.status.success(),
        "{}",
        String::from_utf8_lossy(&clean.stderr)
    );

    let missing = sb.run_message_gate(&sb.dir.join("no-such-list"), "subject\n");
    assert!(
        !missing.status.success(),
        "no list is no check, and it refuses"
    );
    let hollow = sb.run_message_gate(&sb.terms("# only a comment\n"), "subject\n");
    assert!(!hollow.status.success(), "an empty list refuses");
}

#[test]
fn the_commit_msg_hook_refuses_a_term_before_it_looks_at_the_format_and_past_its_bypass() {
    let sb = Sandbox::new("hook");
    let terms = sb.terms(TERMS);
    let hook = common::repository::root().join("tools/git-hooks/commit-msg");
    let file = sb.dir.join("COMMIT_EDITMSG");
    std::fs::write(&file, "subject\n\na body with a Needle in it\n").expect("write the message");

    let out = Command::new("bash")
        .arg(&hook)
        .arg(&file)
        .current_dir(sb.repo())
        .env("SCE_RESTRICTED_TERMS", &terms)
        .env("SKIP_COMMIT_MSG", "1")
        .env_remove("SCE_RESTRICTED_OVERRIDE")
        .output()
        .expect("bash runs the hook");

    assert!(
        !out.status.success(),
        "SKIP_COMMIT_MSG does not cover the term gate"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("matches the restricted-term list"),
        "{stderr}"
    );
    assert!(!stderr.contains("check skipped"), "{stderr}");
}

// ── A range of commits, which is what a push publishes ─────────────────────

const ZERO: &str = "0000000000000000000000000000000000000000";

impl Sandbox {
    fn rev(&self, spec: &str) -> String {
        let out = Command::new("git")
            .arg("-C")
            .arg(self.repo())
            .args(["rev-parse", spec])
            .output()
            .expect("git runs");
        assert!(out.status.success(), "rev-parse {spec} failed");
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    /// Run `script` in the sandbox in a strict shell, with the gate sourced.
    fn bash(&self, script: &str, terms: &Path, env: &[(&str, &Path)]) -> Output {
        let gate = common::repository::root().join("tools/git-hooks/restricted-term-gate.sh");
        let script = format!("set -euo pipefail\nsource '{}'\n{script}\n", gate.display());
        let mut cmd = Command::new("bash");
        cmd.arg("-c")
            .arg(script)
            .current_dir(self.repo())
            .env("SCE_RESTRICTED_TERMS", terms)
            .env_remove("SCE_RESTRICTED_OVERRIDE")
            .env("SCE_RESTRICTED_OVERRIDE_LOG", self.dir.join("override.log"));
        for (key, value) in env {
            cmd.env(key, value);
        }
        cmd.output().expect("bash runs the gate")
    }

    fn run_range(&self, terms: &Path, revs: &[&str]) -> Output {
        let args: Vec<String> = revs.iter().map(|r| format!("'{r}'")).collect();
        self.bash(
            &format!("restricted_term_gate_range probe {}", args.join(" ")),
            terms,
            &[],
        )
    }

    /// Run the real `pre-push` hook with `refs` on its standard input.
    fn run_pre_push(&self, terms: &Path, refs: &str) -> Output {
        let hook = common::repository::root().join("tools/git-hooks/pre-push");
        let mut cmd = Command::new("bash");
        cmd.arg(&hook)
            .current_dir(self.repo())
            .env("SCE_RESTRICTED_TERMS", terms)
            .env_remove("SCE_RESTRICTED_OVERRIDE")
            .env("SCE_RESTRICTED_OVERRIDE_LOG", self.dir.join("override.log"))
            .env("SKIP_PREPUSH", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn().expect("bash runs the hook");
        child
            .stdin
            .take()
            .expect("stdin is piped")
            .write_all(refs.as_bytes())
            .expect("write the refs");
        child.wait_with_output().expect("the hook ends")
    }

    /// One git command with an identity of its own, for a merge or a branch switch.
    fn as_probe(&self, args: &[&str]) {
        let mut all = vec![
            "-c",
            "user.name=probe",
            "-c",
            "user.email=probe@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ];
        all.extend_from_slice(args);
        self.git(&all);
    }
}

/// The lines that name a hit of either tier.
fn said_lines(out: &Output) -> Vec<String> {
    String::from_utf8_lossy(&out.stderr)
        .lines()
        .filter(|l| l.starts_with("  ") && l.contains("term #"))
        .map(str::to_string)
        .collect()
}

fn stderr_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn a_range_is_held_to_what_each_commit_adds_even_when_a_later_commit_took_it_out() {
    let sb = Sandbox::new("range-history");
    let terms = sb.terms(TERMS);
    sb.stage("notes.txt", b"one\nthe SCE_PROBE_TERM_7 is here\n");
    sb.commit("first");
    let dirty = sb.rev("HEAD");
    sb.stage("notes.txt", b"one\nnothing\n");
    sb.commit("second");

    let out = sb.run_range(&terms, &["HEAD"]);
    assert!(
        !out.status.success(),
        "the tip is clean and the history is not:\n{}",
        stderr_of(&out)
    );
    assert_eq!(
        said_lines(&out),
        vec![format!("  {} notes.txt: line(s) 2 — term #0", &dirty[..12])],
        "{}",
        stderr_of(&out)
    );
    assert!(!stderr_of(&out).contains("SCE_PROBE_TERM_7"));

    let after = sb.run_range(&terms, &[&format!("{dirty}..HEAD")]);
    assert!(
        after.status.success(),
        "a range that starts after it does not carry it:\n{}",
        stderr_of(&after)
    );
}

#[test]
fn a_range_names_the_message_and_the_loose_tier_and_names_a_line_once() {
    let sb = Sandbox::new("range-tiers");
    let terms = sb.terms(TERMS);
    sb.stage("notes.txt", b"a SCE_PROBE_TERM_9 and a Needle\n");
    sb.commit("fix: a Needle in the subject");
    let at = sb.rev("HEAD")[..12].to_string();

    let out = sb.run_range(&terms, &["HEAD"]);
    assert!(!out.status.success());
    assert_eq!(
        said_lines(&out),
        vec![
            format!("  {at} (commit message): line(s) 1 — loose term #1"),
            format!("  {at} notes.txt: line(s) 1 — term #0"),
            format!("  {at} notes.txt: line(s) 1 — loose term #1"),
        ],
        "the strict hit is not named again as a loose one:\n{}",
        stderr_of(&out)
    );
    let said = stderr_of(&out);
    assert!(!said.contains("SCE_PROBE_TERM_9") && !said.contains("Needle"));
}

#[test]
fn an_old_line_is_not_a_commit_s_and_a_merge_is_read_by_what_its_side_added() {
    let sb = Sandbox::new("range-merge");
    let terms = sb.terms(TERMS);
    sb.stage("old.txt", b"an old SCE_PROBE_TERM_1 line\n");
    sb.commit("base");
    let base = sb.rev("HEAD");

    sb.as_probe(&["switch", "-q", "-c", "side"]);
    sb.stage("side.txt", b"a SCE_PROBE_TERM_2 on the side\n");
    sb.commit("side");
    let side = sb.rev("HEAD")[..12].to_string();
    sb.as_probe(&["switch", "-q", "-"]);
    sb.stage("main.txt", b"a clean line\n");
    sb.commit("main");
    sb.as_probe(&["merge", "--no-ff", "-q", "side", "-m", "merge the side"]);

    let out = sb.run_range(&terms, &[&format!("{base}..HEAD")]);
    assert_eq!(
        said_lines(&out),
        vec![format!("  {side} side.txt: line(s) 1 — term #0")],
        "the old line (before the range) and the merge itself add nothing:\n{}",
        stderr_of(&out)
    );
}

#[test]
fn a_range_git_cannot_read_and_a_list_it_cannot_read_both_refuse() {
    let sb = Sandbox::new("range-refuses");
    let terms = sb.terms(TERMS);
    sb.stage("notes.txt", b"nothing restricted\n");
    sb.commit("one");

    let bad = sb.run_range(&terms, &["no-such-revision..HEAD"]);
    assert!(!bad.status.success());
    assert!(
        stderr_of(&bad).contains("could not be read"),
        "{}",
        stderr_of(&bad)
    );

    let clean = sb.run_range(&terms, &["HEAD"]);
    assert!(clean.status.success(), "{}", stderr_of(&clean));

    let missing = sb.run_range(&sb.dir.join("absent.list"), &["HEAD"]);
    assert!(!missing.status.success(), "no list is no check");
    // `terms()` rewrites the one list file, so this is the last use of `terms`.
    let hollow = sb.run_range(&sb.terms("# only a comment\n"), &["HEAD"]);
    assert!(!hollow.status.success(), "an empty list refuses");
}

#[test]
fn the_pre_push_hook_reads_the_range_it_publishes_and_the_bypass_does_not_cover_it() {
    let sb = Sandbox::new("pre-push");
    let terms = sb.terms(TERMS);
    sb.stage("notes.txt", b"a SCE_PROBE_TERM_3 line\n");
    sb.commit("old");
    let published = sb.rev("HEAD");
    sb.stage("notes.txt", b"a SCE_PROBE_TERM_3 line\nand a clean one\n");
    sb.commit("clean");
    let head = sb.rev("HEAD");

    // Only the commits after the remote's are read: the old one is already public.
    let update = format!("refs/heads/main {head} refs/heads/main {published}\n");
    let clean = sb.run_pre_push(&terms, &update);
    assert!(clean.status.success(), "{}", stderr_of(&clean));
    assert!(stderr_of(&clean).contains("SKIP_PREPUSH=1"));

    // A ref the remote does not have yet publishes all of it, and SKIP_PREPUSH does not skip that.
    let new_ref = format!("refs/heads/main {head} refs/heads/main {ZERO}\n");
    let dirty = sb.run_pre_push(&terms, &new_ref);
    assert!(!dirty.status.success(), "{}", stderr_of(&dirty));
    assert!(
        stderr_of(&dirty).contains("term #0"),
        "{}",
        stderr_of(&dirty)
    );
    assert!(!stderr_of(&dirty).contains("SCE_PROBE_TERM_3"));

    // A remote commit this clone does not hold is a force push: what no remote ref reaches is read.
    let unknown = format!(
        "refs/heads/main {head} refs/heads/main {}\n",
        "1".repeat(40)
    );
    let forced = sb.run_pre_push(&terms, &unknown);
    assert!(!forced.status.success(), "{}", stderr_of(&forced));

    // A deletion publishes no commit.
    let deletion = format!("(delete) {ZERO} refs/heads/main {head}\n");
    let gone = sb.run_pre_push(&terms, &deletion);
    assert!(gone.status.success(), "{}", stderr_of(&gone));
}

#[test]
fn a_commit_made_past_the_hooks_is_warned_about_by_post_commit_and_a_clean_one_is_not() {
    let sb = Sandbox::new("post-commit");
    let terms = sb.terms(TERMS);
    let hooks = common::repository::root().join("tools/git-hooks");
    let commit = |message: &str| {
        Command::new("git")
            .arg("-C")
            .arg(sb.repo())
            .args([
                "-c",
                &format!("core.hooksPath={}", hooks.display()),
                "-c",
                "user.name=probe",
                "-c",
                "user.email=probe@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-q",
                "--no-verify",
                "-m",
                message,
            ])
            .env("SCE_RESTRICTED_TERMS", &terms)
            .env_remove("SCE_RESTRICTED_OVERRIDE")
            .output()
            .expect("git runs")
    };

    sb.stage("notes.txt", b"nothing restricted\n");
    let clean = commit("a plain message");
    assert!(clean.status.success(), "{}", stderr_of(&clean));
    assert!(
        !stderr_of(&clean).contains("REFUSED"),
        "{}",
        stderr_of(&clean)
    );

    sb.stage(
        "notes.txt",
        b"nothing restricted\na SCE_PROBE_TERM_5 here\n",
    );
    let dirty = commit("a plain message");
    assert!(
        dirty.status.success(),
        "the commit exists and a post-commit hook cannot undo it:\n{}",
        stderr_of(&dirty)
    );
    let said = stderr_of(&dirty);
    assert!(said.contains("notes.txt: line(s) 2 — term #0"), "{said}");
    assert!(said.contains("would be REFUSED at the push"), "{said}");
    assert!(!said.contains("SCE_PROBE_TERM_5"), "{said}");
}

#[test]
fn an_override_writes_itself_to_a_log_that_holds_no_text_and_an_unwritable_log_is_said() {
    let sb = Sandbox::new("override-log");
    let terms = sb.terms(TERMS);
    sb.stage("notes.txt", b"a SCE_PROBE_TERM_6 line\n");
    sb.commit("a plain message");
    let log = sb.dir.join("override.log");
    let one = Path::new("1");

    let pushed = sb.bash(
        "restricted_term_gate_range pre-push HEAD",
        &terms,
        &[("SCE_RESTRICTED_OVERRIDE", one)],
    );
    assert!(pushed.status.success(), "{}", stderr_of(&pushed));
    assert!(stderr_of(&pushed).contains("DISABLED"));

    let staged = sb.bash(
        "restricted_term_gate_staged",
        &terms,
        &[("SCE_RESTRICTED_OVERRIDE", one)],
    );
    assert!(staged.status.success(), "{}", stderr_of(&staged));

    let written = std::fs::read_to_string(&log).expect("the override wrote a log");
    let rows: Vec<Vec<&str>> = written.lines().map(|l| l.split('\t').collect()).collect();
    assert_eq!(rows.len(), 2, "one row per use:\n{written}");
    assert_eq!(rows[0][1], "pre-push");
    assert_eq!(rows[1][1], "pre-commit");
    assert!(rows.iter().all(|r| r.len() == 4 && r[0].ends_with('Z')));
    assert!(!written.contains("SCE_PROBE_TERM_6"));

    let nowhere = sb.dir.join("no-such-folder/override.log");
    let unlogged = sb.bash(
        "restricted_term_gate_range pre-push HEAD",
        &terms,
        &[
            ("SCE_RESTRICTED_OVERRIDE", one),
            ("SCE_RESTRICTED_OVERRIDE_LOG", &nowhere),
        ],
    );
    assert!(
        unlogged.status.success(),
        "a log that cannot be written does not turn the override into a refusal:\n{}",
        stderr_of(&unlogged)
    );
    assert!(
        stderr_of(&unlogged).contains("could not be written"),
        "{}",
        stderr_of(&unlogged)
    );
}

#[test]
fn every_hook_that_guards_the_list_carries_the_executable_bit() {
    use std::os::unix::fs::PermissionsExt;
    for hook in ["pre-commit", "commit-msg", "pre-push", "post-commit"] {
        let path = common::repository::root()
            .join("tools/git-hooks")
            .join(hook);
        let mode = std::fs::metadata(&path)
            .unwrap_or_else(|e| panic!("{} is missing: {e}", path.display()))
            .permissions()
            .mode();
        assert!(mode & 0o111 != 0, "{} is not executable", path.display());
    }
}
