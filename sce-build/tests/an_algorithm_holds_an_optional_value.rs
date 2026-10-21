// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An algorithm holds a value that may be absent (docs/adr/0055, step 8): its
//! parameter, its return and its local may be of `optional<T>` over a number or
//! a `bool`, and a field of a record it reads or returns may be one. Each backend
//! spells the value in the one optional type its statechart's variable of the
//! same element is held in, and reads it with the spellings the expression
//! emitter gives every optional value, so a statechart hands one over and takes
//! one back as it holds it. An optional text, byte string, value of an enum or
//! record is a statechart's, and an algorithm's slot refuses it by name.
//!
//! The scenario `static_state_algorithm_optional.json` replays the machine and
//! the algorithms of the fixture directory on every engine; this file holds what
//! the build answers before anything runs.
//!
//! # What was wrong
//!
//! Measured in the code on 2026-10-22: an algorithm's reader refused every
//! `optional<T>` slot, and every backend refused an algorithm whose record had a
//! field that may be absent, so a goal whose day may be absent could not be
//! ordered or counted by an algorithm.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::model::{AlgorithmStmt, AlgorithmValueType, OptionalElemType, SceType};
use tempfile::tempdir;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn fixtures() -> PathBuf {
    repo_root().join("sce-build/tests/fixtures/static_datamodel")
}

/// `sce-codegen <args…> <path>`, with JSON diagnostics, and `-o` a fresh
/// directory for a command that writes one. Returns `(exit_ok, stderr + stdout,
/// the text of every file written)`.
fn run_on(args: &[&str], path: &Path) -> (bool, String, String) {
    let out_dir = tempdir().expect("tempdir");
    let mut command = Command::new(sce_codegen_bin());
    command
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(path);
    if matches!(args.first(), Some(&"generate") | Some(&"lower")) {
        command.arg("-o").arg(out_dir.path());
    }
    let out = command.output().expect("invoke sce-codegen");
    let mut written = String::new();
    for entry in std::fs::read_dir(out_dir.path()).expect("the output directory") {
        let path = entry.expect("an entry").path();
        if path.is_file() {
            written.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
            written.push('\n');
        }
    }
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
        written,
    )
}

/// [`run_on`] of `text`, written as `file` in a directory beside the documents
/// of the fixture directory named in `siblings`.
fn run_probe(args: &[&str], text: &str, file: &str, siblings: &[&str]) -> (bool, String, String) {
    let dir = tempdir().expect("tempdir");
    for name in siblings {
        std::fs::copy(fixtures().join(name), dir.path().join(name)).expect("copy a sibling");
    }
    std::fs::write(dir.path().join(file), text).expect("write the probe");
    run_on(args, &dir.path().join(file))
}

/// The `generate` arguments of each generated backend, by the language it names.
fn every_backend() -> Vec<(&'static str, Vec<&'static str>)> {
    vec![
        ("rust", vec!["generate", "-l", "rust"]),
        ("kotlin", vec!["generate", "-l", "kotlin"]),
        ("python", vec!["generate", "-l", "python"]),
        ("c11", vec!["generate", "-l", "c11"]),
        ("cpp", vec!["generate", "-l", "cpp"]),
        (
            "go",
            vec![
                "generate",
                "-l",
                "go",
                "--go-module-prefix",
                "example.test/generated",
            ],
        ),
    ]
}

/// An algorithm of no import whose one parameter, return and local are declared
/// `param`, `ret` and `local`, returning `null`.
fn slots(param: &str, ret: &str, local: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="slot_probe" version="1.0">
  <sce:signature>
    <sce:param name="day" type="{param}"/>
    <sce:return type="{ret}"/>
  </sce:signature>
  <sce:body>
    <sce:var name="held" type="{local}" init="null"/>
    <sce:return expr="held"/>
  </sce:body>
</scxml>
"#
    )
}

// ── The slots ────────────────────────────────────────────────────────────

/// A parameter, the return and a local of `optional<T>` over a number or a
/// `bool` are read as the optional value they declare.
#[test]
fn an_algorithms_parameter_return_and_local_hold_an_optional_number() {
    use sce_build::forge::model::ForgeDocument;
    for (written, elem) in [
        ("optional&lt;int64&gt;", SceType::Int64),
        ("optional&lt;bool&gt;", SceType::Bool),
        ("optional&lt;float32&gt;", SceType::Float32),
    ] {
        let text = slots(written, written, written);
        let parsed = sce_build::forge::parser::parse_forge_with_imports(
            &text,
            sce_build::DocumentLabel::symmetric("slot_probe"),
        )
        .unwrap_or_else(|e| panic!("`{written}` is read on every slot: {e:?}"))
        .expect("an algorithm is a forge document");
        let ForgeDocument::Algorithm(algorithm) = &parsed.document else {
            panic!("expected an algorithm, got {:?}", parsed.document.kind());
        };
        let optional = AlgorithmValueType::Optional {
            elem: OptionalElemType::Scalar(elem),
        };
        assert_eq!(
            algorithm.signature.params[0].sce_type, optional,
            "{written}"
        );
        assert_eq!(
            algorithm.signature.return_type.as_ref(),
            Some(&optional),
            "{written}"
        );
        let Some(AlgorithmStmt::Var { sce_type, .. }) = algorithm.body.first() else {
            panic!("the local is the body's first statement");
        };
        assert_eq!(sce_type, &optional, "{written}");
    }
}

