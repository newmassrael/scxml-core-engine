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
//! Every run here reads a synthetic term list through `SCE_RESTRICTED_TERMS`.
//! The real list lives outside the repository and is never opened by a test:
//! reading it would copy the terms it exists to keep out.

#![cfg(unix)]

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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
