//! `sce:retain` / `sce:initial` — a field whose value outlives the program.
//!
//! ⚠ THE MEASUREMENT THAT PROMPTED IT. This tree's NL→IR survey read one
//! specification that says a mode "keeps its previous state not only
//! across ignition-off but across battery-off", and gives a table of
//! factory defaults for seven outputs. Neither fact had anywhere to live:
//! the platform model carries a slot's type and value space and says
//! nothing about persistence, and SCE had no way to say it either. Swept
//! across the whole corpus the same sentence shape appears in 23 sections
//! and the factory-default table in 12, so it is that domain's ordinary
//! vocabulary rather than one document's quirk.
//!
//! ⚠⚠ WHAT IS BEING HELD IN PLACE IS A DIVISION OF LABOUR, not a
//! persistence implementation. SCE cannot keep a value between runs. It
//! declares that one is kept, refuses the two orphan shapes, and checks
//! the single question it can answer — whether the initial value is one
//! the field's type could ever hold.
//!
//! ⚠⚠⚠ THE SCOPE LABEL IS NOT VALIDATED, ON PURPOSE, and
//! `an_unknown_scope_label_is_accepted_because_scopes_are_the_domains`
//! is what stops that from silently becoming a list of automotive words
//! inside a general tool.

use std::process::Command;

fn bin() -> String {
    env!("CARGO_BIN_EXE_sce-codegen").to_string()
}

struct Tmp(std::path::PathBuf);

