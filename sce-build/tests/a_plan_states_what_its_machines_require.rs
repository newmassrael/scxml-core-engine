// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! `sce-codegen --plan --plan-requires generate` states what the machines of
//! each planned generation need linked, which is how a build learns what to
//! link a generated C11 machine with before it has run the generator.
//!
//! The statement is the one the backend made while rendering
//! (`a_c11_machine_includes_what_it_states_it_requires` holds it to what the
//! machine includes); what this holds is that it reaches the plan's answer in
//! the form `cmake/SCECodegenPlan.cmake` reads into `<PREFIX>_REQUIRES`:
//!
//! * one line per requirement, `\trequires\t<generation>\t<requirement>`,
//!   after every other line, with an empty `<generation>` for the run's own
//!   machines and, with `--plan-children`, the child document of a planned
//!   generation for that generation's;
//! * a line that begins with a tab names no file, so the run's own lines are
//!   the plan's without the flag, and a reader of the plain answer reads the
//!   same answer;
//! * a requirement is stated for what the machine does: the engine and the
//!   DOM binding for a script, the client for a BasicHTTP send, the clock
//!   for a delayed one, and nothing for a machine that does none of these;
//! * nothing is stated without the flag, the flag is refused without
//!   `--plan`, and a backend that states nothing (C++) states nothing;
//! * the run that writes the machines publishes the same set in its
//!   manifest's `runtime_requires`, `[]` included, and a backend that states
//!   nothing publishes no field.
//!
//! The documents are written here; the one tracked file the test reads is
//! the manifest's schema.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A machine whose data model is evaluated by the script engine.
const SCRIPTED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript"
       initial="s0" name="scripted">
    <datamodel><data id="Var1" expr="0"/></datamodel>
    <state id="s0"><transition cond="Var1 == 0" target="done"/></state>
    <final id="done"/>
</scxml>
"#;

/// A machine with no data model: it links nothing beside the header-only
/// runtime.
const PLAIN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s0" name="plain">
    <state id="s0">
        <onentry><raise event="go"/></onentry>
        <transition event="go" target="done"/>
    </state>
    <final id="done"/>
</scxml>
"#;

/// A machine that sends over BasicHTTP.
const POSTER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript"
       initial="s0" name="poster">
    <state id="s0">
        <onentry>
            <send type="http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"
                  target="http://localhost:8080/test" event="posted"/>
        </onentry>
        <transition event="posted" target="done"/>
    </state>
    <final id="done"/>
</scxml>
"#;

/// A machine with no data model that schedules an event: it reads the time.
const TIMED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s0" name="timed">
    <state id="s0">
        <onentry><send event="later" delay="10ms"/></onentry>
        <transition event="later" target="done"/>
    </state>
    <final id="done"/>
</scxml>
"#;

/// A machine with no data model that starts the scripted one by `src`.
const PARENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="running" name="parent">
    <state id="running">
        <invoke type="http://www.w3.org/TR/scxml/" src="file:scripted.scxml"/>
    </state>
</scxml>
"#;

/// The documents, written where their author keeps them; the directory
/// (kept alive) and the directory's path.
fn authored() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("authored");
    std::fs::create_dir_all(&root).expect("make the documents' directory");
    for (name, text) in [
        ("scripted.scxml", SCRIPTED),
        ("plain.scxml", PLAIN),
        ("poster.scxml", POSTER),
        ("parent.scxml", PARENT),
        ("timed.scxml", TIMED),
    ] {
        std::fs::write(root.join(name), text).expect("write a document");
    }
    (dir, root)
}

/// `sce-codegen [flags] generate <root>/<document> -l <language> --no-format
/// -o <out> --input-root <root>`, as the CMake routes ask a plan.
fn run(flags: &[&str], root: &Path, document: &str, language: &str, out: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .args(flags)
        .arg("generate")
        .arg(root.join(document))
        .args(["-l", language, "--no-format", "-o"])
        .arg(out)
        .arg("--input-root")
        .arg(root)
        .output()
        .expect("spawn sce-codegen")
}

