// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A build is configured by asking the generator which files each generation
// writes (`sce-codegen --plan generate`), and a tree asks several hundred such
// plans. What lets a build ask each one once per state of its inputs, and many
// at once, is the generator's, and is held here:
//   - `--reads` lists what a run read: the prerequisites a depfile of the
//     same run names, absolute, less every file the run or a child generation
//     it planned produces; asking for the list changes no answer, a
//     generation writes the list its plan writes, and a command that records
//     nothing it reads refuses to be asked for one;
//   - `plan-batch` answers each request byte for byte as the plan of it alone
//     does, keeps a refusal as the refused run's stderr, dates each answer when
//     its request was written, and runs nothing but plans.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
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

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn text(path: &Path) -> String {
    path.display().to_string()
}

fn lines_of(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
        .lines()
        .map(str::to_string)
        .collect()
}

/// `sce-codegen <arguments>`.
fn run(arguments: &[String]) -> Output {
    Command::new(codegen())
        .args(arguments)
        .output()
        .expect("run sce-codegen")
}

fn modified(path: &Path) -> SystemTime {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or_else(|e| panic!("the date of {}: {e}", path.display()))
}

const DOOR: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="closed" name="Door">
  <state id="closed"><transition event="open" target="opened"/></state>
  <state id="opened"><transition event="close" target="closed"/></state>
</scxml>
"#;

/// The directory of a committed conformance fixture.
fn fixture_dir(id: &str) -> PathBuf {
    repo_root().join("resources").join(id)
}

/// The plan of `test<id>` asked as the conformance route asks it, with the
/// plans of its children: `--plan --plan-children generate`, its input root
/// its own directory.
fn conformance_plan(id: &str, language: &str, out: &Path, reads: Option<&Path>) -> Vec<String> {
    let dir = fixture_dir(id);
    let mut arguments = vec!["--plan".to_string(), "--plan-children".to_string()];
    if let Some(reads) = reads {
        arguments.extend(["--reads".to_string(), text(reads)]);
    }
    arguments.extend([
        "generate".to_string(),
        text(&dir.join(format!("test{id}.scxml"))),
        "-l".to_string(),
        language.to_string(),
        "-o".to_string(),
        text(out),
        "--input-root".to_string(),
        text(&dir),
        "--no-format".to_string(),
    ]);
    arguments
}

#[test]
fn a_plan_lists_what_it_read_and_answers_as_it_does_without_the_list() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let door = tmp.path().join("door.scxml");
    fs::write(&door, DOOR).expect("write the document");
    let out = tmp.path().join("out");
    let reads = tmp.path().join("door.reads");
    let plan = |extra: &[String]| {
        let mut arguments = vec!["--plan".to_string()];
        arguments.extend_from_slice(extra);
        arguments.extend([
            "generate".to_string(),
            text(&door),
            "-o".to_string(),
            text(&out),
            "-l".to_string(),
            "cpp".to_string(),
            "--no-format".to_string(),
        ]);
        run(&arguments)
    };

    let plain = plan(&[]);
    let listed = plan(&["--reads".to_string(), text(&reads)]);

    assert!(plain.status.success(), "{}", stderr(&plain));
    assert!(listed.status.success(), "{}", stderr(&listed));
    assert_eq!(
        String::from_utf8_lossy(&listed.stdout),
        String::from_utf8_lossy(&plain.stdout),
        "asking a plan for what it read changed its answer"
    );
    assert!(!out.exists(), "a plan asked for its reads wrote the output");

    let (read, absent) = read_list(&reads);
    assert!(
        read.contains(&text(&door)),
        "the document the plan read is not in its list: {read:?}"
    );
    let program = fs::canonicalize(codegen()).expect("the generator is on disk");
    assert!(
        read.iter()
            .any(|line| fs::canonicalize(line).is_ok_and(|path| path == program)),
        "the generator, which decides the answer as much as the document, is not in the \
         list: {read:?}"
    );
    assert!(
        read.iter().any(|line| line.ends_with(".jinja2")),
        "the templates the plan rendered are not in its list: {read:?}"
    );
    assert!(
        read.iter()
            .chain(&absent)
            .all(|line| Path::new(line).is_absolute()),
        "a path of the list is relative, so a reader elsewhere reads another file: \
         {read:?} {absent:?}"
    );
    assert!(
        read.iter().all(|line| Path::new(line).exists()),
        "the list says a file was read that is not there, which a reader takes as changed \
         on every configure: {read:?}"
    );
    assert!(
        absent.iter().all(|line| !Path::new(line).exists()),
        "the list says a file is absent that is there: {absent:?}"
    );
    assert!(
        read.iter()
            .chain(&absent)
            .all(|line| !Path::new(line).starts_with(&out)),
        "the list names a file the plan writes: {read:?} {absent:?}"
    );
}

