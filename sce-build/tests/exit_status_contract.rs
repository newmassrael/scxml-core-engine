// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Every way this process can end non-zero carries a diagnostic record,
// and the status it ends with is one `SCE_ERROR_CONTRACT.md` §6 assigns.
//
// §6 states both halves as universals — "A non-zero exit with no NDJSON
// record is a contract violation", and a table that maps a code prefix
// to a status — and until this file there was nothing that fed the
// binary a failure and checked either one. Eight failure modes were
// live:
//
//   * `addr2sce --symbol <miss>`, `sce2sym` with no match and
//     `generate-integration --stem <miss>` exited **1**, a status §6
//     does not define at all, with prose and no record.
//   * `addr2sce` with no mode flag, `--pc` without `--elf`, a non-hex
//     `--pc`, and `--hardfault` on empty stdin exited **2** — the
//     status §6 reserves for `xml/*`. A caller reading only the status
//     was told its *document* was malformed.
//   * The argument parser's own failure path did the same for every
//     subcommand: a missing argument or an unknown flag left through
//     clap's `parse()`, which prints prose and exits 2.
//
// None of that is visible to the checks that already existed.
// `error_format_json` pins the shape of records that *are* emitted; a
// path that emits none is outside its reach. The diagnostic schema
// cannot see a process that never produced a record. The only thing
// that separates "this failure is reportable" from "this failure is
// prose on stderr" is running it.
//
// The two sides are independent sources. The documented side is parsed
// out of §6's markdown table, which is hand-written prose; the actual
// side is the exit status of a real subprocess, decided by
// `ToDiagnostics::exit_code` in Rust. Editing the table moves the first
// and not the second; editing an `exit_code` impl moves the second and
// not the first.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Lower bound on rows parsed out of §6. The table is found by shape;
/// a heading rename or a reformat that stopped matching would empty the
/// map and leave every comparison below vacuously true.
const MIN_TABLE_ROWS: usize = 14;

/// Lower bound on invocations actually executed.
const MIN_PROBES: usize = 24;

/// Rows of §6 this file does not exercise, each with the reason.
///
/// Stated rather than omitted: a table row with no probe is a row that
/// can rot, and a silently-skipped one reads exactly like a covered one.
/// The partition test below fails if this list and the probed set stop
/// covering the table exactly, so adding a row forces a decision here.
const ROWS_NOT_PROBED_HERE: &[(&str, &str)] = &[
    (
        "5",
        "`import/*` needs a document set whose imports resolve to a \
         missing sibling; `cli_check_cross_doc` owns that fixture shape",
    ),
    (
        "6",
        "`manifest/*` is raised by the forge dependency-manifest walker, \
         driven by `forge_conformance`",
    ),
    (
        "8",
        "`io/filesystem` is a forge-internal I/O failure — reaching it \
         needs an unreadable path mid-pipeline, not a CLI argument",
    ),
    (
        "10",
        "`mesh/deploy-*` needs a deploy declaration; `mesh_error_format_json` \
         drives the mesh families",
    ),
    ("11", "`mesh/topology-*` — same fixture family as 10"),
    ("12", "`mesh/codegen-*` — same fixture family as 10"),
    ("13", "`mesh/io` — same fixture family as 10"),
    ("14", "`mesh/external-*` — same fixture family as 10"),
];

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// §6's table, as the document spells it: exit status -> the code
/// patterns that status is assigned to.
///
/// Parsed from the markdown rather than from any Rust constant. A
/// pattern is any backticked token holding a `/`; `0` has none, which is
/// how success is told apart from a row whose patterns failed to parse.
fn documented_table() -> BTreeMap<i32, Vec<String>> {
    let text = std::fs::read_to_string(repo_root().join("SCE_ERROR_CONTRACT.md"))
        .expect("read SCE_ERROR_CONTRACT.md");
    let mut in_section = false;
    let mut table = BTreeMap::new();
    for line in text.lines() {
        if line.starts_with("## ") {
            in_section = line.contains("Exit codes");
            continue;
        }
        if !in_section || !line.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = line.trim_matches('|').split('|').collect();
        if cells.len() < 2 {
            continue;
        }
        let status = cells[0].trim().trim_matches('`');
        let Ok(status) = status.parse::<i32>() else {
            continue;
        };
        let patterns: Vec<String> = backticked(cells[1])
            .into_iter()
            .filter(|t| t.contains('/'))
            .collect();
        table.insert(status, patterns);
    }
    table
}