fn succeeded(output: &Output, what: &str) -> Vec<String> {
    assert!(
        output.status.success(),
        "{what} failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_string)
        .collect()
}

/// The statements of a plan's answer, as `(generation, requirement)`, and
/// every other line; a line that begins with a tab and is no statement is a
/// failure.
fn split(lines: &[String]) -> (Vec<(String, String)>, Vec<String>) {
    let mut stated = Vec::new();
    let mut rest = Vec::new();
    for line in lines {
        let Some(statement) = line.strip_prefix('\t') else {
            rest.push(line.clone());
            continue;
        };
        let fields: Vec<&str> = statement.split('\t').collect();
        assert!(
            fields.len() == 3 && fields[0] == "requires" && !fields[2].is_empty(),
            "a line that begins with a tab is not `\\trequires\\t<generation>\\t<requirement>`: \
             {line:?}"
        );
        stated.push((fields[1].to_string(), fields[2].to_string()));
    }
    (stated, rest)
}

/// The requirements stated for `generation` (empty: the run's own).
fn stated_for<'a>(stated: &'a [(String, String)], generation: &str) -> Vec<&'a str> {
    stated
        .iter()
        .filter(|(stated_generation, _)| stated_generation == generation)
        .map(|(_, requirement)| requirement.as_str())
        .collect()
}

#[test]
fn a_scripted_c11_machine_states_the_engine_and_the_dom_binding() {
    let (dir, root) = authored();
    let out = dir.path().join("out");
    let lines = succeeded(
        &run(
            &["--plan", "--plan-requires"],
            &root,
            "scripted.scxml",
            "c11",
            &out,
        ),
        "the plan of the scripted machine",
    );
    let (stated, rest) = split(&lines);
    let own = stated_for(&stated, "");
    assert!(
        own.contains(&"lua") && own.contains(&"dom"),
        "the scripted machine states {own:?}, not the Lua engine and the DOM binding it embeds"
    );
    assert!(
        !own.contains(&"http"),
        "the scripted machine states an HTTP client it never sends with: {own:?}"
    );
    assert!(
        !own.contains(&"clock"),
        "the scripted machine states a clock it schedules nothing with: {own:?}"
    );

    // The statements come after every file, and the files are the plain
    // answer's.
    let first_statement = lines
        .iter()
        .position(|line| line.starts_with('\t'))
        .expect("a statement");
    assert!(
        lines[first_statement..]
            .iter()
            .all(|line| line.starts_with('\t')),
        "a file line after a statement: {lines:?}"
    );
    let plain = succeeded(
        &run(&["--plan"], &root, "scripted.scxml", "c11", &out),
        "the plain plan of the scripted machine",
    );
    assert_eq!(
        rest, plain,
        "the run's own lines moved under --plan-requires"
    );
    assert!(
        !plain.iter().any(|line| line.contains('\t')),
        "a plan asked without --plan-requires states something: {plain:?}"
    );
}

#[test]
fn a_machine_with_no_data_model_states_nothing() {
    let (dir, root) = authored();
    let out = dir.path().join("out");
    let lines = succeeded(
        &run(
            &["--plan", "--plan-requires"],
            &root,
            "plain.scxml",
            "c11",
            &out,
        ),
        "the plan of the plain machine",
    );
    let (stated, rest) = split(&lines);
    assert!(
        stated.is_empty(),
        "a machine with no data model states {stated:?}"
    );
    assert!(!rest.is_empty(), "the plan names no file");
}

#[test]
fn a_machine_that_schedules_states_the_clock_and_nothing_else() {
    let (dir, root) = authored();
    let out = dir.path().join("out");
    let lines = succeeded(
        &run(
            &["--plan", "--plan-requires"],
            &root,
            "timed.scxml",
            "c11",
            &out,
        ),
        "the plan of the timed machine",
    );
    let (stated, _) = split(&lines);
    assert_eq!(
        stated_for(&stated, ""),
        vec!["clock"],
        "a machine with no data model that schedules an event states {stated:?}"
    );
}

#[test]
fn a_machine_that_sends_over_basic_http_states_the_client() {
    let (dir, root) = authored();
    let out = dir.path().join("out");
    let lines = succeeded(
        &run(
            &["--plan", "--plan-requires"],
            &root,
            "poster.scxml",
            "c11",
            &out,
        ),
        "the plan of the BasicHTTP machine",
    );
    let (stated, _) = split(&lines);
    assert!(
        stated_for(&stated, "").contains(&"http"),
        "a machine that posts over BasicHTTP states {stated:?}"
    );
}