/// Every file a plan's answer says it writes, its child generations'
/// included: a line without a tab is the run's own file, a
/// `<child>\t<file>` line one of a child generation's, and a
/// `<child>\t\t<argument>...` line states arguments and names no file.
fn planned_files(plan: &Output) -> Vec<String> {
    String::from_utf8_lossy(&plan.stdout)
        .lines()
        .filter_map(|line| match line.split_once('\t') {
            None => Some(line.to_string()),
            Some((_, rest)) if rest.starts_with('\t') => None,
            Some((_, file)) => Some(file.to_string()),
        })
        .collect()
}

/// A plan's list of reads, split as a reader splits it: the files it read,
/// and the files it looked for and did not find (`?<path>`).
fn read_list(path: &Path) -> (Vec<String>, Vec<String>) {
    let mut read = Vec::new();
    let mut absent = Vec::new();
    for line in lines_of(path) {
        match line.strip_prefix('?') {
            Some(missing) => absent.push(missing.to_string()),
            None => read.push(line),
        }
    }
    (read, absent)
}

/// A generation is keyed on what it read (ADR 0097): an action cache asks the
/// run that writes for the list, and keys the result on the contents of the
/// files it names. The list has to be the one the plan of the same run
/// writes, since a plan is that run in memory; a generation that listed less
/// would be found current after a file it read had changed.
#[test]
fn a_generation_lists_what_its_plan_lists() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let door = tmp.path().join("door.scxml");
    fs::write(&door, DOOR).expect("write the document");
    let planned = tmp.path().join("planned.reads");
    let generated = tmp.path().join("generated.reads");
    let out = tmp.path().join("out");
    let arguments = |reads: &Path, plan: bool| {
        let mut arguments = Vec::new();
        if plan {
            arguments.push("--plan".to_string());
        }
        arguments.extend([
            "--reads".to_string(),
            text(reads),
            "generate".to_string(),
            text(&door),
            "-o".to_string(),
            text(&out),
            "-l".to_string(),
            "rust".to_string(),
        ]);
        arguments
    };

    let plan = run(&arguments(&planned, true));
    assert!(plan.status.success(), "{}", stderr(&plan));
    assert!(!out.exists(), "a plan wrote its output");
    let generation = run(&arguments(&generated, false));
    assert!(generation.status.success(), "{}", stderr(&generation));
    assert!(out.exists(), "the generation wrote no output");

    let listed = lines_of(&generated);
    assert!(
        listed.iter().any(|line| line == &text(&door)),
        "the generation's list does not name its document: {listed:?}"
    );
    assert!(
        listed.iter().any(|line| line.ends_with(".jinja2")),
        "the generation's list names no template, so a template edit would \
         leave its key where it was: {listed:?}"
    );
    assert_eq!(
        listed,
        lines_of(&planned),
        "a generation listed other reads than its plan"
    );
    let written: Vec<PathBuf> = fs::read_dir(&out)
        .expect("read the output")
        .map(|entry| entry.expect("an entry").path())
        .collect();
    for file in &written {
        assert!(
            !listed.iter().any(|line| line == &text(file)),
            "the generation listed its own output {} as a read",
            file.display()
        );
    }
}

