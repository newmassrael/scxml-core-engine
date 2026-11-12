// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The transition table a document declares (`sce:transition-table="emit"`,
// docs/SCE_TRANSITION_TABLE.md) is constant data nothing in the machine reads, and
// the claim made for it is that it costs a program nothing unless the program
// names it: a program that drives the machine and never names the table carries
// none of it. This file measures that, on what a build produces, for the three
// backends whose output is compiled ahead of time into an image:
//
// - C: the machine's `.c` and a program that drives it, linked with
//   `-Os -ffunction-sections -fdata-sections -Wl,--gc-sections`
//   (`-Wl,-dead_strip` on macOS, whose linker spells it so);
// - C++: the object of a unit that drives the machine, compiled with
//   `-Os -ffunction-sections -fdata-sections`. The C++ machine is linked against
//   `sce_base`, which this file does not build, so the object is what is
//   measured: no other part of a program includes the generated header, so an
//   image is free of the table when every object of it is;
// - Rust: a crate that drives the machine, built with `cargo build --release`
//   and nothing else (no LTO).
//
// "None of it" is read on the bytes of the image: neither the transition's
// annotation, which the document makes distinctive and which nothing but the
// table holds, nor the source hash, which nothing but the table's constant
// holds. Each measurement has a control beside it, built the same way from a
// program that does name the table and in which both are found, so the
// measurement is one that can fail.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use regex::Regex;
use sce_build::filters;
use sce_build::toolchain;

/// The machine's name: the file stem, which every backend derives its names from.
const STEM: &str = "ttimage";

/// The annotation of the one transition: a token nothing else in a build of this
/// machine spells, so finding it in an image means finding the table.
const ANNOTATION: &str = "TTIMAGE-ANNOTATION-q7Zx3w";

/// The document: one transition, carrying the annotation, in a machine that
/// declares its table.
fn document() -> String {
    format!(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" sce:transition-table="emit"
       version="1.0" name="{STEM}" initial="idle" datamodel="null">
  <state id="idle">
    <transition event="go" target="done" sce:req="{ANNOTATION}"/>
  </state>
  <final id="done"/>
</scxml>
"#
    )
}

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent dir")
        .to_path_buf()
}

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("transition_table_image")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch");
    dir
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

/// Run `command`, failing with everything it printed.
fn run_ok(mut command: Command, what: &str) -> String {
    let output = command
        .output()
        .unwrap_or_else(|e| panic!("run {what}: {e}"));
    output_ok(output, what)
}

/// Generate the document in `lang` into `work/out`, and answer the output folder
/// and the source hash the machine's files carry in their header. `machine_files`
/// are the files of the machine that hold its table between them.
fn generate(lang: &str, work: &Path, machine_files: &[&str]) -> (PathBuf, String) {
    let doc = work.join(format!("{STEM}.scxml"));
    std::fs::write(&doc, document()).expect("write the document");
    let out = work.join("out");
    let mut command = Command::new(sce_codegen_bin());
    command
        .env("SCE_WORKSPACE_ROOT", repo_root())
        .args(["generate", "-l", lang, "-o"])
        .arg(&out)
        .arg(&doc);
    run_ok(command, &format!("generate -l {lang}"));
    let source = machine_files
        .iter()
        .map(|file| {
            std::fs::read_to_string(out.join(format!("{STEM}{file}")))
                .unwrap_or_else(|e| panic!("[{lang}] read {STEM}{file}: {e}"))
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        source.contains(ANNOTATION),
        "[{lang}] the generated source does not carry the annotation, so its absence from an image would prove nothing"
    );
    let hash = Regex::new(r"source-hash:\s*([0-9a-f]{64})")
        .unwrap()
        .captures(&source)
        .map(|c| c[1].to_string())
        .unwrap_or_else(|| panic!("[{lang}] the machine file names no source hash"));
    assert!(
        source.contains(&format!("\"{hash}\"")),
        "[{lang}] the table does not hold the source hash as a string constant"
    );
    (out, hash)
}

/// Whether `image` holds the bytes of `text`.
fn holds(image: &Path, text: &str) -> bool {
    let bytes = std::fs::read(image).unwrap_or_else(|e| panic!("read {}: {e}", image.display()));
    bytes
        .windows(text.len())
        .any(|window| window == text.as_bytes())
}

/// The measured image holds neither the annotation nor the hash, and the control,
/// built the same way from a program that names the table, holds both.
fn assert_measured(lang: &str, measured: &Path, control: &Path, hash: &str) {
    assert!(
        holds(control, ANNOTATION) && holds(control, hash),
        "[{lang}] the control, which names the table, does not hold its annotation and its hash \
         ({}), so the measurement below could not have failed",
        control.display()
    );
    assert!(
        !holds(measured, ANNOTATION),
        "[{lang}] a program that never names the transition table carries its annotation ({})",
        measured.display()
    );
    assert!(
        !holds(measured, hash),
        "[{lang}] a program that never names the transition table carries its source hash ({})",
        measured.display()
    );
}

/// The linker flag that drops every section nothing references.
fn gc_sections() -> &'static str {
    if cfg!(target_os = "macos") {
        "-Wl,-dead_strip"
    } else {
        "-Wl,--gc-sections"
    }
}

