// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Every backend a `--language` flag's help names must be one that flag
// accepts, and every backend it accepts must be one its help names.
//
// The failure this exists for was live in four of the seven routes:
//
//   * `generate-w3c --help` named `c11`, and `generate-w3c -l c11` exits
//     20 — the same shape as the `--registry` defect
//     `cli_documented_catalog_paths` was written for, one flag over.
//   * The same help omitted `python`, which generates all 202 fixtures
//     and which the repository's own `w3c-python` gate drives.
//   * `generate-conformance --help` named five backends while its
//     dispatcher emitted a 188 KB C harness for a sixth.
//   * `generate` and `orchestrate` omitted `python`, which works — while
//     `check`'s help promised "`check -l X` and `generate -l X` always
//     agree" and correctly named all six.
//
// Counting names cannot find any of this: every name in every menu was
// a real backend, and the sets were still wrong in both directions. The
// only thing that separates a documented backend from a served one is
// handing the documented name to the flag that documents it.
//
// The two sides are deliberately independent sources. The documented
// side is parsed out of the binary's own `--help` (which
// `LanguageRoute::flag_summary` renders); the served side comes from
// running the binary and reading the `cli/*` code on its stderr (which
// the hand-written dispatcher `match` decides). Mutating the table moves
// the first and not the second; mutating a dispatcher moves the second
// and not the first. An oracle that read the table for both would agree
// with itself no matter what either said.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;
use sce_build::cli_language::LanguageRoute;
use sce_build::generator::Language;

/// Lower bound on routes discovered. The menu is parsed by shape out of
/// help text; a clap formatting change that stopped matching would empty
/// the work list and every assertion below would pass vacuously.
const MIN_ROUTES: usize = 7;

/// Lower bound on (route, backend) pairs actually executed.
const MIN_PROBES: usize = 40;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

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

fn help_text(subcommand: &str) -> String {
    let argv: Vec<String> = vec![subcommand.to_string(), "--help".to_string()];
    let (ok, stdout, stderr) = run(&argv);
    assert!(ok, "`{subcommand} --help` failed:\n{stderr}");
    stdout
}

fn top_level_help() -> String {
    let (ok, stdout, stderr) = run(&["--help".to_string()]);
    assert!(ok, "`--help` failed:\n{stderr}");
    stdout
}

/// Subcommand names, read from the top-level help's `Commands:` block —
/// not from [`LanguageRoute::ALL`], so a subcommand that grows a
/// `--language` flag without a table entry is discovered here.
fn subcommands() -> Vec<String> {
    let help = top_level_help();
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

/// The backend menu a subcommand's help declares, or `None` when the
/// subcommand has no `--language` flag.
///
/// Reads the parenthesised list on the `--language` entry — the shape
/// every route renders through `LanguageRoute::flag_summary`.
fn documented_menu(subcommand: &str) -> Option<Vec<String>> {
    let help = help_text(subcommand);
    // The flag entry, then the first parenthesised comma list under it.
    let flag = Regex::new(r"(?m)^\s+(-l, )?--language\b").expect("flag pattern");
    let m = flag.find(&help)?;
    let menu = Regex::new(r"\(([a-z0-9]+(?:, [a-z0-9]+)+)\)").expect("menu pattern");
    let tail = &help[m.start()..];
    let caps = menu.captures(tail).unwrap_or_else(|| {
        panic!(
            "`{subcommand} --help` documents --language but no backend menu \
             follows it. The menu is what a caller reads to learn which \
             backends this route serves; without one the flag documents \
             nothing:\n{tail}"
        )
    });
    Some(caps[1].split(", ").map(|s| s.to_string()).collect())
}

/// Arguments that reach a route's language check and then stop.
///
/// Every entry fails (or lists) for a reason that is not a language
/// reason, so the classification below sees the language verdict and
/// nothing else. Nothing here writes into the tree.
///
/// A route missing from this table is caught by
/// [`every_language_route_is_probed`] rather than silently skipped.
fn probe_args(subcommand: &str, language: &str) -> Option<Vec<String>> {
    let missing = "__sce_no_such_input__.scxml";
    let args: Vec<&str> = match subcommand {
        "generate" => vec!["generate", missing, "-o", "/dev/null", "-l", language],
        "check" => vec!["check", "--scxml", missing, "-l", language],
        "orchestrate" => vec![
            "orchestrate",
            "--scxml",
            missing,
            "-l",
            language,
            "--output-dir",
            "/dev/null",
        ],
        // The C11 refusal fires on the backend match, which runs before
        // the registry is read, so a registry that does not exist still
        // reaches it.
        "generate-w3c" => vec![
            "generate-w3c",
            "-l",
            language,
            "--list",
            "--registry",
            "__sce_no_such_registry__.json",
        ],
        "generate-integration" => vec![
            "generate-integration",
            "-l",
            language,
            "--stem",
            "__sce_no_such_stem__",
        ],
        "generate-conformance" => vec![
            "generate-conformance",
            "-l",
            language,
            "--manifest",
            "__sce_no_such_manifest__.json",
            "--output-dir",
            "/dev/null",
        ],
        // This one parses `--language` only after loading the catalog,
        // so the probe has to hand it a real one. Listing is read-only.
        "list-fixtures" => vec![
            "list-fixtures",
            "--manifest",
            "tests/forge/conformance/fixtures.json",
            "-l",
            language,
        ],
        _ => return None,
    };
    let mut argv: Vec<String> = args.into_iter().map(|s| s.to_string()).collect();
    argv.push("--error-format".to_string());
    argv.push("json".to_string());
    Some(argv)
}

/// What a route answered about one backend.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// No language diagnostic — the route took the name and moved on.
    Served,
    /// `cli/unknown-language` — the name parsed to no backend at all.
    Unknown(Vec<String>),
    /// `cli/unsupported-language` — a real backend this route refuses.
    Unsupported(Vec<String>),
}

