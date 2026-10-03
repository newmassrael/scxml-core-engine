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

use sce_app_core::{call, FigureRenderer, FigureRequest, RenderError, SceCodegen, WorkStore};
use serde_json::json;

/// The stand-in generator. It decides what to do by what the staged model says,
/// so one program serves every case. Written once, before any test starts it:
/// a script that is open for writing when another test forks is a program that
/// cannot be run ("text file busy").
const SCRIPT: &str = r#"#!/bin/sh
if [ "$1" = "--version" ]; then echo "stand-in-codegen 1.2.3 (abc123)"; exit 0; fi
doc="$4"; out="$6"
model="$(cat "$doc")"
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
