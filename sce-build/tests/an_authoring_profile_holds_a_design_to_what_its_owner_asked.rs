// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A design is held to what its owner asked for, stated in a file.
//!
//! # What was wrong
//!
//! A statechart that imports event-schemas and does not declare
//! `sce:interface="closed"` says nothing, and nothing could say more: a
//! statechart with no schema is a W3C conformance document, a legacy machine,
//! or a design about to be shown to an owner, and the product cannot tell
//! which. Measured 2026-09-30, a draft handed on as passing had its closed
//! declaration removed to get past a check, every import still in place. The
//! product reads intent off no feature of the document, so the intent has to
//! be stated: an authoring profile, kept beside the specification.
//!
//! # What holds now
//!
//! `--profile` on `check`, `generate`, `orchestrate`, `accept` and
//! `acceptance-check` reads it. Each behaviour is set beside a control that
//! differs in one thing, so a subcommand that refused everything, or judged
//! nothing, fails here as surely as one that did the other.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sce_build::acceptance_record::{AcceptanceRecord, SourceRole};
use sha2::{Digest, Sha256};

const CODEGEN: &str = env!("CARGO_BIN_EXE_sce-codegen");

const SCHEMA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="job_requested"
       sce:event-name="job.requested">
  <datamodel sce:payload="none"/>
</scxml>
"#;

/// A statechart that imports the schema, closed or open.
fn statechart(closed: bool) -> String {
    let declaration = if closed {
        " sce:interface=\"closed\""
    } else {
        ""
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="job" initial="idle" datamodel="ecmascript"{declaration}>
  <sce:import src="job_requested.scxml" kind="event-schema" as="Requested"/>
  <state id="idle">
    <transition event="job.requested" target="running"/>
  </state>
  <state id="running"/>
</scxml>
"#
    )
}

const PROFILE: &str =
    r#"{"record":"sce-authoring-profile","v":1,"name":"owner-review","interface":"closed"}"#;

/// The same profile with another label: different bytes, so a different
/// profile to an acceptance record.
const OTHER_PROFILE: &str =
    r#"{"record":"sce-authoring-profile","v":1,"name":"house-review","interface":"closed"}"#;

/// A profile that constrains nothing.
const EMPTY_PROFILE: &str = r#"{"record":"sce-authoring-profile","v":1}"#;

fn scratch(label: &str) -> PathBuf {
    let dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// A directory holding the schema, a profile and both statecharts.
fn design(label: &str) -> PathBuf {
    let dir = scratch(label);
    let put = |name: &str, body: &str| fs::write(dir.join(name), body).expect("write fixture");
    put("job_requested.scxml", SCHEMA);
    put("open.scxml", &statechart(false));
    put("closed.scxml", &statechart(true));
    put("profile.json", PROFILE);
    put("other-profile.json", OTHER_PROFILE);
    put("empty-profile.json", EMPTY_PROFILE);
    dir
}

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(CODEGEN)
        .arg("--error-format=json")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("spawn sce-codegen")
}

