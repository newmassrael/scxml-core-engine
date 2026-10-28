// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A `sce-static` statechart calls an imported algorithm whose slot is
//! `optional<enum:X>` (docs/adr/0055, steps 8 and 9): the call is named where the
//! import is read, the enum judge holds each argument to the enum document of the
//! slot, and every backend that spells the element in an algorithm's slot hands the
//! value over and takes it back by the declared name of its variant, where it is
//! present, as it crosses for a value of the enum and for a record's optional enum
//! field. A byte string that may be absent in a slot stays refused where the import is
//! read, since the byte string an algorithm holds is not the one a statechart's
//! variable holds.
//!
//! # What was wrong
//!
//! Measured in the code on 2026-10-28: `value_signature` named the slot for every
//! backend and Kotlin's adapter crossed it, while Rust's adapter refused the call with
//! "takes or returns a record or a list", and the adapters of C++, Go and Python handed
//! the machine's own enum to the algorithm's parameter as it was, a value of a type the
//! algorithm does not hold.

use std::path::{Path, PathBuf};
use std::process::Command;

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

/// An algorithm that takes and answers an optional value of the enum
/// `enum_item_status.scxml`.
const STATUS_NEXT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="status_next" version="1.0">
  <sce:import kind="enum" src="enum_item_status.scxml" as="Status"/>
  <sce:signature>
    <sce:param name="given" type="optional&lt;enum:Status&gt;"/>
    <sce:return type="optional&lt;enum:Status&gt;"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="given"/>
  </sce:body>
</scxml>
"#;

/// A statechart that holds an optional value of the enum and hands it to the algorithm
/// on `go`, taking the answer back into the same variable.
const MACHINE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="optional_enum_call">
  <sce:import kind="enum" src="enum_item_status.scxml" as="Status"/>
  <sce:import kind="algorithm" src="algorithm_status_next.scxml" as="Next"/>
  <datamodel>
    <data id="status" sce:type="optional&lt;enum:Status&gt;" expr="null"/>
  </datamodel>
  <state id="idle">
    <transition event="go" type="internal">
      <assign location="status" expr="Next(status)"/>
    </transition>
  </state>
</scxml>
"#;

/// `sce-codegen <args…> <machine>` in a directory that holds the enum, the algorithm
/// and the machine `machine` (the text of `optional_enum_call.scxml`): `(exit_ok,
/// stderr + stdout, the text of every file written)`.
fn run(args: &[&str], algorithm: &str, machine: &str) -> (bool, String, String) {
    let dir = tempdir().expect("tempdir");
    std::fs::copy(
        fixtures().join("enum_item_status.scxml"),
        dir.path().join("enum_item_status.scxml"),
    )
    .expect("copy the enum");
    std::fs::write(dir.path().join("algorithm_status_next.scxml"), algorithm)
        .expect("write the algorithm");
    std::fs::write(dir.path().join("optional_enum_call.scxml"), machine)
        .expect("write the machine");
    let out_dir = tempdir().expect("tempdir");
    let mut command = Command::new(sce_codegen_bin());
    command
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(dir.path().join("optional_enum_call.scxml"));
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

/// Each of `needles` is in `written`, the text one backend generated.
fn holds_each(lang: &str, written: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            written.contains(needle),
            "{lang} writes `{needle}`:\n{written}"
        );
    }
}

/// Rust's adapter takes the machine's `Option` over its own enum and maps it into the
/// algorithm's where it is present, and maps the answer back.
#[test]
fn rust_crosses_an_optional_enum_slot_by_the_declared_name_of_the_variant() {
    let (ok, out, written) = run(&["generate", "-l", "rust"], STATUS_NEXT, MACHINE);
    assert!(ok, "Rust generates the machine:\n{out}");
    holds_each(
        "Rust",
        &written,
        &[
            "fn call_Next__(",
            "p0: Option<OptionalEnumCallStatusEnum>",
            "match p0 {",
            "match r {",
            "None => None",
        ],
    );
}

/// C++'s adapter takes the machine's `std::optional` over its own enum and maps it,
/// where it is present, by the runtime's `mapPresent`, in the parameter and the answer.
#[test]
fn cpp_crosses_an_optional_enum_slot_by_the_declared_name_of_the_variant() {
    let (ok, out, written) = run(&["generate", "-l", "cpp"], STATUS_NEXT, MACHINE);
    assert!(ok, "C++ generates the machine:\n{out}");
    holds_each(
        "C++",
        &written,
        &[
            "call_Next__(",
            "std::optional<OptionalEnumCallStatusEnum> p0",
            "mapPresent(p0",
            "mapPresent(r",
        ],
    );
}

