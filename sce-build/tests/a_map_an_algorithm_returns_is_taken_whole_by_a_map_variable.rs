// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A statechart takes the map an imported algorithm returns, whole, into a map
//! variable by an `<assign>` whose expression is the call (docs/adr/0108, D5):
//! `fixtures/static_datamodel/static_map_returned.scxml` assigns `kept`, a map
//! of three, what `algorithm_amounts_above.scxml` returns, a map of two. The
//! checker holds the variable to the return's entries and to a bound no
//! smaller than the return's, each refused by name, and the map returned is no
//! value anywhere but the right side of that `<assign>`. The Interpreter
//! writes the call into the document's own `<assign>`; Rust and C11 have the
//! algorithm build the map in a destination of the return's bound that the
//! call's adapter holds, and the variable take it from there whole once the
//! call has built it, under its own seed; every other lane refuses the call by
//! name. The scenario `scenarios/static_map_returned.json` runs the machine on
//! the three engines that lower it.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn fixtures() -> PathBuf {
    repo_root().join("sce-build/tests/fixtures/static_datamodel")
}

/// The statechart's name in the fixture directory.
const STATECHART: &str = "static_map_returned.scxml";

/// The documents the statechart imports, copied beside a variant of it.
const SIBLINGS: [&str; 2] = ["schema_entry.scxml", "algorithm_amounts_above.scxml"];

/// An algorithm lent no map that returns one of a single entry, written beside
/// a variant of the statechart as `algorithm_one_amount.scxml`.
const ONE_AMOUNT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm"
       name="one_amount"
       version="1.0">
  <sce:signature>
    <sce:param name="id" type="uint32"/>
    <sce:param name="amount" type="int32"/>
    <sce:return type="map&lt;uint32, int32&gt;[2]" may-fail="true"/>
  </sce:signature>
  <sce:body>
    <sce:var name="one" type="map&lt;uint32, int32&gt;[2]"/>
    <sce:put target="one" key="id" expr="amount"/>
    <sce:return expr="one"/>
  </sce:body>
</scxml>
"#;

/// The well-formed statechart, as committed.
fn returned() -> String {
    std::fs::read_to_string(fixtures().join(STATECHART)).expect("read the statechart")
}

/// `text` with the one occurrence of `from` replaced by `to`.
fn edited(text: &str, from: &str, to: &str) -> String {
    assert_eq!(
        text.matches(from).count(),
        1,
        "`{from}` stands once in the document"
    );
    text.replacen(from, to, 1)
}

/// The declaration of `kept` as committed.
const KEPT: &str =
    r#"<data id="kept" sce:type="map&lt;uint32, int32&gt;[3]" sce:direction="out"/>"#;

/// `sce-codegen --error-format=json <args> <path>`: whether it passed, and
/// what it printed.
fn run(args: &[&str], path: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(path)
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// `document`, written beside the documents it may import, in `dir`: the path
/// of the statechart.
fn beside(dir: &Path, document: &str) -> PathBuf {
    for name in SIBLINGS {
        std::fs::copy(fixtures().join(name), dir.join(name)).expect("copy an import");
    }
    std::fs::write(dir.join("algorithm_one_amount.scxml"), ONE_AMOUNT)
        .expect("write the algorithm lent no map");
    let path = dir.join(STATECHART);
    std::fs::write(&path, document).expect("write the statechart");
    path
}

/// `check` of `document`, written beside the documents it may import.
fn check(document: &str) -> (bool, String) {
    let dir = tempfile::tempdir().expect("a scratch directory");
    let path = beside(dir.path(), document);
    run(&["check"], &path)
}

/// `document` is refused by `check`, its message holding `part`.
fn refused(document: &str, part: &str) {
    let (ok, out) = check(document);
    assert!(!ok, "expected a refusal holding `{part}`:\n{out}");
    assert!(
        out.contains(part),
        "expected a refusal holding `{part}`:\n{out}"
    );
}

/// What `generate -l <lane>` of the committed statechart wrote: every file of
/// the output at any depth with one of the extensions `exts`, in the order of
/// their paths.
fn generated(lane: &str, exts: &[&str]) -> String {
    let out_dir = tempfile::tempdir().expect("an output directory");
    let out = out_dir.path().to_str().expect("a utf-8 path").to_string();
    let (ok, printed) = run(
        &["generate", "-l", lane, "-o", out.as_str()],
        &fixtures().join(STATECHART),
    );
    assert!(ok, "{lane} generates the statechart:\n{printed}");
    let mut files = Vec::new();
    let mut dirs = vec![out_dir.path().to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).expect("read the output") {
            let path = entry.expect("an output entry").path();
            if path.is_dir() {
                dirs.push(path);
            } else if path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| exts.contains(&e))
            {
                files.push(path);
            }
        }
    }
    files.sort();
    let mut code = String::new();
    for path in files {
        code.push_str(&std::fs::read_to_string(&path).expect("read a generated file"));
        code.push('\n');
    }
    code
}

