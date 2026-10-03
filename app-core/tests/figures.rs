// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The renderer that runs `sce-codegen`, held against a stand-in program.
//!
//! What is under test is what the workbench does around the generator: how the
//! model is staged, what is passed, what comes back and in which order, what a
//! refusal looks like, and that a generator that hangs, escapes its folder or
//! says nothing is a refusal of its own and not a wedged window. A stand-in
//! shell script speaks the generator's protocol, so none of it depends on the
//! product being built; the real generator is exercised by the one test at the
//! end, which says so when it cannot run.

#![cfg(unix)]

mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use sce_app_core::{
    call, Document, FigureRenderer, FigureRequest, ModelReviewer, RenderError, ReviewRequest,
    SceCodegen, Verdict, WorkStore,
};
use serde_json::json;

/// The stand-in generator. It decides what to do by what the staged model says,
/// so one program serves every case. Written once, before any test starts it:
/// a script that is open for writing when another test forks is a program that
/// cannot be run ("text file busy").
const SCRIPT: &str = r#"#!/bin/sh
if [ "$1" = "--version" ]; then echo "stand-in-codegen 1.2.3 (abc123)"; exit 0; fi
doc="$4"; out="$6"
model="$(cat "$doc")"
case "$3" in
  check)
    case "$*" in *--lint*) ;; *) echo '{"v":1,"code":"cli/no-lint","message":"the check was run without --lint"}' >&2; exit 3;; esac
    case "$model" in
      *REFUSE*) echo '{"v":1,"code":"validation/invalid-reference","stage":"validation","message":"no such state","location":{"file":"x","line":3,"col":22},"fix":{"kind":"replace_one_of","candidates":["a","b"]}}' >&2; exit 3;;
      *SILENT*) exit 3;;
      *HANG*) sleep 30; exit 0;;
      *NOMANIFEST*) exit 0;;
    esac
    printf '{"v":1,"kind":"check","document_kind":{"name":"statechart","declared":true},"unresolved":[{"id":"q1","node_path":"states.a","reason":"Which card values open the door?","location":{"line":4}}],"open":[{"kind":"question","message":"1 question open (q1)"}],"args":"%s"}\n' "$*"
    exit 0;;
  pseudo)
    case "$model" in
      *NOPAGE*) echo '{"v":1,"code":"cli/pseudo-unsupported","message":"not abbreviated"}' >&2; exit 20;;
      *CRASHPAGE*) echo "boom" >&2; exit 139;;
    esac
    printf 'page args: %s\n  trailing   spaces   \n' "$*"
    exit 0;;
esac
case "$model" in
  *REFUSE*) echo '{"v":1,"id":"x","code":"cli/diagram-does-not-fit","message":"the figure needs 925 pt"}' >&2; exit 20;;
  *HANG*) sleep 30; exit 0;;
  *ESCAPE*) echo /etc/hostname; exit 0;;
  *EMPTY*) exit 0;;
  *CRASH*) echo "segmentation fault" >&2; exit 139;;
  *SPAM*) i=0; while [ $i -lt 20000 ]; do echo "noise noise noise noise noise noise noise noise" >&2; i=$((i+1)); done; exit 20;;
esac
mkdir -p "$out"
printf '<svg>args: %s | size: %s</svg>\n' "$*" "$(wc -c < "$doc")" > "$out/picture.svg"
printf '<svg>fields</svg>\n' > "$out/fields-1.svg"
echo "$out/picture.svg"
echo "$out/fields-1.svg"
"#;

fn stand_in() -> &'static PathBuf {
    static PROGRAM: OnceLock<PathBuf> = OnceLock::new();
    PROGRAM.get_or_init(|| {
        let dir = common::scratch("figures-standin");
        let path = dir.join("sce-codegen");
        std::fs::write(&path, SCRIPT).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    })
}

fn generator() -> SceCodegen {
    SceCodegen::at(stand_in())
}

fn request(model: &str) -> FigureRequest<'_> {
    FigureRequest {
        model,
        ..FigureRequest::default()
    }
}

