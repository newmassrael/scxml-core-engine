// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A document that declares `sce:transition-table="emit"` is generated with its
// transitions as constant data in the source of every backend, and
// `sce-codegen transition-table` lists the same transitions as an export. They
// are one fact with two surfaces, and this file holds each backend's table to
// the export of the document it was generated from: the same transitions, in
// the same order, with the same owning state, first target, event, whole target
// set, every event descriptor, index and annotation, and the same `source_hash`
// the file's own header carries.
//
// The first comparison reads the generated source back rather than compiling it, so it
// runs on a host with no toolchain. Where a toolchain is found, the same output is
// also built in the backend's own toolchain and the table it holds is compared with
// the export at the level the language allows: C++ by static assertions over every
// field, Python by importing the module, and Rust, Go, Kotlin and C by building a
// program against the real runtime and running it. A table that reads back right
// and does not compile, or compiles to other values, is still wrong.
//
// Absent the declaration, nothing changes: the generated files are held
// byte-identical by the committed-tree drift gates, and this file checks that none
// of them names the table.

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use regex::Regex;
use sce_build::filters;
use sce_build::forge::codegen_matrix::emitting;
use sce_build::forge::model::ForgeKind;
use sce_build::generator::Language;
use sce_build::toolchain;

/// Every backend that emits a machine, by the identifier `generate -l` takes.
fn backends() -> Vec<&'static str> {
    emitting(ForgeKind::Statechart)
        .map(|language| language.canonical_name())
        .collect()
}

/// The machine's name: the file stem, which every backend derives its names from.
const STEM: &str = "ttemit";

/// One document that reaches every shape a table entry can take: a literal event, a
/// literal event that is also the prefix of another (a branch of Kotlin's event
/// tree), several descriptors, the wildcard, a prefix family, no event, a first
/// target that is a `<history>`, no target, several targets, an annotation of
/// several tokens carrying characters a string literal has to escape, and a state
/// whose id is not lower case.
const DOC: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" sce:transition-table="emit"
       version="1.0" name="ttemit" initial="idle" datamodel="null">
  <state id="idle">
    <transition event="go" target="Busy_2" sce:req="12 REQ-&quot;Q&quot; a\b $x {y} 50%"/>
    <transition event="door" target="Busy_2"/>
    <transition event="door.open" target="Busy_2" sce:req="3"/>
    <transition event="a b" target="Busy_2"/>
    <transition event="*" target="Busy_2"/>
    <transition event="err.*" target="Busy_2"/>
    <transition cond="In('Busy_2')" target="Busy_2"/>
    <transition event="back" target="h"/>
    <transition event="stop"/>
    <transition event="fork" target="r1 r2"/>
  </state>
  <state id="Busy_2" initial="inner">
    <history id="h"><transition target="inner"/></history>
    <state id="inner"/>
    <transition event="done" target="end" sce:req="REQ-9"/>
  </state>
  <parallel id="par">
    <state id="r1"/>
    <state id="r2"/>
  </parallel>
  <final id="end"/>
</scxml>
"#;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent dir")
        .to_path_buf()
}

/// One transition as the generated table holds it, with the ids resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    from: String,
    to: Option<String>,
    event: Option<String>,
    /// The whole target set, space-separated.
    targets: String,
    /// Every descriptor as written, space-separated.
    events: String,
    index: u64,
    annotation: String,
}

/// Run the CLI and return its stdout, panicking with stderr on failure.
fn run(args: &[&str], what: &str) -> String {
    let output = Command::new(sce_codegen_bin())
        .env("SCE_WORKSPACE_ROOT", repo_root())
        .args(args)
        .output()
        .expect("spawn sce-codegen");
    assert!(
        output.status.success(),
        "{what} failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    String::from_utf8(output.stdout).expect("utf-8 stdout")
}

fn write_doc(dir: &Path, text: &str) -> PathBuf {
    let path = dir.join(format!("{STEM}.scxml"));
    std::fs::write(&path, text).expect("write fixture document");
    path
}

/// The export of `doc`: its header record and its rows.
fn export(doc: &Path) -> (serde_json::Value, Vec<serde_json::Value>) {
    let stdout = run(
        &["transition-table", "--header", doc.to_str().unwrap()],
        "transition-table --header",
    );
    let mut lines: Vec<serde_json::Value> = stdout
        .lines()
        .map(|l| serde_json::from_str(l).expect("a line of the export is JSON"))
        .collect();
    let header = lines.remove(0);
    assert_eq!(header["record"], "header");
    (header, lines)
}

/// What the table of each backend must hold, derived from the export alone: the
/// rule the contract states, applied to the rows rather than to the model the
/// generator used.
fn expected_entries(rows: &[serde_json::Value]) -> Vec<Entry> {
    let states: Vec<&str> = rows
        .iter()
        .filter(|r| r["kind"] == "state")
        .map(|r| r["from"].as_str().unwrap())
        .collect();
    rows.iter()
        .filter(|r| r["kind"] == "transition")
        .map(|r| {
            let targets = r["targets"].as_array().expect("a transition has targets");
            let descriptors: Vec<&str> = r["event"].as_str().unwrap().split_whitespace().collect();
            let event = match descriptors.as_slice() {
                [only] if *only != "(eventless)" && *only != "*" && !only.ends_with(".*") => {
                    Some(only.to_string())
                }
                _ => None,
            };
            let source = r["source"].as_str().unwrap();
            Entry {
                from: r["from"].as_str().unwrap().to_string(),
                to: targets
                    .first()
                    .and_then(|t| t.as_str())
                    .filter(|t| states.contains(t))
                    .map(str::to_string),
                event,
                targets: targets
                    .iter()
                    .map(|t| t.as_str().expect("a target is a string"))
                    .collect::<Vec<_>>()
                    .join(" "),
                events: if descriptors == ["(eventless)"] {
                    String::new()
                } else {
                    descriptors.join(" ")
                },
                index: r["index"].as_u64().unwrap(),
                annotation: if source == "(none)" {
                    String::new()
                } else {
                    source.to_string()
                },
            }
        })
        .collect()
}

/// The identifiers a backend names a state and an event by, from the ids, using
/// the same filters its template applies.
struct Names {
    state: fn(&str) -> String,
    event: fn(&str) -> String,
}

/// Which events are the prefix of another and so take Kotlin's `.Self`.
fn kotlin_branch_events(events: &[String]) -> std::collections::HashSet<String> {
    let set: std::collections::BTreeSet<String> = events.iter().cloned().collect();
    let tree = sce_build::kotlin::build_event_tree(&set);
    sce_build::kotlin::collect_branch_events(&tree, "")
}

/// Undo the escapes the backends' literal filters write.
fn unescape(literal: &str) -> String {
    let mut out = String::new();
    let mut chars = literal.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('x') => {
                let hex: String = chars.by_ref().take(2).collect();
                out.push(char::from_u32(u32::from_str_radix(&hex, 16).unwrap()).unwrap());
            }
            Some(other) => out.push(other),
            None => panic!("a literal ends inside an escape: {literal}"),
        }
    }
    out
}