/// Every record a run wrote on stderr, in order.
fn records(out: &Output) -> Vec<serde_json::Value> {
    String::from_utf8_lossy(&out.stderr)
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

fn codes(out: &Output) -> Vec<String> {
    records(out)
        .iter()
        .filter_map(|v| v["code"].as_str().map(str::to_string))
        .collect()
}

/// The manifest an accepted run wrote, put through the checked-in schema
/// first: a `profile` a consumer cannot read is no answer.
fn manifest(out: &Output) -> serde_json::Value {
    assert!(
        out.status.success(),
        "the run was expected to be accepted: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let manifest: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("the manifest is one JSON line");
    let schema: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../schemas/sce-manifest.v1.schema.json"),
        )
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

fn sha256_of(path: &Path) -> String {
    let digest = Sha256::digest(fs::read(path).expect("read"));
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The refusal a profile the statechart departs from produces, on the route
/// `args` names.
fn assert_refused_for_the_profile(out: &Output, machine: &str) {
    assert_eq!(
        out.status.code(),
        Some(3),
        "a valid document the profile refuses is a post-parse rejection: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let refused = records(out);
    assert_eq!(codes(out), ["profile/interface-not-closed"], "{refused:?}");
    assert_eq!(refused[0]["actual"], machine, "{:?}", refused[0]);
    let message = refused[0]["message"].as_str().expect("a message");
    assert!(
        message.contains("'owner-review'") && message.contains("(Requested)"),
        "the record names the profile and the schemas the statechart describes a boundary with: \
         {message}"
    );
}

// ── check ─────────────────────────────────────────────────────────────

/// The refusal is the profile's, not the document's: the same open statechart
/// passes without a profile and carries no `profile` key, and the same
/// profile passes the statechart that closes its interface.
#[test]
fn a_statechart_that_leaves_its_interface_open_is_refused_under_a_profile_that_closes_it() {
    let dir = design("profile-check-single");

    let without = run(&dir, &["check", "open.scxml", "-l", "rust"]);
    let without = manifest(&without);
    assert!(
        without.get("profile").is_none(),
        "a run given no profile names none: {without}"
    );

    let refused = run(
        &dir,
        &[
            "check",
            "open.scxml",
            "-l",
            "rust",
            "--profile",
            "profile.json",
        ],
    );
    assert_refused_for_the_profile(&refused, "open");

    let held = run(
        &dir,
        &[
            "check",
            "closed.scxml",
            "-l",
            "rust",
            "--profile",
            "profile.json",
        ],
    );
    let held = manifest(&held);
    assert_eq!(held["profile"]["name"], "owner-review");
    assert_eq!(held["profile"]["judged"], 1);
    assert_eq!(
        held["profile"]["sha256"],
        sha256_of(&dir.join("profile.json")),
        "the manifest names the profile by the digest of its bytes"
    );
}

/// A profile that constrains nothing judges nothing and says so: `judged` is
/// the number of documents it ASKS something of, and it asks nothing of a
/// statechart, so a run that was given one is not read as a run that passed a
/// constraint it never had. The same holds for a profile of house rules and
/// guidance alone, which is reported and handed over and enforces nothing; a
/// statechart the profile enforces something on is the control that the count
/// is not simply zero.
#[test]
fn a_profile_that_constrains_nothing_refuses_nothing_and_judges_nothing() {
    let dir = design("profile-check-empty");
    fs::write(
        dir.join("rules-only.json"),
        r#"{"record":"sce-authoring-profile","v":1,
            "house_rules":[{"id":"H1","rule":"An unmentioned event is ignored."}],
            "guidance":["Ask first."]}"#,
    )
    .expect("write");
    let held = manifest(&run(
        &dir,
        &[
            "check",
            "open.scxml",
            "-l",
            "rust",
            "--profile",
            "empty-profile.json",
        ],
    ));
    assert!(
        held["profile"].get("name").is_none(),
        "a profile with no label publishes none: {held}"
    );
    assert_eq!(held["profile"]["judged"], 0, "{held}");
    let rules = manifest(&run(
        &dir,
        &[
            "check",
            "open.scxml",
            "-l",
            "rust",
            "--profile",
            "rules-only.json",
        ],
    ));
    assert_eq!(rules["profile"]["judged"], 0, "{rules}");
    assert_eq!(rules["profile"]["guidance"], 1, "{rules}");
    // The control: a profile that enforces one thing on a statechart counts it.
    let enforcing = manifest(&run(
        &dir,
        &[
            "check",
            "closed.scxml",
            "-l",
            "rust",
            "--profile",
            "profile.json",
        ],
    ));
    assert_eq!(enforcing["profile"]["judged"], 1, "{enforcing}");
}

/// A forge document is not a statechart and no version-1 setting applies to
/// it: the profile is given and holds nothing to it, and the manifest says
/// zero instead of leaving a reader to assume otherwise.
#[test]
fn a_run_of_forge_documents_alone_reports_that_it_judged_none() {
    let dir = design("profile-check-forge");
    let single = manifest(&run(
        &dir,
        &[
            "check",
            "job_requested.scxml",
            "-l",
            "rust",
            "--profile",
            "profile.json",
        ],
    ));
    assert_eq!(single["profile"]["judged"], 0, "{single}");

    let set = manifest(&run(
        &dir,
        &[
            "check",
            "--forge",
            "job_requested.scxml",
            "-l",
            "rust",
            "--profile",
            "profile.json",
        ],
    ));
    assert_eq!(set["profile"]["judged"], 0, "{set}");
}

/// On a document set every statechart is judged and every finding is listed:
/// the owner decides about the whole design, and a list that stops at the
/// first statechart hides how far the departure goes.
#[test]
fn every_statechart_of_a_set_is_judged_and_every_finding_is_listed() {
    let dir = design("profile-check-set");
    fs::write(dir.join("second.scxml"), statechart(false)).expect("write");

    let refused = run(
        &dir,
        &[
            "check",
            "--scxml",
            "open.scxml",
            "--scxml",
            "second.scxml",
            "-l",
            "rust",
            "--profile",
            "profile.json",
        ],
    );
    assert_eq!(refused.status.code(), Some(3));
    let refused = records(&refused);
    let machines: Vec<&str> = refused
        .iter()
        .filter(|r| r["code"] == "profile/interface-not-closed")
        .filter_map(|r| r["actual"].as_str())
        .collect();
    assert_eq!(machines, ["open", "second"], "{refused:?}");

    // The control: one closed and one open is one finding, so the list is the
    // departures and not the statecharts.
    let mixed = run(
        &dir,
        &[
            "check",
            "--scxml",
            "closed.scxml",
            "--scxml",
            "second.scxml",
            "-l",
            "rust",
            "--profile",
            "profile.json",
        ],
    );
    assert_eq!(codes(&mixed), ["profile/interface-not-closed"]);

    let held = manifest(&run(
        &dir,
        &[
            "check",
            "--scxml",
            "closed.scxml",
            "-l",
            "rust",
            "--profile",
            "profile.json",
        ],
    ));
    assert_eq!(held["profile"]["judged"], 1, "{held}");
}

// ── an unusable profile ───────────────────────────────────────────────

/// A profile the product cannot fully read is refused whole. Applying the
/// part that was understood would say a document was held to a profile it was
/// not, and the refusal is the first thing said about the run.
#[test]
fn a_profile_the_product_cannot_fully_read_is_refused_whole() {
    let dir = design("profile-unusable");
    let write = |name: &str, body: &str| fs::write(dir.join(name), body).expect("write");
    write(
        "unknown-setting.json",
        r#"{"record":"sce-authoring-profile","v":1,"naming":"camel"}"#,
    );
    write(
        "another-unknown.json",
        r#"{"record":"sce-authoring-profile","v":1,"interface":"closed","naming":"snake"}"#,
    );
    write("newer.json", r#"{"record":"sce-authoring-profile","v":2}"#);
    write(
        "wrong-kind.json",
        r#"{"record":"sce-acceptance-record","v":1}"#,
    );

    let mut ids = Vec::new();
    for profile in [
        "unknown-setting.json",
        "newer.json",
        "wrong-kind.json",
        "missing.json",
    ] {
        // Against the CLOSED statechart, which the profile would pass: the
        // refusal is the profile's, not a departure by the document.
        let out = run(
            &dir,
            &["check", "closed.scxml", "-l", "rust", "--profile", profile],
        );
        assert_eq!(out.status.code(), Some(20), "{profile}");
        let record = records(&out).remove(0);
        assert_eq!(
            record["code"], "cli/profile-unusable",
            "{profile}: {record}"
        );
        assert_eq!(record["actual"], profile, "the path the caller typed");
        ids.push(record["id"].as_str().expect("an id").to_string());
    }
    let mut distinct = ids.clone();
    distinct.sort();
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        ids.len(),
        "each refusal is its own finding: {ids:?}"
    );

    // Keyed on which refusal it is, never on the path: the same fault in two
    // files is one finding.
    let a = run(
        &dir,
        &["check", "closed.scxml", "--profile", "unknown-setting.json"],
    );
    let b = run(
        &dir,
        &["check", "closed.scxml", "--profile", "another-unknown.json"],
    );
    assert_eq!(records(&a)[0]["id"], records(&b)[0]["id"]);

    // The first thing said: a document that would itself be refused is not
    // reached.
    let out = run(
        &dir,
        &[
            "check",
            "no-such.scxml",
            "--profile",
            "unknown-setting.json",
        ],
    );
    assert_eq!(codes(&out), ["cli/profile-unusable"]);
}

// ── generate and orchestrate ──────────────────────────────────────────

/// A design the profile refuses produces nothing, and the producer agrees
/// with the check that predicts it.
#[test]
fn generate_and_orchestrate_hold_to_the_profile_and_write_nothing() {
    let dir = design("profile-produce");

    let generated = run(
        &dir,
        &[
            "generate",
            "open.scxml",
            "-l",
            "rust",
            "-o",
            "generated",
            "--profile",
            "profile.json",
        ],
    );
    assert_refused_for_the_profile(&generated, "open");
    assert!(
        !dir.join("generated").exists()
            || fs::read_dir(dir.join("generated"))
                .expect("read")
                .next()
                .is_none(),
        "a design the profile refuses generates nothing"
    );

    let orchestrated = run(
        &dir,
        &[
            "orchestrate",
            "--scxml",
            "open.scxml",
            "-l",
            "rust",
            "-o",
            "orchestrated",
            "--profile",
            "profile.json",
        ],
    );
    assert_refused_for_the_profile(&orchestrated, "open");
    assert!(!dir.join("orchestrated").exists());

    // The controls: the closed statechart is generated, and its manifest
    // names the profile the same way `check`'s does.
    let generated = manifest(&run(
        &dir,
        &[
            "generate",
            "closed.scxml",
            "-l",
            "rust",
            "-o",
            "generated",
            "--profile",
            "profile.json",
        ],
    ));
    assert_eq!(generated["profile"]["judged"], 1, "{generated}");
    let orchestrated = manifest(&run(
        &dir,
        &[
            "orchestrate",
            "--scxml",
            "closed.scxml",
            "-l",
            "rust",
            "-o",
            "orchestrated",
            "--profile",
            "profile.json",
        ],
    ));
    assert_eq!(orchestrated["profile"]["judged"], 1, "{orchestrated}");
    assert_eq!(generated["profile"], orchestrated["profile"]);
}

// ── the acceptance record ─────────────────────────────────────────────

/// A design accepted under one profile is not the answer for another: the
/// digest is part of what the design was held to, and a role left out is part
/// of the answer.
#[test]
fn an_acceptance_answers_only_for_the_profile_it_was_taken_under() {
    let dir = design("profile-accept");
    fs::create_dir_all(dir.join("spec")).expect("mkdir");
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/requirement_closure/iso13400_2_nl_socket_handling.manifest.json"),
        dir.join("spec/manifest.json"),
    )
    .expect("copy the committed manifest");
    let accept = |document: &str, out: &str, extra: &[&str]| {
        let mut args = vec![
            "accept",
            document,
            "--manifest",
            "spec/manifest.json",
            "--variant",
            "base",
            "--root",
            ".",
            "--out",
            out,
        ];
        args.extend_from_slice(extra);
        run(&dir, &args)
    };
    let check = |record: &str, extra: &[&str]| {
        let mut args = vec![
            "acceptance-check",
            record,
            "--variant",
            "base",
            "--root",
            ".",
        ];
        args.extend_from_slice(extra);
        run(&dir, &args)
    };

    // A statechart that departs from the profile is refused, not accepted:
    // the record would say the owner accepted a design under a profile that
    // design breaks.
    let refused = accept(
        "open.scxml",
        "open.record.json",
        &["--profile", "profile.json"],
    );
    assert_refused_for_the_profile(&refused, "open");
    assert!(!dir.join("open.record.json").exists());

    // The control: without a profile the same design is accepted.
    let bare = accept("open.scxml", "bare.record.json", &[]);
    assert!(
        bare.status.success(),
        "{}",
        String::from_utf8_lossy(&bare.stderr)
    );

    let held = accept(
        "closed.scxml",
        "held.record.json",
        &["--profile", "profile.json"],
    );
    assert!(
        held.status.success(),
        "{}",
        String::from_utf8_lossy(&held.stderr)
    );
    let record: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.join("held.record.json")).expect("read"))
            .expect("the record is JSON");
    let pinned = record["authored_from"].as_array().expect("authored_from");
    assert_eq!(pinned.len(), 1, "{record}");
    assert_eq!(pinned[0]["role"], "profile");
    assert_eq!(pinned[0]["path"], "profile.json");
    assert_eq!(pinned[0]["sha256"], sha256_of(&dir.join("profile.json")));

    // Same profile: the acceptance holds.
    let same = check("held.record.json", &["--profile", "profile.json"]);
    assert!(
        same.status.success(),
        "{}",
        String::from_utf8_lossy(&same.stderr)
    );

    // Another profile — one label changed — is another request, and the
    // sentence says the design was HELD to the profile, not authored from it.
    let other = check("held.record.json", &["--profile", "other-profile.json"]);
    assert_eq!(codes(&other), ["cli/acceptance-lapsed"]);
    let said = records(&other)[0]["message"]
        .as_str()
        .expect("a message")
        .to_string();
    assert!(
        said.contains("the design was held to the authoring profile profile.json"),
        "{said}"
    );

    // A file that is no profile is refused as unusable, as `accept` refuses
    // it, and is not reported as a lapse: the question is about a profile.
    fs::write(
        dir.join("not-a-profile.json"),
        r#"{"record":"sce-decision-record","v":1}"#,
    )
    .expect("write");
    let wrong = check("held.record.json", &["--profile", "not-a-profile.json"]);
    assert_eq!(codes(&wrong), ["cli/profile-unusable"]);

    // A role left out is part of the answer, in both directions: a record
    // taken under no profile does not answer for a request that names one.
    let unpinned = check("bare.record.json", &["--profile", "profile.json"]);
    assert_eq!(codes(&unpinned), ["cli/acceptance-lapsed"]);
    let said = records(&unpinned)[0]["message"]
        .as_str()
        .expect("a message")
        .to_string();
    assert!(
        said.contains("does not say which authoring profile the design was held to"),
        "{said}"
    );

    // Asked about the design alone, both hold: naming nothing is asking
    // whether the design has moved.
    assert!(check("held.record.json", &[]).status.success());
    assert!(check("bare.record.json", &[]).status.success());

    // The profile file is pinned, so editing it lapses the acceptance as
    // surely as editing the design.
    fs::write(dir.join("profile.json"), OTHER_PROFILE).expect("edit the profile");
    let moved = check("held.record.json", &[]);
    assert_eq!(codes(&moved), ["cli/acceptance-lapsed"]);
    let said = records(&moved)[0]["message"]
        .as_str()
        .expect("a message")
        .to_string();
    assert!(
        said.contains("the authoring profile this design was held to changed"),
        "{said}"
    );

    // One profile at most: what a design is held to is one profile, and two
    // would leave open which of them it followed. The control is the same
    // call with one.
    let take = |profiles: &[&Path]| {
        let sources: Vec<(SourceRole, &Path)> = profiles
            .iter()
            .map(|profile| (SourceRole::Profile, *profile))
            .collect();
        AcceptanceRecord::take_authored(
            &dir,
            &dir.join("closed.scxml"),
            &dir.join("spec/manifest.json"),
            "base",
            &sources,
        )
    };
    let profile = dir.join("profile.json");
    let other = dir.join("other-profile.json");
    take(&[&profile]).expect("one profile is a record");
    let twice = take(&[&profile, &other]).expect_err("two profiles are not");
    assert!(
        twice.to_string().contains("2 authoring profiles given"),
        "{twice}"
    );
}

