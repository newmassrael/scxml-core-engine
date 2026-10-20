// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A document that declares `sce:transition-function="emit"` is generated with a
// pure function of (current state, event) beside its machine (SCE Accepted
// Subset 2.21, docs/SCE_TRANSITION_FUNCTION.md). The function is computed once,
// by `sce_build::model::TransitionFunction::of`, and this file holds the two
// things that depend on it to it, in each of the six backends:
//
// - the file `generate` writes, read back from its source on any host, and
//   compiled ALONE with the language's own toolchain and nothing else — which
//   is what proves it names no SCE library — then run, answering every
//   (state, event) pair, every configuration, every name and the hash exactly
//   as the producer's table;
// - the machine `generate` writes beside it, built against the backend's real
//   runtime and driven to every state of the function by the shortest event
//   path the function itself gives, and sent every event there: the
//   configuration the engine reaches, and whether it took or discarded the
//   event, is what the function says. That is what makes the selection the
//   generator does at build time and the one the engine does at run time one
//   fact rather than two that happen to agree. The two fixtures are
//   integration stems (`integration_resources/<stem>/`), so the C++ AOT engine
//   and the Interpreter, which this file cannot build, are driven the same way
//   by their drivers under `tests/integration/`, as every other channel is by
//   its own.
//
// The compiling checks run where a toolchain is found; the rest run anywhere.
// The declaration, its one value, the forge root that does not take it, and
// the refusals it brings are held at the command line.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use regex::Regex;
use sce_build::filters;
use sce_build::forge::diagnostic::ToDiagnostics;
use sce_build::generator::Language;
use sce_build::model::{EmittedTransitionFunction, TransitionFunction};
use sce_build::parser::SCXMLParser;
use sce_build::toolchain;

/// The two fixtures the function is held on: a flat cycle, and a nested
/// lifecycle that reaches every selection rule.
const FIXTURES: [&str; 2] = ["flat_lifecycle", "nested_lifecycle"];

/// Every backend, by the identifier `generate -l` takes.
fn backends() -> Vec<&'static str> {
    Language::ALL
        .iter()
        .map(|language| language.canonical_name())
        .collect()
}

/// The template each backend renders the function with, under
/// `tools/codegen/templates/`.
const TEMPLATES: [&str; 6] = [
    "rust/transition_function.rs.jinja2",
    "transition_function.jinja2",
    "c/transition_function.h.jinja2",
    "go/transition_function.go.jinja2",
    "kotlin/transition_function.kt.jinja2",
    "python/transition_function.py.jinja2",
];

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent dir")
        .to_path_buf()
}

/// The canonical document of `stem`: an integration stem, so the same
/// document every channel's driver holds its engine to.
fn fixture(stem: &str) -> PathBuf {
    repo_root()
        .join("integration_resources")
        .join(stem)
        .join(format!("{stem}.scxml"))
}

/// A scratch directory of its own for `name`, emptied first.
fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("emitted_transition_function")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch");
    dir
}

fn codegen(args: &[&str]) -> Output {
    Command::new(sce_codegen_bin())
        .env("SCE_WORKSPACE_ROOT", repo_root())
        .args(args)
        .output()
        .expect("spawn sce-codegen")
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

/// Run `command`, failing with everything it printed unless it succeeded.
fn run_ok(mut command: Command, what: &str) -> String {
    let output = command
        .output()
        .unwrap_or_else(|e| panic!("run {what}: {e}"));
    output_ok(output, what)
}

/// A C, C++, Go or Kotlin string literal of `text`, written here and not by
/// the templates. The ids and names it is given are XML names and event
/// names, so it never meets a `$` Kotlin would read as a template.
fn c_literal(text: &str) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\t', "\\t");
    format!("\"{escaped}\"")
}

/// `generate -l <lang>` of `doc` into `out`.
fn generate(lang: &str, doc: &Path, out: &Path) {
    output_ok(
        codegen(&[
            "generate",
            "-l",
            lang,
            "-o",
            out.to_str().unwrap(),
            doc.to_str().unwrap(),
        ]),
        &format!("generate -l {lang}"),
    );
}

/// The file `generate -l <lang>` writes the machine of `stem` to.
fn machine_file(lang: &str, out: &Path, stem: &str) -> PathBuf {
    out.join(match lang {
        "rust" => format!("{stem}_sm.rs"),
        "cpp" | "c11" => format!("{stem}_sm.h"),
        "go" => format!("{stem}_sm.go"),
        "kotlin" => format!("{stem}Sm.kt"),
        "python" => format!("{stem}_sm.py"),
        other => panic!("no machine file for {other}"),
    })
}

/// The Go package the function of `stem` is generated as: the machine's
/// package name, as the Go filters spell it, and `_transition_function`.
fn go_package(stem: &str) -> String {
    format!(
        "{}_transition_function",
        filters::to_snake_case(stem.to_string())
    )
}

/// The file `generate -l <lang>` writes the function of `stem` to, as
/// `docs/SCE_TRANSITION_FUNCTION.md` names it.
fn function_file(lang: &str, out: &Path, stem: &str) -> PathBuf {
    match lang {
        "rust" => out.join(format!("{stem}_transition_function.rs")),
        "cpp" | "c11" => out.join(format!("{stem}_transition_function.h")),
        "go" => out
            .join(go_package(stem))
            .join(format!("{stem}_transition_function.go")),
        "kotlin" => out.join(format!("{stem}TransitionFunction.kt")),
        "python" => out.join(format!("{stem}_transition_function.py")),
        other => panic!("no function file for {other}"),
    }
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The `code` of every record on a `--error-format=json` stderr, in order.
fn wire_codes(stderr: &str) -> Vec<String> {
    stderr
        .lines()
        .filter(|line| line.starts_with('{'))
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|record| record["code"].as_str().map(str::to_string))
        .collect()
}

/// The `source-hash` the header of `source` carries.
fn header_hash(source: &str) -> String {
    Regex::new(r"source-hash:\s*([0-9a-f]{64})")
        .unwrap()
        .captures(source)
        .unwrap_or_else(|| panic!("no source-hash header in:\n{source}"))[1]
        .to_string()
}

/// The producer's function of the fixture, as the library hands it out
/// (`sce_build::transition_function_from_string`): the value a program that
/// renders the function itself reads, held here to every generated one.
fn produced(doc: &Path) -> TransitionFunction {
    let label = doc.to_str().unwrap();
    sce_build::transition_function_from_string(&read(doc), label)
        .unwrap_or_else(|refused| panic!("{label}: the library refuses the fixture: {refused:?}"))
}

/// What the generated files of a run that read the fixture carry: the
/// producer's function and that run's digest, `hash`, bound the way every
/// route binds them (`sce_build::bind_run_digest`).
fn emitted(doc: &Path, hash: &str) -> EmittedTransitionFunction {
    let mut model = SCXMLParser::new()
        .parse_file(doc.to_str().unwrap())
        .expect("the fixture parses");
    sce_build::analyzer::analyze(&mut model, doc.to_str().unwrap());
    sce_build::bind_run_digest(&mut model, hash);
    let emitted = model
        .emitted_transition_function
        .expect("the fixture declares the function");
    assert_eq!(
        emitted.function,
        produced(doc),
        "the function a run binds is not the one the library hands out"
    );
    emitted
}

/// What the generated function must print, from the producer's value: the
/// same lines the check program below writes from the compiled one.
fn listing(emitted: &EmittedTransitionFunction) -> String {
    let function = &emitted.function;
    let mut out = String::new();
    out.push_str(&format!("hash\t{}\n", emitted.source_hash));
    out.push_str(&format!("initial\t{}\n", function.initial));
    for state in &function.states {
        out.push_str(&format!(
            "state\t{}\t{}\t{}\n",
            state.id,
            state.is_final,
            state.configuration.join(" ")
        ));
    }
    for event in &function.events {
        out.push_str(&format!("event\t{event}\n"));
    }
    for state in &function.states {
        for cell in &state.next {
            out.push_str(&format!(
                "next\t{}\t{}\t{}\n",
                state.id,
                cell.event,
                cell.to.as_deref().unwrap_or("-")
            ));
        }
    }
    out
}

/// A program that prints the compiled function in [`listing`]'s shape, and
/// checks along the way that every name reads back as the value it names.
fn listing_program(function_file: &Path) -> String {
    format!(
        r#"#[path = {path:?}]
mod tf;

fn main() {{
    println!("hash\t{{}}", tf::SOURCE_HASH);
    println!("initial\t{{}}", tf::INITIAL.as_str());
    for state in tf::ALL_STATES {{
        assert_eq!(tf::State::from_name(state.as_str()), Some(state));
        println!(
            "state\t{{}}\t{{}}\t{{}}",
            state.as_str(),
            tf::is_final(state),
            tf::configuration(state).join(" ")
        );
    }}
    for event in tf::ALL_EVENTS {{
        assert_eq!(tf::Event::from_name(event.as_str()), Some(event));
        println!("event\t{{}}", event.as_str());
    }}
    for state in tf::ALL_STATES {{
        for event in tf::ALL_EVENTS {{
            println!(
                "next\t{{}}\t{{}}\t{{}}",
                state.as_str(),
                event.as_str(),
                tf::transition(state, event).map_or("-", tf::State::as_str)
            );
        }}
    }}
    assert_eq!(tf::State::from_name(""), None);
    assert_eq!(tf::Event::from_name(""), None);
    assert_eq!(tf::State::default(), tf::INITIAL);
}}
"#,
        path = function_file.to_str().unwrap()
    )
}

