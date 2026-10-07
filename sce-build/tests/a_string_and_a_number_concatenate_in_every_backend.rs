// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! `'E' + (7300 + n)` is a string in every backend, as it is in ECMAScript.
//!
//! ECMA-262 13.15.3 makes `+` a concatenation as soon as one operand is a
//! string, whatever the other turns out to be. The Lua emitter has always
//! honoured it (`..`, or `_scxml_add` while the types are open); the typed
//! Forge emitters did not. Type inference joined the two operands as numbers,
//! a string had no place in that lattice, the sum came out `Unknown`, and each
//! emitter then wrote the operator between its operands as written:
//!
//! * C and C++ spelled `"E" + (7300 + n)`, which adds to a `const char *`: it
//!   compiles, and at run time reads whatever lies that many bytes past the
//!   literal -- an event identifier that is a fragment of an unrelated string;
//! * Rust and Go spelled the same text and were refused by their compilers;
//! * Python spelled it and raised `TypeError` when the line first ran;
//! * Kotlin alone happened to concatenate, because `String.plus` takes `Any`.
//!
//! One document, six answers, four of them wrong and only one of those wrong
//! answers visible before the value is used. A generator whose promise is that
//! a document means the same in every backend cannot leave this to the
//! language each backend lands in.
//!
//! # What is measured
//!
//! * Every backend in [`Language::ALL`] generates the document, and no
//!   generated line adds a number to a string literal as written. The set of
//!   backends is derived, so a seventh one is asked the same question without
//!   anyone remembering to add it here.
//! * Each backend that lowers it spells the integer's decimal digits the way
//!   its language does, and the pieces are in the order they were written.
//! * A string held in a conditional is parenthesised where the language's
//!   conditional reaches rightwards, so `(c ? a : b) + n` stays that and does
//!   not become `c ? a : b + n`.
//! * An operand whose text differs between languages -- a float, a boolean --
//!   is refused, by the construct, in every backend; it is not left to what the
//!   target language prints.
//! * C has nowhere to put the joined text and says so, rather than generate the
//!   pointer addition this file exists to prevent.
//! * Where the target toolchain is present, the generated code is RUN and the
//!   value it returns is `E7301`, not merely well formed.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::generator::Language;
use sce_build::{compile_forge_with_imports, DocumentLabel, ForgeCompileOptions};

fn resource_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources")
}

/// A synthetic algorithm: nothing in it comes from a product. The sum is
/// parenthesised on purpose -- it is the shape an identifier built from a
/// base and an index takes, and the one that read memory in C++.
const DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm"
       version="1.0">
  <sce:signature>
    <sce:param name="n" type="int32"/>
    <sce:return type="string"/>
  </sce:signature>
  <sce:body>
    <sce:var name="r" type="string" init="''"/>
    <sce:if cond="n >= 1 &amp;&amp; n &lt;= 66">
      <sce:assign target="r" expr="'E' + (7300 + n)"/>
    </sce:if>
    <sce:return expr="r"/>
  </sce:body>
</scxml>
"#;

/// The same, with the string held in a conditional on the LEFT of the `+`.
const CONDITIONAL_LEFT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm"
       version="1.0">
  <sce:signature>
    <sce:param name="n" type="int32"/>
    <sce:param name="flag" type="bool"/>
    <sce:return type="string"/>
  </sce:signature>
  <sce:body>
    <sce:var name="r" type="string" init="''"/>
    <sce:assign target="r" expr="(flag ? 'A' : 'B') + n"/>
    <sce:return expr="r"/>
  </sce:body>
</scxml>
"#;

/// A document that joins a string to `operand`, a value whose text is not the
/// same in every language.
fn joins(operand_type: &str, operand: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm"
       version="1.0">
  <sce:signature>
    <sce:param name="x" type="{operand_type}"/>
    <sce:return type="string"/>
  </sce:signature>
  <sce:body>
    <sce:var name="r" type="string" init="''"/>
    <sce:assign target="r" expr="'v=' + {operand}"/>
    <sce:return expr="r"/>
  </sce:body>
</scxml>
"#
    )
}

/// What generating a document for `language` produced: its source text, or the
/// refusal that stopped it.
enum Outcome {
    Generated(String),
    Refused(String),
}

