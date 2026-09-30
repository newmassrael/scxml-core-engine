// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An event that carries no data is declared as such, not left out.
//!
//! # What was wrong
//!
//! A statechart that declares `sce:interface="closed"` is held to the
//! event-schemas it imports, and an event-schema had to declare at least one
//! field. So an event the specification says carries nothing — or does not
//! say — had no way in: an author facing it either invented a field, which
//! the specification did not state, or dropped the event from the interface
//! and lost the check. Measured 2026-09-30 on a draft of a request client
//! whose signals carry no data: every schema was refused with
//! `validation/empty-collection`, and the draft the model then handed on as
//! "passing" had been checked with the closed declaration removed.
//!
//! # What holds now
//!
//! A schema with no field says why, on the `<datamodel>` that would have held
//! them: `sce:payload="none"` when the specification says the event carries
//! no data, or `sce:unresolved` when it does not say. Silence is still
//! refused — an empty datamodel reads the same as a field the author forgot.
//!
//! The rest is the condition the change was accepted on: a schema with no
//! field is a build-time contract and nothing else. The machine generated for
//! an event it names is the machine generated with no schema at all, on every
//! backend, so a data-bearing delivery is treated as W3C SCXML treats it; and
//! no record, host-run request or result can be made of it, since no backend
//! emits a payload struct for it.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sce_build::forge::unpseudo;
use sce_build::DocumentLabel;

const NONE: &str = include_str!("fixtures/event_schema_payload_free/job_requested.scxml");
const OPEN: &str = include_str!("fixtures/event_schema_payload_free/job_cancelled.scxml");

const LANGUAGES: [&str; 6] = ["rust", "cpp", "kotlin", "go", "python", "c11"];

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn scratch(label: &str) -> PathBuf {
    let dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// A directory holding `files`, named as given.
fn lay(label: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = scratch(label);
    for (name, body) in files {
        std::fs::write(dir.join(name), body).expect("write fixture");
    }
    dir
}

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(sce_codegen_bin())
        .current_dir(dir)
        .args(["--error-format", "json"])
        .args(args)
        .output()
        .expect("run sce-codegen")
}

/// The first diagnostic a refused run wrote.
fn refusal(out: &Output) -> serde_json::Value {
    assert!(
        !out.status.success(),
        "the run was expected to be refused: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    let line = stderr.lines().next().expect("a refusal writes a record");
    serde_json::from_str(line).expect("the record is one JSON object")
}

/// The manifest an accepted run wrote, validated against the checked-in
/// schema first: an `open` list a consumer cannot read is no answer.
fn manifest(out: &Output) -> serde_json::Value {
    assert!(
        out.status.success(),
        "the run was expected to be accepted: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let manifest: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("the manifest is one JSON line");
    let schema: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repo_root().join("schemas/sce-manifest.v1.schema.json"))
            .expect("read manifest schema"),
    )
    .expect("manifest schema is JSON");
    let validator = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(&schema)
        .expect("manifest schema compiles");
    let violations: Vec<String> = match validator.validate(&manifest) {
        Ok(()) => Vec::new(),
        Err(errors) => errors.map(|e| e.to_string()).collect(),
    };
    assert!(violations.is_empty(), "{violations:?}\n{manifest}");
    manifest
}

/// A statechart that keeps to a closed interface declared by two
/// payload-free schemas. Written line for line like [`open_without_them`],
/// so the two differ in nothing a source position could tell apart.
const CLOSED_WITH_THEM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="job" initial="idle" datamodel="ecmascript"
       sce:interface="closed">
  <sce:import src="job_requested.scxml" kind="event-schema" as="Requested"/>
  <sce:import src="job_cancelled.scxml" kind="event-schema" as="Cancelled"/>
  <state id="idle">
    <transition event="job.requested" target="running"/>
  </state>
  <state id="running">
    <transition event="job.cancelled" target="idle"/>
  </state>
</scxml>
"#;

