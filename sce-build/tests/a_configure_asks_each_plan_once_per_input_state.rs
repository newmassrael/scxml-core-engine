// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The CMake module every build-time route asks the generator's plans through
// (cmake/SCECodegenPlan.cmake) keeps each answer between configures, and asks
// the ones it does not keep in batches (`sce-codegen plan-batch`). Held here,
// through a real `cmake`, and skipped where there is no `cmake` 3.20 or later,
// on three plans of the shapes the routes ask: a document alone, a document
// whose generation writes a child document with a generation of its own, and
// a parent staged with its deploy when the tree is configured, whose deploy
// places its in-line child on a partition of its own; and on a generation
// declared through `sce_codegen_generation` in the top-level directory:
//   - plans asked ahead go to the generator in one batch, and a plan no
//     request asked ahead is sent alone and counted so;
//   - a generation is asked ahead when it is called, declared when its
//     directory ends, after the settle of the cache was moved behind it, and
//     the generator expression it hands back names the machine it planned;
//   - a configure with nothing changed asks the generator nothing, and reads
//     the answers it was given before;
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
    /// Of them, the ones answered from what was kept when they were asked,
    /// whether a batch asked ahead had just kept them or an earlier
    /// configure had.
    kept: u32,
    /// Requests sent to the generator, whenever they were sent.
    sent: u32,
    /// The batches they were sent in.
    batches: u32,
    /// Of the requests sent, the ones sent one to a batch when they were
    /// asked, because no request asked them ahead.
    alone: u32,
}

fn costs(log: &str) -> Costs {
    let said = regex::Regex::new(
        r"generation plans: (\d+) asked, (\d+) answered from the cache; (\d+) sent to sce-codegen in (\d+) batch\(es\), (\d+) of them alone",
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
        batches: count(4),
        alone: count(5),
    }
}

fn spent(asked: u32, kept: u32, sent: u32, batches: u32, alone: u32) -> Costs {
    Costs {
        asked,
        kept,
        sent,
        batches,
        alone,
    }
}

/// The files the probe's plans declared, as it printed them.
fn planned_files(log: &str) -> String {
    log.lines()
        .filter_map(|line| {
            line.split_once("PROBE_FILES=")
                .map(|(_, files)| files.to_string())
        })
        .collect::<Vec<_>>()
        .join("|")
}

/// Date `path` now, as an edit would.
fn touch(path: &Path) {
    fs::File::options()
        .write(true)
        .open(path)
        .and_then(|file| file.set_modified(SystemTime::now()))
        .unwrap_or_else(|e| panic!("touch {}: {e}", path.display()));
}

