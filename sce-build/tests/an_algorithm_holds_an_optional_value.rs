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
//! record is read on an algorithm's slots too (docs/adr/0055, step 9), and a
//! generated backend that does not spell one yet refuses it by name where it
//! renders it; the Interpreter's lowering spells all but the byte string.
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

/// An optional text, byte string, value of an enum or record is read on an
/// algorithm's parameter, return and local as a statechart's variable reads it
/// (docs/adr/0055, step 9): the reader admits every element a statechart holds
/// one of, and the model carries the type as written.
#[test]
fn an_algorithms_slot_reads_every_optional_element_a_statechart_holds() {
    use sce_build::forge::model::ForgeDocument;
    for (written, elem) in [
        (
            "optional&lt;string&gt;",
            OptionalElemType::Scalar(SceType::String),
        ),
        (
            "optional&lt;bytes&gt;",
            OptionalElemType::Scalar(SceType::Bytes),
        ),
        (
            "optional&lt;record:Goal&gt;",
            OptionalElemType::Record {
                alias: "Goal".to_string(),
            },
        ),
    ] {
        let imports = if written.contains("record:Goal") {
            "  <sce:import kind=\"event-schema\" src=\"schema_goal.scxml\" as=\"Goal\"/>\n"
        } else {
            ""
        };
        let text = slots(written, written, written)
            .replace("  <sce:signature>", &format!("{imports}  <sce:signature>"));
        let parsed = sce_build::forge::parser::parse_forge_with_imports(
            &text,
            sce_build::DocumentLabel::symmetric("slot_probe"),
        )
        .unwrap_or_else(|e| panic!("`{written}` is read on every slot: {e:?}"))
        .expect("an algorithm is a forge document");
        let ForgeDocument::Algorithm(algorithm) = &parsed.document else {
            panic!("expected an algorithm, got {:?}", parsed.document.kind());
        };
        let optional = AlgorithmValueType::Optional { elem };
        assert_eq!(
            algorithm.signature.params[0].sce_type, optional,
            "{written}"
        );
        assert_eq!(
            algorithm.signature.return_type.as_ref(),
            Some(&optional),
            "{written}"
        );
    }
}

/// The generated backends, by the language `generate -l` names, whose step 9 of
/// docs/adr/0055 spells an optional text, byte string, value of an enum and record
/// in an algorithm's slots: each of them generates what the others refuse. A backend
/// joins here in its step and keeps its other arms.
const SPELLING_EVERY_OPTIONAL_ELEMENT: &[&str] = &["python", "kotlin"];

