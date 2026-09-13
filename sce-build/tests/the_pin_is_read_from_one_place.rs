// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The Mnemosyne revision has one reader in shell, and a revision root has one
// list of what it must carry.
//
// WHAT THIS IS ABOUT — measured 2026-09-14, during the bump to `df1f17ce`.
//
// The pin is a REVISION-level fact. `[tool] pin` in each `mnemosyne.toml`
// names the revision allowed to act on that workspace, and a binary carrying
// another stamp execs into the pinned build rather than judging under its own.
// Procurement and verification were BINARY-level: one `cargo install` of
// `mnemosyne-cli`, and one gate check of `mnemosyne-cli`. A revision root
// holding that binary alone therefore satisfied every check this repository
// had — while the MCP servers configured against the same workspaces execed
// toward a `mnemosyne-mcp` that was not in it and died at start-up, reaching
// their client as `CONNECTION_CLOSED` with no revision and no install line.
//
// Two properties keep that from coming back, and neither is a style rule:
//
//   ONE READER  a second `sed` over the workflow line is a second copy of the
//               revision, free to drift the moment one of them is edited. The
//               gate carried exactly such a copy until this round.
//   ONE LIST    the set of binaries a root must carry is what makes "install
//               the revision" a complete act rather than a per-binary one. Two
//               declarations of it are two answers to "is this root filled?".
//
// ⚠ These are shell properties on purpose. `sce-build/tests/*.rs` and
// `tools/mnemosyne-adoption/tests/_mnemosyne_bin.py` also read the pin, in
// their own languages, and making them shell out to read it would trade a
// legible reader for a fragile one. What is forbidden is a SECOND SHELL
// reader, because the shell consumers — the gate, the installer, the MCP
// launcher — are the ones that fill and judge the root together.

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .expect("git rev-parse");
    PathBuf::from(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Every tracked `*.sh` under `scripts/`, with its text.
///
/// Enumerated from the tree rather than from a list here: a list would be a
/// third place to forget, which is the shape of the defect this file is about.
fn shell_scripts(root: &Path) -> Vec<(String, String)> {
    let out = std::process::Command::new("git")
        .current_dir(root)
        .args(["ls-files", "scripts/*.sh", "scripts/**/*.sh"])
        .output()
        .expect("git ls-files");
    let mut found = Vec::new();
    for rel in String::from_utf8_lossy(&out.stdout).lines() {
        let rel = rel.trim();
        if rel.is_empty() {
            continue;
        }
        if let Ok(text) = fs::read_to_string(root.join(rel)) {
            found.push((rel.to_string(), text));
        }
    }
    found
}

/// A line that PARSES the pin, as opposed to one that merely names it.
///
/// The discriminator is the capture group: `mnemosyne_pin_rev` extracts the
/// 40-hex value, while the comments and failure messages that mention
/// `MNEMOSYNE_REV` carry no `[0-9a-f]` class. Matching the bare token would
/// have counted every one of those and made the assertion unpassable — and
/// matching nothing at all would have made it vacuous, which the floor below
/// is what catches.
fn parses_the_pin(text: &str) -> bool {
    text.lines()
        .any(|l| l.contains("MNEMOSYNE_REV") && l.contains("[0-9a-f]"))
}

#[test]
fn exactly_one_shell_file_parses_the_revision() {
    let root = repo_root();
    let scripts = shell_scripts(&root);

    // Floor first: an enumeration that stopped finding the tree would make
    // every assertion below pass by having examined nothing. This tree
    // carries dozens; the bound is deliberately far under that so deleting a
    // script stays an ordinary edit.
    assert!(
        scripts.len() >= 10,
        "found {} shell script(s) under scripts/ — the sweep is not reaching the tree",
        scripts.len()
    );

    let readers: Vec<&str> = scripts
        .iter()
        .filter(|(_, text)| parses_the_pin(text))
        .map(|(rel, _)| rel.as_str())
        .collect();

    assert_eq!(
        readers,
        vec!["scripts/lib/mnemosyne_pin.sh"],
        "the revision must be parsed in exactly one shell file. A second reader is a \
         second copy of the pin: it passes while both agree and diverges silently the \
         moment one is edited, which is what `scripts/gates/ledger-citations.sh` did \
         until 2026-09-14. Source `scripts/lib/mnemosyne_pin.sh` instead."
    );
}

#[test]
fn the_revision_root_has_one_list_of_what_it_must_carry() {
    let root = repo_root();
    let scripts = shell_scripts(&root);

    let declarations: Vec<&str> = scripts
        .iter()
        .filter(|(_, text)| text.contains("MNEMOSYNE_PIN_BINARIES=("))
        .map(|(rel, _)| rel.as_str())
        .collect();

    assert_eq!(
        declarations,
        vec!["scripts/lib/mnemosyne_pin.sh"],
        "the binaries a revision root must carry are declared in exactly one place"
    );

    let lib = fs::read_to_string(root.join("scripts/lib/mnemosyne_pin.sh"))
        .expect("read scripts/lib/mnemosyne_pin.sh");
    let listed = lib
        .lines()
        .find_map(|l| l.strip_prefix("MNEMOSYNE_PIN_BINARIES=("))
        .and_then(|rest| rest.split_once(')'))
        .map(|(names, _)| names.split_whitespace().count())
        .expect("MNEMOSYNE_PIN_BINARIES is a one-line array literal");

    // An arity floor, not a count to keep in step. One name would mean the
    // list had collapsed back to "install the CLI", which is the state the
    // MCP servers died in — and a list is what makes the install complete.
    assert!(
        listed >= 2,
        "MNEMOSYNE_PIN_BINARIES lists {listed} binary(ies); a revision root serves more \
         than the CLI, and a list of one is the per-binary procurement this removed"
    );
}

/// Every workspace's `[tool] pin` names the revision the workflow procures.
///
/// This copy of the revision CANNOT be removed, and that is why it is checked
/// here rather than deleted. Mnemosyne reads `mnemosyne.toml`; it cannot read
/// this repository's workflow, so the enforcement half has to restate the
/// value. The MCP client's copy could be removed and was; this one is the
/// residue, and a residue nobody checks is the same defect wearing a reason.
///
/// The state it catches is not hypothetical. On 2026-09-14 the workflow was
/// bumped to `df1f17ce` while the five pins still read `ecee1fe0`, and the
/// new binary exec'd into the OLD build on every local run — the gate
/// reported green for a revision it had not run, and said so only in a note
/// above its own report. On a CI runner, where that old root does not exist,
/// the same tree is a refusal instead.
#[test]
fn every_workspace_pin_names_the_procured_revision() {
    let root = repo_root();

    let workflow = fs::read_to_string(root.join(".github/workflows/spec-citations.yml"))
        .expect("read .github/workflows/spec-citations.yml");
    let rev = workflow
        .lines()
        .find_map(|l| l.trim().strip_prefix("MNEMOSYNE_REV:"))
        .map(str::trim)
        .expect("the workflow declares MNEMOSYNE_REV");
    assert_eq!(rev.len(), 40, "MNEMOSYNE_REV is a 40-hex revision: {rev:?}");
    let short = &rev[..8];

    let out = std::process::Command::new("git")
        .current_dir(&root)
        .args(["ls-files", "*mnemosyne.toml"])
        .output()
        .expect("git ls-files");
    let configs: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    // A floor, because "every pin matches" is trivially true of no pins at
    // all — and an enumeration that stopped finding the workspaces is exactly
    // how this assertion would go quiet while the drift it exists for grew.
    assert!(
        configs.len() >= 5,
        "found {} mnemosyne workspace config(s); this tree carries five",
        configs.len()
    );

    let mut wrong: Vec<String> = Vec::new();
    for rel in &configs {
        let text = fs::read_to_string(root.join(rel)).expect("read a mnemosyne.toml");
        match text
            .lines()
            .find_map(|l| l.trim().strip_prefix("pin = "))
            .map(|v| v.trim().trim_matches('"').to_string())
        {
            Some(p) if p == short => {}
            Some(p) => wrong.push(format!("{rel}: pin = {p:?}, workflow procures {short:?}")),
            None => wrong.push(format!("{rel}: no [tool] pin")),
        }
    }

    assert!(
        wrong.is_empty(),
        "a workspace pin disagrees with the revision the workflow installs. Bump both \
         together — `MNEMOSYNE_REV` and every `[tool] pin` — and install before \
         declaring:\n  {}",
        wrong.join("\n  ")
    );
}

#[test]
fn the_mcp_launcher_derives_the_root_instead_of_naming_a_revision() {
    let root = repo_root();
    let launcher = root.join("scripts/mnemosyne_mcp.sh");
    let text = fs::read_to_string(&launcher).expect("read scripts/mnemosyne_mcp.sh");

    assert!(
        text.contains("mnemosyne_pin_root"),
        "the launcher must derive the revision root, so a bump reaches the MCP client \
         with no second edit"
    );

    // A revision-keyed path spelled out in code is the third copy of the pin —
    // the one that lived in `~/.claude.json` and that no gate could read.
    // Comments may quote one as history; a command may not.
    let code_naming_a_rev: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('#'))
        .filter(|l| {
            l.contains("mnemosyne-rev/")
                && l.split("mnemosyne-rev/").nth(1).is_some_and(|rest| {
                    rest.len() >= 8 && rest[..8].chars().all(|c| c.is_ascii_hexdigit())
                })
        })
        .collect();

    assert!(
        code_naming_a_rev.is_empty(),
        "the launcher names a revision directly: {code_naming_a_rev:?} — derive it from \
         the pin instead"
    );
}
