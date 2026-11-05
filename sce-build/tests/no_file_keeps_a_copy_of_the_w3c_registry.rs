// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! No file of the tree decides a question about the W3C conformance suite
//! from a list of test numbers of its own.
//!
//! The conformance registry (`tests/w3c/conformance/fixtures.json`) is where
//! the set of fixtures and the harness of each are decided. Until 2026-11,
//! four files answered "is this a BasicHTTP fixture" from a hand-kept list
//! instead: the C++ runner's command line, the pre-js of its WASM build, the
//! interactive verification test and the Python bindings' test. Each list
//! disagreed with the registry about one fixture, which the registry runs
//! under its `simple` harness and every list ran under the HTTP endpoint, and
//! the interactive test's list left out one the registry does run there.
//! Nothing failed: a list that is a little wrong builds and runs. Each now
//! asks the registry, or a file generated from it.
//!
//! What this refuses is the shape such a list takes: [`SHORTEST_COPY`] or
//! more registry ids in a row, separated by commas. The ids are read from the
//! registry, never spelled here, so a fixture registered tomorrow is held as
//! the ones registered today, and this file can hold no copy of its own.
//!
//! * **Quoted** (`"<id>", "<id>", ...` or with single quotes), in any file:
//!   a quoted id is a string a program compares against, whatever the file
//!   is written in.
//! * **Bare** (`<id>, <id>, ...`), in code only, with its comments removed
//!   (`//` and `/* */` in the C family, JavaScript, Rust, Go, Kotlin and
//!   Swift; `#` and docstrings in Python; `#` in CMake and the shell; `{# #}`
//!   and the emitted language's comments in a template). A comment that
//!   names the fixtures a rule was derived from is a citation, and the
//!   engine's sources cite fixtures that way throughout.
//! * **Prose and markup** (Markdown under `docs/`, XML and SCXML documents,
//!   plain text) are read for the quoted form only. A bare list of numbers
//!   there is a citation a reader reads, inside an XML comment or a sentence,
//!   not a set a program decides with; a program does not read Markdown.
//!
//! Allowed, each with the reason in its entry: the registry itself; the
//! committed files generated from it ([`DERIVED_OUTPUTS`], each held to
//! naming its generator, which a build reruns); the trees SCE did not write
//! or that hold the fixtures themselves ([`NOT_SCANNED`]); and a list of
//! other numbers that happens to be made of registry ids
//! ([`COINCIDENCES`], each held to still being there).
//!
//! The scan reads every file git tracks, so this suite is one of the
//! tree-wide gates (`UNFILTERABLE_GATES` in `workflow_trigger_coverage`):
//! the next copy can be written in a file that does not exist today.

mod common;

use std::collections::BTreeSet;
use std::sync::OnceLock;

use common::cmake_text::cmake_code;
use common::repository::{paths_git_tracks, root};
use common::source_lexing::{comments_blanked, language_of, Lang};
use sce_build::w3c_registry::{W3cRegistry, W3C_REGISTRY_RELATIVE_PATH};

/// How many registry ids in a row make a copy of a set. Two or three in a row
/// is how a sentence or a table names the fixtures one row came from; the
/// smallest harness the registry declares has more members than this.
const SHORTEST_COPY: usize = 4;

/// Trees the scan does not read, and why.
const NOT_SCANNED: &[(&str, &str)] = &[
    (
        "third_party/",
        "vendored code SCE did not write; its numbers are that project's",
    ),
    ("vendor/", "vendored code, as above"),
    (
        "resources/",
        "the W3C fixtures themselves: upstream documents and their metadata, where a \
         test number is the name of the document it is in",
    ),
    ("target/", "build output that happens to be tracked"),
    ("build/", "build output that happens to be tracked"),
    (
        "examples/doom_wasm/doomgeneric/",
        "a vendored port of DOOM under its own LICENSE: its lookup tables are numbers \
         SCE did not write, and runs of them fall inside the registry's range",
    ),
];

/// A committed file generated from the registry: a copy by construction, and
/// current because a build regenerates it.
struct DerivedOutput {
    path: &'static str,
    /// The program that writes it from the registry, which the file names.
    generator: &'static str,
    /// What reruns the generator when the registry changes.
    kept_current_by: &'static str,
}

const DERIVED_OUTPUTS: &[DerivedOutput] = &[DerivedOutput {
    path: "web/visualizer/test-list.js",
    generator: "tools/generate_test_list.py",
    kept_current_by: "the `generate_test_list` target of tests/w3c/W3CPrograms.cmake, built \
                      by default, whose command reruns the generator whenever the registry \
                      or the generator is newer than the file",
}];