/// The child's generation is planned beside the run's, and its statement is
/// its own, named by the document the generation reads.
#[test]
fn a_child_generation_states_what_its_machine_requires() {
    let (dir, root) = authored();
    let out = dir.path().join("out");
    let lines = succeeded(
        &run(
            &["--plan", "--plan-children", "--plan-requires"],
            &root,
            "parent.scxml",
            "c11",
            &out,
        ),
        "the plan of the parent and its child",
    );
    let (stated, rest) = split(&lines);
    // The child's generation: the line stating its arguments,
    // `<child document>\t\t<argument>...`.
    let child = rest
        .iter()
        .find_map(|line| {
            let (document, after) = line.split_once('\t')?;
            (after.starts_with('\t') && document.ends_with("scripted.scxml"))
                .then(|| document.to_string())
        })
        .unwrap_or_else(|| panic!("the child's generation was not planned: {lines:?}"));
    let theirs = stated_for(&stated, &child);
    assert!(
        theirs.contains(&"lua") && theirs.contains(&"dom"),
        "the scripted child's generation states {theirs:?} (all statements: {stated:?})"
    );
    // Every statement names the run or a generation the plan holds.
    for (generation, requirement) in &stated {
        assert!(
            generation.is_empty() || *generation == child,
            "`{requirement}` is stated for {generation:?}, which the plan holds no generation of"
        );
    }
}

#[test]
fn a_backend_that_states_nothing_states_nothing() {
    let (dir, root) = authored();
    let out = dir.path().join("out");
    let lines = succeeded(
        &run(
            &["--plan", "--plan-requires"],
            &root,
            "scripted.scxml",
            "cpp",
            &out,
        ),
        "the C++ plan of the scripted machine",
    );
    let (stated, _) = split(&lines);
    assert!(
        stated.is_empty(),
        "the C++ backend links its engine for every machine and states nothing, yet the plan \
         states {stated:?}"
    );
}

/// The manifest of the run that writes `document`, validated against the
/// checked-in schema before it is returned.
fn generate_manifest(document: &str, language: &str) -> serde_json::Value {
    let (dir, root) = authored();
    let out = dir.path().join("out");
    let lines = succeeded(
        &run(&[], &root, document, language, &out),
        "the run that writes the machine",
    );
    let line = lines.last().expect("the run writes its manifest");
    let manifest: serde_json::Value =
        serde_json::from_str(line).expect("the manifest is one JSON line");
    let schema: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../schemas/sce-manifest.v1.schema.json"),
        )
        .expect("read the manifest schema"),
    )
    .expect("the manifest schema is JSON");
    let validator = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(&schema)
        .expect("the manifest schema compiles");
    let violations: Vec<String> = match validator.validate(&manifest) {
        Ok(()) => Vec::new(),
        Err(errors) => errors.map(|e| e.to_string()).collect(),
    };
    assert!(violations.is_empty(), "{violations:?}\n{manifest}");
    manifest
}

/// The run that writes the machine publishes in its manifest the set the plan
/// states, so a build that generates first and links after reads the same
/// answer as one that asks first; the empty set is published as `[]`, and a
/// backend that states nothing publishes no field.
#[test]
fn the_manifest_publishes_what_the_plan_states() {
    for (document, expected) in [("timed.scxml", vec!["clock"]), ("plain.scxml", Vec::new())] {
        let manifest = generate_manifest(document, "c11");
        assert_eq!(
            manifest["runtime_requires"],
            serde_json::json!(expected),
            "{document}: {manifest}"
        );
    }

    let scripted = generate_manifest("scripted.scxml", "c11");
    let (dir, root) = authored();
    let planned = succeeded(
        &run(
            &["--plan", "--plan-requires"],
            &root,
            "scripted.scxml",
            "c11",
            &dir.path().join("out"),
        ),
        "the plan of the scripted machine",
    );
    let (stated, _) = split(&planned);
    assert_eq!(
        scripted["runtime_requires"],
        serde_json::json!(stated_for(&stated, "")),
        "the manifest and the plan disagree about the scripted machine"
    );

    let cpp = generate_manifest("scripted.scxml", "cpp");
    assert!(
        cpp.get("runtime_requires").is_none(),
        "the C++ backend states nothing, yet its manifest says {cpp}"
    );
}

#[test]
fn plan_requires_without_a_plan_is_refused() {
    let (dir, root) = authored();
    let out = dir.path().join("out");
    let refused = run(&["--plan-requires"], &root, "scripted.scxml", "c11", &out);
    assert!(
        !refused.status.success(),
        "--plan-requires was accepted on a run that writes"
    );
    assert!(!out.exists(), "the refused run wrote {}", out.display());
}