/// A C program that drives the machine to its final state, and never names the
/// table.
fn c_drive() -> String {
    let upper = STEM.to_uppercase();
    let event = filters::to_c11_upper_ident("go".into());
    format!(
        "#include \"{STEM}_sm.h\"\n#include <string.h>\n\n\
         int main(void) {{\n\
         \x20   {STEM}_t sm;\n\
         \x20   {STEM}_event_with_meta_t carrier;\n\
         \x20   if ({STEM}_init(&sm) != {upper}_START_OK) {{\n\
         \x20       return 1;\n\
         \x20   }}\n\
         \x20   memset(&carrier, 0, sizeof carrier);\n\
         \x20   carrier.event = {upper}_EVENT_{event};\n\
         \x20   {STEM}_raise_external(&sm, &carrier);\n\
         \x20   {STEM}_step(&sm);\n\
         \x20   int ended = {STEM}_is_in_final_state(&sm);\n\
         \x20   {STEM}_destroy(&sm);\n\
         \x20   return ended ? 0 : 1;\n\
         }}\n"
    )
}

/// A C program that names the table: it prints the annotation and the hash.
fn c_named() -> String {
    let upper = STEM.to_uppercase();
    format!(
        "#include \"{STEM}_sm.h\"\n#include <stdio.h>\n\n\
         int main(void) {{\n\
         \x20   const {STEM}_transition_t *t = {STEM}_transitions();\n\
         \x20   if (t == NULL) {{\n        return 1;\n    }}\n\
         \x20   puts(t[0].annotation);\n\
         \x20   puts({upper}_SOURCE_HASH);\n\
         \x20   return 0;\n\
         }}\n"
    )
}

/// Build `program` with the machine's `.c` under the size discipline, and answer
/// the image.
fn build_c(cc: &Path, work: &Path, out: &Path, name: &str, program: &str) -> PathBuf {
    let main = work.join(format!("{name}.c"));
    std::fs::write(&main, program).expect("write the program");
    let image = work.join(name);
    let mut compile = Command::new(cc);
    compile
        .args([
            "-std=c11",
            "-Os",
            "-ffunction-sections",
            "-fdata-sections",
            gc_sections(),
            "-I",
        ])
        .arg(repo_root().join("backends/c/runtime/include"))
        .arg("-I")
        .arg(out)
        .arg(&main)
        .arg(out.join(format!("{STEM}_sm.c")))
        .arg("-o")
        .arg(&image);
    run_ok(
        compile,
        &format!("cc over the machine and the {name} program"),
    );
    image
}

/// A C program that drives the machine and never calls
/// `<machine>_transitions()` links to an image that holds none of the table.
#[test]
fn a_c_program_that_never_names_the_table_links_none_of_it() {
    let Some(cc) = toolchain::locate_any(&["gcc", "cc"]) else {
        toolchain::skipped(
            "a_c_program_that_never_names_the_table_links_none_of_it: gcc/cc not on PATH",
        );
        return;
    };
    let work = scratch("c");
    let (out, hash) = generate("c11", &work, &["_sm.h", "_sm.c"]);

    let measured = build_c(&cc, &work, &out, "drive", &c_drive());
    run_ok(
        Command::new(&measured),
        "the C program that drives the machine to its final state",
    );
    let control = build_c(&cc, &work, &out, "named", &c_named());
    let printed = run_ok(Command::new(&control), "the C program that names the table");
    assert!(
        printed.contains(ANNOTATION) && printed.contains(&hash),
        "the C control printed what the table does not hold:\n{printed}"
    );
    assert_measured("c11", &measured, &control, &hash);
}

/// Compile one C++ unit over the generated header under the size discipline, and
/// answer its object.
fn compile_cpp(gpp: &Path, work: &Path, out: &Path, name: &str, unit: &str) -> PathBuf {
    let source = work.join(format!("{name}.cpp"));
    std::fs::write(&source, unit).expect("write the unit");
    let object = work.join(format!("{name}.o"));
    let mut compile = Command::new(gpp);
    compile.args([
        "-std=c++20",
        "-Os",
        "-ffunction-sections",
        "-fdata-sections",
        "-c",
    ]);
    for include in common::cpp_engine::engine_include_dirs()
        .into_iter()
        .chain([
            repo_root().join("third_party/spdlog/include"),
            out.to_path_buf(),
        ])
    {
        compile.arg("-I").arg(include);
    }
    compile.arg(&source).arg("-o").arg(&object);
    run_ok(compile, &format!("g++ over the {name} unit"));
    object
}

