// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A workflow runs on an image it names, not on the one GitHub currently calls
//! "latest".
//!
//! `ubuntu-latest` is a label GitHub moves. On 2026-11-06 it moved from
//! `ubuntu-24.04` to `ubuntu-26.04` while pushes were in flight: the runs before
//! 06:56 UTC were on the old image and the runs after 07:19 on the new one, and the
//! same commits changed verdict with it. The new image ships a `quickjs.c` warning
//! the old compiler did not raise (`-Werror=discarded-qualifiers`), no `libxml2`
//! package under that name, and a clang whose version is written into the committed
//! `embed/MANIFEST.json`, so five lanes went red for a reason no commit had made and
//! no commit could fix by itself.
//!
//! A pin makes the image a thing the repository changes, in a commit, with the
//! failures of the move in front of the person making it. The label that moves under
//! the repository is refused here, in every workflow, for every operating system.
//!
//! The same holds for a tool a workflow installs: `emsdk install latest` is a name the
//! tool's releases move, so it is refused beside the image labels, and the release the
//! lane was measured on is named in its place.

use std::fs;
use std::path::{Path, PathBuf};

/// The labels GitHub moves. A name with a version in it (`ubuntu-24.04`,
/// `macos-15`, `windows-2025`) is fixed until GitHub retires the image.
const MOVING_LABELS: &[&str] = &["ubuntu-latest", "macos-latest", "windows-latest"];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// Every workflow as `(file name, text)`, sorted for stable diagnostics.
fn workflows() -> Vec<(String, String)> {
    let dir = repo_root().join(".github/workflows");
    let mut out: Vec<(String, String)> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {}", dir.display(), e))
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|x| x.to_str())
                .is_some_and(|x| x == "yml" || x == "yaml")
        })
        .map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let text = fs::read_to_string(e.path())
                .unwrap_or_else(|err| panic!("read {}: {}", e.path().display(), err));
            (name, text)
        })
        .collect();
    out.sort();
    assert!(
        out.len() > 5,
        "found only {} workflow file(s) — enumeration is broken",
        out.len()
    );
    out
}

/// The moving label a line of YAML names, if it names one outside a comment.
///
/// A whole-line comment and the tail after ` #` are prose, and prose may say why
/// a label is avoided. A label is a word of its own, so `ubuntu-latest-arm` or a
/// longer name that merely contains one is not matched by accident.
fn moving_label(line: &str) -> Option<&'static str> {
    let code = match line.find(" #") {
        Some(at) => &line[..at],
        None => line,
    };
    if code.trim_start().starts_with('#') {
        return None;
    }
    code.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '.'))
        .find_map(|word| MOVING_LABELS.iter().copied().find(|label| *label == word))
}

#[test]
fn no_workflow_runs_on_a_label_github_moves() {
    let mut found = Vec::new();
    for (name, text) in workflows() {
        for (idx, line) in text.lines().enumerate() {
            if let Some(label) = moving_label(line) {
                found.push(format!("{name}:{}: `{label}`", idx + 1));
            }
        }
    }
    assert!(
        found.is_empty(),
        "a workflow names an image GitHub moves; name the version it was measured on \
         (`ubuntu-24.04`, not `ubuntu-latest`) so the move is a commit of ours:\n  {}",
        found.join("\n  ")
    );
}

/// The tool installs a workflow may not make by the name `latest`: a name the tool's own releases move.
/// `emsdk install latest` moved to Emscripten 7.0.0 on 2026-11-06, and the visualizer WASM build, green the
/// run before on 6.0.13, stopped compiling for a reason no commit of ours had made.
const MOVING_TOOL_INSTALLS: &[&str] = &["install latest", "activate latest"];

/// The moving tool install a line of YAML makes, if it makes one outside a comment.
fn moving_tool_install(line: &str) -> Option<&'static str> {
    let code = match line.find(" #") {
        Some(at) => &line[..at],
        None => line,
    };
    if code.trim_start().starts_with('#') {
        return None;
    }
    MOVING_TOOL_INSTALLS
        .iter()
        .copied()
        .find(|install| code.contains(install))
}

#[test]
fn no_workflow_installs_a_tool_by_a_name_its_releases_move() {
    let mut found = Vec::new();
    for (name, text) in workflows() {
        for (idx, line) in text.lines().enumerate() {
            if let Some(install) = moving_tool_install(line) {
                found.push(format!("{name}:{}: `{install}`", idx + 1));
            }
        }
    }
    assert!(
        found.is_empty(),
        "a workflow installs a tool by `latest`; name the release it was measured on \
         (`emsdk install 6.0.13`) so the move is a commit of ours:\n  {}",
        found.join("\n  ")
    );
}

#[test]
fn the_tool_check_reads_an_install_and_leaves_prose_and_releases_alone() {
    assert_eq!(
        moving_tool_install("        ./emsdk install latest"),
        Some("install latest")
    );
    assert_eq!(
        moving_tool_install("        ./emsdk activate latest"),
        Some("activate latest")
    );
    assert_eq!(moving_tool_install("        ./emsdk install 6.0.13"), None);
    assert_eq!(
        moving_tool_install("        ./emsdk install 6.0.13 # not install latest"),
        None
    );
    assert_eq!(
        moving_tool_install("        # `latest` moved to 7.0.0; install latest was the cause"),
        None
    );
}

#[test]
fn the_check_reads_a_label_and_leaves_prose_and_versions_alone() {
    // A control for the check above: it must fire on each moving label and stay
    // quiet on a pinned one, a comment, and a name that only contains a label.
    assert_eq!(
        moving_label("    runs-on: ubuntu-latest"),
        Some("ubuntu-latest")
    );
    assert_eq!(
        moving_label("        os: [macos-latest, macos-15]"),
        Some("macos-latest")
    );
    assert_eq!(
        moving_label("    runs-on: \"windows-latest\""),
        Some("windows-latest")
    );
    assert_eq!(moving_label("    runs-on: ubuntu-24.04"), None);
    assert_eq!(
        moving_label("    runs-on: ubuntu-24.04 # not ubuntu-latest"),
        None
    );
    assert_eq!(moving_label("      # ubuntu-latest pre-installs it"), None);
    assert_eq!(moving_label("    runs-on: ubuntu-latest-arm"), None);
}

#[test]
fn the_workflows_do_run_on_a_pinned_image() {
    // The check above passes on a tree with no `runs-on` at all; this keeps it from
    // being satisfied by the key going unread.
    let pinned = workflows()
        .iter()
        .filter(|(_, text)| {
            text.lines()
                .any(|l| l.trim_start().starts_with("runs-on: ubuntu-24.04"))
        })
        .count();
    assert!(
        pinned > 5,
        "only {pinned} workflow(s) name `ubuntu-24.04`; the runner key is not being read"
    );
}