// ── the names a document defines ──────────────────────────────────────

/// A statechart whose ids are spelled three ways, that takes an event its
/// schema declares and raises one of its own.
const CROOKED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="job" initial="Idle" datamodel="ecmascript">
  <sce:import src="job_requested.scxml" kind="event-schema" as="Requested"/>
  <datamodel><data id="retryCount" expr="0"/></datamodel>
  <state id="Idle">
    <transition event="job.requested" target="running"/>
  </state>
  <state id="running">
    <onentry><raise event="job.done"/></onentry>
  </state>
</scxml>
"#;

const NAMES: &str = r#"{"record":"sce-authoring-profile","v":1,"name":"owner-review",
    "names":{"state":{"style":"snake"},"data":{"style":"snake"},"event":{"style":"upper_snake"}}}"#;

/// What the document defines is judged and what its interface supplies is not:
/// `job.requested` is spelled in lower case, the profile asks for upper snake
/// case, and only `job.done` — the event the document raises itself — is
/// refused for it.
#[test]
fn the_names_a_document_defines_are_held_to_the_profile_and_the_names_it_is_given_are_not() {
    let dir = design("profile-names");
    fs::write(dir.join("crooked.scxml"), CROOKED).expect("write");
    fs::write(dir.join("names.json"), NAMES).expect("write");
    fs::write(
        dir.join("events-only.json"),
        r#"{"record":"sce-authoring-profile","v":1,"names":{"event":{"style":"snake"}}}"#,
    )
    .expect("write");

    let refused = run(
        &dir,
        &[
            "check",
            "crooked.scxml",
            "-l",
            "rust",
            "--profile",
            "names.json",
        ],
    );
    assert_eq!(refused.status.code(), Some(3));
    let found = records(&refused);
    let said: Vec<(&str, &str)> = found
        .iter()
        .map(|r| (r["code"].as_str().unwrap(), r["actual"].as_str().unwrap()))
        .collect();
    assert_eq!(
        said,
        [
            ("profile/name-style", "Idle"),
            ("profile/name-style", "job.done"),
            ("profile/name-style", "retryCount"),
        ],
        "the state, the event the document raises and the data id; `running` is snake_case and \
         `job.requested` is the schema's: {found:?}"
    );
    let message = found[0]["message"].as_str().expect("a message");
    assert!(
        message.contains("'idle'"),
        "the respelling is offered: {message}"
    );

    // The control: a profile the same document keeps to is not refused, and
    // the schema's own lower-case event is not what makes it pass.
    let held = manifest(&run(
        &dir,
        &[
            "check",
            "crooked.scxml",
            "-l",
            "rust",
            "--profile",
            "events-only.json",
        ],
    ));
    assert_eq!(held["profile"]["judged"], 1, "{held}");
}