/// The same machine with neither import and no declaration — the schemaless
/// baseline every event-schema-free document has always been.
fn open_without_them() -> String {
    CLOSED_WITH_THEM
        .replace("\n       sce:interface=\"closed\">", "\n       >")
        .replace(
            "  <sce:import src=\"job_requested.scxml\" kind=\"event-schema\" as=\"Requested\"/>",
            "  <!-- no schema for job.requested -->",
        )
        .replace(
            "  <sce:import src=\"job_cancelled.scxml\" kind=\"event-schema\" as=\"Cancelled\"/>",
            "  <!-- no schema for job.cancelled -->",
        )
}

// ── The declaration ────────────────────────────────────────────────────

#[test]
fn a_schema_that_says_the_event_carries_no_data_is_accepted() {
    let dir = lay("pf-none", &[("job_requested.scxml", NONE)]);
    let out = run(&dir, &["check", "job_requested.scxml"]);
    let manifest = manifest(&out);
    assert_eq!(manifest["document_kind"]["name"], "event-schema");
    assert!(
        manifest.get("open").is_none() && manifest.get("unresolved").is_none(),
        "the specification settled it, so nothing is left open: {manifest}"
    );
}

#[test]
fn a_schema_that_leaves_the_payload_open_is_accepted_and_says_so() {
    let dir = lay("pf-open", &[("job_cancelled.scxml", OPEN)]);
    let manifest = manifest(&run(&dir, &["check", "job_cancelled.scxml"]));
    assert_eq!(manifest["unresolved"][0]["id"], "cancel-payload");
    assert_eq!(manifest["unresolved"][0]["kind"], "unresolved");
    assert_eq!(manifest["open"][0]["kind"], "question");

    // Accepted is not finished: the strict check refuses it until answered.
    let strict = refusal(&run(
        &dir,
        &["check", "--strict-unresolved", "job_cancelled.scxml"],
    ));
    assert_eq!(strict["code"], "validation/unresolved-placeholder");
    assert_eq!(strict["actual"], "cancel-payload");
}

/// The same schema as [`OPEN`] with the payload a choice the author made
/// rather than a question: `sce:assumed` where the fixture says
/// `sce:unresolved`.
fn assumed_payload() -> String {
    let assumed = OPEN
        .replace("sce:unresolved-reason", "sce:assumed-reason")
        .replace("sce:unresolved=", "sce:assumed=");
    assert_ne!(assumed, OPEN, "the fixture no longer says sce:unresolved");
    assumed
}

#[test]
fn a_payload_the_author_chose_is_an_assumed_value_and_does_not_block_the_strict_check() {
    let assumed = assumed_payload();
    let dir = lay("pf-assumed", &[("job_cancelled.scxml", &assumed)]);
    let answer = manifest(&run(&dir, &["check", "job_cancelled.scxml"]));
    assert_eq!(answer["unresolved"][0]["kind"], "assumed");
    assert_eq!(answer["open"][0]["kind"], "assumed");
    assert_eq!(answer["open"].as_array().expect("open").len(), 1);

    // A value the author chose is on the record and blocks nothing: that is
    // the whole difference between the two markers, here as everywhere.
    manifest(&run(
        &dir,
        &["check", "--strict-unresolved", "job_cancelled.scxml"],
    ));

    // The page writes it in the marker's own word and reads back the same.
    let parsed = sce_build::forge::parser::parse_forge_with_imports(
        &assumed,
        DocumentLabel {
            identifier: "job_cancelled",
            diagnostic_label: "job_cancelled",
        },
    )
    .expect("parses")
    .expect("a forge document");
    let page = sce_build::forge::pseudo::render(&parsed.document).expect("renders");
    assert!(
        page.contains("\n  assumed cancel-payload\n"),
        "the page has to say the payload was chosen, not asked: {page}"
    );
    let read_back = unpseudo::parse(&page).expect("reads back");
    assert_eq!(
        unpseudo::ir_for_comparison(&parsed.document).expect("serialises"),
        unpseudo::ir_for_comparison(&read_back).expect("serialises"),
    );
}

