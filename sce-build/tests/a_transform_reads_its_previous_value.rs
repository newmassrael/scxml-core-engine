//! `previous(x)` — a transform output reads a field from the activation
//! before this one.
//!
//! ⚠ WHAT THIS HOLDS IN PLACE, and what it does not yet. The law is
//! validated: `x` must be a field of this document, must say what it was
//! before the first activation (`sce:initial`), and a read through
//! `previous()` is not a dependency, so it cannot close a cycle. No backend
//! LOWERS it yet, so every language refuses such a document with
//! `generate/unsupported-feature` — before rendering, so the verdict does
//! not depend on the language, and never as "unknown function", which would
//! be a false sentence about a valid document.
//!
//! Every refusal here is asserted by CODE and by where it points, not by
//! message text: the code is what a consumer dispatches on, and the row is
//! what lets an author find the line.

use std::path::{Path, PathBuf};
use std::process::Command;

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
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Run {
    exit: Option<i32>,
    stderr: String,
}

fn generate(doc: &Path, out: &Path, lang: &str) -> Run {
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
        .output()
        .expect("spawn sce-codegen");
    Run {
        exit: run.status.code(),
        stderr: String::from_utf8_lossy(&run.stderr).into_owned(),
    }
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

fn transform(name: &str, fields: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="{name}">
  <datamodel>
    <data id="reported" sce:type="int32" sce:direction="in"/>
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
        r#"    <data id="shown" sce:type="int32" sce:direction="out" sce:initial="0"
          expr="reported > 0 ? reported : previous(shown)"/>"#,
    )
}

const LANGUAGES: [&str; 6] = ["python", "cpp", "rust", "go", "kotlin", "c"];

#[test]
fn a_latch_is_valid_and_every_language_refuses_to_generate_it_yet() {
    let t = Tmp::new("latch");
    let text = latch();
    let doc = t.write("latch.scxml", &text);
    for lang in LANGUAGES {
        let run = generate(&doc, &t.0, lang);
        assert_ne!(
            run.exit,
            Some(0),
            "{lang} generated a document nothing lowers"
        );
        let record = only_record(&run);
        // ⚠ The discriminator against the old reading: not a cycle, and
        // not an unknown function.
        assert_eq!(
            record["code"], "generate/unsupported-feature",
            "{lang}: {record}"
        );
        assert!(
            record["message"]
                .as_str()
                .is_some_and(|m| m.contains("previous(shown)")),
            "{lang}: the refusal does not name the read: {record}"
        );
        assert_eq!(
            record["location"]["line"].as_u64(),
            Some(row_of(&text, "previous(shown)")),
            "{lang}: the refusal does not point at the read: {record}"
        );
    }
}

#[test]
fn check_reaches_the_verdict_generate_does() {
    let t = Tmp::new("check");
    let doc = t.write("latch.scxml", &latch());
    let generated = only_record(&generate(&doc, &t.0, "rust"));
    let checked = only_record(&check(&doc, "rust"));
    assert_eq!(checked["id"], generated["id"]);
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
            r#"    <data id="shown" sce:type="int32" sce:direction="out" expr="shown + reported"/>"#,
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
            r#"    <data id="a" sce:type="int32" sce:direction="out" sce:initial="0" expr="previous(b) + reported"/>
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
            r#"    <data id="a" sce:type="int32" sce:direction="out" expr="previous(b) + reported"/>
    <data id="b" sce:type="int32" sce:direction="out" sce:initial="0" expr="a * 2"/>"#,
        ),
    );
    let record = only_record(&generate(&fixed, &t.0, "python"));
    assert_eq!(record["code"], "generate/unsupported-feature", "{record}");
}

#[test]
fn a_field_read_previously_must_say_what_it_was_first() {
    let t = Tmp::new("noinitial");
    let text = transform(
        "noinitial",
        r#"    <data id="shown" sce:type="int32" sce:direction="out"
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
        r#"    <data id="shown" sce:type="int32" sce:direction="out" sce:initial="0"
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
                r#"    <data id="shown" sce:type="int32" sce:direction="out" sce:initial="0"
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
        r#"    <data id="shown" sce:type="int32" sce:direction="out"
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
            r#"    <data id="a" sce:type="string" sce:direction="out" expr="reported > 0 ? 'b' : 'c'"/>
    <data id="b" sce:type="string" sce:direction="out" expr="a"/>"#,
        ),
    );
    let run = generate(&doc, &t.0, "python");
    assert_eq!(run.exit, Some(0), "{}", run.stderr);
}