/// What the profile hands over that nothing checks is counted on the manifest,
/// and a profile that hands nothing over leaves the count out.
#[test]
fn a_run_says_how_many_instructions_it_handed_over_that_nothing_checked() {
    let dir = design("profile-guidance");
    fs::write(
        dir.join("guided.json"),
        r#"{"record":"sce-authoring-profile","v":1,"interface":"closed",
            "guidance":["Ask before writing.","Comment in the owner's language."]}"#,
    )
    .expect("write");
    let guided = manifest(&run(
        &dir,
        &[
            "check",
            "closed.scxml",
            "-l",
            "rust",
            "--profile",
            "guided.json",
        ],
    ));
    assert_eq!(guided["profile"]["guidance"], 2, "{guided}");
    let plain = manifest(&run(
        &dir,
        &[
            "check",
            "closed.scxml",
            "-l",
            "rust",
            "--profile",
            "profile.json",
        ],
    ));
    assert!(plain["profile"].get("guidance").is_none(), "{plain}");
}

/// An evidence with no anchor is refused under `evidence: anchored`, and one
/// that names where the specification states it is not.
#[test]
fn an_evidence_with_no_anchor_is_refused_under_a_profile_that_asks_for_one() {
    let dir = design("profile-evidence");
    let basis = |evidence: &str| {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="gate" initial="a" datamodel="ecmascript">
  <sce:kind-basis>
    {evidence}
    <sce:rejected kind="timer">it is not periodic</sce:rejected>
  </sce:kind-basis>
  <state id="a"/>
</scxml>
"#
        )
    };
    fs::write(
        dir.join("unanchored.scxml"),
        basis("<sce:evidence>the gate opens on request</sce:evidence>"),
    )
    .expect("write");
    fs::write(
        dir.join("anchored.scxml"),
        basis(r#"<sce:evidence provenance="SPEC@2#4.1">the gate opens on request</sce:evidence>"#),
    )
    .expect("write");
    fs::write(
        dir.join("anchors.json"),
        r#"{"record":"sce-authoring-profile","v":1,"evidence":"anchored"}"#,
    )
    .expect("write");

    let refused = run(
        &dir,
        &[
            "check",
            "unanchored.scxml",
            "-l",
            "rust",
            "--profile",
            "anchors.json",
        ],
    );
    assert_eq!(refused.status.code(), Some(3));
    assert_eq!(codes(&refused), ["profile/evidence-unanchored"]);
    assert_eq!(
        records(&refused)[0]["actual"],
        "the gate opens on request",
        "the record names the evidence, which is all that tells two of them apart"
    );

    let held = manifest(&run(
        &dir,
        &[
            "check",
            "anchored.scxml",
            "-l",
            "rust",
            "--profile",
            "anchors.json",
        ],
    ));
    assert_eq!(held["profile"]["judged"], 1, "{held}");
    // And without the profile, the unanchored one is accepted: the refusal is
    // the profile's.
    manifest(&run(&dir, &["check", "unanchored.scxml", "-l", "rust"]));
}