/// The sheets come back in the order the generator wrote them, named by its
/// files, with the model and the options it was handed, and its own version.
#[test]
fn a_model_is_staged_drawn_and_returned_in_the_generators_order() {
    let drawn = generator()
        .render(&FigureRequest {
            model: "<scxml/>",
            name: Some("door-lock"),
            page: Some("a3-landscape"),
            lexicon: Some("ko"),
            min_pt: Some(8.5),
            ..FigureRequest::default()
        })
        .unwrap();
    let names: Vec<&str> = drawn.sheets.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["picture.svg", "fields-1.svg"]);
    assert_eq!(
        drawn.generator.as_deref(),
        Some("stand-in-codegen 1.2.3 (abc123)")
    );
    let first = &drawn.sheets[0].svg;
    assert!(first.contains("--error-format json diagram"), "{first}");
    assert!(first.contains("--page a3-landscape"), "{first}");
    assert!(first.contains("--lexicon ko"), "{first}");
    assert!(first.contains("--min-pt 8.5"), "{first}");
    // The product names its figures by the document's file, so the staged file
    // carries the name it was given; with none it is `model`.
    assert!(first.contains("/door-lock.scxml "), "{first}");
    let unnamed = generator().render(&request("<scxml/>")).unwrap();
    assert!(
        unnamed.sheets[0].svg.contains("/model.scxml "),
        "{}",
        unnamed.sheets[0].svg
    );
    assert!(
        first.contains("size: 8"),
        "the model's bytes were staged: {first}"
    );
}

/// The product's refusal reaches the caller with its own code and sentence, and
/// through the command layer as a kind the screen branches on.
#[test]
fn a_refusal_is_the_products_own_words_and_a_kind_the_screen_can_branch_on() {
    match generator().render(&request("<scxml>REFUSE</scxml>")) {
        Err(RenderError::Refused { code, message }) => {
            assert_eq!(code, "cli/diagram-does-not-fit");
            assert!(message.contains("925 pt"), "{message}");
        }
        other => panic!("{other:?}"),
    }

    let store = WorkStore::at(common::scratch("figures-command"));
    let id = store.create_work("Refused").unwrap().id;
    store
        .save_model(&id, "<scxml>REFUSE</scxml>", None, None)
        .unwrap();
    let error = call(&store, &generator(), "figures", json!({"id": id.as_str()})).unwrap_err();
    assert_eq!(error.kind, "sce-refused");
    assert_eq!(error.detail, json!({"code": "cli/diagram-does-not-fit"}));
}

/// A generator that does not finish is stopped, and the wait is bounded.
#[test]
fn a_generator_that_hangs_is_stopped() {
    let started = Instant::now();
    let error = generator()
        .with_timeout(Duration::from_millis(300))
        .render(&request("<scxml>HANG</scxml>"))
        .unwrap_err();
    assert!(matches!(error, RenderError::TimedOut { .. }), "{error:?}");
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "stopped, not waited for: {:?}",
        started.elapsed()
    );
}

/// What the generator names outside the folder it was given is not read.
#[test]
fn a_file_outside_the_output_folder_is_not_read() {
    match generator().render(&request("<scxml>ESCAPE</scxml>")) {
        Err(RenderError::Failed { reason }) => {
            assert!(reason.contains("outside its output folder"), "{reason}");
        }
        other => panic!("{other:?}"),
    }
}

/// A generator that succeeds and draws nothing is a failure, not an empty
/// picture; one that dies is a failure that says what it wrote.
#[test]
fn nothing_drawn_and_a_crash_are_failures_that_say_so() {
    match generator().render(&request("<scxml>EMPTY</scxml>")) {
        Err(RenderError::Failed { reason }) => assert!(reason.contains("no figure"), "{reason}"),
        other => panic!("{other:?}"),
    }
    match generator().render(&request("<scxml>CRASH</scxml>")) {
        Err(RenderError::Failed { reason }) => {
            assert!(
                reason.contains("segmentation fault") && reason.contains("139"),
                "{reason}"
            );
        }
        other => panic!("{other:?}"),
    }
}

/// A generator that writes a great deal to standard error cannot wedge the
/// wait: its output goes to a file, not to a pipe nobody is draining.
#[test]
fn a_noisy_generator_does_not_wedge_the_wait() {
    let started = Instant::now();
    let error = generator()
        .with_timeout(Duration::from_secs(20))
        .render(&request("<scxml>SPAM</scxml>"))
        .unwrap_err();
    assert!(matches!(error, RenderError::Failed { .. }), "{error:?}");
    assert!(
        started.elapsed() < Duration::from_secs(15),
        "{:?}",
        started.elapsed()
    );
}

