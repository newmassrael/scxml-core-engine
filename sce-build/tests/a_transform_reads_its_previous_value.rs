//! `previous(x)` — a transform output reads a field from the activation
//! before this one.
//!
//! The law is validated before any backend sees the document: `x` must be a
//! field of this document, must say what it was before the first activation
//! (`sce:initial`), and a read through `previous()` is not a dependency, so
//! it cannot close a cycle. Every backend then lowers it the same way — each
//! output stays a pure function, with one more parameter per value read
//! previously, and a HOLDER keeps those values between activations and
//! computes every output of one activation before it replaces any of them.
//!
//! Two cells are refused, each once, before rendering and naming the read:
//! `bytes` on every backend, and `string` on C11. What a holder
//! DOES is `transform_previous_value` in the forge conformance catalog, run
//! on all six backends; this file holds the verdicts in place, and the names
//! a holder introduces for itself.
//!
//! Every refusal here is asserted by CODE and by where it points, not by
//! message text: the code is what a consumer dispatches on, and the row is
//! what lets an author find the line.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::toolchain;

fn sce_codegen_bin() -> String {
    env!("CARGO_BIN_EXE_sce-codegen").to_string()
}

struct Tmp(PathBuf);

impl Tmp {
    fn new(label: &str) -> Self {
        let d = std::env::temp_dir().join(format!("sce_previous_{label}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("create temp dir");
        Tmp(d)
    }
    fn write(&self, name: &str, body: &str) -> PathBuf {
        let p = self.0.join(name);
        std::fs::write(&p, body).expect("write document");
        p
    }
    /// A directory of its own under this one, for one language's output.
    fn dir(&self, name: &str) -> PathBuf {
        let d = self.0.join(name);
        std::fs::create_dir_all(&d).expect("create output dir");
        d
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Run {
    exit: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Run {
    /// The manifest line a successful run writes.
    fn manifest(&self) -> serde_json::Value {
        self.stdout
            .lines()
            .find(|l| l.trim_start().starts_with('{'))
            .map(|l| serde_json::from_str(l).expect("manifest line is one JSON object"))
            .unwrap_or_else(|| panic!("no manifest line:\n{}", self.stdout))
    }

    /// The artifacts a successful `generate` lists in its manifest line.
    fn artifacts(&self) -> Vec<PathBuf> {
        let manifest = self.manifest();
        manifest["artifacts"]
            .as_array()
            .unwrap_or_else(|| panic!("manifest lists no artifacts: {manifest}"))
            .iter()
            .map(|a| PathBuf::from(a["path"].as_str().expect("artifact path")))
            .collect()
    }

    /// The one artifact whose name ends in `suffix`.
    fn artifact(&self, suffix: &str) -> PathBuf {
        let matching: Vec<PathBuf> = self
            .artifacts()
            .into_iter()
            .filter(|p| p.to_string_lossy().ends_with(suffix))
            .collect();
        assert_eq!(
            matching.len(),
            1,
            "artifacts ending in {suffix}: {matching:?}"
        );
        matching.into_iter().next().unwrap()
    }
}

fn generate(doc: &Path, out: &Path, lang: &str) -> Run {
    generate_with(doc, out, lang, &[])
}

fn generate_with(doc: &Path, out: &Path, lang: &str, extra: &[&str]) -> Run {
    let run = Command::new(sce_codegen_bin())
        .args([
            "generate",
            doc.to_str().unwrap(),
            "-l",
            lang,
            "-o",
            out.to_str().unwrap(),
            "--error-format=json",
        ])
        .args(extra)
        .output()
        .expect("spawn sce-codegen");
    Run {
        exit: run.status.code(),
        stdout: String::from_utf8_lossy(&run.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&run.stderr).into_owned(),
    }
}

fn generated(doc: &Path, out: &Path, lang: &str) -> Run {
    let run = generate(doc, out, lang);
    assert_eq!(run.exit, Some(0), "{lang} refused:\n{}", run.stderr);
    run
}

fn check(doc: &Path, lang: &str) -> Run {
    let run = Command::new(sce_codegen_bin())
        .args([
            "check",
            doc.to_str().unwrap(),
            "--language",
            lang,
            "--error-format=json",
        ])
        .output()
        .expect("spawn sce-codegen");
    Run {
        exit: run.status.code(),
        stdout: String::from_utf8_lossy(&run.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&run.stderr).into_owned(),
    }
}

/// The one record a refused run writes.
fn only_record(run: &Run) -> serde_json::Value {
    let records: Vec<serde_json::Value> = run
        .stderr
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with('{'))
        .map(|l| serde_json::from_str(l).expect("stderr line is one JSON object"))
        .collect();
    assert_eq!(records.len(), 1, "expected one record:\n{}", run.stderr);
    records.into_iter().next().unwrap()
}

/// The 1-based row of `doc` that holds `needle`.
fn row_of(doc: &str, needle: &str) -> u64 {
    doc.lines()
        .position(|l| l.contains(needle))
        .map(|i| i as u64 + 1)
        .unwrap_or_else(|| panic!("{needle} is not in the document"))
}

/// Run `program` in `cwd` and return what it printed, failing on a
/// non-zero exit.
fn run_ok(program: &Path, args: &[&str], cwd: &Path) -> String {
    let out = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", program.display()));
    assert!(
        out.status.success(),
        "{} {args:?} failed:\nstdout: {}\nstderr: {}",
        program.display(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn transform(name: &str, fields: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="{name}">
  <datamodel>
{fields}
  </datamodel>
</scxml>
"#
    )
}

/// A latch: the reported value, or — while nothing is reported — what was
/// shown the activation before. It reads its own previous value, which
/// the text search this replaced refused as the cycle `shown → shown`.
fn latch() -> String {
    transform(
        "latch",
        r#"    <data id="reported" sce:type="int32" sce:direction="in"/>
    <data id="shown" sce:type="int32" sce:direction="out" sce:initial="0"
          expr="reported > 0 ? reported : previous(shown)"/>"#,
    )
}

/// A counter and an edge: an output cell with a hex first value and an
/// input bool cell, and no conditional, so every backend lowers it.
fn counter() -> String {
    transform(
        "counter",
        r#"    <data id="step" sce:type="int32" sce:direction="in"/>
    <data id="level" sce:type="bool" sce:direction="in" sce:initial="false"/>
    <data id="count" sce:type="int32" sce:direction="out" sce:initial="0x0"
          expr="previous(count) + step"/>
    <data id="rising" sce:type="bool" sce:direction="out"
          expr="level &amp;&amp; !previous(level)"/>"#,
    )
}

/// One activation late: what was reported the activation before. Its first
/// value is the empty string, which a field's `sce:initial` may be.
fn delayed(sce_type: &str) -> String {
    transform(
        "delayed",
        &format!(
            r#"    <data id="reported" sce:type="{sce_type}" sce:direction="in" sce:initial=""/>
    <data id="shown" sce:type="{sce_type}" sce:direction="out"
          expr="previous(reported)"/>"#
        ),
    )
}

const LANGUAGES: [&str; 6] = ["python", "cpp", "rust", "go", "kotlin", "c"];

#[test]
fn every_backend_generates_a_holder_for_a_document_that_reads_previous() {
    let t = Tmp::new("counter");
    let doc = t.write("counter.scxml", &counter());
    let pure = t.write(
        "doubled.scxml",
        &transform(
            "doubled",
            r#"    <data id="step" sce:type="int32" sce:direction="in"/>
    <data id="twice" sce:type="int32" sce:direction="out" expr="step * 2"/>"#,
        ),
    );
    for lang in LANGUAGES {
        let run = generated(&doc, &t.dir(lang), lang);
        // The manifest tells the host it must drive a holder, and names
        // it — and every name it gives is one the generated code defines,
        // or the host would be told to call something that is not there.
        let manifest = run.manifest();
        let holder = manifest["holder"]
            .as_object()
            .unwrap_or_else(|| panic!("{lang}: no `holder` in {manifest}"));
        let code: String = run
            .artifacts()
            .iter()
            .map(|p| std::fs::read_to_string(p).expect("read artifact"))
            .collect();
        for key in ["holder", "outputs", "new", "reset", "update"] {
            let name = holder[key]
                .as_str()
                .unwrap_or_else(|| panic!("{lang}: holder.{key} in {manifest}"));
            assert!(
                code.contains(name),
                "{lang}: the manifest names `{name}` as holder.{key}, which the \
                 generated code does not define"
            );
        }
        // A transform that reads nothing through `previous()` is pure
        // functions, and says so by carrying no holder.
        let pure_run = generated(&pure, &t.dir(&format!("{lang}_pure")), lang);
        assert!(
            pure_run.manifest().get("holder").is_none(),
            "{lang}: a pure transform's manifest names a holder"
        );
    }
}

#[test]
fn a_latch_holds_what_it_showed_while_nothing_is_reported() {
    let Some(python) = toolchain::require_or_skip("python3", "run a generated latch") else {
        return;
    };
    let t = Tmp::new("latch");
    let doc = t.write("latch.scxml", &latch());
    let module = generated(&doc, &t.dir("py"), "python").artifact(".py");
    let stem = module.file_stem().unwrap().to_str().unwrap().to_string();
    let printed = run_ok(
        &python,
        &[
            "-c",
            &format!(
                "import {stem}\n\
                 h = {stem}.Latch()\n\
                 print([h.update(r).shown for r in (0, 5, 0, 0, 7, 0)])\n\
                 h.reset()\n\
                 print(h.update(0).shown)"
            ),
        ],
        module.parent().unwrap(),
    );
    // The first activation reads the field's sce:initial, each later one
    // what the one before showed, and reset() is the first activation again.
    assert_eq!(printed, "[0, 5, 5, 5, 7, 7]\n0\n");
}

#[test]
fn a_string_cell_is_lowered_where_the_holder_owns_the_string() {
    let t = Tmp::new("string");
    let text = delayed("string");
    let doc = t.write("delayed.scxml", &text);
    for lang in ["python", "cpp", "rust", "go", "kotlin"] {
        generated(&doc, &t.dir(lang), lang);
    }
    // C11 keeps a string as a pointer into the caller's buffer — refused,
    // at the read, before rendering.
    let run = generate(&doc, &t.dir("c"), "c");
    assert_ne!(run.exit, Some(0), "c generated a string cell");
    let record = only_record(&run);
    assert_eq!(record["code"], "generate/unsupported-feature", "{record}");
    assert_eq!(
        record["location"]["line"].as_u64(),
        Some(row_of(&text, "previous(reported)")),
        "the refusal does not point at the read: {record}"
    );
}

#[test]
fn a_string_holder_compiles_and_runs_on_rust() {
    // ⚠ Rust declares a string output `-> String` and a string inside an
    // expression is borrowed, so until the returned value was made owned no
    // transform with a string output compiled on Rust — with or without
    // `previous()`. Every shape that makes a string is here: a parameter
    // returned as it is, a literal, a conditional whose one arm is a sibling
    // output (already owned) and whose other is a literal (borrowed), and a
    // kept string passed back in.
    let Some(rustc) = toolchain::require_or_skip("rustc", "compile a generated holder") else {
        return;
    };
    let t = Tmp::new("rust_string");
    let doc = t.write(
        "latest.scxml",
        &transform(
            "latest",
            r#"    <data id="reported" sce:type="string" sce:direction="in" sce:initial=""/>
    <data id="shown" sce:type="string" sce:direction="out"
          expr="len(reported) > 0 ? reported : previous(reported)"/>
    <data id="tagged" sce:type="string" sce:direction="out"
          expr="len(reported) > 0 ? shown : 'none'"/>
    <data id="fixed" sce:type="string" sce:direction="out" expr="'always'"/>"#,
        ),
    );
    let out = t.dir("rs");
    let module = generated(&doc, &out, "rust").artifact(".rs");
    std::fs::write(
        out.join("main.rs"),
        format!(
            "#[path = \"{}\"]\nmod latest;\n\n\
             fn main() {{\n\
             \tlet mut h = latest::Latest::new();\n\
             \tfor r in [\"a\", \"\", \"b\"] {{\n\
             \t\tlet o = h.update(r);\n\
             \t\tprint!(\"{{}}/{{}}/{{}} \", o.shown, o.tagged, o.fixed);\n\
             \t}}\n\
             \th.reset();\n\
             \tlet o = h.update(\"\");\n\
             \tprintln!(\"[{{}}]/{{}}\", o.shown, o.tagged);\n\
             }}\n",
            module.file_name().unwrap().to_str().unwrap()
        ),
    )
    .expect("write main.rs");
    run_ok(
        &rustc,
        &["--edition=2021", "-D", "warnings", "-o", "main", "main.rs"],
        &out,
    );
    // An empty report shows the last one; reset() is the first activation
    // again, whose previous report is the empty `sce:initial`.
    assert_eq!(
        run_ok(&out.join("main"), &[], &out),
        "a/a/always a/none/always b/b/always []/none\n"
    );
}

#[test]
fn a_bytes_cell_is_refused_on_every_backend() {
    let t = Tmp::new("bytes");
    let text = delayed("bytes");
    let doc = t.write("delayed.scxml", &text);
    for lang in LANGUAGES {
        let run = generate(&doc, &t.dir(lang), lang);
        assert_ne!(run.exit, Some(0), "{lang} generated a bytes cell");
        let record = only_record(&run);
        assert_eq!(
            record["code"], "generate/unsupported-feature",
            "{lang}: {record}"
        );
        assert_eq!(
            record["location"]["line"].as_u64(),
            Some(row_of(&text, "previous(reported)")),
            "{lang}: the refusal does not point at the read: {record}"
        );
    }
}

#[test]
fn check_reaches_the_verdict_generate_does() {
    let t = Tmp::new("check");
    let doc = t.write("delayed.scxml", &delayed("string"));
    let generated = only_record(&generate(&doc, &t.dir("c"), "c"));
    let checked = only_record(&check(&doc, "c"));
    assert_eq!(checked["id"], generated["id"]);
}

#[test]
fn a_field_spelled_like_a_value_read_previously_collides() {
    // A value read through `previous(shown)` is the parameter
    // `previous_shown` of every output function, so a field of that name
    // would be two parameters with one name.
    let t = Tmp::new("collision");
    let doc = t.write(
        "collision.scxml",
        &transform(
            "collision",
            r#"    <data id="reported" sce:type="int32" sce:direction="in"/>
    <data id="previous_shown" sce:type="int32" sce:direction="in"/>
    <data id="shown" sce:type="int32" sce:direction="out" sce:initial="0"
          expr="previous(shown) + reported + previous_shown"/>"#,
        ),
    );
    let record = only_record(&generate(&doc, &t.0, "python"));
    assert_eq!(record["code"], "validation/duplicate-id", "{record}");
}

#[test]
fn a_transform_that_keeps_state_is_not_imported_as_a_pure_function() {
    // A stateless import calls the imported transform with the caller's
    // inputs alone, and a transform reading `previous()` needs its kept
    // values too — emitted, the call has too few arguments, with exit 0.
    let t = Tmp::new("import");
    // The pair differs in one thing: whether its one output reads its own
    // previous value. The pure one is the control — the same import of it
    // generates on every backend, so the refusal below is about the state
    // and not about the import.
    t.write(
        "accumulated.scxml",
        &transform(
            "accumulated",
            r#"    <data id="step" sce:type="int32" sce:direction="in"/>
    <data id="total" sce:type="int32" sce:direction="out" sce:initial="0"
          expr="previous(total) + step"/>"#,
        ),
    );
    t.write(
        "doubled.scxml",
        &transform(
            "doubled",
            r#"    <data id="step" sce:type="int32" sce:direction="in"/>
    <data id="total" sce:type="int32" sce:direction="out" expr="step * 2"/>"#,
        ),
    );
    let caller = |src: &str, expr: &str| {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="caller">
  <sce:import src="{src}" kind="transform" as="imported"/>
  <datamodel>
    <data id="step" sce:type="int32" sce:direction="in"/>
    <data id="shown" sce:type="int32" sce:direction="out" expr="{expr}"/>
  </datamodel>
</scxml>
"#
        )
    };
    let pure = t.write(
        "calls_pure.scxml",
        &caller("doubled.scxml", "imported(step) + 1"),
    );
    // Declared and never named: nothing is emitted for it, so nothing is
    // called with too few arguments — this one passes.
    let unnamed = t.write(
        "declares_stateful.scxml",
        &caller("accumulated.scxml", "step + 1"),
    );
    let text = caller("accumulated.scxml", "imported(step) + 1");
    let stateful = t.write("calls_stateful.scxml", &text);
    for lang in LANGUAGES {
        // Go refuses every import it cannot qualify by module, first; the
        // prefix is what lets the question this asks be asked at all.
        let extra: &[&str] = if lang == "go" {
            &["--go-module-prefix", "probe"]
        } else {
            &[]
        };
        for (doc, what) in [
            (&pure, "a call of a pure transform"),
            (&unnamed, "a stateful transform declared and never named"),
        ] {
            let run = generate_with(doc, &t.dir(&format!("{lang}_control")), lang, extra);
            assert_eq!(
                run.exit,
                Some(0),
                "{lang}: {what} was refused:\n{}",
                run.stderr
            );
        }
        let run = generate_with(&stateful, &t.dir(lang), lang, extra);
        assert_ne!(run.exit, Some(0), "{lang} imported a stateful transform");
        let record = only_record(&run);
        assert_eq!(
            record["code"], "generate/unsupported-feature",
            "{lang}: {record}"
        );
        // At the import, in the importing document.
        assert_eq!(
            record["location"]["line"].as_u64(),
            Some(row_of(&text, "<sce:import")),
            "{lang}: {record}"
        );
    }
}

/// Fields spelled like every name a holder introduces for itself: Go's
/// receiver and C11's holder parameter (`holder`), C11's record local
/// (`out`), Python's instance (`self`), and the C++ member that keeps
/// `previous(total)` (`previous_total_`).
fn crowded() -> String {
    transform(
        "crowded",
        r#"    <data id="holder" sce:type="int32" sce:direction="in"/>
    <data id="out" sce:type="int32" sce:direction="in"/>
    <data id="self" sce:type="int32" sce:direction="in"/>
    <data id="previous_total_" sce:type="int32" sce:direction="in"/>
    <data id="total" sce:type="int32" sce:direction="out" sce:initial="0"
          expr="previous(total) + holder + out + self + previous_total_"/>"#,
    )
}

/// Two activations: 0 + (1+2+3+4), then 10 + (1+1+1+1). ⚠ A C++ member
/// shadowed by a parameter compiles cleanly and answers 4 second — its
/// commit lands on the parameter — so only running the holder tells.
const CROWDED_TOTALS: &str = "10 14\n";

#[test]
fn a_field_spelled_like_a_holders_own_name_does_not_take_it_c() {
    let Some(cc) = toolchain::require_any_or_skip(&["gcc", "cc"], "compile a generated holder")
    else {
        return;
    };
    let t = Tmp::new("crowded_c");
    let doc = t.write("crowded.scxml", &crowded());
    let out = t.dir("c");
    let header = generated(&doc, &out, "c").artifact(".h");
    std::fs::write(
        out.join("main.c"),
        format!(
            "#include <stdio.h>\n#include \"{}\"\n\
             int main(void) {{\n\
             \tcrowded_state_t h;\n\
             \tcrowded_init(&h);\n\
             \tint a = crowded_update(&h, 1, 2, 3, 4).total;\n\
             \tint b = crowded_update(&h, 1, 1, 1, 1).total;\n\
             \tprintf(\"%d %d\\n\", a, b);\n\
             \treturn 0;\n\
             }}\n",
            header.file_name().unwrap().to_str().unwrap()
        ),
    )
    .expect("write main.c");
    run_ok(
        &cc,
        &["-std=c11", "-Wall", "-Werror", "-o", "main", "main.c"],
        &out,
    );
    assert_eq!(run_ok(&out.join("main"), &[], &out), CROWDED_TOTALS);
}

#[test]
fn a_field_spelled_like_a_holders_own_name_does_not_take_it_cpp() {
    let Some(cxx) = toolchain::require_or_skip("g++", "compile a generated holder") else {
        return;
    };
    let t = Tmp::new("crowded_cpp");
    let doc = t.write("crowded.scxml", &crowded());
    let out = t.dir("cpp");
    let header = generated(&doc, &out, "cpp").artifact(".h");
    std::fs::write(
        out.join("main.cpp"),
        format!(
            "#include <cstdio>\n#include \"{}\"\n\
             int main() {{\n\
             \tSCE::Generated::Crowded::Crowded h;\n\
             \tint a = h.update(1, 2, 3, 4).total;\n\
             \tint b = h.update(1, 1, 1, 1).total;\n\
             \tstd::printf(\"%d %d\\n\", a, b);\n\
             \treturn 0;\n\
             }}\n",
            header.file_name().unwrap().to_str().unwrap()
        ),
    )
    .expect("write main.cpp");
    run_ok(
        &cxx,
        &["-std=c++17", "-Wall", "-Werror", "-o", "main", "main.cpp"],
        &out,
    );
    assert_eq!(run_ok(&out.join("main"), &[], &out), CROWDED_TOTALS);
}

#[test]
fn a_field_spelled_like_a_holders_own_name_does_not_take_it_python() {
    let Some(python) = toolchain::require_or_skip("python3", "run a generated holder") else {
        return;
    };
    let t = Tmp::new("crowded_py");
    let doc = t.write("crowded.scxml", &crowded());
    let module = generated(&doc, &t.dir("py"), "python").artifact(".py");
    let stem = module.file_stem().unwrap().to_str().unwrap().to_string();
    let printed = run_ok(
        &python,
        &[
            "-c",
            &format!(
                "import {stem}\n\
                 h = {stem}.Crowded()\n\
                 a = h.update(1, 2, 3, 4).total\n\
                 b = h.update(1, 1, 1, 1).total\n\
                 print(a, b)"
            ),
        ],
        module.parent().unwrap(),
    );
    assert_eq!(printed, CROWDED_TOTALS);
}

#[test]
fn a_field_spelled_like_a_holders_own_name_does_not_take_it_go() {
    let Some(go) = toolchain::require_or_skip("go", "run a generated holder") else {
        return;
    };
    let t = Tmp::new("crowded_go");
    let doc = t.write("crowded.scxml", &crowded());
    let module = t.dir("gomod");
    let package = module.join("crowded");
    std::fs::create_dir_all(&package).expect("create package dir");
    generated(&doc, &package, "go");
    std::fs::write(module.join("go.mod"), "module probe\n\ngo 1.21\n").expect("write go.mod");
    std::fs::write(
        module.join("main.go"),
        "package main\n\n\
         import (\n\t\"fmt\"\n\n\t\"probe/crowded\"\n)\n\n\
         func main() {\n\
         \th := crowded.NewCrowded()\n\
         \ta := h.Update(1, 2, 3, 4).Total\n\
         \tb := h.Update(1, 1, 1, 1).Total\n\
         \tfmt.Printf(\"%d %d\\n\", a, b)\n\
         }\n",
    )
    .expect("write main.go");
    assert_eq!(run_ok(&go, &["run", "."], &module), CROWDED_TOTALS);
}

/// A retained count read through `previous()` beside a level that is not
/// retained. A holder `restored` from a stored 10 counts on from it while
/// the level starts at its `sce:initial` (so it rises at once); `reset()`
/// is the first day again.
fn stored() -> String {
    transform(
        "stored",
        r#"    <data id="step" sce:type="int32" sce:direction="in"/>
    <data id="level" sce:type="bool" sce:direction="in" sce:initial="false"/>
    <data id="count" sce:type="int32" sce:direction="out" sce:retain="nvm"
          sce:initial="0" expr="previous(count) + step"/>
    <data id="rising" sce:type="bool" sce:direction="out"
          expr="level &amp;&amp; !previous(level)"/>"#,
    )
}

/// `restored(10)` then one activation, then `reset()` and one more:
/// 10 + 1, a rise from the initial `false`, then 0 + 1.
const STORED_RUN: &str = "11 1 1\n";

#[test]
fn a_retained_field_starts_from_what_the_host_stored() {
    let t = Tmp::new("stored");
    let doc = t.write("stored.scxml", &stored());
    // The manifest names the extra constructor exactly when a retained
    // field is read through `previous()`.
    for lang in LANGUAGES {
        let run = generated(&doc, &t.dir(&format!("{lang}_manifest")), lang);
        assert!(
            run.manifest()["holder"]["restored"].is_string(),
            "{lang}: no `restored` in {}",
            run.manifest()
        );
    }
    let counted = t.write("counter.scxml", &counter());
    let run = generated(&counted, &t.dir("unretained"), "rust");
    assert!(
        run.manifest()["holder"].get("restored").is_none(),
        "a holder with no retained field names `restored`"
    );

    if let Some(cc) = toolchain::require_any_or_skip(&["gcc", "cc"], "run a restored holder") {
        let out = t.dir("c");
        let header = generated(&doc, &out, "c").artifact(".h");
        std::fs::write(
            out.join("main.c"),
            format!(
                "#include <stdio.h>\n#include \"{}\"\n\
                 int main(void) {{\n\
                 \tstored_state_t h;\n\
                 \tstored_restore(&h, 10);\n\
                 \tstored_outputs_t a = stored_update(&h, 1, true);\n\
                 \tstored_reset(&h);\n\
                 \tstored_outputs_t b = stored_update(&h, 1, false);\n\
                 \tprintf(\"%d %d %d\\n\", a.count, a.rising ? 1 : 0, b.count);\n\
                 \treturn 0;\n\
                 }}\n",
                header.file_name().unwrap().to_str().unwrap()
            ),
        )
        .expect("write main.c");
        run_ok(
            &cc,
            &[
                "-std=c11", "-Wall", "-Wextra", "-Werror", "-o", "main", "main.c",
            ],
            &out,
        );
        assert_eq!(run_ok(&out.join("main"), &[], &out), STORED_RUN, "c");
    }

    if let Some(cxx) = toolchain::require_or_skip("g++", "run a restored holder") {
        let out = t.dir("cpp");
        let header = generated(&doc, &out, "cpp").artifact(".h");
        std::fs::write(
            out.join("main.cpp"),
            format!(
                "#include <cstdio>\n#include \"{}\"\n\
                 int main() {{\n\
                 \tauto h = SCE::Generated::Stored::Stored::restored(10);\n\
                 \tconst auto a = h.update(1, true);\n\
                 \th.reset();\n\
                 \tconst auto b = h.update(1, false);\n\
                 \tstd::printf(\"%d %d %d\\n\", a.count, a.rising ? 1 : 0, b.count);\n\
                 \treturn 0;\n\
                 }}\n",
                header.file_name().unwrap().to_str().unwrap()
            ),
        )
        .expect("write main.cpp");
        run_ok(
            &cxx,
            &[
                "-std=c++17",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-o",
                "main",
                "main.cpp",
            ],
            &out,
        );
        assert_eq!(run_ok(&out.join("main"), &[], &out), STORED_RUN, "cpp");
    }

    if let Some(python) = toolchain::require_or_skip("python3", "run a restored holder") {
        let module = generated(&doc, &t.dir("py"), "python").artifact(".py");
        let stem = module.file_stem().unwrap().to_str().unwrap().to_string();
        let printed = run_ok(
            &python,
            &[
                "-c",
                &format!(
                    "import {stem}\n\
                     h = {stem}.Stored.restored(10)\n\
                     a = h.update(1, True)\n\
                     h.reset()\n\
                     b = h.update(1, False)\n\
                     print(a.count, int(a.rising), b.count)"
                ),
            ],
            module.parent().unwrap(),
        );
        assert_eq!(printed, STORED_RUN, "python");
    }

    if let Some(go) = toolchain::require_or_skip("go", "run a restored holder") {
        let module = t.dir("gomod");
        let package = module.join("stored");
        std::fs::create_dir_all(&package).expect("create package dir");
        generated(&doc, &package, "go");
        std::fs::write(module.join("go.mod"), "module probe\n\ngo 1.21\n").expect("write go.mod");
        std::fs::write(
            module.join("main.go"),
            "package main\n\n\
             import (\n\t\"fmt\"\n\n\t\"probe/stored\"\n)\n\n\
             func main() {\n\
             \th := stored.RestoredStored(10)\n\
             \ta := h.Update(1, true)\n\
             \th.Reset()\n\
             \tb := h.Update(1, false)\n\
             \trose := 0\n\
             \tif a.Rising {\n\
             \t\trose = 1\n\
             \t}\n\
             \tfmt.Printf(\"%d %d %d\\n\", a.Count, rose, b.Count)\n\
             }\n",
        )
        .expect("write main.go");
        assert_eq!(run_ok(&go, &["run", "."], &module), STORED_RUN, "go");
    }

    if let Some(rustc) = toolchain::require_or_skip("rustc", "run a restored holder") {
        let out = t.dir("rs");
        let module = generated(&doc, &out, "rust").artifact(".rs");
        std::fs::write(
            out.join("main.rs"),
            format!(
                "#[path = \"{}\"]\nmod stored;\n\n\
                 fn main() {{\n\
                 \tlet mut h = stored::Stored::restored(10);\n\
                 \tlet a = h.update(1, true);\n\
                 \th.reset();\n\
                 \tlet b = h.update(1, false);\n\
                 \tprintln!(\"{{}} {{}} {{}}\", a.count, a.rising as i32, b.count);\n\
                 }}\n",
                module.file_name().unwrap().to_str().unwrap()
            ),
        )
        .expect("write main.rs");
        run_ok(
            &rustc,
            &["--edition=2021", "-D", "warnings", "-o", "main", "main.rs"],
            &out,
        );
        assert_eq!(run_ok(&out.join("main"), &[], &out), STORED_RUN, "rust");
    }

    if let Some(kotlinc) = toolchain::require_or_skip("kotlinc", "compile a restored holder") {
        let out = t.dir("kt");
        let file = generated(&doc, &out, "kotlin").artifact(".kt");
        run_ok(
            &kotlinc,
            &[
                "-Werror",
                file.file_name().unwrap().to_str().unwrap(),
                "-d",
                "classes",
            ],
            &out,
        );
    }
}

#[test]
fn a_direct_self_read_is_still_a_cycle() {
    // The pair of the latch above: the same read WITHOUT `previous()` is
    // this activation's value, and a value defined by itself is a cycle.
    let t = Tmp::new("selfread");
    let doc = t.write(
        "selfread.scxml",
        &transform(
            "selfread",
            r#"    <data id="reported" sce:type="int32" sce:direction="in"/>
    <data id="shown" sce:type="int32" sce:direction="out" expr="shown + reported"/>"#,
        ),
    );
    let record = only_record(&generate(&doc, &t.0, "python"));
    assert_eq!(record["code"], "validation/transform-output-cycle");
}

#[test]
fn a_read_through_previous_breaks_a_cycle_between_outputs() {
    let t = Tmp::new("pair");
    let broken = t.write(
        "broken.scxml",
        &transform(
            "broken",
            r#"    <data id="reported" sce:type="int32" sce:direction="in"/>
    <data id="a" sce:type="int32" sce:direction="out" sce:initial="0" expr="previous(b) + reported"/>
    <data id="b" sce:type="int32" sce:direction="out" expr="a * 2"/>"#,
        ),
    );
    let record = only_record(&generate(&broken, &t.0, "python"));
    assert_eq!(record["code"], "validation/missing-attribute", "{record}");
    // ⚠ b is the field read previously, so b is the one that must say what
    // it was first — not a, which declares an initial nobody reads.
    assert_eq!(record["fix"]["element"], "field 'b'");

    let fixed = t.write(
        "fixed.scxml",
        &transform(
            "fixed",
            r#"    <data id="reported" sce:type="int32" sce:direction="in"/>
    <data id="a" sce:type="int32" sce:direction="out" expr="previous(b) + reported"/>
    <data id="b" sce:type="int32" sce:direction="out" sce:initial="0" expr="a * 2"/>"#,
        ),
    );
    generated(&fixed, &t.0, "python");
}

#[test]
fn a_field_read_previously_must_say_what_it_was_first() {
    let t = Tmp::new("noinitial");
    let text = transform(
        "noinitial",
        r#"    <data id="reported" sce:type="int32" sce:direction="in"/>
    <data id="shown" sce:type="int32" sce:direction="out"
          expr="reported > 0 ? reported : previous(shown)"/>"#,
    );
    let doc = t.write("noinitial.scxml", &text);
    let record = only_record(&generate(&doc, &t.0, "python"));
    assert_eq!(record["code"], "validation/missing-attribute");
    assert_eq!(record["fix"]["kind"], "add_attribute");
    assert_eq!(record["fix"]["element"], "field 'shown'");
    assert_eq!(record["fix"]["attr"], "sce:initial");
    // At the read, which is why the attribute is needed.
    assert_eq!(
        record["location"]["line"].as_u64(),
        Some(row_of(&text, "previous(shown)"))
    );
}

#[test]
fn previous_names_only_a_field_of_this_document() {
    let t = Tmp::new("unknown");
    let text = transform(
        "unknown",
        r#"    <data id="reported" sce:type="int32" sce:direction="in"/>
    <data id="shown" sce:type="int32" sce:direction="out" sce:initial="0"
          expr="reported > 0 ? reported : previous(shwn)"/>"#,
    );
    let doc = t.write("unknown.scxml", &text);
    let record = only_record(&generate(&doc, &t.0, "python"));
    assert_eq!(record["code"], "expression/unknown-identifier", "{record}");
    assert_eq!(record["actual"], "shwn");
    let candidates = record["fix"]["candidates"].as_array().expect("candidates");
    assert!(candidates.iter().any(|c| c == "shown"), "{record}");
    assert_eq!(
        record["location"]["line"].as_u64(),
        Some(row_of(&text, "previous(shwn)"))
    );
}

#[test]
fn a_misused_previous_is_refused_as_what_it_is() {
    for misuse in [
        "previous()",
        "previous(shown, reported)",
        "previous(1)",
        "previous(shown + 1)",
    ] {
        let t = Tmp::new("misuse");
        let text = transform(
            "misuse",
            &format!(
                r#"    <data id="reported" sce:type="int32" sce:direction="in"/>
    <data id="shown" sce:type="int32" sce:direction="out" sce:initial="0"
          expr="reported + {misuse}"/>"#
            ),
        );
        let doc = t.write("misuse.scxml", &text);
        let record = only_record(&generate(&doc, &t.0, "python"));
        assert_eq!(
            record["code"], "expression/parse-mismatch",
            "{misuse}: {record}"
        );
        assert_eq!(record["actual"], misuse, "{record}");
        assert_eq!(
            record["location"]["line"].as_u64(),
            Some(row_of(&text, misuse)),
            "{misuse}: {record}"
        );
    }
}

#[test]
fn an_initial_value_nothing_reads_is_still_refused() {
    // The orphan rule moved from the parser into validation, where the
    // reads are known. For a field nothing reads it is exactly what it was.
    let t = Tmp::new("orphan");
    let text = transform(
        "orphan",
        r#"    <data id="reported" sce:type="int32" sce:direction="in"/>
    <data id="shown" sce:type="int32" sce:direction="out"
          sce:initial="7"
          expr="reported + 1"/>"#,
    );
    let doc = t.write("orphan.scxml", &text);
    let record = only_record(&generate(&doc, &t.0, "python"));
    assert_eq!(
        record["code"], "validation/attribute-rule-violated",
        "{record}"
    );
    assert_eq!(record["actual"], "7");
    // ⚠ On the attribute's own row. The parser used to answer from the
    // element; after moving, the rule would have had no row at all.
    assert_eq!(
        record["location"]["line"].as_u64(),
        Some(row_of(&text, "sce:initial=\"7\""))
    );
}

#[test]
fn a_string_that_spells_an_output_is_not_a_read() {
    // The text search this replaced took `'b'` for a read of the output `b`
    // and refused a document with no cycle in it.
    let t = Tmp::new("literal");
    let doc = t.write(
        "literal.scxml",
        &transform(
            "literal",
            r#"    <data id="reported" sce:type="int32" sce:direction="in"/>
    <data id="a" sce:type="string" sce:direction="out" expr="reported > 0 ? 'b' : 'c'"/>
    <data id="b" sce:type="string" sce:direction="out" expr="a"/>"#,
        ),
    );
    generated(&doc, &t.0, "python");
}
