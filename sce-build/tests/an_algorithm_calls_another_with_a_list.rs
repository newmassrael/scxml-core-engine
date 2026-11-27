// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! An algorithm calls another algorithm that takes or returns a list, and takes
//! a record whose schema holds a bounded list (SCE_FORGE.md section 4.12,
//! "Where a list crosses" and "A record that holds a list"; docs/adr/0086,
//! section 7.3), on the backends that lend a list across: Rust.
//!
//! * `probe_caller` takes `record:Order`, whose `lines` is a
//!   `list<uint16>[4]`, lent by reference in the type the schema's unit
//!   declares; it lends `order.lines`, its own list parameter and its list
//!   local to `probe_sum`, which takes `list<uint16>`, in an expression and with
//!   `<sce:call>`, walks `order.lines` with a `<sce:foreach>`, and takes whole
//!   into its list local the `list<uint16>[3]` `probe_tail` returns.
//! * The four units compile under `rustc -D warnings`, and a program built of
//!   them runs the caller and checks the list it answers.
//! * A list that does not fit the parameter or the local, a returned list taken
//!   where no failure is received or by a `<sce:call>`, and a record that holds
//!   a list anywhere but a parameter are refused by name; every other backend
//!   refuses the caller by name.
//!
//! What was wrong: an algorithm that takes or returns a list was called by a
//! host alone, and a record whose schema holds a list was refused by every
//! generated lane where an algorithm named it, though `check` accepted it.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::codegen_matrix::emitting_in_part;
use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, Located};
use sce_build::forge::model::ForgeKind;
use sce_build::generator::{GeneratedOutput, Language};
use sce_build::ForgeCompileOptions;

/// An order: a number and a list of at most four numbers.
const ORDER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="probe_order" sce:event-name="probe.order">
  <datamodel>
    <data id="id" sce:type="uint32" sce:direction="in"/>
    <data id="lines" sce:type="list&lt;uint16&gt;[4]" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// The sum of a list of any length.
const SUM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="probe_sum" version="1.0">
  <sce:signature>
    <sce:param name="xs" type="list&lt;uint16&gt;"/>
    <sce:return type="uint32" may-fail="true"/>
  </sce:signature>
  <sce:body>
    <sce:var name="total" type="uint32" init="0"/>
    <sce:foreach item="x" in="xs">
      <sce:assign target="total" expr="total + x"/>
    </sce:foreach>
    <sce:return expr="total"/>
  </sce:body>
</scxml>
"#;

/// The count of a list of at most two: a parameter that states its bound.
const PAIR: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="probe_pair" version="1.0">
  <sce:signature>
    <sce:param name="xs" type="list&lt;uint16&gt;[2]"/>
    <sce:return type="uint32" may-fail="true"/>
  </sce:signature>
  <sce:body>
    <sce:var name="n" type="uint32" init="0"/>
    <sce:foreach item="x" in="xs">
      <sce:assign target="n" expr="n + 1"/>
    </sce:foreach>
    <sce:return expr="n"/>
  </sce:body>
</scxml>
"#;

/// The first three elements of a list of any length.
const TAIL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="probe_tail" version="1.0">
  <sce:signature>
    <sce:param name="xs" type="list&lt;uint16&gt;"/>
    <sce:return type="list&lt;uint16&gt;[3]" may-fail="true"/>
  </sce:signature>
  <sce:body>
    <sce:var name="out" type="list&lt;uint16&gt;[3]"/>
    <sce:foreach item="x" in="xs">
      <sce:if cond="len(out) &lt; 3">
        <sce:append target="out" expr="x"/>
      </sce:if>
    </sce:foreach>
    <sce:return expr="out"/>
  </sce:body>
</scxml>
"#;

/// The caller: every way a list crosses, over the order it is lent.
const CALLER_BODY: &str = r#"    <sce:var name="kept" type="list&lt;uint16&gt;[4]"/>
    <sce:var name="total" type="uint32" init="Sum(order.lines)"/>
    <sce:foreach item="line" in="order.lines">
      <sce:assign target="total" expr="total + line"/>
    </sce:foreach>
    <sce:call target="Sum" args="extra"/>
    <sce:assign target="kept" expr="Tail(order.lines)"/>
    <sce:if cond="total &gt; Sum(kept) &amp;&amp; len(order.lines) &gt; 3">
      <sce:append target="kept" expr="1"/>
    </sce:if>
    <sce:return expr="kept"/>"#;