/// A command that records nothing it reads is refused the list rather than
/// left to write none: a cache reading no list would key the run on nothing.
#[test]
fn a_command_that_records_no_reads_refuses_the_list() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let door = tmp.path().join("door.scxml");
    fs::write(&door, DOOR).expect("write the document");
    let reads = tmp.path().join("door.reads");

    let checked = run(&[
        "--reads".to_string(),
        text(&reads),
        "check".to_string(),
        text(&door),
        "-l".to_string(),
        "rust".to_string(),
    ]);

    assert!(
        !checked.status.success(),
        "--reads was accepted by a command that records nothing it reads"
    );
    assert!(
        stderr(&checked).contains("--reads applies only to"),
        "the refusal does not say where the flag applies: {}",
        stderr(&checked)
    );
    assert!(!reads.exists(), "a refused run wrote a list of reads");
}

/// A fixture whose generation writes the stub of a hybrid `<invoke>`, whose
/// own generation the plan holds: the list is the whole plan's, and the stub,
/// which the child's generation reads, is a file the plan writes.
#[test]
fn a_plan_of_children_lists_no_file_the_plan_writes() {
    for language in ["cpp", "c11"] {
        let tmp = tempfile::tempdir().expect("temp dir");
        let out = tmp.path().join("out");
        let reads = tmp.path().join("plan.reads");

        let plan = run(&conformance_plan("216", language, &out, Some(&reads)));

        assert!(plan.status.success(), "{language}: {}", stderr(&plan));
        assert!(
            String::from_utf8_lossy(&plan.stdout).contains('\t'),
            "{language}: no child generation was planned, so this proves nothing about one"
        );
        let (read, absent) = read_list(&reads);
        assert!(
            read.contains(&text(&fixture_dir("216").join("test216.scxml"))),
            "{language}: the fixture is not in the list: {read:?}"
        );
        // Neither as read nor as absent: a file the plan writes is on no
        // disk before the build, and after it every build rewrites it, so
        // either way its answer would be planned again on every configure.
        let written: Vec<&String> = read
            .iter()
            .chain(&absent)
            .filter(|line| Path::new(line).starts_with(&out))
            .collect();
        assert!(
            written.is_empty(),
            "{language}: the list names files the plan writes, so the answer would be \
             planned again on every configure: {written:?}"
        );
    }
}