/// Options are words, not flags: a page that is `--out` is refused before
/// anything runs.
#[test]
fn an_option_that_is_a_flag_is_refused_before_it_runs() {
    for bad in [
        FigureRequest {
            model: "<scxml/>",
            page: Some("--out"),
            ..FigureRequest::default()
        },
        FigureRequest {
            model: "<scxml/>",
            lexicon: Some("a b"),
            ..FigureRequest::default()
        },
        FigureRequest {
            model: "<scxml/>",
            min_pt: Some(-3.0),
            ..FigureRequest::default()
        },
        FigureRequest {
            model: "<scxml/>",
            min_pt: Some(f64::NAN),
            ..FigureRequest::default()
        },
    ] {
        match generator().render(&bad) {
            Err(RenderError::Failed { .. }) => {}
            other => panic!("{bad:?}: {other:?}"),
        }
    }
}

fn review_request(model: &str) -> ReviewRequest<'_> {
    ReviewRequest {
        model,
        ..ReviewRequest::default()
    }
}

/// An accepted model is the product's own verdict, what it leaves open and the
/// questions it marks, and its page byte for byte in the vocabulary asked for.
#[test]
fn an_accepted_model_comes_back_with_its_open_matters_and_its_page() {
    let read = generator()
        .review(&ReviewRequest {
            model: "<scxml/>",
            name: Some("door-lock"),
            lexicon: Some("ko"),
            ..ReviewRequest::default()
        })
        .unwrap();
    assert_eq!(read.check.verdict, Verdict::Accepted);
    assert_eq!(read.check.kind.as_deref(), Some("statechart"));
    assert_eq!(read.check.open, ["1 question open (q1)"]);
    assert_eq!(read.check.unresolved.len(), 1);
    assert_eq!(read.check.unresolved[0].id, "q1");
    assert_eq!(read.check.unresolved[0].line, Some(4));
    assert_eq!(
        read.check.unresolved[0].reason.as_deref(),
        Some("Which card values open the door?")
    );
    assert!(read.check.records.is_empty());
    assert_eq!(
        read.generator.as_deref(),
        Some("stand-in-codegen 1.2.3 (abc123)")
    );
    assert_eq!(read.page_refusal, None);

    // Byte for byte: the trailing spaces and the final newline are the page's.
    let page = read.page.expect("an accepted model has a page");
    assert!(
        page.starts_with("page args: --error-format json pseudo "),
        "{page}"
    );
    assert!(page.contains("/door-lock.scxml --lexicon ko\n"), "{page}");
    assert!(page.ends_with("  trailing   spaces   \n"), "{page:?}");
}

/// The check is run as an author runs it, with the design lints on: the
/// stand-in refuses a check that did not ask for them, so every accepted review
/// above is the proof.
#[test]
fn the_check_is_run_with_the_design_lints_on() {
    let read = generator().review(&review_request("<scxml/>")).unwrap();
    assert_eq!(read.check.verdict, Verdict::Accepted);
    assert!(read.check.records.is_empty(), "{:?}", read.check.records);
}

/// A model the product refuses is an answer, with every record it wrote and no
/// page: not an error, because the person is to be shown why.
#[test]
fn a_refused_model_is_an_answer_with_the_products_records_and_no_page() {
    let read = generator()
        .review(&review_request("<scxml>REFUSE</scxml>"))
        .unwrap();
    assert_eq!(read.check.verdict, Verdict::Refused);
    assert_eq!(read.check.kind, None);
    assert!(read.check.open.is_empty() && read.check.unresolved.is_empty());
    assert_eq!(read.page, None);
    let record = &read.check.records[0];
    assert_eq!(record.code, "validation/invalid-reference");
    assert_eq!(record.stage.as_deref(), Some("validation"));
    assert_eq!(record.line, Some(3));
    assert_eq!(
        record.fix.as_ref().unwrap()["candidates"],
        json!(["a", "b"])
    );
}