const LITERAL: &str = r#""((?:[^"\\]|\\.)*)""#;

fn opt(captures: &regex::Captures, group: usize) -> Option<String> {
    captures.get(group).map(|m| m.as_str().to_string())
}

/// Read the table of one backend's generated source back into entries whose
/// states and events are the identifiers the file spells.
fn read_table(lang: &str, source: &str, machine: &str) -> Vec<Entry> {
    let (pattern, group_count) = match lang {
        "cpp" => (
            format!(
                r"TransitionRecord\{{\s*State::(\w+),\s*(?:State::(\w+)|std::nullopt),\s*(?:Event::(\w+)|std::nullopt),\s*{LITERAL},\s*{LITERAL},\s*(\d+)u,\s*{LITERAL}\s*\}}"
            ),
            7,
        ),
        "rust" => (
            format!(
                r"from:\s*{machine}State::(\w+),\s*to:\s*(?:Some\({machine}State::(\w+)\)|None),\s*event:\s*(?:Some\({machine}Event::(\w+)\)|None),\s*targets:\s*{LITERAL},\s*events:\s*{LITERAL},\s*index:\s*(\d+),\s*annotation:\s*{LITERAL},?\s*\}}"
            ),
            7,
        ),
        "go" => (
            format!(
                r"\{{From:\s*{machine}State(\w+),\s*(?:To:\s*{machine}State(\w+),\s*HasTo:\s*true,\s*)?(?:Event:\s*{machine}Event(\w+),\s*HasEvent:\s*true,\s*)?Targets:\s*{LITERAL},\s*Events:\s*{LITERAL},\s*Index:\s*(\d+),\s*Annotation:\s*{LITERAL},?\s*\}}"
            ),
            7,
        ),
        "kotlin" => (
            format!(
                r"{machine}Transition\(\s*{machine}State\.(\w+),\s*(?:{machine}State\.(\w+)|null),\s*(?:{machine}Event\.([\w.]+)|null),\s*{LITERAL},\s*{LITERAL},\s*(\d+),\s*{LITERAL},?\s*\)"
            ),
            7,
        ),
        "python" => (
            format!(
                r"{machine}Transition\(\s*{machine}State\.(\w+),\s*(?:{machine}State\.(\w+)|None),\s*(?:{machine}Event\.(\w+)|None),\s*{LITERAL},\s*{LITERAL},\s*(\d+),\s*{LITERAL},?\s*\)"
            ),
            7,
        ),
        "c11" => {
            let prefix = Regex::new(r"#define\s+(\w+)_TRANSITION_COUNT\b")
                .unwrap()
                .captures(source)
                .map(|c| c[1].to_string())
                .expect("the header defines the transition count");
            (
                format!(
                    r"\{{\s*{prefix}_STATE_(\w+),\s*(?:{prefix}_STATE_(\w+)),\s*(?:{prefix}_EVENT_(\w+)),\s*{LITERAL},\s*{LITERAL},\s*(\d+)u,\s*{LITERAL}\s*\}}"
                ),
                7,
            )
        }
        other => panic!("no reader for {other}"),
    };
    let re = Regex::new(&pattern).unwrap_or_else(|e| panic!("{lang}: {e}"));
    re.captures_iter(source)
        .map(|c| {
            assert_eq!(c.len() - 1, group_count);
            let none_state = lang == "c11" && opt(&c, 2).as_deref() == Some("COUNT");
            let none_event = lang == "c11" && opt(&c, 3).as_deref() == Some("NONE");
            Entry {
                from: c[1].to_string(),
                to: if none_state { None } else { opt(&c, 2) },
                event: if none_event { None } else { opt(&c, 3) },
                targets: unescape(&c[4]),
                events: unescape(&c[5]),
                index: c[6].parse().unwrap(),
                annotation: unescape(&c[7]),
            }
        })
        .collect()
}