/// The file that declares the build rerunning the derived outputs' generators.
const DERIVED_OUTPUTS_BUILD: &str = "tests/w3c/W3CPrograms.cmake";

/// A list of numbers that are not test numbers and happen to be registry ids.
/// Keyed on the line a run starts on, so the rest of the file stays read.
struct Coincidence {
    path: &'static str,
    line_contains: &'static str,
    why: &'static str,
}

const COINCIDENCES: &[Coincidence] = &[Coincidence {
    path: "tools/reference-oracles/davx5_sync.py",
    line_contains: "statuses = [",
    why: "the HTTP status codes a CalDAV client is answered with; several of them are \
          also W3C test numbers",
}];

/// One run of registry ids: where it starts and the ids it names.
#[derive(Debug)]
struct IdRun {
    line: usize,
    ids: Vec<String>,
}

/// How a file is read: as code with its comments removed, or as prose.
enum Reading {
    Code(String),
    Prose,
}

fn registry() -> W3cRegistry {
    W3cRegistry::load(&root().join(W3C_REGISTRY_RELATIVE_PATH)).expect("the registry loads")
}

fn registry_ids(registry: &W3cRegistry) -> BTreeSet<String> {
    registry.fixtures().iter().map(|f| f.id.clone()).collect()
}

/// `text` with every `#` comment blanked: from an unquoted `#` that opens the
/// line or follows whitespace to the end of the line. Line numbering is kept.
fn hash_comments_blanked(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for (index, line) in text.split('\n').enumerate() {
        if index > 0 {
            out.push('\n');
        }
        let mut quote: Option<char> = None;
        let mut previous = ' ';
        let mut cut = line.len();
        for (at, c) in line.char_indices() {
            match quote {
                Some(q) if c == q => quote = None,
                Some(_) => {}
                None if c == '"' || c == '\'' => quote = Some(c),
                None if c == '#' && previous.is_whitespace() => {
                    cut = at;
                    break;
                }
                None => {}
            }
            previous = c;
        }
        out.push_str(&line[..cut]);
    }
    out
}

/// How `path` is read for bare runs.
fn reading(path: &str, text: &str) -> Reading {
    let name = path.rsplit('/').next().unwrap_or(path);
    let ext = name.rsplit_once('.').map_or("", |(_, ext)| ext);
    if name == "CMakeLists.txt" || ext == "cmake" {
        return Reading::Code(cmake_code(text));
    }
    if matches!(ext, "sh" | "bash") {
        return Reading::Code(hash_comments_blanked(text));
    }
    // JavaScript comments are the C family's: `//` and `/* */`.
    if matches!(ext, "js" | "mjs" | "cjs" | "ts") {
        return Reading::Code(comments_blanked(text, Lang::CFamily));
    }
    match language_of(path) {
        Some(lang) => Reading::Code(comments_blanked(text, lang)),
        None => Reading::Prose,
    }
}

