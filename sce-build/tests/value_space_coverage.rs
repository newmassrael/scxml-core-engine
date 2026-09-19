//! `sce-codegen coverage` — which declared values does a document never name?
//!
//! ⚠ WHAT THIS HOLDS IN PLACE, AND WHY IT IS NOT A TYPE CHECK. A
//! conditional chain ending in `else` is TOTAL: it produces a value for
//! every input, so nothing in the type system has anything to complain
//! about. The question the author needs answered is a different one —
//! *which* of the variants I declared reach only that `else`, and did I
//! mean them to?
//!
//! ⚠⚠ THE MEASUREMENT THAT PROMPTED IT. This tree's NL→IR survey found
//! exactly one such hole BY HAND: an operating-logic table covering two
//! numeric ranges with three values between them that no row mentioned.
//! It was found by a person noticing an arithmetic gap in a table. Every
//! hole found that way is one a different reader would have missed, so
//! the finding belongs in the tool.
//!
//! ⚠⚠⚠ THE NEGATIVE CONTROL IS THE LOAD-BEARING HALF. A reporter that
//! listed every variant of every enum would satisfy "it reports the
//! uncovered ones" while telling a consumer nothing, so
//! `a_document_that_names_every_variant_reports_nothing` is what
//! separates "it measures coverage" from "it prints the enum".

use std::process::Command;

fn bin() -> String {
    env!("CARGO_BIN_EXE_sce-codegen").to_string()
}

struct Tmp(std::path::PathBuf);