fn generate_document(document: &str, language: Language) -> Outcome {
    match compile_forge_with_imports(
        document,
        DocumentLabel::symmetric("concat_probe"),
        language,
        &resource_dir(),
        &ForgeCompileOptions::default(),
    ) {
        Ok(output) => Outcome::Generated(
            output
                .files
                .into_iter()
                .map(|(_, source)| source)
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        Err(e) => Outcome::Refused(format!("{e:?}")),
    }
}

fn generate(language: Language) -> Outcome {
    generate_document(DOCUMENT, language)
}

fn generated(document: &str, language: Language) -> String {
    match generate_document(document, language) {
        Outcome::Generated(source) => source,
        Outcome::Refused(why) => panic!("{language:?} refused a document it must lower: {why}"),
    }
}

/// The text of `source` with whitespace removed, so that a spelling is asked
/// about and not the formatter's choice of spaces.
fn compact(source: &str) -> String {
    source.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Whether a generated line adds a number to a string LITERAL as it was
/// written: a quoted string, a plus, an opening parenthesis around the sum.
/// That text is what C and C++ turn into pointer arithmetic and Rust, Go and
/// Python turn into an error.
fn adds_a_number_to_a_literal(source: &str) -> Option<String> {
    source
        .lines()
        .find(|line| {
            // The sum closes with the `)` that follows `n`. What comes after
            // it says whether it was converted: `.toString()` (Kotlin) is the
            // digits, a bare `;`, `,` or end of line is the sum itself.
            let line = compact(line);
            ["\"E\"+(7300+n)", "'E'+(7300+n)"].iter().any(|written| {
                line.match_indices(written)
                    .any(|(at, _)| !line[at + written.len()..].starts_with(".toString()"))
            })
        })
        .map(|line| line.trim().to_string())
}

/// The lines of `source` that mention the base number, each with its
/// whitespace removed: the part of a generated file this test asks about,
/// without the file's header and comments around it.
fn lines_naming_the_base(source: &str) -> Vec<String> {
    source
        .lines()
        .filter(|line| line.contains("7300"))
        .map(compact)
        .collect()
}

#[test]
fn no_backend_spells_a_string_plus_a_number_as_written() {
    let mut wrong = Vec::new();
    let mut refused = Vec::new();
    for &language in Language::ALL {
        match generate(language) {
            Outcome::Generated(source) => {
                if let Some(line) = adds_a_number_to_a_literal(&source) {
                    wrong.push(format!("{language:?}: {line}"));
                }
            }
            // A refusal that names the construct is an answer. It is the
            // refusal being absent, with wrong code generated, that fails.
            // What it said is kept, so a refusal for some other reason (the
            // document itself not being accepted) cannot pass for one.
            Outcome::Refused(why) => refused.push(format!("{language:?}: {why}")),
        }
    }
    assert!(
        refused.iter().all(|why| why.contains("concatenation")),
        "a backend refused the document for a reason other than the concatenation:\n  {}",
        refused.join("\n  ")
    );
    assert!(
        wrong.is_empty(),
        "a number is added to a string literal as written, which is not concatenation in:\n  {}",
        wrong.join("\n  ")
    );
}

/// Each backend that lowers the document spells the digits its own way. The
/// spelling is the language's, so it is stated here per language; what is the
/// same everywhere is that the integer is converted and the literal comes first.
#[test]
fn each_backend_converts_the_integer_to_its_decimal_digits() {
    let spellings: &[(Language, &str)] = &[
        (Language::Cpp, "std::string(\"E\")+std::to_string(7300+n)"),
        (Language::Kotlin, "\"E\"+(7300+n).toString()"),
        (Language::Rust, "format!(\"{}{}\",\"E\",7300+n)"),
        (Language::Go, "\"E\"+strconv.FormatInt(int64(7300+n),10)"),
        (Language::Python, "'E'+str(7300+n)"),
    ];
    for &(language, spelling) in spellings {
        let lines = lines_naming_the_base(&generated(DOCUMENT, language));
        assert!(
            lines.iter().any(|line| line.contains(spelling)),
            "{language:?} does not spell the concatenation as `{spelling}`; the lines that \
             name the base are:\n  {}",
            lines.join("\n  ")
        );
    }
    // The set is a table of what this file knows how to spell; a backend that
    // joins it is asked about, and a backend that is neither spelled above nor
    // C is a seventh nobody taught this test.
    let known: Vec<Language> = spellings.iter().map(|(l, _)| *l).collect();
    for &language in Language::ALL {
        assert!(
            known.contains(&language) || language == Language::C11,
            "{language:?} has no spelling in this test: say how it joins a string to an integer"
        );
    }
}

#[test]
fn a_go_file_that_uses_strconv_imports_it() {
    let source = generated(DOCUMENT, Language::Go);
    assert!(
        source.contains("\"strconv\""),
        "the Go file calls strconv and does not import it:\n{source}"
    );
}

#[test]
fn a_go_file_that_does_not_use_strconv_does_not_import_it() {
    // An unused import is a compile error in Go, so the import must follow the
    // text and not the document's kind.
    let document = DOCUMENT.replace("'E' + (7300 + n)", "n + 1").replace(
        "<sce:return type=\"string\"/>",
        "<sce:return type=\"int32\"/>",
    );
    let document = document.replace(
        "<sce:var name=\"r\" type=\"string\" init=\"''\"/>",
        "<sce:var name=\"r\" type=\"int32\" init=\"0\"/>",
    );
    let source = generated(&document, Language::Go);
    assert!(
        !source.contains("strconv"),
        "an integer-only Go file imports strconv:\n{source}"
    );
}

/// A conditional in the left operand must stay a unit. Written bare, Kotlin's
/// `if (flag) "A" else "B" + n` reads the `+` into the else branch and Python's
/// `'A' if flag else 'B' + str(n)` does the same: the other operand is lost
/// from the branch that was taken.
#[test]
fn a_conditional_string_is_parenthesised_before_the_plus() {
    for (language, spelling) in [
        (Language::Kotlin, "(if(flag)\"A\"else\"B\")+n.toString()"),
        (Language::Python, "('A'ifflagelse'B')+str(n)"),
    ] {
        let lines: Vec<String> = generated(CONDITIONAL_LEFT, language)
            .lines()
            .filter(|line| line.contains("flag") && line.contains("\"A\"") || line.contains("'A'"))
            .map(compact)
            .collect();
        assert!(
            lines.iter().any(|line| line.contains(spelling)),
            "{language:?} does not keep the conditional a unit (`{spelling}`); the lines that \
             hold it are:\n  {}",
            lines.join("\n  ")
        );
    }
}

/// What the author wrote is not always what a backend prints: `1.5`, `true` and
/// a byte array have one text in ECMAScript and another, or none, in each
/// target language, and a float has a different text in two languages that do
/// print one. The generator refuses the construct and names it.
#[test]
fn an_operand_whose_text_differs_between_languages_is_refused_by_name() {
    for (operand_type, operand, what) in
        [("float64", "x", "floating-point"), ("bool", "x", "boolean")]
    {
        let document = joins(operand_type, operand);
        // C refuses the concatenation itself, whatever it joins (see
        // `c_names_that_it_has_no_storage_for_the_joined_text`), so it has no
        // opinion about the operand: the five backends that can lower a
        // concatenation are the ones asked which operands they will carry.
        for &language in Language::ALL.iter().filter(|l| **l != Language::C11) {
            match generate_document(&document, language) {
                Outcome::Refused(why) => assert!(
                    why.contains("concatenation") && why.contains(what),
                    "{language:?} refused `'v=' + {operand_type}` without naming it:\n  {why}"
                ),
                Outcome::Generated(source) => panic!(
                    "{language:?} generated `'v=' + {operand_type}` instead of refusing it:\n{source}"
                ),
            }
        }
    }
}

#[test]
fn c_names_that_it_has_no_storage_for_the_joined_text() {
    match generate(Language::C11) {
        Outcome::Refused(why) => assert!(
            why.contains("string concatenation in C"),
            "C refused, but not by the concatenation:\n  {why}"
        ),
        Outcome::Generated(source) => {
            panic!("C generated a string concatenation it has nowhere to store:\n{source}")
        }
    }
}

// ── running what was generated ──────────────────────────────────────────

/// Whether `tool` can be run here. Absent is reported and the test stops, and
/// is REFUSED where the repository requires every compiler
/// (`SCE_REQUIRE_ALL_COMPILERS`), as every other test that needs a toolchain:
/// a check that quietly did not run is not a check that passed.
fn tool_present(tool: &str, flag: &str) -> bool {
    let present = Command::new(tool)
        .arg(flag)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !present {
        assert!(
            std::env::var_os("SCE_REQUIRE_ALL_COMPILERS").is_none(),
            "{tool} is required (SCE_REQUIRE_ALL_COMPILERS is set) and is absent"
        );
        eprintln!("not run: {tool} is not installed here");
    }
    present
}

/// Run `command` and return its standard output, or fail with what it said.
fn run(mut command: Command, what: &str) -> String {
    let out = command
        .output()
        .unwrap_or_else(|e| panic!("run {what}: {e}"));
    assert!(
        out.status.success(),
        "{what} failed:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sce-concat-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the scratch folder");
    dir
}

/// Python is the one backend every runner carries, and the one where the bug
/// was a `TypeError` at the first call. The generated module is imported and
/// its function called, and the value is the identifier the document names.
#[test]
fn the_generated_python_returns_the_identifier() {
    if !tool_present("python3", "--version") {
        return;
    }
    let source = generated(DOCUMENT, Language::Python);
    let dir = scratch("py");
    std::fs::write(dir.join("concat_probe.py"), &source).expect("write the module");
    let driver = "import concat_probe as m, inspect\n\
                  f = [v for k, v in vars(m).items() if inspect.isfunction(v) and v.__module__ == m.__name__][0]\n\
                  print(f(1), f(66), repr(f(0)))\n";
    let out = Command::new("python3")
        .arg("-c")
        .arg(driver)
        .current_dir(&dir)
        .output()
        .expect("run python3");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "the generated Python did not run:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "E7301 E7366 ''",
        "the generated Python does not return the identifier"
    );
}

/// What a driver prints for the three calls, in every language that is run.
const EXPECTED_RUN: &str = "E7301 E7366 ";

/// Go refused `"E" + (7300 + n)` at compile time, so what is run here is also
/// what compiles: the file is built with the standard library alone (the
/// document is not may-fail, so it names no runtime package) and its function
/// called. The package and function are READ off the generated file, not
/// spelled here, so a change in the naming rule does not make this test stale
/// without making it fail.
#[test]
fn the_generated_go_compiles_and_returns_the_identifier() {
    if !tool_present("go", "version") {
        return;
    }
    let source = generated(DOCUMENT, Language::Go);
    let package = source
        .lines()
        .find_map(|line| line.strip_prefix("package "))
        .expect("the generated Go names its package")
        .trim()
        .to_string();
    let function = source
        .lines()
        .find_map(|line| line.strip_prefix("func "))
        .and_then(|rest| rest.split('(').next())
        .expect("the generated Go declares a function")
        .to_string();
    let dir = scratch("go");
    std::fs::create_dir_all(dir.join(&package)).expect("make the package folder");
    std::fs::write(dir.join(&package).join("concat_probe.go"), &source).expect("write the file");
    std::fs::write(dir.join("go.mod"), "module probe\n\ngo 1.21\n").expect("write go.mod");
    std::fs::write(
        dir.join("main.go"),
        format!(
            "package main\n\nimport (\n\t\"fmt\"\n\n\t\"probe/{package}\"\n)\n\n\
             func main() {{\n\tfmt.Printf(\"%s %s %s\\n\", {package}.{function}(1), \
             {package}.{function}(66), {package}.{function}(0))\n}}\n"
        ),
    )
    .expect("write the driver");
    let mut go = Command::new("go");
    go.arg("run")
        .arg(".")
        .current_dir(&dir)
        .env("GOFLAGS", "-mod=mod");
    let printed = run(go, "go run");
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        printed,
        EXPECTED_RUN.trim_end(),
        "the generated Go does not return the identifier"
    );
}

