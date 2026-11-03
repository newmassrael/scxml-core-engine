// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The CMake module every build-time route asks the generator's plans through
// (cmake/SCECodegenPlan.cmake) keeps each answer between configures, and asks
// the ones it does not keep in batches (`sce-codegen plan-batch`). Held here,
// through a real `cmake`, and skipped where there is no `cmake` 3.20 or later:
//   - a configure with nothing changed asks the generator nothing, and reads
//     the answer it was given before;
//   - an edited document and a rebuilt generator are asked again, in the batch
//     that refreshes what the last configure asked, before the plan is asked;
//   - another argument is another plan, which no kept answer answers;
//   - an answer no plan asks for any more is removed.
//
// Unix only: the probe writes its paths into CMake text as they are, which a
// path with a backslash would not survive.
#![cfg(unix)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

fn codegen() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn text(path: &Path) -> String {
    path.display().to_string()
}

/// The version of the `cmake` on PATH, when there is one.
fn cmake_version() -> Option<(u32, u32)> {
    let out = Command::new("cmake").arg("--version").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let said = String::from_utf8_lossy(&out.stdout).into_owned();
    let version = said.lines().next()?.strip_prefix("cmake version ")?;
    let mut parts = version.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    Some((major, minor))
}

/// What a configure said its plans cost.
#[derive(Debug, PartialEq, Eq)]
struct Costs {
    /// Plans the configure asked for.
    asked: u32,
    /// Of them, the ones answered from what was kept when they were asked.
    kept: u32,
    /// Requests sent to the generator, whenever they were sent.
    sent: u32,
}

fn costs(log: &str) -> Costs {
    let said = regex::Regex::new(
        r"generation plans: (\d+) asked, (\d+) answered from the cache; (\d+) sent to sce-codegen",
    )
    .expect("pattern");
    let found = said
        .captures(log)
        .unwrap_or_else(|| panic!("the configure did not say what its plans cost:\n{log}"));
    let count = |at: usize| found[at].parse().expect("a count");
    Costs {
        asked: count(1),
        kept: count(2),
        sent: count(3),
    }
}

/// The files the probe's plan declared, as it printed them.
fn planned_files(log: &str) -> String {
    log.lines()
        .find_map(|line| {
            line.split_once("PROBE_FILES=")
                .map(|(_, files)| files.to_string())
        })
        .unwrap_or_else(|| panic!("the probe did not print its plan:\n{log}"))
}

/// Date `path` now, as an edit would.
fn touch(path: &Path) {
    fs::File::options()
        .write(true)
        .open(path)
        .and_then(|file| file.set_modified(SystemTime::now()))
        .unwrap_or_else(|e| panic!("touch {}: {e}", path.display()));
}

/// One project asks one plan through the module, configured again and again.
/// The generator is a copy, so it can be dated as a rebuilt one is without
/// touching the one cargo built.
#[test]
fn a_configure_reuses_a_kept_plan_until_its_document_an_argument_or_the_generator_changes() {
    match cmake_version() {
        Some(version) if version >= (3, 20) => {}
        other => {
            eprintln!("SKIP: no cmake 3.20 or later on PATH ({other:?})");
            return;
        }
    }
    let sandbox = tempfile::tempdir().expect("temp dir");
    let fixture = sandbox.path().join("fixture");
    fs::create_dir_all(&fixture).expect("fixture dir");
    let document = fixture.join("test144.scxml");
    fs::copy(repo_root().join("resources/144/test144.scxml"), &document).expect("copy the fixture");
    let bin = sandbox.path().join("bin");
    fs::create_dir_all(&bin).expect("bin dir");
    let generator = bin.join("sce-codegen");
    fs::copy(codegen(), &generator).expect("copy the generator");

    let project = sandbox.path().join("project");
    fs::create_dir_all(&project).expect("project dir");
    fs::write(
        project.join("CMakeLists.txt"),
        format!(
            "cmake_minimum_required(VERSION 3.20)\n\
             project(sce_codegen_plan_cache_probe NONE)\n\
             include(\"{module}\")\n\
             sce_codegen_plan(_PROBE\n\
                 STEM test144\n\
                 DOCUMENT \"{document}\"\n\
                 OUTPUT_DIR \"${{CMAKE_BINARY_DIR}}/generated\"\n\
                 LANGUAGE cpp\n\
                 CHILD_GENERATIONS\n\
                 ARGS ${{PROBE_ARGS}})\n\
             message(STATUS \"PROBE_FILES=${{_PROBE_FILES}}\")\n",
            module = text(&repo_root().join("cmake/SCECodegenPlan.cmake")),
            document = text(&document),
        ),
    )
    .expect("write the probe");

    let build = sandbox.path().join("build");
    let configure = |extra: &[&str]| -> String {
        let out = Command::new("cmake")
            .arg("-S")
            .arg(&project)
            .arg("-B")
            .arg(&build)
            .arg(format!("-DSCE_CODEGEN={}", text(&generator)))
            .args(extra)
            .output()
            .expect("run cmake");
        let log = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(out.status.success(), "the probe did not configure:\n{log}");
        log
    };

    let first = configure(&[]);
    assert_eq!(
        costs(&first),
        Costs {
            asked: 1,
            kept: 0,
            sent: 1
        },
        "{first}"
    );
    let files = planned_files(&first);
    assert!(
        files.contains("test144_sm.h"),
        "the plan does not name the machine: {files}"
    );

    let second = configure(&[]);
    assert_eq!(
        costs(&second),
        Costs {
            asked: 1,
            kept: 1,
            sent: 0
        },
        "a configure with nothing changed asked the generator again:\n{second}"
    );
    assert_eq!(
        planned_files(&second),
        files,
        "the kept answer is not the answer"
    );

    touch(&document);
    let third = configure(&[]);
    assert_eq!(
        costs(&third),
        Costs {
            asked: 1,
            kept: 1,
            sent: 1
        },
        "the plan of an edited document was not asked again, in the batch before any plan \
         is asked for:\n{third}"
    );
    assert_eq!(planned_files(&third), files);

    touch(&generator);
    let fourth = configure(&[]);
    assert_eq!(
        costs(&fourth),
        Costs {
            asked: 1,
            kept: 1,
            sent: 1
        },
        "a rebuilt generator's plan was not asked again before the plan is asked for:\n{fourth}"
    );

    let fifth = configure(&["-DPROBE_ARGS=--limit;probe_bound=3"]);
    assert_eq!(
        costs(&fifth),
        Costs {
            asked: 1,
            kept: 0,
            sent: 1
        },
        "a plan asked with another argument was answered from another plan's answer:\n{fifth}"
    );
    assert_eq!(planned_files(&fifth), files);

    let entries = build.join("sce-codegen-plans").join("entries");
    let keys: BTreeSet<String> = fs::read_dir(&entries)
        .expect("the answers are kept")
        .map(|entry| {
            let name = entry.expect("an entry").file_name();
            let name = name.to_string_lossy();
            name.split('.').next().unwrap_or_default().to_string()
        })
        .collect();
    assert_eq!(
        keys.len(),
        1,
        "the answer no plan asks for any more was kept: {keys:?}"
    );
}
