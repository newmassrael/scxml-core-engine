//! `<sce:cycle>` — an ordered sequence of alternatives the document
//! states, over a value space it imports.
//!
//! ⚠ THE DESIGN THIS FILE HOLDS IN PLACE WAS THE SECOND ONE. The first
//! walked the imported enum's declaration order and skipped the
//! unavailable variants: no new element, one intrinsic, and wrong. In
//! the surveyed material the specification's list reads
//! ECO · NORMAL · SPORT · CHAUFFEUR · MY · SMART · SNOW while the
//! platform's value space declares the same alternatives
//! ECO · NORMAL · SPORT · MY · SMART · SNOW · CHAUFFEUR. The first three
//! agree, so a fixture that only ever steps one place would have passed
//! while "the next alternative" was wrong for every author who got as
//! far as the fourth.
//!
//! `an_order_is_the_documents_not_the_value_spaces` is that measurement
//! turned into an assertion: the two orders DISAGREE in the fixture, on
//! purpose, so a future change that reads the order off the enum cannot
//! pass.
//!
//! ⚠⚠ WHAT IS NOT HERE YET. This file covers the DECLARATION and its
//! checks. Navigation over a cycle (`next` / `prev` / `first` and the
//! snap invariant) is a separate change with its own lowering per
//! backend; declaring the order is what it will stand on, and landing
//! the two together would mean neither could be reviewed on its own.

use std::process::Command;

fn bin() -> String {
    env!("CARGO_BIN_EXE_sce-codegen").to_string()
}

struct Tmp(std::path::PathBuf);