/// Both fixtures are accepted and generated in every backend: the machine, and
/// the function beside it under the name the reference doc gives it, carrying
/// as a value the hash of its own header, which is the machine's.
#[test]
fn every_fixture_generates_its_machine_and_its_function_in_every_backend() {
    // The constant each backend names the hash by (`SOURCE_HASH`, Go's
    // `SourceHash`) and the literal after it, across a line break if a
    // formatter put one there.
    let carried = Regex::new(r#"(?:SOURCE_HASH|SourceHash)[^"]*"([0-9a-f]{64})""#).unwrap();
    for lang in backends() {
        for stem in FIXTURES {
            let out = scratch(&format!("generated_{lang}_{stem}"));
            generate(lang, &fixture(stem), &out);
            let machine = machine_file(lang, &out, stem);
            assert!(machine.is_file(), "[{lang} {stem}] no machine was written");
            let function = function_file(lang, &out, stem);
            assert!(
                function.is_file(),
                "[{lang} {stem}] no function was written at {}",
                function.display()
            );
            let source = read(&function);
            let hash = header_hash(&source);
            assert_eq!(
                carried.captures(&source).map(|c| c[1].to_string()),
                Some(hash.clone()),
                "[{lang} {stem}] the function does not carry the hash of its own header"
            );
            assert_eq!(
                header_hash(&read(&machine)),
                hash,
                "[{lang} {stem}] the machine and the function name different source sets"
            );
        }
    }
}

/// One taken pair of a function as a source spells it: the state, the event,
/// and the state the event takes the machine to.
type Cell = (String, String, String);

/// The identifiers a backend names a state and an event by, from their ids,
/// using the filters its template applies.
#[derive(Clone, Copy)]
struct Names {
    state: fn(&str) -> String,
    event: fn(&str) -> String,
}

fn names_of(lang: &str) -> Names {
    match lang {
        "rust" | "go" => Names {
            state: |s| filters::to_state_variant(s.to_string()),
            event: |e| filters::to_event_variant(e.to_string()),
        },
        "cpp" => Names {
            state: |s| filters::capitalize_state(s.to_string()),
            event: |e| filters::to_cpp_event_variant(e.into()),
        },
        "c11" => Names {
            state: |s| filters::to_c11_upper_ident(s.into()),
            event: |e| filters::to_c11_upper_ident(e.into()),
        },
        "kotlin" => Names {
            state: |s| filters::to_state_class_name(s.to_string()),
            event: |e| filters::to_event_variant(e.to_string()),
        },
        "python" => Names {
            state: |s| filters::to_python_const(s.to_string()),
            event: |e| filters::to_python_const(e.to_string()),
        },
        other => panic!("no names for {other}"),
    }
}

/// The prefix every symbol of the C function of `stem` carries, upper case.
fn c_prefix_upper(stem: &str) -> String {
    format!("{}_TRANSITION_FUNCTION", stem.to_uppercase())
}

/// The taken pairs the producer's table holds, spelled as `lang` spells them.
fn expected_cells(lang: &str, function: &TransitionFunction) -> Vec<Cell> {
    let names = names_of(lang);
    let mut cells: Vec<Cell> = function
        .states
        .iter()
        .flat_map(|state| {
            state.next.iter().filter_map(move |cell| {
                cell.to.as_ref().map(|to| {
                    (
                        (names.state)(&state.id),
                        (names.event)(&cell.event),
                        (names.state)(to),
                    )
                })
            })
        })
        .collect();
    cells.sort();
    cells
}

/// The taken pairs the generated source of `lang` holds, read back from its
/// text. Rust and Python write one entry per pair; the others write one block
/// per state (a `switch` or a `when` on the event), so a state's block is
/// found first and its entries in it.
fn written_cells(lang: &str, source: &str, stem: &str) -> Vec<Cell> {
    let flat = |pattern: &str| -> Vec<Cell> {
        Regex::new(pattern)
            .unwrap()
            .captures_iter(source)
            .map(|c| (c[1].to_string(), c[2].to_string(), c[3].to_string()))
            .collect()
    };
    let nested = |block: &str, entry: &str| -> Vec<Cell> {
        let entry = Regex::new(entry).unwrap();
        let mut cells = Vec::new();
        for row in Regex::new(block).unwrap().captures_iter(source) {
            for cell in entry.captures_iter(&row[2]) {
                cells.push((row[1].to_string(), cell[1].to_string(), cell[2].to_string()));
            }
        }
        cells
    };
    let mut cells = match lang {
        "rust" => flat(r"\(State::(\w+), Event::(\w+)\) => Some\(State::(\w+)\),"),
        "python" => flat(r"\(State\.(\w+), Event\.(\w+)\): State\.(\w+),"),
        "cpp" => nested(
            r"case State::(\w+):\s*switch \(event\) \{([^{}]*)\}",
            r"case Event::(\w+):\s*return State::(\w+);",
        ),
        "c11" => {
            let p = c_prefix_upper(stem);
            nested(
                &format!(r"case {p}_STATE_(\w+):\s*switch \(event\) \{{([^{{}}]*)\}}"),
                &format!(r"case {p}_EVENT_(\w+):\s*\*next = {p}_STATE_(\w+);\s*return true;"),
            )
        }
        "go" => nested(
            r"case State(\w+):\s*switch event \{([^{}]*)\}",
            r"case Event(\w+):\s*return State(\w+), true",
        ),
        "kotlin" => nested(
            r"State\.(\w+) -> when \(event\) \{([^{}]*)\}",
            r"Event\.(\w+) -> State\.(\w+)",
        ),
        other => panic!("no reader for {other}"),
    };
    cells.sort();
    cells
}

/// The table every backend's generated source holds is the producer's, pair
/// for pair, read back from the text — so this runs on a host with no
/// toolchain.
#[test]
fn the_generated_source_holds_the_producers_table() {
    for lang in backends() {
        for stem in FIXTURES {
            let out = scratch(&format!("source_{lang}_{stem}"));
            generate(lang, &fixture(stem), &out);
            let source = read(&function_file(lang, &out, stem));
            let function = produced(&fixture(stem));
            let expected = expected_cells(lang, &function);
            assert!(!expected.is_empty(), "[{stem}] the producer takes no pair");
            assert_eq!(
                written_cells(lang, &source, stem),
                expected,
                "[{lang} {stem}] the generated table is not the producer's"
            );
            if lang == "rust" {
                assert_eq!(
                    source.contains("        _ => None,\n    }\n}"),
                    !function.every_pair_taken,
                    "[{stem}] the refusing arm is there exactly when some pair is refused"
                );
            }
        }
    }
}

/// Every function the Rust file offers returns its answer and does nothing
/// else, so a call whose answer is dropped is a mistake the compiler should
/// name: each `pub fn` carries `#[must_use]`, which is also what keeps the file
/// clean under `clippy::pedantic` (`must_use_candidate`) in a crate that denies
/// it.
#[test]
fn every_rust_function_asks_that_its_answer_be_used() {
    let function_line = Regex::new(r"^\s*pub (const )?fn ").unwrap();
    for stem in FIXTURES {
        let out = scratch(&format!("must_use_{stem}"));
        generate("rust", &fixture(stem), &out);
        let source = read(&function_file("rust", &out, stem));
        let lines: Vec<&str> = source.lines().collect();
        let functions: Vec<usize> = (0..lines.len())
            .filter(|&i| function_line.is_match(lines[i]))
            .collect();
        assert_eq!(functions.len(), 7, "[{stem}] the Rust file's functions");
        for i in functions {
            assert_eq!(
                lines[i - 1].trim(),
                "#[must_use]",
                "[{stem}] `{}` does not ask that its answer be used",
                lines[i].trim()
            );
        }
    }
}

/// The nested fixture reaches every selection rule the producer's unit tests
/// name, so the table it yields is stated here in full: a change to the
/// producer that moves one pair moves this.
#[test]
fn the_nested_fixture_yields_the_table_its_rules_give() {
    let function = produced(&fixture("nested_lifecycle"));
    assert_eq!(function.initial, "loading");
    assert_eq!(
        function.events,
        [
            "abort",
            "pause",
            "reset",
            "done",
            "resume",
            "data.ready",
            "data.failed",
            "go"
        ]
    );
    let states: Vec<&str> = function.states.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(
        states,
        ["working", "held", "retrying", "loading", "ready", "closed"]
    );
    let row = |state: &str| -> Vec<Option<String>> {
        function
            .state(state)
            .unwrap_or_else(|| panic!("{state} is a state of the function"))
            .next
            .iter()
            .map(|cell| cell.to.clone())
            .collect()
    };
    // One cell per event, `-` where the event is refused. Columns: abort,
    // pause, reset, done, resume, data.ready, data.failed, go.
    let expect = |cells: [&str; 8]| -> Vec<Option<String>> {
        cells
            .iter()
            .map(|cell| (*cell != "-").then(|| cell.to_string()))
            .collect()
    };
    let table = [
        (
            "working",
            [
                "closed", "held", "working", "closed", "working", "-", "-", "-",
            ],
        ),
        (
            "held",
            [
                "held", "held", "held", "held", "working", "held", "held", "held",
            ],
        ),
        (
            "retrying",
            ["closed", "-", "-", "-", "-", "ready", "loading", "-"],
        ),
        (
            "loading",
            ["closed", "-", "-", "-", "-", "ready", "retrying", "-"],
        ),
        ("ready", ["closed", "-", "-", "-", "-", "-", "-", "working"]),
        ("closed", ["-", "-", "-", "-", "-", "-", "-", "-"]),
    ];
    for (state, cells) in table {
        assert_eq!(row(state), expect(cells), "{state}");
    }
}

/// The generated function compiles ALONE: a `#![no_std]` library holding the
/// module and nothing else, under `-D warnings`. Then a program holding the
/// same module prints every answer it gives, and that is the producer's table.
#[test]
fn the_generated_function_compiles_alone_and_answers_as_the_producer() {
    let Some(rustc) = toolchain::locate("rustc") else {
        toolchain::skipped(
            "the_generated_function_compiles_alone_and_answers_as_the_producer: rustc not on PATH",
        );
        return;
    };
    for stem in FIXTURES {
        let work = scratch(&format!("alone_{stem}"));
        let out = work.join("out");
        generate("rust", &fixture(stem), &out);
        let function_path = function_file("rust", &out, stem);
        let source = read(&function_path);

        // No crate but `core`: a `#![no_std]` library with no `--extern`.
        let library = work.join("alone.rs");
        std::fs::write(
            &library,
            format!(
                "#![no_std]\n#[path = {:?}]\npub mod tf;\n",
                function_path.to_str().unwrap()
            ),
        )
        .expect("write the library");
        let mut compile = Command::new(&rustc);
        compile
            .args([
                "--edition",
                "2021",
                "--crate-type",
                "rlib",
                "--crate-name",
                "alone",
            ])
            .args(["-D", "warnings", "--out-dir"])
            .arg(&work)
            .arg(&library);
        output_ok(
            compile.output().expect("run rustc"),
            &format!("[{stem}] rustc over the function alone, no_std"),
        );

        let program = work.join("listing.rs");
        std::fs::write(&program, listing_program(&function_path)).expect("write the program");
        let binary = work.join("listing");
        let mut build = Command::new(&rustc);
        build
            .args(["--edition", "2021", "-o"])
            .arg(&binary)
            .arg(&program);
        output_ok(
            build.output().expect("run rustc"),
            &format!("[{stem}] rustc over the listing program"),
        );
        let printed = output_ok(
            Command::new(&binary).output().expect("run the listing"),
            &format!("[{stem}] the listing program"),
        );
        let function = emitted(&fixture(stem), &header_hash(&source));
        assert_eq!(
            printed,
            listing(&function),
            "[{stem}] the compiled function does not answer as the producer"
        );
    }
}

/// The file is committed into crates that deny `clippy::pedantic`, so it is
/// held to that lint group itself: clippy over the function alone, every
/// pedantic lint an error. A consumer should not need an `allow` of its own to
/// keep a generated file.
#[test]
fn the_generated_rust_function_is_clean_under_clippy_pedantic() {
    let Some(clippy) = toolchain::locate("clippy-driver") else {
        toolchain::skipped(
            "the_generated_rust_function_is_clean_under_clippy_pedantic: clippy-driver not on PATH",
        );
        return;
    };
    for stem in FIXTURES {
        let work = scratch(&format!("pedantic_{stem}"));
        let out = work.join("out");
        generate("rust", &fixture(stem), &out);
        let function_path = function_file("rust", &out, stem);
        let library = work.join("pedantic.rs");
        std::fs::write(
            &library,
            format!(
                "#![no_std]\n#[path = {:?}]\npub mod tf;\n",
                function_path.to_str().unwrap()
            ),
        )
        .expect("write the library");
        let mut lint = Command::new(&clippy);
        lint.args([
            "--edition",
            "2021",
            "--crate-type",
            "rlib",
            "--crate-name",
            "pedantic",
        ])
        .args(["-D", "warnings", "-D", "clippy::pedantic", "--out-dir"])
        .arg(&work)
        .arg(&library);
        output_ok(
            lint.output().expect("run clippy-driver"),
            &format!("[{stem}] clippy::pedantic over the function alone"),
        );
    }
}

// ── The other five backends, compiled alone and run ─────────────────────────
//
// Each program below is written here, in the language's own syntax, and not by
// a template: it prints the compiled function in `listing`'s shape, so the
// comparison is with the producer's value and nothing the templates wrote.
// "Alone" means what each toolchain can be asked for: the file and the
// standard library, with no SCE include path, module, jar or runtime given.

/// A C++ translation unit that includes the generated header and nothing else,
/// and has the compiler check every answer the producer's table gives as a
/// constant expression.
fn cpp_constant_checks(header: &Path, stem: &str, emitted: &EmittedTransitionFunction) -> String {
    let function = &emitted.function;
    let names = names_of("cpp");
    let state = |id: &str| format!("tf::State::{}", (names.state)(id));
    let event = |name: &str| format!("tf::Event::{}", (names.event)(name));
    let mut unit = format!(
        "#include {}\n\nnamespace tf = SCE::Generated::{stem}::transition_function;\n\n",
        c_literal(header.to_str().unwrap())
    );
    let mut check = |condition: String| unit.push_str(&format!("static_assert({condition});\n"));
    check(format!(
        "tf::SOURCE_HASH == {}",
        c_literal(&emitted.source_hash)
    ));
    check(format!("tf::INITIAL == {}", state(&function.initial)));
    check(format!(
        "tf::ALL_STATES.size() == {}",
        function.states.len()
    ));
    check(format!(
        "tf::ALL_EVENTS.size() == {}",
        function.events.len()
    ));
    for (index, row) in function.states.iter().enumerate() {
        let s = state(&row.id);
        check(format!("tf::ALL_STATES[{index}] == {s}"));
        check(format!("tf::getStateName({s}) == {}", c_literal(&row.id)));
        check(format!(
            "tf::getStateFromName({}) == {s}",
            c_literal(&row.id)
        ));
        check(format!(
            "tf::isFinal({s}) == {}",
            if row.is_final { "true" } else { "false" }
        ));
        check(format!(
            "tf::configuration({s}).size() == {}",
            row.configuration.len()
        ));
        for (depth, id) in row.configuration.iter().enumerate() {
            check(format!(
                "tf::configuration({s})[{depth}] == {}",
                c_literal(id)
            ));
        }
        for cell in &row.next {
            let e = event(&cell.event);
            match &cell.to {
                Some(to) => check(format!("tf::transition({s}, {e}) == {}", state(to))),
                None => check(format!("!tf::transition({s}, {e}).has_value()")),
            }
        }
    }
    for (index, name) in function.events.iter().enumerate() {
        let e = event(name);
        check(format!("tf::ALL_EVENTS[{index}] == {e}"));
        check(format!("tf::getEventName({e}) == {}", c_literal(name)));
        check(format!("tf::getEventFromName({}) == {e}", c_literal(name)));
    }
    check("!tf::getStateFromName(\"\").has_value()".to_string());
    check("!tf::getEventFromName(\"\").has_value()".to_string());
    unit.push_str("\nint main() {\n    return 0;\n}\n");
    unit
}

/// A C++ program that prints the compiled function in `listing`'s shape.
const CPP_LISTING: &str = r#"#include @HEADER@

#include <iostream>
#include <string>
#include <string_view>

namespace tf = SCE::Generated::@STEM@::transition_function;

int main() {
    std::cout << "hash\t" << tf::SOURCE_HASH << "\n";
    std::cout << "initial\t" << tf::getStateName(tf::INITIAL) << "\n";
    for (tf::State state : tf::ALL_STATES) {
        if (tf::getStateFromName(tf::getStateName(state)) != state) {
            return 2;
        }
        std::string configuration;
        for (std::string_view id : tf::configuration(state)) {
            if (!configuration.empty()) {
                configuration.push_back(' ');
            }
            configuration.append(id.data(), id.size());
        }
        std::cout << "state\t" << tf::getStateName(state) << "\t" << (tf::isFinal(state) ? "true" : "false")
                  << "\t" << configuration << "\n";
    }
    for (tf::Event event : tf::ALL_EVENTS) {
        if (tf::getEventFromName(tf::getEventName(event)) != event) {
            return 2;
        }
        std::cout << "event\t" << tf::getEventName(event) << "\n";
    }
    for (tf::State state : tf::ALL_STATES) {
        for (tf::Event event : tf::ALL_EVENTS) {
            const auto next = tf::transition(state, event);
            std::cout << "next\t" << tf::getStateName(state) << "\t" << tf::getEventName(event) << "\t"
                      << (next ? tf::getStateName(*next) : std::string_view("-")) << "\n";
        }
    }
    return 0;
}
"#;

/// The generated C++ compiles alone — a translation unit that includes the
/// header and nothing else, under `-std=c++17 -Werror` with no include path —
/// and answers as the producer, both as constant expressions the compiler
/// checks and as a program that prints every answer. It also compiles in one
/// translation unit with the machine generated beside it.
#[test]
fn the_cpp_function_compiles_alone_and_answers_as_the_producer() {
    let Some(gpp) = toolchain::locate("g++") else {
        toolchain::skipped(
            "the_cpp_function_compiles_alone_and_answers_as_the_producer: g++ not on PATH",
        );
        return;
    };
    for stem in FIXTURES {
        let work = scratch(&format!("cpp_alone_{stem}"));
        let out = work.join("out");
        generate("cpp", &fixture(stem), &out);
        let header = function_file("cpp", &out, stem);
        let function = emitted(&fixture(stem), &header_hash(&read(&header)));

        let alone = work.join("alone.cpp");
        std::fs::write(&alone, cpp_constant_checks(&header, stem, &function))
            .expect("write the unit");
        let mut compile = Command::new(&gpp);
        compile
            .args([
                "-std=c++17",
                "-Wall",
                "-Wextra",
                "-Wpedantic",
                "-Werror",
                "-fsyntax-only",
            ])
            .arg(&alone);
        run_ok(compile, &format!("[{stem}] g++ over the function alone"));

        let program = work.join("listing.cpp");
        std::fs::write(
            &program,
            CPP_LISTING
                .replace("@HEADER@", &c_literal(header.to_str().unwrap()))
                .replace("@STEM@", stem),
        )
        .expect("write the program");
        let binary = work.join("listing");
        let mut build = Command::new(&gpp);
        build.args(["-std=c++17", "-o"]).arg(&binary).arg(&program);
        run_ok(build, &format!("[{stem}] g++ over the listing program"));
        assert_eq!(
            run_ok(
                Command::new(&binary),
                &format!("[{stem}] the listing program")
            ),
            listing(&function),
            "[{stem}] the compiled C++ function does not answer as the producer"
        );

        // One translation unit with the machine: the two headers declare
        // nothing in common.
        let both = work.join("both.cpp");
        std::fs::write(
            &both,
            format!(
                "#include \"{stem}_sm.h\"\n#include \"{stem}_transition_function.h\"\n\nint main() {{\n    return 0;\n}}\n"
            ),
        )
        .expect("write the unit");
        let mut together = Command::new(&gpp);
        together.args(["-std=c++20", "-fsyntax-only"]);
        for include in [
            repo_root().join("sce/include"),
            repo_root().join("third_party/spdlog/include"),
            repo_root().join("third_party/nlohmann_json/include"),
            out,
        ] {
            together.arg("-I").arg(include);
        }
        together.arg(&both);
        run_ok(
            together,
            &format!("[{stem}] g++ over the machine and the function in one unit"),
        );
    }
}

/// A C program that prints the compiled function in `listing`'s shape. `@p@`
/// is the function's symbol prefix and `@P@` the same in upper case.
const C_LISTING: &str = r#"#include @HEADER@

#include <stdio.h>

int main(void) {
    const @p@_state_t *states = @p@_all_states();
    const @p@_event_t *events = @p@_all_events();
    printf("hash\t%s\n", @P@_SOURCE_HASH);
    printf("initial\t%s\n", @p@_state_name(@P@_INITIAL));
    for (size_t i = 0; i < (size_t)@P@_NUM_STATES; ++i) {
        @p@_state_t back;
        if (!@p@_state_from_name(@p@_state_name(states[i]), &back) || back != states[i]) {
            return 2;
        }
        @p@_configuration_t configuration = @p@_configuration(states[i]);
        printf("state\t%s\t%s\t", @p@_state_name(states[i]), @p@_is_final(states[i]) ? "true" : "false");
        for (size_t k = 0; k < configuration.count; ++k) {
            if (k > 0) {
                printf(" ");
            }
            printf("%s", configuration.ids[k]);
        }
        printf("\n");
    }
    for (size_t j = 0; j < (size_t)@P@_NUM_EVENTS; ++j) {
        @p@_event_t back;
        if (!@p@_event_from_name(@p@_event_name(events[j]), &back) || back != events[j]) {
            return 2;
        }
        printf("event\t%s\n", @p@_event_name(events[j]));
    }
    for (size_t i = 0; i < (size_t)@P@_NUM_STATES; ++i) {
        for (size_t j = 0; j < (size_t)@P@_NUM_EVENTS; ++j) {
            @p@_state_t next = states[i];
            const char *to = @p@_transition(states[i], events[j], &next) ? @p@_state_name(next) : "-";
            printf("next\t%s\t%s\t%s\n", @p@_state_name(states[i]), @p@_event_name(events[j]), to);
        }
    }
    {
        @p@_state_t state;
        @p@_event_t event;
        if (@p@_state_from_name("", &state) || @p@_event_from_name("", &event)) {
            return 2;
        }
    }
    return 0;
}
"#;

/// The generated C compiles alone — a translation unit that includes the
/// header and nothing else, under `-std=c11 -Wpedantic -Werror` with no
/// include path — and a program over it prints every answer as the producer.
#[test]
fn the_c_function_compiles_alone_and_answers_as_the_producer() {
    let Some(gcc) = toolchain::locate_any(&["gcc", "cc"]) else {
        toolchain::skipped(
            "the_c_function_compiles_alone_and_answers_as_the_producer: gcc/cc not on PATH",
        );
        return;
    };
    for stem in FIXTURES {
        let work = scratch(&format!("c_alone_{stem}"));
        let out = work.join("out");
        generate("c11", &fixture(stem), &out);
        let header = function_file("c11", &out, stem);
        let function = emitted(&fixture(stem), &header_hash(&read(&header)));
        let include = c_literal(header.to_str().unwrap());

        let alone = work.join("alone.c");
        std::fs::write(
            &alone,
            format!("#include {include}\n\nint main(void) {{\n    return 0;\n}}\n"),
        )
        .expect("write the unit");
        let mut compile = Command::new(&gcc);
        compile
            .args([
                "-std=c11",
                "-Wall",
                "-Wextra",
                "-Wpedantic",
                "-Werror",
                "-fsyntax-only",
            ])
            .arg(&alone);
        run_ok(compile, &format!("[{stem}] cc over the function alone"));

        let prefix = format!("{stem}_transition_function");
        let program = work.join("listing.c");
        std::fs::write(
            &program,
            C_LISTING
                .replace("@HEADER@", &include)
                .replace("@p@", &prefix)
                .replace("@P@", &c_prefix_upper(stem)),
        )
        .expect("write the program");
        let binary = work.join("listing");
        let mut build = Command::new(&gcc);
        build.args(["-std=c11", "-o"]).arg(&binary).arg(&program);
        run_ok(build, &format!("[{stem}] cc over the listing program"));
        assert_eq!(
            run_ok(
                Command::new(&binary),
                &format!("[{stem}] the listing program")
            ),
            listing(&function),
            "[{stem}] the compiled C function does not answer as the producer"
        );
    }
}

/// A Go program that prints the compiled function in `listing`'s shape.
const GO_LISTING: &str = r#"package main

import (
	"fmt"
	"os"
	"strings"

	tf "example.com/tfalone/@PACKAGE@"
)

func main() {
	fmt.Printf("hash\t%s\n", tf.SourceHash)
	fmt.Printf("initial\t%s\n", tf.Initial.String())
	for _, state := range tf.AllStates {
		if back, ok := tf.ParseState(state.String()); !ok || back != state {
			os.Exit(2)
		}
		fmt.Printf("state\t%s\t%t\t%s\n", state.String(), tf.IsFinal(state), strings.Join(tf.Configuration(state), " "))
	}
	for _, event := range tf.AllEvents {
		if back, ok := tf.ParseEvent(event.String()); !ok || back != event {
			os.Exit(2)
		}
		fmt.Printf("event\t%s\n", event.String())
	}
	for _, state := range tf.AllStates {
		for _, event := range tf.AllEvents {
			next := "-"
			if to, ok := tf.Transition(state, event); ok {
				next = to.String()
			}
			fmt.Printf("next\t%s\t%s\t%s\n", state.String(), event.String(), next)
		}
	}
	if _, ok := tf.ParseState(""); ok {
		os.Exit(2)
	}
	if _, ok := tf.ParseEvent(""); ok {
		os.Exit(2)
	}
}
"#;

/// The generated Go package builds alone — copied into a module that requires
/// nothing — and a program over it prints every answer as the producer.
#[test]
fn the_go_function_compiles_alone_and_answers_as_the_producer() {
    let Some(go) = toolchain::locate("go") else {
        toolchain::skipped(
            "the_go_function_compiles_alone_and_answers_as_the_producer: go not on PATH",
        );
        return;
    };
    for stem in FIXTURES {
        let work = scratch(&format!("go_alone_{stem}"));
        let out = work.join("out");
        generate("go", &fixture(stem), &out);
        let source = function_file("go", &out, stem);
        let function = emitted(&fixture(stem), &header_hash(&read(&source)));

        let package = go_package(stem);
        let project = work.join("project");
        std::fs::create_dir_all(project.join(&package)).expect("create the package directory");
        std::fs::copy(
            &source,
            project
                .join(&package)
                .join(source.file_name().expect("a file name")),
        )
        .expect("copy the package");
        std::fs::write(
            project.join("go.mod"),
            "module example.com/tfalone\n\ngo 1.22\n",
        )
        .expect("write go.mod");
        std::fs::write(
            project.join("main.go"),
            GO_LISTING.replace("@PACKAGE@", &package),
        )
        .expect("write the program");

        let mut run = Command::new(&go);
        run.args(["run", "."])
            .current_dir(&project)
            .env("GOWORK", "off")
            .env("GOFLAGS", "-mod=mod");
        assert_eq!(
            run_ok(run, &format!("[{stem}] the Go listing program")),
            listing(&function),
            "[{stem}] the compiled Go function does not answer as the producer"
        );
    }
}

/// A Kotlin program that prints the compiled function in `listing`'s shape.
const KOTLIN_LISTING: &str = r#"import @PACKAGE@.*

fun main() {
    print("hash\t" + SOURCE_HASH + "\n")
    print("initial\t" + INITIAL.scxmlName + "\n")
    for (state in ALL_STATES) {
        check(stateFromName(state.scxmlName) == state) { "a state name does not read back" }
        print("state\t" + state.scxmlName + "\t" + isFinal(state) + "\t" + configuration(state).joinToString(" ") + "\n")
    }
    for (event in ALL_EVENTS) {
        check(eventFromName(event.scxmlName) == event) { "an event name does not read back" }
        print("event\t" + event.scxmlName + "\n")
    }
    for (state in ALL_STATES) {
        for (event in ALL_EVENTS) {
            print("next\t" + state.scxmlName + "\t" + event.scxmlName + "\t" + (transition(state, event)?.scxmlName ?: "-") + "\n")
        }
    }
    check(stateFromName("") == null && eventFromName("") == null) { "an empty name reads as a value" }
}
"#;

/// The generated Kotlin compiles alone — `kotlinc` over the file and the
/// program, with no classpath — and the program prints every answer as the
/// producer.
#[test]
fn the_kotlin_function_compiles_alone_and_answers_as_the_producer() {
    let Some(kotlinc) = toolchain::locate("kotlinc") else {
        toolchain::skipped(
            "the_kotlin_function_compiles_alone_and_answers_as_the_producer: kotlinc not on PATH",
        );
        return;
    };
    let Some(java) = toolchain::locate("java") else {
        toolchain::skipped(
            "the_kotlin_function_compiles_alone_and_answers_as_the_producer: java not on PATH",
        );
        return;
    };
    for stem in FIXTURES {
        let work = scratch(&format!("kotlin_alone_{stem}"));
        let out = work.join("out");
        generate("kotlin", &fixture(stem), &out);
        let source = function_file("kotlin", &out, stem);
        let function = emitted(&fixture(stem), &header_hash(&read(&source)));

        let program = work.join("Check.kt");
        std::fs::write(
            &program,
            KOTLIN_LISTING.replace(
                "@PACKAGE@",
                &format!(
                    "com.sce.generated.{}_transition_function",
                    stem.to_lowercase()
                ),
            ),
        )
        .expect("write the program");
        let jar = work.join("check.jar");
        let mut compile = Command::new(&kotlinc);
        compile
            .arg("-nowarn")
            .arg(&source)
            .arg(&program)
            .arg("-include-runtime")
            .arg("-d")
            .arg(&jar);
        run_ok(
            compile,
            &format!("[{stem}] kotlinc over the function alone"),
        );
        let mut run = Command::new(&java);
        run.arg("-cp").arg(&jar).arg("CheckKt");
        assert_eq!(
            run_ok(run, &format!("[{stem}] the Kotlin listing program")),
            listing(&function),
            "[{stem}] the compiled Kotlin function does not answer as the producer"
        );
    }
}

/// A Python program that imports the generated module from its directory
/// alone and prints it in `listing`'s shape.
const PYTHON_LISTING: &str = r#"
import importlib
import sys

sys.path.insert(0, sys.argv[1])
tf = importlib.import_module(sys.argv[2])
lines = ["hash\t" + tf.SOURCE_HASH, "initial\t" + tf.INITIAL.value]
for state in tf.ALL_STATES:
    assert tf.State.from_name(state.value) is state, state
    assert str(state) == state.value, state
    lines.append(
        "state\t%s\t%s\t%s"
        % (
            state.value,
            "true" if tf.is_final(state) else "false",
            " ".join(tf.configuration(state)),
        )
    )
for event in tf.ALL_EVENTS:
    assert tf.Event.from_name(event.value) is event, event
    lines.append("event\t" + event.value)
for state in tf.ALL_STATES:
    for event in tf.ALL_EVENTS:
        to = tf.transition(state, event)
        lines.append(
            "next\t%s\t%s\t%s" % (state.value, event.value, "-" if to is None else to.value)
        )
assert tf.State.from_name("") is None and tf.Event.from_name("") is None
sys.stdout.write("\n".join(lines) + "\n")
"#;

/// The generated Python imports alone — from its own directory, in an
/// isolated interpreter with no SCE runtime on the path — and answers every
/// question as the producer.
#[test]
fn the_python_function_imports_alone_and_answers_as_the_producer() {
    let Some(python) = toolchain::locate_any(&["python3", "python"]) else {
        toolchain::skipped(
            "the_python_function_imports_alone_and_answers_as_the_producer: python not on PATH",
        );
        return;
    };
    for stem in FIXTURES {
        let work = scratch(&format!("python_alone_{stem}"));
        let out = work.join("out");
        generate("python", &fixture(stem), &out);
        let function = emitted(
            &fixture(stem),
            &header_hash(&read(&function_file("python", &out, stem))),
        );
        let mut run = Command::new(&python);
        run.args(["-I", "-c", PYTHON_LISTING])
            .arg(&out)
            .arg(format!("{stem}_transition_function"));
        assert_eq!(
            run_ok(run, &format!("[{stem}] the Python listing program")),
            listing(&function),
            "[{stem}] the imported Python function does not answer as the producer"
        );
    }
}

/// A program that drives the generated machine to every state of the function
/// and sends it every event there, against the function generated beside it.
fn equivalence_program(machine_file: &Path, function_file: &Path, machine: &str) -> String {
    format!(
        r#"#[path = {machine_path:?}]
mod machine;
#[path = {function_path:?}]
mod tf;

use sce_rust_runtime::{{Engine, StatePolicy}};

type Policy = machine::{machine}Policy;

fn fresh() -> Engine<Policy> {{
    let mut engine = Engine::new(Policy::new());
    engine.initialize();
    engine
}}

fn send(engine: &mut Engine<Policy>, event: tf::Event) {{
    engine.raise_external_by_name(event.as_str(), "");
    engine.step();
}}

/// The active configuration, outermost first.
fn active(engine: &Engine<Policy>) -> Vec<&'static str> {{
    let mut names: Vec<&'static str> = engine
        .get_active_states()
        .into_iter()
        .map(Policy::get_state_name)
        .collect();
    names.reverse();
    names
}}

/// The shortest sequence of events that takes the function from INITIAL to
/// `target`, read from the function alone.
fn path_to(target: tf::State) -> Option<Vec<tf::Event>> {{
    let mut frontier: Vec<(tf::State, Vec<tf::Event>)> = vec![(tf::INITIAL, Vec::new())];
    let mut seen = vec![tf::INITIAL];
    let mut next = 0;
    while next < frontier.len() {{
        let (state, path) = frontier[next].clone();
        next += 1;
        if state == target {{
            return Some(path);
        }}
        for event in tf::ALL_EVENTS {{
            if let Some(to) = tf::transition(state, event) {{
                if !seen.contains(&to) {{
                    seen.push(to);
                    let mut longer = path.clone();
                    longer.push(event);
                    frontier.push((to, longer));
                }}
            }}
        }}
    }}
    None
}}

fn main() {{
    assert_eq!(
        active(&fresh()),
        tf::configuration(tf::INITIAL),
        "the machine does not start where the function does"
    );
    let mut pairs = 0usize;
    for state in tf::ALL_STATES {{
        let path = path_to(state)
            .unwrap_or_else(|| panic!("{{}} is not reached from INITIAL", state.as_str()));
        for event in tf::ALL_EVENTS {{
            let mut engine = fresh();
            for step in &path {{
                send(&mut engine, *step);
            }}
            pairs += 1;
            if tf::is_final(state) {{
                assert_eq!(
                    engine.terminal_state().map(Policy::get_state_name),
                    Some(state.as_str()),
                    "the path to {{}} does not end the machine there",
                    state.as_str()
                );
                continue;
            }}
            assert_eq!(
                active(&engine),
                tf::configuration(state),
                "the path to {{}} leaves the machine elsewhere",
                state.as_str()
            );
            let discarded = engine.discarded_external_events();
            send(&mut engine, event);
            let pair = format!("({{}}, {{}})", state.as_str(), event.as_str());
            match tf::transition(state, event) {{
                None => {{
                    assert_eq!(
                        engine.discarded_external_events(),
                        discarded + 1,
                        "{{pair}}: the function refuses it and the machine took it"
                    );
                    assert_eq!(
                        active(&engine),
                        tf::configuration(state),
                        "{{pair}}: the machine moved on an event it discarded"
                    );
                }}
                Some(to) => {{
                    assert_eq!(
                        engine.discarded_external_events(),
                        discarded,
                        "{{pair}}: the function takes it and the machine discarded it"
                    );
                    if tf::is_final(to) {{
                        assert_eq!(
                            engine.terminal_state().map(Policy::get_state_name),
                            Some(to.as_str()),
                            "{{pair}}: the machine did not end in {{}}",
                            to.as_str()
                        );
                    }} else {{
                        assert_eq!(
                            active(&engine),
                            tf::configuration(to),
                            "{{pair}}: the machine is not where the function says"
                        );
                    }}
                }}
            }}
        }}
    }}
    println!("pairs {{pairs}}");
}}
"#,
        machine_path = machine_file.to_str().unwrap(),
        function_path = function_file.to_str().unwrap(),
    )
}

