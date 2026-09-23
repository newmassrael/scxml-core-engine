//! Navigating a `<sce:cycle>` — `cycle_has` / `cycle_first` /
//! `cycle_next` / `cycle_prev`.
//!
//! ⚠ THESE ARE EXPANDED, NOT LOWERED. The four calls are rewritten into
//! ordinary conditionals once, before any backend sees the document
//! ([`sce_build::forge::cycle_expand`]), because nothing about walking a
//! declared list differs per language — unlike `round`, whose halfway
//! rule genuinely does. So there is no per-backend arm to test; what
//! needs holding down is the REWRITE, and that it produces an expression
//! every backend already accepts.
//!
//! ⚠⚠ THE ARITHMETIC IS CHECKED BY RUNNING IT. Python is the one target
//! this test can execute without a toolchain, so the expansion's answers
//! are asserted against a table of cursor-and-availability cases. The
//! other five are checked for *acceptance* — that the expanded text is
//! something they can compile — which is the half that would otherwise
//! rot when a sixth conditional form is added.
//!
//! ⚠⚠⚠ THE BOUNDARY CASES ARE THE POINT. Wrapping, a cursor sitting on
//! a stop that is no longer present, a cursor that is not a stop at all,
//! and nothing present at all: each is a place where a plausible
//! implementation quietly picks a different answer, and each is asserted
//! below with the reason the chosen answer is the right one.

use std::process::Command;

fn bin() -> String {
    env!("CARGO_BIN_EXE_sce-codegen").to_string()
}

struct Tmp(std::path::PathBuf);