#[test]
fn silence_is_refused_and_the_refusal_names_both_ways_out() {
    let silent = NONE.replace(" sce:payload=\"none\"", "");
    let dir = lay("pf-silent", &[("job_requested.scxml", &silent)]);
    let record = refusal(&run(&dir, &["check", "job_requested.scxml"]));
    assert_eq!(record["code"], "validation/empty-collection");
    let message = record["message"].as_str().expect("a message");
    assert!(
        message.contains("sce:payload=\"none\"") && message.contains("sce:unresolved"),
        "the refusal has to say how to answer it: {message}"
    );
    // Placed on the datamodel, where the answer is written.
    let datamodel_line = silent
        .lines()
        .position(|l| l.contains("<datamodel"))
        .expect("a datamodel")
        + 1;
    assert_eq!(record["location"]["line"], datamodel_line);
}

#[test]
fn statements_that_contradict_each_other_are_refused() {
    let none = "<datamodel sce:payload=\"none\"/>";
    for (label, datamodel, code) in [
        (
            "a field beside none",
            "<datamodel sce:payload=\"none\"><data id=\"x\" sce:type=\"uint8\" \
             sce:direction=\"in\"/></datamodel>",
            "validation/incompatible-attributes",
        ),
        (
            "an open marker beside none",
            "<datamodel sce:payload=\"none\" sce:unresolved=\"p\" \
             sce:unresolved-reason=\"r\"/>",
            "validation/incompatible-attributes",
        ),
        (
            "an assumed marker beside none",
            "<datamodel sce:payload=\"none\" sce:assumed=\"p\" sce:assumed-reason=\"r\"/>",
            "validation/incompatible-attributes",
        ),
        (
            "a value that is not none",
            "<datamodel sce:payload=\"empty\"/>",
            "validation/invalid-attribute",
        ),
    ] {
        let document = NONE.replace(none, datamodel);
        assert_ne!(
            document, NONE,
            "{label}: the fixture no longer has the line"
        );
        let dir = lay("pf-contradiction", &[("job_requested.scxml", &document)]);
        let record = refusal(&run(&dir, &["check", "job_requested.scxml"]));
        assert_eq!(record["code"], code, "{label}: {record}");
    }
}

// ── The page ───────────────────────────────────────────────────────────

#[test]
fn the_page_says_which_it_is_and_reads_back_as_the_same_schema() {
    for (label, document, expected) in [
        (
            "job_requested",
            NONE,
            "event-schema job_requested event job.requested\n  payload none\n",
        ),
        (
            "job_cancelled",
            OPEN,
            "event-schema job_cancelled event job.cancelled\n  unresolved cancel-payload\n    \
             reason the specification does not say whether a cancellation names a reason\n",
        ),
    ] {
        let parsed = sce_build::forge::parser::parse_forge_with_imports(
            document,
            DocumentLabel {
                identifier: label,
                diagnostic_label: label,
            },
        )
        .expect("the fixture parses")
        .expect("it is a forge document");
        let page = sce_build::forge::pseudo::render(&parsed.document).expect("it renders");
        assert_eq!(page, expected, "{label}");

        // A page that showed only the head would read as an event that
        // carries nothing, whichever it is.
        let read_back = unpseudo::parse(&page).expect("the page reads back");
        assert_eq!(
            unpseudo::ir_for_comparison(&parsed.document).expect("a model serialises"),
            unpseudo::ir_for_comparison(&read_back).expect("a model serialises"),
            "{label}: the model moved across the round trip"
        );
    }
}

// ── A closed interface ─────────────────────────────────────────────────

#[test]
fn a_closed_statechart_may_declare_its_events_in_payload_free_schemas() {
    let dir = lay(
        "pf-closed",
        &[
            ("job_requested.scxml", NONE),
            ("job_cancelled.scxml", OPEN),
            ("job.scxml", CLOSED_WITH_THEM),
        ],
    );
    manifest(&run(&dir, &["check", "job.scxml"]));

    // The set checks every document as itself, on every backend, and the
    // question one member leaves open is the set's.
    let set = manifest(&run(
        &dir,
        &[
            "check",
            "--document",
            "job.scxml",
            "--document",
            "job_requested.scxml",
            "--document",
            "job_cancelled.scxml",
        ],
    ));
    assert_eq!(set["unresolved"][0]["id"], "cancel-payload");
    assert_eq!(
        set["unresolved"][0]["location"]["file"], "job_cancelled.scxml",
        "each record names the member it was written in: {set}"
    );
    assert_eq!(set["open"][0]["kind"], "question", "{set}");
    assert!(
        set.get("needs_parent").is_none() && set.get("document_kind").is_none(),
        "a parent and a kind stay one document's answer: {set}"
    );
}