/// The generated machine, built against the real Rust runtime, and the
/// function generated beside it agree on every pair: the configuration the
/// engine reaches, and whether it took the event or discarded it.
#[test]
fn the_generated_machine_and_the_generated_function_agree_on_every_pair() {
    let Some(cargo) = toolchain::locate("cargo") else {
        toolchain::skipped(
            "the_generated_machine_and_the_generated_function_agree_on_every_pair: cargo not on PATH",
        );
        return;
    };
    for stem in FIXTURES {
        let work = scratch(&format!("engine_{stem}"));
        let out = work.join("out");
        generate("rust", &fixture(stem), &out);
        let function = produced(&fixture(stem));

        let project = work.join("project");
        std::fs::create_dir_all(project.join("src")).expect("create project");
        std::fs::write(
            project.join("Cargo.toml"),
            format!(
                "[package]\nname = \"tf-equivalence\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\n\n\
                 [dependencies]\nsce-rust-runtime = {{ path = {:?} }}\n\n[workspace]\n",
                repo_root().join("backends/rust/runtime").to_str().unwrap()
            ),
        )
        .expect("write manifest");
        std::fs::write(
            project.join("src/main.rs"),
            equivalence_program(
                &out.join(format!("{stem}_sm.rs")),
                &out.join(format!("{stem}_transition_function.rs")),
                &filters::to_pascal_case(stem.to_string()),
            ),
        )
        .expect("write program");
        // The repo's own lock, so the runtime's dependencies resolve to the
        // versions already fetched.
        std::fs::copy(repo_root().join("Cargo.lock"), project.join("Cargo.lock"))
            .expect("copy the lock file");

        let manifest = project.join("Cargo.toml");
        let target_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("emitted_transition_function_rust_target");
        let run = common::run_cargo_offline_first(|| {
            let mut build = Command::new(&cargo);
            build
                .args(["run", "--quiet", "--manifest-path"])
                .arg(&manifest)
                .env("CARGO_TARGET_DIR", &target_dir);
            build
        });
        let printed = output_ok(run.output, &format!("[{stem}] the equivalence program"));
        assert_eq!(
            printed.trim(),
            format!("pairs {}", function.states.len() * function.events.len()),
            "[{stem}] the program did not check every pair"
        );
    }
}

