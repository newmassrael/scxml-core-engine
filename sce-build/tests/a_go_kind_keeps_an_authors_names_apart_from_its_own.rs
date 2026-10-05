// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A name an author gives a parameter, a variable or a datum of a forge kind
//! must not decide what the generated Go does.
//!
//! The codec kind has its own oracle
//! (`a_go_codec_keeps_an_authors_names_apart_from_its_own`), and it found
//! nothing: a codec field is written Pascal and the decoder's locals stay lower
//! case. This is the same question for the kinds whose generated Go is a
//! FUNCTION the author's names are locals and parameters of — algorithm,
//! condition, filter, interpolation, lookup, observer, transform and validator.
//! There the author's name is spelled as it is written, in the same scope as
//! the predeclared names the body calls: an algorithm variable called `uint16`
//! beside `uint16(b)`, a parameter called `len` beside `len(data)`.
//!
//! # What is measured
//!
//! Every document of those kinds under `tests/forge/resources` is generated as
//! Go with each name it declares (`<data id>`, `<sce:param name>`,
//! `<sce:var name>`) renamed to each candidate, and the result is built. A name
//! the parser or the generator refuses is an answer — the author is told before
//! any code exists. What may not happen is a name that is accepted and then
//! makes the generated Go not compile.
//!
//! # What is not measured
//!
//! The generated Go is built, not run. Go is statically typed, so a name that
//! hides a type or a function the body calls is a compile error, which is what
//! this catches. A name that is accepted, compiles, and does something else —
//! the way a Python validator input called `delta` stored the difference as its
//! previous reading — would not be seen here, and the Python oracle that found
//! it runs what it generates because Python cannot say so at build time.
//!
//! The candidates are derived, not listed: Go's universe scope, which the
//! language fixes, joined with every identifier the committed Go output of that
//! kind uses, and a few shapes a name can take (camelCase, capitalised, with a
//! digit).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;
use sce_build::generator::Language;
use sce_build::{compile_forge_with_imports, DocumentLabel, ForgeCompileOptions};

const GO_MODULE_PREFIX: &str = "example.com/sce-forge";

/// The kinds whose generated Go is a function an author's names are locals of.
const KINDS: &[&str] = &[
    "algorithm",
    "condition",
    "filter",
    "interpolation",
    "lookup",
    "observer",
    "transform",
    "validator",
];

/// The shapes a name can take, so a spelling that is wrong for a shape is found
/// for every author who writes in it.
const SHAPES: &[&str] = &["engineRpm", "Foo", "MY_CONST", "x1", "rawValue"];

/// Go's predeclared identifiers and the packages the generated Go imports: the
/// scope an author's name meets, which the language and the templates fix.
const UNIVERSE: &[&str] = &[
    "any",
    "bool",
    "byte",
    "comparable",
    "complex64",
    "complex128",
    "error",
    "float32",
    "float64",
    "int",
    "int8",
    "int16",
    "int32",
    "int64",
    "rune",
    "string",
    "uint",
    "uint8",
    "uint16",
    "uint32",
    "uint64",
    "uintptr",
    "true",
    "false",
    "iota",
    "nil",
    "append",
    "cap",
    "clear",
    "close",
    "complex",
    "copy",
    "delete",
    "imag",
    "len",
    "make",
    "max",
    "min",
    "new",
    "panic",
    "print",
    "println",
    "real",
    "recover",
    "math",
    "fmt",
    "errors",
    "testing",
    "codec",
];

fn resource_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources")
}

fn expected_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/expected")
}

fn go_runtime() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../backends/go/forge-runtime")
}

fn options() -> ForgeCompileOptions {
    ForgeCompileOptions {
        go_module_prefix: Some(GO_MODULE_PREFIX.to_string()),
        ..ForgeCompileOptions::default()
    }
}

/// The identifiers of Go source, with comments and literals left out.
fn identifiers_of(source: &str) -> BTreeSet<String> {
    let chars: Vec<char> = source.chars().collect();
    let mut found = BTreeSet::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && chars.get(i + 1) == Some(&'*') {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
        } else if c == '"' || c == '\'' {
            i += 1;
            while i < chars.len() && chars[i] != c {
                if chars[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
        } else if c == '`' {
            i += 1;
            while i < chars.len() && chars[i] != '`' {
                i += 1;
            }
            i += 1;
        } else if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            found.insert(chars[start..i].iter().collect());
        } else {
            i += 1;
        }
    }
    found
}

