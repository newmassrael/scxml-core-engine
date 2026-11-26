// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! No workflow installs a system package at job time (docs/adr/0145, D1).
//!
//! A job that runs `apt-get install` reads a package archive the repository does
//! not name, on every run: its verdict can be decided by the network (a DNS
//! failure in job 130569626348, HTTP 503 in jobs 135541943618 and 135550606075)
//! and its toolchain is whatever the archive holds that day. A gate's packages
//! are its image's instead (`container` of its registry entry, built from a
//! Containerfile that reads a dated snapshot), and the lane runs the gate in
//! that image through `scripts/gate`.
//!
//! So an install is refused in every file under `.github/`, in any spelling apt
//! takes: `apt-get` or `apt`, by any path, with any options before `install`,
//! on a line of its own or continued, in a step's script or a here-document
//! it runs. A comment is prose and is not read. There is no exception: every
//! lane that ran `apt-get` runs its gates in an image.

use std::fs;
use std::path::{Path, PathBuf};

/// apt's options that take a value as the next word (`-o Dpkg::Options::=...`),
/// which is not the command that follows.
const APT_OPTIONS_WITH_A_VALUE: &[&str] = &[
    "-o",
    "-c",
    "-t",
    "--option",
    "--config-file",
    "--target-release",
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// Every file under `.github/`, as its path from the repository root and its
/// text, sorted. A file that is not text is not a script and is passed over.
fn github_files() -> Vec<(String, String)> {
    let root = repo_root();
    let mut pending = vec![root.join(".github")];
    let mut out = Vec::new();
    while let Some(dir) = pending.pop() {
        let entries = fs::read_dir(&dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display()));
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            let rel = path
                .strip_prefix(&root)
                .expect("a file under .github is under the repository root")
                .to_string_lossy()
                .replace('\\', "/");
            out.push((rel, text));
        }
    }
    out.sort();
    out
}

/// The lines of `text` a shell would run, each with the 1-based number of the
/// line it starts on: whole-line comments dropped, a trailing comment cut, and
/// a line that ends in a backslash joined to the next, as the shell joins them.
fn command_lines(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut pending: Option<(usize, String)> = None;
    for (index, raw) in text.lines().enumerate() {
        let trimmed = raw.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        let code = match trimmed.find(" #") {
            Some(at) => &trimmed[..at],
            None => trimmed,
        };
        let (code, continues) = match code.strip_suffix('\\') {
            Some(head) => (head, true),
            None => (code, false),
        };
        let (number, mut line) = pending.take().unwrap_or((index + 1, String::new()));
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(code.trim());
        if continues {
            pending = Some((number, line));
        } else {
            out.push((number, line));
        }
    }
    if let Some(last) = pending {
        out.push(last);
    }
    out
}

/// The command a run of apt names after `words`, its options passed over.
fn apt_command<'a>(words: &[&'a str]) -> Option<&'a str> {
    let mut rest = words.iter();
    while let Some(word) = rest.next() {
        if APT_OPTIONS_WITH_A_VALUE.contains(word) {
            rest.next();
            continue;
        }
        if word.starts_with('-') {
            continue;
        }
        return Some(*word);
    }
    None
}

/// Whether `line` runs apt's `install`: `apt-get` or `apt`, by any path,
/// then any options, then `install`.
fn installs_a_package(line: &str) -> bool {
    let words: Vec<&str> = line
        .split(|c: char| c.is_whitespace() || matches!(c, ';' | '&' | '|' | '(' | ')' | '`'))
        .map(|word| word.trim_matches(|c| c == '"' || c == '\''))
        .filter(|word| !word.is_empty())
        .collect();
    words.iter().enumerate().any(|(at, &word)| {
        let program = word.rsplit('/').next().unwrap_or(word);
        matches!(program, "apt-get" | "apt") && apt_command(&words[at + 1..]) == Some("install")
    })
}

/// The numbers of the lines of `text` that install a package.
fn install_lines(text: &str) -> Vec<usize> {
    command_lines(text)
        .into_iter()
        .filter(|(_, line)| installs_a_package(line))
        .map(|(number, _)| number)
        .collect()
}

#[test]
fn no_workflow_installs_a_package_at_job_time() {
    let files = github_files();
    assert!(
        files.len() > 20,
        "read only {} file(s) under .github; the walk is broken, and a check that \
         reads nothing passes",
        files.len()
    );

    let mut refused: Vec<String> = Vec::new();
    for (path, text) in &files {
        let lines = install_lines(text);
        if !lines.is_empty() {
            let at: Vec<String> = lines.iter().map(usize::to_string).collect();
            refused.push(format!("{path}: line(s) {}", at.join(", ")));
        }
    }
    assert!(
        refused.is_empty(),
        "a workflow installs packages at job time, so its verdict reads an archive \
         the repository does not name:\n  {}\n\
         Name the packages in the Containerfile of the image the gate runs in \
         (`container` of its entry under tools/git-hooks/gates/) and run the gate \
         through `scripts/gate` (docs/adr/0145).",
        refused.join("\n  ")
    );
}

#[test]
fn the_scanner_reads_an_install_in_any_spelling_and_leaves_the_rest() {
    for line in [
        "sudo apt-get install -y cmake",
        "apt install git",
        "run: sudo apt-get install -y libxml2-dev",
        "- run: apt install -y git",
        "sudo apt-get -y -q install ninja-build",
        "apt-get --no-install-recommends install clang-19",
        "apt-get -o Dpkg::Options::=--force-confnew install tzdata",
        "DEBIAN_FRONTEND=noninteractive /usr/bin/apt-get install -y g++",
        "sudo apt-get update && sudo apt-get install -y qemu-system-arm",
    ] {
        assert!(installs_a_package(line), "not read as an install: {line:?}");
    }
    for line in [
        "sudo apt-get update",
        "apt-get remove -y cmake",
        "apt-cache policy cmake",
        "echo apt-get is not run here",
        "scripts/gate tree-hygiene",
    ] {
        assert!(!installs_a_package(line), "read as an install: {line:?}");
    }

    // A comment is prose, whole or at the end of a line.
    assert!(install_lines("    # sudo apt-get install -y cmake\n").is_empty());
    assert!(install_lines("    run: make # then apt-get install -y cmake\n").is_empty());
    // A command continued over lines is the one command it is, at its first line.
    assert_eq!(
        install_lines("run: |\n  sudo apt-get \\\n    install -y \\\n    cmake\n"),
        vec![2]
    );
    // A here-document a step runs is commands too.
    assert_eq!(
        install_lines("run: |\n  bash <<'EOF'\n  apt-get install -y cmake\n  EOF\n"),
        vec![3]
    );
}