// ── The other engines, driven against the function beside them ─────────────
//
// The same drive as the Rust program above, in each language: a fresh machine
// per pair, the shortest event path the function gives to the state, then the
// event. A program prints `pairs <n>` and nothing else when every pair agrees,
// and a `MISMATCH:` line per disagreement otherwise. The configuration is
// compared as a set of ids — a state's ancestors are fixed by the document, so
// the set says what the ordered list says — because not every engine hands its
// configuration out in an order.

/// What a drive over every pair of `function` prints when they all agree.
fn every_pair(function: &TransitionFunction) -> String {
    format!("pairs {}", function.states.len() * function.events.len())
}

/// The Go drive: the machine's package, built against the real runtime, and
/// the function's package inside it.
const GO_EQUIVALENCE: &str = r#"package main

import (
	"fmt"
	"os"
	"sort"

	machine "example.com/tfequivalence/@STEM@"
	tf "example.com/tfequivalence/@STEM@/@PACKAGE@"
	sce "github.com/newmassrael/sce-go-runtime"
)

type engine = sce.Engine[machine.@MACHINE@State, machine.@MACHINE@Event]

func fresh() *engine {
	policy := machine.New@MACHINE@Policy()
	started := sce.NewEngine[machine.@MACHINE@State, machine.@MACHINE@Event](&policy)
	started.Initialize()
	return started
}

func send(running *engine, event tf.Event) {
	running.RaiseExternalByName(event.String(), "")
	running.Step()
}

func active(running *engine) []string {
	names := []string{}
	for _, state := range running.GetActiveStates() {
		names = append(names, state.String())
	}
	sort.Strings(names)
	return names
}

func sorted(ids []string) []string {
	names := append([]string{}, ids...)
	sort.Strings(names)
	return names
}

func same(a, b []string) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if a[i] != b[i] {
			return false
		}
	}
	return true
}

func terminal(running *engine) string {
	state, ended := running.TerminalState()
	if !ended {
		return ""
	}
	return state.String()
}

type step struct {
	state tf.State
	path  []tf.Event
}

func pathTo(target tf.State) ([]tf.Event, bool) {
	frontier := []step{{tf.Initial, nil}}
	seen := map[tf.State]bool{tf.Initial: true}
	for next := 0; next < len(frontier); next++ {
		current := frontier[next]
		if current.state == target {
			return current.path, true
		}
		for _, event := range tf.AllEvents {
			if to, ok := tf.Transition(current.state, event); ok && !seen[to] {
				seen[to] = true
				longer := append(append([]tf.Event{}, current.path...), event)
				frontier = append(frontier, step{to, longer})
			}
		}
	}
	return nil, false
}

func main() {
	bad := 0
	expect := func(ok bool, what string) {
		if !ok {
			fmt.Println("MISMATCH:", what)
			bad++
		}
	}
	expect(same(active(fresh()), sorted(tf.Configuration(tf.Initial))), "the machine does not start where the function does")
	pairs := 0
	for _, state := range tf.AllStates {
		path, reached := pathTo(state)
		if !reached {
			expect(false, state.String()+" is not reached from Initial")
			continue
		}
		for _, event := range tf.AllEvents {
			running := fresh()
			for _, taken := range path {
				send(running, taken)
			}
			pairs++
			pair := "(" + state.String() + ", " + event.String() + ")"
			if tf.IsFinal(state) {
				expect(terminal(running) == state.String(), pair+": the path does not end the machine there")
				continue
			}
			expect(same(active(running), sorted(tf.Configuration(state))), pair+": the path leaves the machine elsewhere")
			discarded := running.DiscardedExternalEvents()
			send(running, event)
			to, ok := tf.Transition(state, event)
			if !ok {
				expect(running.DiscardedExternalEvents() == discarded+1, pair+": the function refuses it and the machine took it")
				expect(same(active(running), sorted(tf.Configuration(state))), pair+": the machine moved on an event it discarded")
				continue
			}
			expect(running.DiscardedExternalEvents() == discarded, pair+": the function takes it and the machine discarded it")
			if tf.IsFinal(to) {
				expect(terminal(running) == to.String(), pair+": the machine did not end where the function says")
			} else {
				expect(same(active(running), sorted(tf.Configuration(to))), pair+": the machine is not where the function says")
			}
		}
	}
	fmt.Printf("pairs %d\n", pairs)
	if bad > 0 {
		os.Exit(1)
	}
}
"#;