// ── the documents an interface is made of ─────────────────────────────

/// A schema that declares an event spelled in two cases, with a field id in a
/// third.
const CROOKED_SCHEMA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="job_started"
       sce:event-name="Job.started">
  <datamodel><data id="retryCount" sce:type="uint8" sce:direction="in"/></datamodel>
</scxml>
"#;

const SNAKE_NAMES: &str = r#"{"record":"sce-authoring-profile","v":1,"name":"owner-review",
    "names":{"event":{"style":"snake"},"data":{"style":"snake"}}}"#;

/// The names an interface is made of are written in its event-schemas, and it
/// is there that a boundary event is spelled once: a statechart takes the name
/// from the schema and is not judged for it, so a rule that reached only the
/// statechart would leave them outside the profile.
#[test]
fn an_event_schema_is_held_to_the_names_it_declares() {
    let dir = design("profile-schema-names");
    fs::write(dir.join("crooked_schema.scxml"), CROOKED_SCHEMA).expect("write");
    fs::write(dir.join("snake.json"), SNAKE_NAMES).expect("write");

    let refused = run(
        &dir,
        &[
            "check",
            "crooked_schema.scxml",
            "-l",
            "rust",
            "--profile",
            "snake.json",
        ],
    );
    assert_eq!(
        refused.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&refused.stderr)
    );
    let found = records(&refused);
    let said: Vec<(&str, &str)> = found
        .iter()
        .map(|r| (r["code"].as_str().unwrap(), r["actual"].as_str().unwrap()))
        .collect();
    assert_eq!(
        said,
        [
            ("profile/name-style", "Job.started"),
            ("profile/name-style", "retryCount")
        ],
        "{found:?}"
    );

    // The control: the schema the fixtures already carry is snake_case
    // throughout, and the profile judges it.
    let held = manifest(&run(
        &dir,
        &[
            "check",
            "job_requested.scxml",
            "-l",
            "rust",
            "--profile",
            "snake.json",
        ],
    ));
    assert_eq!(held["profile"]["judged"], 1, "{held}");

    // On a set the statechart and its schema are both judged, and the count
    // says so: a profile that reached only the statechart would say 1.
    let set = manifest(&run(
        &dir,
        &[
            "check",
            "--document",
            "closed.scxml",
            "--document",
            "job_requested.scxml",
            "-l",
            "rust",
            "--profile",
            "snake.json",
        ],
    ));
    assert_eq!(set["profile"]["judged"], 2, "{set}");
    let refused_set = run(
        &dir,
        &[
            "check",
            "--document",
            "closed.scxml",
            "--document",
            "crooked_schema.scxml",
            "-l",
            "rust",
            "--profile",
            "snake.json",
        ],
    );
    assert_eq!(
        codes(&refused_set),
        ["profile/name-style", "profile/name-style"]
    );
}