/// Classify by the wire `code` on stderr, not by prose.
///
/// The two `cli/*` codes are the documented diagnostic vocabulary, so
/// reading them is reading the contract rather than reading the table
/// under test — a message reworded without changing what is served must
/// not move this verdict.
fn classify(stderr: &str) -> Verdict {
    for line in stderr.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let code = v["code"].as_str().unwrap_or_default();
        if code != "cli/unknown-language" && code != "cli/unsupported-language" {
            continue;
        }
        let candidates: Vec<String> = v["fix"]["candidates"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|c| c.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        return if code == "cli/unknown-language" {
            Verdict::Unknown(candidates)
        } else {
            Verdict::Unsupported(candidates)
        };
    }
    Verdict::Served
}

fn ask(subcommand: &str, language: &str) -> Verdict {
    let argv =
        probe_args(subcommand, language).unwrap_or_else(|| panic!("no probe for `{subcommand}`"));
    let (_, _, stderr) = run(&argv);
    classify(&stderr)
}

/// Subcommands whose help carries a `--language` flag, with their menus.
fn documented_routes() -> BTreeMap<String, Vec<String>> {
    let mut found = BTreeMap::new();
    for name in subcommands() {
        if let Some(menu) = documented_menu(&name) {
            found.insert(name, menu);
        }
    }
    assert!(
        found.len() >= MIN_ROUTES,
        "only {} subcommands were found to document a backend menu, expected \
         at least {MIN_ROUTES}. The menu is parsed by shape out of help text, \
         so a formatting change that stopped matching would look exactly like \
         this — and would make the comparisons below vacuous. Found: {:?}",
        found.len(),
        found.keys().collect::<Vec<_>>()
    );
    found
}

/// Every route in the table documents a menu, and every subcommand that
/// documents one is in the table. A route added to the CLI without a
/// table entry has nowhere to declare its restriction; a table entry for
/// a route that no longer takes `--language` describes nothing.
#[test]
fn table_and_cli_agree_on_which_routes_take_a_language() {
    let documented: BTreeSet<String> = documented_routes().keys().cloned().collect();
    let tabled: BTreeSet<String> = LanguageRoute::ALL
        .iter()
        .map(|r| r.subcommand().to_string())
        .collect();

    let missing_from_table: Vec<_> = documented.difference(&tabled).collect();
    assert!(
        missing_from_table.is_empty(),
        "these subcommands document a --language menu but have no \
         LanguageRoute entry, so their menu is written by hand and can \
         drift from what they serve: {missing_from_table:?}"
    );

    let missing_from_cli: Vec<_> = tabled.difference(&documented).collect();
    assert!(
        missing_from_cli.is_empty(),
        "these LanguageRoute entries name a subcommand whose help has no \
         --language menu: {missing_from_cli:?}"
    );
}