impl Tmp {
    fn new(label: &str) -> Self {
        let d = std::env::temp_dir().join(format!("sce_cycle_{label}_{}", std::process::id()));
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

/// ⚠ THE VALUE SPACE'S ORDER IS DELIBERATELY NOT THE DOCUMENT'S. It
/// declares CHAUFFEUR last; every cycle below puts CHAUFFEUR fourth.
/// That disagreement is the fixture's whole job — see the module note.
const MODE_ENUM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="enum" name="DriveMode" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="ECO" value="0"/>
      <sce:variant name="NORMAL" value="1"/>
      <sce:variant name="SPORT" value="2"/>
      <sce:variant name="MY" value="3"/>
      <sce:variant name="SMART" value="4"/>
      <sce:variant name="SNOW" value="5"/>
      <sce:variant name="CHAUFFEUR" value="6"/>
      <sce:variant name="OFF" value="7"/>
    </data>
  </datamodel>
</scxml>
"#;

fn doc(cycle: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="modes">
  <sce:import as="Mode" src="drivemode.scxml" kind="enum"/>
{cycle}
  <datamodel>
    <data id="cursor" sce:type="enum:Mode" sce:direction="in"/>
    <data id="lamp" sce:type="int32" sce:direction="out"
          expr="cursor === Mode.SPORT ? 1 : 0"/>
  </datamodel>
</scxml>
"#
    )
}

/// The document's own order, which is not the value space's.
const SPEC_ORDER: &str = r#"  <sce:cycle id="basicModes" of="Mode">
    <sce:step name="ECO"/>
    <sce:step name="NORMAL"/>
    <sce:step name="SPORT"/>
    <sce:step name="CHAUFFEUR"/>
    <sce:step name="MY"/>
    <sce:step name="SMART"/>
    <sce:step name="SNOW"/>
  </sce:cycle>"#;

fn build(t: &Tmp, body: &str, label: &str) -> (Option<i32>, String) {
    let d = t.write("modes.scxml", body);
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

/// ⚠ `--emit-ast` takes a PATH, not a stream. The first version of this
/// helper read stdout and got the manifest line instead, so the test
/// failed saying the AST "does not carry ECO" when what it actually had
/// was no AST at all — a reminder that an empty read and a wrong read
/// look identical from the assertion's side.
fn emit_ast(t: &Tmp, body: &str) -> String {
    let d = t.write("modes.scxml", body);
    let out = t.0.join("ast_out");
    std::fs::create_dir_all(&out).expect("create output dir");
    let ast = t.0.join("ast.json");
    let run = Command::new(bin())
        .args([
            "generate",
            d.to_str().unwrap(),
            "-l",
            "rust",
            "-o",
            out.to_str().unwrap(),
            "--emit-ast",
            ast.to_str().unwrap(),
        ])
        .output()
        .expect("spawn sce-codegen");
    assert_eq!(
        run.status.code(),
        Some(0),
        "emit-ast generation failed:\n{}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    std::fs::read_to_string(&ast).expect("the AST file the flag named")
}

#[test]
fn a_declared_cycle_builds() {
    let t = Tmp::new("ok");
    t.write("drivemode.scxml", MODE_ENUM);
    let (rc, log) = build(&t, &doc(SPEC_ORDER), "ok");
    assert_eq!(rc, Some(0), "a well-formed cycle was refused:\n{log}");
}

#[test]
fn an_order_is_the_documents_not_the_value_spaces() {
    // ⚠⚠⚠ THE ASSERTION THE WHOLE ELEMENT EXISTS FOR. The fixture's
    // enum declares CHAUFFEUR LAST and the cycle puts it FOURTH. If a
    // future change ever derives the order from the value space — the
    // design this replaced — the sequence carried into the AST comes
    // back in the enum's order and this fails.
    //
    // Reading the order out of the AST rather than out of generated code
    // is deliberate: the AST is where a consumer would read it, and a
    // backend that does not yet lower navigation would otherwise make
    // this untestable.
    let t = Tmp::new("order");
    t.write("drivemode.scxml", MODE_ENUM);
    let ast = emit_ast(&t, &doc(SPEC_ORDER));
    let spec_order = ["ECO", "NORMAL", "SPORT", "CHAUFFEUR", "MY", "SMART", "SNOW"];

    let mut at = Vec::new();
    for name in spec_order {
        let needle = format!("\"{name}\"");
        at.push(
            ast.find(&needle)
                .unwrap_or_else(|| panic!("the AST does not carry step `{name}`:\n{ast}")),
        );
    }
    for w in at.windows(2) {
        assert!(
            w[0] < w[1],
            "the cycle's steps are not in the DOCUMENT's order — \
             the value space declares CHAUFFEUR last and the document \
             puts it fourth, so an order read off the enum is the \
             defect this asserts against:\n{ast}"
        );
    }
}

#[test]
fn a_step_that_names_no_value_is_refused() {
    // A stop nothing declares is not a stop that does nothing: the stops
    // ARE positions, so it lengthens the sequence and shifts every stop
    // after it. The defect would surface as "the wrong alternative"
    // somewhere else entirely.
    let t = Tmp::new("bad_step");
    t.write("drivemode.scxml", MODE_ENUM);
    let cycle = r#"  <sce:cycle id="basicModes" of="Mode">
    <sce:step name="ECO"/>
    <sce:step name="NORMAL"/>
    <sce:step name="SPRT"/>
  </sce:cycle>"#;
    let (rc, log) = build(&t, &doc(cycle), "bad_step");
    assert_ne!(rc, Some(0), "a step naming no variant built:\n{log}");
    assert!(
        log.contains("validation/invalid-attribute"),
        "refused, but not as validation/invalid-attribute:\n{log}"
    );
    assert!(
        log.contains("SPORT"),
        "the refusal does not offer the values it could have been:\n{log}"
    );
}

#[test]
fn a_cycle_over_a_value_space_that_is_not_imported_is_refused() {
    let t = Tmp::new("bad_of");
    t.write("drivemode.scxml", MODE_ENUM);
    let cycle = r#"  <sce:cycle id="basicModes" of="Modes">
    <sce:step name="ECO"/>
    <sce:step name="NORMAL"/>
  </sce:cycle>"#;
    let (rc, log) = build(&t, &doc(cycle), "bad_of");
    assert_ne!(
        rc,
        Some(0),
        "a cycle over an unimported alias built:\n{log}"
    );
    assert!(
        log.contains("validation/invalid-attribute") && log.contains("Mode"),
        "the refusal does not name the aliases that ARE imported:\n{log}"
    );
}

#[test]
fn a_cycle_of_fewer_than_two_stops_is_refused() {
    // `next` and `prev` on a one-element cycle both answer that element,
    // so every navigation is a no-op. The document would describe a
    // choice the user cannot make and nothing downstream would say so.
    let t = Tmp::new("too_short");
    t.write("drivemode.scxml", MODE_ENUM);
    let cycle = r#"  <sce:cycle id="basicModes" of="Mode">
    <sce:step name="ECO"/>
  </sce:cycle>"#;
    let (rc, log) = build(&t, &doc(cycle), "too_short");
    assert_ne!(rc, Some(0), "a one-stop cycle built:\n{log}");
}

#[test]
fn a_repeated_stop_is_refused() {
    // One name in two positions is two different cursor positions with
    // the same value — `next` from either is ambiguous, and picking the
    // first silently is a guess about which the author meant.
    let t = Tmp::new("dup_step");
    t.write("drivemode.scxml", MODE_ENUM);
    let cycle = r#"  <sce:cycle id="basicModes" of="Mode">
    <sce:step name="ECO"/>
    <sce:step name="NORMAL"/>
    <sce:step name="ECO"/>
  </sce:cycle>"#;
    let (rc, log) = build(&t, &doc(cycle), "dup_step");
    assert_ne!(rc, Some(0), "a cycle with a repeated stop built:\n{log}");
}

#[test]
fn one_value_space_carries_several_cycles() {
    // ⚠ The surveyed material has two lists over overlapping
    // alternatives — a basic-mode list and an off-road list, with one
    // name on both — which is why a cycle is named rather than being a
    // property of the value space. A design that hung the order on the
    // enum could not express this at all.
    let t = Tmp::new("two_cycles");
    t.write("drivemode.scxml", MODE_ENUM);
    let cycles = r#"  <sce:cycle id="basicModes" of="Mode">
    <sce:step name="ECO"/>
    <sce:step name="NORMAL"/>
    <sce:step name="SNOW"/>
  </sce:cycle>
  <sce:cycle id="offRoadModes" of="Mode">
    <sce:step name="SNOW"/>
    <sce:step name="SPORT"/>
  </sce:cycle>"#;
    let (rc, log) = build(&t, &doc(cycles), "two_cycles");
    assert_eq!(
        rc,
        Some(0),
        "two cycles over one value space were refused — the same \
         alternative may be a stop on both:\n{log}"
    );
}

#[test]
fn two_cycles_may_not_share_an_id() {
    let t = Tmp::new("dup_id");
    t.write("drivemode.scxml", MODE_ENUM);
    let cycles = r#"  <sce:cycle id="modes" of="Mode">
    <sce:step name="ECO"/>
    <sce:step name="NORMAL"/>
  </sce:cycle>
  <sce:cycle id="modes" of="Mode">
    <sce:step name="SNOW"/>
    <sce:step name="SPORT"/>
  </sce:cycle>"#;
    let (rc, log) = build(&t, &doc(cycles), "dup_id");
    assert_ne!(rc, Some(0), "two cycles shared an id:\n{log}");
}

#[test]
fn a_step_may_carry_the_condition_that_makes_it_present() {
    // The source notation writes one table whose rows are the
    // alternatives in order, each carrying the condition under which it
    // is present. Keeping the condition on the step is what removes the
    // hand-typed bit positions an availability mask would have needed.
    let t = Tmp::new("when");
    t.write("drivemode.scxml", MODE_ENUM);
    let cycle = r#"  <sce:cycle id="basicModes" of="Mode">
    <sce:step name="ECO" when="ecoOn"/>
    <sce:step name="NORMAL"/>
    <sce:step name="SNOW" when="snowOn"/>
  </sce:cycle>"#;
    let body = doc(cycle).replace(
        r#"    <data id="cursor" sce:type="enum:Mode" sce:direction="in"/>"#,
        r#"    <data id="cursor" sce:type="enum:Mode" sce:direction="in"/>
    <data id="ecoOn" sce:type="bool" sce:direction="in"/>
    <data id="snowOn" sce:type="bool" sce:direction="in"/>"#,
    );
    let (rc, log) = build(&t, &body, "when");
    assert_eq!(rc, Some(0), "a step condition was refused:\n{log}");
}