/// The generated Go machine, built against the real Go runtime, and the Go
/// function generated beside it agree on every pair.
#[test]
fn the_go_machine_and_the_go_function_agree_on_every_pair() {
    let Some(go) = toolchain::locate("go") else {
        toolchain::skipped(
            "the_go_machine_and_the_go_function_agree_on_every_pair: go not on PATH",
        );
        return;
    };
    for stem in FIXTURES {
        let work = scratch(&format!("go_engine_{stem}"));
        let project = work.join("project");
        let out = project.join(stem);
        generate("go", &fixture(stem), &out);
        let function = produced(&fixture(stem));
        std::fs::write(
            project.join("go.mod"),
            format!(
                "module example.com/tfequivalence\n\ngo 1.22\n\nrequire github.com/newmassrael/sce-go-runtime v0.0.0\n\n\
                 replace github.com/newmassrael/sce-go-runtime => {}\n",
                repo_root().join("backends/go/runtime").to_str().unwrap()
            ),
        )
        .expect("write go.mod");
        std::fs::write(
            project.join("main.go"),
            GO_EQUIVALENCE
                .replace("@STEM@", stem)
                .replace("@PACKAGE@", &go_package(stem))
                .replace("@MACHINE@", &filters::to_pascal_case(stem.to_string())),
        )
        .expect("write the program");
        let mut run = Command::new(&go);
        run.args(["run", "."])
            .current_dir(&project)
            .env("GOFLAGS", "-mod=mod")
            .env("GOWORK", "off");
        assert_eq!(
            run_ok(run, &format!("[{stem}] the Go equivalence program")).trim(),
            every_pair(&function),
            "[{stem}] the Go program did not check every pair"
        );
    }
}

/// The C drive. `@M@` is the machine's symbol prefix and `@UM@` the same in
/// upper case; `@p@` and `@P@` are the function's.
const C_EQUIVALENCE: &str = r#"#include "@M@_sm.h"
#include "@M@_transition_function.h"

#include <stdio.h>
#include <string.h>

typedef @M@_t machine_t;

static int bad;

static void expect_that(int ok, const char *what, const char *state, const char *event) {
    if (!ok) {
        printf("MISMATCH: (%s, %s): %s\n", state, event, what);
        bad++;
    }
}

static void deliver(machine_t *sm, @p@_event_t event) {
    @M@_event_t member;
    if (!@M@_resolve_event_by_name(@p@_event_name(event), &member)) {
        return;
    }
    @M@_event_with_meta_t carrier;
    memset(&carrier, 0, sizeof carrier);
    carrier.event = member;
    @M@_raise_external(sm, &carrier);
    @M@_step(sm);
}

/* Whether the machine's active states are exactly the ids `configuration` names. */
static int holds_configuration(const machine_t *sm, @p@_configuration_t configuration) {
    size_t active = 0;
    for (int s = 0; s < (int)@UM@_STATE_COUNT; ++s) {
        if (@M@_in_state(sm, (@M@_state_t)s)) {
            active++;
        }
    }
    if (active != configuration.count) {
        return 0;
    }
    for (size_t k = 0; k < configuration.count; ++k) {
        @M@_state_t state;
        if (!@M@_state_from_name(configuration.ids[k], &state) || !@M@_in_state(sm, state)) {
            return 0;
        }
    }
    return 1;
}

static const char *terminal_name(const machine_t *sm) {
    @M@_state_t state;
    if (!@M@_terminal_state(sm, &state)) {
        return "";
    }
    return @M@_state_name(state);
}

/* The shortest sequence of events from the initial state to `target`, read
   from the function alone, written to `path`; its length, or -1. */
static int shortest_path(@p@_state_t target, @p@_event_t *path) {
    @p@_state_t frontier[@P@_NUM_STATES];
    int parent[@P@_NUM_STATES];
    @p@_event_t via[@P@_NUM_STATES];
    int seen[@P@_NUM_STATES];
    const @p@_event_t *events = @p@_all_events();
    int head = 0;
    int tail = 0;
    memset(seen, 0, sizeof seen);
    frontier[tail] = @P@_INITIAL;
    parent[tail] = -1;
    via[tail] = (@p@_event_t)0;
    tail++;
    seen[@P@_INITIAL] = 1;
    while (head < tail) {
        int at = head++;
        if (frontier[at] == target) {
            int length = 0;
            for (int k = at; parent[k] >= 0; k = parent[k]) {
                length++;
            }
            int position = length;
            for (int k = at; parent[k] >= 0; k = parent[k]) {
                path[--position] = via[k];
            }
            return length;
        }
        for (size_t j = 0; j < (size_t)@P@_NUM_EVENTS; ++j) {
            @p@_state_t to = frontier[at];
            if (@p@_transition(frontier[at], events[j], &to) && !seen[to]) {
                seen[to] = 1;
                frontier[tail] = to;
                parent[tail] = at;
                via[tail] = events[j];
                tail++;
            }
        }
    }
    return -1;
}

int main(void) {
    const @p@_state_t *states = @p@_all_states();
    const @p@_event_t *events = @p@_all_events();
    int pairs = 0;
    {
        machine_t sm;
        @M@_init(&sm);
        expect_that(holds_configuration(&sm, @p@_configuration(@P@_INITIAL)),
                    "the machine does not start where the function does", "", "");
        @M@_destroy(&sm);
    }
    for (size_t i = 0; i < (size_t)@P@_NUM_STATES; ++i) {
        @p@_state_t state = states[i];
        const char *state_name = @p@_state_name(state);
        @p@_event_t path[@P@_NUM_STATES];
        int length = shortest_path(state, path);
        if (length < 0) {
            expect_that(0, "the state is not reached from the initial state", state_name, "");
            continue;
        }
        for (size_t j = 0; j < (size_t)@P@_NUM_EVENTS; ++j) {
            @p@_event_t event = events[j];
            const char *event_name = @p@_event_name(event);
            machine_t sm;
            @M@_init(&sm);
            for (int k = 0; k < length; ++k) {
                deliver(&sm, path[k]);
            }
            pairs++;
            if (@p@_is_final(state)) {
                expect_that(strcmp(terminal_name(&sm), state_name) == 0,
                            "the path does not end the machine there", state_name, event_name);
                @M@_destroy(&sm);
                continue;
            }
            expect_that(holds_configuration(&sm, @p@_configuration(state)),
                        "the path leaves the machine elsewhere", state_name, event_name);
            uint32_t discarded = @M@_discarded_external_events(&sm);
            deliver(&sm, event);
            @p@_state_t to = state;
            if (!@p@_transition(state, event, &to)) {
                expect_that(@M@_discarded_external_events(&sm) == discarded + 1u,
                            "the function refuses it and the machine took it", state_name, event_name);
                expect_that(holds_configuration(&sm, @p@_configuration(state)),
                            "the machine moved on an event it discarded", state_name, event_name);
            } else {
                expect_that(@M@_discarded_external_events(&sm) == discarded,
                            "the function takes it and the machine discarded it", state_name, event_name);
                if (@p@_is_final(to)) {
                    expect_that(strcmp(terminal_name(&sm), @p@_state_name(to)) == 0,
                                "the machine did not end where the function says", state_name, event_name);
                } else {
                    expect_that(holds_configuration(&sm, @p@_configuration(to)),
                                "the machine is not where the function says", state_name, event_name);
                }
            }
            @M@_destroy(&sm);
        }
    }
    printf("pairs %d\n", pairs);
    return bad ? 1 : 0;
}
"#;

/// The generated C machine, built with the C runtime's headers, and the C
/// function generated beside it — both included in one translation unit —
/// agree on every pair.
#[test]
fn the_c_machine_and_the_c_function_agree_on_every_pair() {
    let Some(gcc) = toolchain::locate_any(&["gcc", "cc"]) else {
        toolchain::skipped(
            "the_c_machine_and_the_c_function_agree_on_every_pair: gcc/cc not on PATH",
        );
        return;
    };
    for stem in FIXTURES {
        let work = scratch(&format!("c_engine_{stem}"));
        let out = work.join("out");
        generate("c11", &fixture(stem), &out);
        let function = produced(&fixture(stem));
        let program = work.join("check.c");
        std::fs::write(
            &program,
            C_EQUIVALENCE
                .replace("@UM@", &stem.to_uppercase())
                .replace("@M@", stem)
                .replace("@p@", &format!("{stem}_transition_function"))
                .replace("@P@", &c_prefix_upper(stem)),
        )
        .expect("write the program");
        let binary = work.join("check");
        let mut compile = Command::new(&gcc);
        compile
            .args(["-std=c11", "-I"])
            .arg(repo_root().join("backends/c/runtime/include"))
            .arg("-I")
            .arg(&out)
            .arg(&program)
            .arg(out.join(format!("{stem}_sm.c")))
            .arg("-o")
            .arg(&binary);
        run_ok(
            compile,
            &format!("[{stem}] cc over the machine, the function and the drive"),
        );
        assert_eq!(
            run_ok(
                Command::new(&binary),
                &format!("[{stem}] the C equivalence program")
            )
            .trim(),
            every_pair(&function),
            "[{stem}] the C program did not check every pair"
        );
    }
}

/// The Kotlin runtime jar the Gradle build of `backends/kotlin/runtime` left,
/// the newest when there are several.
fn kotlin_runtime_jar() -> Option<PathBuf> {
    let libs = repo_root().join("backends/kotlin/runtime/build/libs");
    let mut jars: Vec<PathBuf> = std::fs::read_dir(libs)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            name.starts_with("sce-kotlin-runtime-jvm-")
                && name.ends_with(".jar")
                && !name.ends_with("-sources.jar")
        })
        .collect();
    jars.sort();
    jars.pop()
}