impl Tmp {
    fn new(label: &str) -> Self {
        let d = std::env::temp_dir().join(format!("sce_retain_{label}_{}", std::process::id()));
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

const MODE_ENUM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="enum" name="DriveMode" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="ECO" value="0"/>
      <sce:variant name="NORMAL" value="1"/>
      <sce:variant name="SPORT" value="2"/>
    </data>
  </datamodel>
</scxml>
"#;

/// The shape the survey needs: the previous mode is an input, retained
/// across power cycles, seeded with a factory default.
fn doc(retain_attrs: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="mode_hold">
  <sce:import as="Mode" src="drivemode.scxml" kind="enum"/>
  <datamodel>
    <data id="prevMode" sce:type="enum:Mode" sce:direction="in"{retain_attrs}/>
    <data id="ignOn" sce:type="bool" sce:direction="in"/>
    <data id="mode" sce:type="int32" sce:direction="out"
          expr="!ignOn ? 0 : prevMode === Mode.SPORT ? 2 : 1"/>
  </datamodel>
</scxml>
"#
    )
}

fn generate(t: &Tmp, body: &str, label: &str) -> (Option<i32>, String) {
    let d = t.write("mode_hold.scxml", body);
    let out = t.0.join(format!("out_{label}"));
    std::fs::create_dir_all(&out).expect("create output dir");
    let run = Command::new(bin())
        .args([
            "generate",
            d.to_str().unwrap(),
            "-l",
            "rust",
            "-o",
            out.to_str().unwrap(),
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
fn a_retained_field_with_an_initial_value_generates() {
    let t = Tmp::new("ok");
    t.write("drivemode.scxml", MODE_ENUM);
    let (rc, log) = generate(
        &t,
        &doc(r#" sce:retain="battery" sce:initial="NORMAL""#),
        "ok",
    );
    assert_eq!(
        rc,
        Some(0),
        "a well-formed retained field was refused:\n{log}"
    );
}

#[test]
fn each_half_of_the_pair_requires_the_other() {
    // ⚠ BOTH DIRECTIONS, because each orphan is silent in its own way.
    // `sce:retain` alone leaves the field with no value on the first run,
    // before anything has ever been stored. `sce:initial` alone is read
    // by nothing, because a field that is not retained is computed
    // afresh every cycle — the invisible-attribute failure that
    // `validation/unknown-sce-attribute` exists to prevent, reached here
    // by a name that IS on the roster.
    for (label, attrs, missing) in [
        ("retain_alone", r#" sce:retain="battery""#, "sce:initial"),
        ("initial_alone", r#" sce:initial="NORMAL""#, "sce:retain"),
    ] {
        let t = Tmp::new(label);
        t.write("drivemode.scxml", MODE_ENUM);
        let (rc, log) = generate(&t, &doc(attrs), label);
        assert_ne!(
            rc,
            Some(0),
            "{label}: an orphan half generated cleanly:\n{log}"
        );
        // A missing partner is a rule the pair breaks, not a value
        // outside a set, so no candidate list rides the refusal.
        assert!(
            log.contains("validation/attribute-rule-violated"),
            "{label}: refused, but not as validation/attribute-rule-violated:\n{log}"
        );
        assert!(
            log.contains(missing),
            "{label}: the refusal does not name the missing partner `{missing}`:\n{log}"
        );
    }
}

#[test]
fn an_initial_value_outside_the_value_space_is_refused() {
    // ⚠⚠ THE ASSERTION THIS WHOLE SURFACE IS FOR. The initial value is
    // taken on the day a system is first switched on and on no other
    // day, so it is the value least likely to be reached by any test and
    // the one whose mistake survives longest. `SPORTS` is the misspelling
    // a reader's eye slides over; nothing else in the document would ever
    // reject it.
    let t = Tmp::new("bad_variant");
    t.write("drivemode.scxml", MODE_ENUM);
    let (rc, log) = generate(
        &t,
        &doc(r#" sce:retain="battery" sce:initial="SPORTS""#),
        "bad_variant",
    );
    assert_ne!(
        rc,
        Some(0),
        "an initial value naming no variant generated:\n{log}"
    );
    assert!(
        log.contains("validation/invalid-attribute"),
        "refused, but not as validation/invalid-attribute:\n{log}"
    );
    // And it says what could have been written — the repair, not just the
    // complaint.
    for variant in ["ECO", "NORMAL", "SPORT"] {
        assert!(
            log.contains(variant),
            "the refusal does not offer `{variant}` as a candidate:\n{log}"
        );
    }
}

#[test]
fn an_out_of_range_integer_initial_is_refused() {
    // The same question one type down: `int_value_range` answers for
    // every integer width, so the bound comes from the declared type
    // rather than from a list that would go stale beside it.
    let t = Tmp::new("bad_int");
    let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="counter">
  <datamodel>
    <data id="count" sce:type="uint8" sce:direction="in"
          sce:retain="battery" sce:initial="300"/>
    <data id="doubled" sce:type="int32" sce:direction="out" expr="count * 2"/>
  </datamodel>
</scxml>
"#;
    let (rc, log) = generate(&t, body, "bad_int");
    assert_ne!(
        rc,
        Some(0),
        "300 was accepted as a uint8 initial value:\n{log}"
    );
    // An integer range is a rule no list of values can state.
    assert!(
        log.contains("validation/attribute-rule-violated"),
        "refused, but not as validation/attribute-rule-violated:\n{log}"
    );
    assert!(
        log.contains("255"),
        "the refusal does not state the range it broke:\n{log}"
    );
}

#[test]
fn an_unknown_scope_label_is_accepted_because_scopes_are_the_domains() {
    // ⚠⚠⚠ THE GENERALITY CONTROL. The measurement behind this surface is
    // automotive — one store survives ignition-off, another survives
    // battery removal — and it would have been easy to enumerate those
    // two. Then SCE would carry one domain's vocabulary, and the next
    // domain's `"sd-card"` would be a build failure with no defect
    // behind it.
    //
    // So the scope is opaque: SCE compares it for equality and never
    // interprets it, exactly as it refuses to normalise a `sce:req` id.
    // This test is what keeps that true — without it, someone adding a
    // "sensible" list of known scopes would break no test at all.
    let t = Tmp::new("odd_scope");
    t.write("drivemode.scxml", MODE_ENUM);
    let (rc, log) = generate(
        &t,
        &doc(r#" sce:retain="a-store-this-tree-has-never-heard-of" sce:initial="ECO""#),
        "odd_scope",
    );
    assert_eq!(
        rc,
        Some(0),
        "an unfamiliar scope label was refused — scopes are the domain's \
         to name, not SCE's to enumerate:\n{log}"
    );
}