/// A profile whose settings do not reach a kind does not count it as judged,
/// so a run of such documents is not read as a pass.
#[test]
fn a_document_no_setting_reaches_is_not_counted_as_judged() {
    let dir = design("profile-schema-unreached");
    // `interface` is about a statechart; a schema is not one.
    let unreached = manifest(&run(
        &dir,
        &[
            "check",
            "job_requested.scxml",
            "-l",
            "rust",
            "--profile",
            "profile.json",
        ],
    ));
    assert_eq!(unreached["profile"]["judged"], 0, "{unreached}");
    // `names.state` is about a statechart too, and a schema has no state.
    fs::write(
        dir.join("states.json"),
        r#"{"record":"sce-authoring-profile","v":1,"names":{"state":{"style":"snake"}}}"#,
    )
    .expect("write");
    let states = manifest(&run(
        &dir,
        &[
            "check",
            "job_requested.scxml",
            "-l",
            "rust",
            "--profile",
            "states.json",
        ],
    ));
    assert_eq!(states["profile"]["judged"], 0, "{states}");
}

// ── house rules ───────────────────────────────────────────────────────

const HOUSE_RULES: &str = r#"{"record":"sce-authoring-profile","v":1,"name":"owner-review",
    "house_rules":[{"id":"H1","rule":"An event a state does not mention is ignored."}]}"#;