/// The source of every file of `lang` a generate run wrote, concatenated.
fn generated_source(lang: &str, out: &Path) -> String {
    let exts: &[&str] = match lang {
        "cpp" => &["h", "inl"],
        "c11" => &["h", "c"],
        "rust" => &["rs"],
        "go" => &["go"],
        "kotlin" => &["kt"],
        "python" => &["py"],
        other => panic!("{other}"),
    };
    let mut files: Vec<PathBuf> = std::fs::read_dir(out)
        .expect("read output dir")
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| exts.contains(&e))
        })
        .collect();
    files.sort();
    files
        .iter()
        .map(|p| std::fs::read_to_string(p).expect("read generated file"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn generate(lang: &str, out: &Path, doc: &Path) {
    run(
        &[
            "generate",
            "-l",
            lang,
            "-o",
            out.to_str().unwrap(),
            doc.to_str().unwrap(),
        ],
        &format!("generate -l {lang}"),
    );
}

/// The machine's name as the Rust, Go, Kotlin and Python templates spell it.
fn machine_name() -> String {
    filters::to_pascal_case(STEM.to_string())
}

fn names_of(lang: &str) -> Names {
    match lang {
        "cpp" => Names {
            state: |s| filters::capitalize_state(s.to_string()),
            event: |e| filters::to_cpp_event_variant(e.into()),
        },
        "c11" => Names {
            state: |s| filters::to_c11_upper_ident(s.into()),
            event: |e| filters::to_c11_upper_ident(e.into()),
        },
        "rust" | "go" => Names {
            state: |s| filters::to_state_variant(s.to_string()),
            event: |e| filters::to_event_variant(e.to_string()),
        },
        "kotlin" => Names {
            state: |s| filters::to_state_class_name(s.to_string()),
            // The `.Self` of a branch event is added by the caller, which knows
            // the whole event set.
            event: |e| filters::to_event_class_name(e.to_string()),
        },
        "python" => Names {
            state: |s| filters::to_python_const(s.to_string()),
            event: |e| filters::to_python_const(e.to_string()),
        },
        other => panic!("{other}"),
    }
}

/// The entries the export demands, spelled as `lang` spells them.
fn expected_spelled(lang: &str, expected: &[Entry], all_events: &[String]) -> Vec<Entry> {
    let names = names_of(lang);
    let branch = kotlin_branch_events(all_events);
    expected
        .iter()
        .map(|e| Entry {
            from: (names.state)(&e.from),
            to: e.to.as_deref().map(names.state),
            event: e.event.as_deref().map(|ev| {
                let spelled = (names.event)(ev);
                if lang == "kotlin" && branch.contains(ev) {
                    format!("{spelled}.Self")
                } else {
                    spelled
                }
            }),
            targets: e.targets.clone(),
            events: e.events.clone(),
            index: e.index,
            annotation: e.annotation.clone(),
        })
        .collect()
}

/// The events the table can name: the literal event of every transition.
fn literal_events(expected: &[Entry]) -> Vec<String> {
    let mut events: Vec<String> = expected.iter().filter_map(|e| e.event.clone()).collect();
    events.sort();
    events.dedup();
    events
}

#[test]
fn every_backend_carries_the_table_the_export_lists() {
    let dir = tempfile::tempdir().expect("tempdir");
    let doc = write_doc(dir.path(), DOC);
    let (header, rows) = export(&doc);
    let expected = expected_entries(&rows);
    let events = literal_events(&expected);
    assert_eq!(
        expected.len(),
        10 + 1,
        "the fixture has eleven transitions; the comparison is vacuous below that"
    );
    assert!(
        expected.iter().any(|e| e.to.is_none())
            && expected.iter().any(|e| e.event.is_none())
            && expected.iter().any(|e| e.annotation.contains('$')),
        "the fixture must reach an absent target, an absent event and an annotation to escape"
    );

    for lang in backends() {
        let out = dir.path().join(format!("out_{lang}"));
        generate(lang, &out, &doc);
        let source = generated_source(lang, &out);
        let table = read_table(lang, &source, &machine_name());
        assert_eq!(
            table,
            expected_spelled(lang, &expected, &events),
            "[{lang}] the table is not the export's transitions"
        );
        assert!(
            source.contains(header["source_hash"].as_str().unwrap()),
            "[{lang}] the table does not carry the source hash the export names"
        );
    }
}

/// The hash a backend embeds in the table is the one in the file's own header.
#[test]
fn the_table_carries_the_hash_of_the_header_of_its_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let doc = write_doc(dir.path(), DOC);
    let header_hash = Regex::new(r"source-hash:\s*([0-9a-f]{64})").unwrap();
    for lang in backends() {
        let out = dir.path().join(format!("out_{lang}"));
        generate(lang, &out, &doc);
        let source = generated_source(lang, &out);
        let header: BTreeMap<String, usize> =
            header_hash
                .captures_iter(&source)
                .fold(BTreeMap::new(), |mut acc, c| {
                    *acc.entry(c[1].to_string()).or_default() += 1;
                    acc
                });
        assert_eq!(
            header.len(),
            1,
            "[{lang}] the files of one run disagree about the source hash: {header:?}"
        );
        let hash = header.keys().next().unwrap();
        let embedded = Regex::new(&format!(r#""{hash}""#)).unwrap();
        assert!(
            embedded.is_match(&source),
            "[{lang}] no string constant of the generated source holds the hash of its own header"
        );
    }
}

/// A document that does not declare a table gets none, and one that does gets it
/// in every backend: the declaration is what moves the output, and nothing on the
/// command line.
#[test]
fn the_table_is_emitted_only_when_the_document_declares_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let undeclared = dir.path().join("undeclared");
    let declared = dir.path().join("declared");
    std::fs::create_dir_all(&undeclared).expect("create directory");
    std::fs::create_dir_all(&declared).expect("create directory");
    let without_doc = write_doc(
        &undeclared,
        &DOC.replace(r#" sce:transition-table="emit""#, ""),
    );
    assert_ne!(
        std::fs::read_to_string(&without_doc).unwrap(),
        DOC,
        "the fixture must differ from DOC by the declaration alone"
    );
    let with_doc = write_doc(&declared, DOC);
    // The one value every backend's table carries and nothing else writes: the
    // hash as a named constant (`SOURCE_HASH`, `<Machine>SourceHash`, the C++
    // accessor `sourceHash`), distinct from the `source-hash` header comment.
    let marker = Regex::new(r"SOURCE_HASH|SourceHash|sourceHash").unwrap();
    for lang in backends() {
        let without = dir.path().join(format!("without_{lang}"));
        let with = dir.path().join(format!("with_{lang}"));
        generate(lang, &without, &without_doc);
        generate(lang, &with, &with_doc);
        assert!(
            !marker.is_match(&generated_source(lang, &without)),
            "[{lang}] a document that declares no table was generated with one"
        );
        assert!(
            marker.is_match(&generated_source(lang, &with)),
            "[{lang}] sce:transition-table=\"emit\" did not put a table into the output"
        );
    }
}

/// The declaration has one value, and any other is refused with the attribute's
/// name rather than read as a spelling of "no".
#[test]
fn a_value_other_than_emit_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let doc = write_doc(
        dir.path(),
        &DOC.replace(
            r#"sce:transition-table="emit""#,
            r#"sce:transition-table="yes""#,
        ),
    );
    let out = dir.path().join("out");
    let output = Command::new(sce_codegen_bin())
        .env("SCE_WORKSPACE_ROOT", repo_root())
        .args(["generate", "-l", "rust", "-o", out.to_str().unwrap()])
        .arg(&doc)
        .output()
        .expect("spawn sce-codegen");
    assert!(
        !output.status.success(),
        "a refused declaration generated a machine"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("sce:transition-table"),
        "the refusal does not name the attribute:\n{stderr}"
    );
}

/// Each backend writes an annotation as the literal its language needs, and that
/// literal is the one written here from the export.
///
/// Reading a table back unescapes what the template wrote, so a `$` left bare in
/// Kotlin reads back as the `$` it should be: the round trip cannot tell `\$` from
/// `$`, and only the compiler can, by reading `$x` as the interpolation of a name that
/// is not there. This asks the source itself for the spelling the language requires.
#[test]
fn every_backend_writes_an_annotation_as_the_literal_its_language_needs() {
    let work = scratch("annotation_spelling");
    let doc = write_doc(&work, DOC);
    let (_, rows) = export(&doc);
    let expected = expected_entries(&rows);
    let annotations: Vec<&str> = expected
        .iter()
        .map(|e| e.annotation.as_str())
        .filter(|a| !a.is_empty())
        .collect();
    assert!(
        annotations.iter().any(|a| a.contains('$')),
        "the document must carry a `$` for this to ask about it"
    );
    for lang in emitting(ForgeKind::Statechart)
        // Python: `literal_of` writes no Python literal; its table is held by
        // `the_python_table_imports_and_holds_the_export`, which imports the
        // module and compares the values the literals build.
        .filter(|language| *language != Language::Python)
        .map(Language::canonical_name)
    {
        let out = work.join(format!("out_{lang}"));
        generate(lang, &out, &doc);
        let source = generated_source(lang, &out);
        for annotation in &annotations {
            let literal = literal_of(lang, annotation);
            assert!(
                source.contains(&literal),
                "[{lang}] the source does not hold the annotation as {literal}"
            );
        }
    }
}

/// An annotation is opaque: whatever characters its tokens carry reach the table
/// as the same text, in every backend.
#[test]
fn an_annotation_round_trips_through_every_backend_unread() {
    let tokens = [
        "7",
        "REQ-\"Q\"",
        "a\\b",
        "$x",
        "{y}",
        "50%",
        "caf\u{e9}",
        "tab\\n",
        "x'y",
        "`z`",
    ];
    let doc_text = DOC.replace(
        r#"sce:req="12 REQ-&quot;Q&quot; a\b $x {y} 50%""#,
        &format!(
            r#"sce:req="{}""#,
            tokens
                .iter()
                .map(|t| t.replace('"', "&quot;"))
                .collect::<Vec<_>>()
                .join(" ")
        ),
    );
    let dir = tempfile::tempdir().expect("tempdir");
    let doc = write_doc(dir.path(), &doc_text);
    let (_, rows) = export(&doc);
    let expected = expected_entries(&rows);
    assert_eq!(expected[0].annotation, tokens.join(" "));
    let events = literal_events(&expected);
    for lang in backends() {
        let out = dir.path().join(format!("out_{lang}"));
        generate(lang, &out, &doc);
        let table = read_table(lang, &generated_source(lang, &out), &machine_name());
        assert_eq!(
            table,
            expected_spelled(lang, &expected, &events),
            "[{lang}] an annotation did not survive the table"
        );
    }
}

/// A machine with no transition has a table, and it is empty.
#[test]
fn a_machine_without_transitions_has_an_empty_table() {
    let doc_text = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" sce:transition-table="emit"
       version="1.0" name="ttemit" initial="only" datamodel="null">
  <final id="only"/>
</scxml>
"#;
    let dir = tempfile::tempdir().expect("tempdir");
    let doc = write_doc(dir.path(), doc_text);
    for lang in backends() {
        let out = dir.path().join(format!("out_{lang}"));
        generate(lang, &out, &doc);
        let source = generated_source(lang, &out);
        assert!(
            read_table(lang, &source, &machine_name()).is_empty(),
            "[{lang}] a machine with no transition lists one"
        );
    }
}

/// The templates read the table from the record list the generator hands them,
/// and from nowhere else: no block between the markers walks the model's
/// states or transitions.
#[test]
fn no_template_reads_transitions_from_the_model() {
    let templates = repo_root().join("tools/codegen/templates");
    let files = [
        "utility_methods.jinja2",
        "rust/state_machine.rs.jinja2",
        "go/state_machine.go.jinja2",
        "kotlin/state_machine.kt.jinja2",
        "python/state_machine.py.jinja2",
        "c/state_machine.h.jinja2",
        "c/state_machine.c.jinja2",
    ];
    let forbidden = [
        "model.states",
        "state.transitions",
        "model.events",
        "history_states",
    ];
    let mut blocks = 0;
    for file in files {
        let text = std::fs::read_to_string(templates.join(file)).expect("read template");
        let mut rest = text.as_str();
        while let Some(start) = rest.find("emitted-transition-table:begin") {
            let after = &rest[start..];
            let end = after
                .find("emitted-transition-table:end")
                .unwrap_or_else(|| panic!("{file}: a begin marker has no end marker"));
            let block = &after[..end];
            for needle in forbidden {
                assert!(
                    !block.contains(needle),
                    "{file}: a table block reads `{needle}` from the model instead of the record list"
                );
            }
            blocks += 1;
            rest = &after[end..];
        }
    }
    assert!(
        blocks >= files.len(),
        "found {blocks} marked block(s) in {} template files; a template lost its markers",
        files.len()
    );
}

// ── compile and parse checks where a toolchain is found ──────────────────

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("emitted_transition_table");
    let dir = dir.join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch");
    dir
}

/// A C++ literal of `text`, the way the template writes one.
fn cpp_literal(text: &str) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    format!("\"{escaped}\"")
}

