//! An `sce:` attribute nothing reads is refused, not dropped.
//!
//! ⚠ WHAT THIS HOLDS IN PLACE. The forge parser looks attributes up BY
//! NAME, so an attribute it does not look for is not unused — it is
//! invisible. Measured 2026-09-18, before the check these tests cover:
//!
//!   · `sce:totallyMadeUpAttribute="…"` generated with exit 0.
//!   · The conformance fixture `crossfile_filter_transform.scxml` carried
//!     `sce:pre-transform="tempConvert(rawSensor)"`, announced in its own
//!     comment as pre-processing the filter's input. Nothing read it. The
//!     filter smoothed the raw input and the fixture passed.
//!
//! A misspelling is the same failure with a likelier cause:
//! `sce:directon="out"` left the field at its default and said nothing.
//!
//! ⚠⚠ THE NEGATIVE CONTROLS CARRY THE WEIGHT. A check that refused every
//! document would satisfy the first test here, so
//! `a_document_using_only_known_names_is_accepted` and
//! `an_author_named_interpolation_axis_is_accepted` are what separate
//! "the list is right" from "the list is empty". The second of those is
//! the sharper one: `sce:axis-<input id>` is named by the author, so no
//! literal list can contain it and a list-only check refuses a valid
//! document.

use std::process::Command;

fn sce_codegen_bin() -> String {
    env!("CARGO_BIN_EXE_sce-codegen").to_string()
}

struct Tmp(std::path::PathBuf);

impl Tmp {
    fn new(label: &str) -> Self {
        let d =
            std::env::temp_dir().join(format!("sce_unknown_attr_{label}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("create temp dir");
        Tmp(d)
    }
    fn generate(&self, body: &str) -> (Option<i32>, String) {
        let p = self.0.join("doc.scxml");
        std::fs::write(&p, body).expect("write document");
        let run = Command::new(sce_codegen_bin())
            .args([
                "generate",
                p.to_str().unwrap(),
                "-l",
                "cpp",
                "-o",
                self.0.to_str().unwrap(),
            ])
            .output()
            .expect("spawn sce-codegen");
        (
            run.status.code(),
            String::from_utf8_lossy(&run.stderr).into_owned(),
        )
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn transform_with(extra: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="doc">
  <datamodel>
    <data id="raw" sce:type="int32" sce:direction="in"/>
    <data id="out" sce:type="int32" sce:direction="out" expr="raw * 2"{extra}/>
  </datamodel>
</scxml>
"#
    )
}

#[test]
fn an_attribute_nothing_reads_is_refused() {
    let t = Tmp::new("unknown");
    let (code, stderr) = t.generate(&transform_with(r#" sce:totallyMadeUpAttribute="x""#));
    assert_ne!(
        code,
        Some(0),
        "an sce: attribute nothing reads generated:\n{stderr}"
    );
    assert!(
        stderr.contains("totallyMadeUpAttribute"),
        "the refusal does not name the attribute:\n{stderr}"
    );
}

#[test]
fn a_misspelling_is_refused_and_the_right_name_is_offered() {
    // The suggestion is the half that makes the refusal actionable: an
    // author who mistyped a name needs to be told which one they meant,
    // not handed the whole vocabulary.
    let t = Tmp::new("typo");
    let (code, stderr) = t.generate(&transform_with(r#" sce:directon="out""#));
    assert_ne!(code, Some(0), "a misspelt sce: attribute generated");
    assert!(
        stderr.contains("direction"),
        "the refusal does not suggest the intended name:\n{stderr}"
    );
}

#[test]
fn a_document_using_only_known_names_is_accepted() {
    let t = Tmp::new("clean");
    let (code, stderr) = t.generate(&transform_with(""));
    assert_eq!(code, Some(0), "a valid document was refused:\n{stderr}");
}

#[test]
fn an_author_named_interpolation_axis_is_accepted() {
    // `sce:axis-<input id>` takes its suffix from the document, so a
    // literal allow-list cannot hold it. An input nobody has used before
    // is the case a list-only check gets wrong.
    let t = Tmp::new("axis");
    let doc = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="interpolation" name="doc">
  <datamodel>
    <data id="pressureBar" sce:type="uint16" sce:direction="in"/>
    <data id="limit" sce:type="float64" sce:direction="out"
          sce:interpolation="linear" sce:out-of-bounds="clamp"
          sce:axis-pressureBar="0 10 20">
      1.0 2.0 3.0
    </data>
  </datamodel>
</scxml>
"#;
    let (code, stderr) = t.generate(doc);
    assert_eq!(
        code,
        Some(0),
        "an author-named axis attribute was refused:\n{stderr}"
    );
}
