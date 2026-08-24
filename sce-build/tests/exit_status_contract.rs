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
///
/// A reason has to be a fact about coverage, not about fixtures. Six of
/// these once delegated to a gate on the strength of that gate owning
/// the family's *documents* — and owning a document is not asserting a
/// status. Measured across the whole test tree, `mesh_error_format_json`
/// is the only gate that asserts a mesh exit status at all, and it
/// asserts two: 10 and 11. Rows 5, 6 and 12 were asserted nowhere, by
/// anyone, and each turned out to be one invocation away; they are
/// probed above now. What stays here is what a probe was actually run
/// for and did not reach.
const ROWS_NOT_PROBED_HERE: &[(&str, &str)] = &[
    (
        "8",
        "`io/filesystem` is raised inside the forge pipeline, past the \
         point a CLI argument can steer. Made a forge input unreadable \
         mid-pipeline and the §synth-6.2.6 source-hash walk refuses \
         first, with `cli/read-input` — the same refusal the row-13 \
         probe below hits, and for the same reason",
    ),
    (
        "10",
        "`mesh/deploy-*` is asserted by `mesh_error_format_json`, which \
         drives `generate --deploy` and pins `status.code() == Some(10)` \
         on two fixtures — a real assertion on this row, unlike the \
         reasons that used to sit beside it",
    ),
    (
        "11",
        "`mesh/topology-*` — same gate, `Some(11)` on \
         `topology_machine_not_found_is_ndjson`",
    ),
    (
        "13",
        "`mesh/io` is raised when the partition walker re-parses a \
         machine's SCXML. Probed by making a declared machine source \
         unreadable: the drift walk refuses first (`cli/read-input`, \
         exit 20), so the mesh stage is never entered. Reaching it \
         needs a source readable at hash time and not at partition \
         time — a race, not an invocation",
    ),
    (
        "14",
        "`mesh/external-*` needs a deploy naming an external OEM config \
         (vsomeip.json / zenoh.json5) under a transport binding. Probed \
         with a hand-written device-level `transport:` key and the \
         deploy schema refused it as an unknown field (`mesh/deploy-parse`, \
         exit 10) — the fixture has to come from the binding shape \
         `mesh_error_format_json` already stages, not from this file",
    ),
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
/// or a prefix ending in `*`.
///
/// The wildcard has to be a prefix rule rather than a `family/*` one.
/// §6 spells five of its rows as `mesh/deploy-*`, `mesh/topology-*`,
/// `mesh/codegen-*` and `mesh/external-*` — the split is inside the
/// family, past the slash — and a matcher that only understood `/*`
/// matched none of them. That went unseen for as long as those rows
/// went unprobed: with no code reaching the comparison, a rule that
/// could never fire and a rule that never had to are the same green.
/// The first probe aimed at row 12 reported its code as belonging to
/// no row at all.
fn pattern_matches(pattern: &str, code: &str) -> bool {
    match pattern.strip_suffix('*') {
        // At least one character past the prefix, so `cli/*` names the
        // codes under `cli/` and not the empty stem itself.
        Some(prefix) => code.len() > prefix.len() && code.starts_with(prefix),
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
        // One line, no newline anywhere — the shape whose `expand`
        // output stays entirely inside stdout's line buffer. Every
        // other document flushes on its own newlines, so a write that
        // fails does so during `write_all`; this one only fails when
        // something asks for the flush. Without it in the sweep, the
        // buffered-tail half of the stdout contract is unprobed.
        std::fs::write(
            dir.join("single_line.scxml"),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
             <scxml xmlns=\"http://www.w3.org/2005/07/scxml\" version=\"1.0\" \
             name=\"single_line\" initial=\"s0\">\
             <state id=\"s0\"><transition event=\"go\" target=\"s1\"/></state>\
             <final id=\"s1\"/></scxml>",
        )
        .expect("write single_line.scxml");

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
        // A forge document whose `<sce:import src>` resolves to
        // nothing — the `import/*` family, which reaches its refusal
        // before any backend is consulted.
        std::fs::write(
            dir.join("missing_import.scxml"),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <scxml xmlns=\"http://www.w3.org/2005/07/scxml\" \
             xmlns:sce=\"http://sce.dev/ext\" \
             sce:kind=\"link\" name=\"missing_import\" version=\"1.0\">\n\
             \x20 <sce:import as=\"peer\" src=\"no_such_sibling.scxml\" kind=\"codec\"/>\n\
             \x20 <sce:link-class>udp</sce:link-class>\n\
             \x20 <sce:framer ref=\"peer\"/>\n\
             \x20 <sce:backpressure>drop</sce:backpressure>\n\
             </scxml>\n",
        )
        .expect("write missing_import.scxml");

        // Two codecs importing each other, in a directory of their
        // own: the `manifest` subcommand walks a whole directory, so
        // the cycle has to be the only thing in it.
        std::fs::create_dir_all(dir.join("cycle")).expect("create cycle dir");
        for (name, other) in [("a", "b"), ("b", "a")] {
            std::fs::write(
                dir.join("cycle").join(format!("{name}.scxml")),
                format!(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                     <scxml xmlns=\"http://www.w3.org/2005/07/scxml\" \
                     xmlns:sce=\"http://sce.dev/ext\" \
                     sce:kind=\"codec\" sce:default-endian=\"little\" name=\"{name}\">\n\
                     \x20 <sce:import as=\"{other}\" src=\"{other}.scxml\" kind=\"codec\"/>\n\
                     \x20 <datamodel/>\n\
                     </scxml>\n"
                ),
            )
            .expect("write cycle document");
        }

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
    //
    // The discriminator is a native `cond="cpp:…"`, which names the language
    // it is written in and so can only ever be lowered by that one backend.
    // It used to be `<sce:action>` on `-l cpp`, and that stopped discriminating
    // the day every backend grew a native-action path (2026-08-24) — a test
    // built on a gap retires itself when the gap closes, while still reading as
    // a pass. This one cannot close the same way: a `cpp:` guard is C++ source,
    // so "the other backends refuse it" is a property of the construct rather
    // than of how much has been written yet.
    add(
        "generate/backend-cannot-lower",
        vec![
            s("check"),
            repo_root()
                .join("examples/smart_light/smart_light.scxml")
                .display()
                .to_string(),
            s("-l"),
            s("rust"),
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

    // ── stage families reached past the CLI boundary ──
    //
    // These three were delegated to other gates on the reading that
    // owning a family's *fixtures* is owning its exit status. It is
    // not: `cli_check_cross_doc` asserts only that its refusals are
    // non-zero, and `forge_conformance` asserts no status at all. The
    // rows were unprobed everywhere, which is the state the partition
    // assertion below exists to make visible — so they are probed here
    // instead, each by the shortest invocation that reaches it.
    add(
        "import/file-not-found",
        vec![
            s("check"),
            fx.path("missing_import.scxml"),
            s("-l"),
            s("rust"),
        ],
        None,
    );
    add(
        "manifest/circular-dependency",
        vec![s("manifest"), fx.path("cycle")],
        None,
    );
    // `mesh/codegen-*` needs a document the mesh pipeline compiles,
    // which is a two-machine topology rather than anything that can be
    // written inline — so it comes from the tracked fixture. C++ is
    // the one backend mesh codegen emits for; every other `--language`
    // reaches the refusal.
    let mesh = repo_root().join("sce-build/tests/fixtures/author_literal_mesh");
    add(
        "mesh/codegen-unsupported-language",
        vec![
            s("generate"),
            mesh.join("mesh_parent.scxml").display().to_string(),
            s("-l"),
            s("rust"),
            s("-o"),
            fx.path("gen"),
            s("--deploy"),
            mesh.join("deploy.yaml").display().to_string(),
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

/// Lower bound on invocations driven with their reader already gone.
const MIN_CLOSED_READER_PROBES: usize = 8;

/// Lower bound on those that actually reached the failing write.
///
/// A probe whose subcommand happens to print nothing exits 0 and
/// proves nothing about the write path. Without this the sweep could
/// go quiet — every probe silently producing no output — and read
/// exactly like a sweep that found the contract kept.
///
/// It is a floor under the sweep, not the per-probe rule: a single
/// writer falling silent stays above any floor worth setting. Which
/// probes must report is decided per probe, by running each one twice
/// — see [`a_reader_that_stops_reading_is_not_a_way_out_of_the_table`].
const MIN_CLOSED_READER_REFUSALS: usize = 5;

/// A reader that stops reading is not a way out of §6.
///
/// The status table is stated as the whole set — "a status outside it
/// is a defect, not an undocumented convention" — and that has to hold
/// for endings this process does not choose. A consumer closing the
/// pipe is the everyday one: `| head`, `| grep -q`, a build system
/// that has what it needs. `list-fixtures` exists *for* those
/// consumers; its help tells build systems to read it without a JSON
/// parser.
///
/// `println!` panics when the write fails, so that invocation exited
/// **101** with a panic message and no NDJSON record — a status §6
/// never mentions, reached by a condition the consumer chose. It went
/// unnoticed because it was also a race: the panic needed the writer
/// to still have bytes left when the reader vanished, so
/// `list-fixtures … | head` failed about half the time and every
/// shorter-spoken subcommand simply never lost the race. Closing the
/// reader *before* the child writes removes the race from the probe:
/// the first write fails, whatever its size.
///
/// The three subcommands that already handled the failure — `expand`,
/// `requirements`, `unresolved` — are probed alongside the rest rather
/// than trusted, because they are the sibling evidence for what the
/// rule is, and a rule kept in three places out of ten is the shape
/// this whole file exists to refuse.
#[test]
fn a_reader_that_stops_reading_is_not_a_way_out_of_the_table() {
    let fx = Fixtures::build();
    let table = documented_table();
    let documented: BTreeSet<i32> = table.keys().copied().collect();

    let w3c_catalog = repo_root().join("tests/w3c/conformance/fixtures.json");
    let forge_catalog = repo_root().join("tests/forge/conformance/fixtures.json");
    let argvs: Vec<Vec<String>> = vec![
        vec![
            "list-fixtures".into(),
            "--manifest".into(),
            w3c_catalog.display().to_string(),
            "--catalog".into(),
            "w3c".into(),
        ],
        vec![
            "list-fixtures".into(),
            "--manifest".into(),
            forge_catalog.display().to_string(),
            "--catalog".into(),
            "forge".into(),
        ],
        vec![
            "list-fixtures".into(),
            "--manifest".into(),
            forge_catalog.display().to_string(),
            "--catalog".into(),
            "forge".into(),
            "--format".into(),
            "cmake".into(),
        ],
        vec!["expand".into(), fx.path("valid.scxml")],
        // Same subcommand, output with no newline in it — see
        // `single_line.scxml`. The two are one probe apart and fail
        // through different halves of the writer.
        vec!["expand".into(), fx.path("single_line.scxml")],
        vec!["check".into(), fx.path("valid.scxml")],
        vec![
            "check".into(),
            fx.path("valid.scxml"),
            "-l".into(),
            "rust".into(),
        ],
        vec![
            "generate".into(),
            fx.path("valid.scxml"),
            "-l".into(),
            "rust".into(),
            "-o".into(),
            fx.path("gen"),
        ],
        vec!["requirements".into(), fx.path("valid.scxml")],
        vec!["unresolved".into(), fx.path("valid.scxml")],
        vec![
            "sce2sym".into(),
            "--sourcemap".into(),
            fx.path("gen/sce_sourcemap.json"),
        ],
    ];

    let mut violations: Vec<String> = Vec::new();
    let mut reached = 0usize;
    for argv in &argvs {
        // Which probes *must* report is decided here rather than
        // listed: run the invocation with a reader that stays, and
        // whatever it wrote is what a vanished reader would have made
        // it fail on. A subcommand that prints nothing (`requirements`
        // against a document carrying no annotations) has no write to
        // fail and is held to nothing; every other one must report.
        //
        // Deciding it by list instead would rot the moment a
        // subcommand's output became conditional, and deciding it in
        // aggregate — "at least N probes reported" — cannot see one
        // writer falling silent, which is the shape a stdout helper
        // regresses in.
        let control = Command::new(sce_codegen_bin())
            .arg("--error-format=json")
            .args(argv)
            .current_dir(repo_root())
            .stdin(Stdio::null())
            .output()
            .expect("run sce-codegen with its reader intact");
        let wrote_something = !control.stdout.is_empty();

        let mut child = Command::new(sce_codegen_bin())
            .arg("--error-format=json")
            .args(argv)
            .current_dir(repo_root())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn sce-codegen");
        // Close the read end before the child writes. Everything after
        // this is the child answering a question it cannot avoid.
        drop(child.stdout.take().expect("piped stdout"));
        let out = child.wait_with_output().expect("await sce-codegen");
        let status = out.status.code().expect("terminated by signal");
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        let codes: Vec<&str> = stderr
            .lines()
            .filter_map(|l| serde_json::from_str::<serde_json::Value>(l.trim()).ok())
            .filter_map(|v| v.get("code").and_then(|c| c.as_str()).map(str::to_string))
            .map(|s| Box::leak(s.into_boxed_str()) as &str)
            .collect();

        if status != 0 {
            reached += 1;
        }
        if wrote_something && status == 0 {
            violations.push(format!(
                "`{argv:?}` writes {} bytes to stdout, yet exited 0 with its reader \
                 gone — the failed write went unreported and the caller cannot tell \
                 truncated output from complete output",
                control.stdout.len(),
            ));
        }
        if stderr.contains("panicked at") {
            violations.push(format!(
                "`{argv:?}` panicked instead of reporting: exit {status}\n{stderr}"
            ));
            continue;
        }
        if status != 0 && !documented.contains(&status) {
            violations.push(format!(
                "`{argv:?}` exited {status}, which §6 does not document (rows {documented:?})"
            ));
        }
        if status != 0 && codes.is_empty() {
            violations.push(format!(
                "`{argv:?}` exited {status} with no NDJSON record; §6: \"A non-zero \
                 exit with no NDJSON record is a contract violation\". stderr:\n{stderr}"
            ));
        }
        for code in &codes {
            let allowed = documented_status(&table, code);
            if !allowed.is_empty() && !allowed.contains(&status) {
                violations.push(format!(
                    "`{argv:?}` reported `{code}` but exited {status}; §6 assigns {allowed:?}"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "{} of {} closed-reader probes broke the contract:\n  {}",
        violations.len(),
        argvs.len(),
        violations.join("\n  "),
    );
    assert!(
        argvs.len() >= MIN_CLOSED_READER_PROBES,
        "ran only {} closed-reader probes; expected at least {MIN_CLOSED_READER_PROBES}",
        argvs.len(),
    );
    assert!(
        reached >= MIN_CLOSED_READER_REFUSALS,
        "only {reached} of {} probes reached the failing write; the rest exited 0 \
         because they printed nothing, so this sweep would pass against a binary \
         that panics on every write it makes",
        argvs.len(),
    );
}
