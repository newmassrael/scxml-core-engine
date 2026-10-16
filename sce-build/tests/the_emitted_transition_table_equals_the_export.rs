// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `sce-codegen generate --emit-transition-table` puts the transitions of a
// machine into the generated source of every backend, and
// `sce-codegen transition-table` lists the same transitions as an export. They
// are one fact with two surfaces, and this file holds each backend's table to
// the export of the document it was generated from: the same transitions, in
// the same order, with the same owning state, first target, event, index and
// annotation, and the same `source_hash` the file's own header carries.
//
// The comparison reads the generated source back rather than compiling it, so it
// runs on a host with no toolchain; where a toolchain is found, the same output is
// also compiled (C++ with static assertions over the table, C, Kotlin) or parsed
// (Rust, Go, Python), because a table that parses to the right values and does
// not compile is still wrong.
//
// Absent the flag, nothing changes: the generated files are held byte-identical
// by the committed-tree drift gates, and this file checks that none of them names
// the table.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use regex::Regex;
use sce_build::filters;
use sce_build::toolchain;

/// The six backends, by the identifier `generate -l` takes.
const BACKENDS: [&str; 6] = ["cpp", "c11", "rust", "go", "kotlin", "python"];

/// The machine's name: the file stem, which every backend derives its names from.
const STEM: &str = "ttemit";