/// What the product leaves unexplained is a failure that says what was seen: a
/// refusal with no record, a success with no manifest, a page that dies.
#[test]
fn a_product_that_answers_wrongly_is_a_failure_that_says_so() {
    for (model, wanted) in [
        ("<scxml>SILENT</scxml>", "it wrote nothing"),
        ("<scxml>NOMANIFEST</scxml>", "no manifest"),
        ("<scxml>CRASHPAGE</scxml>", "boom"),
    ] {
        match generator().review(&review_request(model)) {
            Err(RenderError::Failed { reason }) => assert!(reason.contains(wanted), "{reason}"),
            other => panic!("{model}: {other:?}"),
        }
    }
}

/// A page the product will not write of a document it accepted is its own refusal,
/// shown beside the verdict: the owner still sees what was checked.
#[test]
fn an_accepted_model_whose_page_is_refused_keeps_its_verdict() {
    let read = generator()
        .review(&review_request("<scxml>NOPAGE</scxml>"))
        .unwrap();
    assert_eq!(read.check.verdict, Verdict::Accepted);
    assert_eq!(read.page, None);
    let refusal = read.page_refusal.expect("the page was refused");
    assert_eq!(refusal.code, "cli/pseudo-unsupported");
    assert_eq!(refusal.message, "not abbreviated");
}

/// The wait on each run is bounded, and a vocabulary that is a flag is refused
/// before anything runs.
#[test]
fn a_review_is_bounded_and_takes_words_not_flags() {
    let started = Instant::now();
    let error = generator()
        .with_timeout(Duration::from_millis(300))
        .review(&review_request("<scxml>HANG</scxml>"))
        .unwrap_err();
    assert!(matches!(error, RenderError::TimedOut { .. }), "{error:?}");
    assert!(started.elapsed() < Duration::from_secs(10));

    let bad = ReviewRequest {
        model: "<scxml/>",
        lexicon: Some("--out"),
        ..ReviewRequest::default()
    };
    assert!(matches!(
        generator().review(&bad),
        Err(RenderError::Failed { .. })
    ));
}

/// Through the command layer: the same answer with where the model stands to the
/// text, and a work with no model has nothing to review.
#[test]
fn the_review_command_says_where_the_model_stands_and_refuses_a_work_with_none() {
    let store = WorkStore::at(common::scratch("review-command"));
    let id = store.create_work("Door lock").unwrap().id;
    let first = match store.save_source(&id, "The door opens.", None).unwrap() {
        sce_app_core::Saved::Saved { revision, .. } => revision,
        other => panic!("{other:?}"),
    };

    let none = call(&store, &generator(), "review", json!({"id": id.as_str()})).unwrap_err();
    assert_eq!(none.kind, "not-found");

    store
        .save_model(&id, "<scxml/>", None, Some(&first))
        .unwrap();
    let read = call(
        &store,
        &generator(),
        "review",
        json!({"id": id.as_str(), "lexicon": "ko"}),
    )
    .unwrap();
    assert_eq!(read["standing"], "current");
    assert_eq!(read["check"]["verdict"], "accepted");
    assert_eq!(read["check"]["open"][0], "1 question open (q1)");
    assert!(read["page"].as_str().unwrap().contains("--lexicon ko"));
    assert_eq!(read["page_refusal"], serde_json::Value::Null);
    // The product titles its output by the document's name: the work's own.
    assert!(read["page"]
        .as_str()
        .unwrap()
        .contains(&format!("/{}.scxml", id.slug())));

    // The text moves on and the same model is behind; the review still answers.
    store
        .save_source(&id, "The door opens and closes.", Some(&first))
        .unwrap();
    let behind = call(&store, &generator(), "review", json!({"id": id.as_str()})).unwrap();
    assert_eq!(behind["standing"], "behind");

    let unknown = call(
        &store,
        &generator(),
        "review",
        json!({"id": id.as_str(), "colour": "blue"}),
    )
    .unwrap_err();
    assert_eq!(unknown.kind, "bad-request");
}

