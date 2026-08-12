// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Every catalog path the CLI's own `--help` names must be a path that
// flag accepts.
//
// The failure this exists for is not hypothetical. When the W3C
// conformance registry moved from `tests/CMakeLists.txt` to
// `tests/w3c/conformance/fixtures.json`, the loader, the project-root
// marker and CMake all followed; `--registry`'s help did not. A caller
// that did what the help said got exit 20 and a raw serde message about
// column 1. The path the help named still existed on disk, so an
// existence check would have reported success — existence is not
// acceptance, and only feeding the documented path to the flag that
// documents it can tell the two apart.
//
// The scan derives its work list from the binary's own help output
// rather than a hand-kept list of flags, so a new flag that documents a
// catalog path is covered the moment it ships: it appears in the
// discovered set, finds no probe, and fails here.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// stdout of an invocation run from the repository root.
fn run(args: &[String]) -> (bool, String, String) {
    let out = Command::new(sce_codegen_bin())
        .args(args)
        .current_dir(repo_root())
        .output()
        .expect("run sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn help_text(args: &[&str]) -> String {
    let mut argv: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    argv.push("--help".to_string());
    let (ok, stdout, stderr) = run(&argv);
    assert!(ok, "`{argv:?}` failed:\n{stderr}");
    stdout
}

/// Subcommand names, read from the top-level help's `Commands:` block.
fn subcommands() -> Vec<String> {
    let help = help_text(&[]);
    let entry = Regex::new(r"^  ([a-z][a-z0-9-]*)\s\s+\S").expect("subcommand pattern");
    let mut names = Vec::new();
    let mut inside = false;
    for line in help.lines() {
        if line.starts_with("Commands:") {
            inside = true;
            continue;
        }
        if !inside {
            continue;
        }
        if line.starts_with("Options:") {
            break;
        }
        if let Some(c) = entry.captures(line) {
            let name = c[1].to_string();
            if name != "help" {
                names.push(name);
            }
        }
    }
    assert!(
        names.len() >= 15,
        "the top-level help listed {} subcommands; the Commands: block is \
         parsed by shape, and a formatting change that empties it would \
         make every assertion below vacuous",
        names.len()
    );
    names
}

/// Every (flag, repository-relative catalog path) pair one subcommand's
/// long help states.
///
/// A catalog is named by file: `fixtures.json` is what both the forge
/// and the W3C catalogs are called, and `CMakeLists.txt` is what the
/// W3C one used to be. Matching the filename rather than a directory
/// keeps a moved catalog inside the scan.
fn documented_catalog_paths(subcommand: &str) -> Vec<(String, String)> {
    let help = help_text(&[subcommand]);
    let flag = Regex::new(r"^\s{2,6}(?:-\S, )?--([a-z0-9-]+)").expect("flag pattern");
    let catalog =
        Regex::new(r"[A-Za-z0-9_./-]*(?:fixtures\.json|CMakeLists\.txt)").expect("path pattern");

    let mut found: BTreeSet<(String, String)> = BTreeSet::new();
    let mut current: Option<String> = None;
    for line in help.lines() {
        if let Some(c) = flag.captures(line) {
            current = Some(format!("--{}", &c[1]));
            continue;
        }
        let Some(flag_name) = current.as_ref() else {
            continue;
        };
        for m in catalog.find_iter(line) {
            let path = m.as_str().trim_start_matches("./").to_string();
            // A bare filename says nothing about where the catalog is;
            // only a repository-relative path is a claim this test can
            // check.
            if path.contains('/') {
                found.insert((flag_name.clone(), path));
            }
        }
    }
    found.into_iter().collect()
}

/// One invocation that makes the CLI read `path` through `flag` and
/// report a verdict.
///
/// The invocation is spelled out rather than derived because "accepted"
/// has no universal probe: `--registry` is read by a listing mode,
/// `--manifest` by a renderer that needs somewhere to write. Both
/// directions of the table are checked below, so a row cannot outlive
/// the help text it stands for, nor a help text go unprobed.
struct Probe {
    subcommand: &'static str,
    flag: &'static str,
    path: &'static str,
    /// `{path}` is replaced with the documented path, `{out}` with a
    /// scratch directory.
    args: &'static [&'static str],
}

const PROBES: &[Probe] = &[
    Probe {
        subcommand: "generate-w3c",
        flag: "--registry",
        path: "tests/w3c/conformance/fixtures.json",
        args: &[
            "generate-w3c",
            "-l",
            "rust",
            "--list",
            "--registry",
            "{path}",
        ],
    },
    Probe {
        subcommand: "generate-conformance",
        flag: "--manifest",
        path: "tests/forge/conformance/fixtures.json",
        args: &[
            "generate-conformance",
            "-l",
            "rust",
            "--manifest",
            "{path}",
            "-o",
            "{out}",
        ],
    },
    Probe {
        subcommand: "list-fixtures",
        flag: "--manifest",
        path: "tests/forge/conformance/fixtures.json",
        args: &["list-fixtures", "--manifest", "{path}"],
    },
    Probe {
        subcommand: "list-fixtures",
        flag: "--catalog",
        path: "tests/forge/conformance/fixtures.json",
        args: &[
            "list-fixtures",
            "--catalog",
            "forge",
            "--manifest",
            "{path}",
        ],
    },
    Probe {
        subcommand: "list-fixtures",
        flag: "--catalog",
        path: "tests/w3c/conformance/fixtures.json",
        args: &["list-fixtures", "--catalog", "w3c", "--manifest", "{path}"],
    },
];