impl Tmp {
    /// ⚠ The nonce is load-bearing. Naming the directory after the label
    /// and the process id alone collided when two tests in this file
    /// both used the label `python`: `cargo test` runs them as threads of
    /// ONE process, so the paths were identical and whichever finished
    /// first deleted the other's artifacts on `Drop`. The failure read as
    /// "python cannot find the generated module", which points at the
    /// feature rather than at the harness.
    fn new(label: &str) -> Self {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NONCE: AtomicUsize = AtomicUsize::new(0);
        let d = std::env::temp_dir().join(format!(
            "sce_cyclenav_{label}_{}_{}",
            std::process::id(),
            NONCE.fetch_add(1, Ordering::Relaxed)
        ));
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

/// Four stops plus an `OFF` that is deliberately NOT a stop — the
/// "cursor is not on the cycle" case needs a value the cycle does not
/// contain.
const MODE_ENUM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="enum" name="DriveMode" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="ECO" value="0"/>
      <sce:variant name="NORMAL" value="1"/>
      <sce:variant name="SPORT" value="2"/>
      <sce:variant name="SNOW" value="3"/>
      <sce:variant name="OFF" value="9"/>
    </data>
  </datamodel>
</scxml>
"#;

/// Every stop carries a condition, so availability can be driven from
/// the test. The outputs are integers because that is what the harness
/// can read back without knowing each backend's enum spelling.
const DOC: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="modes">
  <sce:import as="Mode" src="drivemode.scxml" kind="enum"/>
  <sce:cycle id="modes" of="Mode">
    <sce:step name="ECO"    when="ecoOn"/>
    <sce:step name="NORMAL" when="normalOn"/>
    <sce:step name="SPORT"  when="sportOn"/>
    <sce:step name="SNOW"   when="snowOn"/>
  </sce:cycle>
  <datamodel>
    <data id="cursor" sce:type="enum:Mode" sce:direction="in"/>
    <data id="ecoOn" sce:type="bool" sce:direction="in"/>
    <data id="normalOn" sce:type="bool" sce:direction="in"/>
    <data id="sportOn" sce:type="bool" sce:direction="in"/>
    <data id="snowOn" sce:type="bool" sce:direction="in"/>

    <data id="here" sce:type="bool" sce:direction="out"
          expr="cycle_has(modes, cursor)"/>
    <data id="firstOne" sce:type="int32" sce:direction="out"
          expr="cycle_first(modes, cursor)"/>
    <data id="nextOne" sce:type="int32" sce:direction="out"
          expr="cycle_next(modes, cursor)"/>
    <data id="prevOne" sce:type="int32" sce:direction="out"
          expr="cycle_prev(modes, cursor)"/>
  </datamodel>
</scxml>
"#;

fn generate(t: &Tmp, lang: &str) -> (Option<i32>, std::path::PathBuf, String) {
    let d = t.write("modes.scxml", DOC);
    let out = t.0.join(format!("out_{lang}"));
    std::fs::create_dir_all(&out).expect("create output dir");
    let mut cmd = Command::new(bin());
    cmd.args([
        "generate",
        d.to_str().unwrap(),
        "-l",
        lang,
        "-o",
        out.to_str().unwrap(),
        "--error-format=json",
    ]);
    // Go's imports are module-qualified and have no bare form, so a
    // document with an `<sce:import>` needs the module path. Nothing to
    // do with cycles — it is what this backend requires of any importing
    // document.
    if lang == "go" {
        cmd.args(["--go-module-prefix", "example.com/generated"]);
    }
    let run = cmd.output().expect("spawn sce-codegen");
    (
        run.status.code(),
        out,
        format!(
            "{}{}",
            String::from_utf8_lossy(&run.stdout),
            String::from_utf8_lossy(&run.stderr)
        ),
    )
}

/// ECO=0 NORMAL=1 SPORT=2 SNOW=3 OFF=9 — the enum's own values, which
/// the integer-typed outputs carry.
const ECO: u8 = 0;
const NORMAL: u8 = 1;
const SPORT: u8 = 2;
const SNOW: u8 = 3;
const OFF: u8 = 9;

struct Case {
    cursor: u8,
    present: [bool; 4],
    has: bool,
    first: u8,
    next: u8,
    prev: u8,
    why: &'static str,
}

const CASES: &[Case] = &[
    Case {
        cursor: NORMAL,
        present: [true, true, true, true],
        has: true,
        first: ECO,
        next: SPORT,
        prev: ECO,
        why: "everything present: one step each way, and first is the head",
    },
    Case {
        cursor: SNOW,
        present: [true, true, true, true],
        has: true,
        first: ECO,
        next: ECO,
        prev: SPORT,
        why: "the last stop wraps forward to the head",
    },
    Case {
        cursor: ECO,
        present: [true, true, true, true],
        has: true,
        first: ECO,
        next: NORMAL,
        prev: SNOW,
        why: "the head wraps backward to the tail",
    },
    Case {
        cursor: ECO,
        present: [true, false, false, true],
        has: true,
        first: ECO,
        next: SNOW,
        prev: SNOW,
        why: "absent stops are skipped in both directions",
    },
    Case {
        cursor: NORMAL,
        present: [true, false, true, false],
        has: false,
        first: ECO,
        next: SPORT,
        prev: ECO,
        why: "a cursor on an ABSENT stop still navigates from its \
              position — `has` is false, and the document is what decides \
              whether to snap",
    },
    Case {
        cursor: OFF,
        present: [true, true, true, true],
        has: false,
        first: ECO,
        next: OFF,
        prev: OFF,
        why: "a cursor that is not a stop at all does not move: walking \
              the cycle is the only thing these do, and it is not on it. \
              Standing still is the loud failure; silently jumping to the \
              head would hide a document that forgot its snap rule",
    },
    Case {
        cursor: NORMAL,
        present: [false, false, false, false],
        has: false,
        first: NORMAL,
        next: NORMAL,
        prev: NORMAL,
        why: "nothing present: `first` answers the cursor it was given, \
              because naming a stop whose own condition is false would be \
              the primitive claiming a presence the document denied",
    },
    Case {
        cursor: SPORT,
        present: [false, false, true, false],
        has: true,
        first: SPORT,
        next: SPORT,
        prev: SPORT,
        why: "the only present stop is its own successor and predecessor",
    },
];

#[test]
fn the_expansion_computes_the_declared_navigation() {
    let t = Tmp::new("python");
    t.write("drivemode.scxml", MODE_ENUM);
    let (rc, out, log) = generate(&t, "python");
    assert_eq!(rc, Some(0), "python generation failed:\n{log}");

    // ⚠ A document that keeps its value space in the document imports
    // the enum, and the generated Python says `from . import <enum>` —
    // so the module cannot be loaded on its own. Generating the enum
    // beside it and adding an `__init__.py` makes the directory a
    // package, which is what that import needs. This is the price of the
    // value space being IN the document rather than folded into
    // booleans, and it is the right price.
    let enum_doc = t.0.join("drivemode.scxml");
    let run = Command::new(bin())
        .args([
            "generate",
            enum_doc.to_str().unwrap(),
            "-l",
            "python",
            "-o",
            out.to_str().unwrap(),
        ])
        .output()
        .expect("spawn sce-codegen");
    assert_eq!(
        run.status.code(),
        Some(0),
        "the enum document failed to generate:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    std::fs::write(out.join("__init__.py"), "").expect("write __init__.py");

    // The loader below names `modes.py` directly, so an artifact under a
    // different name would surface as a Python import error rather than
    // as a missing file. Asserting it here makes that the harness's
    // complaint instead of the interpreter's.
    assert!(
        out.join("modes.py").is_file(),
        "the transform did not emit modes.py; the output dir holds: {:?}",
        std::fs::read_dir(&out)
            .expect("read output dir")
            .filter_map(Result::ok)
            .map(|e| e.file_name())
            .collect::<Vec<_>>()
    );

    for case in CASES {
        let [eco, normal, sport, snow] = case.present;
        let script = format!(
            r#"
import importlib.util, sys, pathlib
out = pathlib.Path(r"{}")
pkg = out.name
sys.path.insert(0, str(out.parent))
# The package has to be registered BEFORE the module, or its
# `from . import …` has no parent to resolve against.
pspec = importlib.util.spec_from_file_location(
    pkg, out / "__init__.py", submodule_search_locations=[str(out)])
parent = importlib.util.module_from_spec(pspec)
sys.modules[pkg] = parent
pspec.loader.exec_module(parent)
spec = importlib.util.spec_from_file_location(pkg + ".modes", out / "modes.py")
m = importlib.util.module_from_spec(spec)
sys.modules[pkg + ".modes"] = m
spec.loader.exec_module(m)
kw = dict(cursor={}, eco_on={}, normal_on={}, sport_on={}, snow_on={})
print(int(m.compute_here(**kw)), int(m.compute_first_one(**kw)),
      int(m.compute_next_one(**kw)), int(m.compute_prev_one(**kw)))
"#,
            out.display(),
            case.cursor,
            py_bool(eco),
            py_bool(normal),
            py_bool(sport),
            py_bool(snow),
        );
        let run = Command::new("python3")
            .arg("-c")
            .arg(&script)
            .output()
            .expect("spawn python3");
        let stdout = String::from_utf8_lossy(&run.stdout);
        assert!(
            run.status.success(),
            "python refused the expanded module:\n{}\n{}",
            String::from_utf8_lossy(&run.stderr),
            stdout
        );
        let got: Vec<u8> = stdout
            .split_whitespace()
            .map(|s| s.parse().expect("an integer per output"))
            .collect();
        let want = vec![u8::from(case.has), case.first, case.next, case.prev];
        assert_eq!(
            got, want,
            "cursor={} present={:?}\n  {}\n  (has, first, next, prev)",
            case.cursor, case.present, case.why
        );
    }
}

fn py_bool(b: bool) -> &'static str {
    if b {
        "True"
    } else {
        "False"
    }
}

#[test]
fn every_backend_accepts_the_expanded_form() {
    // ⚠ The expansion produces ONE expression shape for all six targets,
    // so what each backend has to do is accept it. This is the half that
    // silently rots: an expansion that emits something only Python's
    // printer tolerates would still pass the arithmetic test above.
    //
    // ⚠ Go was excluded here, pinned by a test asserting its refusal,
    // while its emitter refused every conditional expression. It now
    // lowers them to a typed function literal (`go_conditional` in
    // forge/expr.rs), so it takes the expansion like the other five.
    for lang in ["python", "cpp", "rust", "kotlin", "c", "go"] {
        let t = Tmp::new(lang);
        t.write("drivemode.scxml", MODE_ENUM);
        let (rc, _, log) = generate(&t, lang);
        assert_eq!(
            rc,
            Some(0),
            "{lang}: the expanded cycle navigation was refused:\n{log}"
        );
    }
}

#[test]
fn a_call_naming_no_cycle_is_refused() {
    // The expansion leaves an unknown cycle id alone, so the call reaches
    // the type checker as an ordinary call to a name nothing provides —
    // a refusal rather than a silent pass-through. Asserting it here is
    // what makes that claim more than a comment.
    let t = Tmp::new("unknown_id");
    t.write("drivemode.scxml", MODE_ENUM);
    let body = DOC.replace("cycle_next(modes, cursor)", "cycle_next(nosuch, cursor)");
    let d = t.write("modes.scxml", &body);
    let out = t.0.join("out");
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
    assert_ne!(
        run.status.code(),
        Some(0),
        "a call naming no cycle generated cleanly:\n{}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
}
