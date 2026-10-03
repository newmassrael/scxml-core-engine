// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the workbench asks of `sce-codegen` about requirements and acceptance, held
//! against a stand-in program, and once against the real generator.
//!
//! What is under test is what the workbench does around the product: how a work is
//! laid out for it (the record pins files by path, so the layout is part of the
//! record), what is passed (`--channel direct` is how the owner's own button states
//! itself), how its lines are read, and what each way of not answering looks like.
//! Whether a requirement is carried or an acceptance holds is the product's, and the
//! real generator is asked at the end, which says so when it cannot run.

#![cfg(unix)]

mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::OnceLock;

use sce_app_core::{
    Acceptor, CheckOutcome, ModelFiles, RenderError, Requirements, SceCodegen, Snapshot,
};

/// The stand-in generator, deciding what to do by what the staged design (or the record
/// it is shown) says. It insists on the layout the workbench promises: a check that
/// finds no design or no source under `--root` is a failure of the layout, not an answer.
const SCRIPT: &str = r#"#!/bin/sh
if [ "$1" = "--version" ]; then echo "stand-in-codegen 1.2.3 (abc123)"; exit 0; fi
cmd="$3"; first="$4"; all="$*"
usage() { echo "{\"v\":1,\"code\":\"cli/usage\",\"message\":\"$1\"}" >&2; exit 2; }
flag() { prev=""; for a in "$@"; do if [ "$prev" = "$FLAG" ]; then echo "$a"; return; fi; prev="$a"; done; }
variant() { case "$all" in *"--variant base"*) ;; *) usage "no --variant base";; esac; }
case "$cmd" in
  requirements)
    model="$(cat "$first")"
    case "$model" in
      *REFUSE*) echo '{"v":1,"code":"validation/invalid-reference","message":"no such state"}' >&2; exit 3;;
      *SILENT*) exit 3;;
      *NONE*) echo '{"kind":"extraction","denominator":"derived"}'; exit 0;;
    esac
    echo '{"kind":"extraction","declared":{"ids":"native"},"denominator":"derived"}'
    echo '{"kind":"requirement","id":"R1","outcome":"missing","section":"1.1","at":{"page":1},"node_paths":[]}'
    echo '{"kind":"requirement","id":"R2","outcome":"implemented","section":"1.2","node_paths":["states.a","states.b.transitions[0]"]}'
    echo 'a line that is not json'
    echo '{"kind":"requirement","id":"R3"}'
    echo '{"kind":"requirement","id":"R4","outcome":"needs-scenario"}'
    exit 0;;
  acceptance-report)
    variant
    model="$(cat "$first")"
    case "$model" in *NOPAGE*) echo '{"v":1,"code":"cli/acceptance-report-unsupported","message":"no page of this"}' >&2; exit 20;; esac
    printf 'ACCEPTANCE REPORT\nargs: %s\n  trailing   spaces   \n' "$all"
    exit 0;;
  accept)
    variant
    FLAG=--out; out="$(flag "$@")"; FLAG=--root; root="$(flag "$@")"
    model="$(cat "$first")"
    case "$model" in
      *REFUSE*) echo '{"v":1,"code":"cli/accept-refused","message":"the design is not acceptable"}' >&2; exit 20;;
      *SILENT*) exit 3;;
      *NORECORD*) exit 0;;
    esac
    files="$(cd "$root" && find . -type f | sort | tr '\n' ' ')"
    case "$model" in
      *OPEN*) open='[{"kind":"question","message":"1 question open (q1)"},{"kind":"self-delivered","message":"sends itself an event"},{"kind":"unworded"}]';;
      *) open='[]';;
    esac
    printf '{"record":"sce-acceptance-record","v":1,"args":"%s","files":"%s","open_at_acceptance":%s}\n' "$all" "$files" "$open" > "$out"
    exit 0;;
  acceptance-check)
    variant
    record="$(cat "$first")"
    case "$record" in
      *LAPSE*) echo '{"v":1,"id":"x","code":"cli/acceptance-lapsed","stage":"cli","message":"/scratch/acceptance.json: the acceptance no longer holds: a: changed; b: gone","actual":"a: changed; b: gone"}' >&2; exit 20;;
      *BROKEN*) echo '{"v":1,"code":"cli/acceptance-unreadable","message":"the record does not parse"}' >&2; exit 3;;
      *SILENT*) exit 3;;
    esac
    FLAG=--root; root="$(flag "$@")"
    [ -f "$root/design/model.scxml" ] || { echo '{"v":1,"code":"cli/missing-design","message":"no design under the root"}' >&2; exit 3; }
    [ -f "$root/spec/source.txt" ] || { echo '{"v":1,"code":"cli/missing-source","message":"no source under the root"}' >&2; exit 3; }
    exit 0;;