/// docs/adr/0108, D5: the committed statechart is accepted, in place and
/// beside copies of what it imports.
#[test]
fn a_map_variable_takes_the_map_an_algorithm_returns() {
    let (ok, out) = run(&["check"], &fixtures().join(STATECHART));
    assert!(ok, "check accepts the statechart:\n{out}");
    let (ok, out) = check(&returned());
    assert!(ok, "check accepts it beside copies of its imports:\n{out}");
}

/// docs/adr/0108, D5: the variable a map return is assigned to holds its
/// bound, as a list variable holds a list return's: one that holds fewer is
/// refused, naming the call, the variable and both bounds.
#[test]
fn a_map_variable_that_holds_fewer_than_the_return_is_refused_by_name() {
    let smaller = edited(
        &returned(),
        KEPT,
        r#"<data id="kept" sce:type="map&lt;uint32, int32&gt;[1]" sce:direction="out"/>"#,
    );
    refused(
        &smaller,
        "`AmountsAbove` may return a map of 2 entries, and `kept` holds at most 1 (its bound)",
    );
    // ... and one that holds as many as the return is accepted.
    let same = edited(
        &returned(),
        KEPT,
        r#"<data id="kept" sce:type="map&lt;uint32, int32&gt;[2]" sce:direction="out"/>"#,
    );
    let (ok, out) = check(&same);
    assert!(
        ok,
        "a map variable of the return's own bound takes it:\n{out}"
    );
}

/// docs/adr/0108, D5: the variable holds the return's entries; a map of
/// another value is refused, naming what the call returns.
#[test]
fn a_map_variable_of_other_entries_is_refused() {
    let other = edited(
        &returned(),
        KEPT,
        r#"<data id="kept" sce:type="map&lt;uint32, int64&gt;[3]" sce:direction="out"/>"#,
    );
    refused(&other, "`AmountsAbove` returns a map<uint32, int32>");
    refused(&other, "which `kept` does not hold");
}

/// docs/adr/0108, D5: the map a call returns is the right side of the
/// `<assign>` that takes it whole, and no value anywhere else.
#[test]
fn a_map_an_algorithm_returns_is_no_value_of_an_expression() {
    let operand = edited(
        &returned(),
        r#"<assign location="least" expr="25"/>"#,
        r#"<assign location="errors" expr="len(AmountsAbove(amounts, least))"/>"#,
    );
    refused(&operand, "`AmountsAbove` returns, used as a value");
}