/// C++ built `"E" + (7300 + n)` as pointer arithmetic, so the first thing to
/// run is the generated header itself: it is compiled with nothing but the
/// standard library, which also asks whether it includes what it names
/// (`std::string`, `std::to_string`). The namespace and function are read off
/// the generated file.
#[test]
fn the_generated_cpp_compiles_and_returns_the_identifier() {
    if !tool_present("g++", "--version") {
        return;
    }
    let source = generated(DOCUMENT, Language::Cpp);
    let namespace = source
        .lines()
        .find_map(|line| line.strip_prefix("namespace "))
        .and_then(|rest| rest.split_whitespace().next())
        .expect("the generated C++ opens a namespace")
        .to_string();
    let function = source
        .lines()
        .find_map(|line| line.strip_prefix("inline std::string "))
        .and_then(|rest| rest.split('(').next())
        .expect("the generated C++ declares a string function")
        .to_string();
    let dir = scratch("cpp");
    std::fs::write(dir.join("concat_probe.h"), &source).expect("write the header");
    std::fs::write(
        dir.join("main.cpp"),
        format!(
            "#include \"concat_probe.h\"\n#include <iostream>\n\
             int main() {{\n  using namespace {namespace};\n  \
             std::cout << {function}(1) << ' ' << {function}(66) << ' ' << {function}(0) << '\\n';\n}}\n"
        ),
    )
    .expect("write the driver");
    let mut compile = Command::new("g++");
    compile
        .args(["-std=c++17", "-Wall", "-Werror", "main.cpp", "-o", "probe"])
        .current_dir(&dir);
    run(compile, "g++");
    let mut probe = Command::new(dir.join("probe"));
    probe.current_dir(&dir);
    let printed = run(probe, "the compiled C++ probe");
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        printed,
        EXPECTED_RUN.trim_end(),
        "the generated C++ does not return the identifier"
    );
}