esac
usage "no such command $cmd"
"#;

fn stand_in() -> &'static PathBuf {
    static PROGRAM: OnceLock<PathBuf> = OnceLock::new();
    PROGRAM.get_or_init(|| {
        let dir = common::scratch("acceptance-standin");
        let path = dir.join("sce-codegen");
        std::fs::write(&path, SCRIPT).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    })
}

fn generator() -> SceCodegen {
    SceCodegen::at(stand_in())
}

const MANIFEST: &str = "{\"doc_id\":\"door\",\"rev\":\"1\",\"requirements\":[]}\n";
const SIDECAR: &str = "{\"doc_id\":\"door\",\"rev\":\"1\",\"text\":{}}\n";

fn snapshot(model: &str, sidecar: bool, answers: Option<&str>) -> Snapshot {
    Snapshot {
        model: ModelFiles::single(model),
        requirements: Requirements::new(MANIFEST.to_string(), sidecar.then(|| SIDECAR.to_string()))
            .unwrap(),
        source: "The door opens.\n".to_string(),
        answers: answers.map(str::to_string),
    }
}

/// The product's lines are read as it writes them: each requirement with its outcome,
/// where the text anchors it and where the design carries it; a line that is not JSON and
/// a requirement with no outcome are not requirements.
#[test]
fn the_requirements_are_the_lines_the_product_wrote_and_nothing_it_did_not() {
    let report = generator()
        .report_requirements(&snapshot("<scxml/>", true, None))
        .unwrap();
    assert_eq!(report.denominator.as_deref(), Some("derived"));
    assert_eq!(
        report.generator.as_deref(),
        Some("stand-in-codegen 1.2.3 (abc123)")
    );
    let seen: Vec<(&str, &str)> = report
        .outcomes
        .iter()
        .map(|o| (o.id.as_str(), o.outcome.as_str()))
        .collect();
    assert_eq!(
        seen,
        [
            ("R1", "missing"),
            ("R2", "implemented"),
            ("R4", "needs-scenario")
        ]
    );
    assert_eq!(report.outcomes[0].section.as_deref(), Some("1.1"));
    assert!(report.outcomes[0].node_paths.is_empty());
    assert_eq!(
        report.outcomes[1].node_paths,
        ["states.a", "states.b.transitions[0]"]
    );
    assert_eq!(report.outcomes[2].section, None);
}

/// The page is the product's, byte for byte, and it is asked for the layout the record
/// will pin: the design under `design/`, the list under `spec/`, the sidecar only when
/// the list came with one.
#[test]
fn the_page_is_asked_for_the_staged_work_and_comes_back_byte_for_byte() {
    let with = generator()
        .report_requirements(&snapshot("<scxml/>", true, None))
        .unwrap();
    let page = with.page.expect("a page");
    assert!(page.starts_with("ACCEPTANCE REPORT\n"), "{page}");
    assert!(
        page.ends_with("  trailing   spaces   \n"),
        "not trimmed: {page:?}"
    );
    assert!(page.contains("design/model.scxml "), "{page}");
    assert!(page.contains("spec/requirements.manifest.json "), "{page}");
    assert!(page.contains("--variant base"), "{page}");
    assert!(page.contains("--sidecar "), "{page}");
    assert_eq!(with.page_refusal, None);

    let without = generator()
        .report_requirements(&snapshot("<scxml/>", false, None))
        .unwrap();
    assert!(!without.page.unwrap().contains("--sidecar"));
}