/// An algorithm `probe_caller` over the fixtures, whose body is `body`, that
/// declares `may-fail` when `may_fail`.
fn caller(body: &str, may_fail: bool) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="probe_caller" version="1.0">
  <sce:import kind="event-schema" src="probe_order.scxml" as="Order"/>
  <sce:import kind="algorithm" src="probe_sum.scxml" as="Sum"/>
  <sce:import kind="algorithm" src="probe_tail.scxml" as="Tail"/>
  <sce:signature>
    <sce:param name="order" type="record:Order"/>
    <sce:param name="extra" type="list&lt;uint16&gt;"/>
    <sce:return type="list&lt;uint16&gt;[4]" may-fail="{may_fail}"/>
  </sce:signature>
  <sce:body>
{body}
  </sce:body>
</scxml>
"#
    )
}

/// The fixtures and the caller written into one directory, and the caller's
/// path.
fn fixture(caller_text: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("a scratch directory");
    for (name, text) in [
        ("probe_order.scxml", ORDER),
        ("probe_sum.scxml", SUM),
        ("probe_pair.scxml", PAIR),
        ("probe_tail.scxml", TAIL),
        ("probe_caller.scxml", caller_text),
    ] {
        std::fs::write(dir.path().join(name), text).expect("write a document");
    }
    let path = dir.path().join("probe_caller.scxml");
    (dir, path)
}

/// The options every backend is asked with: Go is handed the module path its
/// imports are under, which every other backend ignores.
fn options() -> ForgeCompileOptions {
    ForgeCompileOptions {
        go_module_prefix: Some("example.com/generated".to_string()),
        ..ForgeCompileOptions::default()
    }
}

fn generated(path: &Path, language: Language) -> Result<GeneratedOutput, Located<ForgeError>> {
    sce_build::compile_forge_file(path, language, &[], &options())
}

/// The unit `language` writes for the document at `path`, named for its stem.
fn unit(path: &Path, language: Language) -> (String, String) {
    let output = generated(path, language)
        .unwrap_or_else(|e| panic!("{} does not generate: {}", path.display(), e.error));
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .expect("a utf-8 stem");
    output
        .files
        .into_iter()
        .find(|(file, _)| file.strip_suffix(".rs") == Some(stem))
        .unwrap_or_else(|| panic!("{}: no unit named for it", path.display()))
}

/// The code and the text of the refusal `result` carries.
fn refused(result: Result<GeneratedOutput, Located<ForgeError>>) -> (DiagnosticCode, String) {
    let err = match result {
        Ok(_) => panic!("the document generated"),
        Err(refusal) => refusal,
    };
    let record = err.error.to_diagnostics().remove(0);
    (record.code, err.error.to_string())
}

/// What the Rust caller holds. Pinned by `record_param_type` (the record lent by
/// reference), `handed_list` (a parameter as the slice it is, a local and a
/// record's list field as the slice they hold), the list field renamed in
/// `render_algorithm`, the list arm of `CallTarget::list_parameter` (the
/// statement form) and `lower_whole_list_call` (the returned list copied out of
/// a destination of the callee's type), each in `sce-build/src/forge`.
#[test]
fn a_rust_caller_lends_and_takes_lists_and_reads_a_record_that_holds_one() {
    let (_dir, path) = fixture(&caller(CALLER_BODY, true));
    let (_, text) = unit(&path, Language::Rust);
    for expected in [
        "order: &",
        "extra: &[u16]",
        "kept: &mut SceOwnedList<u16, 4>",
        "(order.lines.as_slice())?",
        "for &line in order.lines.iter() {",
        "(extra)?;",
        "let sce_taken_kept = {",
        "let mut sce_dest = sce_portable_bytes::SceOwnedList::new();",
        "(order.lines.as_slice(), &mut sce_dest)?;",
        "kept.clear();",
        "kept.extend_from_slice(&sce_taken_kept)?;",
        "(kept.as_slice())?",
        "order.lines.len()",
    ] {
        assert!(
            text.contains(expected),
            "the Rust caller holds `{expected}`:\n{text}"
        );
    }
}