/// The generated C++ compiles, and its table says what the export says, checked by
/// the compiler.
#[test]
fn the_cpp_table_compiles_and_holds_the_export() {
    let Some(gpp) = toolchain::locate("g++") else {
        toolchain::skipped("the_cpp_table_compiles_and_holds_the_export: g++ not on PATH");
        return;
    };
    let work = scratch("cpp");
    let doc = write_doc(&work, DOC);
    let (_, rows) = export(&doc);
    let expected = expected_entries(&rows);
    let out = work.join("out");
    generate("cpp", &out, &doc);

    let names = names_of("cpp");
    let mut stub = format!(
        "#include \"{STEM}_sm.h\"\nusing P = SCE::Generated::{STEM}::{STEM}Policy;\n\
         static_assert(P::transitionCount() == {n}u);\n\
         static_assert(P::transitions.size() == {n}u);\n",
        n = expected.len()
    );
    for (i, e) in expected.iter().enumerate() {
        stub.push_str(&format!(
            "static_assert(P::transitions[{i}].from == P::State::{});\n",
            (names.state)(&e.from)
        ));
        match &e.to {
            Some(to) => stub.push_str(&format!(
                "static_assert(P::transitions[{i}].to.has_value() && *P::transitions[{i}].to == P::State::{});\n",
                (names.state)(to)
            )),
            None => stub.push_str(&format!(
                "static_assert(!P::transitions[{i}].to.has_value());\n"
            )),
        }
        match &e.event {
            Some(ev) => stub.push_str(&format!(
                "static_assert(P::transitions[{i}].event.has_value() && *P::transitions[{i}].event == P::Event::{});\n",
                (names.event)(ev)
            )),
            None => stub.push_str(&format!(
                "static_assert(!P::transitions[{i}].event.has_value());\n"
            )),
        }
        stub.push_str(&format!(
            "static_assert(std::string_view(P::transitions[{i}].targets) == std::string_view({}));\n",
            cpp_literal(&e.targets)
        ));
        stub.push_str(&format!(
            "static_assert(std::string_view(P::transitions[{i}].events) == std::string_view({}));\n",
            cpp_literal(&e.events)
        ));
        stub.push_str(&format!(
            "static_assert(P::transitions[{i}].index == {}u);\n",
            e.index
        ));
        stub.push_str(&format!(
            "static_assert(std::string_view(P::transitions[{i}].annotation) == std::string_view({}));\n",
            cpp_literal(&e.annotation)
        ));
    }
    stub.push_str("int main() { return 0; }\n");
    let stub_path = work.join("stub.cpp");
    std::fs::write(&stub_path, stub).expect("write stub");

    let mut cmd = Command::new(&gpp);
    cmd.args(["-std=c++20", "-fsyntax-only"]);
    for inc in [
        repo_root().join("sce/include"),
        repo_root().join("third_party/spdlog/include"),
        repo_root().join("third_party/nlohmann_json/include"),
        out,
    ] {
        cmd.arg("-I").arg(inc);
    }
    cmd.arg(&stub_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = cmd.output().expect("run g++");
    assert!(
        output.status.success(),
        "g++ rejected the generated header or a static assertion over its table:\n{}",
        String::from_utf8_lossy(&output.stderr),
    );
}

/// The generated Python imports, and the table it builds is the export's.
///
/// Importing, not parsing: a module that parses can still fail the moment it is
/// loaded. The table's first version used `dataclass` in a file that only imports it
/// when the machine has a payload or a host invoker, and `ast.parse` passed it.
#[test]
fn the_python_table_imports_and_holds_the_export() {
    let Some(python) = toolchain::locate_any(&["python3", "python"]) else {
        toolchain::skipped("the_python_table_imports_and_holds_the_export: python not on PATH");
        return;
    };
    // The runtime every generated machine imports needs the Lua binding, and a host
    // without it cannot import any machine at all: that is a missing tool, not a
    // table that is wrong.
    let has_lupa = Command::new(&python)
        .args(["-I", "-c", "import lupa"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !has_lupa {
        toolchain::skipped(
            "the_python_table_imports_and_holds_the_export: lupa is not importable \
             (`python3 -m pip install lupa`)",
        );
        return;
    }
    let work = scratch("python");
    let doc = write_doc(&work, DOC);
    let (header, rows) = export(&doc);
    let expected = expected_entries(&rows);
    let events = literal_events(&expected);
    let out = work.join("out");
    generate("python", &out, &doc);

    let script = r#"
import json, sys
sys.path.insert(0, sys.argv[1])
sys.path.insert(0, sys.argv[2])
import importlib
module = importlib.import_module(sys.argv[3])
table = [
    {
        "from": t.from_state.name,
        "to": None if t.to is None else t.to.name,
        "event": None if t.event is None else t.event.name,
        "targets": t.targets,
        "events": t.events,
        "index": t.index,
        "annotation": t.annotation,
    }
    for t in module.TRANSITIONS
]
print(json.dumps({"hash": module.SOURCE_HASH, "table": table}))
"#;
    let output = Command::new(&python)
        .args(["-I", "-c", script])
        .arg(&out)
        .arg(repo_root().join("backends/python/runtime"))
        .arg(format!("{STEM}_sm"))
        .output()
        .expect("run python");
    assert!(
        output.status.success(),
        "the generated Python does not import:\n{}",
        String::from_utf8_lossy(&output.stderr),
    );
    let loaded: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("the script prints JSON");
    assert_eq!(loaded["hash"], header["source_hash"]);
    let table: Vec<Entry> = loaded["table"]
        .as_array()
        .expect("a table")
        .iter()
        .map(|e| Entry {
            from: e["from"].as_str().unwrap().to_string(),
            to: e["to"].as_str().map(str::to_string),
            event: e["event"].as_str().map(str::to_string),
            targets: e["targets"].as_str().unwrap().to_string(),
            events: e["events"].as_str().unwrap().to_string(),
            index: e["index"].as_u64().unwrap(),
            annotation: e["annotation"].as_str().unwrap().to_string(),
        })
        .collect();
    assert_eq!(table, expected_spelled("python", &expected, &events));
}

// ── run the table in each backend's own toolchain ─────────────────────────
//
// Reading a table back from its source proves what was written, and compiling it
// proves it is code. Neither proves what the compiled table IS: a string literal the
// template escaped wrongly still compiles (Kotlin reads `$x` as an interpolation, and
// `\x` is an escape in some languages and not in others), and an enumerator that
// resolves to a different variant than the one spelled is no error. So each of
// Rust, Go, Kotlin and C is built against its real runtime, the table is read at
// run time, and every field of every entry is compared with the export. The
// expectation is written here, in the language's own syntax, from the export and the
// filters, and not by the template under test, the way the C++ assertions are.

/// A literal of `text` in `lang`, written here and not by the templates.
fn literal_of(lang: &str, text: &str) -> String {
    let literal = cpp_literal(text);
    if lang == "kotlin" {
        literal.replace('$', "\\$")
    } else {
        literal
    }
}

/// Every field of every entry of the table, as a list of `(what, condition)` in
/// the syntax of `lang`. `table` is the expression naming the table.
fn field_conditions(
    lang: &str,
    machine: &str,
    table: &str,
    spelled: &[Entry],
) -> Vec<(String, String)> {
    let upper = STEM.to_uppercase();
    let mut out = Vec::new();
    for (i, e) in spelled.iter().enumerate() {
        let at = format!("{table}[{i}]");
        let (from, to, event, targets, events, index, annotation);
        match lang {
            "rust" => {
                from = format!("{at}.from == {machine}State::{}", e.from);
                to = match &e.to {
                    Some(s) => format!("{at}.to == Some({machine}State::{s})"),
                    None => format!("{at}.to.is_none()"),
                };
                event = match &e.event {
                    Some(v) => format!("{at}.event == Some({machine}Event::{v})"),
                    None => format!("{at}.event.is_none()"),
                };
                targets = format!("{at}.targets == {}", literal_of(lang, &e.targets));
                events = format!("{at}.events == {}", literal_of(lang, &e.events));
                index = format!("{at}.index == {}", e.index);
                annotation = format!("{at}.annotation == {}", literal_of(lang, &e.annotation));
            }
            "go" => {
                from = format!("{at}.From == tt.{machine}State{}", e.from);
                to = match &e.to {
                    Some(s) => format!("{at}.HasTo && {at}.To == tt.{machine}State{s}"),
                    None => format!("!{at}.HasTo"),
                };
                event = match &e.event {
                    Some(v) => format!("{at}.HasEvent && {at}.Event == tt.{machine}Event{v}"),
                    None => format!("!{at}.HasEvent"),
                };
                targets = format!("{at}.Targets == {}", literal_of(lang, &e.targets));
                events = format!("{at}.Events == {}", literal_of(lang, &e.events));
                index = format!("int({at}.Index) == {}", e.index);
                annotation = format!("{at}.Annotation == {}", literal_of(lang, &e.annotation));
            }
            "kotlin" => {
                from = format!("{at}.from == {machine}State.{}", e.from);
                to = match &e.to {
                    Some(s) => format!("{at}.to == {machine}State.{s}"),
                    None => format!("{at}.to == null"),
                };
                event = match &e.event {
                    Some(v) => format!("{at}.event == {machine}Event.{v}"),
                    None => format!("{at}.event == null"),
                };
                targets = format!("{at}.targets == {}", literal_of(lang, &e.targets));
                events = format!("{at}.events == {}", literal_of(lang, &e.events));
                index = format!("{at}.index == {}", e.index);
                annotation = format!("{at}.annotation == {}", literal_of(lang, &e.annotation));
            }
            "c11" => {
                from = format!("{at}.from == {upper}_STATE_{}", e.from);
                to = match &e.to {
                    Some(s) => format!("{at}.to == {upper}_STATE_{s}"),
                    None => format!("{at}.to == {upper}_STATE_COUNT"),
                };
                event = match &e.event {
                    Some(v) => format!("{at}.event == {upper}_EVENT_{v}"),
                    None => format!("{at}.event == {upper}_EVENT_NONE"),
                };
                targets = format!(
                    "strcmp({at}.targets, {}) == 0",
                    literal_of(lang, &e.targets)
                );
                events = format!("strcmp({at}.events, {}) == 0", literal_of(lang, &e.events));
                index = format!("{at}.index == {}u", e.index);
                annotation = format!(
                    "strcmp({at}.annotation, {}) == 0",
                    literal_of(lang, &e.annotation)
                );
            }
            other => panic!("{other}"),
        }
        for (field, cond) in [
            ("from", from),
            ("to", to),
            ("event", event),
            ("targets", targets),
            ("events", events),
            ("index", index),
            ("annotation", annotation),
        ] {
            out.push((format!("entry {i} {field}"), cond));
        }
    }
    out
}

/// Run `command`, failing with everything it printed.
fn run_ok(mut command: Command, what: &str) -> String {
    let output = command
        .output()
        .unwrap_or_else(|e| panic!("run {what}: {e}"));
    output_ok(output, what)
}

/// What a finished `what` printed, failing with all of it unless it succeeded.
fn output_ok(output: Output, what: &str) -> String {
    assert!(
        output.status.success(),
        "{what} failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// What a backend's table is held to: the export's header and the entries spelled
/// the way that backend spells them, with the scratch folder and the document.
fn held_to(lang: &str) -> (serde_json::Value, Vec<Entry>, PathBuf, PathBuf) {
    let work = scratch(&format!("{lang}_run"));
    let doc = write_doc(&work, DOC);
    let (header, rows) = export(&doc);
    let expected = expected_entries(&rows);
    let events = literal_events(&expected);
    let spelled = expected_spelled(lang, &expected, &events);
    (header, spelled, work, doc)
}

/// The generated Rust is built as a crate of its own against the real runtime, and
/// its table is compared with the export when that program runs.
#[test]
fn the_rust_table_compiles_and_holds_the_export() {
    let Some(cargo) = toolchain::locate("cargo") else {
        toolchain::skipped("the_rust_table_compiles_and_holds_the_export: cargo not on PATH");
        return;
    };
    let (header, spelled, work, doc) = held_to("rust");
    let out = work.join("out");
    generate("rust", &out, &doc);
    let machine = machine_name();

    let mut program = format!(
        "#[path = {:?}]\nmod machine;\nuse machine::*;\n\nfn main() {{\n    let t = {machine}Transition::ALL;\n    assert_eq!(t.len(), {});\n    assert_eq!({machine}Transition::SOURCE_HASH, {:?});\n",
        out.join(format!("{STEM}_sm.rs")).to_str().unwrap(),
        spelled.len(),
        header["source_hash"].as_str().unwrap(),
    );
    for (what, cond) in field_conditions("rust", &machine, "t", &spelled) {
        program.push_str(&format!("    assert!({cond}, {what:?});\n"));
    }
    program.push_str("}\n");

    let project = work.join("project");
    std::fs::create_dir_all(project.join("src")).expect("create project");
    std::fs::write(
        project.join("Cargo.toml"),
        format!(
            "[package]\nname = \"ttemit-check\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\n\n\
             [dependencies]\nsce-rust-runtime = {{ path = {:?} }}\n\n[workspace]\n",
            repo_root().join("backends/rust/runtime").to_str().unwrap()
        ),
    )
    .expect("write manifest");
    std::fs::write(project.join("src/main.rs"), program).expect("write program");
    // The repo's own lock, so the runtime's dependencies resolve to the versions
    // already fetched and the build needs no network.
    std::fs::copy(repo_root().join("Cargo.lock"), project.join("Cargo.lock"))
        .expect("copy the lock file");

    // Offline first, through the helper every probe-crate build shares
    // (`common::run_cargo_offline_first`): a cache that answers keeps the build
    // off the network, and one that cannot, on a cold runner, reaches it rather
    // than failing the push for a registry it never needed.
    let manifest = project.join("Cargo.toml");
    let target_dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("emitted_transition_table_rust_target");
    let run = common::run_cargo_offline_first(|| {
        let mut build = Command::new(&cargo);
        build
            .args(["run", "--quiet", "--manifest-path"])
            .arg(&manifest)
            .env("CARGO_TARGET_DIR", &target_dir);
        build
    });
    output_ok(run.output, "the generated Rust table program");
}

/// The generated Go is built as a package of its own against the real runtime, and
/// its table is compared with the export when that program runs.
#[test]
fn the_go_table_compiles_and_holds_the_export() {
    let Some(go) = toolchain::locate("go") else {
        toolchain::skipped("the_go_table_compiles_and_holds_the_export: go not on PATH");
        return;
    };
    let (header, spelled, work, doc) = held_to("go");
    let machine = machine_name();
    generate("go", &work.join("project").join(STEM), &doc);

    let mut program = format!(
        "package main\n\nimport (\n\t\"fmt\"\n\t\"os\"\n\n\ttt \"example.com/ttemitcheck/{STEM}\"\n)\n\n\
         func main() {{\n\tbad := 0\n\texpect := func(ok bool, what string) {{\n\t\tif !ok {{\n\t\t\tfmt.Println(\"MISMATCH:\", what)\n\t\t\tbad++\n\t\t}}\n\t}}\n\
         \tt := tt.{machine}Transitions\n\texpect(len(t) == {}, \"entry count\")\n\texpect(tt.{machine}SourceHash == {}, \"source hash\")\n",
        spelled.len(),
        literal_of("go", header["source_hash"].as_str().unwrap()),
    );
    for (what, cond) in field_conditions("go", &machine, "t", &spelled) {
        program.push_str(&format!("\texpect({cond}, {})\n", literal_of("go", &what)));
    }
    program.push_str("\tif bad > 0 {\n\t\tos.Exit(1)\n\t}\n}\n");

    let project = work.join("project");
    std::fs::write(
        project.join("go.mod"),
        format!(
            "module example.com/ttemitcheck\n\ngo 1.22\n\nrequire github.com/newmassrael/sce-go-runtime v0.0.0\n\n\
             replace github.com/newmassrael/sce-go-runtime => {}\n",
            repo_root().join("backends/go/runtime").to_str().unwrap()
        ),
    )
    .expect("write go.mod");
    std::fs::write(project.join("main.go"), program).expect("write the program");

    let mut build = Command::new(&go);
    build
        .args(["run", "."])
        .current_dir(&project)
        .env("GOFLAGS", "-mod=mod")
        .env("GOWORK", "off");
    run_ok(build, "the generated Go table program");
}

/// The generated Kotlin is compiled against the real runtime jar, and its table is
/// compared with the export when that program runs.
#[test]
fn the_kotlin_table_compiles_and_holds_the_export() {
    let Some(kotlinc) = toolchain::locate("kotlinc") else {
        toolchain::skipped("the_kotlin_table_compiles_and_holds_the_export: kotlinc not on PATH");
        return;
    };
    let Some(java) = toolchain::locate("java") else {
        toolchain::skipped("the_kotlin_table_compiles_and_holds_the_export: java not on PATH");
        return;
    };
    let libs = repo_root().join("backends/kotlin/runtime/build/libs");
    let mut jars: Vec<PathBuf> = std::fs::read_dir(&libs)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            name.starts_with("sce-kotlin-runtime-jvm-")
                && name.ends_with(".jar")
                && !name.ends_with("-sources.jar")
        })
        .collect();
    jars.sort();
    let Some(runtime_jar) = jars.pop() else {
        toolchain::skipped(
            "the_kotlin_table_compiles_and_holds_the_export: runtime jar missing under \
             backends/kotlin/runtime/build/libs (run `./gradlew :sce-kotlin-runtime:assemble`)",
        );
        return;
    };
    let (header, spelled, work, doc) = held_to("kotlin");
    let out = work.join("out");
    generate("kotlin", &out, &doc);
    let machine = machine_name();

    let mut program = format!(
        "import com.sce.generated.{STEM}.*\n\nfun main() {{\n    var bad = 0\n    fun expect(ok: Boolean, what: String) {{\n        if (!ok) {{\n            System.err.println(\"MISMATCH: \" + what)\n            bad++\n        }}\n    }}\n\
         \x20   val t = {machine}Transitions.all\n    expect(t.size == {}, \"entry count\")\n    expect({machine}Transitions.SOURCE_HASH == {}, \"source hash\")\n",
        spelled.len(),
        literal_of("kotlin", header["source_hash"].as_str().unwrap()),
    );
    for (what, cond) in field_conditions("kotlin", &machine, "t", &spelled) {
        program.push_str(&format!(
            "    expect({cond}, {})\n",
            literal_of("kotlin", &what)
        ));
    }
    program.push_str("    if (bad > 0) System.exit(1)\n}\n");
    let main_path = work.join("Check.kt");
    std::fs::write(&main_path, program).expect("write the program");

    let jar = work.join("check.jar");
    let mut compile = Command::new(&kotlinc);
    compile
        .arg("-nowarn")
        .arg("-cp")
        .arg(&runtime_jar)
        .arg(out.join(format!("{STEM}Sm.kt")))
        .arg(&main_path)
        .arg("-include-runtime")
        .arg("-d")
        .arg(&jar);
    run_ok(compile, "kotlinc over the generated machine and the check");

    let mut run = Command::new(&java);
    run.arg("-cp")
        .arg(format!(
            "{}:{}",
            jar.to_str().unwrap(),
            runtime_jar.to_str().unwrap()
        ))
        .arg("CheckKt");
    run_ok(run, "the generated Kotlin table program");
}

/// The generated C is built with its own header and run, and its table is compared
/// with the export. A machine that has no datamodel needs nothing beyond libc.
#[test]
fn the_c_table_compiles_and_holds_the_export() {
    let Some(gcc) = toolchain::locate_any(&["gcc", "cc"]) else {
        toolchain::skipped("the_c_table_compiles_and_holds_the_export: gcc/cc not on PATH");
        return;
    };
    let (header, spelled, work, doc) = held_to("c11");
    let out = work.join("out");
    generate("c11", &out, &doc);
    let lower = STEM.to_lowercase();
    let upper = STEM.to_uppercase();

    let mut program = format!(
        "#include \"{STEM}_sm.h\"\n#include <stdio.h>\n#include <string.h>\n\nstatic int bad;\n\
         #define EXPECT(cond, what) do {{ if (!(cond)) {{ fprintf(stderr, \"MISMATCH: %s\\n\", what); bad++; }} }} while (0)\n\n\
         int main(void) {{\n    EXPECT({upper}_TRANSITION_COUNT == {}u, \"entry count\");\n    EXPECT(strcmp({upper}_SOURCE_HASH, {}) == 0, \"source hash\");\n",
        spelled.len(),
        literal_of("c11", header["source_hash"].as_str().unwrap()),
    );
    for (what, cond) in field_conditions("c11", "", &format!("{lower}_transitions"), &spelled) {
        program.push_str(&format!(
            "    EXPECT({cond}, {});\n",
            literal_of("c11", &what)
        ));
    }
    program.push_str("    return bad ? 1 : 0;\n}\n");
    let main_path = work.join("check.c");
    std::fs::write(&main_path, program).expect("write the program");

    let binary = work.join("check");
    let mut compile = Command::new(&gcc);
    compile
        .args(["-std=c11", "-I"])
        .arg(repo_root().join("backends/c/runtime/include"))
        .arg("-I")
        .arg(&out)
        .arg(&main_path)
        .arg(out.join(format!("{STEM}_sm.c")))
        .arg("-o")
        .arg(&binary);
    run_ok(compile, "gcc over the generated machine and the check");
    run_ok(Command::new(&binary), "the generated C table program");
}
