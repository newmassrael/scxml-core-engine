// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An algorithm holds a list of the values of an enum, `list<enum:E>`
//! (SCE_FORGE.md §4.12): a read-only parameter, a buffer local and a return,
//! read by `len`, an index and `<sce:foreach>` and appended to, each element a
//! value of its enum and of nothing else.
//!
//! * Rust lowers one: the generated text, and the text compiled under
//!   `rustc -D warnings` and run.
//! * What an element is no value of is refused by the enum rule, before any
//!   backend is asked (`forge::static_enum`).
//! * A backend not in `ALGORITHM_ENUM_LIST_BACKENDS` refuses the algorithm by
//!   name where it renders it, and a `check` that names no backend reports it
//!   per backend.
//! * The Interpreter's lowering holds the list as the array of its elements'
//!   declared names.
//! * A statechart holds no such list and takes none from a call.
//! * Another algorithm is lent one, on Rust alone, by a list parameter whose
//!   element is a value of the same enum.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::codegen_matrix::{language_wire_name, sweeps, ALGORITHM_ENUM_LIST_BACKENDS};
use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, Located};
use sce_build::generator::{GeneratedOutput, Language};
use sce_build::ForgeCompileOptions;

/// A closed enum: its declared values are not its variants' positions.
const STATUS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="enum" name="probe_status" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="open" value="2"/>
      <sce:variant name="done" value="7"/>
    </data>
  </datamodel>
</scxml>
"#;

/// An open enum, whose Rust type carries the number rather than a variant.
const LEVEL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="enum" name="probe_level" sce:underlying-type="uint8" sce:strict-variants="false">
  <datamodel>
    <data id="variants">
      <sce:variant name="low" value="1"/>
      <sce:variant name="high" value="9"/>
    </data>
  </datamodel>
</scxml>
"#;

/// The imports of both enums.
const ENUMS: &str = r#"<sce:import kind="enum" src="probe_status.scxml" as="Status"/>
  <sce:import kind="enum" src="probe_level.scxml" as="Level"/>"#;

/// The import of the closed enum alone.
const STATUS_ONLY: &str = r#"<sce:import kind="enum" src="probe_status.scxml" as="Status"/>"#;

/// The import of the open enum alone.
const LEVEL_ONLY: &str = r#"<sce:import kind="enum" src="probe_level.scxml" as="Level"/>"#;

/// An algorithm named `name` that imports `imports`, whose signature is
/// `signature` and whose body is `body`.
fn algorithm(name: &str, imports: &str, signature: &str, body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="{name}" version="1.0">
  {imports}
  <sce:signature>
    {signature}
  </sce:signature>
  <sce:body>
    {body}
  </sce:body>
</scxml>
"#
    )
}

/// Whether `xs` holds `wanted`: a parameter walked by a loop and read at an
/// index its guard proves, in a body that cannot fail.
fn status_any() -> String {
    algorithm(
        "status_any",
        STATUS_ONLY,
        r#"<sce:param name="xs" type="list&lt;enum:Status&gt;"/>
    <sce:param name="wanted" type="enum:Status"/>
    <sce:return type="bool"/>"#,
        r#"<sce:var name="seen" type="bool" init="false"/>
    <sce:foreach item="s" in="xs">
      <sce:if cond="s === wanted">
        <sce:assign target="seen" expr="true"/>
      </sce:if>
    </sce:foreach>
    <sce:var name="i" type="uint32" init="0"/>
    <sce:while cond="i &lt; len(xs)">
      <sce:if cond="xs[i] === wanted &amp;&amp; xs[i] !== Status.open">
        <sce:assign target="seen" expr="true"/>
      </sce:if>
      <sce:assign target="i" expr="i + 1"/>
    </sce:while>
    <sce:return expr="seen"/>"#,
    )
}

/// The length of `xs`, whose enum the body names nowhere but in the signature.
fn status_count() -> String {
    algorithm(
        "status_count",
        STATUS_ONLY,
        r#"<sce:param name="xs" type="list&lt;enum:Status&gt;"/>
    <sce:return type="uint32"/>"#,
        r#"<sce:return expr="len(xs)"/>"#,
    )
}

