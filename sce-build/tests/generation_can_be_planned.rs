// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `--plan`: a generation that writes nothing and prints the files it would
// have written. A build system has to declare a command's outputs before it
// has run the command, and a document can produce files its author never
// wrote — an inline `<invoke>` becomes a child document with a machine of its
// own — so the answer has to come from the generation and not from a list kept
// beside it.
//
// Each property is a test of its own:
//   - the run writes nothing, the output directory included;
//   - what it prints is exactly the set of files the same run writes;
//   - what it prints is paths and nothing else: no manifest line among them;
//   - a document that synthesizes children has them in the plan;
//   - the exit status is the generation's own: a document it refuses is refused;
//   - what is on disk does not change the answer;
//   - a command it cannot serve, and `--assert-unchanged`, refuse it.
//
// The first two together are the claim a build depends on: a file the run
// writes and the plan does not name is an undeclared output, which ninja
// refuses on the rebuild of a directory (`depfile mentions … as an output, but
// no such output was declared`) where a fresh build never reads it back.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn codegen() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

const DOOR: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="closed" name="Door">
  <state id="closed"><transition event="open" target="opened"/></state>
  <state id="opened"><transition event="close" target="closed"/></state>
</scxml>
"#;

/// A statechart whose `<invoke>` carries inline content, which generation
/// synthesizes into a child document of its own. The committed fixture, so the
/// test is about what the repository's own fixtures do.
fn invoking_fixture(dir: &Path) -> PathBuf {
    let stem = "donedata_local_invoke";
    let source = repo_root()
        .join("integration_resources")
        .join(stem)
        .join(format!("{stem}.scxml"));
    let staged = dir.join(format!("{stem}.scxml"));
    fs::copy(&source, &staged).expect("stage the fixture");
    staged
}

fn write_door(dir: &Path) -> PathBuf {
    let path = dir.join("door.scxml");
    fs::write(&path, DOOR).expect("write the document");
    path
}

/// `sce-codegen [flags] generate <scxml> -o <out> -l <lang>`.
fn generate(flags: &[&str], scxml: &Path, out: &Path, language: &str) -> Output {
    Command::new(codegen())
        .args(flags)
        .arg("generate")
        .arg(scxml)
        .arg("-o")
        .arg(out)
        .args(["-l", language])
        .output()
        .expect("run sce-codegen")
}

fn stdout_lines(out: &Output) -> Vec<String> {
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// Every file under `dir`, as a path relative to it.
fn files_under(dir: &Path) -> BTreeSet<String> {
    fn walk(root: &Path, dir: &Path, files: &mut BTreeSet<String>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(root, &path, files);
            } else {
                let relative = path.strip_prefix(root).expect("under the root");
                files.insert(relative.to_string_lossy().into_owned());
            }
        }
    }
    let mut files = BTreeSet::new();
    walk(dir, dir, &mut files);
    files
}