/// The closed machine with one `sce:assumed` on its first state, citing `rule`.
fn citing(rule: &str) -> String {
    statechart(true).replace(
        "<state id=\"idle\">",
        &format!(
            "<state id=\"idle\" sce:assumed=\"{rule}\" \
             sce:assumed-reason=\"the specification names no event for this state\">"
        ),
    )
}

/// A house rule is the owner's standing answer, so a citation of one is said
/// apart from a value chosen without an answer — by the manifest's `open`, by
/// the marker's own record, and by the acceptance record — and a run given no
/// profile cannot tell a rule's id from any other.
#[test]
fn a_citation_of_a_house_rule_is_said_to_be_the_owners_standing_answer() {
    let dir = design("profile-house-rules");
    fs::write(dir.join("citing.scxml"), citing("H1")).expect("write");
    fs::write(dir.join("stranger.scxml"), citing("H9")).expect("write");
    fs::write(dir.join("rules.json"), HOUSE_RULES).expect("write");

    let kinds = |manifest: &serde_json::Value| -> Vec<String> {
        manifest["open"]
            .as_array()
            .map(|open| {
                open.iter()
                    .map(|m| m["kind"].as_str().unwrap().to_string())
                    .collect()
            })
            .unwrap_or_default()
    };
    let under = manifest(&run(
        &dir,
        &[
            "check",
            "citing.scxml",
            "-l",
            "rust",
            "--profile",
            "rules.json",
        ],
    ));
    assert_eq!(kinds(&under), ["house-rule"], "{under}");
    assert_eq!(under["unresolved"][0]["house_rule"], true, "{under}");
    assert!(
        under["open"][0]["message"]
            .as_str()
            .unwrap()
            .contains("1 place(s) apply the profile's house rule(s) (H1)"),
        "{under}"
    );

    // Controls: no profile, and an id the profile does not hold, are both a
    // value chosen without an answer.
    let bare = manifest(&run(&dir, &["check", "citing.scxml", "-l", "rust"]));
    assert_eq!(kinds(&bare), ["assumed"], "{bare}");
    assert!(bare["unresolved"][0].get("house_rule").is_none(), "{bare}");
    let stranger = manifest(&run(
        &dir,
        &[
            "check",
            "stranger.scxml",
            "-l",
            "rust",
            "--profile",
            "rules.json",
        ],
    ));
    assert_eq!(kinds(&stranger), ["assumed"], "{stranger}");

    // `generate` and the marker list say the same.
    let generated = manifest(&run(
        &dir,
        &[
            "generate",
            "citing.scxml",
            "-l",
            "rust",
            "-o",
            "out",
            "--profile",
            "rules.json",
        ],
    ));
    assert_eq!(kinds(&generated), ["house-rule"], "{generated}");
    let listed = Command::new(CODEGEN)
        .args(["unresolved", "citing.scxml", "--profile", "rules.json"])
        .current_dir(&dir)
        .output()
        .expect("spawn sce-codegen");
    let line: serde_json::Value =
        serde_json::from_slice(&listed.stdout).expect("one NDJSON record");
    assert_eq!(line["id"], "H1");
    assert_eq!(line["house_rule"], true, "{line}");
    let unmarked = Command::new(CODEGEN)
        .args(["unresolved", "citing.scxml"])
        .current_dir(&dir)
        .output()
        .expect("spawn sce-codegen");
    let line: serde_json::Value =
        serde_json::from_slice(&unmarked.stdout).expect("one NDJSON record");
    assert!(line.get("house_rule").is_none(), "{line}");
}