/// [`CLOSED_WITH_THEM`] with the declaration taken off and every import left
/// where it was: the draft a check was got past by removing
/// `sce:interface="closed"`.
fn open_with_them() -> String {
    let open = CLOSED_WITH_THEM.replace("\n       sce:interface=\"closed\">", "\n       >");
    assert_ne!(open, CLOSED_WITH_THEM);
    open
}

#[test]
fn a_statechart_that_imports_schemas_and_is_not_closed_says_its_boundary_is_open() {
    let dir = lay(
        "pf-open-interface",
        &[
            ("job_requested.scxml", NONE),
            ("job_cancelled.scxml", OPEN),
            ("job.scxml", &open_with_them()),
            ("closed.scxml", CLOSED_WITH_THEM),
        ],
    );
    let interface_line = |answer: &serde_json::Value| {
        answer["open"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|m| m["kind"] == "interface")
            .map(|m| m["message"].as_str().expect("a message").to_string())
    };

    // One document, by `check` and by `generate`: the same sentence.
    let checked = manifest(&run(&dir, &["check", "job.scxml"]));
    let said = interface_line(&checked)
        .unwrap_or_else(|| panic!("an open boundary with schemas imported is said: {checked}"));
    assert!(
        said.contains("(Requested, Cancelled)") && said.contains("sce:interface=\"closed\""),
        "{said}"
    );
    let generated = manifest(&run(
        &dir,
        &[
            "generate",
            "job.scxml",
            "-l",
            "rust",
            "-o",
            dir.join("out").to_str().expect("a utf-8 path"),
        ],
    ));
    assert_eq!(interface_line(&generated), Some(said.clone()));

    // A set says it too: the imports are resolved there, and each member is
    // held to the same sentence.
    let set = manifest(&run(
        &dir,
        &[
            "check",
            "--document",
            "job.scxml",
            "--document",
            "job_requested.scxml",
            "--document",
            "job_cancelled.scxml",
        ],
    ));
    assert_eq!(interface_line(&set), Some(said));

    // Closed says nothing because it is closed.
    let closed = manifest(&run(&dir, &["check", "closed.scxml"]));
    assert_eq!(interface_line(&closed), None, "{closed}");
}

#[test]
fn a_statechart_with_no_schema_says_nothing_of_its_interface() {
    // The default interface is open and is not a finding: nothing tells the
    // product this statechart was meant to have a schema. Only the mismatch
    // is said.
    let dir = lay("pf-no-schema", &[("job.scxml", &open_without_them())]);
    let answer = manifest(&run(&dir, &["check", "job.scxml"]));
    assert!(answer.get("open").is_none(), "{answer}");
}

#[test]
fn the_strict_check_holds_every_member_of_a_set() {
    let dir = lay(
        "pf-strict-set",
        &[
            ("job_requested.scxml", NONE),
            ("job_cancelled.scxml", OPEN),
            ("job.scxml", CLOSED_WITH_THEM),
        ],
    );
    let files = [
        "--document",
        "job.scxml",
        "--document",
        "job_requested.scxml",
        "--document",
        "job_cancelled.scxml",
    ];

    // Without the flag the set is accepted, and says what it leaves open.
    manifest(&run(&dir, &[&["check"], &files[..]].concat()));

    // With it, the question one member leaves open is the set's, and the
    // refusal is where that member wrote it.
    let strict = refusal(&run(
        &dir,
        &[&["check", "--strict-unresolved"], &files[..]].concat(),
    ));
    assert_eq!(strict["code"], "validation/unresolved-placeholder");
    assert_eq!(strict["actual"], "cancel-payload");
    assert_eq!(
        strict["location"]["file"], "job_cancelled.scxml",
        "{strict}"
    );

    // Answered as a choice the author made, it is on the record and blocks
    // nothing, as on one document.
    std::fs::write(dir.join("job_cancelled.scxml"), assumed_payload()).expect("rewrite the schema");
    manifest(&run(
        &dir,
        &[&["check", "--strict-unresolved"], &files[..]].concat(),
    ));

    // A statechart of the set is held to it too.
    let asking = CLOSED_WITH_THEM.replace(
        "<transition event=\"job.cancelled\" target=\"idle\"/>",
        "<transition event=\"job.cancelled\" target=\"idle\" sce:unresolved=\"cancel-target\" \
         sce:unresolved-reason=\"the specification does not say where a cancellation goes\"/>",
    );
    assert_ne!(asking, CLOSED_WITH_THEM);
    std::fs::write(dir.join("job.scxml"), asking).expect("rewrite the statechart");
    let strict = refusal(&run(
        &dir,
        &[&["check", "--strict-unresolved"], &files[..]].concat(),
    ));
    assert_eq!(strict["actual"], "cancel-target", "{strict}");
    assert_eq!(strict["location"]["file"], "job.scxml", "{strict}");
}