/// An optional text, byte string, value of an enum or record is held by a
/// statechart's variable and by no algorithm's slot: each is refused by name, as
/// a rule of the position, on the parameter, the return and the local alike.
#[test]
fn an_algorithms_slot_refuses_an_optional_value_it_does_not_hold_by_name() {
    for written in [
        "optional&lt;string&gt;",
        "optional&lt;bytes&gt;",
        "optional&lt;enum:Status&gt;",
        "optional&lt;record:Goal&gt;",
    ] {
        for (param, ret, local) in [
            (written, "optional&lt;int64&gt;", "optional&lt;int64&gt;"),
            ("optional&lt;int64&gt;", written, "optional&lt;int64&gt;"),
            ("optional&lt;int64&gt;", "optional&lt;int64&gt;", written),
        ] {
            let text = slots(param, ret, local).replace(
                "  <sce:signature>",
                "  <sce:import kind=\"enum\" src=\"enum_item_status.scxml\" as=\"Status\"/>\n  \
                 <sce:import kind=\"event-schema\" src=\"schema_goal.scxml\" as=\"Goal\"/>\n  \
                 <sce:signature>",
            );
            let (ok, out, _) = run_probe(
                &["check", "-l", "rust"],
                &text,
                "slot_probe.scxml",
                &["enum_item_status.scxml", "schema_goal.scxml"],
            );
            assert!(!ok, "`{written}` is refused on an algorithm's slot:\n{out}");
            assert!(
                out.contains("validation/attribute-rule-violated")
                    && out.contains("an algorithm's optional parameter, return or local"),
                "`{written}` is refused by name:\n{out}"
            );
        }
    }
}

// ── Each backend's spelling ──────────────────────────────────────────────

/// `goal_until` of the fixture directory — an optional day in, an optional count
/// of days out, an early return of `null` — spelled by each backend in its one
/// optional type, read through the emitter's spellings of a test and a proven
/// read.
#[test]
fn every_backend_spells_an_optional_parameter_and_return_in_its_optional_type() {
    let path = fixtures().join("algorithm_goal_until.scxml");
    for (lang, args) in every_backend() {
        let (ok, out, written) = run_on(&args, &path);
        assert!(ok, "{lang} generates goal_until:\n{out}");
        let needles: &[&str] = match lang {
            "rust" => &[
                "day: Option<i64>",
                "Result<Option<i64>,",
                "day.is_none()",
                "return Ok(None);",
                "sce_forge_runtime::optional::proven(day)",
            ],
            "kotlin" => &[
                "day: Long?",
                "AlgorithmResult<Long?>",
                "day == null",
                "com.sce.forge.runtime.SceOptional.proven(day)",
            ],
            "python" => &[
                "from typing import Optional",
                "day: Optional[int]",
                "-> Optional[int]:",
                "if day is None:",
                "return None",
            ],
            "c11" => &[
                "#include \"sce/forge/optional.h\"",
                "sce_optional_int64_t day",
                "sce_optional_int64_t value;",
                "!day.present",
                "sce_optional_int64_none()",
                "day.value",
            ],
            "cpp" => &[
                "#include <optional>",
                "#include <sce/forge/optional.h>",
                "std::optional<int64_t> day",
                "AlgorithmResult<std::optional<int64_t>>",
                "!day.has_value()",
                "(*day)",
            ],
            "go" => &[
                "\"github.com/newmassrael/sce-forge-runtime/optional\"",
                "day optional.Of[int64]",
                "!day.IsPresent()",
                "optional.None[int64]()",
                "day.Proven()",
            ],
            other => panic!("no needles for {other}"),
        };
        for needle in needles {
            assert!(
                written.contains(needle),
                "{lang} spells `{needle}`:\n{written}"
            );
        }
    }
}

/// `goal_earliest` and `goal_day_sum` of the fixture directory — a list of records
/// whose day may be absent in, an optional local, `??` in the body — are generated
/// on every backend: a record's optional field is read as an optional value, and
/// the local is declared in the optional type it starts absent in.
#[test]
fn every_backend_reads_a_records_optional_field_and_holds_an_optional_local() {
    for document in [
        "algorithm_goal_earliest.scxml",
        "algorithm_goal_day_sum.scxml",
    ] {
        let path = fixtures().join(document);
        for (lang, args) in every_backend() {
            let (ok, out, written) = run_on(&args, &path);
            assert!(ok, "{lang} generates {document}:\n{out}");
            if document != "algorithm_goal_earliest.scxml" {
                continue;
            }
            let local = match lang {
                "rust" => "let mut best: Option<i64> = None;",
                "kotlin" => "var best: Long? = null",
                "python" => "best: Optional[int] = None",
                "c11" => "sce_optional_int64_t best = sce_optional_int64_none();",
                "cpp" => "std::optional<int64_t> best = std::nullopt;",
                "go" => "var best optional.Of[int64] = optional.None[int64]()",
                other => panic!("no local for {other}"),
            };
            assert!(
                written.contains(local),
                "{lang} declares the optional local `{local}`:\n{written}"
            );
        }
    }
}

/// The unit an event schema of an optional day generates holds the record an
/// algorithm reads, in each backend's optional type.
#[test]
fn every_backend_generates_the_record_of_an_optional_day() {
    let path = fixtures().join("schema_goal.scxml");
    for (lang, args) in every_backend() {
        let (ok, out, written) = run_on(&args, &path);
        assert!(ok, "{lang} generates schema_goal:\n{out}");
        let field = match lang {
            "rust" => "pub day: Option<i64>,",
            "kotlin" => "val day: Long?",
            "python" => "day: Optional[int]",
            "c11" => "sce_optional_int64_t day;",
            "cpp" => "std::optional<int64_t> day;",
            "go" => "Day optional.Of[int64]",
            other => panic!("no field for {other}"),
        };
        assert!(
            written.contains(field),
            "{lang} holds `{field}`:\n{written}"
        );
    }
}