/// The element at `at`, then each element that is not `wanted`, then the first
/// of those where there is one and `done` where there is none, then `done`: a
/// working list, the output, a local taken from an element, and an element read
/// back from a buffer.
fn status_kept() -> String {
    algorithm(
        "status_kept",
        STATUS_ONLY,
        r#"<sce:param name="xs" type="list&lt;enum:Status&gt;"/>
    <sce:param name="wanted" type="enum:Status"/>
    <sce:param name="at" type="uint32"/>
    <sce:return type="list&lt;enum:Status&gt;[8]" may-fail="true"/>"#,
        r#"<sce:var name="rest" type="list&lt;enum:Status&gt;[6]"/>
    <sce:foreach item="s" in="xs">
      <sce:if cond="s !== wanted">
        <sce:append target="rest" expr="s"/>
      </sce:if>
    </sce:foreach>
    <sce:var name="picked" type="enum:Status" init="xs[at]"/>
    <sce:var name="out" type="list&lt;enum:Status&gt;[8]"/>
    <sce:append target="out" expr="picked"/>
    <sce:foreach item="r" in="rest">
      <sce:append target="out" expr="r"/>
    </sce:foreach>
    <sce:append target="out" expr="len(rest) &gt; 0 ? rest[0] : Status.done"/>
    <sce:append target="out" expr="Status.done"/>
    <sce:return expr="out"/>"#,
    )
}

/// The elements of `xs` that are not `low`, over the open enum.
fn levels_kept() -> String {
    algorithm(
        "levels_kept",
        LEVEL_ONLY,
        r#"<sce:param name="xs" type="list&lt;enum:Level&gt;"/>
    <sce:return type="list&lt;enum:Level&gt;[4]"/>"#,
        r#"<sce:var name="out" type="list&lt;enum:Level&gt;[4]"/>
    <sce:foreach item="l" in="xs">
      <sce:if cond="l !== Level.low">
        <sce:append target="out" expr="l"/>
      </sce:if>
    </sce:foreach>
    <sce:return expr="out"/>"#,
    )
}

/// A directory that holds the two enums and `document` as `<name>.scxml`, and
/// the path of the document.
fn written(name: &str, document: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("a scratch directory");
    std::fs::write(dir.path().join("probe_status.scxml"), STATUS).expect("write the closed enum");
    std::fs::write(dir.path().join("probe_level.scxml"), LEVEL).expect("write the open enum");
    let path = dir.path().join(format!("{name}.scxml"));
    std::fs::write(&path, document).expect("write the document");
    (dir, path)
}

/// What `language` generates for the document at `path`. Go is handed the module
/// path its imports are under, which every other backend ignores.
fn generated(path: &Path, language: Language) -> Result<GeneratedOutput, Located<ForgeError>> {
    let options = ForgeCompileOptions {
        go_module_prefix: Some("example.com/probe".to_string()),
        ..ForgeCompileOptions::default()
    };
    sce_build::compile_forge_file(path, language, &[], &options)
}

/// The Rust unit of `document`, named `name`.
fn rust_unit(name: &str, document: &str) -> String {
    let (_dir, path) = written(name, document);
    let output = generated(&path, Language::Rust)
        .unwrap_or_else(|e| panic!("{name} does not generate on Rust: {}", e.error));
    output
        .files
        .into_iter()
        .find(|(file, _)| file.strip_suffix(".rs") == Some(name))
        .unwrap_or_else(|| panic!("{name}: no Rust unit named for it"))
        .1
}

/// The code of the first record the refusal of `result` carries, and its text.
fn refused(result: Result<GeneratedOutput, Located<ForgeError>>) -> (DiagnosticCode, String) {
    let err = match result {
        Ok(_) => panic!("the probe generated"),
        Err(refusal) => refusal,
    };
    let record = err.error.to_diagnostics().remove(0);
    (record.code, err.error.to_string())
}

/// Whether `text` holds `needle`, with `text` in the failure.
fn holds(text: &str, needle: &str) {
    assert!(text.contains(needle), "the unit holds `{needle}`:\n{text}");
}