impl Tmp {
    fn new(label: &str) -> Self {
        let d = std::env::temp_dir().join(format!("sce_coverage_{label}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("create temp dir");
        Tmp(d)
    }
    fn write(&self, name: &str, body: &str) -> std::path::PathBuf {
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

/// ⚠ THE ALIAS IS DELIBERATELY UNLIKE THE DOCUMENT'S IDENTITY. The
/// import is aliased `Fuel` while the document is `FuelType` in a file
/// named `fueltype.scxml`, so a CORRECT qualified reference never
/// contains the string `Fuel.` and the unresolved-alias defect always
/// does.
///
/// ⚠⚠ The first version of this fixture had the alias and the file
/// agree (`Fuel` / `fuel.scxml`), and the test could not tell the two
/// apart: Kotlin's correct emit IS `<Class>.<VARIANT>`, so the defect's
/// output and the fix's output were the same string. A fixture whose
/// names collide cannot measure a difference between them.
const FUEL_ENUM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="enum" name="FuelType" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="GSL" value="0"/>
      <sce:variant name="DSL" value="1"/>
      <sce:variant name="LPI" value="2"/>
      <sce:variant name="OTHERS" value="3"/>
    </data>
  </datamodel>
</scxml>
"#;

/// Names ONE of four variants. The other three reach the `else`.
const NAMES_ONE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="names_one">
  <sce:import as="Fuel" src="fueltype.scxml" kind="enum"/>
  <datamodel>
    <data id="fuel" sce:type="enum:Fuel" sce:direction="in"/>
    <data id="lamp" sce:type="int32" sce:direction="out"
          expr="fuel === Fuel.DSL ? 1 : 0"/>
  </datamodel>
</scxml>
"#;

/// Names ALL four. Nothing reaches a default unexamined.
const NAMES_ALL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="names_all">
  <sce:import as="Fuel" src="fueltype.scxml" kind="enum"/>
  <datamodel>
    <data id="fuel" sce:type="enum:Fuel" sce:direction="in"/>
    <data id="lamp" sce:type="int32" sce:direction="out"
          expr="fuel === Fuel.DSL ? 1
              : fuel === Fuel.GSL ? 2
              : fuel === Fuel.LPI ? 3
              : fuel === Fuel.OTHERS ? 4
              : 0"/>
  </datamodel>
</scxml>
"#;

fn coverage(doc: &std::path::Path) -> (Option<i32>, String, String) {
    let run = Command::new(bin())
        .args(["coverage", doc.to_str().unwrap()])
        .output()
        .expect("spawn sce-codegen");
    (
        run.status.code(),
        String::from_utf8_lossy(&run.stdout).into_owned(),
        String::from_utf8_lossy(&run.stderr).into_owned(),
    )
}

#[test]
fn a_document_that_names_one_variant_reports_the_other_three() {
    let t = Tmp::new("one");
    t.write("fueltype.scxml", FUEL_ENUM);
    let doc = t.write("names_one.scxml", NAMES_ONE);
    let (code, out, err) = coverage(&doc);
    assert_eq!(code, Some(0), "coverage failed:\n{err}");
    let lines: Vec<&str> = out.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 1, "expected one record, got:\n{out}");
    for needle in [
        "\"input\":\"fuel\"",
        "\"value_space\":\"Fuel\"",
        "\"covered\":[\"DSL\"]",
    ] {
        assert!(
            lines[0].contains(needle),
            "record missing {needle}:\n{}",
            lines[0]
        );
    }
    // The three it never names, and ONLY those three — a report that
    // also listed `DSL` would be describing the enum, not the coverage.
    for v in ["GSL", "LPI", "OTHERS"] {
        assert!(
            lines[0].contains(&format!("\"{v}\"")),
            "{v} reaches only the default and is not reported:\n{}",
            lines[0]
        );
    }
    let uncovered = lines[0]
        .split("\"uncovered\":")
        .nth(1)
        .expect("record carries an uncovered list");
    assert!(
        !uncovered.contains("DSL"),
        "a named variant was reported as uncovered:\n{}",
        lines[0]
    );
}

#[test]
fn a_document_that_names_every_variant_reports_nothing() {
    // ⚠ The load-bearing control. Without it, a reporter that printed
    // every variant of every enum would pass the test above.
    let t = Tmp::new("all");
    t.write("fueltype.scxml", FUEL_ENUM);
    let doc = t.write("names_all.scxml", NAMES_ALL);
    let (code, out, err) = coverage(&doc);
    assert_eq!(code, Some(0), "coverage failed:\n{err}");
    assert!(
        out.trim().is_empty(),
        "a document naming every variant still produced a record:\n{out}"
    );
}

/// Two variants, one named. The other owns the default unambiguously.
const TWO_VARIANT_ENUM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="enum" name="OnOff" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="OFF" value="0"/>
      <sce:variant name="ON" value="1"/>
    </data>
  </datamodel>
</scxml>
"#;

const NAMES_ONE_OF_TWO: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="one_of_two">
  <sce:import as="Sw" src="onoff.scxml" kind="enum"/>
  <datamodel>
    <data id="sw" sce:type="enum:Sw" sce:direction="in"/>
    <data id="lamp" sce:type="int32" sce:direction="out"
          expr="sw === Sw.ON ? 1 : 0"/>
  </datamodel>
</scxml>
"#;

/// Three variants, two named. One owns the default unambiguously.
const NAMES_TWO_OF_THREE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="two_of_three">
  <sce:import as="R" src="result.scxml" kind="enum"/>
  <datamodel>
    <data id="r" sce:type="enum:R" sce:direction="in"/>
    <data id="lamp" sce:type="int32" sce:direction="out"
          expr="r === R.ok ? 1 : r === R.error ? 2 : 0"/>
  </datamodel>
</scxml>
"#;

const THREE_VARIANT_ENUM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="enum" name="result" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="ok" value="0"/>
      <sce:variant name="error" value="1"/>
      <sce:variant name="timeout" value="2"/>
    </data>
  </datamodel>
</scxml>
"#;

#[test]
fn one_unnamed_variant_is_not_a_question() {
    // ⚠⚠ THE FALSE-ALARM RULE, AND WHY IT IS NOT A WEAKENING. Naming all
    // but one variant DOES say what happens to the last one — by saying
    // what happens to everything else. Reporting it is noise, and the
    // corpus that prompted this rule contained **34** such cases against
    // a handful of real gaps. An author who learns the list contains
    // noise stops reading it, and the real gap goes with it.
    //
    // Both shapes are asserted (2 variants/1 named, 3 variants/2 named)
    // because a rule written as "silent when exactly one is missing"
    // must not turn out to mean "silent when the enum is small".
    let t = Tmp::new("one_missing");
    t.write("onoff.scxml", TWO_VARIANT_ENUM);
    t.write("result.scxml", THREE_VARIANT_ENUM);

    let two = t.write("one_of_two.scxml", NAMES_ONE_OF_TWO);
    let (code, out, err) = coverage(&two);
    assert_eq!(code, Some(0), "coverage failed:\n{err}");
    assert!(
        out.trim().is_empty(),
        "2 variants with 1 named produced a record — `OFF` owns the default \
         unambiguously:\n{out}"
    );

    let three = t.write("two_of_three.scxml", NAMES_TWO_OF_THREE);
    let (code, out, err) = coverage(&three);
    assert_eq!(code, Some(0), "coverage failed:\n{err}");
    assert!(
        out.trim().is_empty(),
        "3 variants with 2 named produced a record — `timeout` owns the \
         default unambiguously:\n{out}"
    );
}

#[test]
fn coverage_is_a_report_not_a_refusal() {
    // Exit 0 even with uncovered variants. A non-zero exit would make a
    // CI lane treat "not every variant is named" as a failure, and the
    // cheapest way to clear that is to add a rule that does nothing —
    // worse than the fall-through, because it looks decided.
    let t = Tmp::new("exit");
    t.write("fueltype.scxml", FUEL_ENUM);
    let doc = t.write("names_one.scxml", NAMES_ONE);
    let (code, out, _) = coverage(&doc);
    assert_eq!(code, Some(0));
    assert!(
        !out.trim().is_empty(),
        "the fixture should report something"
    );
}

#[test]
fn a_variant_reference_resolves_to_a_name_the_backend_declares() {
    // ⚠ THE DEFECT THIS PINS. `Fuel.DSL` in an expression used to be
    // lowered by case-converting the ALIAS — emitting `fuel.DSL`, a name
    // bound nowhere, with exit 0. Python and C emitted it verbatim; C++
    // emitted `Fuel.DSL` with a `.` where `::` belongs. Three backends
    // produced code that cannot compile, and the generator said nothing.
    //
    // ⚠⚠ THE ASSERTION IS "NO UNBOUND ALIAS", NOT "equals this string".
    // Pinning the exact qualified text per backend would restate the
    // spelling matrix here — a third copy of the thing `enum_naming`
    // exists to own. What must never appear is the alias as a bare
    // lowercase object, which is the defect's signature.
    let t = Tmp::new("variant_ref");
    t.write("fueltype.scxml", FUEL_ENUM);
    let doc = t.write("names_one.scxml", NAMES_ONE);
    for (lang, ext) in [
        ("python", "py"),
        ("cpp", "h"),
        ("rust", "rs"),
        ("kotlin", "kt"),
        ("c", "h"),
    ] {
        let out = t.0.join(format!("ref_{lang}"));
        std::fs::create_dir_all(&out).expect("create output dir");
        let run = Command::new(bin())
            .args([
                "generate",
                doc.to_str().unwrap(),
                "-l",
                lang,
                "-o",
                out.to_str().unwrap(),
            ])
            .output()
            .expect("spawn sce-codegen");
        assert_eq!(
            run.status.code(),
            Some(0),
            "{lang}: generation failed:\n{}",
            String::from_utf8_lossy(&run.stderr)
        );
        let body: String = std::fs::read_dir(&out)
            .expect("read output dir")
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some(ext))
            .map(|e| std::fs::read_to_string(e.path()).unwrap_or_default())
            .collect();
        assert!(!body.is_empty(), "{lang}: no .{ext} artifact to inspect");
        // The defect's signature: the ALIAS appearing as the object of
        // the member access. The fixture's alias (`Fuel`) is unlike the
        // document's identity (`FuelType` / `fueltype.scxml`), so a
        // correct qualified reference can never contain these.
        for unbound in ["fuel.DSL", "Fuel.DSL"] {
            assert!(
                !body.contains(unbound),
                "{lang}: emitted the unresolved alias `{unbound}` — the variant \
                 reference was not lowered:\n{body}"
            );
        }
        // And it does name the variant somehow — a lowering that dropped
        // the comparison entirely would pass the negative check alone.
        assert!(
            body.contains("DSL"),
            "{lang}: the variant is not referenced at all:\n{body}"
        );
    }
}

#[test]
fn an_enum_typed_input_generates_instead_of_panicking() {
    // ⚠ NOT a coverage test — a crash this file's fixtures are the first
    // in the tree to reach. Every per-language `*_type` helper answers an
    // Enum with `unreachable!` by design (the alias resolves only through
    // the import table), and the transform parameter list called them
    // directly. So the first document to put a value space ON AN INPUT
    // rather than flattening it to booleans panicked the generator.
    //
    // ⚠⚠ The enum kind had tests. The transform kind had tests. The PAIR
    // had none, and that is where it broke — which is why this assertion
    // lives next to the feature that first needed the pair.
    let t = Tmp::new("enum_param");
    t.write("fueltype.scxml", FUEL_ENUM);
    let doc = t.write("names_one.scxml", NAMES_ONE);
    for lang in ["python", "cpp", "rust", "kotlin", "c"] {
        let out = t.0.join(format!("out_{lang}"));
        std::fs::create_dir_all(&out).expect("create output dir");
        let run = Command::new(bin())
            .args([
                "generate",
                doc.to_str().unwrap(),
                "-l",
                lang,
                "-o",
                out.to_str().unwrap(),
            ])
            .output()
            .expect("spawn sce-codegen");
        let err = String::from_utf8_lossy(&run.stderr);
        assert!(
            !err.contains("panicked"),
            "{lang}: generator panicked on an enum-typed transform input:\n{err}"
        );
        assert_eq!(
            run.status.code(),
            Some(0),
            "{lang}: generation failed:\n{err}"
        );
    }
}

// ── `sce:default-covers` — the answering half ──────────────────────
//
// ⚠ WHY THE ANSWER IS PART OF THE FEATURE. A report the author cannot
// respond to re-asks the same question on every run. The corpus that
// prompted this surface has one document where eleven of fourteen
// variants share a default BY DESIGN — the specification's own table
// says "Others" — so without a way to record that, the question list
// never shrinks, and a list that never shrinks is one nobody reads.
//
// ⚠⚠ THE CLAIM NAMES VARIANTS SO IT EXPIRES. That is the property
// `an_acknowledgement_does_not_cover_a_variant_added_later` holds in
// place, and it is the reason this is not a boolean.

/// Acknowledges the three variants the document does not name.
const NAMES_ONE_ACKNOWLEDGED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="names_one">
  <sce:import as="Fuel" src="fueltype.scxml" kind="enum"/>
  <datamodel>
    <data id="fuel" sce:type="enum:Fuel" sce:direction="in"
          sce:default-covers="GSL LPI OTHERS"/>
    <data id="lamp" sce:type="int32" sce:direction="out"
          expr="fuel === Fuel.DSL ? 1 : 0"/>
  </datamodel>
</scxml>
"#;

/// A five-variant version of `FUEL_ENUM` — what the platform model
/// looks like the day a value is added upstream.
const FUEL_ENUM_GROWN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="enum" name="FuelType" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="GSL" value="0"/>
      <sce:variant name="DSL" value="1"/>
      <sce:variant name="LPI" value="2"/>
      <sce:variant name="OTHERS" value="3"/>
      <sce:variant name="HYDROGEN" value="4"/>
    </data>
  </datamodel>
</scxml>
"#;

fn generate_rc(doc: &std::path::Path, out: &std::path::Path) -> (Option<i32>, String) {
    std::fs::create_dir_all(out).expect("create output dir");
    let run = Command::new(bin())
        .args([
            "generate",
            doc.to_str().unwrap(),
            "-l",
            "rust",
            "-o",
            out.to_str().unwrap(),
            // ⚠ The wire form, not the human one. A diagnostic's CODE is
            // what a consumer routes on and what this file asserts;
            // human mode prints the message alone, so asserting a code
            // against it measures the message's wording instead.
            "--error-format=json",
        ])
        .output()
        .expect("spawn sce-codegen");
    (
        run.status.code(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&run.stdout),
            String::from_utf8_lossy(&run.stderr)
        ),
    )
}

#[test]
fn an_acknowledged_variant_stops_being_a_question() {
    let t = Tmp::new("ack");
    t.write("fueltype.scxml", FUEL_ENUM);
    let doc = t.write("names_one.scxml", NAMES_ONE_ACKNOWLEDGED);
    let (code, out, err) = coverage(&doc);
    assert_eq!(code, Some(0), "coverage failed:\n{err}");
    assert!(
        out.trim().is_empty(),
        "every uncovered variant was acknowledged by name, and the report \
         asked anyway:\n{out}"
    );
    // ⚠ And the document still builds — the acknowledgement is a claim
    // about the value space, not a change to what the code does.
    let (rc, log) = generate_rc(&doc, &t.0.join("ack_out"));
    assert_eq!(rc, Some(0), "an acknowledged document was refused:\n{log}");
}

#[test]
fn an_acknowledgement_does_not_cover_a_variant_added_later() {
    // ⚠⚠⚠ THE PROPERTY THE WHOLE DESIGN EXISTS FOR. The enum document is
    // generated from a platform model, so its variant list GROWS without
    // the transform's author touching anything. A boolean "the default is
    // deliberate" would go silent forever on that day — precisely the day
    // the author has something new to decide.
    //
    // The document below is byte-for-byte the one that reported nothing
    // in the test above. The only thing that changed is the value space.
    let t = Tmp::new("ack_grown");
    t.write("fueltype.scxml", FUEL_ENUM_GROWN);
    let doc = t.write("names_one.scxml", NAMES_ONE_ACKNOWLEDGED);
    let (code, out, err) = coverage(&doc);
    assert_eq!(code, Some(0), "coverage failed:\n{err}");
    let lines: Vec<&str> = out.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(
        lines.len(),
        1,
        "a variant added to the value space after the acknowledgement was \
         written did not come back as a question:\n{out}"
    );
    assert!(
        lines[0].contains("\"uncovered\":[\"HYDROGEN\"]"),
        "the question should be about the NEW variant alone — the three \
         acknowledged ones were answered:\n{}",
        lines[0]
    );
}

/// An acknowledgement naming something the value space does not declare
/// — the shape a renamed variant leaves behind.
const ACK_UNKNOWN_NAME: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="names_one">
  <sce:import as="Fuel" src="fueltype.scxml" kind="enum"/>
  <datamodel>
    <data id="fuel" sce:type="enum:Fuel" sce:direction="in"
          sce:default-covers="GSL LPI KEROSENE"/>
    <data id="lamp" sce:type="int32" sce:direction="out"
          expr="fuel === Fuel.DSL ? 1 : 0"/>
  </datamodel>
</scxml>
"#;

/// An acknowledgement naming a variant the document itself tests for.
const ACK_TESTED_NAME: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="names_one">
  <sce:import as="Fuel" src="fueltype.scxml" kind="enum"/>
  <datamodel>
    <data id="fuel" sce:type="enum:Fuel" sce:direction="in"
          sce:default-covers="GSL LPI DSL"/>
    <data id="lamp" sce:type="int32" sce:direction="out"
          expr="fuel === Fuel.DSL ? 1 : 0"/>
  </datamodel>
</scxml>
"#;

/// An acknowledgement on a field with no value space at all.
const ACK_ON_A_BOOL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="names_one">
  <sce:import as="Fuel" src="fueltype.scxml" kind="enum"/>
  <datamodel>
    <data id="fuel" sce:type="enum:Fuel" sce:direction="in"/>
    <data id="warnOn" sce:type="bool" sce:direction="in"
          sce:default-covers="GSL LPI"/>
    <data id="lamp" sce:type="int32" sce:direction="out"
          expr="fuel === Fuel.DSL ? 1 : 0"/>
  </datamodel>
</scxml>
"#;

#[test]
fn a_false_acknowledgement_is_refused_by_the_build() {
    // ⚠ THE GAP IS A REPORT; THE CLAIM IS A REFUSAL. Falling through to a
    // default is legal, which is why `coverage` is exit-0 about it. A
    // document asserting something untrue ABOUT ITSELF is the other
    // thing — the same failure `validation/unknown-sce-attribute` exists
    // to prevent, where the author's sentence and the machine's
    // behaviour part company in silence.
    //
    // ⚠⚠ AND IT MUST BE `generate`, NOT `coverage`. An author who could
    // silence the question with a name that means nothing would have a
    // worse tool than one with no way to silence it, so the check runs
    // on every build rather than when someone asks for the report.
    for (label, body, code) in [
        (
            "unknown",
            ACK_UNKNOWN_NAME,
            "validation/default-covers-unknown-variant",
        ),
        (
            "tested",
            ACK_TESTED_NAME,
            "validation/default-covers-tested-variant",
        ),
        (
            "not_a_value_space",
            ACK_ON_A_BOOL,
            "validation/default-covers-not-a-value-space",
        ),
    ] {
        let t = Tmp::new(&format!("ack_false_{label}"));
        t.write("fueltype.scxml", FUEL_ENUM);
        let doc = t.write("names_one.scxml", body);
        let (rc, log) = generate_rc(&doc, &t.0.join("out"));
        assert_ne!(
            rc,
            Some(0),
            "{label}: a false sce:default-covers claim generated cleanly:\n{log}"
        );
        assert!(
            log.contains(code),
            "{label}: refused, but not as {code}:\n{log}"
        );
    }
}

#[test]
fn a_statechart_is_answered_with_silence_not_an_error() {
    // A consumer sweeping a whole tree must not have to pre-filter by
    // kind: a document with no declared value spaces has nothing to
    // report, and that is not a failure.
    let t = Tmp::new("statechart");
    let doc = t.write(
        "plain.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       initial="idle" datamodel="null">
  <state id="idle"/>
</scxml>
"#,
    );
    let (code, out, err) = coverage(&doc);
    assert_eq!(code, Some(0), "a statechart was refused:\n{err}");
    assert!(
        out.trim().is_empty(),
        "a statechart produced a record:\n{out}"
    );
}