/// The coroutines jar the Kotlin runtime is built against and runs with: the
/// version `gradle/libs.versions.toml` names, from the Gradle cache the
/// runtime's own build filled.
fn kotlin_coroutines_jar() -> Option<PathBuf> {
    let versions = read(&repo_root().join("gradle/libs.versions.toml"));
    let version = versions.lines().find_map(|line| {
        let value = line
            .trim()
            .strip_prefix("coroutines")?
            .trim_start()
            .strip_prefix('=')?;
        Some(value.trim().trim_matches('"').to_string())
    })?;
    let gradle_home = std::env::var_os("GRADLE_USER_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".gradle")))?;
    let artifact = gradle_home
        .join("caches/modules-2/files-2.1/org.jetbrains.kotlinx/kotlinx-coroutines-core-jvm")
        .join(&version);
    let name = format!("kotlinx-coroutines-core-jvm-{version}.jar");
    std::fs::read_dir(artifact)
        .ok()?
        .flatten()
        .map(|entry| entry.path().join(&name))
        .find(|jar| jar.is_file())
}

/// The Kotlin drive.
const KOTLIN_EQUIVALENCE: &str = r#"import @MACHINE_PACKAGE@.@MACHINE@StateMachine
import @PACKAGE@.ALL_EVENTS
import @PACKAGE@.ALL_STATES
import @PACKAGE@.Event
import @PACKAGE@.INITIAL
import @PACKAGE@.State
import @PACKAGE@.configuration
import @PACKAGE@.isFinal
import @PACKAGE@.transition

fun fresh(): @MACHINE@StateMachine {
    val machine = @MACHINE@StateMachine()
    machine.initialize()
    return machine
}

fun send(machine: @MACHINE@StateMachine, event: Event) {
    machine.sendEventByName(event.scxmlName)
    machine.tick()
}

fun active(machine: @MACHINE@StateMachine): Set<String> =
    machine.activeConfiguration.map { machine.nameOfState(it) }.toSet()

fun terminal(machine: @MACHINE@StateMachine): String =
    machine.terminalState?.let { machine.nameOfState(it) } ?: ""

fun pathTo(target: State): List<Event>? {
    val frontier = mutableListOf(Pair(INITIAL, emptyList<Event>()))
    val seen = mutableSetOf(INITIAL)
    var index = 0
    while (index < frontier.size) {
        val (state, path) = frontier[index]
        index++
        if (state == target) return path
        for (event in ALL_EVENTS) {
            val to = transition(state, event)
            if (to != null && seen.add(to)) {
                frontier.add(Pair(to, path + event))
            }
        }
    }
    return null
}

fun main() {
    val bad = mutableListOf<String>()
    fun expect(ok: Boolean, what: String) {
        if (!ok) bad.add(what)
    }
    expect(active(fresh()) == configuration(INITIAL).toSet(), "the machine does not start where the function does")
    var pairs = 0
    for (state in ALL_STATES) {
        val path = pathTo(state)
        if (path == null) {
            expect(false, state.scxmlName + " is not reached from INITIAL")
            continue
        }
        for (event in ALL_EVENTS) {
            val machine = fresh()
            for (taken in path) send(machine, taken)
            pairs++
            val pair = "(" + state.scxmlName + ", " + event.scxmlName + ")"
            if (isFinal(state)) {
                expect(terminal(machine) == state.scxmlName, pair + ": the path does not end the machine there")
                continue
            }
            expect(active(machine) == configuration(state).toSet(), pair + ": the path leaves the machine elsewhere")
            val discarded = machine.discardedExternalEvents()
            send(machine, event)
            val to = transition(state, event)
            if (to == null) {
                expect(machine.discardedExternalEvents() == discarded + 1, pair + ": the function refuses it and the machine took it")
                expect(active(machine) == configuration(state).toSet(), pair + ": the machine moved on an event it discarded")
            } else {
                expect(machine.discardedExternalEvents() == discarded, pair + ": the function takes it and the machine discarded it")
                if (isFinal(to)) {
                    expect(terminal(machine) == to.scxmlName, pair + ": the machine did not end where the function says")
                } else {
                    expect(active(machine) == configuration(to).toSet(), pair + ": the machine is not where the function says")
                }
            }
        }
    }
    for (line in bad) println("MISMATCH: " + line)
    println("pairs " + pairs)
    if (bad.isNotEmpty()) System.exit(1)
}
"#;

/// The generated Kotlin machine, compiled against the real runtime jar, and
/// the Kotlin function generated beside it agree on every pair.
#[test]
fn the_kotlin_machine_and_the_kotlin_function_agree_on_every_pair() {
    let what = "the_kotlin_machine_and_the_kotlin_function_agree_on_every_pair";
    let Some(kotlinc) = toolchain::locate("kotlinc") else {
        toolchain::skipped(&format!("{what}: kotlinc not on PATH"));
        return;
    };
    let Some(java) = toolchain::locate("java") else {
        toolchain::skipped(&format!("{what}: java not on PATH"));
        return;
    };
    let Some(runtime_jar) = kotlin_runtime_jar() else {
        toolchain::skipped(&format!(
            "{what}: runtime jar missing under backends/kotlin/runtime/build/libs \
             (run `./gradlew :sce-kotlin-runtime:assemble`)"
        ));
        return;
    };
    let Some(coroutines_jar) = kotlin_coroutines_jar() else {
        toolchain::skipped(&format!(
            "{what}: the kotlinx-coroutines jar the runtime is built against is not in the \
             Gradle cache (run `./gradlew :sce-kotlin-runtime:assemble`)"
        ));
        return;
    };
    let classpath = format!(
        "{}:{}",
        runtime_jar.to_str().unwrap(),
        coroutines_jar.to_str().unwrap()
    );
    for stem in FIXTURES {
        let work = scratch(&format!("kotlin_engine_{stem}"));
        let out = work.join("out");
        generate("kotlin", &fixture(stem), &out);
        let function_path = function_file("kotlin", &out, stem);
        let function = produced(&fixture(stem));
        let program = work.join("Check.kt");
        std::fs::write(
            &program,
            KOTLIN_EQUIVALENCE
                .replace(
                    "@MACHINE_PACKAGE@",
                    &format!("com.sce.generated.{}", stem.to_lowercase()),
                )
                .replace(
                    "@PACKAGE@",
                    &format!(
                        "com.sce.generated.{}_transition_function",
                        stem.to_lowercase()
                    ),
                )
                .replace("@MACHINE@", &filters::to_pascal_case(stem.to_string())),
        )
        .expect("write the program");
        let jar = work.join("check.jar");
        let mut compile = Command::new(&kotlinc);
        compile
            .arg("-nowarn")
            .arg("-cp")
            .arg(&classpath)
            .arg(machine_file("kotlin", &out, stem))
            .arg(&function_path)
            .arg(&program)
            .arg("-include-runtime")
            .arg("-d")
            .arg(&jar);
        run_ok(
            compile,
            &format!("[{stem}] kotlinc over the machine, the function and the drive"),
        );
        let mut run = Command::new(&java);
        run.arg("-cp")
            .arg(format!("{}:{classpath}", jar.to_str().unwrap()))
            .arg("CheckKt");
        assert_eq!(
            run_ok(run, &format!("[{stem}] the Kotlin equivalence program")).trim(),
            every_pair(&function),
            "[{stem}] the Kotlin program did not check every pair"
        );
    }
}

/// The Python drive: the machine's module and the function's, imported from
/// the directory they were generated into, and the real runtime.
const PYTHON_EQUIVALENCE: &str = r#"
import importlib
import sys

sys.path.insert(0, sys.argv[1])
sys.path.insert(0, sys.argv[2])
machine = importlib.import_module(sys.argv[3])
tf = importlib.import_module(sys.argv[4])


def fresh():
    engine = machine.create_engine()
    engine.initialize()
    return engine


def send(engine, event):
    member = engine.policy.get_event_from_name(event.value)
    if member is not None:
        engine.send_event(member)


def active(engine):
    return {str(state) for state in engine.active_configuration()}


def terminal(engine):
    state = engine.terminal_state
    return "" if state is None else str(state)


def path_to(target):
    frontier = [(tf.INITIAL, [])]
    seen = {tf.INITIAL}
    index = 0
    while index < len(frontier):
        state, path = frontier[index]
        index += 1
        if state is target:
            return path
        for event in tf.ALL_EVENTS:
            to = tf.transition(state, event)
            if to is not None and to not in seen:
                seen.add(to)
                frontier.append((to, path + [event]))
    return None


bad = []


def expect(ok, what):
    if not ok:
        bad.append(what)


expect(
    active(fresh()) == set(tf.configuration(tf.INITIAL)),
    "the machine does not start where the function does",
)
pairs = 0
for state in tf.ALL_STATES:
    path = path_to(state)
    if path is None:
        expect(False, state.value + " is not reached from INITIAL")
        continue
    for event in tf.ALL_EVENTS:
        engine = fresh()
        for taken in path:
            send(engine, taken)
        pairs += 1
        pair = "(%s, %s)" % (state.value, event.value)
        if tf.is_final(state):
            expect(terminal(engine) == state.value, pair + ": the path does not end the machine there")
            continue
        expect(
            active(engine) == set(tf.configuration(state)),
            pair + ": the path leaves the machine elsewhere",
        )
        discarded = engine.discarded_external_events()
        send(engine, event)
        to = tf.transition(state, event)
        if to is None:
            expect(
                engine.discarded_external_events() == discarded + 1,
                pair + ": the function refuses it and the machine took it",
            )
            expect(
                active(engine) == set(tf.configuration(state)),
                pair + ": the machine moved on an event it discarded",
            )
        else:
            expect(
                engine.discarded_external_events() == discarded,
                pair + ": the function takes it and the machine discarded it",
            )
            if tf.is_final(to):
                expect(terminal(engine) == to.value, pair + ": the machine did not end where the function says")
            else:
                expect(
                    active(engine) == set(tf.configuration(to)),
                    pair + ": the machine is not where the function says",
                )
for line in bad:
    print("MISMATCH: " + line)
print("pairs %d" % pairs)
sys.exit(1 if bad else 0)
"#;

/// The generated Python machine, run on the real Python runtime, and the
/// Python function generated beside it agree on every pair.
#[test]
fn the_python_machine_and_the_python_function_agree_on_every_pair() {
    let what = "the_python_machine_and_the_python_function_agree_on_every_pair";
    let Some(python) = toolchain::locate_any(&["python3", "python"]) else {
        toolchain::skipped(&format!("{what}: python not on PATH"));
        return;
    };
    // The runtime every generated machine runs on needs the Lua binding, and
    // a host without it can run no machine at all: a missing tool, not a
    // function that disagrees.
    let has_lupa = Command::new(&python)
        .args(["-I", "-c", "import lupa"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !has_lupa {
        toolchain::skipped(&format!(
            "{what}: lupa is not importable (`python3 -m pip install lupa`)"
        ));
        return;
    }
    for stem in FIXTURES {
        let work = scratch(&format!("python_engine_{stem}"));
        let out = work.join("out");
        generate("python", &fixture(stem), &out);
        let function = produced(&fixture(stem));
        let mut run = Command::new(&python);
        run.args(["-I", "-c", PYTHON_EQUIVALENCE])
            .arg(&out)
            .arg(repo_root().join("backends/python/runtime"))
            .arg(format!("{stem}_sm"))
            .arg(format!("{stem}_transition_function"));
        assert_eq!(
            run_ok(run, &format!("[{stem}] the Python equivalence program")).trim(),
            every_pair(&function),
            "[{stem}] the Python program did not check every pair"
        );
    }
}

/// A document that does not declare the function gets no file for it, in any
/// backend.
#[test]
fn an_undeclared_document_is_generated_without_a_function() {
    let work = scratch("undeclared");
    let text = read(&fixture("flat_lifecycle"));
    let without = text.replace("\n       sce:transition-function=\"emit\"", "");
    assert_ne!(
        without, text,
        "the fixture must differ by the declaration alone"
    );
    let doc = work.join("flat_lifecycle.scxml");
    std::fs::write(&doc, without).expect("write the document");
    for lang in backends() {
        let out = work.join(format!("out_{lang}"));
        generate(lang, &doc, &out);
        assert!(
            machine_file(lang, &out, "flat_lifecycle").is_file(),
            "[{lang}] no machine was written"
        );
        assert!(
            !function_file(lang, &out, "flat_lifecycle").exists(),
            "[{lang}] a document that declares no function was generated with one"
        );
    }
}

/// A document that declares the function is accepted by every backend:
/// `check` naming each one passes, and a `check` sweep reports every backend
/// `ok`.
#[test]
fn every_backend_accepts_the_declaration() {
    let doc = fixture("flat_lifecycle");
    let doc_str = doc.to_str().unwrap();
    for lang in backends() {
        output_ok(
            codegen(&["--error-format=json", "check", doc_str, "-l", lang]),
            &format!("check -l {lang}"),
        );
    }
    let swept = codegen(&["check", doc_str]);
    let manifest: serde_json::Value = serde_json::from_str(output_ok(swept, "check sweep").trim())
        .expect("the sweep emits a manifest");
    let verdicts = manifest["languages"]
        .as_array()
        .expect("per-backend verdicts");
    for verdict in verdicts {
        assert_eq!(verdict["status"], "ok", "{verdict}");
    }
    assert_eq!(verdicts.len(), Language::ALL.len(), "{manifest}");
}

/// The declaration has one value; any other is refused with the attribute's
/// name, by `check` and `generate` alike.
#[test]
fn a_value_other_than_emit_is_refused() {
    let work = scratch("other_value");
    let text = std::fs::read_to_string(fixture("flat_lifecycle")).expect("read the fixture");
    let doc = work.join("flat_lifecycle.scxml");
    std::fs::write(
        &doc,
        text.replace(
            r#"sce:transition-function="emit""#,
            r#"sce:transition-function="yes""#,
        ),
    )
    .expect("write the document");
    let out = work.join("out");
    let doc_str = doc.to_str().unwrap();
    for args in [
        vec!["--error-format=json", "check", doc_str],
        vec![
            "--error-format=json",
            "generate",
            doc_str,
            "-l",
            "rust",
            "-o",
            out.to_str().unwrap(),
        ],
    ] {
        let refused = codegen(&args);
        assert!(
            !refused.status.success(),
            "{args:?}: a refused declaration passed"
        );
        let stderr = String::from_utf8_lossy(&refused.stderr);
        assert_eq!(
            wire_codes(&stderr),
            ["validation/attribute-rule-violated"],
            "{stderr}"
        );
        assert!(stderr.contains("sce:transition-function"), "{stderr}");
    }
}

/// A forge root does not take the declaration: the forge pipeline knows the
/// root attributes each kind takes by name, and this is none of them, so it is
/// refused as an `sce:` attribute that kind does not know.
#[test]
fn a_forge_root_does_not_take_the_declaration() {
    let work = scratch("forge_root");
    let doc = work.join("view_mode.scxml");
    std::fs::write(
        &doc,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="enum" name="view_mode" sce:underlying-type="uint8"
       sce:transition-function="emit">
  <datamodel>
    <data id="variants">
      <sce:variant name="month" value="0"/>
      <sce:variant name="week" value="1"/>
    </data>
  </datamodel>
</scxml>
"#,
    )
    .expect("write the document");
    let refused = codegen(&["--error-format=json", "check", doc.to_str().unwrap()]);
    assert!(
        !refused.status.success(),
        "a forge root took the declaration"
    );
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert_eq!(
        wire_codes(&stderr),
        ["validation/unknown-sce-attribute"],
        "{stderr}"
    );
    assert!(stderr.contains("sce:transition-function"), "{stderr}");
}

/// A document that declares the function and holds what it refuses, several
/// of some: the document every route below is asked about.
const IMPURE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="impure" initial="idle" datamodel="null"
       sce:transition-function="emit">
  <state id="idle">
    <onentry><raise event="poke"/></onentry>
    <transition event="go" target="both"/>
    <transition target="idle" cond="In('idle')"/>
    <transition event="split" target="r1 r2"/>
    <transition event="door.*" target="idle"/>
  </state>
  <parallel id="both">
    <state id="r1"/>
    <state id="r2"/>
  </parallel>
  <state id="outer">
    <history id="h"><transition target="inner"/></history>
    <state id="inner"><transition event="go" target="done"/></state>
    <final id="done"/>
  </state>
</scxml>
"#;

/// The code of every record [`IMPURE`] earns, sorted.
const IMPURE_CODES: [&str; 13] = [
    "scxml/dead-transition",
    "scxml/transition-function-dead-end",
    "scxml/transition-function-dead-end",
    "scxml/transition-function-rule",
    "scxml/transition-function-rule",
    "scxml/transition-function-rule",
    "scxml/transition-function-rule",
    "scxml/transition-function-rule",
    "scxml/transition-function-rule",
    "scxml/transition-function-rule",
    "scxml/transition-function-unmatched-descriptor",
    "scxml/unreachable-state",
    "scxml/unreachable-state",
];

/// [`IMPURE`], written into `dir` as `file`.
fn write_impure(dir: &Path, file: &str) -> PathBuf {
    let doc = dir.join(file);
    std::fs::write(&doc, IMPURE).expect("write the document");
    doc
}

/// Every record a `--error-format=json` run that must fail wrote, in the
/// order written, each line whole: two routes that agree on the lines agree
/// on the code, the location and the anchors alike.
fn refused_records(args: &[&str]) -> Vec<String> {
    let refused = codegen(args);
    assert!(
        !refused.status.success(),
        "{args:?}: an impure declaration passed"
    );
    String::from_utf8_lossy(&refused.stderr)
        .lines()
        .filter(|line| line.starts_with('{'))
        .map(str::to_string)
        .collect()
}

fn sorted(mut records: Vec<String>) -> Vec<String> {
    records.sort();
    records
}

/// The sorted codes of `records`.
fn codes_of(records: &[String]) -> Vec<String> {
    let mut codes = wire_codes(&records.join("\n"));
    codes.sort();
    codes
}

/// What `check` on the one document at `doc` reports, in the order written:
/// what every other route is held to. Held itself to every refusal first, so
/// a route that agrees with it agrees about all of them.
fn checked_records(doc: &str) -> Vec<String> {
    let records = refused_records(&["--error-format=json", "check", doc, "-l", "rust"]);
    assert_eq!(
        codes_of(&records),
        IMPURE_CODES,
        "check no longer reports every refusal of the document"
    );
    records
}

/// Every refusal the declaration brings comes in one run, and `check` and
/// `generate` report the same records.
#[test]
fn check_and_generate_report_every_refusal_at_once() {
    let work = scratch("impure");
    let doc = write_impure(&work, "impure.scxml");
    let doc_str = doc.to_str().unwrap();
    let out = work.join("out");
    let checked = checked_records(doc_str);
    let generated = refused_records(&[
        "--error-format=json",
        "generate",
        doc_str,
        "-l",
        "rust",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(
        sorted(generated),
        sorted(checked),
        "check and generate disagree"
    );
    assert!(
        !out.join("impure_sm.rs").exists(),
        "a refused document generated a machine"
    );
}

/// `check` over a document set reports what `check` reports for the one
/// document: every record, the same records. The set is compiled by the
/// library, whose result carries one error, so without the pass over the set
/// the route reported the first refusal alone.
#[test]
fn a_document_set_check_reports_every_refusal_check_reports() {
    let work = scratch("impure_set");
    let doc = write_impure(&work, "impure.scxml");
    let doc_str = doc.to_str().unwrap();
    let checked = checked_records(doc_str);
    let set = refused_records(&[
        "--error-format=json",
        "check",
        "--scxml",
        doc_str,
        "-l",
        "rust",
    ]);
    assert_eq!(
        sorted(set),
        sorted(checked),
        "check on a set and check on the document disagree"
    );
}

/// `orchestrate` reports what `check` reports, and writes nothing for a
/// refused document.
#[test]
fn orchestrate_reports_every_refusal_check_reports() {
    let work = scratch("impure_orchestrate");
    let doc = write_impure(&work, "impure.scxml");
    let doc_str = doc.to_str().unwrap();
    let out = work.join("out");
    let checked = checked_records(doc_str);
    let orchestrated = refused_records(&[
        "--error-format=json",
        "orchestrate",
        "--scxml",
        doc_str,
        "-l",
        "rust",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(
        sorted(orchestrated),
        sorted(checked),
        "orchestrate and check disagree"
    );
    assert!(
        !out.join("impure_sm.rs").exists(),
        "orchestrate generated a machine for a refused document"
    );
}

/// A conformance suite of one fixture, `test<id>.scxml`, for `generate-w3c`.
struct W3cSuite {
    resources: PathBuf,
    registry: PathBuf,
    document: PathBuf,
}

impl W3cSuite {
    /// `text` as fixture `id` under `work`, with a registry naming it alone:
    /// the committed registry's harnesses and version, which are what the
    /// loader asks of one.
    fn stage(work: &Path, id: &str, text: &str) -> Self {
        let resources = work.join("resources");
        let fixture_dir = resources.join(id);
        std::fs::create_dir_all(&fixture_dir).expect("create the fixture directory");
        let document = fixture_dir.join(format!("test{id}.scxml"));
        std::fs::write(&document, text).expect("write the fixture");

        let committed = std::fs::read_to_string(
            repo_root().join(sce_build::w3c_registry::W3C_REGISTRY_RELATIVE_PATH),
        )
        .expect("read the committed registry");
        let mut registry: serde_json::Value =
            serde_json::from_str(&committed).expect("the registry is JSON");
        registry["fixtures"] = serde_json::json!([{
            "id": id,
            "harness": "simple",
            "summary": "a fixture declaring a transition function"
        }]);
        let registry_path = work.join("fixtures.json");
        std::fs::write(
            &registry_path,
            serde_json::to_string_pretty(&registry).expect("serialize the registry"),
        )
        .expect("write the registry");
        Self {
            resources,
            registry: registry_path,
            document,
        }
    }

    /// `generate-w3c -l rust` of this suite into `out`, records as JSON.
    fn args<'a>(&'a self, out: &'a Path) -> Vec<&'a str> {
        vec![
            "--error-format=json",
            "generate-w3c",
            "-l",
            "rust",
            "--registry",
            self.registry.to_str().unwrap(),
            "--resources",
            self.resources.to_str().unwrap(),
            "--output-dir",
            out.to_str().unwrap(),
        ]
    }
}

/// `generate-w3c` reports, for a fixture that declares the function and does
/// not keep it, every record `check` reports for the same file, and fails the
/// fixture rather than generating it.
#[test]
fn generate_w3c_reports_every_refusal_check_reports() {
    let work = scratch("impure_w3c");
    let suite = W3cSuite::stage(&work, "900", IMPURE);
    let out = work.join("out");
    let checked = checked_records(suite.document.to_str().unwrap());
    let records = refused_records(&suite.args(&out));
    // The run ends on the batch's own record, naming the failed fixture.
    let (batch, findings): (Vec<String>, Vec<String>) = records
        .into_iter()
        .partition(|line| wire_codes(line) == ["cli/scxml-generate"]);
    assert_eq!(batch.len(), 1, "{batch:?}");
    assert!(
        batch[0].contains("900"),
        "the batch record does not name the fixture: {}",
        batch[0]
    );
    assert_eq!(
        sorted(findings),
        sorted(checked),
        "generate-w3c and check disagree"
    );
    assert!(
        !out.join("backends/rust/tests/src/generated/test900")
            .exists(),
        "generate-w3c generated a refused fixture"
    );
}

/// The library's file entry point rejects a declaring document on the record
/// `check` lists first, whole: its result carries one error, and it is the
/// first of the records the routes above report.
#[test]
fn the_library_file_entry_rejects_on_the_first_record() {
    let work = scratch("impure_library_file");
    let doc = write_impure(&work, "impure.scxml");
    let doc_str = doc.to_str().unwrap();
    let first: serde_json::Value =
        serde_json::from_str(&checked_records(doc_str)[0]).expect("a record is JSON");
    let refused = match sce_build::compile_scxml_lang_typed(
        doc_str,
        &sce_build::find_template_dir_for(Language::Rust),
        Language::Rust,
    ) {
        Ok(_) => panic!("the library compiled an impure declaration"),
        Err(err) => err,
    };
    let record = serde_json::to_value(&refused.to_diagnostics()[0]).expect("a record serializes");
    assert_eq!(record, first, "the library and check disagree");
}

/// The in-memory entry point, which the WASM build calls, rejects a declaring
/// document on the refusal `check` lists first. Compared by what the record
/// says rather than whole, since a document handed over as a string is
/// located under the name it was given, not under a path.
#[test]
fn the_library_in_memory_entry_rejects_on_the_first_record() {
    let work = scratch("impure_library_memory");
    let doc = write_impure(&work, "impure.scxml");
    let first: serde_json::Value =
        serde_json::from_str(&checked_records(doc.to_str().unwrap())[0]).expect("a record is JSON");
    let templates = sce_build::template_registry::embedded_templates_for(Language::Rust);
    let refused = match sce_build::compile_from_string_lang_typed(
        IMPURE,
        "impure",
        &templates,
        Language::Rust,
    ) {
        Ok(_) => panic!("the in-memory entry compiled an impure declaration"),
        Err(err) => err,
    };
    let record = serde_json::to_value(&refused.to_diagnostics()[0]).expect("a record serializes");
    for field in ["code", "actual", "expected"] {
        assert_eq!(
            record[field], first[field],
            "the in-memory entry and check disagree on `{field}`"
        );
    }
}

/// A document that declares the function and keeps it, with nothing a
/// design-time lint reports (the library entry points lint every document),
/// and a final state named `pass`, so it is a conformance fixture as well.
const KEPT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="null" sce:transition-function="emit">
  <state id="idle"><transition event="go" target="pass"/></state>
  <final id="pass"/>
</scxml>
"#;

/// [`KEPT`], written into `dir` as `kept.scxml`.
fn write_kept(dir: &Path) -> PathBuf {
    let doc = dir.join("kept.scxml");
    std::fs::write(&doc, KEPT).expect("write the document");
    doc
}

/// A statechart that declares nothing and runs, in its one state, an in-line
/// child declaring the function, with `child` as the child's states. Its
/// final state is `pass`, so it is a conformance fixture as well.
fn host_with_child(child: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="running" datamodel="ecmascript">
  <state id="running">
    <invoke type="scxml" id="lamp">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
               initial="off" datamodel="ecmascript" sce:transition-function="emit">
{child}
        </scxml>
      </content>
    </invoke>
    <transition event="stop" target="pass"/>
  </state>
  <final id="pass"/>
</scxml>
"#
    )
}

/// A child the function admits: two states that take each other.
const PURE_CHILD: &str = r#"          <state id="off"><transition event="on" target="lit"/></state>
          <state id="lit"><transition event="off" target="off"/></state>"#;

/// A child the function refuses once, for its `<parallel>`: every state is
/// reached and left, and every descriptor names an event.
const IMPURE_CHILD: &str = r#"          <state id="off"><transition event="split" target="both"/></state>
          <parallel id="both">
            <state id="r1"><transition event="join" target="off"/></state>
            <state id="r2"><transition event="join" target="off"/></state>
          </parallel>"#;

/// The synthesized name of the child [`host_with_child`] runs, in a document
/// whose file stem is `stem`.
fn child_stem(stem: &str) -> String {
    format!("{stem}__sce_synth_invoke__lamp")
}

/// The machine `stem` and the function written beside it in `dir`, named
/// after the same stem, carrying the hash the machine's header carries.
fn assert_function_beside(dir: &Path, stem: &str) {
    let machine = std::fs::read_to_string(dir.join(format!("{stem}_sm.rs")))
        .unwrap_or_else(|e| panic!("{stem}_sm.rs in {}: {e}", dir.display()));
    let function = std::fs::read_to_string(dir.join(format!("{stem}_transition_function.rs")))
        .unwrap_or_else(|e| panic!("no function beside {stem}_sm.rs in {}: {e}", dir.display()));
    assert_eq!(
        header_hash(&function),
        header_hash(&machine),
        "{stem}: the function and its machine name different source sets"
    );
}

/// An in-line child declaring the function is held to it as a root is, by
/// `check` and `generate` alike, and its refusal is placed where its author
/// wrote it: in the enclosing document, on the row of the `<parallel>`, not
/// in the child's own coordinates under the name the child is synthesized
/// as.
#[test]
fn an_inline_child_is_held_to_its_declaration_where_its_author_wrote_it() {
    let work = scratch("impure_child");
    let host = host_with_child(IMPURE_CHILD);
    let doc = work.join("host.scxml");
    std::fs::write(&doc, &host).expect("write the document");
    let doc_str = doc.to_str().unwrap();
    let out = work.join("out");
    let checked = refused_records(&["--error-format=json", "check", doc_str, "-l", "rust"]);
    let generated = refused_records(&[
        "--error-format=json",
        "generate",
        doc_str,
        "-l",
        "rust",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(
        sorted(generated),
        sorted(checked.clone()),
        "check and generate disagree about the child"
    );
    assert_eq!(
        codes_of(&checked),
        ["scxml/transition-function-rule"],
        "{checked:?}"
    );
    let record: serde_json::Value = serde_json::from_str(&checked[0]).expect("a record is JSON");
    let row = host
        .lines()
        .position(|line| line.contains("<parallel"))
        .expect("the child holds a parallel")
        + 1;
    assert_eq!(record["actual"], "parallel", "{record}");
    assert_eq!(
        record["location"]["file"], doc_str,
        "the child's refusal does not name the file its author wrote: {record}"
    );
    assert_eq!(
        record["location"]["line"], row,
        "the child's refusal is not on the row its author wrote: {record}"
    );
    assert!(
        !out.join("host_sm.rs").exists(),
        "a document whose child was refused generated a machine"
    );
}

/// `generate` writes an in-line child's function beside the child's machine,
/// named after the child's stem and bound to the hash the child's machine
/// carries.
#[test]
fn generate_writes_an_inline_childs_function_beside_its_machine() {
    let work = scratch("pure_child");
    let doc = work.join("host.scxml");
    std::fs::write(&doc, host_with_child(PURE_CHILD)).expect("write the document");
    let out = work.join("out");
    output_ok(
        codegen(&[
            "generate",
            "-l",
            "rust",
            "-o",
            out.to_str().unwrap(),
            doc.to_str().unwrap(),
        ]),
        "generate -l rust of the host",
    );
    assert_function_beside(&out, &child_stem("host"));
}

/// `orchestrate` writes the function beside the machine, bound to the hash
/// every file of the run carries.
#[test]
fn orchestrate_writes_the_function_beside_the_machine() {
    let work = scratch("orchestrate_function");
    let doc = write_kept(&work);
    let out = work.join("out");
    output_ok(
        codegen(&[
            "orchestrate",
            "--scxml",
            doc.to_str().unwrap(),
            "-l",
            "rust",
            "-o",
            out.to_str().unwrap(),
        ]),
        "orchestrate -l rust",
    );
    assert_function_beside(&out, "kept");
}

/// `check` on a set renders the function as `orchestrate` does, so it
/// accepts the declaration `orchestrate` generates: the set compile refuses
/// a declared function it is given no hash for.
#[test]
fn check_on_a_set_accepts_the_function_orchestrate_writes() {
    let work = scratch("check_set_function");
    let doc = write_kept(&work);
    let manifest = output_ok(
        codegen(&["check", "--scxml", doc.to_str().unwrap(), "-l", "rust"]),
        "check --scxml -l rust",
    );
    let manifest: serde_json::Value =
        serde_json::from_str(manifest.trim()).expect("check emits a manifest");
    let verdicts = manifest["languages"]
        .as_array()
        .expect("per-backend verdicts");
    assert_eq!(verdicts.len(), 1, "{manifest}");
    assert_eq!(verdicts[0]["language"], "rust", "{manifest}");
    assert_eq!(verdicts[0]["status"], "ok", "{manifest}");
}

/// The directory `generate-w3c -l rust` writes fixture `id` into under `out`.
fn w3c_rust_dir(out: &Path, id: &str) -> PathBuf {
    out.join("backends/rust/tests/src/generated")
        .join(format!("test{id}"))
}

/// `generate-w3c` writes a fixture's function beside its machine.
#[test]
fn generate_w3c_writes_a_fixtures_function_beside_its_machine() {
    let work = scratch("w3c_function");
    let suite = W3cSuite::stage(&work, "902", KEPT);
    let out = work.join("out");
    output_ok(codegen(&suite.args(&out)), "generate-w3c -l rust");
    assert_function_beside(&w3c_rust_dir(&out, "902"), "test902");
}

/// `generate-w3c` writes an in-line child's function beside the child's
/// machine, named after the child's stem.
#[test]
fn generate_w3c_writes_an_inline_childs_function_beside_its_machine() {
    let work = scratch("w3c_child_function");
    let suite = W3cSuite::stage(&work, "903", &host_with_child(PURE_CHILD));
    let out = work.join("out");
    output_ok(codegen(&suite.args(&out)), "generate-w3c -l rust");
    assert_function_beside(&w3c_rust_dir(&out, "903"), &child_stem("test903"));
}

/// The one refusal a library entry point with no source hash gives a
/// document whose function it cannot generate, and where it says to go.
fn assert_refused_for_want_of_a_hash(route: &str, refused: Result<(), sce_build::CompileError>) {
    let err = match refused {
        Ok(()) => panic!("{route}: the machine was generated without its function"),
        Err(err) => err,
    };
    let record = &err.to_diagnostics()[0];
    assert_eq!(
        record.code.as_str(),
        "generate/source-hash-unavailable",
        "{route}: {}",
        record.message
    );
    assert!(
        record.message.contains(route) && record.message.contains("sce-codegen generate"),
        "{route}: the refusal does not name the route and the command that generates it: {}",
        record.message
    );
}

/// The library's file entry points compute no source hash, so they refuse a
/// document declaring the function rather than return its machine alone.
#[test]
fn the_library_file_entries_refuse_a_function_they_cannot_bind() {
    let work = scratch("library_file_unbound");
    let doc = write_kept(&work);
    let doc_str = doc.to_str().unwrap();
    let template_dir = sce_build::find_template_dir_for(Language::Rust);
    let route = "the library's file entry points";
    assert_refused_for_want_of_a_hash(
        route,
        sce_build::compile_scxml_lang_typed(doc_str, &template_dir, Language::Rust).map(|_| ()),
    );
    assert_refused_for_want_of_a_hash(
        route,
        sce_build::compile_scxml_to_string_typed(doc_str, &template_dir).map(|_| ()),
    );
}

/// The in-memory entry points, the WASM build's among them, have no source
/// set to hash, and refuse the declaration the same way.
#[test]
fn the_library_in_memory_entries_refuse_a_function_they_cannot_bind() {
    let templates = sce_build::template_registry::embedded_templates_for(Language::Rust);
    let route = "the library's in-memory entry points";
    assert_refused_for_want_of_a_hash(
        route,
        sce_build::compile_from_string_lang_typed(KEPT, "kept", &templates, Language::Rust)
            .map(|_| ()),
    );
    assert_refused_for_want_of_a_hash(
        route,
        sce_build::compile_from_string_typed(KEPT, "kept", &templates).map(|_| ()),
    );
}

/// The set compile, given no hash, refuses the declaration and names the
/// option that would have carried one.
#[test]
fn the_set_compile_refuses_a_function_it_is_given_no_hash_for() {
    let work = scratch("set_compile_unbound");
    let doc = write_kept(&work);
    let template_dir = sce_build::find_template_dir_for(Language::Rust);
    assert_refused_for_want_of_a_hash(
        "ForgeCompileOptions::source_hash",
        sce_build::compile_scxml_with_imports(
            &[doc.as_path()],
            &[],
            &template_dir,
            Language::Rust,
            &sce_build::ForgeCompileOptions::default(),
            None,
        )
        .map(|_| ()),
    );
}

/// [`KEPT`] declaring its transition table instead of its function: the
/// other output that carries the run's source hash.
fn kept_table() -> String {
    KEPT.replace(
        r#"sce:transition-function="emit""#,
        r#"sce:transition-table="emit""#,
    )
}

/// The routes with no source hash refuse a document declaring its
/// transition table as they refuse one declaring its function: one
/// predicate decides both, so a machine is never returned without the table
/// its document asked for.
#[test]
fn the_routes_without_a_hash_refuse_a_declared_table_too() {
    let work = scratch("table_unbound");
    let text = kept_table();
    assert_ne!(text, KEPT, "the document must declare the table");
    let doc = work.join("kept.scxml");
    std::fs::write(&doc, &text).expect("write the document");
    let doc_str = doc.to_str().unwrap();
    let template_dir = sce_build::find_template_dir_for(Language::Rust);
    let templates = sce_build::template_registry::embedded_templates_for(Language::Rust);
    assert_refused_for_want_of_a_hash(
        "the library's file entry points",
        sce_build::compile_scxml_lang_typed(doc_str, &template_dir, Language::Rust).map(|_| ()),
    );
    assert_refused_for_want_of_a_hash(
        "the library's in-memory entry points",
        sce_build::compile_from_string_lang_typed(&text, "kept", &templates, Language::Rust)
            .map(|_| ()),
    );
    assert_refused_for_want_of_a_hash(
        "ForgeCompileOptions::source_hash",
        sce_build::compile_scxml_with_imports(
            &[doc.as_path()],
            &[],
            &template_dir,
            Language::Rust,
            &sce_build::ForgeCompileOptions::default(),
            None,
        )
        .map(|_| ()),
    );
}

/// A document declaring both outputs is refused once, naming both.
#[test]
fn a_refusal_names_every_declaration_left_unbound() {
    let text = KEPT.replace(
        r#"sce:transition-function="emit""#,
        r#"sce:transition-table="emit" sce:transition-function="emit""#,
    );
    let templates = sce_build::template_registry::embedded_templates_for(Language::Rust);
    let err = match sce_build::compile_from_string_lang_typed(
        &text,
        "kept",
        &templates,
        Language::Rust,
    ) {
        Ok(_) => panic!("a document declaring both outputs was generated without them"),
        Err(err) => err,
    };
    let record = &err.to_diagnostics()[0];
    assert_eq!(record.code.as_str(), "generate/source-hash-unavailable");
    assert!(
        record
            .message
            .contains(r#"sce:transition-table="emit" and sce:transition-function="emit""#),
        "{}",
        record.message
    );
}

/// The Rust module index of `text`, generated as `kept`, bound to a digest
/// when `bind` is set as a route that writes drift-headed files binds it.
fn module_index(text: &str, bind: bool) -> String {
    let mut model = SCXMLParser::new()
        .parse_string(text, "kept")
        .expect("the document parses");
    sce_build::analyzer::analyze(&mut model, "kept.scxml");
    if bind {
        sce_build::bind_run_digest(&mut model, &"ef".repeat(32));
    }
    sce_build::generator::generate_rust_module_index(
        &model,
        &sce_build::find_template_dir_for(Language::Rust),
        "kept",
    )
    .expect("the module index renders")
}

/// The Rust module index names a bound function's module beside the
/// machine's, and adds nothing for a document that declares no function or
/// whose function no route bound.
#[test]
fn the_rust_module_index_declares_a_bound_function_and_nothing_else() {
    let machine_only = "pub use kept_sm::*;";
    let undeclared = module_index(
        &KEPT.replace(r#" sce:transition-function="emit""#, ""),
        true,
    );
    assert!(
        undeclared.trim_end().ends_with(machine_only)
            && !undeclared.contains("transition_function"),
        "{undeclared}"
    );
    let unbound = module_index(KEPT, false);
    assert!(
        unbound.trim_end().ends_with(machine_only) && !unbound.contains("transition_function"),
        "{unbound}"
    );
    let bound = module_index(KEPT, true);
    assert!(
        bound
            .trim_end()
            .ends_with("pub use kept_sm::*;\npub mod kept_transition_function;"),
        "{bound}"
    );
}

/// `generate-w3c` declares a fixture's function module in the fixture's
/// `mod.rs`, so the W3C Rust suite compiles the file it wrote.
#[test]
fn generate_w3c_declares_a_fixtures_function_in_its_module_index() {
    let work = scratch("w3c_function_index");
    let suite = W3cSuite::stage(&work, "904", KEPT);
    let out = work.join("out");
    output_ok(codegen(&suite.args(&out)), "generate-w3c -l rust");
    let index = std::fs::read_to_string(w3c_rust_dir(&out, "904").join("mod.rs"))
        .expect("read the fixture's mod.rs");
    assert!(
        index.contains("pub mod test904_transition_function;"),
        "{index}"
    );
}

/// `generate-w3c` declares an in-line child's function module in the
/// fixture's `mod.rs`, beside the child's machine.
#[test]
fn generate_w3c_declares_an_inline_childs_function_in_its_module_index() {
    let work = scratch("w3c_child_function_index");
    let suite = W3cSuite::stage(&work, "905", &host_with_child(PURE_CHILD));
    let out = work.join("out");
    output_ok(codegen(&suite.args(&out)), "generate-w3c -l rust");
    let index = std::fs::read_to_string(w3c_rust_dir(&out, "905").join("mod.rs"))
        .expect("read the fixture's mod.rs");
    let child = child_stem("test905");
    assert!(
        index.contains(&format!("mod {child}_sm;"))
            && index.contains(&format!("pub mod {child}_transition_function;")),
        "{index}"
    );
}

/// Every backend's template reads the function the producer built, and nothing
/// else: no line between its markers walks the model's states, transitions or
/// events.
#[test]
fn every_template_reads_the_function_and_not_the_model() {
    for template in TEMPLATES {
        let text = read(&repo_root().join("tools/codegen/templates").join(template));
        let start = text
            .find("transition-function:begin")
            .unwrap_or_else(|| panic!("{template}: the template marks its block"));
        let end = text
            .find("transition-function:end")
            .unwrap_or_else(|| panic!("{template}: the block has an end marker"));
        let block = &text[start..end];
        for forbidden in [
            "model.states",
            "model.events",
            "model.initial",
            "model.history_states",
            ".transitions",
            "model.emitted_transition_table",
        ] {
            assert!(
                !block.contains(forbidden),
                "{template}: reads `{forbidden}` from the model instead of the function"
            );
        }
        assert!(
            block.contains("model.emitted_transition_function"),
            "{template}: does not read the function"
        );
    }
}