/// A design the product measures but writes no page of keeps its outcomes and says why
/// there is no page; a design it will not measure, or says nothing of, is not an answer.
#[test]
fn a_page_the_product_will_not_write_does_not_take_the_outcomes_with_it() {
    let report = generator()
        .report_requirements(&snapshot("<scxml><!-- NOPAGE --></scxml>", false, None))
        .unwrap();
    assert_eq!(report.outcomes.len(), 3);
    assert_eq!(report.page, None);
    let refusal = report.page_refusal.expect("the page's refusal");
    assert_eq!(refusal.code, "cli/acceptance-report-unsupported");

    match generator().report_requirements(&snapshot("<scxml>REFUSE</scxml>", false, None)) {
        Err(RenderError::Refused { code, .. }) => assert_eq!(code, "validation/invalid-reference"),
        other => panic!("{other:?}"),
    }
    for (marker, what) in [("SILENT", "no word"), ("NONE", "no requirement")] {
        let model = format!("<scxml>{marker}</scxml>");
        assert!(
            matches!(
                generator().report_requirements(&snapshot(&model, false, None)),
                Err(RenderError::Failed { .. })
            ),
            "{what}"
        );
    }
}

/// The owner's button states itself `direct`; the work is laid out in the fixed layout
/// the record pins; the owner's answers are given to the product only when there are some.
#[test]
fn an_acceptance_is_taken_for_the_laid_out_work_and_stated_as_the_owners_own() {
    let taken = generator()
        .take_acceptance(&snapshot("<scxml/>", true, None))
        .unwrap();
    let record: serde_json::Value = serde_json::from_str(&taken.record).unwrap();
    assert_eq!(
        record["record"], "sce-acceptance-record",
        "the product's bytes"
    );
    let args = record["args"].as_str().unwrap();
    assert!(args.contains("--channel direct"), "{args}");
    assert!(args.contains("--variant base"), "{args}");
    assert!(args.contains("--out "), "{args}");
    assert!(
        !args.contains("--decisions"),
        "no answers were given: {args}"
    );
    assert_eq!(
        record["files"],
        "./design/model.scxml ./spec/requirements.manifest.json \
         ./spec/requirements.sidecar.json ./spec/source.txt "
    );
    assert!(taken.open.is_empty());

    let answered = generator()
        .take_acceptance(&snapshot("<scxml/>", false, Some("{\"answers\":{}}\n")))
        .unwrap();
    let record: serde_json::Value = serde_json::from_str(&answered.record).unwrap();
    assert!(record["args"].as_str().unwrap().contains("--decisions "));
    assert!(record["files"]
        .as_str()
        .unwrap()
        .contains("./spec/answers.json "));
}

/// What the design leaves open is read from the record's own words: an entry the product
/// gave no sentence for is not made up.
#[test]
fn what_the_design_left_open_is_the_records_own_sentences() {
    let taken = generator()
        .take_acceptance(&snapshot("<scxml><!-- OPEN --></scxml>", false, None))
        .unwrap();
    assert_eq!(
        taken.open,
        ["1 question open (q1)", "sends itself an event"]
    );
}

/// The product saying no, saying nothing, or accepting and leaving no record is a failure
/// of its own kind each time, and none of them is an acceptance.
#[test]
fn a_product_that_will_not_accept_gives_no_record() {
    match generator().take_acceptance(&snapshot("<scxml>REFUSE</scxml>", false, None)) {
        Err(RenderError::Refused { code, .. }) => assert_eq!(code, "cli/accept-refused"),
        other => panic!("{other:?}"),
    }
    for marker in ["SILENT", "NORECORD"] {
        let model = format!("<scxml>{marker}</scxml>");
        assert!(
            matches!(
                generator().take_acceptance(&snapshot(&model, false, None)),
                Err(RenderError::Failed { .. })
            ),
            "{marker}"
        );
    }
}

