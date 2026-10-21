// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `generate --with-imports`: one run generates a design and every document it
// imports, the standard library's included, each once, below one output root
// at the directory its package requires.
//
// The design is shaped like a project outside this repository: a directory of
// its own holding a `sce-static` statechart that imports a local event-schema,
// a local enum, a local algorithm that itself imports another local algorithm,
// a standard algorithm (which imports two more of the library's) and a
// standard enum. Before this mode existed such a project re-implemented the
// import walk SCE's own regeneration scripts do in shell, with a table of
// package directories beside it.
//
// Each property is a test of its own:
//   - Kotlin: every forge unit lands in `com/sce/generated/<snake>/`, the
//     package the machine's import line names, whatever the machine's prefix,
//     and the machine in its `--kotlin-package-prefix` package;
//   - Rust: every unit lands beside the others, as `use super::` names them;
//   - no file is written twice, and the run's manifest names each once;
//   - a second run with `--assert-unchanged` passes, and an edit to one
//     imported document makes it fail;
//   - `--plan` names exactly the files the run writes, and writes nothing;
//   - a broken import is refused where it is written, the chain of imports
//     that reached it carried as `reached-from` sites, before anything is
//     written;
//   - a document the run reached that its own generation refuses is refused
//     with the same chain;
//   - every backend that lowers `sce-static` generates the design;
//   - a child an `<invoke src>` names by its file is generated into its
//     parent's package, and not as a machine of its own;
//   - `orchestrate --document` takes a standard document by its library name.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sce_build::generator::Language;

fn codegen() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

const MACHINE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" initial="idle" datamodel="sce-static" name="closure_machine">
  <sce:import kind="event-schema" src="schema_pick.scxml" as="Pick"/>
  <sce:import kind="enum" src="enum_level.scxml" as="Level"/>
  <sce:import kind="algorithm" src="algorithm_outer.scxml" as="OuterPick"/>
  <sce:import kind="algorithm" src="sce:std/time/days_from_civil.scxml" as="DaysFromCivil"/>
  <sce:import kind="enum" src="sce:std/mesh/rpc_status.scxml" as="Status"/>
  <datamodel>
    <data id="shownLevel" sce:type="enum:Level" expr="Level.low" sce:direction="out"/>
    <data id="outcome" sce:type="enum:Status" expr="Status.ok" sce:direction="out"/>
    <data id="picked" sce:type="uint8" expr="0" sce:direction="out"/>
    <data id="civilYear" sce:type="int32" expr="2026" sce:direction="out"/>
    <data id="civilMonth" sce:type="uint8" expr="9" sce:direction="out"/>
    <data id="civilDay" sce:type="uint8" expr="23" sce:direction="out"/>
    <data id="epochDays" sce:type="int64" expr="0" sce:direction="out"/>
    <data id="refusals" sce:type="uint32" expr="0" sce:direction="out"/>
  </datamodel>
  <state id="idle">
    <transition event="pick" type="internal">
      <assign location="picked" expr="OuterPick(_event.data.count)"/>
      <assign location="shownLevel" expr="_event.data.level"/>
    </transition>
    <transition event="civil" type="internal">
      <assign location="epochDays" expr="DaysFromCivil(civilYear, civilMonth, civilDay)"/>
      <assign location="outcome" expr="Status.notFound"/>
    </transition>
    <transition event="error.execution" type="internal">
      <assign location="refusals" expr="refusals + 1"/>
    </transition>
  </state>
</scxml>
"#;

/// The line of `MACHINE` its import of the outer algorithm is written on.
const MACHINE_IMPORTS_OUTER_AT: u64 = 5;

const SCHEMA_PICK: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="event-schema" name="schema_pick" sce:event-name="pick">
  <sce:import src="enum_level.scxml" kind="enum" as="Level"/>
  <datamodel>
    <data id="count" sce:type="uint8" sce:direction="in"/>
    <data id="level" sce:type="enum:Level" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

const ENUM_LEVEL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="enum" name="enum_level" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="low" value="1"/>
      <sce:variant name="high" value="2"/>
    </data>
  </datamodel>
</scxml>
"#;