/// Every backticked token in `s`.
fn backticked(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = s;
    while let Some(open) = rest.find('`') {
        rest = &rest[open + 1..];
        let Some(close) = rest.find('`') else { break };
        out.push(rest[..close].to_string());
        rest = &rest[close + 1..];
    }
    out
}

/// Whether `code` is named by `pattern`, which is either an exact code
/// or a `family/*` wildcard.
fn pattern_matches(pattern: &str, code: &str) -> bool {
    match pattern.strip_suffix("/*") {
        Some(family) => code.starts_with(family) && code[family.len()..].starts_with('/'),
        None => pattern == code,
    }
}

/// The status §6 assigns to `code`, applying the precedence the section
/// states: when an exact row and a `family/*` row both name a code, the
/// exact one wins. Without that rule the table is not a function and
/// `cli/query-no-match` — named by row `1` and covered by row `20`'s
/// `cli/*` — has two answers.
fn documented_status(table: &BTreeMap<i32, Vec<String>>, code: &str) -> Vec<i32> {
    let exact: Vec<i32> = table
        .iter()
        .filter(|(_, patterns)| patterns.iter().any(|p| p == code))
        .map(|(status, _)| *status)
        .collect();
    if !exact.is_empty() {
        return exact;
    }
    table
        .iter()
        .filter(|(_, patterns)| patterns.iter().any(|p| pattern_matches(p, code)))
        .map(|(status, _)| *status)
        .collect()
}

struct Outcome {
    status: i32,
    codes: Vec<String>,
    prose_lines: Vec<String>,
    stdout: Vec<u8>,
    /// Whether stderr carried an ESC byte. §7 forbids colour escapes in
    /// JSON mode outright, and the argument parser is the one component
    /// in this binary that styles its own output — so the path this
    /// file added is exactly the one that could reintroduce them.
    styled: bool,
}

fn probe(args: &[&str], stdin: Option<&str>) -> Outcome {
    let mut cmd = Command::new(sce_codegen_bin());
    cmd.arg("--error-format=json")
        .args(args)
        .current_dir(repo_root())
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn sce-codegen");
    if let Some(text) = stdin {
        child
            .stdin
            .as_mut()
            .expect("piped stdin")
            .write_all(text.as_bytes())
            .expect("write stdin");
    }
    let out = child.wait_with_output().expect("await sce-codegen");
    let mut codes = Vec::new();
    let mut prose_lines = Vec::new();
    for line in String::from_utf8_lossy(&out.stderr).lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(v) => match v.get("code").and_then(|c| c.as_str()) {
                Some(code) => codes.push(code.to_string()),
                None => prose_lines.push(line.to_string()),
            },
            Err(_) => prose_lines.push(line.to_string()),
        }
    }
    Outcome {
        status: out.status.code().expect("terminated by signal"),
        codes,
        prose_lines,
        styled: out.stderr.contains(&0x1b),
        stdout: out.stdout,
    }
}

/// A temporary tree carrying the fixtures the probes need, plus a real
/// sourcemap produced by the binary under test.
struct Fixtures {
    dir: PathBuf,
}