/// Rust refused `&str + i64` outright, so what is run here is also what
/// compiles. The generated file is a free function; it is built as a module of
/// a one-file program, with `format!` taking the integer and the string alike.
#[test]
fn the_generated_rust_compiles_and_returns_the_identifier() {
    if !tool_present("rustc", "--version") {
        return;
    }
    let source = generated(DOCUMENT, Language::Rust);
    let function = source
        .lines()
        .find_map(|line| line.strip_prefix("pub fn "))
        .and_then(|rest| rest.split('(').next())
        .expect("the generated Rust declares a public function")
        .to_string();
    let dir = scratch("rs");
    std::fs::write(
        dir.join("main.rs"),
        format!(
            "mod probe {{\n{source}\n}}\nfn main() {{\n    println!(\"{{}} {{}} {{}}\", \
             probe::{function}(1), probe::{function}(66), probe::{function}(0));\n}}\n"
        ),
    )
    .expect("write the program");
    let mut compile = Command::new("rustc");
    compile
        .args(["--edition", "2021", "main.rs", "-o", "probe"])
        .current_dir(&dir);
    run(compile, "rustc");
    let mut probe = Command::new(dir.join("probe"));
    probe.current_dir(&dir);
    let printed = run(probe, "the compiled Rust probe");
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        printed,
        EXPECTED_RUN.trim_end(),
        "the generated Rust does not return the identifier"
    );
}