#[test]
fn rust_holds_each_list_of_enum_values_in_the_enums_own_type() {
    let any = rust_unit("status_any", &status_any());
    holds(&any, "use super::probe_status;\n");
    holds(&any, "xs: &[probe_status::ProbeStatus]");
    holds(&any, "for &s in xs.iter() {");
    // A proven read of the list, through the runtime (SCE_FORGE.md §3.4.1).
    holds(&any, "sce_forge_runtime::algorithm::proven::at(&xs, i)");
    holds(&any, "probe_status::ProbeStatus::Open");

    let kept = rust_unit("status_kept", &status_kept());
    holds(
        &kept,
        "out: &mut SceOwnedList<probe_status::ProbeStatus, 8>",
    );
    holds(
        &kept,
        "let mut rest = SceOwnedList::<probe_status::ProbeStatus, 6>::with_capacity(6);",
    );
    holds(&kept, "for &r in rest.iter() {");
    holds(&kept, "rest.push(s)?;");
    holds(&kept, "= sce_forge_runtime::algorithm::at(&xs, at)?;");
    holds(&kept, "out.push(probe_status::ProbeStatus::Done)?;");

    // The enum of a list the body reads by its length alone is imported.
    let count = rust_unit("status_count", &status_count());
    holds(&count, "use super::probe_status;\n");

    // An open enum's type is `Copy` too, so a loop copies each element out.
    let levels = rust_unit("levels_kept", &levels_kept());
    holds(&levels, "xs: &[probe_level::ProbeLevel]");
    holds(&levels, "for &l in xs.iter() {");
    holds(&levels, "out.push(l)?;");
}

/// What the four algorithms answer, each asked of the generated function.
const MAIN: &str = r#"
use probe_level::ProbeLevel;
use probe_status::ProbeStatus::{self, Done, Open};
use sce_portable_bytes::SceOwnedList;

fn kept(xs: &[ProbeStatus], wanted: ProbeStatus, at: u32) -> Option<Vec<ProbeStatus>> {
    let mut out = SceOwnedList::new();
    status_kept::status_kept(xs, wanted, at, &mut out)
        .ok()
        .map(|()| out.as_slice().to_vec())
}

fn main() {
    assert!(status_any::status_any(&[Open, Done], Done));
    assert!(status_any::status_any(&[Open], Open));
    assert!(!status_any::status_any(&[Open], Done));
    assert!(!status_any::status_any(&[], Open));
    assert_eq!(status_count::status_count(&[Open, Done, Open]), 3);
    assert_eq!(
        kept(&[Open, Done, Open], Done, 1),
        Some(vec![Done, Open, Open, Open, Done])
    );
    assert_eq!(kept(&[Done], Done, 0), Some(vec![Done, Done, Done]));
    // An index outside the list fails, as it does over a list of numbers.
    assert_eq!(kept(&[Done], Done, 1), None);
    assert_eq!(kept(&[], Done, 0), None);
    let mut levels = SceOwnedList::new();
    let unnamed = ProbeLevel::from_underlying(4);
    let given = [ProbeLevel::Low, unnamed, ProbeLevel::High];
    assert!(levels_kept::levels_kept(&given, &mut levels).is_ok());
    assert_eq!(levels.as_slice(), [unnamed, ProbeLevel::High]);
}
"#;