impl Fixtures {
    fn build() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "sce-exit-contract-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("gen")).expect("create fixture dir");
        std::fs::write(
            dir.join("valid.scxml"),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <scxml xmlns=\"http://www.w3.org/2005/07/scxml\" version=\"1.0\" \
             name=\"probe\" initial=\"s0\">\n\
             \x20 <state id=\"s0\"><transition event=\"go\" target=\"s1\"/></state>\n\
             \x20 <final id=\"s1\"/>\n\
             </scxml>\n",
        )
        .expect("write valid.scxml");
        std::fs::write(
            dir.join("malformed.scxml"),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <scxml xmlns=\"http://www.w3.org/2005/07/scxml\" version=\"1.0\" \
             name=\"bad\" initial=\"s0\">\n\
             \x20 <state id=\"s0\"><transition event=\"go\" target=\"s1\"\n\
             </scxml>\n",
        )
        .expect("write malformed.scxml");
        std::fs::write(
            dir.join("dangling.scxml"),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <scxml xmlns=\"http://www.w3.org/2005/07/scxml\" version=\"1.0\" \
             name=\"dangling\" initial=\"s0\">\n\
             \x20 <state id=\"s0\"><transition event=\"go\" target=\"nowhere\"/></state>\n\
             </scxml>\n",
        )
        .expect("write dangling.scxml");
        std::fs::write(dir.join("bad.json"), "not json at all\n").expect("write bad.json");

        // A typed native guard whose expression does not parse — the
        // only shape that reaches the `expression/*` family from a CLI
        // invocation. Derived from the tracked fixture rather than
        // written out here: a fresh copy would drift from whatever the
        // typed-guard pipeline demands of a document, and the point of
        // the probe is that the *rest* of the document is exactly what
        // the pipeline accepts.
        let guard_src = repo_root()
            .join("sce-build/tests/fixtures/event_schema/statechart_cross_state_guard.scxml");
        let guard_text = std::fs::read_to_string(&guard_src)
            .unwrap_or_else(|e| panic!("read {}: {e}", guard_src.display()));
        const INTACT: &str = r#"cond="_event.data.elapsed_ms === 0""#;
        const TRUNCATED: &str = r#"cond="_event.data.elapsed_ms ===""#;
        assert!(
            guard_text.contains(INTACT),
            "the typed-guard fixture no longer spells {INTACT}; the \
             expression probe below would silently become a valid document"
        );
        std::fs::write(
            dir.join("unparseable_guard.scxml"),
            guard_text
                .replace(INTACT, TRUNCATED)
                .replace("statechart_cross_state_guard", "unparseable_guard"),
        )
        .expect("write unparseable_guard.scxml");
        // The guard document imports its event schema by sibling path.
        let schema_src = guard_src
            .parent()
            .expect("fixture parent")
            .join("schema_job_completed_multi.scxml");
        std::fs::copy(&schema_src, dir.join("schema_job_completed_multi.scxml"))
            .unwrap_or_else(|e| panic!("copy {}: {e}", schema_src.display()));

        // A real sourcemap, so the query probes run against a
        // well-formed artifact rather than a missing one — a miss and a
        // missing file are the two outcomes this file must keep apart.
        let gen = Command::new(sce_codegen_bin())
            .args(["generate"])
            .arg(dir.join("valid.scxml"))
            .arg("-o")
            .arg(dir.join("gen"))
            .args(["-l", "rust"])
            .current_dir(repo_root())
            .output()
            .expect("generate the sourcemap fixture");
        assert!(
            gen.status.success(),
            "fixture generation failed:\n{}",
            String::from_utf8_lossy(&gen.stderr)
        );
        assert!(
            dir.join("gen/sce_sourcemap.json").exists(),
            "fixture generation produced no sourcemap under {}",
            dir.join("gen").display()
        );
        Fixtures { dir }
    }

    fn path(&self, rel: &str) -> String {
        self.dir.join(rel).display().to_string()
    }
}