/// The plan's lines as paths relative to `out`.
fn planned(out: &Output, dir: &Path) -> BTreeSet<String> {
    stdout_lines(out)
        .iter()
        .map(|line| {
            Path::new(line)
                .strip_prefix(dir)
                .unwrap_or_else(|_| panic!("`{line}` is not under {}", dir.display()))
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

#[test]
fn a_plan_writes_nothing_not_even_the_directory() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let door = write_door(tmp.path());
    let out = tmp.path().join("out");
    let before = files_under(tmp.path());

    let plan = generate(&["--plan"], &door, &out, "cpp");

    assert!(plan.status.success(), "the plan failed: {}", stderr(&plan));
    assert!(!out.exists(), "the plan created {}", out.display());
    assert_eq!(files_under(tmp.path()), before, "the plan wrote a file");
}

#[test]
fn a_plan_names_exactly_the_files_the_same_run_writes() {
    for language in ["cpp", "c11"] {
        let tmp = tempfile::tempdir().expect("temp dir");
        let fixture = invoking_fixture(tmp.path());
        let planned_into = tmp.path().join("planned");
        let written_into = tmp.path().join("written");
        let before = files_under(tmp.path());

        let plan = generate(&["--plan"], &fixture, &planned_into, language);

        assert!(plan.status.success(), "{language}: {}", stderr(&plan));
        // Nowhere: a generation extracts an inline invoke's child document
        // beside the document it read as well as into the output directory,
        // and the plan has to leave that one alone too.
        assert_eq!(
            files_under(tmp.path()),
            before,
            "{language}: the plan wrote a file, beside the document or in the output directory"
        );

        let real = generate(&[], &fixture, &written_into, language);
        assert!(real.status.success(), "{language}: {}", stderr(&real));
        let named = planned(&plan, &planned_into);
        let wrote = files_under(&written_into);
        assert_eq!(
            named, wrote,
            "{language}: the plan names {named:?} and the run wrote {wrote:?}: a file only \
             the run wrote is an output no build declared"
        );
    }
}

#[test]
fn a_plan_prints_paths_and_no_manifest() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let door = write_door(tmp.path());
    let out = tmp.path().join("out");

    let plan = generate(&["--plan"], &door, &out, "cpp");

    let lines = stdout_lines(&plan);
    assert!(!lines.is_empty(), "the plan named nothing");
    for line in &lines {
        assert!(
            !line.starts_with('{'),
            "a manifest line among the paths, which a reader takes each line to be: {line}"
        );
        assert!(
            Path::new(line).starts_with(&out),
            "`{line}` is not a path under the output directory"
        );
    }
}

#[test]
fn a_document_that_synthesizes_children_has_them_in_its_plan() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let fixture = invoking_fixture(tmp.path());
    let out = tmp.path().join("out");

    let plan = generate(&["--plan"], &fixture, &out, "cpp");

    let named = planned(&plan, &out);
    let synthesized: Vec<&String> = named
        .iter()
        .filter(|name| name.contains("__sce_synth_invoke__") && name.ends_with("_sm.h"))
        .collect();
    assert!(
        synthesized.len() >= 2,
        "donedata_local_invoke synthesizes two children (inv_param, inv_content); the plan \
         names {named:?}"
    );
    assert!(
        named.contains("donedata_local_invoke_sm.h"),
        "the parent's own header is not in the plan: {named:?}"
    );
}

#[test]
fn a_document_the_generation_refuses_is_refused_by_the_plan() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let broken = tmp.path().join("broken.scxml");
    fs::write(
        &broken,
        "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\"><state",
    )
    .expect("write");
    let out = tmp.path().join("out");

    let real = generate(&[], &broken, &out, "cpp");
    let plan = generate(&["--plan"], &broken, &tmp.path().join("planned"), "cpp");

    assert!(
        !real.status.success(),
        "the plain run accepted a broken document"
    );
    assert_eq!(
        plan.status.code(),
        real.status.code(),
        "the plan's exit status is not the generation's: {}",
        stderr(&plan)
    );
    assert!(
        stdout_lines(&plan).is_empty(),
        "a refused run named files: {:?}",
        stdout_lines(&plan)
    );
}

#[test]
fn what_is_on_disk_does_not_change_the_answer() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let fixture = invoking_fixture(tmp.path());
    let out = tmp.path().join("out");

    let fresh = generate(&["--plan"], &fixture, &out, "cpp");
    // The tree written for real, then a stray file beside it: neither is a
    // reason for the plan to differ, because it judges nothing against disk.
    let real = generate(&[], &fixture, &out, "cpp");
    assert!(real.status.success(), "{}", stderr(&real));
    fs::write(out.join("stray.txt"), "left over").expect("write a stray file");
    let after = generate(&["--plan"], &fixture, &out, "cpp");

    assert!(fresh.status.success() && after.status.success());
    assert_eq!(
        stdout_lines(&fresh),
        stdout_lines(&after),
        "the plan depends on what is already in the output directory"
    );
}

#[test]
fn a_command_the_plan_cannot_serve_refuses_it() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let door = write_door(tmp.path());

    let check = Command::new(codegen())
        .args(["--plan", "check"])
        .arg(&door)
        .output()
        .expect("run sce-codegen");

    assert!(!check.status.success(), "`--plan check` was accepted");
    assert!(
        stderr(&check).contains("--plan applies only to `generate`"),
        "the refusal does not say why: {}",
        stderr(&check)
    );
}

#[test]
fn a_plan_and_an_assertion_are_not_asked_together() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let door = write_door(tmp.path());

    let both = generate(
        &["--plan", "--assert-unchanged"],
        &door,
        &tmp.path().join("out"),
        "cpp",
    );

    assert!(!both.status.success(), "both flags were accepted");
    assert!(
        stderr(&both).contains("cannot be used with"),
        "the refusal does not name the conflict: {}",
        stderr(&both)
    );
}