/// Each generated backend refuses an algorithm that holds an optional value it
/// does not spell yet, by name, where it renders the slot or the local
/// (docs/adr/0055, step 9): `generate/unsupported-feature`, "an optional value has
/// no <language> lowering yet", on the parameter, the return and the local alike.
/// A number and a `bool` are spelled by every backend, so the other two slots of
/// each probe are of `int64`.
#[test]
fn every_backend_refuses_an_algorithms_optional_slot_it_does_not_spell_by_name() {
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
            // The body answers `null`, so each probe is judged fit whatever the
            // return is: the refusal is the backend's. A document imports what
            // its slots name and nothing else.
            let mut imports = String::new();
            if written.contains("enum:Status") {
                imports.push_str(
                    "  <sce:import kind=\"enum\" src=\"enum_item_status.scxml\" as=\"Status\"/>\n",
                );
            }
            if written.contains("record:Goal") {
                imports.push_str(
                    "  <sce:import kind=\"event-schema\" src=\"schema_goal.scxml\" as=\"Goal\"/>\n",
                );
            }
            let text = slots(param, ret, local)
                .replace("  <sce:signature>", &format!("{imports}  <sce:signature>"))
                .replace("<sce:return expr=\"held\"/>", "<sce:return expr=\"null\"/>");
            for (lang, args) in every_backend() {
                // C++ spells every element but a byte string, which an algorithm holds
                // as a view, a buffer or a vector by the slot (docs/adr/0055, step 9):
                // what it emits is held by `an_optional_record_is_spelled_by_the_cpp_backend`.
                if lang == "cpp" && !written.contains("bytes") {
                    continue;
                }
                let (ok, out, written_text) = run_probe(
                    &args,
                    &text,
                    "slot_probe.scxml",
                    &["enum_item_status.scxml", "schema_goal.scxml"],
                );
                // Python holds every element in `Optional[T]` over the type its
                // present value has there (docs/adr/0055, step 9): it spells each
                // slot, and the probe is generated.
                if SPELLING_EVERY_OPTIONAL_ELEMENT.contains(&lang) {
                    assert!(
                        ok,
                        "{lang} spells `{written}` on an algorithm's slot:\n{out}"
                    );
                    // Python's `Optional[T]`, Kotlin's `T?`.
                    let optional = if lang == "kotlin" { "?" } else { "Optional[" };
                    assert!(
                        written_text.contains(optional),
                        "{lang} holds `{written}` in its optional type:\n{written_text}"
                    );
                    continue;
                }
                assert!(
                    !ok,
                    "{lang} refuses `{written}` on an algorithm's slot:\n{out}"
                );
                assert!(
                    out.contains("generate/unsupported-feature")
                        && out.contains("an optional value has no"),
                    "{lang} refuses `{written}` by name:\n{out}"
                );
            }
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

// ── A statechart's call ──────────────────────────────────────────────────

/// `sce-codegen check -l <lang>` of `path` for each of the six generated backends,
/// and `sce-codegen lower` of it for the Interpreter: the seven targets a scenario
/// is replayed on. Each exits 0, or the test names the target and its refusal.
fn every_target_lowers(path: &Path) {
    for (lang, args) in every_backend() {
        let mut checked = args.clone();
        checked[0] = "check";
        let (ok, out, _) = run_on(&checked, path);
        assert!(ok, "{lang} lowers {}:\n{out}", path.display());
    }
    let (ok, out, _) = run_on(&["lower"], path);
    assert!(
        ok,
        "the Interpreter's lowering lowers {}:\n{out}",
        path.display()
    );
}

/// The fixture machine of `static_state_algorithm_optional.json` hands the
/// algorithms a list of records whose day may be absent, an optional variable,
/// `null` and a day of `int64`, and takes optional days back: it is checked fit
/// on all seven targets, or no regeneration generates it and no driver replays it.
///
/// ⚠ It was not (measured 2026-10-22, integ-6): `check -l rust` exited 7 with
/// "`Earliest(goals)` has no Rust lowering: optional<int64> ... where int64 is
/// expected". The assignment to an optional variable typed its value as the
/// author wrote it, where the alias of an algorithm that takes a list names no
/// function — only the call it is rewritten to does — so the optional day the
/// call answers was taken for a present one and lowered into an `int64`.
#[test]
fn the_optional_scenario_machine_is_lowered_on_all_seven_targets() {
    every_target_lowers(&fixtures().join("static_state_algorithm_optional.scxml"));
}

/// A call of an algorithm that takes a list is typed as the call it is lowered
/// as wherever the lowering asks a value its type: the optional day it answers,
/// assigned to an optional variable and logged.
#[test]
fn a_call_that_hands_over_a_list_is_typed_as_it_is_lowered() {
    let dir = tempdir().expect("tempdir");
    for name in ["schema_goal.scxml", "algorithm_goal_earliest.scxml"] {
        std::fs::copy(fixtures().join(name), dir.path().join(name)).expect("copy a sibling");
    }
    let probe = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="earliest_probe">
  <sce:import kind="event-schema" src="schema_goal.scxml" as="Goal"/>
  <sce:import kind="algorithm" src="algorithm_goal_earliest.scxml" as="Earliest"/>
  <datamodel>
    <data id="goals" sce:type="list&lt;record:Goal&gt;" sce:capacity="4"/>
    <data id="first" sce:type="optional&lt;int64&gt;" expr="null"/>
  </datamodel>
  <state id="idle">
    <transition event="go" type="internal">
      <assign location="first" expr="Earliest(goals)"/>
      <log label="earliest" expr="Earliest(goals)"/>
    </transition>
  </state>
</scxml>
"#;
    std::fs::write(dir.path().join("earliest_probe.scxml"), probe).expect("write the probe");
    every_target_lowers(&dir.path().join("earliest_probe.scxml"));
}

/// The fixture machine's Rust adapter answers the algorithm's optional day.
#[test]
fn the_rust_adapter_answers_the_algorithms_optional_day() {
    let path = fixtures().join("static_state_algorithm_optional.scxml");
    // Rust's adapter of the call that hands over the list takes the machine's
    // records and answers the algorithm's optional day as it is.
    let (ok, out, written) = run_on(&["generate", "-l", "rust"], &path);
    assert!(ok, "Rust generates the machine:\n{out}");
    assert!(
        written.contains("fn call_Earliest__v(") && written.contains(") -> Option<i64> {"),
        "the adapter answers the optional day:\n{written}"
    );
}

/// The Interpreter lowers the machine with each algorithm beside it: a proven
/// read is the value, `??` the library's `otherwise`, and `null` the absent value.
#[test]
fn the_interpreter_lowers_an_algorithm_that_holds_an_optional_value() {
    let path = fixtures().join("static_state_algorithm_optional.scxml");
    let (ok, out, written) = run_on(&["lower"], &path);
    assert!(ok, "the ecmascript lowering lowers the machine:\n{out}");
    for needle in ["if (day === null) { return null; }", "SceStatic.otherwise("] {
        assert!(
            written.contains(needle),
            "the lowered document holds `{needle}`:\n{written}"
        );
    }
}

/// An algorithm with a parameter of `int64`.
const PLUS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="plus_one" version="1.0">
  <sce:signature>
    <sce:param name="day" type="int64"/>
    <sce:return type="int64" may-fail="true"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="day + 1"/>
  </sce:body>
</scxml>
"#;

/// A statechart that imports `goal_until` as `Until` and [`PLUS`] as `Plus`, holds
/// optional days `first` and `left` and a day `days`, and assigns `days` and
/// `left` on `go` from `expr` and `left_expr`.
fn caller(expr: &str, left_expr: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="caller_probe">
  <sce:import kind="algorithm" src="algorithm_goal_until.scxml" as="Until"/>
  <sce:import kind="algorithm" src="plus_one.scxml" as="Plus"/>
  <datamodel>
    <data id="first" sce:type="optional&lt;int64&gt;" expr="null"/>
    <data id="left" sce:type="optional&lt;int64&gt;" expr="null"/>
    <data id="days" sce:type="int64" expr="0"/>
  </datamodel>
  <state id="idle">
    <transition event="go" type="internal">
      <assign location="days" expr="{expr}"/>
      <assign location="left" expr="{left_expr}"/>
    </transition>
  </state>
</scxml>
"#
    )
}