impl Drop for Fixtures {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// (label, argv, stdin) for every failure this file drives.
///
/// Spread across subcommands on purpose: the defect this file was
/// written for was per-call-site, so a probe set concentrated on one
/// route would have found one of eight.
fn probes(fx: &Fixtures) -> Vec<(String, Vec<String>, Option<String>)> {
    let map = fx.path("gen");
    let bin = sce_codegen_bin().display().to_string();
    let mut out: Vec<(String, Vec<String>, Option<String>)> = Vec::new();
    let mut add = |label: &str, argv: Vec<String>, stdin: Option<&str>| {
        out.push((label.to_string(), argv, stdin.map(str::to_string)));
    };
    let s = |v: &str| v.to_string();

    // ── the argument parser's own failures ──
    add("usage/missing-positional", vec![s("generate")], None);
    add(
        "usage/unknown-flag",
        vec![
            s("generate"),
            fx.path("valid.scxml"),
            s("-o"),
            fx.path("gen"),
            s("-l"),
            s("rust"),
            s("--frobnicate"),
        ],
        None,
    );
    add(
        "usage/value-outside-enumeration",
        vec![
            s("list-fixtures"),
            s("--manifest"),
            fx.path("bad.json"),
            s("--catalog"),
            s("nope"),
        ],
        None,
    );
    add("usage/no-subcommand", vec![], None);
    add(
        "usage/two-exclusive-modes",
        vec![
            s("addr2sce"),
            map.clone(),
            s("--symbol"),
            s("x"),
            s("--hardfault"),
        ],
        None,
    );
    add(
        "usage/no-mode-at-all",
        vec![s("addr2sce"), map.clone()],
        None,
    );
    add(
        "usage/flag-without-its-companion",
        vec![s("addr2sce"), map.clone(), s("--pc"), s("0x1000")],
        None,
    );
    add(
        "usage/unparseable-value",
        vec![
            s("addr2sce"),
            map.clone(),
            s("--pc"),
            s("zzzz"),
            s("--elf"),
            bin.clone(),
        ],
        None,
    );

    // ── queries that ran and matched nothing ──
    add(
        "query/symbol-absent",
        vec![
            s("addr2sce"),
            map.clone(),
            s("--symbol"),
            s("no_such_symbol"),
        ],
        None,
    );
    add(
        "query/sce2sym-no-match",
        vec![
            s("sce2sym"),
            map.clone(),
            s("--state"),
            s("definitely_not_a_state"),
        ],
        None,
    );

    // ── xml / validation, the two pipeline families a CLI probe reaches ──
    for route in ["generate", "check"] {
        let mut argv = vec![s(route), fx.path("malformed.scxml"), s("-l"), s("rust")];
        if route == "generate" {
            argv.extend([s("-o"), fx.path("gen")]);
        }
        add(&format!("xml/{route}-malformed"), argv, None);
        let mut argv = vec![s(route), fx.path("dangling.scxml"), s("-l"), s("rust")];
        if route == "generate" {
            argv.extend([s("-o"), fx.path("gen")]);
        }
        add(&format!("validation/{route}-dangling"), argv, None);
    }
    // ── the typed-expression pipeline ──
    add(
        "expression/unparseable-guard",
        vec![
            s("check"),
            fx.path("unparseable_guard.scxml"),
            s("-l"),
            s("rust"),
        ],
        None,
    );
    // ── a backend that does not lower a construct the document uses ──
    add(
        "generate/backend-cannot-lower",
        vec![
            s("check"),
            repo_root()
                .join("sce-build/tests/fixtures/event_schema/statechart_native_action.scxml")
                .display()
                .to_string(),
            s("-l"),
            s("cpp"),
        ],
        None,
    );
    add(
        "xml/requirements-malformed",
        vec![s("requirements"), fx.path("malformed.scxml")],
        None,
    );
    add(
        "xml/unresolved-malformed",
        vec![s("unresolved"), fx.path("malformed.scxml")],
        None,
    );

    // ── CLI-boundary failures across routes ──
    add(
        "cli/generate-missing-input",
        vec![
            s("generate"),
            fx.path("nope.scxml"),
            s("-o"),
            fx.path("gen"),
            s("-l"),
            s("rust"),
        ],
        None,
    );
    add(
        "cli/generate-unknown-language",
        vec![
            s("generate"),
            fx.path("valid.scxml"),
            s("-o"),
            fx.path("gen"),
            s("-l"),
            s("klingon"),
        ],
        None,
    );
    add(
        "cli/generate-w3c-unsupported-language",
        vec![s("generate-w3c"), s("-l"), s("c11"), s("--list")],
        None,
    );
    add(
        "cli/expand-missing-input",
        vec![s("expand"), fx.path("nope.scxml")],
        None,
    );
    add(
        "cli/read-metadata-missing-input",
        vec![s("read-metadata"), fx.path("nope.txt")],
        None,
    );
    add(
        "cli/manifest-not-a-directory",
        vec![s("manifest"), fx.path("nope")],
        None,
    );
    add(
        "cli/addr2sce-missing-sourcemap",
        vec![s("addr2sce"), fx.path("nope"), s("--symbol"), s("x")],
        None,
    );
    add(
        "cli/addr2sce-image-without-symbols",
        vec![
            s("addr2sce"),
            map.clone(),
            s("--pc"),
            s("0x1000"),
            s("--elf"),
            s("/dev/null"),
        ],
        None,
    );
    add(
        "cli/hardfault-empty-stdin",
        vec![
            s("addr2sce"),
            map.clone(),
            s("--hardfault"),
            s("--elf"),
            bin.clone(),
        ],
        Some(""),
    );
    add(
        "cli/hardfault-unparseable-line",
        vec![
            s("addr2sce"),
            map.clone(),
            s("--hardfault"),
            s("--elf"),
            bin.clone(),
        ],
        Some("zzzz\n"),
    );
    add(
        "cli/generate-conformance-bad-manifest",
        vec![
            s("generate-conformance"),
            s("--manifest"),
            fx.path("bad.json"),
            s("--output-dir"),
            fx.path("gen"),
            s("-l"),
            s("rust"),
        ],
        None,
    );
    add(
        "cli/generate-integration-unknown-stem",
        vec![
            s("generate-integration"),
            s("-l"),
            s("rust"),
            s("--stem"),
            s("no_such_stem"),
        ],
        None,
    );
    out
}

/// §6's own universal: a non-zero exit always carries a record, always
/// with a status the table assigns, and never with anything on stdout.
#[test]
fn every_non_zero_exit_carries_a_record_the_table_assigns() {
    let fx = Fixtures::build();
    let table = documented_table();
    let cases = probes(&fx);
    assert!(
        cases.len() >= MIN_PROBES,
        "probe list shrank to {} — below the {MIN_PROBES} floor",
        cases.len()
    );

    let mut failures: Vec<String> = Vec::new();
    for (label, argv, stdin) in &cases {
        let argv: Vec<&str> = argv.iter().map(String::as_str).collect();
        let got = probe(&argv, stdin.as_deref());
        if got.status == 0 {
            failures.push(format!("[{label}] expected a failure, exited 0"));
            continue;
        }
        if !got.prose_lines.is_empty() {
            failures.push(format!(
                "[{label}] exit {} wrote non-record lines to stderr under \
                 --error-format=json: {:?}",
                got.status, got.prose_lines
            ));
        }
        if got.styled {
            failures.push(format!(
                "[{label}] stderr carried an ANSI escape; §7 forbids them in \
                 JSON mode"
            ));
        }
        let Some(code) = got.codes.first() else {
            failures.push(format!(
                "[{label}] exit {} carried no NDJSON record — \
                 SCE_ERROR_CONTRACT.md §6 names that a contract violation",
                got.status
            ));
            continue;
        };
        if !got.stdout.is_empty() {
            failures.push(format!(
                "[{label}] exit {} wrote {} byte(s) to stdout; §10.2 requires \
                 stdout to be empty on failure",
                got.status,
                got.stdout.len()
            ));
        }
        if !table.contains_key(&got.status) {
            failures.push(format!(
                "[{label}] exited {} for `{code}`, a status §6's table does not define",
                got.status
            ));
            continue;
        }
        let owners = documented_status(&table, code);
        if owners != vec![got.status] {
            failures.push(format!(
                "[{label}] `{code}` exited {} — §6 assigns it {owners:?}",
                got.status
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "exit-status contract violated by {} of {} probes:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}

/// The other direction: every code a probe produced must be one §6
/// assigns *somewhere*, and the status it left with must be the row that
/// names it — not merely some row.
#[test]
fn no_probe_produces_a_code_the_table_does_not_place() {
    let fx = Fixtures::build();
    let table = documented_table();
    let mut misplaced = Vec::new();
    for (label, argv, stdin) in probes(&fx) {
        let argv: Vec<&str> = argv.iter().map(String::as_str).collect();
        let got = probe(&argv, stdin.as_deref());
        for code in &got.codes {
            let owners = documented_status(&table, code);
            match owners.as_slice() {
                [] => misplaced.push(format!("[{label}] `{code}` appears in no §6 row")),
                [single] if *single == got.status => {}
                _ => misplaced.push(format!(
                    "[{label}] `{code}` exited {} but §6 places it at {owners:?}",
                    got.status
                )),
            }
        }
    }
    assert!(
        misplaced.is_empty(),
        "codes placed inconsistently with §6:\n{}",
        misplaced.join("\n")
    );
}

/// The table this file judges against must have been read. A heading
/// rename or a reformat that emptied the parse would make every
/// comparison above pass without comparing anything.
#[test]
fn the_documented_table_parses() {
    let table = documented_table();
    assert!(
        table.len() >= MIN_TABLE_ROWS,
        "§6's table parsed to {} row(s), below the {MIN_TABLE_ROWS} floor — \
         the section heading or the table shape changed",
        table.len()
    );
    assert_eq!(
        table.get(&0).map(Vec::len),
        Some(0),
        "row `0` is success and names no code pattern"
    );
    for (status, patterns) in &table {
        if *status == 0 {
            continue;
        }
        assert!(
            !patterns.is_empty(),
            "row `{status}` names no code pattern — a row a consumer \
             cannot route on"
        );
    }
}

/// Every row of §6 is either exercised here or listed as not exercised,
/// with a reason. A row in neither set is a row nothing checks.
#[test]
fn probed_and_declared_rows_partition_the_table() {
    let fx = Fixtures::build();
    let table = documented_table();
    let mut probed: BTreeSet<i32> = BTreeSet::new();
    for (_, argv, stdin) in probes(&fx) {
        let argv: Vec<&str> = argv.iter().map(String::as_str).collect();
        let got = probe(&argv, stdin.as_deref());
        if got.status != 0 {
            probed.insert(got.status);
        }
    }
    // Success is reached by every passing run in the suite; it needs no
    // failure probe here.
    probed.insert(0);

    let declared: BTreeSet<i32> = ROWS_NOT_PROBED_HERE
        .iter()
        .map(|(status, _)| status.parse().expect("declared row is a number"))
        .collect();

    let overlap: Vec<i32> = probed.intersection(&declared).copied().collect();
    assert!(
        overlap.is_empty(),
        "rows {overlap:?} are declared unprobed yet a probe reached them — \
         drop them from ROWS_NOT_PROBED_HERE"
    );

    let covered: BTreeSet<i32> = probed.union(&declared).copied().collect();
    let documented: BTreeSet<i32> = table.keys().copied().collect();
    let uncovered: Vec<i32> = documented.difference(&covered).copied().collect();
    assert!(
        uncovered.is_empty(),
        "§6 rows {uncovered:?} are neither probed here nor declared unprobed — \
         add a probe or state why there is none"
    );

    let stale: Vec<i32> = declared.difference(&documented).copied().collect();
    assert!(
        stale.is_empty(),
        "ROWS_NOT_PROBED_HERE names {stale:?}, which §6 no longer documents"
    );
}

/// `--help` and `--version` are requests that succeeded, not failures.
///
/// Pinned because the repair for the usage defect runs inside the
/// parser's error path, and clap reports both of these through that same
/// path: routing them like a failure would put help on stderr under a
/// `cli/usage` record and exit 20.
#[test]
fn help_and_version_stay_successful_output_on_stdout() {
    for argv in [
        vec!["--help"],
        vec!["--version"],
        vec!["generate", "--help"],
        vec!["addr2sce", "--help"],
    ] {
        let out = Command::new(sce_codegen_bin())
            .args(&argv)
            .current_dir(repo_root())
            .output()
            .expect("run sce-codegen");
        assert_eq!(
            out.status.code(),
            Some(0),
            "`{argv:?}` must exit 0; stderr:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            !out.stdout.is_empty(),
            "`{argv:?}` must write its output to stdout"
        );
        assert!(
            out.stderr.is_empty(),
            "`{argv:?}` must write nothing to stderr, got:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