/// One project asks three plans through the module, configured again and
/// again. The generator is a copy, so it can be dated as a rebuilt one is
/// without touching the one cargo built.
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
    fn copy(from: &str, to: &Path) {
        fs::create_dir_all(to.parent().expect("a directory")).expect("make the directory");
        fs::copy(repo_root().join(from), to).expect("copy a fixture");
    }
    // A document alone.
    let document = sandbox.path().join("fixture/test144.scxml");
    copy("resources/144/test144.scxml", &document);
    // A document whose generation writes the stub of a hybrid `<invoke>`,
    // whose own generation the plan holds, with the child it names beside it.
    let hybrid = sandbox.path().join("hybrid/test216.scxml");
    copy("resources/216/test216.scxml", &hybrid);
    copy(
        "resources/216/test216sub1.scxml",
        &sandbox.path().join("hybrid/test216sub1.scxml"),
    );
    let generator = sandbox.path().join("bin/sce-codegen");
    fs::create_dir_all(generator.parent().expect("bin dir")).expect("bin dir");
    fs::copy(codegen(), &generator).expect("copy the generator");

    let project = sandbox.path().join("project");
    fs::create_dir_all(&project).expect("project dir");
    fs::write(
        project.join("CMakeLists.txt"),
        format!(
            "cmake_minimum_required(VERSION 3.20)\n\
             project(sce_codegen_plan_cache_probe NONE)\n\
             include(\"{module}\")\n\
             # The parent and its deploy, staged when the tree is configured, as\n\
             # the test tree stages the Mesh rule-3 override.\n\
             set(MESH \"${{CMAKE_BINARY_DIR}}/mesh\")\n\
             file(MAKE_DIRECTORY \"${{MESH}}\")\n\
             configure_file(\"{mesh}/parent_synth_inline.scxml\"\n\
                 \"${{MESH}}/parent_synth_inline.scxml\" COPYONLY)\n\
             configure_file(\"{mesh}/deploy_synth_inline.yaml\"\n\
                 \"${{MESH}}/deploy_synth_inline.yaml\" COPYONLY)\n\
             # The two asked ahead, in one batch.\n\
             sce_codegen_plan_request(STEM test216 DOCUMENT \"{hybrid}\"\n\
                 OUTPUT_DIR \"${{CMAKE_BINARY_DIR}}/generated\" LANGUAGE cpp\n\
                 INPUT_ROOT \"{hybrid_dir}\" CHILD_GENERATIONS)\n\
             sce_codegen_plan_request(STEM parent_synth_inline\n\
                 DOCUMENT \"${{MESH}}/parent_synth_inline.scxml\" OUTPUT_DIR \"${{MESH}}\"\n\
                 LANGUAGE cpp CHILD_GENERATIONS\n\
                 ARGS --deploy \"${{MESH}}/deploy_synth_inline.yaml\")\n\
             sce_codegen_plan_prefetch()\n\
             sce_codegen_plan(_HYBRID STEM test216 DOCUMENT \"{hybrid}\"\n\
                 OUTPUT_DIR \"${{CMAKE_BINARY_DIR}}/generated\" LANGUAGE cpp\n\
                 INPUT_ROOT \"{hybrid_dir}\" CHILD_GENERATIONS)\n\
             message(STATUS \"PROBE_FILES=${{_HYBRID_FILES}};${{_HYBRID_GENERATIONS}}\")\n\
             sce_codegen_plan(_DEPLOYED STEM parent_synth_inline\n\
                 DOCUMENT \"${{MESH}}/parent_synth_inline.scxml\" OUTPUT_DIR \"${{MESH}}\"\n\
                 LANGUAGE cpp CHILD_GENERATIONS\n\
                 ARGS --deploy \"${{MESH}}/deploy_synth_inline.yaml\")\n\
             message(STATUS \"PROBE_FILES=${{_DEPLOYED_FILES}};${{_DEPLOYED_GENERATIONS}}\")\n\
             # The one asked when it is declared, which is sent alone.\n\
             sce_codegen_plan(_PROBE\n\
                 STEM test144\n\
                 DOCUMENT \"{document}\"\n\
                 OUTPUT_DIR \"${{CMAKE_BINARY_DIR}}/generated\"\n\
                 LANGUAGE cpp\n\
                 CHILD_GENERATIONS\n\
                 ARGS ${{PROBE_ARGS}})\n\
             message(STATUS \"PROBE_FILES=${{_PROBE_FILES}}\")\n\
             # A generation, asked ahead when it is called and declared when\n\
             # this directory ends, after its batch; the headers it hands back\n\
             # are read once the build system is generated.\n\
             sce_codegen_generation(_DECLARED\n\
                 DOCUMENT \"{document}\"\n\
                 OUTPUT_DIR \"${{CMAKE_BINARY_DIR}}/declared\"\n\
                 LANGUAGE cpp)\n\
             file(GENERATE OUTPUT \"${{CMAKE_BINARY_DIR}}/declared_headers.txt\"\n\
                 CONTENT \"${{_DECLARED_HEADERS}}\")\n",
            module = text(&repo_root().join("cmake/SCECodegenPlan.cmake")),
            mesh = text(&repo_root().join("tests/mesh")),
            hybrid = text(&hybrid),
            hybrid_dir = text(hybrid.parent().expect("hybrid dir")),
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
        spent(4, 3, 4, 3, 1),
        "the two plans asked ahead were not sent in one batch, the third alone and the \
         generation's in the batch its directory's end sends:\n{first}"
    );
    let files = planned_files(&first);
    assert!(
        files.contains("test144_sm.h") && files.contains("parent_synth_inline_sm.h"),
        "the plans do not name the machines: {files}"
    );
    let declared = fs::read_to_string(build.join("declared_headers.txt"))
        .expect("the build system was generated with the generation's headers");
    assert!(
        declared
            .split(';')
            .any(|header| header.ends_with("/declared/test144_sm.h")),
        "the generator expression the generation handed back does not name the machine its \
         declaration planned: {declared:?}"
    );

    // A second configure that asked any plan again would say so here; one whose
    // deferred declaration came after the settle would ask the generation's,
    // whose answer the settle had removed as asked by no plan.
    let second = configure(&[]);
    assert_eq!(
        costs(&second),
        spent(4, 4, 0, 0, 0),
        "a configure with nothing changed asked the generator again; the reasons the \
         configure gives:\n{second}"
    );
    assert_eq!(
        planned_files(&second),
        files,
        "the kept answers are not the answers"
    );

    touch(&document);
    let third = configure(&[]);
    assert_eq!(
        costs(&third),
        spent(4, 4, 2, 1, 0),
        "the two plans of an edited document were not asked again, together, in the batch \
         before any plan is asked for:\n{third}"
    );
    assert_eq!(planned_files(&third), files);

    touch(&generator);
    let fourth = configure(&[]);
    assert_eq!(
        costs(&fourth),
        spent(4, 4, 4, 1, 0),
        "a rebuilt generator's plans were not asked again in one batch before any plan is \
         asked for:\n{fourth}"
    );

    let fifth = configure(&["-DPROBE_ARGS=--limit;probe_bound=3"]);
    assert_eq!(
        costs(&fifth),
        spent(4, 3, 1, 1, 1),
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
        4,
        "an answer no plan asks for any more was kept, or one a plan asks was removed: \
         {keys:?}"
    );
}