/// `sce-codegen check -l rust` of the statechart [`caller`], with `goal_until`
/// and [`PLUS`] beside it: `(exit_ok, stderr + stdout)`.
fn check_caller(expr: &str, left_expr: &str) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    std::fs::copy(
        fixtures().join("algorithm_goal_until.scxml"),
        dir.path().join("algorithm_goal_until.scxml"),
    )
    .expect("copy the algorithm");
    std::fs::write(dir.path().join("plus_one.scxml"), PLUS).expect("write the algorithm");
    std::fs::write(dir.path().join("caller.scxml"), caller(expr, left_expr))
        .expect("write the probe");
    let (ok, out, _) = run_on(&["check", "-l", "rust"], &dir.path().join("caller.scxml"));
    (ok, out)
}

/// A value of `T` and `null` stand where an optional parameter is declared, and
/// an optional value where an `int64` is declared is refused, naming the reads
/// that make it present: the rule an expression keeps everywhere else.
#[test]
fn an_optional_argument_stands_only_where_an_optional_parameter_is_declared() {
    let (ok, out) = check_caller("Plus(first ?? 0)", "Until(days, first)");
    assert!(
        ok,
        "an optional value, read with a default, and one handed whole:\n{out}"
    );
    let (ok, out) = check_caller("Plus(0)", "Until(days, 5)");
    assert!(
        ok,
        "a value of int64 is lifted into the optional parameter:\n{out}"
    );
    let (ok, out) = check_caller("Plus(0)", "Until(days, null)");
    assert!(ok, "`null` stands for the optional parameter:\n{out}");
    let (ok, out) = check_caller("Plus(first)", "null");
    assert!(
        !ok,
        "an optional value where an int64 is declared is refused:\n{out}"
    );
    assert!(
        out.contains("expression/type-mismatch") && out.contains("may be absent"),
        "refused naming the reads that make it present:\n{out}"
    );
}