/// docs/adr/0108, D5 on the Interpreter: the document's own `<assign>` takes
/// the call of the algorithm the document carries, whose array is the map.
#[test]
fn the_interpreter_writes_the_call_into_the_assign() {
    let path = fixtures().join(STATECHART);
    let lowered =
        sce_build::forge::static_js::lower_file(path.to_str().expect("a utf-8 path"), Vec::new())
            .unwrap_or_else(|e| panic!("the Interpreter lowers the statechart: {e:?}"));
    let at = lowered
        .find(r#"location="kept""#)
        .unwrap_or_else(|| panic!("no <assign> to `kept`:\n{lowered}"));
    let element = &lowered[at..];
    let element = &element[..element.find("/>").expect("the <assign> is closed")];
    assert!(
        element.contains("SceStatic.algorithms.amounts_above("),
        "the <assign> to `kept` does not call the algorithm: {element}"
    );
}

/// docs/adr/0108, D5 and D8 on Rust: the adapter builds the map in a
/// destination of the return's bound, keyed under a seed drawn from the one
/// the machine hands it, and the statement empties the variable, which keeps
/// its seed, and puts each entry of the map into it.
#[test]
fn a_rust_adapter_builds_the_map_in_a_destination_of_the_returns_bound() {
    let code = generated("rust", &["rs"]);
    for held in [
        "let mut r: ::sce_rust_runtime::BoundedMap<u32, i32, 2> = \
         ::sce_rust_runtime::FromSeed::from_seed(sce_seed.derive(0));",
        "sce_seed: ::sce_rust_runtime::MapSeed",
        ".clear(); for (sce_key, sce_value) in sce_returned.iter() {",
    ] {
        assert!(code.contains(held), "no `{held}` in the machine:\n{code}");
    }
}

/// docs/adr/0108, D5 and D8 on C11: the destination of the return's bound is
/// a member of the machine's storage for the call, started by the adapter
/// under a seed drawn from the one the machine hands it, the algorithm builds
/// the map in it through its core, and the statement has the variable take it
/// whole only where the call did not fail.
#[test]
fn a_c11_adapter_builds_the_map_in_the_storage_of_the_call() {
    let code = generated("c11", &["h", "c"]);
    for held in [
        "_start(&s->r, sce_map_seed_derive(seed, 0u));",
        "&s->r.core, failure);",
        "if (sce_failure_.failed) {",
        "sce_map_assign(&(",
    ] {
        assert!(code.contains(held), "no `{held}` in the machine:\n{code}");
    }
}

/// docs/adr/0108, D5: a lane that lends an algorithm no map takes none back
/// either, and refuses the call by name, before it reads the maps the
/// statechart holds, by the one refusal it asks first
/// (`static_lowering::lent_map_unlowered`), here of an algorithm lent no map.
#[test]
fn every_lane_that_takes_no_map_back_refuses_the_call_by_name() {
    let one = edited(
        &edited(
            &returned(),
            r#"<sce:import kind="algorithm" src="algorithm_amounts_above.scxml" as="AmountsAbove"/>"#,
            r#"<sce:import kind="algorithm" src="algorithm_one_amount.scxml" as="OneAmount"/>"#,
        ),
        r#"expr="AmountsAbove(amounts, least)""#,
        r#"expr="OneAmount(7, least)""#,
    );
    let dir = tempfile::tempdir().expect("a scratch directory");
    let path = beside(dir.path(), &one);
    let (ok, out) = run(&["check"], &path);
    assert!(
        ok,
        "check accepts the call of an algorithm lent no map:\n{out}"
    );
    let unlowered = sce_build::generator::Language::ALL
        .iter()
        .filter(|language| {
            !sce_build::forge::codegen_matrix::ALGORITHM_MAP_BACKENDS.contains(language)
        });
    for &language in unlowered {
        let lane = language.canonical_name();
        let out_dir = tempfile::tempdir().expect("an output directory");
        let out_dir = out_dir.path().to_str().expect("a utf-8 path").to_string();
        let mut args = vec!["generate", "-l", lane, "-o", out_dir.as_str()];
        if language == sce_build::generator::Language::Go {
            args.extend(["--go-module-prefix", "example.test/generated"]);
        }
        let (ok, out) = run(&args, &path);
        assert!(!ok, "{lane} takes no map an algorithm returns yet:\n{out}");
        assert!(
            out.contains("`OneAmount(")
                && out.contains("an imported algorithm that returns the map<uint32, int32>")
                && out.contains(&format!("has no {lane} lowering of the call yet")),
            "{lane} refuses the call by name:\n{out}"
        );
    }
}