/// The heart of it: for every route, the backends it documents are
/// exactly the backends it serves.
///
/// Both directions matter and both were broken. A documented backend the
/// route refuses sends a caller to exit 20 doing precisely what the help
/// said; a served backend the route hides is a capability nobody outside
/// this repository can find.
#[test]
fn documented_backends_are_the_backends_each_route_serves() {
    let documented = documented_routes();
    let mut probes = 0usize;
    let mut failures = Vec::new();

    for (subcommand, menu) in &documented {
        let declared: BTreeSet<&str> = menu.iter().map(String::as_str).collect();
        let mut served = BTreeSet::new();

        for &language in Language::ALL {
            let name = language.canonical_name();
            probes += 1;
            match ask(subcommand, name) {
                Verdict::Served => {
                    served.insert(name);
                }
                Verdict::Unknown(_) => failures.push(format!(
                    "{subcommand}: `-l {name}` was rejected as an unknown \
                     language, but {name} is a backend this build has"
                )),
                Verdict::Unsupported(_) => {}
            }
        }

        for name in declared.difference(&served) {
            failures.push(format!(
                "{subcommand}: help names `{name}` but the subcommand refuses \
                 it. A caller doing exactly what the help says gets a non-zero \
                 exit"
            ));
        }
        for name in served.difference(&declared) {
            failures.push(format!(
                "{subcommand}: accepts `-l {name}` but its help does not name \
                 it, so the capability is unreachable by reading the CLI. \
                 Help says: {menu:?}"
            ));
        }
    }

    assert!(
        probes >= MIN_PROBES,
        "only {probes} (route, backend) pairs ran, expected at least \
         {MIN_PROBES}"
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Every route named in the table has a probe. Without this, a route
/// added to both the CLI and the table but not to `probe_args` would
/// panic inside `ask` with a message about the test rather than about
/// the product — and a probe table that silently skipped a route would
/// leave that route unmeasured while everything stayed green.
#[test]
fn every_language_route_is_probed() {
    for &route in LanguageRoute::ALL {
        let sub = route.subcommand();
        assert!(
            probe_args(sub, "rust").is_some(),
            "`{sub}` is a LanguageRoute but has no probe in probe_args, so \
             nothing measures what it serves"
        );
    }
}

/// The repair a machine consumer follows has to be the menu the route
/// serves. `fix.candidates` is what an external tool rewrites the
/// invocation from; a global candidate list would hand a `generate-w3c`
/// caller `c11` and send them straight back to the same refusal.
#[test]
fn refusal_candidates_are_the_route_menu() {
    let documented = documented_routes();
    let mut checked = 0usize;

    for (subcommand, menu) in &documented {
        // A name no backend answers to, so every route refuses it.
        match ask(subcommand, "ruby") {
            Verdict::Unknown(candidates) => {
                assert_eq!(
                    &candidates, menu,
                    "{subcommand}: the candidates offered for an unknown \
                     language differ from the menu its help declares"
                );
                checked += 1;
            }
            other => {
                panic!("{subcommand}: `-l ruby` gave {other:?}, expected cli/unknown-language")
            }
        }

        // And where a route refuses a real backend, the candidates it
        // offers instead must also be its menu.
        for &language in Language::ALL {
            let name = language.canonical_name();
            if menu.iter().any(|m| m == name) {
                continue;
            }
            if let Verdict::Unsupported(candidates) = ask(subcommand, name) {
                assert_eq!(
                    &candidates, menu,
                    "{subcommand}: refusing `{name}` offered candidates that \
                     are not this route's menu"
                );
                checked += 1;
            }
        }
    }

    assert!(
        checked >= MIN_ROUTES,
        "only {checked} refusals carried a candidate list, expected at least \
         {MIN_ROUTES}"
    );
}

/// A route that refuses a backend has to say so in its own help, naming
/// the backend it refuses.
///
/// Otherwise the menu is the whole story a caller gets, and the natural
/// reading of a five-name menu is "the sixth backend does not exist"
/// rather than "it exists and this route is not how you reach it". For
/// `generate-w3c` the difference is the whole answer: `c11` is a working
/// target of `generate`, and a caller who concluded otherwise from the
/// batch route's menu would give up on a backend they have.
#[test]
fn a_route_that_refuses_a_backend_names_it_in_help() {
    let documented = documented_routes();
    let mut checked = 0usize;

    for (subcommand, menu) in &documented {
        let excluded: Vec<&str> = Language::ALL
            .iter()
            .map(|l| l.canonical_name())
            .filter(|name| !menu.iter().any(|m| m == name))
            .collect();
        if excluded.is_empty() {
            continue;
        }
        let help = help_text(subcommand);
        // Everything after the menu — the reason has to be additional
        // prose, not the menu line read twice.
        for name in &excluded {
            assert!(
                help.contains(name),
                "{subcommand}: refuses `{name}` and its help never mentions \
                 it, so a caller reading the menu learns the backend does \
                 not exist rather than that this route is not how to reach \
                 it. Menu: {menu:?}"
            );
            checked += 1;
        }
    }

    assert!(
        checked >= 3,
        "only {checked} refused backends were checked for an explanation; \
         two routes restrict their menu (one backend and two backends), so \
         three is the count this build has"
    );
}

/// A backend this build serves somewhere has to be reachable by reading
/// the CLI. `python` was served by five routes and named by one, which
/// is how an external author concludes SCE has no Python conformance
/// suite while the repository's own gate generates 219 Python cases.
#[test]
fn every_backend_is_documented_by_some_route() {
    let documented = documented_routes();
    for &language in Language::ALL {
        let name = language.canonical_name();
        let routes: Vec<&String> = documented
            .iter()
            .filter(|(_, menu)| menu.iter().any(|m| m == name))
            .map(|(sub, _)| sub)
            .collect();
        assert!(
            !routes.is_empty(),
            "no subcommand's help names `{name}`, so nothing in the CLI tells \
             a caller this backend exists"
        );
    }
}
