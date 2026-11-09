// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A build declares a statechart's machine by the name the generator gives
//! it, read from the plan, and never by the file stem it spelled itself.
//!
//! # What was wrong
//!
//! A statechart's machine name is its file stem written as an identifier
//! (docs/SCE_ACCEPTED_SUBSET.md, `sce_build::model::statechart_machine_name`):
//! `job-tester.scxml` is the machine `job_tester`, written to
//! `job_tester_sm.h`. `cmake/SCECodegenPlan.cmake` named the machine's files
//! from the stem as the caller gave it (`job-tester_sm.h`), so a hyphenated or
//! camelCase statechart generated through `sce_add_state_machine` (which calls
//! `sce_codegen_generation` with the document alone) was refused by the plan
//! that declared it, as a generation that does not name the file its command
//! produces; and a child generation waited for a writer header no command
//! produces.
//!
//! # What is held
//!
//! A parent `job-tester.scxml` starts a child by `src="worker-unit.scxml"`;
//! the documents are written here, so the test reads no tracked document.
//!
//! * a plan asked with `--plan-machines` states the machine name of the run's
//!   statechart and of each planned generation's, as
//!   `\tmachine\t<generation>\t<name>`, and a plan asked without it states
//!   none ([`a_plan_states_the_machine_its_stem_spells_as_an_identifier`]);
//! * a document rejected at parse time (§scxml-5.8 refuses it to be
//!   generated) states the machine its stub stands in for
//!   ([`a_rejected_statechart_states_the_machine_its_stub_stands_in_for`]);
//! * through a real `cmake`, skipped where there is none of 3.20 or later,
//!   the module declares the parent's machine, the child's, the child's
//!   dependency on the header that writes its document, and the headers
//!   `sce_codegen_generation` hands a target, each by the machine name the
//!   plan states
//!   ([`the_module_declares_a_hyphenated_statechart_by_its_machine_name`]).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The parent, whose stem holds a hyphen.
const PARENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="running">
    <state id="running">
        <invoke type="http://www.w3.org/TR/scxml/" src="worker-unit.scxml"/>
    </state>
</scxml>
"#;

/// The child the parent starts by `src`, whose stem holds a hyphen too.
const CHILD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="idle">
    <state id="idle"/>
</scxml>
"#;

/// A document §scxml-5.8 refuses to generate: its external script cannot be
/// loaded, so the generator writes a stub in place of its machine.
const REJECTED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s0">
    <script src="/nonexistent_sce_plan_machine_probe.js"/>
    <state id="s0"><transition target="done"/></state>
    <final id="done"/>
</scxml>
"#;

const LANGUAGES: &[&str] = &["cpp", "c11"];

/// The parent and its child written where their author keeps them; the
/// directory (kept alive) and the parent's path.
fn authored() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("authored");
    std::fs::create_dir_all(&root).expect("make the documents' directory");
    std::fs::write(root.join("job-tester.scxml"), PARENT).expect("write the parent");
    std::fs::write(root.join("worker-unit.scxml"), CHILD).expect("write the child");
    (dir, root.join("job-tester.scxml"))
}

/// `sce-codegen [flags] generate <document> -l <language> --no-format -o <out>
/// --input-root <root>`, as the module asks a plan.
fn plan(flags: &[&str], document: &Path, language: &str, out: &Path, root: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .args(flags)
        .arg("generate")
        .arg(document)
        .args(["-l", language, "--no-format", "-o"])
        .arg(out)
        .arg("--input-root")
        .arg(root)
        .output()
        .expect("spawn sce-codegen")
}