/// The real generator, when one is named: a lookup is drawn as its picture and
/// its table, and a model SCE does not accept comes back as its refusal.
/// Skipped, and says so, without `SCE_CODEGEN` — the application lane does not
/// build the product, so this is run by whoever has built it.
#[test]
fn the_real_generator_draws_a_model_and_refuses_one_it_will_not() {
    let Some(program) = std::env::var_os("SCE_CODEGEN").filter(|v| !v.is_empty()) else {
        eprintln!("SKIPPED: SCE_CODEGEN names no generator");
        return;
    };
    let real = SceCodegen::at(PathBuf::from(program));
    let lookup = concat!(
        "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" xmlns:sce=\"http://sce.dev/ext\" ",
        "sce:kind=\"lookup\" name=\"signal\"><datamodel>",
        "<data id=\"level\" sce:type=\"uint8\" sce:direction=\"in\"/>",
        "<data id=\"quality\" sce:type=\"string\" sce:direction=\"out\"/>",
        "<data id=\"mapping\" sce:default=\"NONE\">",
        "<sce:entry key=\"0\" value=\"NONE\"/><sce:entry key=\"1\" value=\"LOW\"/>",
        "</data></datamodel></scxml>"
    );
    let drawn = real
        .render(&request(lookup))
        .expect("the product draws a lookup");
    let names: Vec<&str> = drawn.sheets.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["mapping.svg", "fields-1.svg"]);
    assert!(drawn.sheets.iter().all(|s| s.svg.starts_with("<svg")));
    assert!(drawn.generator.as_deref().is_some_and(|g| !g.is_empty()));

    match real.render(&request("<scxml>not a document the product accepts")) {
        Err(RenderError::Refused { code, message }) => {
            assert!(!code.is_empty() && !message.is_empty(), "{code}: {message}");
        }
        other => panic!("the product's own refusal was expected: {other:?}"),
    }
}

/// The real generator reads a model: the verdict, what it leaves open and the
/// questions it marks, and the page in the vocabulary asked for; and a document it
/// refuses is an answer with the product's own records.
#[test]
fn the_real_generator_reads_a_model_and_says_why_it_refuses_one() {
    let Some(program) = std::env::var_os("SCE_CODEGEN").filter(|v| !v.is_empty()) else {
        eprintln!("SKIPPED: SCE_CODEGEN names no generator");
        return;
    };
    let real = SceCodegen::at(PathBuf::from(program));
    let door = |target: &str, marker: &str| {
        format!(
            concat!(
                "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" xmlns:sce=\"http://sce.dev/ext\" ",
                "sce:kind=\"statechart\" version=\"1.0\" initial=\"closed\">",
                "<state id=\"closed\"><transition event=\"open\" target=\"{}\"{}/></state>",
                "<state id=\"opened\"><transition event=\"close\" target=\"closed\"/></state>",
                "</scxml>"
            ),
            target, marker
        )
    };

    let open = door(
        "opened",
        " sce:unresolved=\"open-guard\" sce:unresolved-reason=\"Which card values open the door?\"",
    );
    let read = real
        .review(&ReviewRequest {
            model: &open,
            name: Some("door"),
            lexicon: Some("ko"),
            ..ReviewRequest::default()
        })
        .expect("the product reads a statechart");
    assert_eq!(read.check.verdict, Verdict::Accepted);
    assert_eq!(read.check.kind.as_deref(), Some("statechart"));
    assert_eq!(read.check.unresolved.len(), 1);
    assert_eq!(read.check.unresolved[0].id, "open-guard");
    assert_eq!(
        read.check.unresolved[0].reason.as_deref(),
        Some("Which card values open the door?")
    );
    assert!(
        read.check.open.iter().any(|m| m.contains("open-guard")),
        "{:?}",
        read.check.open
    );
    let page = read.page.expect("an accepted statechart has a page");
    assert!(page.starts_with("#!sce-pseudo"), "{page}");
    assert!(page.contains("lexicon=ko"), "{page}");

    let refused = real
        .review(&review_request(&door("nowhere", "")))
        .expect("a refused model is an answer");
    assert_eq!(refused.check.verdict, Verdict::Refused);
    assert_eq!(refused.page, None);
    assert!(
        refused
            .check
            .records
            .iter()
            .any(|r| r.code == "validation/invalid-reference" && r.line.is_some()),
        "{:?}",
        refused.check.records
    );
}