/// Every identifier the committed Go output of a kind uses, by kind: the kind a
/// generated file says it came from in its header.
fn go_names_by_kind() -> BTreeMap<String, BTreeSet<String>> {
    let kind_re = Regex::new(r#"sce:kind="([a-z_-]+)""#).expect("regex");
    let mut by_kind: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut entries: Vec<PathBuf> = std::fs::read_dir(expected_dir())
        .expect("read the committed outputs")
        .map(|e| e.expect("directory entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "go"))
        .collect();
    entries.sort();
    for path in entries {
        let source = std::fs::read_to_string(&path).expect("read a committed Go output");
        let head: String = source.chars().take(600).collect();
        let Some(kind) = kind_re.captures(&head).map(|c| c[1].to_string()) else {
            continue;
        };
        if KINDS.contains(&kind.as_str()) {
            by_kind
                .entry(kind)
                .or_default()
                .extend(identifiers_of(&source));
        }
    }
    by_kind
}

/// How a name is spelled once folded to snake_case, for the check that two of
/// an author's own names do not meet in one spelling.
fn snake(name: &str) -> String {
    let mut out = String::new();
    let mut previous_lower_or_digit = false;
    for c in name.chars() {
        if c.is_ascii_uppercase() && previous_lower_or_digit {
            out.push('_');
        }
        out.push(c.to_ascii_lowercase());
        previous_lower_or_digit = c.is_ascii_lowercase() || c.is_ascii_digit();
    }
    out
}

/// A document is rewritten by replacing every whole-word occurrence of `from`
/// that is not a member access (`x.from`).
fn rename(text: &str, from: &str, to: &str) -> String {
    let pattern = Regex::new(&format!(r"(^|[^\w.])({})\b", regex::escape(from)))
        .expect("an identifier is a regex");
    pattern
        .replace_all(text, |caps: &regex::Captures| format!("{}{}", &caps[1], to))
        .into_owned()
}

struct Doc {
    stem: String,
    kind: String,
    text: String,
    declared: Vec<String>,
}

fn documents() -> Vec<Doc> {
    let kind_re = Regex::new(r#"sce:kind="([a-z_-]+)""#).expect("regex");
    let declared_re = Regex::new(
        r#"<(?:data|sce:param|sce:var)\b[^>]*?\b(?:id|name)="([A-Za-z_][A-Za-z0-9_]*)""#,
    )
    .expect("regex");
    let mut docs = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(resource_dir())
        .expect("read resources")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "scxml") && p.is_file())
        .collect();
    entries.sort();
    for path in entries {
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let Some(kind) = kind_re.captures(&text).map(|c| c[1].to_string()) else {
            continue;
        };
        if !KINDS.contains(&kind.as_str()) {
            continue;
        }
        let declared: BTreeSet<String> = declared_re
            .captures_iter(&text)
            .map(|c| c[1].to_string())
            .collect();
        docs.push(Doc {
            stem: path.file_stem().unwrap().to_string_lossy().to_string(),
            kind,
            text,
            declared: declared.into_iter().collect(),
        });
    }
    docs
}

/// A generated Go file: the package directory it belongs in, and its source.
struct GoFile {
    package: String,
    source: String,
}

fn package_of(file_name: &str) -> String {
    Path::new(file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(file_name)
        .to_string()
}

/// Generate `text` as Go under the unique name `unique`: the files, or the
/// reason it was refused. The document is renamed by position so each case is
/// its own package.
fn generate(unique: &str, text: &str) -> Result<Vec<GoFile>, String> {
    let name_re = Regex::new(r#"(<scxml\b[^>]*?\bname=")([^"]*)(")"#).expect("regex");
    let text = name_re
        .replace(text, |caps: &regex::Captures| {
            format!("{}{}{}", &caps[1], unique, &caps[3])
        })
        .into_owned();
    compile_forge_with_imports(
        &text,
        DocumentLabel::symmetric(unique),
        Language::Go,
        &resource_dir(),
        &options(),
    )
    .map_err(|e| e.error.to_string())
    .map(|output| {
        output
            .files
            .into_iter()
            .filter(|(file, _)| file.ends_with(".go"))
            .map(|(file, source)| GoFile {
                package: package_of(&file),
                source,
            })
            .collect()
    })
}

/// Build errors by package: `go build` prints `# <package>` and then what is
/// wrong with it, and keeps compiling the packages that are not.
fn failed_packages(build_stderr: &str) -> BTreeMap<String, String> {
    let mut failed: BTreeMap<String, String> = BTreeMap::new();
    let mut current: Option<String> = None;
    for line in build_stderr.lines() {
        if let Some(pkg) = line.strip_prefix("# ") {
            let pkg = pkg.split_whitespace().next().unwrap_or(pkg);
            let pkg = pkg
                .strip_prefix(&format!("{GO_MODULE_PREFIX}/"))
                .unwrap_or(pkg);
            current = Some(pkg.to_string());
            failed.entry(pkg.to_string()).or_default();
        } else if let Some(pkg) = &current {
            let entry = failed.get_mut(pkg).expect("the package was inserted");
            if entry.is_empty() && !line.trim().is_empty() {
                *entry = line.trim().to_string();
            }
        }
    }
    failed
}

/// One accepted renaming that was built: where it came from and its package.
struct Case {
    kind: String,
    stem: String,
    declared: String,
    candidate: String,
    package: String,
}

#[test]
fn an_authors_name_never_decides_whether_the_generated_go_of_a_kind_builds() {
    let Some(go) = sce_build::toolchain::require_or_skip("go", "the Go kind name oracle") else {
        return;
    };
    let docs = documents();
    assert!(
        docs.len() >= 40,
        "implausibly few documents of the eight kinds ({}); the scan broke",
        docs.len()
    );
    let from_outputs = go_names_by_kind();
    assert!(
        from_outputs.len() >= 6,
        "the committed Go outputs name only {} of the kinds; the derivation broke",
        from_outputs.len()
    );

    let proj = std::env::temp_dir().join(format!("sce_go_kind_names_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&proj);
    std::fs::create_dir_all(&proj).expect("mkdir");
    std::fs::write(
        proj.join("go.mod"),
        format!(
            "module {GO_MODULE_PREFIX}\n\ngo 1.22\n\n\
             require github.com/newmassrael/sce-forge-runtime v0.0.0\n\n\
             replace github.com/newmassrael/sce-forge-runtime => {}\n",
            go_runtime()
                .canonicalize()
                .expect("the Go runtime")
                .display()
        ),
    )
    .expect("write go.mod");

    let mut written: BTreeSet<String> = BTreeSet::new();
    let mut write_package = |file: &GoFile| {
        if written.insert(file.package.clone()) {
            let dir = proj.join(&file.package);
            std::fs::create_dir_all(&dir).expect("mkdir package");
            std::fs::write(dir.join(format!("{}.go", file.package)), &file.source)
                .expect("write package");
        }
    };

    // The unrenamed documents are the control: each must build, or a failure of
    // its renamings is not about the name.
    let mut baseline: Vec<(String, String)> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    for doc in &docs {
        let unique = format!("{}__base", doc.stem);
        match generate(&unique, &doc.text) {
            // A document that needs a sibling module (it imports another
            // generated package) cannot be built on its own and is not a case,
            // as the Python oracle does not run one that imports a sibling.
            Ok(files)
                if files
                    .iter()
                    .any(|f| f.source.contains(&format!("\"{GO_MODULE_PREFIX}/"))) =>
            {
                skipped.push(doc.stem.clone());
            }
            Ok(files) => {
                for file in &files {
                    write_package(file);
                }
                baseline.push((doc.stem.clone(), unique));
            }
            Err(_) => skipped.push(doc.stem.clone()),
        }
    }
    // Build the controls alone, first: a document whose own text does not build
    // as Go (a quantity that truncates, a condition it reads from a sibling) is
    // not a case, and its renamings would say nothing about the name.
    let control = Command::new(&go)
        .args(["build", "./..."])
        .current_dir(&proj)
        .output()
        .expect("go builds the controls");
    let control_broken = failed_packages(&String::from_utf8_lossy(&control.stderr));
    let mut unbuildable: Vec<String> = Vec::new();
    baseline.retain(|(stem, package)| {
        let keep = !control_broken.contains_key(package);
        if !keep {
            unbuildable.push(stem.clone());
        }
        keep
    });
    skipped.extend(unbuildable.iter().cloned());
    eprintln!("go kind name oracle: controls that do not build as Go: {unbuildable:?}");

    let mut cases: Vec<Case> = Vec::new();
    let mut attempts = 0usize;
    let mut refused = 0usize;
    let mut folded_together = 0usize;
    let mut other_refusals: BTreeMap<String, usize> = BTreeMap::new();
    for doc in &docs {
        if skipped.contains(&doc.stem) {
            continue;
        }
        let candidates: BTreeSet<String> = from_outputs
            .get(&doc.kind)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .chain(UNIVERSE.iter().map(|s| s.to_string()))
            .chain(SHAPES.iter().map(|s| s.to_string()))
            .filter(|n| n != "_")
            .collect();
        let folded: BTreeSet<String> = doc.declared.iter().map(|n| snake(n)).collect();
        for declared in &doc.declared {
            for candidate in &candidates {
                if candidate == declared || doc.declared.contains(candidate) {
                    continue;
                }
                // Two author names that fold to one spelling are a different
                // defect — two of the AUTHOR's names meeting — and are counted,
                // not asked of this oracle.
                if folded.contains(&snake(candidate)) {
                    folded_together += 1;
                    continue;
                }
                attempts += 1;
                let renamed = rename(&doc.text, declared, candidate);
                // `c<n>`, not a bare number: Go reads a file name ending in
                // `_<GOARCH>` as a build constraint, so the case numbered 386
                // was `<stem>__386.go`, which `go build ./...` leaves out
                // without a word and the oracle counted as built.
                let unique = format!("{}__c{}", doc.stem, cases.len());
                match generate(&unique, &renamed) {
                    Err(why) => {
                        refused += 1;
                        // Every refusal is a name-related answer in this
                        // corpus; one that is not names its cause here so a
                        // bad rewrite cannot hide as "refused".
                        if !(why.contains("cannot declare it")
                            || why.contains("shadows")
                            || why.contains("duplicate")
                            || why.contains("read-only")
                            || why.contains("not declared")
                            || why.contains("unknown")
                            || why.contains("so it would declare one name twice"))
                        {
                            let key: String = why.chars().take(70).collect();
                            *other_refusals.entry(key).or_default() += 1;
                        }
                    }
                    Ok(files) => {
                        for file in &files {
                            write_package(file);
                        }
                        cases.push(Case {
                            kind: doc.kind.clone(),
                            stem: doc.stem.clone(),
                            declared: declared.clone(),
                            candidate: candidate.clone(),
                            package: unique,
                        });
                    }
                }
            }
        }
    }

    let build = Command::new(&go)
        .args(["build", "./..."])
        .current_dir(&proj)
        .output()
        .expect("go builds the generated packages");
    let build_stderr = String::from_utf8_lossy(&build.stderr).to_string();
    let broken = failed_packages(&build_stderr);
    // `go build ./...` skips a directory none of whose files the build
    // includes and says nothing, so a package that was never compiled is
    // indistinguishable from one that built. `go list ./...` names the
    // packages the build did include: every one written must be among them.
    let listed = Command::new(&go)
        .args(["list", "./..."])
        .current_dir(&proj)
        .output()
        .expect("go lists the generated packages");
    let included: BTreeSet<String> = String::from_utf8_lossy(&listed.stdout)
        .lines()
        .map(|p| {
            p.strip_prefix(&format!("{GO_MODULE_PREFIX}/"))
                .unwrap_or(p)
                .to_string()
        })
        .collect();
    let left_out: Vec<&String> = written.difference(&included).collect();
    let _ = std::fs::remove_dir_all(&proj);
    assert!(
        left_out.is_empty(),
        "packages the build did not include, so they were never compiled: {left_out:?}"
    );
    assert!(
        build.status.success() || !broken.is_empty(),
        "go build failed without naming a package:\n{build_stderr}"
    );

    let broken_baselines: Vec<&str> = baseline
        .iter()
        .filter(|(_, package)| broken.contains_key(package))
        .map(|(stem, _)| stem.as_str())
        .collect();
    assert!(
        broken_baselines.is_empty(),
        "the unrenamed document does not build as Go, so its renamings say nothing about \
         the name: {broken_baselines:?}\n{}",
        broken_baselines
            .iter()
            .filter_map(|stem| baseline
                .iter()
                .find(|(s, _)| s == stem)
                .and_then(|(_, package)| broken.get(package)))
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );

    // What each failure says once the position and the candidate are taken out
    // of it, so that one cause shared by hundreds of renamings reads as one
    // line: how many renamings, which kinds, which candidate names, and one
    // example in full.
    let position = Regex::new(r"^\S+\.go:\d+:\d+:\s*").expect("regex");
    struct Template {
        renamings: usize,
        candidates: BTreeSet<String>,
        kinds: BTreeSet<String>,
        example: String,
    }
    let mut by_template: BTreeMap<String, Template> = BTreeMap::new();
    let mut failing = 0usize;
    for case in &cases {
        if let Some(why) = broken.get(&case.package) {
            failing += 1;
            let bare = position.replace(why, "").to_string();
            let template = bare
                .replace(&format!("{}_", case.candidate), "<NAME>_")
                .replace(&case.candidate, "<NAME>");
            let entry = by_template.entry(template).or_insert_with(|| Template {
                renamings: 0,
                candidates: BTreeSet::new(),
                kinds: BTreeSet::new(),
                example: format!(
                    "{} `{}` renamed to {}: {why}",
                    case.stem, case.declared, case.candidate
                ),
            });
            entry.renamings += 1;
            entry.candidates.insert(case.candidate.clone());
            entry.kinds.insert(case.kind.clone());
        }
    }

    eprintln!(
        "go kind name oracle: {} documents ({} skipped), {attempts} renamings tried, {refused} \
         refused, {folded_together} left out because two of the author's own names fold to one, \
         {} built, {failing} did not build",
        docs.len(),
        skipped.len(),
        cases.len()
    );
    assert!(
        cases.len() * 10 >= attempts * 7,
        "only {} of {attempts} renamings generated ({refused} refused); the oracle is mostly \
         asking nothing. Refusals not about a name: {other_refusals:?}",
        cases.len()
    );
    assert!(
        cases.len() >= 2000,
        "only {} renamings were built; skipped documents: {skipped:?}",
        cases.len()
    );

    let mut templates: Vec<(&String, &Template)> = by_template.iter().collect();
    templates.sort_by_key(|(_, t)| std::cmp::Reverse(t.renamings));
    let grouped: Vec<String> = templates
        .iter()
        .map(|(template, t)| {
            let shown: Vec<&str> = t.candidates.iter().take(24).map(String::as_str).collect();
            format!(
                "  {:>5} renamings, {} names, kinds [{}]: {template}\n         names: {}{}\n         e.g. {}",
                t.renamings,
                t.candidates.len(),
                t.kinds.iter().cloned().collect::<Vec<_>>().join(", "),
                shown.join(", "),
                if t.candidates.len() > shown.len() { ", ..." } else { "" },
                t.example
            )
        })
        .collect();
    assert!(
        failing == 0,
        "{failing} of {} accepted renamings make the generated Go not build ({attempts} tried, \
         {refused} refused, {folded_together} left out because two of the author's own names \
         fold to one):\n{}",
        cases.len(),
        grouped.join("\n")
    );
}

/// The names Go predeclares, read from the toolchain: `go doc -all builtin`
/// prints every function, type, variable and constant of the universe scope.
fn go_universe() -> BTreeSet<String> {
    let out = Command::new("go")
        .args(["doc", "-all", "builtin"])
        .output()
        .expect("go doc prints the universe scope");
    assert!(
        out.status.success(),
        "`go doc -all builtin` failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let declared =
        Regex::new(r"(?m)^(?:func|type|var|const) ([A-Za-z_][A-Za-z0-9_]*)").expect("regex");
    // `true` and `false` are one constant block, which the documentation
    // prints as `\ttrue  = 0 == 0`.
    let grouped = Regex::new(r"(?m)^\t([a-z][A-Za-z0-9_]*)\s+= ").expect("regex");
    // The documentation names the arguments of a generic builtin with types of
    // its own (`func len(v Type) int`); they are not declared names.
    const PLACEHOLDERS: &[&str] = &["Type", "Type1", "IntegerType", "FloatType", "ComplexType"];
    declared
        .captures_iter(&text)
        .chain(grouped.captures_iter(&text))
        .map(|c| c[1].to_string())
        .filter(|n| !PLACEHOLDERS.contains(&n.as_str()))
        .collect()
}

/// Every name Go predeclares must be one the generator keeps an author's local
/// off. The oracle above is the arbiter for what a name does; this one is for
/// the list, so that a Go release that adds a builtin (`min`, `max` and `clear`
/// came with 1.21) fails here with the name in the message and not as a build
/// error in a document that happens to call it.
#[test]
fn every_name_go_predeclares_is_one_the_generator_escapes() {
    let Some(_) = sce_build::toolchain::require_or_skip("go", "the Go universe pin") else {
        return;
    };
    let universe = go_universe();
    assert!(
        universe.len() >= 40,
        "implausibly few predeclared names ({}): {universe:?}; the derivation broke",
        universe.len()
    );
    let unescaped: Vec<&String> = universe
        .iter()
        .filter(|name| sce_build::forge::generator::go_local_spelling(name) == **name)
        .collect();
    assert!(
        unescaped.is_empty(),
        "Go predeclares names the generator does not escape: {unescaped:?}"
    );
}

/// The package names a committed Go output of one of the kinds imports from the
/// standard library or from the forge runtime, which is what a body names
/// (`lookup.Lookup`, `math.Abs`) and an author's local of that name would hide.
/// Read from the import block of each file, so a template that starts to import
/// one more package is found here by its name.
fn imported_packages() -> BTreeSet<String> {
    let kind_re = Regex::new(r#"sce:kind="([a-z_-]+)""#).expect("regex");
    let import_re =
        Regex::new(r#"^\s*(?:import\s+)?(?:([A-Za-z_][A-Za-z0-9_]*)\s+)?"([^"]+)"\s*$"#)
            .expect("regex");
    let mut found = BTreeSet::new();
    for entry in std::fs::read_dir(expected_dir()).expect("read the committed outputs") {
        let path = entry.expect("directory entry").path();
        if path.extension().is_none_or(|e| e != "go") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("read a committed Go output");
        let head: String = source.chars().take(600).collect();
        if !kind_re
            .captures(&head)
            .is_some_and(|c| KINDS.contains(&&c[1]))
        {
            continue;
        }
        // The import block ends where the first declaration begins.
        for line in source.lines().take_while(|l| {
            !["type ", "func ", "var ", "const "]
                .iter()
                .any(|k| l.starts_with(k))
        }) {
            let Some(c) = import_re.captures(line) else {
                continue;
            };
            let path = &c[2];
            let first = path.split('/').next().unwrap_or(path);
            // A path with a dot in its first element is a module: the forge
            // runtime is one of ours to escape, and a sibling generated package
            // is named by the author's document and handled by the alias.
            let ours = path.starts_with("github.com/newmassrael/sce-forge-runtime/");
            if first.contains('.') && !ours {
                continue;
            }
            let name = c
                .get(1)
                .map(|m| m.as_str())
                .unwrap_or_else(|| path.rsplit('/').next().unwrap_or(path));
            found.insert(name.to_string());
        }
    }
    found
}

#[test]
fn every_package_the_generated_go_of_a_kind_imports_is_one_the_generator_escapes() {
    let imported = imported_packages();
    assert!(
        imported.len() >= 3,
        "implausibly few imported packages ({imported:?}); the derivation broke"
    );
    let unescaped: Vec<&String> = imported
        .iter()
        .filter(|name| sce_build::forge::generator::go_local_spelling(name) == **name)
        .collect();
    assert!(
        unescaped.is_empty(),
        "generated Go imports packages the generator does not escape: {unescaped:?}"
    );
}