#[test]
fn orchestrate_refuses_a_set_under_the_same_flags_and_writes_nothing() {
    // `check` predicts `orchestrate` on a set, so a refusal `check` gives
    // under a flag is one `orchestrate` can be asked for: the same functions,
    // before anything is generated.
    let dir = lay(
        "pf-orchestrate",
        &[
            ("job_requested.scxml", NONE),
            ("job_cancelled.scxml", OPEN),
            ("job.scxml", CLOSED_WITH_THEM),
        ],
    );
    let out_dir = dir.join("out");
    let orchestrate = |extra: &[&str]| {
        run(
            &dir,
            &[
                &[
                    "orchestrate",
                    "--document",
                    "job.scxml",
                    "--document",
                    "job_requested.scxml",
                    "--document",
                    "job_cancelled.scxml",
                    "-l",
                    "rust",
                    "-o",
                    out_dir.to_str().expect("a utf-8 path"),
                ][..],
                extra,
            ]
            .concat(),
        )
    };

    let strict = refusal(&orchestrate(&["--strict-unresolved"]));
    assert_eq!(strict["code"], "validation/unresolved-placeholder");
    assert_eq!(
        strict["location"]["file"], "job_cancelled.scxml",
        "{strict}"
    );
    assert!(
        !out_dir.exists()
            || std::fs::read_dir(&out_dir)
                .expect("the output directory reads")
                .next()
                .is_none(),
        "a refused design produced files"
    );

    // Without the flag the set builds and the question is published.
    let built = manifest(&orchestrate(&[]));
    assert_eq!(built["open"][0]["kind"], "question", "{built}");

    // `--lint` is accepted on a set that holds a statechart.
    manifest(&orchestrate(&["--lint"]));
}

/// Every distinct code a run wrote to stderr: a set is compiled for every
/// backend, and each one that refuses repeats the record.
fn codes(out: &Output) -> std::collections::BTreeSet<String> {
    String::from_utf8_lossy(&out.stderr)
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|record| record["code"].as_str().map(str::to_string))
        .collect()
}

#[test]
fn a_set_is_linted_and_the_flag_lists_every_finding() {
    // Two design-time defects in one open statechart: a state nothing can
    // enter, and a message the machine sends itself and discards.
    let defective = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="job" initial="idle" datamodel="ecmascript">
  <state id="idle">
    <onentry><send event="announce"/></onentry>
    <transition event="job.requested" target="idle"/>
  </state>
  <state id="never"/>
</scxml>
"#;
    let dir = lay(
        "pf-lint",
        &[("job_requested.scxml", NONE), ("job.scxml", defective)],
    );
    let files = [
        "--document",
        "job.scxml",
        "--document",
        "job_requested.scxml",
    ];

    // One document: the lints are opt-in, so it is accepted.
    manifest(&run(&dir, &["check", "job.scxml"]));

    // A set is linted whether or not it is asked to — the library entry
    // point each of its statecharts is compiled through refuses on the first
    // finding, and reachability is first.
    let plain = run(&dir, &[&["check"], &files[..]].concat());
    assert!(!plain.status.success());
    assert_eq!(
        codes(&plain),
        ["scxml/unreachable-state".to_string()].into(),
        "a set reports the first design-time finding"
    );

    // The flag makes it the answer one document gets under it: every finding.
    let linted = run(&dir, &[&["check", "--lint"], &files[..]].concat());
    assert!(!linted.status.success());
    assert_eq!(
        codes(&linted),
        [
            "scxml/self-send-discarded".to_string(),
            "scxml/unreachable-state".to_string()
        ]
        .into(),
        "under --lint a set reports every finding of the statechart"
    );
}