/// Named apart from its file, so a unit placed by its file's stem and not by
/// the name its import line uses lands in the wrong package.
const ALGORITHM_OUTER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="outer_pick" version="1.0">
  <sce:import kind="algorithm" src="algorithm_inner.scxml" as="even"/>
  <sce:signature>
    <sce:param name="n" type="uint8"/>
    <sce:return type="uint8"/>
  </sce:signature>
  <sce:body>
    <sce:if cond="even(n)">
      <sce:return expr="1"/>
    </sce:if>
    <sce:return expr="0"/>
  </sce:body>
</scxml>
"#;

/// The line of `ALGORITHM_OUTER` its import of the inner algorithm is
/// written on.
const OUTER_IMPORTS_INNER_AT: u64 = 3;

const ALGORITHM_INNER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="inner_even" version="1.0">
  <sce:signature>
    <sce:param name="n" type="uint8"/>
    <sce:return type="bool"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="n % 2 === 0"/>
  </sce:body>
</scxml>
"#;

/// The forge units the design reaches, by the name each is generated under:
/// the five of its own directory and the library's three.
const FORGE_UNITS: &[&str] = &[
    "schema_pick",
    "enum_level",
    "outer_pick",
    "inner_even",
    "days_from_civil",
    "days_in_month",
    "is_leap_year",
    "rpc_status",
];

/// A project directory holding the design under `src/`, generated into
/// `out/` by the caller.
fn project() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("temp dir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).expect("create the source directory");
    for (file, text) in [
        ("closure_machine.scxml", MACHINE),
        ("schema_pick.scxml", SCHEMA_PICK),
        ("enum_level.scxml", ENUM_LEVEL),
        ("algorithm_outer.scxml", ALGORITHM_OUTER),
        ("algorithm_inner.scxml", ALGORITHM_INNER),
    ] {
        fs::write(src.join(file), text).expect("write a document");
    }
    tmp
}

fn machine(tmp: &Path) -> PathBuf {
    tmp.join("src").join("closure_machine.scxml")
}