/// Kotlin concatenated this by accident, because `String.plus` takes `Any`; the
/// digits are now explicit. It is compiled and run all the same, because
/// "happened to work" is the property this change replaces with a decision.
#[test]
fn the_generated_kotlin_compiles_and_returns_the_identifier() {
    if !tool_present("kotlinc", "-version") || !tool_present("java", "-version") {
        return;
    }
    let source = generated(DOCUMENT, Language::Kotlin);
    let package = source
        .lines()
        .find_map(|line| line.strip_prefix("package "))
        .expect("the generated Kotlin names its package")
        .trim()
        .to_string();
    let function = source
        .lines()
        .find_map(|line| line.strip_prefix("fun "))
        .and_then(|rest| rest.split('(').next())
        .expect("the generated Kotlin declares a function")
        .to_string();
    let dir = scratch("kt");
    std::fs::write(dir.join("Probe.kt"), &source).expect("write the file");
    std::fs::write(
        dir.join("Main.kt"),
        format!(
            "import {package}.{function}\n\nfun main() {{\n    println(\"${{{function}(1)}} \
             ${{{function}(66)}} ${{{function}(0)}}\")\n}}\n"
        ),
    )
    .expect("write the driver");
    let mut compile = Command::new("kotlinc");
    compile
        .args(["Probe.kt", "Main.kt", "-include-runtime", "-d", "probe.jar"])
        .current_dir(&dir);
    run(compile, "kotlinc");
    let mut java = Command::new("java");
    java.args(["-cp", "probe.jar", "MainKt"]).current_dir(&dir);
    let printed = run(java, "the compiled Kotlin probe");
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        printed,
        EXPECTED_RUN.trim_end(),
        "the generated Kotlin does not return the identifier"
    );
}