#[test]
fn a_lint_of_a_set_with_no_statechart_is_refused() {
    let dir = lay(
        "pf-lint-forge",
        &[("job_requested.scxml", NONE), ("job_cancelled.scxml", OPEN)],
    );
    let forge_only = run(
        &dir,
        &[
            "check",
            "--lint",
            "--document",
            "job_requested.scxml",
            "--document",
            "job_cancelled.scxml",
        ],
    );
    let record = refusal(&forge_only);
    assert_eq!(record["code"], "cli/usage");
    assert!(
        record["message"]
            .as_str()
            .expect("a message")
            .contains("holds none"),
        "{record}"
    );
}

#[test]
fn a_schema_the_interface_could_not_read_is_reported_in_its_own_words() {
    // The document imports a schema the parse refuses. Judged on regardless,
    // the interface said "the document imports no event-schema" — about a
    // document that imports one — and the cause was never told.
    let silent = NONE.replace(" sce:payload=\"none\"", "");
    let dir = lay(
        "pf-unread",
        &[
            ("job_requested.scxml", &silent),
            ("job_cancelled.scxml", OPEN),
            ("job.scxml", CLOSED_WITH_THEM),
        ],
    );
    let record = refusal(&run(&dir, &["check", "job.scxml"]));
    assert_eq!(record["code"], "validation/empty-collection", "{record}");
    assert_eq!(record["location"]["file"], "job_requested.scxml");
}

// ── What it must not become ────────────────────────────────────────────