/// Go's adapter converts the machine's `optional.Of` over its own enum into the
/// algorithm's by the package's `Convert`, in the parameter and the answer.
#[test]
fn go_crosses_an_optional_enum_slot_by_the_declared_name_of_the_variant() {
    let (ok, out, written) = run(
        &[
            "generate",
            "-l",
            "go",
            "--go-module-prefix",
            "example.test/generated",
        ],
        STATUS_NEXT,
        MACHINE,
    );
    assert!(ok, "Go generates the machine:\n{out}");
    holds_each(
        "Go",
        &written,
        &[
            "Next__(",
            "optional.Of[OptionalEnumCallStatusEnum]",
            "optional.Convert(p0",
            "optional.Convert(r",
        ],
    );
}

/// Python's adapter maps the machine's enum into the algorithm's where it is present,
/// and `None` stays `None`, in the parameter and the answer.
#[test]
fn python_crosses_an_optional_enum_slot_by_the_declared_name_of_the_variant() {
    let (ok, out, written) = run(&["generate", "-l", "python"], STATUS_NEXT, MACHINE);
    assert!(ok, "Python generates the machine:\n{out}");
    holds_each(
        "Python",
        &written,
        &[
            "def _call_Next__(p0):",
            "lambda sce_present: None if sce_present is None else",
            ")(p0)",
            ")(r)",
        ],
    );
}

/// C11 holds the enum in the type its document declares on both sides, so the optional
/// struct over it is the algorithm's own and the adapter hands it over as it is.
#[test]
fn c11_hands_an_optional_enum_slot_over_in_the_struct_of_its_document() {
    let (ok, out, written) = run(&["generate", "-l", "c11"], STATUS_NEXT, MACHINE);
    assert!(ok, "C11 generates the machine:\n{out}");
    holds_each(
        "C11",
        &written,
        &["call_Next__(", "sce_optional_enum_enum_item_status_t p0"],
    );
}

/// Kotlin's adapter, which crossed the slot first, is unchanged: the machine's `T?`
/// over its own enum class, mapped where it is present.
#[test]
fn kotlin_still_crosses_an_optional_enum_slot_by_the_declared_name_of_the_variant() {
    let (ok, out, written) = run(&["generate", "-l", "kotlin"], STATUS_NEXT, MACHINE);
    assert!(ok, "Kotlin generates the machine:\n{out}");
    holds_each(
        "Kotlin",
        &written,
        &[
            "private fun call_Next__(p0: OptionalEnumCallStatusEnum?)",
            "?.let",
        ],
    );
}

/// The Interpreter calls the algorithm itself, with the value its engine holds.
#[test]
fn the_interpreter_lowers_a_call_whose_slot_is_an_optional_enum() {
    let (ok, out, written) = run(&["lower"], STATUS_NEXT, MACHINE);
    assert!(ok, "the ecmascript lowering lowers the machine:\n{out}");
    assert!(
        written.contains("SceStatic.algorithms.status_next("),
        "the call is the algorithm's own:\n{written}"
    );
}

/// An optional byte string in a slot stays refused where the import is read, on every
/// backend, by the wording the import gives it.
#[test]
fn an_optional_byte_string_slot_stays_refused_where_the_import_is_read() {
    let algorithm = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="status_next" version="1.0">
  <sce:signature>
    <sce:param name="given" type="optional&lt;bytes&gt;"/>
    <sce:return type="int64"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="0"/>
  </sce:body>
</scxml>
"#;
    let machine = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="optional_enum_call">
  <sce:import kind="algorithm" src="algorithm_status_next.scxml" as="Next"/>
  <datamodel>
    <data id="count" sce:type="int64" expr="0"/>
    <data id="raw" sce:type="optional&lt;bytes&gt;" sce:capacity="8" expr="null"/>
  </datamodel>
  <state id="idle">
    <transition event="go" type="internal">
      <assign location="count" expr="Next(raw)"/>
    </transition>
  </state>
</scxml>
"#;
    for lang in ["rust", "kotlin", "python", "c11", "cpp", "go"] {
        let (ok, out, _) = run(&["check", "-l", lang], algorithm, machine);
        assert!(!ok, "{lang} refuses the call:\n{out}");
        // The expression layer refuses the argument first, naming the type: an
        // optional byte string is no value of a slot an algorithm holds.
        assert!(
            out.contains("optional<bytes>"),
            "{lang} refuses it naming the type:\n{out}"
        );
    }
}
