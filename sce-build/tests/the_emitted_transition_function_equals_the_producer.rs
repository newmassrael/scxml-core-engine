// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A document that declares `sce:transition-function="emit"` is generated with a
// pure function of (current state, event) beside its machine (SCE Accepted
// Subset 2.21, docs/SCE_TRANSITION_FUNCTION.md). The function is computed once,
// by `sce_build::model::TransitionFunction::of`, and this file holds the two
// things that depend on it to it:
//
// - the Rust file `generate` writes, compiled ALONE — in a `#![no_std]` crate
//   with no dependency, which is what proves it names no SCE crate and needs no
//   allocator — and then run, answering every (state, event) pair, every
//   configuration, every name and the hash exactly as the producer's table;
// - the machine `generate` writes beside it, built against the real Rust
//   runtime and driven to every state of the function by the shortest event
//   path the function itself gives, and sent every event there: the
//   configuration the engine reaches, and whether it took or discarded the
//   event, is what the function says. That is what makes the selection the
//   generator does at build time and the one the engine does at run time one
//   fact rather than two that happen to agree.
//
// The compiling checks run where a toolchain is found; the rest run anywhere.
// The other five backends refuse the declaration by name, and the declaration,
// its one value, the forge root that does not take it, and the refusals it
// brings are held at the command line.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use regex::Regex;
use sce_build::filters;
use sce_build::model::TransitionFunction;
use sce_build::parser::SCXMLParser;
use sce_build::toolchain;

/// The two fixtures the function is held on: a flat cycle, and a nested
/// lifecycle that reaches every selection rule.
const FIXTURES: [&str; 2] = ["flat_lifecycle", "nested_lifecycle"];

/// The five backends that do not generate the function yet.
const REFUSING: [&str; 5] = ["cpp", "c11", "go", "kotlin", "python"];

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent dir")
        .to_path_buf()
}