#[test]
fn the_machine_for_a_payload_free_event_is_the_schemaless_machine_on_every_backend() {
    let with = lay(
        "pf-parity-with",
        &[
            ("job_requested.scxml", NONE),
            ("job_cancelled.scxml", OPEN),
            ("job.scxml", CLOSED_WITH_THEM),
        ],
    );
    let without = lay("pf-parity-without", &[("job.scxml", &open_without_them())]);

    for language in LANGUAGES {
        let mut generated = Vec::new();
        for (dir, label) in [(&with, "with"), (&without, "without")] {
            let out_dir = dir.join(format!("out-{language}"));
            let out = run(
                dir,
                &[
                    "generate",
                    "job.scxml",
                    "-l",
                    language,
                    "-o",
                    out_dir.to_str().expect("a utf-8 path"),
                ],
            );
            assert!(
                out.status.success(),
                "{language} ({label}): {}",
                String::from_utf8_lossy(&out.stderr)
            );
            let mut files: Vec<(String, String)> = std::fs::read_dir(&out_dir)
                .expect("the run wrote its output")
                .map(|entry| entry.expect("a directory entry").path())
                .filter(|path| path.is_file())
                .map(|path| {
                    let text = std::fs::read_to_string(&path).expect("a text artefact");
                    // The one thing that has to differ: the hash of the
                    // documents the machine was generated from.
                    let text: String = text
                        .lines()
                        .filter(|l| !l.contains("source-hash") && !l.contains("source_hash"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    (
                        path.file_name()
                            .expect("a file name")
                            .to_string_lossy()
                            .into_owned(),
                        text,
                    )
                })
                .collect();
            files.sort();
            generated.push(files);
        }
        assert!(
            generated[0]
                .iter()
                .any(|(_, text)| text.contains("JobRequested") || text.contains("job.requested")),
            "{language}: the machine has to name the event, or the comparison is vacuous"
        );
        assert_eq!(
            generated[0], generated[1],
            "{language}: a schema that declares no field changed the machine — a data-bearing \
             delivery would no longer be treated as W3C SCXML treats it"
        );
    }
}

#[test]
fn a_field_cannot_be_read_from_or_sent_with_an_event_that_declares_none() {
    let dir = lay(
        "pf-fields",
        &[
            (
                "schema_none.scxml",
                &NONE
                    .replace("job_requested", "schema_none")
                    .replace("job.requested", "tick.raised"),
            ),
            (
                "read.scxml",
                r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="ecmascript" name="read_it" initial="idle">
  <sce:import kind="event-schema" src="schema_none.scxml" as="Tick"/>
  <state id="idle">
    <transition event="tick.raised" cond="_event.data.count &gt; 0" target="idle"/>
  </state>
</scxml>
"#,
            ),
            (
                "send.scxml",
                r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="ecmascript" name="send_it" initial="idle">
  <sce:import kind="event-schema" src="schema_none.scxml" as="Tick"/>
  <state id="idle">
    <onentry>
      <send event="tick.raised" target="#_parent"><param name="count" expr="1"/></send>
    </onentry>
  </state>
</scxml>
"##,
            ),
        ],
    );
    assert_eq!(
        refusal(&run(&dir, &["check", "read.scxml"]))["code"],
        "validation/cross-kind-field-not-found"
    );
    assert_eq!(
        refusal(&run(&dir, &["check", "send.scxml"]))["code"],
        "validation/event-payload-field-unknown"
    );
}

#[test]
fn no_record_is_made_of_a_schema_that_declares_no_field() {
    let schema = NONE
        .replace("job_requested", "schema_none")
        .replace("job.requested", "tick.raised");
    let dir = lay(
        "pf-records",
        &[
            ("schema_none.scxml", &schema),
            (
                "static.scxml",
                r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="static_it">
  <sce:import kind="event-schema" src="schema_none.scxml" as="Tick"/>
  <datamodel>
    <data id="held" sce:type="record:Tick" sce:direction="out"/>
  </datamodel>
  <state id="idle"/>
</scxml>
"#,
            ),
            (
                "algorithm.scxml",
                r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" version="1.0">
  <sce:import kind="event-schema" src="schema_none.scxml" as="Tick"/>
  <sce:signature>
    <sce:param name="a" type="record:Tick"/>
    <sce:return type="uint8"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="1"/>
  </sce:body>
</scxml>
"#,
            ),
            (
                "invoke.scxml",
                r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="ecmascript" name="invoke_it" initial="asking">
  <sce:import kind="event-schema" src="schema_none.scxml" as="Ask"/>
  <state id="asking">
    <invoke type="x-sce-host" id="perm" sce:request="Ask"/>
  </state>
</scxml>
"#,
            ),
        ],
    );
    for (document, code) in [
        ("static.scxml", "validation/attribute-rule-violated"),
        ("algorithm.scxml", "validation/attribute-rule-violated"),
        ("invoke.scxml", "validation/typed-invoke-schema"),
    ] {
        let record = refusal(&run(&dir, &["check", document]));
        assert_eq!(record["code"], code, "{document}: {record}");
        assert!(
            record["message"]
                .as_str()
                .expect("a message")
                .contains("no field")
                || record["message"]
                    .as_str()
                    .expect("a message")
                    .contains("declares none"),
            "{document}: the refusal has to say why: {record}"
        );
    }
}

// ── The word around it ─────────────────────────────────────────────────

#[test]
fn a_comment_above_the_xml_declaration_is_refused_with_where_it_goes() {
    let dir = lay(
        "pf-declaration",
        &[(
            "c.scxml",
            "<!-- a comment above the declaration -->\n<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <scxml xmlns=\"http://www.w3.org/2005/07/scxml\" version=\"1.0\" name=\"c\" \
             initial=\"a\"><state id=\"a\"/></scxml>\n",
        )],
    );
    let record = refusal(&run(&dir, &["check", "c.scxml"]));
    assert_eq!(record["code"], "xml/parse");
    assert_eq!(record["location"]["line"], 2);
    let message = record["message"].as_str().expect("a message");
    assert!(
        message.contains("first thing in the file") && message.contains("move it below"),
        "the refusal named the fault and not the fix: {message}"
    );
}