/// `sce-codegen --error-format json generate <machine> --with-imports -l
/// <language> -o <out>`, plus `extra`.
fn generate(tmp: &Path, out: &Path, language: &str, extra: &[&str]) -> Output {
    Command::new(codegen())
        .args(["--error-format", "json", "generate"])
        .arg(machine(tmp))
        .args(["--with-imports", "-l", language, "--output-dir"])
        .arg(out)
        .args(extra)
        .output()
        .expect("run sce-codegen")
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn succeeded(out: &Output, what: &str) {
    assert!(
        out.status.success(),
        "{what} failed (exit {:?}): {}",
        out.status.code(),
        stderr(out)
    );
}

/// The one NDJSON record a refused run leaves on stderr.
fn record(out: &Output) -> serde_json::Value {
    let text = stderr(out);
    let line = text
        .lines()
        .find(|l| l.starts_with('{'))
        .unwrap_or_else(|| panic!("no NDJSON record on stderr:\n{text}"));
    serde_json::from_str(line).unwrap_or_else(|e| panic!("record does not parse ({e}): {line}"))
}

/// The manifest's artifacts, each relative to `root` with `/` separators, in
/// the order the manifest lists them.
fn artifacts(out: &Output, root: &Path) -> Vec<String> {
    let stdout = String::from_utf8_lossy(&out.stdout);
    let line = stdout
        .lines()
        .find(|l| l.starts_with('{'))
        .unwrap_or_else(|| panic!("no manifest on stdout:\n{stdout}"));
    let manifest: serde_json::Value =
        serde_json::from_str(line).unwrap_or_else(|e| panic!("manifest does not parse ({e})"));
    assert_eq!(manifest["kind"], "generate", "{manifest}");
    assert!(
        manifest.get("document_kind").is_none(),
        "a run over a set of documents names no one kind: {manifest}"
    );
    manifest["artifacts"]
        .as_array()
        .expect("the manifest lists its artifacts")
        .iter()
        .map(|entry| relative(Path::new(entry["path"].as_str().expect("a path")), root))
        .collect()
}

fn relative(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or_else(|_| panic!("{} is not under {}", path.display(), root.display()))
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Every file under `dir`, relative to it, with its bytes.
fn tree(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).expect("read an output directory") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(root, &path, files);
            } else {
                files.insert(
                    relative(&path, root),
                    fs::read(&path).expect("read an output"),
                );
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(dir, dir, &mut files);
    files
}

/// `artifacts` holds no path twice.
fn each_once(artifacts: &[String]) {
    let distinct: BTreeSet<&String> = artifacts.iter().collect();
    assert_eq!(
        distinct.len(),
        artifacts.len(),
        "a file was written twice: {artifacts:?}"
    );
}

#[test]
fn kotlin_places_each_forge_unit_in_the_package_its_import_line_names() {
    let tmp = project();
    let out = tmp.path().join("out");
    let run = generate(
        tmp.path(),
        &out,
        "kotlin",
        &["--kotlin-package-prefix", "com.acme.app"],
    );
    succeeded(&run, "the Kotlin run");
    let written = artifacts(&run, &out);
    each_once(&written);

    let forge: BTreeSet<String> = written
        .iter()
        .filter(|path| path.starts_with("com/sce/generated/"))
        .cloned()
        .collect();
    let expected: BTreeSet<String> = FORGE_UNITS
        .iter()
        .map(|name| {
            let pascal = sce_build::filters::to_pascal_case(name.to_string());
            format!("com/sce/generated/{name}/{pascal}.kt")
        })
        .collect();
    assert_eq!(
        forge, expected,
        "each forge unit in the package `import com.sce.generated.<snake>.*` names"
    );
    let machine_dir = "com/acme/app/closure_machine/";
    assert!(
        written.contains(&format!("{machine_dir}closure_machineSm.kt")),
        "the machine in its prefixed package: {written:?}"
    );
    let elsewhere: Vec<&String> = written
        .iter()
        .filter(|path| !forge.contains(*path) && !path.starts_with(machine_dir))
        .collect();
    assert!(
        elsewhere.is_empty(),
        "every file is a forge unit's or the machine's: {elsewhere:?}"
    );
    for path in &written {
        assert!(out.join(path).is_file(), "{path} was not written");
    }
}

#[test]
fn rust_places_every_unit_beside_the_others() {
    let tmp = project();
    let out = tmp.path().join("out");
    let run = generate(tmp.path(), &out, "rust", &[]);
    succeeded(&run, "the Rust run");
    let written = artifacts(&run, &out);
    each_once(&written);

    let forge: BTreeSet<String> = written
        .iter()
        .filter(|path| !path.starts_with("closure_machine"))
        .cloned()
        .collect();
    let expected: BTreeSet<String> = FORGE_UNITS
        .iter()
        .map(|name| format!("{name}.rs"))
        .collect();
    assert_eq!(
        forge, expected,
        "each forge unit beside the machine, as `use super::<snake>` names it"
    );
    assert!(
        written.contains(&"closure_machine_sm.rs".to_string()),
        "{written:?}"
    );
    for path in &written {
        assert!(out.join(path).is_file(), "{path} was not written");
    }
}

#[test]
fn a_second_run_asserts_the_tree_unchanged_until_an_import_moves() {
    let tmp = project();
    let out = tmp.path().join("out");
    let extra = ["--kotlin-package-prefix", "com.acme.app"];
    succeeded(
        &generate(tmp.path(), &out, "kotlin", &extra),
        "the writing run",
    );
    let before = tree(&out);

    let mut asserting: Vec<&str> = extra.to_vec();
    asserting.push("--assert-unchanged");
    let check = generate(tmp.path(), &out, "kotlin", &asserting);
    succeeded(&check, "the run over the tree it just wrote");
    assert_eq!(tree(&out), before, "the asserting run wrote");

    // A change to a document two imports down, which the machine never names.
    let inner = tmp.path().join("src").join("algorithm_inner.scxml");
    fs::write(&inner, ALGORITHM_INNER.replace("n % 2", "n % 3")).expect("edit the import");
    let moved = generate(tmp.path(), &out, "kotlin", &asserting);
    assert!(
        !moved.status.success(),
        "an edited import left the tree reported unchanged"
    );
    assert_eq!(
        record(&moved)["code"],
        "forge/generated-output-changed",
        "{}",
        stderr(&moved)
    );
    assert_eq!(tree(&out), before, "the failing run wrote");
}

#[test]
fn the_plan_names_exactly_the_files_the_run_writes_and_writes_nothing() {
    let tmp = project();
    let planned_root = tmp.path().join("planned");
    let plan = generate(tmp.path(), &planned_root, "rust", &["--plan"]);
    succeeded(&plan, "the planning run");
    assert!(!planned_root.exists(), "the planning run wrote");
    let planned: BTreeSet<String> = String::from_utf8_lossy(&plan.stdout)
        .lines()
        .map(|line| relative(Path::new(line), &planned_root))
        .collect();

    let out = tmp.path().join("out");
    succeeded(&generate(tmp.path(), &out, "rust", &[]), "the writing run");
    let written: BTreeSet<String> = tree(&out).into_keys().collect();
    assert_eq!(planned, written);
}

#[test]
fn a_broken_import_is_refused_with_the_chain_that_reached_it() {
    let tmp = project();
    let outer = tmp.path().join("src").join("algorithm_outer.scxml");
    fs::write(
        &outer,
        ALGORITHM_OUTER.replace("algorithm_inner.scxml", "algorithm_absent.scxml"),
    )
    .expect("break the import");
    let out = tmp.path().join("out");

    let run = generate(tmp.path(), &out, "rust", &[]);
    assert_eq!(
        run.status.code(),
        Some(5),
        "an import of a file that is not there is an import/* refusal: {}",
        stderr(&run)
    );
    let refusal = record(&run);
    assert_eq!(refusal["code"], "import/file-not-found", "{refusal}");
    let file = refusal["location"]["file"].as_str().expect("a location");
    assert!(
        file.ends_with("algorithm_outer.scxml"),
        "refused in the document that holds the broken import: {refusal}"
    );
    assert_eq!(
        refusal["location"]["line"], OUTER_IMPORTS_INNER_AT,
        "at the <sce:import> itself: {refusal}"
    );
    let related = refusal["related"]
        .as_array()
        .expect("the chain rides along");
    assert_eq!(related.len(), 1, "{refusal}");
    assert_eq!(related[0]["role"], "reached-from");
    assert!(
        related[0]["location"]["file"]
            .as_str()
            .is_some_and(|f| f.ends_with("closure_machine.scxml")),
        "reached from the machine named on the command line: {refusal}"
    );
    assert_eq!(related[0]["location"]["line"], MACHINE_IMPORTS_OUTER_AT);
    assert!(
        !out.exists(),
        "a run whose walk was refused wrote before refusing"
    );
}

#[test]
fn a_reached_document_its_own_generation_refuses_carries_the_chain() {
    let tmp = project();
    // `n + 1` can reach 256, which a uint8 cannot hold, and the algorithm
    // does not declare `may-fail`: refused when the algorithm itself is
    // generated, which only this run does.
    let inner = tmp.path().join("src").join("algorithm_inner.scxml");
    fs::write(
        &inner,
        ALGORITHM_INNER.replace(
            "    <sce:return expr=\"n % 2 === 0\"/>",
            "    <sce:var name=\"w\" type=\"uint8\" init=\"n + 1\"/>\n    \
             <sce:return expr=\"w % 2 === 0\"/>",
        ),
    )
    .expect("break the inner algorithm");
    let out = tmp.path().join("out");

    let run = generate(tmp.path(), &out, "rust", &[]);
    assert!(
        !run.status.success(),
        "the refused algorithm was generated: {}",
        stderr(&run)
    );
    let refusal = record(&run);
    assert!(
        refusal["location"]["file"]
            .as_str()
            .is_some_and(|f| f.ends_with("algorithm_inner.scxml")),
        "{refusal}"
    );
    let chain: Vec<(String, u64)> = refusal["related"]
        .as_array()
        .expect("the chain rides along")
        .iter()
        .filter(|site| site["role"] == "reached-from")
        .map(|site| {
            let file = site["location"]["file"].as_str().expect("a file");
            let name = Path::new(file)
                .file_name()
                .expect("a file name")
                .to_string_lossy()
                .into_owned();
            (name, site["location"]["line"].as_u64().expect("a line"))
        })
        .collect();
    assert_eq!(
        chain,
        [
            (
                "closure_machine.scxml".to_string(),
                MACHINE_IMPORTS_OUTER_AT
            ),
            ("algorithm_outer.scxml".to_string(), OUTER_IMPORTS_INNER_AT),
        ],
        "the machine's import of the outer algorithm, then the outer one's of the inner: \
         {refusal}"
    );
}

#[test]
fn every_backend_that_lowers_the_static_model_generates_the_design() {
    for language in Language::ALL
        .iter()
        .copied()
        .filter(|language| sce_build::generator::lowers_static_datamodel(*language))
    {
        let tmp = project();
        let out = tmp.path().join("out");
        let name = language.canonical_name();
        // Each flag is ignored by the backends it does not name.
        let extra = [
            "--go-module-prefix",
            "example.com/acme/generated",
            "--kotlin-package-prefix",
            "com.acme.app",
            "--no-format",
        ];
        let run = generate(tmp.path(), &out, name, &extra);
        succeeded(&run, &format!("the {name} run"));
        let written = artifacts(&run, &out);
        each_once(&written);
        assert!(
            written.len() > FORGE_UNITS.len(),
            "{name}: fewer files than units: {written:?}"
        );
        let mut asserting = extra.to_vec();
        asserting.push("--assert-unchanged");
        succeeded(
            &generate(tmp.path(), &out, name, &asserting),
            &format!("the {name} run over the tree it just wrote"),
        );
    }
}

/// `sce-codegen --error-format json generate <documents...> --with-imports -l
/// <language> -o <out>`, plus `extra`.
fn generate_documents(documents: &[PathBuf], out: &Path, language: &str, extra: &[&str]) -> Output {
    Command::new(codegen())
        .args(["--error-format", "json", "generate"])
        .args(documents)
        .args(["--with-imports", "-l", language, "--output-dir"])
        .arg(out)
        .args(extra)
        .output()
        .expect("run sce-codegen")
}

/// A statechart named `name` whose one state holds `body`.
fn plain_statechart(name: &str, body: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<scxml \
         xmlns=\"http://www.w3.org/2005/07/scxml\" version=\"1.0\" initial=\"running\" \
         name=\"{name}\">\n  <state id=\"running\">\n{body}    <transition event=\"stop\" \
         target=\"done\"/>\n  </state>\n  <final id=\"done\"/>\n</scxml>\n"
    )
}