/// A C++ unit that drives the machine and never names `Policy::transitions` or
/// `Policy::sourceHash()` compiles to an object that holds none of the table: the
/// table is an inline variable, emitted by a unit that uses it and by no other.
#[test]
fn a_cpp_unit_that_never_names_the_table_compiles_none_of_it() {
    let Some(gpp) = toolchain::locate("g++") else {
        toolchain::skipped(
            "a_cpp_unit_that_never_names_the_table_compiles_none_of_it: g++ not on PATH",
        );
        return;
    };
    let work = scratch("cpp");
    let (out, hash) = generate("cpp", &work, &["_sm.h", "_sm.inl"]);
    let event = filters::to_cpp_event_variant("go".into());

    let measured = compile_cpp(
        &gpp,
        &work,
        &out,
        "drive",
        &format!(
            "#include \"{STEM}_sm.h\"\n\n\
             using Machine = SCE::Generated::{STEM}::{STEM};\n\n\
             int main() {{\n\
             \x20   Machine sm;\n\
             \x20   if (sm.initialize()) {{\n\
             \x20       return 2;\n\
             \x20   }}\n\
             \x20   sm.processEvent(Machine::Event::{event});\n\
             \x20   return sm.isInFinalState() ? 0 : 1;\n\
             }}\n"
        ),
    );
    let control = compile_cpp(
        &gpp,
        &work,
        &out,
        "named",
        &format!(
            "#include \"{STEM}_sm.h\"\n#include <cstdio>\n\n\
             using P = SCE::Generated::{STEM}::{STEM}Policy;\n\n\
             int main() {{\n\
             \x20   std::puts(P::transitions[0].annotation);\n\
             \x20   std::puts(P::sourceHash());\n\
             \x20   return 0;\n\
             }}\n"
        ),
    );
    assert_measured("cpp", &measured, &control, &hash);
}

/// A Rust crate that drives the machine and never names `<Machine>Transition`
/// builds to an image that holds none of the table: `ALL` and `SOURCE_HASH` are
/// constants, evaluated where they are named and stored nowhere else.
#[test]
fn a_rust_program_that_never_names_the_table_builds_none_of_it() {
    let Some(cargo) = toolchain::locate("cargo") else {
        toolchain::skipped(
            "a_rust_program_that_never_names_the_table_builds_none_of_it: cargo not on PATH",
        );
        return;
    };
    let work = scratch("rust");
    let (out, hash) = generate("rust", &work, &["_sm.rs"]);
    let machine = filters::to_pascal_case(STEM.to_string());
    let module = format!(
        "#[path = {:?}]\nmod machine;\n\n",
        out.join(format!("{STEM}_sm.rs")).to_str().unwrap()
    );

    let project = work.join("project");
    std::fs::create_dir_all(project.join("src/bin")).expect("create project");
    std::fs::write(
        project.join("Cargo.toml"),
        format!(
            "[package]\nname = \"ttimage-check\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\n\n\
             [dependencies]\nsce-rust-runtime = {{ path = {:?} }}\n\n[workspace]\n",
            repo_root().join("backends/rust/runtime").to_str().unwrap()
        ),
    )
    .expect("write manifest");
    std::fs::write(
        project.join("src/bin/drive.rs"),
        format!(
            "{module}fn main() {{\n\
             \x20   let mut engine = sce_rust_runtime::Engine::new(machine::{machine}Policy::new());\n\
             \x20   let Ok(()) = engine.initialize();\n\
             \x20   engine.raise_external_by_name(\"go\", \"\");\n\
             \x20   engine.step();\n\
             \x20   std::process::exit(if engine.is_in_final_state() {{ 0 }} else {{ 1 }});\n\
             }}\n"
        ),
    )
    .expect("write the drive");
    std::fs::write(
        project.join("src/bin/named.rs"),
        format!(
            "{module}fn main() {{\n\
             \x20   println!(\"{{}}\", machine::{machine}Transition::ALL[0].annotation);\n\
             \x20   println!(\"{{}}\", machine::{machine}Transition::SOURCE_HASH);\n\
             }}\n"
        ),
    )
    .expect("write the control");
    // The repo's own lock, so the runtime's dependencies resolve to the versions
    // already fetched and the build needs no network.
    std::fs::copy(repo_root().join("Cargo.lock"), project.join("Cargo.lock"))
        .expect("copy the lock file");

    let manifest = project.join("Cargo.toml");
    let target_dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("transition_table_image_rust_target");
    let build = common::run_cargo_offline_first(|| {
        let mut build = Command::new(&cargo);
        build
            .args(["build", "--release", "--quiet", "--manifest-path"])
            .arg(&manifest)
            .env("CARGO_TARGET_DIR", &target_dir);
        build
    });
    output_ok(
        build.output,
        "cargo build --release over the drive and the control",
    );

    let release = target_dir.join("release");
    let measured = release.join(format!("drive{}", std::env::consts::EXE_SUFFIX));
    let control = release.join(format!("named{}", std::env::consts::EXE_SUFFIX));
    run_ok(
        Command::new(&measured),
        "the Rust program that drives the machine to its final state",
    );
    let printed = run_ok(
        Command::new(&control),
        "the Rust program that names the table",
    );
    assert!(
        printed.contains(ANNOTATION) && printed.contains(&hash),
        "the Rust control printed what the table does not hold:\n{printed}"
    );
    assert_measured("rust", &measured, &control, &hash);
}