/// A statechart with a CLOSED interface and the event schema it imports (the product's
/// own fixture): the entry names its import by file, so the schema has to be staged
/// beside it under that name, and a closed interface is exactly what makes the product
/// refuse the entry when it is not.
const ENTRY_FILE: &str = "door.scxml";
const ENTRY: &str = concat!(
    "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" xmlns:sce=\"http://sce.dev/ext\" ",
    "sce:kind=\"statechart\" sce:interface=\"closed\" version=\"1.0\" initial=\"waiting\">",
    "<sce:import src=\"schema_job_completed_minimal.scxml\" kind=\"event-schema\" ",
    "as=\"JobCompletedSchema\"/>",
    "<state id=\"waiting\"><transition event=\"job.completed\" target=\"done\"/></state>",
    "<state id=\"done\"/></scxml>"
);
const SCHEMA_FILE: &str = "schema_job_completed_minimal.scxml";
const SCHEMA: &str =
    include_str!("../../sce-build/tests/fixtures/event_schema/schema_job_completed_minimal.scxml");

/// The real generator reads and draws a model of two documents when both are staged
/// beside each other, and refuses the entry alone for the import it cannot find.
#[test]
fn the_real_generator_reads_and_draws_a_model_of_several_documents() {
    let Some(program) = std::env::var_os("SCE_CODEGEN").filter(|v| !v.is_empty()) else {
        eprintln!("SKIPPED: SCE_CODEGEN names no generator");
        return;
    };
    let real = SceCodegen::at(PathBuf::from(program));
    let schema = [Document {
        name: SCHEMA_FILE.to_string(),
        text: SCHEMA.to_string(),
    }];

    let read = real
        .review(&ReviewRequest {
            model: ENTRY,
            entry_file: Some(ENTRY_FILE),
            siblings: &schema,
            ..ReviewRequest::default()
        })
        .expect("the product reads the set");
    assert_eq!(read.check.verdict, Verdict::Accepted, "{:?}", read.check);
    assert!(read.page.is_some_and(|p| p.contains("job.completed")));

    let alone = real
        .review(&ReviewRequest {
            model: ENTRY,
            entry_file: Some(ENTRY_FILE),
            ..ReviewRequest::default()
        })
        .expect("a refused model is an answer");
    assert_eq!(
        alone.check.verdict,
        Verdict::Refused,
        "the import is not staged"
    );
    assert!(
        alone
            .check
            .records
            .iter()
            .any(|r| r.code == "import/file-not-found"),
        "{:?}",
        alone.check.records
    );

    let drawn = real
        .render(&FigureRequest {
            model: ENTRY,
            entry_file: Some(ENTRY_FILE),
            siblings: &schema,
            ..FigureRequest::default()
        })
        .expect("the product draws the set");
    assert!(drawn.sheets.iter().all(|s| s.svg.starts_with("<svg")));
    assert!(
        drawn.sheets.iter().any(|s| s.svg.contains("waiting")),
        "the entry's states are drawn"
    );
}

/// A name that would leave the staging folder is refused before anything is written,
/// whatever checked it when the model was saved.
#[test]
fn a_document_name_that_is_a_path_is_refused_at_staging() {
    for bad in ["../escape.scxml", "sub/dir.scxml", "a b.scxml", ""] {
        let sibling = [Document {
            name: bad.to_string(),
            text: "x".to_string(),
        }];
        let refused = generator().render(&FigureRequest {
            model: "<scxml/>",
            siblings: &sibling,
            ..FigureRequest::default()
        });
        assert!(
            matches!(refused, Err(RenderError::Failed { .. })),
            "{bad:?}: {refused:?}"
        );
        let entry = generator().render(&FigureRequest {
            model: "<scxml/>",
            entry_file: Some(bad),
            ..FigureRequest::default()
        });
        assert!(matches!(entry, Err(RenderError::Failed { .. })), "{bad:?}");
    }
    // Two documents under one name are one name too many.
    let twin = [Document {
        name: "door.scxml".to_string(),
        text: "x".to_string(),
    }];
    assert!(matches!(
        generator().render(&FigureRequest {
            model: "<scxml/>",
            entry_file: Some("door.scxml"),
            siblings: &twin,
            ..FigureRequest::default()
        }),
        Err(RenderError::Failed { .. })
    ));
}