fn succeeded(output: &Output, what: &str) -> String {
    assert!(
        output.status.success(),
        "{what} failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// The machine names an answer states, by generation: the run's own under
/// the empty name, each planned generation's under the file name of its
/// document.
fn stated_machines(answer: &str) -> BTreeMap<String, String> {
    let mut stated = BTreeMap::new();
    for line in answer.lines() {
        let Some(statement) = line.strip_prefix("\tmachine\t") else {
            continue;
        };
        let (generation, name) = statement
            .split_once('\t')
            .unwrap_or_else(|| panic!("a machine line with no name: {line:?}"));
        let generation = Path::new(generation)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let previous = stated.insert(generation.clone(), name.to_string());
        assert!(
            previous.is_none(),
            "the machine of the generation {generation:?} is stated twice: {answer}"
        );
    }
    stated
}

/// The file names of the run's own lines: every line with no tab.
fn own_files(answer: &str) -> Vec<String> {
    answer
        .lines()
        .filter(|line| !line.contains('\t'))
        .filter_map(|line| Path::new(line).file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn a_plan_states_the_machine_its_stem_spells_as_an_identifier() {
    for &language in LANGUAGES {
        let (dir, parent) = authored();
        let root = parent.parent().expect("the parent's directory");
        let out = dir.path().join("out");

        let answer = succeeded(
            &plan(
                &["--plan", "--plan-children", "--plan-machines"],
                &parent,
                language,
                &out,
                root,
            ),
            &format!("[{language}] the plan of the parent"),
        );
        let expected: BTreeMap<String, String> = [
            (String::new(), "job_tester".to_string()),
            ("worker-unit.scxml".to_string(), "worker_unit".to_string()),
        ]
        .into_iter()
        .collect();
        assert_eq!(
            stated_machines(&answer),
            expected,
            "[{language}] the plan does not state the machine name of the run and of the \
             child's generation as their stems written as identifiers:\n{answer}"
        );
        let own = own_files(&answer);
        assert!(
            own.iter().any(|file| file == "job_tester_sm.h")
                && !own.iter().any(|file| file.starts_with("job-tester_sm")),
            "[{language}] the run's own files are not named by the machine it states: {own:?}"
        );

        let unasked = succeeded(
            &plan(
                &["--plan", "--plan-children"],
                &parent,
                language,
                &out,
                root,
            ),
            &format!("[{language}] the plan of the parent without --plan-machines"),
        );
        assert!(
            stated_machines(&unasked).is_empty(),
            "[{language}] a plan not asked for the machines states them:\n{unasked}"
        );
    }
}

#[test]
fn a_rejected_statechart_states_the_machine_its_stub_stands_in_for() {
    for &language in LANGUAGES {
        let dir = tempfile::tempdir().expect("tempdir");
        let document = dir.path().join("rejected-doc.scxml");
        std::fs::write(&document, REJECTED).expect("write the rejected document");
        let out = dir.path().join("out");

        let answer = succeeded(
            &plan(
                &["--plan", "--plan-machines"],
                &document,
                language,
                &out,
                dir.path(),
            ),
            &format!("[{language}] the plan of the rejected document"),
        );
        let expected: BTreeMap<String, String> = [(String::new(), "rejected_doc".to_string())]
            .into_iter()
            .collect();
        assert_eq!(
            stated_machines(&answer),
            expected,
            "[{language}] the plan of a rejected document does not state the machine its stub \
             stands in for:\n{answer}"
        );
        assert!(
            own_files(&answer)
                .iter()
                .any(|file| file == "rejected_doc_sm.h"),
            "[{language}] the stub is not written under the machine's name:\n{answer}"
        );
    }
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

/// What the probe printed, `PROBE <language> <what>=<value>`, by
/// `<language> <what>`.
fn probed(log: &str) -> BTreeMap<String, String> {
    log.lines()
        .filter_map(|line| line.split_once("PROBE "))
        .filter_map(|(_, said)| said.split_once('='))
        .map(|(what, value)| (what.to_string(), value.to_string()))
        .collect()
}

/// The file name of each path of a CMake list.
fn names(list: &str) -> Vec<String> {
    list.split(';')
        .filter(|path| !path.is_empty())
        .filter_map(|path| Path::new(path).file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn the_module_declares_a_hyphenated_statechart_by_its_machine_name() {
    match cmake_version() {
        Some(version) if version >= (3, 20) => {}
        other => {
            eprintln!("SKIP: no cmake 3.20 or later on PATH ({other:?})");
            return;
        }
    }
    let (dir, parent) = authored();
    let root = parent.parent().expect("the parent's directory");
    let module = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .join("cmake/SCECodegenPlan.cmake");
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).expect("project dir");
    std::fs::write(
        project.join("CMakeLists.txt"),
        format!(
            "cmake_minimum_required(VERSION 3.20)\n\
             project(sce_machine_name_probe NONE)\n\
             include(\"{module}\")\n\
             set(_LANGUAGES cpp c11)\n\
             foreach(_LANG IN LISTS _LANGUAGES)\n\
                 sce_codegen_plan_request(STEM job-tester DOCUMENT \"{parent}\"\n\
                     OUTPUT_DIR \"${{CMAKE_BINARY_DIR}}/${{_LANG}}\" LANGUAGE ${{_LANG}}\n\
                     INPUT_ROOT \"{root}\" CHILD_GENERATIONS)\n\
             endforeach()\n\
             sce_codegen_plan_prefetch()\n\
             foreach(_LANG IN LISTS _LANGUAGES)\n\
                 sce_codegen_plan(_PARENT STEM job-tester DOCUMENT \"{parent}\"\n\
                     OUTPUT_DIR \"${{CMAKE_BINARY_DIR}}/${{_LANG}}\" LANGUAGE ${{_LANG}}\n\
                     INPUT_ROOT \"{root}\" CHILD_GENERATIONS)\n\
                 message(STATUS \"PROBE ${{_LANG}} MACHINE=${{_PARENT_MACHINE_NAME}}\")\n\
                 message(STATUS \"PROBE ${{_LANG}} OUTPUTS=${{_PARENT_OUTPUTS}}\")\n\
                 message(STATUS \"PROBE ${{_LANG}} DEPFILE=${{_PARENT_DEPFILE}}\")\n\
                 message(STATUS \"PROBE ${{_LANG}} GENERATIONS=${{_PARENT_GENERATIONS}}\")\n\
                 sce_codegen_plan(_CHILD FROM _PARENT CHILD worker-unit)\n\
                 message(STATUS \"PROBE ${{_LANG}} CHILD_MACHINE=${{_CHILD_MACHINE_NAME}}\")\n\
                 message(STATUS \"PROBE ${{_LANG}} CHILD_OUTPUTS=${{_CHILD_OUTPUTS}}\")\n\
                 message(STATUS \"PROBE ${{_LANG}} CHILD_DEPENDS=${{_CHILD_DEPENDS}}\")\n\
             endforeach()\n\
             # The call `sce_add_state_machine` makes: the document alone, its\n\
             # stem taken from its file name, its children planned with it.\n\
             sce_codegen_generation(_DECLARED\n\
                 DOCUMENT \"{parent}\"\n\
                 OUTPUT_DIR \"${{CMAKE_BINARY_DIR}}/declared\"\n\
                 LANGUAGE cpp\n\
                 CHILD_GENERATIONS)\n\
             file(GENERATE OUTPUT \"${{CMAKE_BINARY_DIR}}/declared_headers.txt\"\n\
                 CONTENT \"${{_DECLARED_HEADERS}}\")\n",
            module = module.display(),
            parent = parent.display(),
            root = root.display(),
        ),
    )
    .expect("write the probe");

    let build = dir.path().join("build");
    let out = Command::new("cmake")
        .arg("-S")
        .arg(&project)
        .arg("-B")
        .arg(&build)
        .arg(format!(
            "-DSCE_CODEGEN={}",
            env!("CARGO_BIN_EXE_sce-codegen")
        ))
        .output()
        .expect("run cmake");
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.status.success(),
        "the module refused a hyphenated statechart's plan:\n{log}"
    );
    let said = probed(&log);
    let read = |language: &str, what: &str| -> String {
        said.get(&format!("{language} {what}"))
            .cloned()
            .unwrap_or_else(|| panic!("the probe did not say {language} {what}:\n{log}"))
    };

    for &language in LANGUAGES {
        // A C11 machine is its header and its source, and its depfile sits
        // beside the source; a C++ machine is its header.
        let c11 = language == "c11";
        let machine: &[&str] = if c11 {
            &["job_tester_sm.h", "job_tester_sm.c"]
        } else {
            &["job_tester_sm.h"]
        };
        let depfile = if c11 {
            "job_tester_sm.c.d"
        } else {
            "job_tester_sm.h.d"
        };
        let child_machine: &[&str] = if c11 {
            &["worker_unit_sm.h", "worker_unit_sm.c"]
        } else {
            &["worker_unit_sm.h"]
        };
        assert_eq!(
            read(language, "MACHINE"),
            "job_tester",
            "[{language}]\n{log}"
        );
        assert_eq!(
            names(&read(language, "OUTPUTS")),
            machine,
            "[{language}]\n{log}"
        );
        assert_eq!(
            names(&read(language, "DEPFILE")),
            [depfile],
            "[{language}]\n{log}"
        );
        assert_eq!(
            read(language, "GENERATIONS"),
            "worker-unit",
            "[{language}]\n{log}"
        );
        assert_eq!(
            read(language, "CHILD_MACHINE"),
            "worker_unit",
            "[{language}]\n{log}"
        );
        assert_eq!(
            names(&read(language, "CHILD_OUTPUTS")),
            child_machine,
            "[{language}]\n{log}"
        );
        // The child's command waits for its document and for the header of
        // the machine of the generation that writes it, the parent's.
        let depends = names(&read(language, "CHILD_DEPENDS"));
        assert!(
            depends.iter().any(|name| name == "worker-unit.scxml")
                && depends.iter().any(|name| name == "job_tester_sm.h"),
            "[{language}] the child's generation does not wait for the parent's machine by \
             the name the plan states: {depends:?}\n{log}"
        );
    }

    let declared = std::fs::read_to_string(build.join("declared_headers.txt"))
        .expect("the build system was generated with the generation's headers");
    let declared = names(&declared);
    for header in ["job_tester_sm.h", "worker_unit_sm.h"] {
        assert!(
            declared.iter().any(|name| name == header),
            "the headers `sce_codegen_generation` hands a target do not name {header}: \
             {declared:?}"
        );
    }
}