/// The runs of [`SHORTEST_COPY`] or more registry ids among `tokens` (each a
/// byte range of `text` and the value it names), where consecutive ids are
/// separated by one comma and whitespace.
fn runs<'a>(
    text: &'a str,
    tokens: impl Iterator<Item = (usize, usize, &'a str)>,
    ids: &BTreeSet<String>,
) -> Vec<IdRun> {
    let mut found = Vec::new();
    let mut streak: Vec<(usize, &'a str)> = Vec::new();
    let mut previous_end = 0usize;
    let close = |streak: &mut Vec<(usize, &'a str)>, found: &mut Vec<IdRun>| {
        if streak.len() >= SHORTEST_COPY {
            found.push(IdRun {
                line: text[..streak[0].0].matches('\n').count() + 1,
                ids: streak.iter().map(|(_, id)| id.to_string()).collect(),
            });
        }
        streak.clear();
    };
    for (start, end, value) in tokens {
        if ids.contains(value) {
            if text[previous_end..start].trim() != "," {
                close(&mut streak, &mut found);
            }
            streak.push((start, value));
        } else {
            close(&mut streak, &mut found);
        }
        previous_end = end;
    }
    close(&mut streak, &mut found);
    found
}

/// The runs of quoted registry ids in `text`.
fn quoted_runs(text: &str, ids: &BTreeSet<String>) -> Vec<IdRun> {
    static QUOTED: OnceLock<regex::Regex> = OnceLock::new();
    let quoted =
        QUOTED.get_or_init(|| regex::Regex::new(r#""([^"\n]*)"|'([^'\n]*)'"#).expect("pattern"));
    let tokens = quoted.captures_iter(text).map(|c| {
        let whole = c.get(0).expect("the whole match");
        let value = c.get(1).or_else(|| c.get(2)).map_or("", |m| m.as_str());
        (whole.start(), whole.end(), value)
    });
    runs(text, tokens, ids)
}

/// The runs of bare registry ids in `code`. A token is a whole word with its
/// dots, so a decimal fraction or a version is not read as an id.
fn bare_runs(code: &str, ids: &BTreeSet<String>) -> Vec<IdRun> {
    static WORD: OnceLock<regex::Regex> = OnceLock::new();
    let word = WORD.get_or_init(|| regex::Regex::new(r"[\w.]+").expect("pattern"));
    let tokens = word
        .find_iter(code)
        .map(|m| (m.start(), m.end(), m.as_str()));
    runs(code, tokens, ids)
}

/// Every run of registry ids in one file.
fn copies_in(path: &str, text: &str, ids: &BTreeSet<String>) -> Vec<IdRun> {
    let mut found = quoted_runs(text, ids);
    if let Reading::Code(code) = reading(path, text) {
        found.extend(bare_runs(&code, ids));
    }
    found.sort_by_key(|copy| copy.line);
    found
}

/// Whether `line` of `text` is where an allowed coincidence of `path` starts.
fn coincidence_at(path: &str, text: &str, line: usize) -> Option<&'static Coincidence> {
    let content = text.lines().nth(line - 1)?;
    COINCIDENCES
        .iter()
        .find(|c| c.path == path && content.contains(c.line_contains))
}

#[test]
fn no_file_keeps_a_copy_of_the_w3c_registry() {
    let registry = registry();
    let ids = registry_ids(&registry);
    assert!(
        ids.len() > SHORTEST_COPY,
        "the registry lists {} fixture(s); a run of {SHORTEST_COPY} could not be found",
        ids.len()
    );

    let mut scanned = 0usize;
    let mut found: Vec<String> = Vec::new();
    let mut used_coincidences: BTreeSet<&str> = BTreeSet::new();
    let mut used_outputs: BTreeSet<&str> = BTreeSet::new();
    for path in paths_git_tracks(&[]) {
        if path == W3C_REGISTRY_RELATIVE_PATH
            || NOT_SCANNED.iter().any(|(tree, _)| path.starts_with(tree))
        {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(root().join(&path)) else {
            continue; // a directory (a submodule), a deleted path, or not text
        };
        scanned += 1;
        let copies = copies_in(&path, &text, &ids);
        if copies.is_empty() {
            continue;
        }
        if let Some(output) = DERIVED_OUTPUTS.iter().find(|o| o.path == path) {
            used_outputs.insert(output.path);
            continue;
        }
        for copy in copies {
            if let Some(allowed) = coincidence_at(&path, &text, copy.line) {
                used_coincidences.insert(allowed.line_contains);
                continue;
            }
            found.push(format!(
                "{path}:{}: {} registry ids in a row ({} ... {})",
                copy.line,
                copy.ids.len(),
                copy.ids[0],
                copy.ids[copy.ids.len() - 1]
            ));
        }
    }

    assert!(
        scanned >= 1000,
        "only {scanned} tracked text file(s) reached the scan; the enumeration is broken, \
         and a clean result would prove nothing"
    );
    assert!(
        found.is_empty(),
        "a file keeps its own copy of fixtures the registry at {W3C_REGISTRY_RELATIVE_PATH} \
         decides:\n  {}\nAsk the registry instead: in C++ the generated W3CTestRegistry, in \
         CMake `sce_w3c_registry_fixtures`, elsewhere `sce-codegen list-fixtures` or a file \
         generated from the registry.",
        found.join("\n  ")
    );
    for coincidence in COINCIDENCES {
        assert!(
            used_coincidences.contains(coincidence.line_contains),
            "{} no longer holds the run allowed as `{}` ({}); drop the entry rather than \
             leave an allowance a copy could arrive under",
            coincidence.path,
            coincidence.line_contains,
            coincidence.why
        );
    }
    for output in DERIVED_OUTPUTS {
        assert!(
            used_outputs.contains(output.path),
            "{} holds no run of registry ids, so it is no longer the derived output this \
             entry allows; drop it",
            output.path
        );
    }
}

/// A derived output is allowed because it is generated, so it has to be:
/// the file names its generator and the registry, the generator exists, and
/// the build that reruns it is still declared.
#[test]
fn every_derived_output_names_its_generator_and_is_rebuilt() {
    let tree = cmake_code(
        &std::fs::read_to_string(root().join(DERIVED_OUTPUTS_BUILD))
            .unwrap_or_else(|e| panic!("read {DERIVED_OUTPUTS_BUILD}: {e}")),
    );
    for output in DERIVED_OUTPUTS {
        let text = std::fs::read_to_string(root().join(output.path))
            .unwrap_or_else(|e| panic!("read {}: {e}", output.path));
        assert!(
            text.contains(output.generator) && text.contains(W3C_REGISTRY_RELATIVE_PATH),
            "{} does not name its generator {} and the registry it is generated from, so \
             nothing shows it is not a list kept by hand",
            output.path,
            output.generator
        );
        assert!(
            root().join(output.generator).is_file(),
            "{}'s generator {} is gone",
            output.path,
            output.generator
        );
        assert!(
            tree.contains(output.generator) && tree.contains(output.path),
            "{} is said to be kept current by {}, and {DERIVED_OUTPUTS_BUILD} no longer runs \
             {} to write it",
            output.path,
            output.kept_current_by,
            output.generator
        );
    }
}

/// The shapes the scan exists for are refused, and the shapes it must leave
/// alone are not. Built from the registry's own ids, so this file names none.
#[test]
fn a_copy_is_refused_in_either_spelling_and_a_citation_is_not() {
    let registry = registry();
    let ids = registry_ids(&registry);
    let http: Vec<&str> = registry.ids_with_harness("http");
    assert!(
        http.len() > SHORTEST_COPY,
        "the registry's `http` harness has {} fixture(s), too few to build a run from",
        http.len()
    );
    let run = &http[..SHORTEST_COPY];
    let short = &http[..SHORTEST_COPY - 1];
    let quoted = |q: char, list: &[&str]| {
        list.iter()
            .map(|id| format!("{q}{id}{q}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let bare = |list: &[&str]| list.join(", ");

    let cpp = format!(
        "static const std::set<std::string> HTTP = {{{}}};\n",
        quoted('"', run)
    );
    assert_eq!(copies_in("probe.cpp", &cpp, &ids).len(), 1, "{cpp}");

    let js = format!(
        "// the BasicHTTP fixtures\nconst HTTP = new Set([{}]);\n",
        quoted('\'', run)
    );
    let found = copies_in("probe.js", &js, &ids);
    assert_eq!(found.len(), 1, "{js}");
    assert_eq!(found[0].line, 2, "the run was placed on the wrong line");

    let ints = format!(
        "const std::set<int> SKIP = {{\n    {},\n    {}}};\n",
        bare(&run[..2]),
        bare(&run[2..])
    );
    assert_eq!(
        copies_in("probe.cpp", &ints, &ids).len(),
        1,
        "a bare run broken across lines was accepted: {ints}"
    );

    let cited = format!(
        "// Event data field access (Tests: {})\nint x = 0;\n",
        bare(run)
    );
    assert!(
        copies_in("probe.cpp", &cited, &ids).is_empty(),
        "a citation in a comment was refused"
    );
    let py_cited = format!("\"\"\"The fixtures ({}) pass.\"\"\"\nx = 0\n", bare(run));
    assert!(
        copies_in("probe.py", &py_cited, &ids).is_empty(),
        "a citation in a docstring was refused"
    );
    let cmake_cited = format!("# fixtures {}\nset(X {})\n", bare(run), bare(short));
    assert!(
        copies_in("probe.cmake", &cmake_cited, &ids).is_empty(),
        "a CMake comment, or a run shorter than {SHORTEST_COPY}, was refused"
    );
    let decimals = format!(
        "double v[] = {{{}}};\n",
        run.iter()
            .map(|id| format!("1.{id}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    assert!(
        copies_in("probe.cpp", &decimals, &ids).is_empty(),
        "decimal fractions were read as ids: {decimals}"
    );

    let prose = format!(
        "The fixtures {} pass.\n\n```\nHTTP = {{{}}}\n```\n",
        bare(run),
        quoted('"', run)
    );
    let found = copies_in("docs/probe.md", &prose, &ids);
    assert_eq!(
        found.len(),
        1,
        "Markdown is read for the quoted form only: {found:?}"
    );
    assert_eq!(found[0].line, 4);
    let xml = format!("<!-- the fixtures {} pass -->\n<scxml/>\n", bare(run));
    assert!(
        copies_in("probe.scxml", &xml, &ids).is_empty(),
        "a citation in an XML comment was refused"
    );
}

#[test]
fn a_shell_comment_ends_at_an_unquoted_hash() {
    let text = "echo \"#1\" # tail\n# whole line\nx=a#b\n";
    assert_eq!(hash_comments_blanked(text), "echo \"#1\" \n\nx=a#b\n");
}