/// The batch answers each request with the bytes the plan of it alone
/// prints, from a fixture that starts a child by `src` and synthesizes one,
/// one whose generation writes a hybrid stub, and a parent whose deploy
/// places its in-line child on a partition of its own; a refused document is
/// kept as the refused run's stderr.
#[test]
fn a_batch_answers_each_request_as_the_plan_of_it_alone_does() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let results = tmp.path().join("results");

    // Staged together, as the test tree stages the Mesh rule-3 override: the
    // deploy names the parent's document beside it.
    let mesh = tmp.path().join("mesh");
    fs::create_dir_all(&mesh).expect("mesh dir");
    for name in ["parent_synth_inline.scxml", "deploy_synth_inline.yaml"] {
        fs::copy(repo_root().join("tests/mesh").join(name), mesh.join(name))
            .expect("stage the Mesh fixture");
    }
    let deployed = vec![
        "--plan".to_string(),
        "--plan-children".to_string(),
        "generate".to_string(),
        text(&mesh.join("parent_synth_inline.scxml")),
        "-l".to_string(),
        "cpp".to_string(),
        "-o".to_string(),
        text(&mesh),
        "--no-format".to_string(),
        "--deploy".to_string(),
        text(&mesh.join("deploy_synth_inline.yaml")),
    ];

    let broken = tmp.path().join("broken.scxml");
    fs::write(&broken, "<scxml").expect("write the broken document");
    let refused = vec![
        "--plan".to_string(),
        "generate".to_string(),
        text(&broken),
        "-l".to_string(),
        "cpp".to_string(),
        "-o".to_string(),
        text(&tmp.path().join("refused")),
        "--no-format".to_string(),
    ];

    // The two other steps whose files a build compiles: a set compiled
    // together, and a conformance harness.
    let mut set = vec!["--plan".to_string(), "orchestrate".to_string()];
    for document in SET_DOCUMENTS {
        set.extend(["--forge".to_string(), text(&set_document(document))]);
    }
    set.extend([
        "-l".to_string(),
        "c11".to_string(),
        "-o".to_string(),
        text(&tmp.path().join("set")),
        "--no-format".to_string(),
    ]);
    let harness = vec![
        "--plan".to_string(),
        "generate-conformance".to_string(),
        "-l".to_string(),
        "c11".to_string(),
        "-m".to_string(),
        text(&forge_catalog()),
        "-o".to_string(),
        text(&tmp.path().join("harness")),
    ];

    let requests: Vec<(&str, Vec<String>)> = vec![
        (
            "w3c239",
            conformance_plan("239", "cpp", &tmp.path().join("w3c"), None),
        ),
        (
            "stub216",
            conformance_plan("216", "c11", &tmp.path().join("stub"), None),
        ),
        ("deployed", deployed),
        ("refused", refused),
        ("set", set),
        ("harness", harness),
    ];
    let listing: String = requests
        .iter()
        .map(|(name, arguments)| format!("{name}\t{}\n", arguments.join("\t")))
        .collect();
    let requests_file = tmp.path().join("requests.txt");
    fs::write(&requests_file, &listing).expect("write the requests");

    let batch = Command::new(codegen())
        .arg("plan-batch")
        .arg("--requests")
        .arg(&requests_file)
        .arg("--results")
        .arg(&results)
        .args(["--jobs", "2"])
        .output()
        .expect("run sce-codegen plan-batch");
    assert!(batch.status.success(), "{}", stderr(&batch));
    assert!(
        String::from_utf8_lossy(&batch.stdout).contains("6 request(s), 5 answered, 1 refused"),
        "the batch does not say what it answered: {}",
        String::from_utf8_lossy(&batch.stdout)
    );

    for (name, arguments) in &requests {
        let kept = |kind: &str| results.join(format!("{name}.{kind}"));
        let alone = run(arguments);
        assert_eq!(
            fs::read_to_string(kept("request")).expect("the request is kept"),
            format!("{name}\t{}\n", arguments.join("\t")),
            "{name}: the kept request is not the line asked"
        );
        if alone.status.success() {
            assert_eq!(
                String::from_utf8_lossy(&fs::read(kept("plan")).expect("the answer is kept")),
                String::from_utf8_lossy(&alone.stdout),
                "{name}: the batch's answer is not the plan's own"
            );
            assert!(kept("reads").is_file(), "{name}: no list of reads was kept");
            // No file the plan writes, its children's included, is in the
            // list, as read or as absent, and every file listed as read is
            // there: a configure would otherwise find the answer stale every
            // time, with nothing changed.
            let (read, absent) = read_list(&kept("reads"));
            let written = planned_files(&alone);
            let named: Vec<&String> = read
                .iter()
                .chain(&absent)
                .filter(|line| written.contains(*line))
                .collect();
            assert!(
                named.is_empty(),
                "{name}: the list names files the plan writes: {named:?}"
            );
            let missing: Vec<&String> = read
                .iter()
                .filter(|line| !Path::new(line).exists())
                .collect();
            assert!(
                missing.is_empty(),
                "{name}: the list says files were read that are not there: {missing:?}"
            );
            assert!(!kept("error").exists(), "{name}: an answer kept a refusal");
            // Dated before the plan read anything: no later than the request
            // the batch wrote before running it, so a file changed while the
            // plan ran is not older than the answer.
            assert!(
                modified(&kept("plan")) <= modified(&kept("request")),
                "{name}: the answer is dated after its plan began"
            );
            // And strictly after every file written before the batch began,
            // so a document staged just before it is older than the answer
            // and not tied with it.
            assert!(
                modified(&kept("plan")) > modified(&requests_file),
                "{name}: the answer is not dated past the files written before the batch, \
                 so a document staged just before it ties with the answer and is planned \
                 again on every configure"
            );
        } else {
            assert_eq!(
                String::from_utf8_lossy(&fs::read(kept("error")).expect("the refusal is kept")),
                stderr(&alone),
                "{name}: the kept refusal is not the refused plan's stderr"
            );
            assert!(
                !kept("plan").exists(),
                "{name}: a refused plan kept an answer"
            );
            assert!(
                !kept("reads").exists(),
                "{name}: a refused plan kept a list"
            );
        }
    }

    let answer = |name: &str| {
        fs::read_to_string(results.join(format!("{name}.plan"))).expect("the answer is kept")
    };
    assert!(
        answer("w3c239").contains("\t\t--as-child"),
        "the `src` child's generation was not planned, so the request proves nothing about one"
    );
    assert!(
        answer("deployed").contains("\t\t--deploy\t"),
        "the generation of the partition the deploy places the child on was not planned"
    );

    // What a set and a harness read is what their depfiles name: each
    // document of the set, and the catalog the harness is rendered from.
    let reads = |name: &str| read_list(&results.join(format!("{name}.reads"))).0;
    for document in SET_DOCUMENTS {
        let path = text(&set_document(document));
        assert!(
            reads("set").contains(&path),
            "the plan of the set does not list {path} among what it read"
        );
    }
    assert!(
        reads("harness").contains(&text(&forge_catalog())),
        "the plan of the harness does not list its catalog among what it read"
    );
}