fn fixture(stem: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/transition_function")
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

/// `generate -l rust` of `doc` into `out`.
fn generate_rust(doc: &Path, out: &Path) {
    output_ok(
        codegen(&[
            "generate",
            "-l",
            "rust",
            "-o",
            out.to_str().unwrap(),
            doc.to_str().unwrap(),
        ]),
        "generate -l rust",
    );
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

/// The producer's function of the fixture, carrying `hash`.
fn produced(doc: &Path, hash: &str) -> TransitionFunction {
    let mut model = SCXMLParser::new()
        .parse_file(doc.to_str().unwrap())
        .expect("the fixture parses");
    sce_build::analyzer::analyze(&mut model, doc.to_str().unwrap());
    TransitionFunction::of(&model, hash.to_string())
}

/// What the generated function must print, from the producer's value: the
/// same lines the check program below writes from the compiled one.
fn listing(function: &TransitionFunction) -> String {
    let mut out = String::new();
    out.push_str(&format!("hash\t{}\n", function.source_hash));
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

/// Both fixtures are accepted and generated for Rust: the machine, and the
/// function beside it carrying the hash of its own header.
#[test]
fn every_fixture_generates_its_machine_and_its_function() {
    for stem in FIXTURES {
        let out = scratch(&format!("generated_{stem}"));
        generate_rust(&fixture(stem), &out);
        let machine = out.join(format!("{stem}_sm.rs"));
        let function = out.join(format!("{stem}_transition_function.rs"));
        assert!(machine.is_file(), "[{stem}] no machine was written");
        let source = std::fs::read_to_string(&function)
            .unwrap_or_else(|e| panic!("[{stem}] no function was written: {e}"));
        let hash = header_hash(&source);
        assert!(
            source.contains(&format!("pub const SOURCE_HASH: &str = \"{hash}\";")),
            "[{stem}] the function does not carry the hash of its own header"
        );
        assert_eq!(
            header_hash(&std::fs::read_to_string(&machine).expect("read the machine")),
            hash,
            "[{stem}] the machine and the function name different source sets"
        );
    }
}

/// The table the generated source holds is the producer's, pair for pair, read
/// back from the text — so this runs on a host with no toolchain.
#[test]
fn the_generated_source_holds_the_producers_table() {
    for stem in FIXTURES {
        let out = scratch(&format!("source_{stem}"));
        generate_rust(&fixture(stem), &out);
        let source = std::fs::read_to_string(out.join(format!("{stem}_transition_function.rs")))
            .expect("read the function");
        let function = produced(&fixture(stem), &header_hash(&source));
        let arm = Regex::new(r"\(State::(\w+), Event::(\w+)\) => Some\(State::(\w+)\),").unwrap();
        let mut written: Vec<(String, String, String)> = arm
            .captures_iter(&source)
            .map(|c| (c[1].to_string(), c[2].to_string(), c[3].to_string()))
            .collect();
        written.sort();
        let mut expected: Vec<(String, String, String)> = function
            .states
            .iter()
            .flat_map(|state| {
                state.next.iter().filter_map(move |cell| {
                    cell.to.as_ref().map(|to| {
                        (
                            filters::to_state_variant(state.id.clone()),
                            filters::to_event_variant(cell.event.clone()),
                            filters::to_state_variant(to.clone()),
                        )
                    })
                })
            })
            .collect();
        expected.sort();
        assert!(!expected.is_empty(), "[{stem}] the producer takes no pair");
        assert_eq!(
            written, expected,
            "[{stem}] the generated arms are not the producer's"
        );
        assert_eq!(
            source.contains("        _ => None,\n    }\n}"),
            !function.every_pair_taken,
            "[{stem}] the refusing arm is there exactly when some pair is refused"
        );
    }
}

/// The nested fixture reaches every selection rule the producer's unit tests
/// name, so the table it yields is stated here in full: a change to the
/// producer that moves one pair moves this.
#[test]
fn the_nested_fixture_yields_the_table_its_rules_give() {
    let function = produced(&fixture("nested_lifecycle"), &"0".repeat(64));
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
        generate_rust(&fixture(stem), &out);
        let function_file = out.join(format!("{stem}_transition_function.rs"));
        let source = std::fs::read_to_string(&function_file).expect("read the function");

        // No crate but `core`: a `#![no_std]` library with no `--extern`.
        let library = work.join("alone.rs");
        std::fs::write(
            &library,
            format!(
                "#![no_std]\n#[path = {:?}]\npub mod tf;\n",
                function_file.to_str().unwrap()
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
        std::fs::write(&program, listing_program(&function_file)).expect("write the program");
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
        let function = produced(&fixture(stem), &header_hash(&source));
        assert_eq!(
            printed,
            listing(&function),
            "[{stem}] the compiled function does not answer as the producer"
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
        generate_rust(&fixture(stem), &out);
        let source = std::fs::read_to_string(out.join(format!("{stem}_transition_function.rs")))
            .expect("read the function");
        let function = produced(&fixture(stem), &header_hash(&source));

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

/// A document that does not declare the function gets no file for it.
#[test]
fn an_undeclared_document_is_generated_without_a_function() {
    let work = scratch("undeclared");
    let text = std::fs::read_to_string(fixture("flat_lifecycle")).expect("read the fixture");
    let without = text.replace("\n       sce:transition-function=\"emit\"", "");
    assert_ne!(
        without, text,
        "the fixture must differ by the declaration alone"
    );
    let doc = work.join("flat_lifecycle.scxml");
    std::fs::write(&doc, without).expect("write the document");
    let out = work.join("out");
    generate_rust(&doc, &out);
    assert!(out.join("flat_lifecycle_sm.rs").is_file());
    assert!(
        !out.join("flat_lifecycle_transition_function.rs").exists(),
        "a document that declares no function was generated with one"
    );
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

/// A forge root does not take the declaration: no reader of a forge kind asks
/// for it, so it is refused as an attribute nothing reads.
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
        ["validation/sce-attribute-unread"],
        "{stderr}"
    );
    assert!(stderr.contains("sce:transition-function"), "{stderr}");
}

/// The five backends that do not generate the function refuse the declaration,
/// naming the language, rather than writing the machine without it: `generate`
/// fails, `check` naming the backend fails the same way, and a `check` sweep
/// reports each of them rejected beside Rust's `ok`.
#[test]
fn every_other_backend_refuses_the_declaration_by_name() {
    let doc = fixture("flat_lifecycle");
    let doc_str = doc.to_str().unwrap();
    for lang in REFUSING {
        let out = scratch(&format!("refused_{lang}"));
        for args in [
            vec![
                "--error-format=json",
                "generate",
                doc_str,
                "-l",
                lang,
                "-o",
                out.to_str().unwrap(),
            ],
            vec!["--error-format=json", "check", doc_str, "-l", lang],
        ] {
            let refused = codegen(&args);
            assert!(
                !refused.status.success(),
                "{args:?}: the declaration was not refused"
            );
            let stderr = String::from_utf8_lossy(&refused.stderr);
            assert_eq!(
                wire_codes(&stderr),
                ["generate/unsupported-feature"],
                "{stderr}"
            );
            assert!(
                stderr.contains("sce:transition-function") && stderr.contains(lang),
                "{args:?}: the refusal does not name the declaration and the language:\n{stderr}"
            );
        }
        assert!(
            !out.join("flat_lifecycle_transition_function.rs").exists(),
            "[{lang}] a refused backend wrote a function"
        );
    }

    let swept = codegen(&["check", doc_str]);
    let manifest: serde_json::Value = serde_json::from_str(output_ok(swept, "check sweep").trim())
        .expect("the sweep emits a manifest");
    let verdicts = manifest["languages"]
        .as_array()
        .expect("per-backend verdicts");
    for verdict in verdicts {
        let language = verdict["language"].as_str().expect("language");
        if language == "rust" {
            assert_eq!(verdict["status"], "ok", "{verdict}");
        } else {
            assert_eq!(verdict["status"], "rejected", "{verdict}");
            assert_eq!(verdict["code"], "generate/unsupported-feature", "{verdict}");
        }
    }
    assert_eq!(verdicts.len(), 1 + REFUSING.len(), "{manifest}");
}

/// Every refusal the declaration brings comes in one run, and `check` and
/// `generate` report the same records.
#[test]
fn check_and_generate_report_every_refusal_at_once() {
    let work = scratch("impure");
    let doc = work.join("impure.scxml");
    std::fs::write(
        &doc,
        r#"<?xml version="1.0" encoding="UTF-8"?>
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
"#,
    )
    .expect("write the document");
    let doc_str = doc.to_str().unwrap();
    let out = work.join("out");
    let mut reported: Vec<Vec<String>> = Vec::new();
    for args in [
        vec!["--error-format=json", "check", doc_str, "-l", "rust"],
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
            "{args:?}: an impure declaration passed"
        );
        let mut codes = wire_codes(&String::from_utf8_lossy(&refused.stderr));
        codes.sort();
        reported.push(codes);
    }
    assert_eq!(reported[0], reported[1], "check and generate disagree");
    assert_eq!(
        reported[0],
        [
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
        ]
    );
    assert!(
        !out.join("impure_sm.rs").exists(),
        "a refused document generated a machine"
    );
}

/// The template reads the function the producer built, and nothing else: no
/// line between its markers walks the model's states, transitions or events.
#[test]
fn the_template_reads_the_function_and_not_the_model() {
    let text = std::fs::read_to_string(
        repo_root().join("tools/codegen/templates/rust/transition_function.rs.jinja2"),
    )
    .expect("read the template");
    let start = text
        .find("transition-function:begin")
        .expect("the template marks its block");
    let end = text
        .find("transition-function:end")
        .expect("the block has an end marker");
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
            "the template reads `{forbidden}` from the model instead of the function"
        );
    }
    assert!(block.contains("model.emitted_transition_function"));
}
