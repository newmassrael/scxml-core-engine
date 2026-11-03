// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! `orchestrate --go-module-prefix` reaches every statechart of the set and
//! every child a statechart carries, as `generate --go-module-prefix` reaches
//! its one document.
//!
//! A `datamodel="sce-static"` machine that calls an imported algorithm imports
//! the algorithm's Go package by the module path the run is given, and is
//! refused without one (docs/SCE_ACCEPTED_SUBSET.md §2.15). `orchestrate` took
//! the flag and gave it to the forge documents of the set alone: each
//! statechart was compiled with no forge options, so a set whose statechart, or
//! whose statechart's in-line child, calls an algorithm was refused for want of
//! the prefix the command line had named, and `check` on the set, which
//! compiles it the same way, refused it too.
//!
//! The roots `generate` names a machine and its transition function under
//! (`--cpp-namespace-prefix`, `--kotlin-package-prefix`, `--c-symbol-prefix`)
//! are not taken by `orchestrate`, which gives the machine none
//! (docs/SCE_TRANSITION_FUNCTION.md): the command refuses each as a usage
//! error rather than accept it and generate as though it had not been given.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const PREFIX: &str = "example.com/generated";

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// A scratch directory of its own for `name`, emptied first, holding the
/// algorithm the statecharts below import.
fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("a_document_set_is_given_its_go_module_prefix")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch");
    std::fs::copy(
        repo_root().join("sce-build/tests/fixtures/static_datamodel/algorithm_day_run.scxml"),
        dir.join("algorithm_day_run.scxml"),
    )
    .expect("an algorithm of the fixtures");
    dir
}

fn codegen(args: &[&str]) -> Output {
    Command::new(sce_codegen_bin())
        .env("SCE_WORKSPACE_ROOT", repo_root())
        .args(args)
        .output()
        .expect("spawn sce-codegen")
}

/// The body of a `sce-static` statechart that calls the imported algorithm
/// `Range` in its one transition.
const CALLER_BODY: &str = r#"<sce:import kind="algorithm" src="algorithm_day_run.scxml" as="Range"/>
  <datamodel>
    <data id="shown" sce:type="list&lt;uint8&gt;" sce:capacity="8"/>
    <data id="n" sce:type="uint8" expr="3"/>
  </datamodel>
  <state id="s">
    <transition event="go" type="internal">
      <assign location="shown" expr="Range(n, 4)"/>
    </transition>
  </state>"#;

/// A statechart that calls the algorithm itself.
fn caller() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  {CALLER_BODY}
</scxml>
"#
    )
}

/// A statechart that starts, in its one state, a child written in `<content>`
/// that calls the algorithm.
fn host() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="running" datamodel="sce-static">
  <state id="running">
    <invoke type="scxml" id="kid">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s" datamodel="sce-static">
  {CALLER_BODY}
        </scxml>
      </content>
    </invoke>
    <transition event="done.invoke.kid" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#
    )
}

/// What the run printed, failing with all of it unless it succeeded.
fn succeeded(run: &Output, what: &str) {
    assert!(
        run.status.success(),
        "{what} failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr),
    );
}

/// The Go source at `path` imports a package under [`PREFIX`].
fn imports_under_the_prefix(path: &Path) -> bool {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
        .contains(&format!("\"{PREFIX}/"))
}

/// The statechart of a set that calls an algorithm is generated with the
/// prefix, and `check` on the set accepts it.
#[test]
fn a_statechart_of_the_set_imports_by_the_prefix() {
    let work = scratch("caller");
    let doc = work.join("caller.scxml");
    std::fs::write(&doc, caller()).expect("write the statechart");
    let doc_str = doc.to_str().unwrap();
    let out = work.join("out");
    succeeded(
        &codegen(&[
            "orchestrate",
            "--scxml",
            doc_str,
            "-l",
            "go",
            "--go-module-prefix",
            PREFIX,
            "-o",
            out.to_str().unwrap(),
        ]),
        "orchestrate -l go --go-module-prefix",
    );
    assert!(
        imports_under_the_prefix(&out.join("caller_sm.go")),
        "the machine does not import the algorithm by the prefix it was given"
    );
    succeeded(
        &codegen(&[
            "check",
            "--scxml",
            doc_str,
            "-l",
            "go",
            "--go-module-prefix",
            PREFIX,
        ]),
        "check --scxml -l go --go-module-prefix",
    );
}

/// The in-line child of a statechart of the set is generated with the prefix
/// too: it is a machine of the set, written beside its parent.
#[test]
fn a_child_a_statechart_of_the_set_carries_imports_by_the_prefix() {
    let work = scratch("host");
    let doc = work.join("host.scxml");
    std::fs::write(&doc, host()).expect("write the statechart");
    let out = work.join("out");
    succeeded(
        &codegen(&[
            "orchestrate",
            "--scxml",
            doc.to_str().unwrap(),
            "-l",
            "go",
            "--go-module-prefix",
            PREFIX,
            "-o",
            out.to_str().unwrap(),
        ]),
        "orchestrate -l go --go-module-prefix",
    );
    assert!(
        imports_under_the_prefix(&out.join("host__sce_synth_invoke__kid_sm.go")),
        "the child's machine does not import the algorithm by the prefix the set was given"
    );
}

/// `orchestrate` refuses each root `generate` names a machine under, as a
/// usage error (`cli/usage`, exit 20), and writes nothing: it gives the
/// machine none of them, so accepting one would generate as though it had not
/// been given.
#[test]
fn orchestrate_refuses_the_naming_roots_it_does_not_give() {
    let work = scratch("roots");
    let doc = work.join("caller.scxml");
    std::fs::write(&doc, caller()).expect("write the statechart");
    for (lang, flag) in [
        ("cpp", "--cpp-namespace-prefix"),
        ("kotlin", "--kotlin-package-prefix"),
        ("c11", "--c-symbol-prefix"),
    ] {
        let out = work.join(format!("out_{lang}"));
        let run = codegen(&[
            "--error-format=json",
            "orchestrate",
            "--scxml",
            doc.to_str().unwrap(),
            "-l",
            lang,
            flag,
            "acme",
            "-o",
            out.to_str().unwrap(),
        ]);
        let stderr = String::from_utf8_lossy(&run.stderr);
        assert_eq!(run.status.code(), Some(20), "{flag}: {stderr}");
        assert!(
            stderr.contains("\"code\":\"cli/usage\""),
            "{flag} is not refused as a usage error: {stderr}"
        );
        assert!(
            !out.exists(),
            "{flag}: a refused invocation wrote {}",
            out.display()
        );
    }
}