/// One document that reaches every shape a table entry can take: a literal event, a
/// literal event that is also the prefix of another (a branch of Kotlin's event
/// tree), several descriptors, the wildcard, a prefix family, no event, a first
/// target that is a `<history>`, no target, several targets, an annotation of
/// several tokens carrying characters a string literal has to escape, and a state
/// whose id is not lower case.
const DOC: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
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
                r"TransitionRecord\{{\s*State::(\w+),\s*(?:State::(\w+)|std::nullopt),\s*(?:Event::(\w+)|std::nullopt),\s*(\d+)u,\s*{LITERAL}\s*\}}"
            ),
            5,
        ),
        "rust" => (
            format!(
                r"from:\s*{machine}State::(\w+),\s*to:\s*(?:Some\({machine}State::(\w+)\)|None),\s*event:\s*(?:Some\({machine}Event::(\w+)\)|None),\s*index:\s*(\d+),\s*annotation:\s*{LITERAL},?\s*\}}"
            ),
            5,
        ),
        "go" => (
            format!(
                r"\{{From:\s*{machine}State(\w+),\s*(?:To:\s*{machine}State(\w+),\s*HasTo:\s*true,\s*)?(?:Event:\s*{machine}Event(\w+),\s*HasEvent:\s*true,\s*)?Index:\s*(\d+),\s*Annotation:\s*{LITERAL},?\s*\}}"
            ),
            5,
        ),
        "kotlin" => (
            format!(
                r"{machine}Transition\(\s*{machine}State\.(\w+),\s*(?:{machine}State\.(\w+)|null),\s*(?:{machine}Event\.([\w.]+)|null),\s*(\d+),\s*{LITERAL},?\s*\)"
            ),
            5,
        ),
        "python" => (
            format!(
                r"{machine}Transition\(\s*{machine}State\.(\w+),\s*(?:{machine}State\.(\w+)|None),\s*(?:{machine}Event\.(\w+)|None),\s*(\d+),\s*{LITERAL},?\s*\)"
            ),
            5,
        ),
        "c11" => {
            let prefix = Regex::new(r"#define\s+(\w+)_TRANSITION_COUNT\b")
                .unwrap()
                .captures(source)
                .map(|c| c[1].to_string())
                .expect("the header defines the transition count");
            (
                format!(
                    r"\{{\s*{prefix}_STATE_(\w+),\s*(?:{prefix}_STATE_(\w+)),\s*(?:{prefix}_EVENT_(\w+)),\s*(\d+)u,\s*{LITERAL}\s*\}}"
                ),
                5,
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
                index: c[4].parse().unwrap(),
                annotation: unescape(&c[5]),
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

fn generate(lang: &str, out: &Path, doc: &Path, flag: bool) {
    let mut args = vec!["generate", "-l", lang, "-o", out.to_str().unwrap()];
    if flag {
        args.push("--emit-transition-table");
    }
    args.push(doc.to_str().unwrap());
    run(&args, &format!("generate -l {lang}"));
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

    for lang in BACKENDS {
        let out = dir.path().join(format!("out_{lang}"));
        generate(lang, &out, &doc, true);
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
    for lang in BACKENDS {
        let out = dir.path().join(format!("out_{lang}"));
        generate(lang, &out, &doc, true);
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

/// Without the flag a generated file does not mention the table, and with it
/// every backend does: the option is what moves the output.
#[test]
fn the_table_is_emitted_only_when_asked_for() {
    let dir = tempfile::tempdir().expect("tempdir");
    let doc = write_doc(dir.path(), DOC);
    // The one value every backend's table carries and nothing else writes: the
    // hash as a named constant (`SOURCE_HASH`, `<Machine>SourceHash`, the C++
    // accessor `sourceHash`), distinct from the `source-hash` header comment.
    let marker = Regex::new(r"SOURCE_HASH|SourceHash|sourceHash").unwrap();
    for lang in BACKENDS {
        let without = dir.path().join(format!("without_{lang}"));
        let with = dir.path().join(format!("with_{lang}"));
        generate(lang, &without, &doc, false);
        generate(lang, &with, &doc, true);
        assert!(
            !marker.is_match(&generated_source(lang, &without)),
            "[{lang}] a file generated without --emit-transition-table names the table"
        );
        assert!(
            marker.is_match(&generated_source(lang, &with)),
            "[{lang}] --emit-transition-table did not put a table into the output"
        );
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
    for lang in BACKENDS {
        let out = dir.path().join(format!("out_{lang}"));
        generate(lang, &out, &doc, true);
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
       version="1.0" name="ttemit" initial="only" datamodel="null">
  <final id="only"/>
</scxml>
"#;
    let dir = tempfile::tempdir().expect("tempdir");
    let doc = write_doc(dir.path(), doc_text);
    for lang in BACKENDS {
        let out = dir.path().join(format!("out_{lang}"));
        generate(lang, &out, &doc, true);
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
    generate("cpp", &out, &doc, true);

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

/// The generated C compiles with its table.
#[test]
fn the_c_table_compiles() {
    let Some(gcc) = toolchain::locate_any(&["gcc", "cc"]) else {
        toolchain::skipped("the_c_table_compiles: gcc/cc not on PATH");
        return;
    };
    let work = scratch("c11");
    let doc = write_doc(&work, DOC);
    let out = work.join("out");
    generate("c11", &out, &doc, true);
    let mut compiled = 0;
    for entry in std::fs::read_dir(&out).expect("read out").flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("c") {
            continue;
        }
        let output = Command::new(&gcc)
            .args(["-std=c11", "-fsyntax-only", "-I"])
            .arg(repo_root().join("backends/c/runtime/include"))
            .arg("-I")
            .arg(&out)
            .arg(&path)
            .output()
            .expect("run gcc");
        assert!(
            output.status.success(),
            "gcc rejected {}:\n{}",
            path.display(),
            String::from_utf8_lossy(&output.stderr),
        );
        compiled += 1;
    }
    assert!(compiled > 0, "no .c file was compiled");
}

/// The generated Rust parses with its table.
#[test]
fn the_rust_table_parses() {
    let work = scratch("rust");
    let doc = write_doc(&work, DOC);
    let out = work.join("out");
    generate("rust", &out, &doc, true);
    let source = generated_source("rust", &out);
    syn::parse_file(&source).unwrap_or_else(|e| panic!("the generated Rust does not parse: {e}"));
}

/// The generated Go parses with its table.
#[test]
fn the_go_table_parses() {
    let Some(gofmt) = toolchain::locate("gofmt") else {
        toolchain::skipped("the_go_table_parses: gofmt not on PATH");
        return;
    };
    let work = scratch("go");
    let doc = write_doc(&work, DOC);
    let out = work.join("out");
    generate("go", &out, &doc, true);
    let mut checked = 0;
    for entry in std::fs::read_dir(&out).expect("read out").flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("go") {
            continue;
        }
        let output = Command::new(&gofmt)
            .arg("-e")
            .arg(&path)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .expect("run gofmt");
        assert!(
            output.status.success(),
            "gofmt -e rejected {}:\n{}",
            path.display(),
            String::from_utf8_lossy(&output.stderr),
        );
        checked += 1;
    }
    assert!(checked > 0, "no .go file was checked");
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
    let work = scratch("python");
    let doc = write_doc(&work, DOC);
    let (header, rows) = export(&doc);
    let expected = expected_entries(&rows);
    let events = literal_events(&expected);
    let out = work.join("out");
    generate("python", &out, &doc, true);

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
            index: e["index"].as_u64().unwrap(),
            annotation: e["annotation"].as_str().unwrap().to_string(),
        })
        .collect();
    assert_eq!(table, expected_spelled("python", &expected, &events));
}