/// A record that holds is `Holds`, and the check is shown the layout the record was taken
/// in (the stand-in fails without it); a lapse is an answer, in the product's one
/// sentence, and a record the product cannot read is not a lapse.
#[test]
fn a_check_says_holds_or_the_products_sentence_and_nothing_else_is_a_lapse() {
    let now = snapshot("<scxml/>", true, Some("{\"answers\":{}}\n"));
    assert_eq!(
        generator()
            .check_acceptance(&now, "{\"record\":1}")
            .unwrap(),
        CheckOutcome::Holds
    );
    assert_eq!(
        generator()
            .check_acceptance(&now, "{\"record\":\"LAPSE\"}")
            .unwrap(),
        CheckOutcome::Lapsed {
            says: "a: changed; b: gone".to_string()
        },
        "one sentence, not cut at the product's own `; `"
    );
    match generator().check_acceptance(&now, "{\"record\":\"BROKEN\"}") {
        Err(RenderError::Refused { code, .. }) => assert_eq!(code, "cli/acceptance-unreadable"),
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        generator().check_acceptance(&now, "{\"record\":\"SILENT\"}"),
        Err(RenderError::Failed { .. })
    ));
}

/// A design of the product's own, with the requirement list its own tests hold it to.
const DOOR: &str = include_str!(
    "../../sce-build/tests/fixtures/requirement_closure/doip_nl_connection_states.scxml"
);
const LIST: &str = include_str!(
    "../../sce-build/tests/fixtures/requirement_closure/iso13400_2_nl_socket_handling.manifest.json"
);

/// The real generator: it measures the design against the list, writes the page the owner
/// reads, takes a record that says which surface stated it and what the design left
/// open, and then holds the record to the work as it moves.
#[test]
fn the_real_generator_measures_accepts_and_notices_what_moved() {
    let Some(program) = std::env::var_os("SCE_CODEGEN").filter(|v| !v.is_empty()) else {
        eprintln!("SKIPPED: SCE_CODEGEN names no generator");
        return;
    };
    let real = SceCodegen::at(PathBuf::from(program));
    let work = |model: &str, source: &str, answers: Option<&str>| Snapshot {
        model: ModelFiles::single(model),
        requirements: Requirements::new(LIST.to_string(), None).unwrap(),
        source: source.to_string(),
        answers: answers.map(str::to_string),
    };
    let text = "A text the door was designed from.\n";
    let accepted = work(DOOR, text, None);

    let report = real.report_requirements(&accepted).unwrap();
    let outcome = |id: &str| {
        report
            .outcomes
            .iter()
            .find(|o| o.id == id)
            .unwrap_or_else(|| panic!("{id} is in the list: {:?}", report.outcomes))
    };
    assert_eq!(outcome("3.DoIP-152").outcome, "missing");
    assert_eq!(outcome("3.DoIP-153").outcome, "implemented");
    assert_eq!(outcome("3.DoIP-153").node_paths, ["states.registered"]);
    assert!(report
        .page
        .is_some_and(|p| p.starts_with("ACCEPTANCE REPORT")));

    let taken = real.take_acceptance(&accepted).unwrap();
    let record: serde_json::Value = serde_json::from_str(&taken.record).unwrap();
    assert_eq!(record["record"], "sce-acceptance-record");
    assert_eq!(record["channel"], "direct", "the owner's own button");
    assert!(
        taken
            .open
            .iter()
            .any(|m| m.contains("T_TCP_Initial_Inactivity")),
        "{:?}",
        taken.open
    );
    assert_eq!(
        real.check_acceptance(&accepted, &taken.record).unwrap(),
        CheckOutcome::Holds
    );

    let lapsed = |now: &Snapshot| match real.check_acceptance(now, &taken.record).unwrap() {
        CheckOutcome::Lapsed { says } => says,
        CheckOutcome::Holds => panic!("the acceptance held for a work that moved"),
    };
    let edited_text = lapsed(&work(DOOR, "A text, edited.\n", None));
    assert!(edited_text.contains("spec/source.txt"), "{edited_text}");
    let edited_design = lapsed(&work(&format!("{DOOR}<!-- edited -->\n"), text, None));
    assert!(
        edited_design.contains("design/model.scxml"),
        "{edited_design}"
    );
    assert!(
        !edited_design.contains("scratch"),
        "the product's own words: {edited_design}"
    );
    let answered = lapsed(&work(DOOR, text, Some("{\"answers\":{}}\n")));
    assert!(answered.contains("spec/answers.json"), "{answered}");
}
