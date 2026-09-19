//! A transform output may read a sibling output; a cycle among them may not.
//!
//! ⚠ WHAT THIS HOLDS IN PLACE. Measured 2026-09-18, before the change these
//! tests cover: a `sce:kind="transform"` document whose output referenced a
//! sibling output generated with **exit 0** and emitted, in every language,
//! a function body naming an identifier the signature never bound. Python
//! raised `NameError` at the first call; C++ and Rust would not compile.
//!
//! The rule it violated was WRITTEN DOWN — `type_ctx::transform` said
//! "outputs are not visible to expressions" — and nothing enforced it. That
//! is the worse of the two failures here: a reader of the source would have
//! believed the document was refused.
//!
//! The resolution was not to enforce the old rule but to drop it. Prose
//! specifications routinely name an intermediate value that several outputs
//! consume (the conversion that surfaced this had one coolant-warning state
//! feeding a telltale and two events), and making the author paste that
//! expression into each consumer loses the spec's own name for the thing and
//! lets the copies drift. So the read is lowered to a call — sound, because
//! every `compute_*` here is a pure function of the same inputs.
//!
//! ⚠⚠ THE REFUSAL IS WHAT MAKES THE PERMISSION SAFE. Without a cycle check,
//! the reward for writing a cycle would be generated code that recurses
//! until the stack ends — strictly worse than the unbound name it replaced.
//! Both halves are asserted here; neither is meaningful alone.

use std::process::Command;

fn sce_codegen_bin() -> String {
    env!("CARGO_BIN_EXE_sce-codegen").to_string()
}

struct Tmp(std::path::PathBuf);

impl Tmp {
    fn new(label: &str) -> Self {
        let d = std::env::temp_dir().join(format!("sce_sibling_{label}_{}", std::process::id()));
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

/// `scaled` reads `doubled`. Acyclic, so it must lower.
const CHAIN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="chain">
  <datamodel>
    <data id="raw" sce:type="int32" sce:direction="in"/>
    <data id="doubled" sce:type="int32" sce:direction="out" expr="raw * 2"/>
    <data id="scaled" sce:type="int32" sce:direction="out" expr="doubled + 1"/>
  </datamodel>
</scxml>
"#;

/// `a` and `b` read each other.
const MUTUAL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="mutual">
  <datamodel>
    <data id="raw" sce:type="int32" sce:direction="in"/>
    <data id="a" sce:type="int32" sce:direction="out" expr="b + raw"/>
    <data id="b" sce:type="int32" sce:direction="out" expr="a * 2"/>
  </datamodel>
</scxml>
"#;

/// `a` reads itself — a cycle of length one.
const SELF_REF: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="selfref">
  <datamodel>
    <data id="raw" sce:type="int32" sce:direction="in"/>
    <data id="a" sce:type="int32" sce:direction="out" expr="a + raw"/>
  </datamodel>
</scxml>
"#;

fn generate(doc: &std::path::Path, out: &std::path::Path, lang: &str) -> (Option<i32>, String) {
    let run = Command::new(sce_codegen_bin())
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
    (
        run.status.code(),
        String::from_utf8_lossy(&run.stderr).into_owned(),
    )
}

#[test]
fn a_sibling_read_becomes_a_call_in_every_language() {
    // ⚠ Every backend, not one. The bug emitted the unbound name in all of
    // them, so checking a single language would leave five ways to
    // reintroduce it. The generated text is searched for a CALL of the
    // sibling — the bare name on its own is exactly the defect.
    for (lang, ext, callee) in [
        ("python", "py", "compute_doubled("),
        ("cpp", "h", "computeDoubled("),
        ("rust", "rs", "compute_doubled("),
        ("go", "go", "ComputeDoubled("),
        ("kotlin", "kt", "computeDoubled("),
        ("c", "h", "chain_compute_doubled("),
    ] {
        let t = Tmp::new(&format!("chain_{lang}"));
        let doc = t.write("chain.scxml", CHAIN);
        let (code, stderr) = generate(&doc, &t.0, lang);
        assert_eq!(code, Some(0), "{lang}: generation failed:\n{stderr}");

        let body = std::fs::read_dir(&t.0)
            .expect("read output dir")
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some(ext))
            .map(|e| std::fs::read_to_string(e.path()).unwrap_or_default())
            .collect::<String>();
        assert!(
            !body.is_empty(),
            "{lang}: no .{ext} artifact was written to inspect"
        );
        assert!(
            body.contains(callee),
            "{lang}: the sibling read did not lower to a call ({callee}):\n{body}"
        );
    }
}

#[test]
fn a_mutual_cycle_is_refused() {
    let t = Tmp::new("mutual");
    let doc = t.write("mutual.scxml", MUTUAL);
    let (code, stderr) = generate(&doc, &t.0, "python");
    assert_ne!(code, Some(0), "a mutual output cycle generated:\n{stderr}");
    // The refusal names the path, because the author's next action is to
    // delete one edge of it and they have to know which edges there are.
    assert!(
        stderr.contains("cycle"),
        "the refusal does not say it is a cycle:\n{stderr}"
    );
    for needle in ["a", "b", "mutual"] {
        assert!(
            stderr.contains(needle),
            "the refusal does not name {needle}:\n{stderr}"
        );
    }
}

#[test]
fn a_self_reference_is_refused() {
    // Caught separately because the per-output rename map EXCLUDES the
    // output's own id — that exclusion is what lets a legal document avoid
    // calling itself, so the self case cannot be left to the general walk.
    let t = Tmp::new("selfref");
    let doc = t.write("selfref.scxml", SELF_REF);
    let (code, stderr) = generate(&doc, &t.0, "python");
    assert_ne!(
        code,
        Some(0),
        "an output that reads itself generated:\n{stderr}"
    );
    assert!(
        stderr.contains("cycle"),
        "the refusal does not say it is a cycle:\n{stderr}"
    );
}

#[test]
fn the_refusal_does_not_depend_on_the_language_asked_for() {
    // The check runs before rendering. If it had been put inside one
    // backend, the same document would be legal in the others — and a
    // reviewer reading a green C++ lane would have no reason to doubt it.
    let t = Tmp::new("langs");
    let doc = t.write("mutual.scxml", MUTUAL);
    for lang in ["python", "cpp", "rust", "go", "kotlin", "c"] {
        let (code, _) = generate(&doc, &t.0, lang);
        assert_ne!(
            code,
            Some(0),
            "{lang} accepted a document with an output cycle"
        );
    }
}