/// A document set the C11 conformance project compiles with `orchestrate`,
/// each document importing only those before it.
const SET_DOCUMENTS: &[&str] = &[
    "broadcast_log_conformance_flagged_event",
    "broadcast_log_conformance_flagged",
];

/// The path of a document of [`SET_DOCUMENTS`].
fn set_document(name: &str) -> PathBuf {
    repo_root()
        .join("tests/forge/conformance/broadcast_log_context")
        .join(format!("{name}.scxml"))
}

/// The forge fixture catalog a conformance harness is rendered from.
fn forge_catalog() -> PathBuf {
    repo_root().join("tests/forge/conformance/fixtures.json")
}

/// The batch runs plans and nothing else, keeps each answer under a name
/// inside its directory, and refuses a list it cannot answer before running
/// any of it.
#[test]
fn a_batch_runs_only_plans() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let door = tmp.path().join("door.scxml");
    fs::write(&door, DOOR).expect("write the document");
    let out = tmp.path().join("out");
    let results = tmp.path().join("results");
    let plan = format!(
        "--plan\tgenerate\t{}\t-o\t{}\t-l\tcpp\t--no-format",
        text(&door),
        text(&out)
    );

    for (why, listing) in [
        (
            "a generation that writes",
            format!(
                "write\tgenerate\t{}\t-o\t{}\t-l\tcpp\n",
                text(&door),
                text(&out)
            ),
        ),
        (
            "a plan naming its own list of reads",
            format!(
                "listed\t--plan\t--reads\t{}\tgenerate\t{}\t-o\t{}\t-l\tcpp\n",
                text(&tmp.path().join("elsewhere.reads")),
                text(&door),
                text(&out)
            ),
        ),
        (
            "another command",
            format!("other\t--plan\tcheck\t{}\n", text(&door)),
        ),
        ("a name outside the results", format!("../escape\t{plan}\n")),
        (
            "one name for two requests",
            format!("twice\t{plan}\ntwice\t{plan}\t--limit\tprobe_bound=3\n"),
        ),
    ] {
        let requests = tmp.path().join("requests.txt");
        fs::write(&requests, &listing).expect("write the requests");
        let batch = Command::new(codegen())
            .arg("plan-batch")
            .arg("--requests")
            .arg(&requests)
            .arg("--results")
            .arg(&results)
            .output()
            .expect("run sce-codegen plan-batch");
        assert!(
            !batch.status.success(),
            "{why}: the batch accepted {listing:?}"
        );
        assert!(
            stderr(&batch).contains("plan-batch: line "),
            "{why}: the refusal does not name the line: {}",
            stderr(&batch)
        );
        assert!(!out.exists(), "{why}: a refused batch ran a generation");
        assert!(
            !results.exists(),
            "{why}: a refused batch kept something before it refused"
        );
    }
}