#[test]
fn every_documented_catalog_path_is_accepted_by_the_flag_that_names_it() {
    let scratch = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("documented-catalog-paths");
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).expect("create scratch dir");

    let mut discovered: BTreeSet<(String, String, String)> = BTreeSet::new();
    for subcommand in subcommands() {
        for (flag, path) in documented_catalog_paths(&subcommand) {
            discovered.insert((subcommand.clone(), flag, path));
        }
    }

    // The scan reads prose out of a help formatter. If it ever finds
    // nothing it has to say so loudly, because "no violations" and "no
    // input" print the same.
    assert!(
        discovered.len() >= 5,
        "found {} documented catalog paths across the CLI; the scan reads \
         `--help` output by shape and an empty result means the parse \
         broke, not that the CLI stopped documenting catalogs: {discovered:?}",
        discovered.len()
    );

    let mut violations: Vec<String> = Vec::new();

    for (subcommand, flag, path) in &discovered {
        // Prefer the row written for this exact path, and fall back to
        // any row driving the same flag. The fallback is what makes a
        // drifted help text report the refusal itself rather than only
        // that the table has no row for the new spelling.
        let exact = PROBES.iter().find(|p| {
            p.subcommand == subcommand.as_str() && p.flag == flag && p.path == path.as_str()
        });
        let Some(probe) = exact.or_else(|| {
            PROBES
                .iter()
                .find(|p| p.subcommand == subcommand.as_str() && p.flag == flag)
        }) else {
            violations.push(format!(
                "`{subcommand} --help` documents `{path}` for `{flag}`, and no probe \
                 feeds a path to that flag — add a row to PROBES so the claim is checked"
            ));
            continue;
        };

        let argv: Vec<String> = probe
            .args
            .iter()
            .map(|a| {
                a.replace("{path}", path)
                    .replace("{out}", &scratch.display().to_string())
            })
            .collect();
        let (ok, _stdout, stderr) = run(&argv);
        if !ok {
            violations.push(format!(
                "`{subcommand} --help` documents `{path}` for `{flag}`, but \
                 `sce-codegen {}` refuses it:\n{stderr}",
                argv.join(" ")
            ));
        }
    }

    for probe in PROBES {
        let key = (
            probe.subcommand.to_string(),
            probe.flag.to_string(),
            probe.path.to_string(),
        );
        if !discovered.contains(&key) {
            violations.push(format!(
                "PROBES carries a row for `{} {} {}`, and that subcommand's help \
                 no longer names that path — a stale row probes a claim nobody makes",
                probe.subcommand, probe.flag, probe.path
            ));
        }
    }

    let _ = std::fs::remove_dir_all(&scratch);

    assert!(
        violations.is_empty(),
        "{} documented catalog path(s) disagree with what the CLI accepts:\n{}",
        violations.len(),
        violations.join("\n")
    );
}

#[test]
fn the_registry_flag_documents_the_path_it_falls_back_to() {
    // Acceptance alone would still allow the help to name a second
    // valid catalog while the default resolved elsewhere. Naming the
    // path a caller gets by saying nothing is the stronger claim, and
    // it is the one an external caller reads the help for.
    let documented = sce_build::w3c_registry::W3C_REGISTRY_RELATIVE_PATH;
    let help = help_text(&["generate-w3c"]);
    assert!(
        help.contains(documented),
        "`generate-w3c --help` does not name `{documented}`, which is the \
         registry the subcommand reads when `--registry` is unset:\n{help}"
    );

    let default_run = run(&["generate-w3c", "-l", "rust", "--list"].map(String::from));
    let named_run = run(&[
        "generate-w3c".to_string(),
        "-l".to_string(),
        "rust".to_string(),
        "--list".to_string(),
        "--registry".to_string(),
        documented.to_string(),
    ]);
    assert!(default_run.0, "listing with no --registry failed");
    assert!(named_run.0, "listing with the documented --registry failed");
    assert_eq!(
        default_run.1, named_run.1,
        "the registry `generate-w3c` falls back to and the one its help \
         names produce different listings"
    );
}

#[test]
fn a_registry_that_is_not_the_catalog_is_refused_by_naming_the_catalog() {
    // `tests/CMakeLists.txt` is what the registry used to be, so it is
    // exactly what a caller working from stale instructions passes. It
    // is a real file, which is why the refusal has to carry where the
    // registry actually lives — a JSON parse error at column 1 leaves
    // the caller with nowhere to go.
    let stale = repo_root().join("tests/CMakeLists.txt");
    assert!(stale.is_file(), "the stale registry path still exists");

    let (ok, _stdout, stderr) = run(&[
        "generate-w3c".to_string(),
        "-l".to_string(),
        "rust".to_string(),
        "--list".to_string(),
        "--registry".to_string(),
        stale.display().to_string(),
    ]);
    assert!(!ok, "a build script was accepted as a conformance registry");
    assert!(
        stderr.contains(sce_build::w3c_registry::W3C_REGISTRY_RELATIVE_PATH),
        "the refusal does not say where the registry lives:\n{stderr}"
    );
}