/// The generated units beside the enums they name: accepted by `rustc` with
/// every warning an error, and answering what their documents say when run.
#[test]
fn the_rust_of_a_list_of_enum_values_compiles_under_deny_warnings_and_runs() {
    let Some(rustc) = sce_build::toolchain::require_any_or_skip(
        &["rustc"],
        "compile a generated algorithm over a list of enum values",
    ) else {
        return;
    };
    let dir = tempfile::tempdir().expect("a scratch directory");
    let mut modules = String::new();
    for (name, document) in [
        ("probe_status", STATUS.to_string()),
        ("probe_level", LEVEL.to_string()),
        ("status_any", status_any()),
        ("status_count", status_count()),
        ("status_kept", status_kept()),
        ("levels_kept", levels_kept()),
    ] {
        std::fs::write(
            dir.path().join(format!("{name}.rs")),
            rust_unit(name, &document),
        )
        .expect("write the unit");
        modules.push_str(&format!("pub mod {name};\n"));
    }
    let library = dir.path().join("lib.rs");
    std::fs::write(&library, &modules).expect("write the library root");
    let program = dir.path().join("main.rs");
    std::fs::write(&program, format!("{modules}{MAIN}")).expect("write the program root");
    let runtime = tempfile::tempdir().expect("a scratch directory for the runtime");
    let flags = common::rust_runtime::rustc_flags(runtime.path());

    let compiled = Command::new(&rustc)
        .args([
            "--edition=2021",
            "--crate-type=lib",
            "--emit=metadata",
            "-D",
            "warnings",
            "--out-dir",
        ])
        .arg(dir.path().join("out"))
        .arg(&library)
        .args(&flags)
        .output()
        .expect("rustc runs");
    assert!(
        compiled.status.success(),
        "the generated Rust does not compile:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );

    // The program calls the units of its own crate, whose accessors it does not
    // all read, so what it leaves unread is no error here.
    let binary = dir.path().join("probe");
    let linked = Command::new(&rustc)
        .args(["--edition=2021", "-o"])
        .arg(&binary)
        .arg(&program)
        .args(&flags)
        .output()
        .expect("rustc runs");
    assert!(
        linked.status.success(),
        "the program over the generated Rust does not compile:\n{}",
        String::from_utf8_lossy(&linked.stderr)
    );
    let ran = Command::new(&binary).output().expect("the program runs");
    assert!(
        ran.status.success(),
        "a generated algorithm answered another value:\n{}",
        String::from_utf8_lossy(&ran.stderr)
    );
}

/// The signature the refusals are written in.
const TAKES_STATUSES: &str = r#"<sce:param name="xs" type="list&lt;enum:Status&gt;"/>
    <sce:param name="levels" type="list&lt;enum:Level&gt;"/>
    <sce:param name="at" type="uint32"/>
    <sce:return type="bool" may-fail="true"/>"#;

/// The code and text of the refusal of `body` over both lists, asked of Kotlin,
/// which lowers neither: the body is judged before any backend is asked, so a
/// misuse is refused as the misuse it is and never as a list no lane lowers.
fn misuse(signature: &str, body: &str) -> (DiagnosticCode, String) {
    let (_dir, path) = written("misused", &algorithm("misused", ENUMS, signature, body));
    refused(generated(&path, Language::Kotlin))
}

/// An element is a value of its enum, at an index and as a loop's item: it is
/// compared with a value of that enum and is no number, no operand and no index.
#[test]
fn an_element_is_a_value_of_its_enum_and_of_nothing_else() {
    for (cond, reason) in [
        (
            "xs[at] === 2",
            "a comparison of an enum value with anything but a value of the same enum",
        ),
        (
            "xs[at] === 'done'",
            "a comparison of an enum value with anything but a value of the same enum",
        ),
        (
            "xs[at] === Level.low",
            "a comparison of an enum value with anything but a value of the same enum",
        ),
        (
            "xs[at] === levels[at]",
            "a comparison of an enum value with anything but a value of the same enum",
        ),
        (
            "xs[at] &lt; Status.done",
            "an enum value used as an operand or an argument",
        ),
        (
            "xs[at] + 1 &gt; 0",
            "an enum value used as an operand or an argument",
        ),
        ("xs[xs[at]] === Status.done", "an enum value as an index"),
    ] {
        let body = format!(r#"<sce:return expr="{cond}"/>"#);
        let (code, why) = misuse(TAKES_STATUSES, &body);
        assert!(
            matches!(code, DiagnosticCode::ExpressionUnsupportedConstruct),
            "{cond}: {code:?}: {why}"
        );
        assert!(why.contains(reason), "{cond}: {why}");
    }
    for cond in ["s === 2", "s &gt; Status.open", "s === Level.low"] {
        let body = format!(
            r#"<sce:foreach item="s" in="xs">
      <sce:if cond="{cond}">
        <sce:return expr="true"/>
      </sce:if>
    </sce:foreach>
    <sce:return expr="false"/>"#
        );
        let (code, why) = misuse(TAKES_STATUSES, &body);
        assert!(
            matches!(code, DiagnosticCode::ExpressionUnsupportedConstruct),
            "{cond}: {code:?}: {why}"
        );
    }
}

/// An element stands where a value of its enum is declared, and a number's place
/// takes none.
#[test]
fn an_element_is_returned_as_its_enum_and_never_as_a_number() {
    let signature = r#"<sce:param name="xs" type="list&lt;enum:Status&gt;"/>
    <sce:return type="uint8" may-fail="true"/>"#;
    let (code, why) = misuse(signature, r#"<sce:return expr="xs[0]"/>"#);
    assert!(
        matches!(code, DiagnosticCode::ExpressionUnsupportedConstruct),
        "{code:?}: {why}"
    );
    assert!(
        why.contains("a value of the enum `Status` where a number or a bool is expected"),
        "{why}"
    );
    let of_another = r#"<sce:param name="xs" type="list&lt;enum:Level&gt;"/>
    <sce:return type="enum:Status" may-fail="true"/>"#;
    let (_, why) = misuse(of_another, r#"<sce:return expr="xs[0]"/>"#);
    assert!(
        why.contains("a value of the enum `Level` where a value of the enum `Status` is declared"),
        "{why}"
    );
}

/// A list of enum values takes, by `<sce:append>`, a value of its enum: no
/// number, no text and no value of another enum.
#[test]
fn a_list_of_enum_values_is_appended_a_value_of_its_enum_alone() {
    let signature = r#"<sce:param name="level" type="enum:Level"/>
    <sce:param name="n" type="uint8"/>
    <sce:return type="list&lt;enum:Status&gt;[4]" may-fail="true"/>"#;
    for (appended, reason) in [
        ("7", "a value that is not of the enum `Status`"),
        ("n", "a value that is not of the enum `Status`"),
        ("'done'", "a value that is not of the enum `Status`"),
        (
            "level",
            "a value of the enum `Level` where a value of the enum `Status` is declared",
        ),
        (
            "Level.low",
            "a value of the enum `Level` where a value of the enum `Status` is declared",
        ),
    ] {
        let body = format!(
            r#"<sce:var name="out" type="list&lt;enum:Status&gt;[4]"/>
    <sce:append target="out" expr="{appended}"/>
    <sce:return expr="out"/>"#
        );
        let (code, why) = misuse(signature, &body);
        assert!(
            matches!(code, DiagnosticCode::ExpressionUnsupportedConstruct),
            "{appended}: {code:?}: {why}"
        );
        assert!(why.contains(reason), "{appended}: {why}");
    }
}

/// The length of `xs`, a list of values of the open enum: a callee whose list
/// parameter takes an element of other values than `Status`'s.
fn level_count() -> String {
    algorithm(
        "level_count",
        LEVEL_ONLY,
        r#"<sce:param name="xs" type="list&lt;enum:Level&gt;"/>
    <sce:return type="uint32"/>"#,
        r#"<sce:return expr="len(xs)"/>"#,
    )
}

/// An algorithm `status_caller` that lends its `list<enum:Status>` parameter to
/// the algorithm `callee` imports as `Count`, written beside the enums and
/// `callee` as `<callee_name>.scxml`; the caller's path.
fn status_caller(callee_name: &str, callee: &str) -> (tempfile::TempDir, PathBuf) {
    let (dir, path) = written(
        "status_caller",
        &algorithm(
            "status_caller",
            &format!(
                r#"<sce:import kind="enum" src="probe_status.scxml" as="Status"/>
  <sce:import kind="algorithm" src="{callee_name}.scxml" as="Count"/>"#
            ),
            r#"<sce:param name="xs" type="list&lt;enum:Status&gt;"/>
    <sce:return type="uint32"/>"#,
            r#"<sce:return expr="Count(xs)"/>"#,
        ),
    );
    std::fs::write(dir.path().join(format!("{callee_name}.scxml")), callee)
        .expect("write the callee");
    (dir, path)
}

/// A list of enum values is lent to another algorithm's list parameter as a
/// list of numbers is (SCE_FORGE.md §4.12, "Where a list crosses"), on the
/// backends that lend a list across: on Rust the caller lends its parameter, the
/// slice it is, to the callee's, and every other backend refuses the caller,
/// naming `list<enum:Status>`. Pinned by the list arm of
/// `algorithm_record_slots` (`sce-build/src/lib.rs`) and `handed_list`
/// (`sce-build/src/forge/expr.rs`): with the slot mapped to no list, Rust
/// refuses the call as one a host alone makes.
#[test]
fn a_list_of_enum_values_is_lent_to_another_algorithm_on_rust_alone() {
    let (_dir, path) = status_caller("status_count", &status_count());
    for language in sce_build::forge::codegen_matrix::emitting_in_part(
        sce_build::forge::model::ForgeKind::Algorithm,
    ) {
        let result = generated(&path, language);
        if language == Language::Rust {
            let output = result.unwrap_or_else(|e| panic!("Rust lends the list: {}", e.error));
            let text = output
                .files
                .into_iter()
                .find(|(file, _)| file.strip_suffix(".rs") == Some("status_caller"))
                .expect("the caller's Rust unit")
                .1;
            holds(&text, "xs: &[probe_status::ProbeStatus]");
            holds(&text, "status_count(xs)");
            continue;
        }
        let (code, why) = refused(result);
        assert!(
            matches!(
                code,
                DiagnosticCode::GenerateUnsupportedFeature
                    | DiagnosticCode::ExpressionUnsupportedConstruct
            ),
            "{language:?}: {code:?}: {why}"
        );
        assert!(why.contains("list<enum:Status>"), "{language:?}: {why}");
    }
}

/// A list of the values of one enum is not lent to a list parameter of another
/// enum's values: the element is held to the parameter's by the relation "fits"
/// (docs/adr/0086, section 5.1), two enums one type only where they hold the
/// same values. Pinned by the element check of `judge_list_argument`
/// (`sce-build/src/forge/expr.rs`).
#[test]
fn a_list_of_enum_values_is_refused_by_a_list_parameter_of_another_enum() {
    let (_dir, path) = status_caller("level_count", &level_count());
    let (code, why) = refused(generated(&path, Language::Rust));
    assert!(
        matches!(code, DiagnosticCode::ExpressionUnsupportedConstruct),
        "{code:?}: {why}"
    );
    assert!(
        why.contains("of another document than") && why.contains("handed to `Count`"),
        "{why}"
    );
}

/// A working list of enum values beside an output of numbers: the one list of
/// the algorithm whose element is an enum is in its body.
fn status_numbers() -> String {
    algorithm(
        "status_numbers",
        STATUS_ONLY,
        r#"<sce:param name="first" type="enum:Status"/>
    <sce:return type="list&lt;uint8&gt;[4]" may-fail="true"/>"#,
        r#"<sce:var name="held" type="list&lt;enum:Status&gt;[2]"/>
    <sce:append target="held" expr="first"/>
    <sce:var name="out" type="list&lt;uint8&gt;[4]"/>
    <sce:foreach item="h" in="held">
      <sce:append target="out" expr="Status.toNumber(h)"/>
    </sce:foreach>
    <sce:return expr="out"/>"#,
    )
}

/// Every backend either lowers an algorithm's list of enum values, listed in
/// `ALGORITHM_ENUM_LIST_BACKENDS`, or refuses the algorithm by name, naming the
/// list that stops it: a parameter's and a local's alike.
#[test]
fn a_backend_that_lowers_no_list_of_enum_values_refuses_one_by_name() {
    for (name, document, held) in [
        ("status_any", status_any(), "`xs` of list<enum:Status>"),
        (
            "status_numbers",
            status_numbers(),
            "`held` of list<enum:Status>[2]",
        ),
        ("levels_kept", levels_kept(), "`xs` of list<enum:Level>"),
    ] {
        let (_dir, path) = written(name, &document);
        for &language in Language::ALL {
            let result = generated(&path, language);
            if ALGORITHM_ENUM_LIST_BACKENDS.contains(&language) {
                if let Err(refusal) = result {
                    panic!(
                        "{language:?} lowers a list of enum values and refused {name}: {}",
                        refusal.error
                    );
                }
                continue;
            }
            let lane = language.canonical_name();
            let (code, why) = refused(result);
            assert!(
                matches!(code, DiagnosticCode::GenerateUnsupportedFeature),
                "{lane}: {code:?}: {why}"
            );
            assert!(
                why.contains(&format!(
                    "algorithm '{name}' holds {held}, and {lane} does not lower a list of enum \
                     values in an algorithm yet"
                )),
                "{lane}: {why}"
            );
        }
    }
}

/// `sce-codegen <args…> <file>` in JSON mode: whether it succeeded, what it wrote
/// on stdout, and both streams.
fn codegen(args: &[&str], file: &Path) -> (bool, String, String) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent");
    let out = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("--workspace-root")
        .arg(root)
        .arg("--error-format=json")
        .args(args)
        .arg(file)
        .output()
        .expect("run sce-codegen");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let both = String::from_utf8_lossy(&out.stderr).into_owned() + &stdout;
    (out.status.success(), stdout, both)
}

/// A `check` that names no backend is the document's own verdict: it succeeds,
/// and reports per backend which one lowers the list.
#[test]
fn a_check_that_names_no_backend_reports_the_list_per_backend() {
    let (_dir, path) = written("status_any", &status_any());
    let (ok, stdout, both) = codegen(&["check", "--go-module-prefix", "example.com/probe"], &path);
    assert!(ok, "the document is valid:\n{both}");
    let manifest: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("a sweep writes a manifest");
    let verdicts = manifest["languages"]
        .as_array()
        .expect("a sweep's manifest carries languages");
    for language in Language::ALL.iter().copied().filter(|l| sweeps(*l)) {
        let wire = language_wire_name(language);
        let verdict = verdicts
            .iter()
            .find(|verdict| verdict["language"] == wire)
            .unwrap_or_else(|| panic!("{wire} is reported:\n{both}"));
        if ALGORITHM_ENUM_LIST_BACKENDS.contains(&language) {
            assert_eq!(verdict["status"], "ok", "{wire}:\n{both}");
        } else {
            assert_eq!(verdict["status"], "rejected", "{wire}:\n{both}");
            assert_eq!(
                verdict["code"], "generate/unsupported-feature",
                "{wire}:\n{both}"
            );
        }
    }
}

/// The Interpreter's lowering holds the list as the array of its elements'
/// declared names, as it holds one of them: a loop, an index and a length read
/// it, and an append adds a name.
#[test]
fn the_interpreter_holds_the_list_as_the_declared_names_of_its_elements() {
    let (_dir, path) = written("status_kept", &status_kept());
    let lowered =
        sce_build::forge::static_js::lower_algorithm_document(path.to_str().expect("a utf-8 path"))
            .unwrap_or_else(|e| panic!("the Interpreter lowers the algorithm: {e:?}"));
    for part in [
        "__i0 < xs.length",
        "rest = SceStatic.append(rest, 6, s);",
        "out = SceStatic.append(out, 8, picked);",
        "out = SceStatic.append(out, 8, 'done');",
        "return out;",
    ] {
        assert!(
            lowered.install.contains(part),
            "the installed algorithm holds `{part}`:\n{}",
            lowered.install
        );
    }
}

/// A statechart whose datamodel holds `data`, whose imports are `imports` and
/// whose one transition runs `action`.
fn machine(imports: &str, data: &str, action: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="s" datamodel="sce-static">
  {imports}
  <datamodel>
    {data}
  </datamodel>
  <state id="s">
    <transition event="go" type="internal">
      {action}
    </transition>
  </state>
</scxml>
"#
    )
}

/// A statechart's variable is no list of enum values: the algorithm's list
/// widened nothing the statechart's lowerings do not hold.
#[test]
fn a_statecharts_variable_is_no_list_of_enum_values() {
    let (_dir, path) = written(
        "holder",
        &machine(
            STATUS_ONLY,
            r#"<data id="statuses" sce:type="list&lt;enum:Status&gt;[3]"/>"#,
            r#"<sce:clear target="statuses"/>"#,
        ),
    );
    for lane in ["rust", "kotlin"] {
        let (ok, _, both) = codegen(&["check", "-l", lane], &path);
        assert!(!ok, "{lane} holds no such variable:\n{both}");
        assert!(
            both.contains("`enum:Status` in a statechart's variable is a type of the language"),
            "{lane}:\n{both}"
        );
    }
}

/// A statechart takes no list of enum values from a call, whatever the algorithm
/// declares of it, and is told which of its conditions the call fails.
#[test]
fn a_statechart_takes_no_list_of_enum_values_from_a_call() {
    let (dir, path) = written(
        "taker",
        &machine(
            r#"<sce:import kind="algorithm" src="status_run.scxml" as="Run"/>"#,
            r#"<data id="shown" sce:type="list&lt;uint8&gt;[4]"/>
    <data id="n" sce:type="uint8" expr="1"/>"#,
            r#"<assign location="shown" expr="Run(n)"/>"#,
        ),
    );
    std::fs::write(
        dir.path().join("status_run.scxml"),
        algorithm(
            "status_run",
            STATUS_ONLY,
            r#"<sce:param name="n" type="uint8"/>
    <sce:return type="list&lt;enum:Status&gt;[4]" may-fail="true"/>"#,
            r#"<sce:var name="out" type="list&lt;enum:Status&gt;[4]"/>
    <sce:if cond="n &gt; 0">
      <sce:append target="out" expr="Status.open"/>
    </sce:if>
    <sce:return expr="out"/>"#,
        ),
    )
    .expect("write the algorithm");
    let (ok, _, both) = codegen(&["check", "-l", "rust"], &path);
    assert!(!ok, "a statechart takes no such list:\n{both}");
    assert!(
        both.contains(
            "it returns a list of enum values, and a statechart takes a list of numbers or bools \
             only"
        ),
        "{both}"
    );
}