/// The four units beside each other, compiled with every warning an error, and
/// a program of them that runs the caller over an order of four lines and an
/// extra list: the sum of the lines is 10, walked again to 20; the first three
/// lines are taken whole into the local, whose sum is 6, and 20 > 6 with four
/// lines appends a 1. Pinned as the text test is, and by the types each spelling
/// must agree with: a record passed by value, a list local handed as it is, or
/// a destination of another bound does not compile.
#[test]
fn the_rust_of_a_list_lent_and_taken_compiles_and_runs() {
    let Some(rustc) = sce_build::toolchain::require_any_or_skip(
        &["rustc"],
        "compile and run generated algorithms that hand lists across",
    ) else {
        return;
    };
    let (dir, _) = fixture(&caller(CALLER_BODY, true));
    let crate_dir = tempfile::tempdir().expect("a scratch directory");
    let mut modules = String::new();
    for name in [
        "probe_order.scxml",
        "probe_sum.scxml",
        "probe_tail.scxml",
        "probe_caller.scxml",
    ] {
        let (file, text) = unit(&dir.path().join(name), Language::Rust);
        std::fs::write(crate_dir.path().join(&file), text).expect("write the unit");
        modules.push_str(&format!("pub mod {};\n", file.trim_end_matches(".rs")));
    }
    let runtime = tempfile::tempdir().expect("a scratch directory for the runtime");
    let flags = common::rust_runtime::rustc_flags(runtime.path());

    let lib = crate_dir.path().join("lib.rs");
    std::fs::write(&lib, &modules).expect("write the crate root");
    let checked = Command::new(&rustc)
        .args([
            "--edition=2021",
            "--crate-type=lib",
            "--emit=metadata",
            "-D",
            "warnings",
            "--out-dir",
        ])
        .arg(crate_dir.path().join("out"))
        .arg(&lib)
        .args(&flags)
        .output()
        .expect("rustc runs");
    assert!(
        checked.status.success(),
        "the generated Rust does not compile under -D warnings:\n{}",
        String::from_utf8_lossy(&checked.stderr)
    );

    // A program reads a few items of each unit, so it is built without the
    // lib's deny of warnings: an item no program reads is dead code there.
    let main = crate_dir.path().join("main.rs");
    std::fs::write(
        &main,
        format!(
            "{modules}\n\
             fn main() {{\n\
             \x20   let order = probe_order::ProbeOrderPayload {{\n\
             \x20       id: 7,\n\
             \x20       lines: sce_portable_bytes::BoundedList::from_slice(&[1u16, 2, 3, 4])\n\
             \x20           .expect(\"four lines fit\"),\n\
             \x20   }};\n\
             \x20   let mut kept = sce_portable_bytes::SceOwnedList::<u16, 4>::new();\n\
             \x20   probe_caller::probe_caller(&order, &[5u16, 6], &mut kept)\n\
             \x20       .expect(\"the caller builds its list\");\n\
             \x20   assert_eq!(kept.as_slice(), &[1u16, 2, 3, 1][..]);\n\
             \x20   assert_eq!(order.id, 7);\n\
             }}\n"
        ),
    )
    .expect("write the program");
    let program = crate_dir.path().join("probe");
    let built = Command::new(&rustc)
        .args(["--edition=2021", "--crate-type=bin", "-o"])
        .arg(&program)
        .arg(&main)
        .args(&flags)
        .output()
        .expect("rustc runs");
    assert!(
        built.status.success(),
        "the program of the generated units does not build:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let ran = Command::new(&program).output().expect("the program runs");
    assert!(
        ran.status.success(),
        "the caller does not answer the list it should:\n{}",
        String::from_utf8_lossy(&ran.stderr)
    );
}

/// Every other backend that generates algorithms refuses the caller by name:
/// its record holds a list, which no algorithm of it holds yet, named with the
/// field and the backend that does. Pinned by `listed_record_unlowered` and its
/// gate on `ALGORITHM_LISTED_RECORD_BACKENDS` (`sce-build/src/lib.rs`).
#[test]
fn every_other_backend_refuses_a_record_that_holds_a_list_by_name() {
    let reader = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="probe_reader" version="1.0">
  <sce:import kind="event-schema" src="probe_order.scxml" as="Order"/>
  <sce:signature>
    <sce:param name="order" type="record:Order"/>
    <sce:return type="uint32"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="order.id"/>
  </sce:body>
</scxml>
"#;
    let (dir, _) = fixture(&caller(CALLER_BODY, true));
    let path = dir.path().join("probe_reader.scxml");
    std::fs::write(&path, reader).expect("write the document");
    assert!(
        generated(&path, Language::Rust).is_ok(),
        "Rust takes the record as a parameter"
    );
    for language in emitting_in_part(ForgeKind::Algorithm) {
        if language == Language::Rust {
            continue;
        }
        let (code, why) = refused(generated(&path, language));
        assert!(
            matches!(code, DiagnosticCode::GenerateUnsupportedFeature),
            "{language:?}: {code:?}: {why}"
        );
        for piece in ["`lines`", "holds a list", "rust"] {
            assert!(why.contains(piece), "{language:?} names {piece}: {why}");
        }
    }
}

/// Every other backend refuses a call that lends a list, naming the slot. Pinned
/// by the list arm of `algorithm_record_slots` and its gate on
/// `ALGORITHM_LIST_CALL_BACKENDS` (`sce-build/src/lib.rs`).
#[test]
fn every_other_backend_refuses_a_call_that_lends_a_list_by_name() {
    let body = r#"    <sce:var name="kept" type="list&lt;uint16&gt;[4]"/>
    <sce:var name="total" type="uint32" init="Sum(extra)"/>
    <sce:if cond="total &gt; 3">
      <sce:append target="kept" expr="1"/>
    </sce:if>
    <sce:return expr="kept"/>"#;
    let text = caller(body, true).replace(
        r#"  <sce:import kind="event-schema" src="probe_order.scxml" as="Order"/>
"#,
        "",
    );
    let text = text.replace(
        r#"    <sce:param name="order" type="record:Order"/>
"#,
        "",
    );
    let (_dir, path) = fixture(&text);
    for language in emitting_in_part(ForgeKind::Algorithm) {
        let result = generated(&path, language);
        if language == Language::Rust {
            assert!(result.is_ok(), "Rust lends the list: {:?}", result.err());
            continue;
        }
        // A backend that lowers no part of the caller refuses it by name for
        // that part; one that lowers the rest refuses the call, naming the
        // list slot no algorithm of it fills.
        let (code, why) = refused(result);
        match code {
            DiagnosticCode::ExpressionUnsupportedConstruct => assert!(
                why.contains("takes list<uint16> `xs`") && why.contains("on Rust alone"),
                "{language:?}: {why}"
            ),
            DiagnosticCode::GenerateUnsupportedFeature => {}
            other => panic!("{language:?}: {other:?}: {why}"),
        }
    }
}

/// The caller's refusal on Rust, for the body `body`, with the algorithm
/// `probe_pair` imported as `Pair` beside the others.
fn rust_refusal(body: &str, may_fail: bool) -> (DiagnosticCode, String) {
    let text = caller(body, may_fail).replace(
        "  <sce:signature>",
        "  <sce:import kind=\"algorithm\" src=\"probe_pair.scxml\" as=\"Pair\"/>\n  <sce:signature>",
    );
    let (_dir, path) = fixture(&text);
    refused(generated(&path, Language::Rust))
}

/// A list that may hold more than the parameter's bound is refused where it is
/// lent. Pinned by the bound of the relation "fits" read in
/// `judge_list_argument` (`sce-build/src/forge/expr.rs`): an unbounded list
/// parameter is held at the most any list holds, which `list<uint16>[2]` does
/// not.
#[test]
fn a_list_that_may_pass_the_parameters_bound_is_refused() {
    let (code, why) = rust_refusal(
        r#"    <sce:var name="kept" type="list&lt;uint16&gt;[4]"/>
    <sce:var name="n" type="uint32" init="Pair(extra)"/>
    <sce:if cond="n &gt; 1">
      <sce:append target="kept" expr="1"/>
    </sce:if>
    <sce:return expr="kept"/>"#,
        true,
    );
    assert!(
        matches!(code, DiagnosticCode::ExpressionUnsupportedConstruct),
        "{code:?}: {why}"
    );
    assert!(
        why.contains("may hold more than") && why.contains("list<uint16>[2]"),
        "{why}"
    );
}

/// A list of another element is refused where it is lent, and so is anything
/// that is no list the caller holds. Pinned by the element of the relation
/// "fits" and the unnamed arm of `judge_list_argument`.
#[test]
fn what_is_no_list_of_the_parameters_type_is_refused() {
    let (code, why) = rust_refusal(
        r#"    <sce:var name="kept" type="list&lt;uint16&gt;[4]"/>
    <sce:var name="wide" type="list&lt;uint32&gt;[2]"/>
    <sce:append target="wide" expr="1"/>
    <sce:var name="n" type="uint32" init="Sum(wide)"/>
    <sce:if cond="n &gt; 1">
      <sce:append target="kept" expr="1"/>
    </sce:if>
    <sce:return expr="kept"/>"#,
        true,
    );
    assert!(
        matches!(code, DiagnosticCode::ExpressionUnsupportedConstruct),
        "{code:?}: {why}"
    );
    assert!(
        why.contains("is a uint32 where a uint16 is expected"),
        "{why}"
    );
    let (code, why) = rust_refusal(
        r#"    <sce:var name="kept" type="list&lt;uint16&gt;[4]"/>
    <sce:var name="n" type="uint32" init="Sum(order.id)"/>
    <sce:if cond="n &gt; 1">
      <sce:append target="kept" expr="1"/>
    </sce:if>
    <sce:return expr="kept"/>"#,
        true,
    );
    assert!(
        matches!(code, DiagnosticCode::ExpressionUnsupportedConstruct),
        "{code:?}: {why}"
    );
    assert!(
        why.contains("`order.id`") && why.contains("handed to `Sum`"),
        "{why}"
    );
}

/// A returned list is taken whole only by an `<sce:assign>` to a list local
/// whose type it fits, in a body that receives its failure. Pinned by
/// `lower_whole_list_call` (the bound and the failure), the list-return arm of
/// `algorithm_call_target` (the statement) and the host-only signature a
/// list-returning callee is registered under in `AlgorithmTypes::type_ctx`
/// (anywhere else), each in `sce-build/src/forge/generator.rs`.
#[test]
fn a_returned_list_is_taken_whole_into_a_local_it_fits_and_nowhere_else() {
    // A local of a smaller bound.
    let (code, why) = rust_refusal(
        r#"    <sce:var name="two" type="list&lt;uint16&gt;[2]"/>
    <sce:assign target="two" expr="Tail(extra)"/>
    <sce:var name="kept" type="list&lt;uint16&gt;[4]"/>
    <sce:var name="n" type="uint32" init="Sum(two)"/>
    <sce:if cond="n &gt; 1">
      <sce:append target="kept" expr="1"/>
    </sce:if>
    <sce:return expr="kept"/>"#,
        true,
    );
    assert!(
        matches!(code, DiagnosticCode::ExpressionUnsupportedConstruct),
        "{code:?}: {why}"
    );
    assert!(why.contains("may hold more than"), "{why}");
    // A body that receives no failure.
    let (code, why) = rust_refusal(
        r#"    <sce:var name="kept" type="list&lt;uint16&gt;[4]"/>
    <sce:assign target="kept" expr="Tail(extra)"/>
    <sce:return expr="kept"/>"#,
        false,
    );
    assert!(
        matches!(code, DiagnosticCode::ExpressionUnsupportedConstruct),
        "{code:?}: {why}"
    );
    assert!(why.contains("does not declare may-fail"), "{why}");
    // A statement that keeps no value.
    let (code, why) = rust_refusal(
        r#"    <sce:var name="kept" type="list&lt;uint16&gt;[4]"/>
    <sce:call target="Tail" args="extra"/>
    <sce:return expr="kept"/>"#,
        true,
    );
    assert!(
        matches!(code, DiagnosticCode::ExpressionUnsupportedConstruct),
        "{code:?}: {why}"
    );
    assert!(
        why.contains("returns list<uint16>[3]") && why.contains("<sce:assign>"),
        "{why}"
    );
    // An operand.
    let (code, why) = rust_refusal(
        r#"    <sce:var name="kept" type="list&lt;uint16&gt;[4]"/>
    <sce:var name="n" type="uint32" init="Sum(Tail(extra))"/>
    <sce:if cond="n &gt; 1">
      <sce:append target="kept" expr="1"/>
    </sce:if>
    <sce:return expr="kept"/>"#,
        true,
    );
    assert!(
        matches!(code, DiagnosticCode::ExpressionUnsupportedConstruct),
        "{code:?}: {why}"
    );
    assert!(why.contains("returns list<uint16>[3]"), "{why}");
}

/// A record that holds a list is a parameter alone: an algorithm that returns
/// one is refused by name, naming the field. Pinned by the list check of
/// `resolve_record` (`sce-build/src/forge/generator.rs`), which every place
/// but a parameter resolves its record through.
#[test]
fn a_record_that_holds_a_list_is_refused_anywhere_but_a_parameter() {
    let echo = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="probe_echo" version="1.0">
  <sce:import kind="event-schema" src="probe_order.scxml" as="Order"/>
  <sce:signature>
    <sce:param name="order" type="record:Order"/>
    <sce:return type="record:Order"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="order"/>
  </sce:body>
</scxml>
"#;
    let (dir, _) = fixture(&caller(CALLER_BODY, true));
    let path = dir.path().join("probe_echo.scxml");
    std::fs::write(&path, echo).expect("write the document");
    let (code, why) = refused(generated(&path, Language::Rust));
    assert!(
        matches!(code, DiagnosticCode::GenerateUnsupportedFeature),
        "{code:?}: {why}"
    );
    assert!(
        why.contains("the list field `lines`") && why.contains("as a parameter alone"),
        "{why}"
    );
}
