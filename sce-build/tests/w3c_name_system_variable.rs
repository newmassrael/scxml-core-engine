// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `_name` carries the value W3C SCXML 5.10 says it carries.
//
// "The SCXML Processor MUST bind the variable `_name` at load time to
// the value of the 'name' attribute of the `<scxml>` element" — the
// wording of the specification test 323 cites. Every AOT backend bound
// it to the document's *file stem* instead, so `test323.scxml`
// declaring `name="machineName"` produced a machine whose `_name` read
// `"test323"`. The C++ interpreter's QuickJS engine did not even do
// that: it discarded the value it was handed for the literal
// `"RSMStateMachine"`, under a comment claiming it came from the
// element's name attribute.
//
// What makes this the shape of defect worth a gate rather than a patch
// is why it survived. Four W3C tests reach `_name` (323, 324, 329,
// 346), all four are registered, and all four passed throughout —
// because 323 asserts `conf:isBound="1"` and the other three assert
// that writing to `_name` fails. Not one of them reads the value. A
// conformance suite that is green on a requirement it does not test is
// worse than one that skips it, and no amount of running it harder
// closes that: the assertion has to exist somewhere, and the W3C corpus
// is not ours to edit.
//
// So this is the value assertion, made where it can be made — against
// what the generator emits, for every backend at once. Three properties:
//
//   * a declared name reaches every backend's binding site,
//   * an undeclared one falls back to the document identity rather than
//     to empty, which is W3C 3.2.1's "MUST generate a name",
//   * and an author-hostile name survives the literal boundaries it
//     crosses on the way in.
//
// That last one is not hypothetical and is the reason the fix is more
// than swapping a field. The stem this replaced was a filename and
// could not contain a quote; the attribute is author text and can. The
// C11 backend embeds the value in Lua source *inside* a C string
// literal — two boundaries — and a host-only escape gave
// `_name = 'it's'`: a translation unit that compiles and Lua that does
// not parse. `mlua` (the same interpreter the runtimes use) is what
// decides that here, rather than a hand-rolled check that could drift
// from the grammar.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

static SCRATCH_ID: AtomicU64 = AtomicU64::new(0);

struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new(label: &str) -> Self {
        let id = SCRATCH_ID.fetch_add(1, Ordering::SeqCst);
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("{label}-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        Self(dir)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Every backend whose templates bind `_name`.
const BACKENDS: [&str; 6] = ["rust", "cpp", "go", "kotlin", "python", "c11"];

/// W3C test 323 is the fixture because it is the one the specification
/// sentence belongs to, and because it needs a datamodel — the binding
/// site sits behind `needs_script_engine`, so a document that needs no
/// script engine never renders it. A probe built from a simpler
/// statechart reported every backend clean while changing nothing.
const FIXTURE: &str = "resources/323/test323.scxml";
const DECLARED: &str = "machineName";

/// Enough files must carry the binding for a clean sweep to mean
/// something. Measured: all six backends emit it for this fixture.
const MIN_BACKENDS_BINDING: usize = 6;

/// Stage `FIXTURE` under `stem`, with its root `name` attribute set to
/// `declared`, or removed entirely when `declared` is `None`.
fn stage(stem: &str, declared: Option<&str>, into: &Path) -> PathBuf {
    let text = std::fs::read_to_string(repo_root().join(FIXTURE))
        .unwrap_or_else(|e| panic!("read {FIXTURE}: {e}"));
    let needle = format!(r#"name="{DECLARED}""#);
    assert!(
        text.contains(&needle),
        "{FIXTURE} no longer declares {needle}; this fixture is chosen for \
         that attribute and the staging below is now a no-op"
    );
    let staged = match declared {
        Some(v) => text.replace(&needle, &format!(r#"name="{v}""#)),
        None => text.replace(&needle, ""),
    };
    let path = into.join(format!("{stem}.scxml"));
    std::fs::write(&path, staged).expect("write staged fixture");
    path
}

/// Generate `doc` for `language` and return the concatenated output.
fn generate(doc: &Path, language: &str) -> String {
    let out = ScratchDir::new("w3c-name-out");
    let result = Command::new(sce_codegen_bin())
        .arg("generate")
        .arg(doc)
        .args(["-l", language, "-o"])
        .arg(out.path())
        .current_dir(repo_root())
        .output()
        .expect("sce-codegen runs");
    assert!(
        result.status.success(),
        "generate -l {language} failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let mut all = String::new();
    for entry in std::fs::read_dir(out.path()).expect("read output dir") {
        let p = entry.expect("dir entry").path();
        if p.is_file() {
            all.push_str(&std::fs::read_to_string(&p).unwrap_or_default());
        }
        all.push('\n');
    }
    assert!(
        all.trim().len() > 100,
        "generate -l {language} produced almost nothing"
    );
    all
}

/// The binding sites in `text`, each as the call and the text that
/// follows it.
///
/// Comment lines are dropped first — the templates explain the rule in
/// prose that names the same identifiers — and what remains is
/// whitespace-collapsed before the search. Line-wise matching looked
/// sufficient and was not: Kotlin renders the call across four lines
/// with the value on its own, so the line naming `setupSystemVariables`
/// carried no value and the backend silently measured nothing.
fn binding_sites(text: &str) -> Vec<String> {
    let code: String = text
        .lines()
        .map(str::trim)
        .filter(|l| {
            !l.starts_with("//")
                && !l.starts_with('*')
                && !l.starts_with('#')
                && !l.starts_with("/*")
        })
        .collect::<Vec<_>>()
        .join(" ");
    let collapsed = code.split_whitespace().collect::<Vec<_>>().join(" ");

    const CALLS: [&str; 6] = [
        "setup_system_variables",
        "setupSystemVariables",
        "SetupSystemVariables",
        "\"_name\"",
        "_name = ",
        "def machine_name",
    ];
    let mut sites = Vec::new();
    for call in CALLS {
        let mut from = 0usize;
        while let Some(rel) = collapsed[from..].find(call) {
            let start = from + rel;
            let end = (start + call.len() + 160).min(collapsed.len());
            // Slice on a char boundary — generated prose is not ASCII-only.
            let end = (start..=end)
                .rev()
                .find(|&i| collapsed.is_char_boundary(i))
                .unwrap_or(start);
            sites.push(collapsed[start..end].to_string());
            from = start + call.len();
        }
    }
    sites
}

#[test]
fn every_backend_binds_the_declared_name_and_not_the_file_stem() {
    let scratch = ScratchDir::new("w3c-name-in");
    // A stem that cannot be confused with the declared value, so "the
    // right string appears" cannot be satisfied by accident.
    let doc = stage("stem_is_not_the_name", Some(DECLARED), scratch.path());

    let mut bound = 0usize;
    for backend in BACKENDS {
        let text = generate(&doc, backend);
        let sites: Vec<String> = binding_sites(&text)
            .into_iter()
            .filter(|l| l.contains(DECLARED) || l.contains("stem_is_not_the_name"))
            .collect();
        assert!(
            !sites.is_empty(),
            "{backend}: no binding site carries either the declared name or \
             the stem, so this backend is not being measured at all"
        );
        for site in &sites {
            assert!(
                !site.contains("stem_is_not_the_name"),
                "{backend}: binds `_name` to the file stem.\n  {site}\n\
                 W3C SCXML 5.10 requires the value of the root `name` \
                 attribute, which this document declares as {DECLARED:?}."
            );
        }
        bound += 1;
    }
    assert!(
        bound >= MIN_BACKENDS_BINDING,
        "only {bound} backend(s) reached the assertion (floor \
         {MIN_BACKENDS_BINDING})"
    );
}

#[test]
fn an_undeclared_name_falls_back_to_the_document_identity() {
    let scratch = ScratchDir::new("w3c-name-none");
    let doc = stage("identity_from_the_document", None, scratch.path());

    for backend in BACKENDS {
        let text = generate(&doc, backend);
        let sites = binding_sites(&text);
        assert!(
            !sites.is_empty(),
            "{backend}: emitted no binding site for an unnamed document"
        );
        let carries_identity = sites
            .iter()
            .any(|l| l.contains("identity_from_the_document"));
        assert!(
            carries_identity,
            "{backend}: an undeclared name did not fall back to the document \
             identity. W3C SCXML 3.2.1 requires the processor to generate a \
             name when the attribute is absent; binding the empty string is \
             not generating one.\n  sites: {sites:?}"
        );
    }
}

#[test]
fn an_author_hostile_name_survives_every_literal_boundary_it_crosses() {
    // Quote, backslash and an apostrophe: the first two break a host
    // string literal, the third broke the Lua string the C11 backend
    // nests inside one.
    const HOSTILE: &str = r#"evil" + escape() + "\x it's"#;
    let scratch = ScratchDir::new("w3c-name-hostile");
    let doc = {
        let text = std::fs::read_to_string(repo_root().join(FIXTURE)).expect("read fixture");
        // Written through XML entities so the document itself stays
        // well-formed; the parser hands the decoded text to codegen.
        let encoded = HOSTILE.replace('&', "&amp;").replace('"', "&quot;");
        let staged = text.replace(
            &format!(r#"name="{DECLARED}""#),
            &format!(r#"name="{encoded}""#),
        );
        let path = scratch.path().join("hostile.scxml");
        std::fs::write(&path, staged).expect("write hostile fixture");
        path
    };

    for backend in BACKENDS {
        let text = generate(&doc, backend);
        // The raw value must never appear unescaped: that is exactly the
        // shape that closes the host literal early.
        assert!(
            !text.contains(HOSTILE),
            "{backend}: the author value reached the output unescaped, which \
             closes the string literal it sits in"
        );
        assert!(
            text.contains("escape()"),
            "{backend}: the author value does not appear at all, so this \
             backend is not being measured"
        );
    }

    // C11 is the one backend where the value crosses two boundaries. Its
    // emitted Lua is decoded out of the C string literal and handed to
    // the real interpreter, because "compiles" and "parses as Lua" are
    // different questions and only the second one failed.
    let c = generate(&doc, "c11");
    let line = c
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("\"_name = "))
        .expect("c11 emits a `_name = ...` Lua assignment");
    let lua = decode_c_string_literal(line);
    let interpreter = mlua::Lua::new();
    interpreter
        .load(&lua)
        .into_function()
        .unwrap_or_else(|e| panic!("c11 emitted Lua that does not parse: {lua:?}\n  {e}"));
    // And the value it assigns is the author's, not a truncation of it.
    let value: String = interpreter
        .load(format!("{lua}\nreturn _name"))
        .eval()
        .expect("the emitted assignment evaluates");
    assert_eq!(
        value, HOSTILE,
        "c11: the Lua assignment parses but binds a different string than the \
         document declared"
    );
}

/// Decode one C string literal (the surrounding quotes included) into
/// the bytes the compiler would produce.
fn decode_c_string_literal(line: &str) -> String {
    let body = line
        .trim()
        .trim_end_matches(';')
        .trim_end_matches(')')
        .trim();
    let body = body
        .strip_prefix('"')
        .and_then(|b| b.strip_suffix('"'))
        .unwrap_or_else(|| panic!("not a lone C string literal: {line:?}"));
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => panic!("trailing backslash in {line:?}"),
        }
    }
    out
}