/// What the owner accepted the design WITH is written the way a run under the
/// profile writes it: the record keeps the rule as a rule.
#[test]
fn an_acceptance_records_a_house_rule_as_the_rule_it_is() {
    let dir = design("profile-house-rules-accept");
    fs::create_dir_all(dir.join("spec")).expect("mkdir");
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/requirement_closure/iso13400_2_nl_socket_handling.manifest.json"),
        dir.join("spec/manifest.json"),
    )
    .expect("copy the committed manifest");
    fs::write(dir.join("citing.scxml"), citing("H1")).expect("write");
    fs::write(dir.join("rules.json"), HOUSE_RULES).expect("write");
    let accept = |out: &str, extra: &[&str]| {
        let mut args = vec![
            "accept",
            "citing.scxml",
            "--manifest",
            "spec/manifest.json",
            "--variant",
            "base",
            "--root",
            ".",
            "--out",
            out,
        ];
        args.extend_from_slice(extra);
        run(&dir, &args)
    };
    let kinds_of = |record: &str| -> Vec<String> {
        let record: serde_json::Value =
            serde_json::from_slice(&fs::read(dir.join(record)).expect("read")).expect("JSON");
        record["open_at_acceptance"]
            .as_array()
            .map(|open| {
                open.iter()
                    .map(|m| m["kind"].as_str().unwrap().to_string())
                    .collect()
            })
            .unwrap_or_default()
    };
    let held = accept("held.json", &["--profile", "rules.json"]);
    assert!(
        held.status.success(),
        "{}",
        String::from_utf8_lossy(&held.stderr)
    );
    assert_eq!(kinds_of("held.json"), ["house-rule"]);
    let bare = accept("bare.json", &[]);
    assert!(
        bare.status.success(),
        "{}",
        String::from_utf8_lossy(&bare.stderr)
    );
    assert_eq!(kinds_of("bare.json"), ["assumed"]);
}

/// An id is one rule: a profile that lists it twice is refused whole, since a
/// draft citing it could not say which it applied.
#[test]
fn a_house_rule_listed_twice_makes_the_profile_unusable() {
    let dir = design("profile-house-rules-twice");
    fs::write(
        dir.join("twice.json"),
        r#"{"record":"sce-authoring-profile","v":1,"house_rules":[
            {"id":"H1","rule":"a"},{"id":"H1","rule":"b"}]}"#,
    )
    .expect("write");
    let out = run(
        &dir,
        &[
            "check",
            "closed.scxml",
            "-l",
            "rust",
            "--profile",
            "twice.json",
        ],
    );
    assert_eq!(out.status.code(), Some(20));
    assert_eq!(codes(&out), ["cli/profile-unusable"]);
    assert!(
        records(&out)[0]["message"]
            .as_str()
            .unwrap()
            .contains("listed twice"),
        "{:?}",
        records(&out)
    );
}