#[test]
fn a_child_an_invoke_names_by_its_file_is_generated_into_its_parents_package() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).expect("create the source directory");
    fs::write(src.join("worker.scxml"), plain_statechart("worker", "")).expect("write");
    let parent = src.join("parent_machine.scxml");
    fs::write(
        &parent,
        plain_statechart(
            "parent_machine",
            "    <invoke type=\"scxml\" src=\"worker.scxml\"/>\n",
        ),
    )
    .expect("write");
    let out = tmp.path().join("out");

    let run = generate_documents(
        std::slice::from_ref(&parent),
        &out,
        "kotlin",
        &["--kotlin-package-prefix", "com.acme.app"],
    );
    succeeded(&run, "the run over a parent and its file child");
    let written = artifacts(&run, &out);
    each_once(&written);
    assert!(
        written.contains(&"com/acme/app/parent_machine/workerSm.kt".to_string()),
        "the child in its parent's package: {written:?}"
    );
    assert!(
        !written
            .iter()
            .any(|path| path.starts_with("com/acme/app/worker/")),
        "the child was also generated as a machine of its own: {written:?}"
    );
    let child = fs::read_to_string(out.join("com/acme/app/parent_machine/workerSm.kt"))
        .expect("read the child");
    assert!(
        child.contains("package com.acme.app.parent_machine"),
        "the child declares its parent's package"
    );
}

#[test]
fn several_documents_without_the_flag_are_refused() {
    let tmp = project();
    let src = tmp.path().join("src");
    let run = Command::new(codegen())
        .args(["--error-format", "json", "generate"])
        .arg(src.join("closure_machine.scxml"))
        .arg(src.join("algorithm_outer.scxml"))
        .args(["-l", "rust", "--output-dir"])
        .arg(tmp.path().join("out"))
        .output()
        .expect("run sce-codegen");
    assert_eq!(run.status.code(), Some(20), "{}", stderr(&run));
    assert_eq!(record(&run)["code"], "cli/usage");
}

#[test]
fn orchestrate_takes_a_standard_document_by_its_library_name() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let out = tmp.path().join("out");
    let run = Command::new(codegen())
        .args([
            "--error-format",
            "json",
            "orchestrate",
            "--document",
            "sce:std/time/is_leap_year.scxml",
            "-l",
            "rust",
            "--output-dir",
        ])
        .arg(&out)
        .output()
        .expect("run sce-codegen");
    succeeded(&run, "orchestrate over a standard document");
    assert!(
        out.join("is_leap_year.rs").is_file(),
        "the standard algorithm was not generated: {:?}",
        tree(&out).keys().collect::<Vec<_>>()
    );
}
