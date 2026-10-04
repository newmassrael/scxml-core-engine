// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A name an author gives a parameter, a variable or a datum of a forge kind
//! must not decide what the generated Python does.
//!
//! The codec kind has its own oracle
//! (`a_python_codec_keeps_an_authors_names_apart_from_its_own`); this is the
//! same question for the kinds whose generated Python is a FUNCTION the
//! author's names are locals of: algorithm, condition, filter, interpolation,
//! lookup, observer, transform and validator.
//!
//! Measured 2026-10-04 over 66 documents, before this file existed, 79 of
//! about 2400 renamings generated Python that did something else:
//!
//! - an algorithm variable or parameter called `len`, `bytes` or `bytearray`
//!   hid the builtin the next line calls (`n = len(data)` after a local `len`);
//! - a validator input called `abs` hid the `abs(…)` of its rate-of-change
//!   check, and one called `delta` was overwritten by the template's own
//!   `delta`, so the VALUE stored as its previous reading was the difference —
//!   no error, only a state that drifts;
//! - a transform input called `abs` hid the `abs` its integer remainder calls;
//! - an interpolation input spelled in camelCase or capitalised was declared in
//!   snake_case and read as written: a `NameError`.
//!
//! # What is measured
//!
//! Every document of those kinds under `tests/forge/resources` is generated
//! with each name it declares (`<data id>`, `<sce:param name>`,
//! `<sce:var name>`) renamed to each candidate, and the generated module is
//! RUN under one schedule — every public function called with example
//! arguments, every public class built and each public method called four
//! times with values that change between calls (a state that stores the wrong
//! thing shows on the third call, not the first). What it did is compared with
//! what the unrenamed document's module did, as a structure: a result's
//! dataclass fields, enum values, numbers and strings, and not the names of
//! its classes, which is what a rename legitimately changes.
//!
//! A name the parser or the generator refuses is an answer — the author is told
//! before any code exists. What may not happen is a name that is accepted and
//! then does something else.
//!
//! The candidates are derived, not listed (see [`candidates_by_kind`]): every
//! builtin, joined with every identifier the committed Python of that kind
//! uses, joined with a few SHAPES a name can take (camelCase, capitalised, with
//! a digit) because a spelling that is wrong for a shape is wrong for every
//! author who writes in it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;
use sce_build::generator::Language;
use sce_build::{compile_forge_with_imports, DocumentLabel, ForgeCompileOptions};

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

/// Spellings a name can take, given as candidates whatever the kind.
const SHAPES: &[&str] = &["engineRpm", "Foo", "MY_CONST", "x1", "rawValue"];

fn resource_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources")
}

fn expected_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/expected")
}

fn python_runtime() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../backends/python/forge-runtime")
}

/// Whether `python3` can be run here. Absent is reported, and refused under
/// `SCE_REQUIRE_ALL_COMPILERS`, as every other gate that needs a toolchain.
fn python_present() -> bool {
    Command::new("python3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn require_python() -> bool {
    if python_present() {
        return true;
    }
    assert!(
        std::env::var_os("SCE_REQUIRE_ALL_COMPILERS").is_none(),
        "python3 is required (SCE_REQUIRE_ALL_COMPILERS is set) and is absent"
    );
    eprintln!("python3 absent: the Python kind-name oracle was not run");
    false
}

/// What the committed Python of each kind uses, derived by Python's own `ast`:
/// per kind, every bare name, parameter, function, class and import alias of
/// that kind's goldens, plus every builtin; and, for the guard test, the
/// builtins loaded with no scope binding them and the lowercase names the
/// modules import, across all the kinds.
const DERIVE: &str = r#"
import ast, builtins, glob, json, keyword, re, sys

KIND = re.compile(r'sce:kind="([a-z_-]+)"')
BUILTINS = {n for n in dir(builtins) if n[:1].islower() and not n.startswith("_")}
per_kind = {}
unbound_builtins = set()
lowercase_imports = set()


def bound_in(scope):
    names = set()
    if isinstance(scope, (ast.FunctionDef, ast.AsyncFunctionDef, ast.Lambda)):
        a = scope.args
        for arg in a.posonlyargs + a.args + a.kwonlyargs:
            names.add(arg.arg)
        if a.vararg:
            names.add(a.vararg.arg)
        if a.kwarg:
            names.add(a.kwarg.arg)
    body = scope.body if isinstance(scope.body, list) else [scope.body]
    stack = list(body)
    while stack:
        node = stack.pop()
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
            names.add(node.name)
            continue
        if isinstance(node, ast.Lambda):
            continue
        if isinstance(node, ast.Name) and isinstance(node.ctx, (ast.Store, ast.Del)):
            names.add(node.id)
        if isinstance(node, (ast.Import, ast.ImportFrom)):
            for alias in node.names:
                names.add((alias.asname or alias.name).split(".")[0])
        if isinstance(node, ast.ExceptHandler) and node.name:
            names.add(node.name)
        stack.extend(ast.iter_child_nodes(node))
    return names


def walk(node, enclosing):
    bound = enclosing
    if isinstance(node, (ast.Module, ast.FunctionDef, ast.AsyncFunctionDef, ast.Lambda)):
        bound = enclosing | bound_in(node)
    if isinstance(node, ast.Name) and isinstance(node.ctx, ast.Load):
        if node.id in BUILTINS and node.id not in bound:
            unbound_builtins.add(node.id)
    for child in ast.iter_child_nodes(node):
        walk(child, bound)


for path in sorted(glob.glob(sys.argv[1] + "/*.py")):
    text = open(path, encoding="utf-8").read()
    m = KIND.search(text)
    if not m or m.group(1) == "codec":
        continue
    tree = ast.parse(text)
    names = per_kind.setdefault(m.group(1), set())
    for node in ast.walk(tree):
        if isinstance(node, ast.Name):
            names.add(node.id)
        elif isinstance(node, ast.arg):
            names.add(node.arg)
        elif isinstance(node, (ast.FunctionDef, ast.ClassDef)):
            names.add(node.name)
        elif isinstance(node, (ast.Import, ast.ImportFrom)):
            # A relative import is a sibling module the document's own name
            # produced (`from .condition_threshold import …`), not a name the
            # template brings.
            relative = isinstance(node, ast.ImportFrom) and node.level > 0
            for alias in node.names:
                bound = (alias.asname or alias.name).split(".")[0]
                names.add(bound)
                if bound[:1].islower() and not relative:
                    lowercase_imports.add(bound)
    walk(tree, set())

ok = lambda n: n.isidentifier() and not keyword.iskeyword(n) and not n.startswith("__")
out = {
    "per_kind": {k: sorted(n for n in (v | BUILTINS) if ok(n)) for k, v in per_kind.items()},
    "unbound_builtins": sorted(unbound_builtins),
    "lowercase_imports": sorted(lowercase_imports),
}
print(json.dumps(out))
"#;

struct Derived {
    per_kind: BTreeMap<String, Vec<String>>,
    unbound_builtins: BTreeSet<String>,
    lowercase_imports: BTreeSet<String>,
}

fn derive() -> Derived {
    let out = Command::new("python3")
        .arg("-c")
        .arg(DERIVE)
        .arg(expected_dir())
        .output()
        .expect("python3 derives the names");
    assert!(
        out.status.success(),
        "deriving the names failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let json: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("the derivation prints JSON");
    let strings = |value: &serde_json::Value| -> Vec<String> {
        value
            .as_array()
            .expect("a list")
            .iter()
            .map(|v| v.as_str().expect("a string").to_string())
            .collect()
    };
    Derived {
        per_kind: json["per_kind"]
            .as_object()
            .expect("per_kind is an object")
            .iter()
            .map(|(k, v)| (k.clone(), strings(v)))
            .collect(),
        unbound_builtins: strings(&json["unbound_builtins"]).into_iter().collect(),
        lowercase_imports: strings(&json["lowercase_imports"]).into_iter().collect(),
    }
}

/// The names the generated Python of non-codec kinds reaches for must all be
/// on the list the generator escapes. Derived from the committed output, so a
/// template that starts to call one more builtin, or imports one more
/// lowercase name, fails here with the name in the message; the behavioural
/// test below is the arbiter for everything this cannot see.
#[test]
fn the_names_generated_python_of_the_kinds_reaches_for_are_all_escaped() {
    if !require_python() {
        return;
    }
    let derived = derive();
    assert!(
        derived.per_kind.len() >= KINDS.len(),
        "implausibly few kinds derived ({:?}); the derivation broke",
        derived.per_kind.keys().collect::<Vec<_>>()
    );
    let listed: BTreeSet<&str> = sce_build::forge::generator::PYTHON_GENERATED_NAMES
        .iter()
        .copied()
        .collect();
    // The names a template writes that begin with a single underscore, for
    // the kinds this escape covers: they are the generator's by convention, and
    // an author may begin a name with `_` too.
    let private: BTreeSet<String> = KINDS
        .iter()
        .filter_map(|kind| derived.per_kind.get(*kind))
        .flatten()
        .filter(|n| n.starts_with('_') && !n.starts_with("__"))
        .filter(|n| {
            n.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        })
        .cloned()
        .collect();
    let missing: Vec<&String> = derived
        .unbound_builtins
        .iter()
        .chain(derived.lowercase_imports.iter())
        .chain(private.iter())
        // `self` is a keyword-like name the parser already refuses everywhere;
        // `float`, `int` and `bool` are C++ keywords and refused for the same
        // reason, so no author's name can be them.
        .filter(|n| !matches!(n.as_str(), "self" | "float" | "int" | "bool"))
        .filter(|n| !listed.contains(n.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "generated Python reaches for names PYTHON_GENERATED_NAMES does not escape: {missing:?}"
    );
}

/// How a name is spelled once the generator has snake-cased it, for the check
/// that two author names do not fold into one spelling.
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

/// How a document is rewritten: every whole-word occurrence of `from` that is
/// not a member access (`x.from`) becomes `to`.
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

/// Generate `text` as Python; the module's source, or the reason it was refused.
fn generate(stem: &str, text: &str) -> Result<String, String> {
    compile_forge_with_imports(
        text,
        DocumentLabel::symmetric(stem),
        Language::Python,
        &resource_dir(),
        &ForgeCompileOptions::default(),
    )
    .map_err(|e| e.error.to_string())
    .and_then(|output| {
        let modules: Vec<_> = output
            .files
            .into_iter()
            .filter(|(file, _)| file.ends_with(".py"))
            .collect();
        match modules.len() {
            1 => Ok(modules.into_iter().next().unwrap().1),
            n => Err(format!("{n} python modules, not one")),
        }
    })
}

/// Runs each (baseline, renamed) pair under one schedule and reports the pairs
/// that differ. Reads a JSON list of `[base_path, case_path, from, to]`.
const RUNNER: &str = r#"
import ast, dataclasses, enum, importlib.util, json, re, sys

pairs = json.load(open(sys.argv[1], encoding="utf-8"))


def spellings(name):
    snake = re.sub(r"(?<=[a-z0-9])([A-Z])", r"_\1", name).lower()
    pascal = "".join(p[:1].upper() + p[1:] for p in snake.split("_") if p)
    return [name, name[:1].upper() + name[1:], name.upper(), snake, pascal, snake.upper()]


def translator(src, dst):
    """Rewrites each whole-word spelling of `src` in a string as `dst`'s.

    A word is delimited by anything that is not a letter or a digit, so the
    underscore of `rpm_out_of_range` ends `rpm` and the `id` inside `valid`
    is not a word. Without that, the short names an author may give (`id`,
    `set`) were replaced inside words that are not theirs.
    """
    def cased_like(old, new):
        if not (old[:1].isalpha() and new[:1].isalpha()):
            return old
        return (old[:1].upper() if new[:1].isupper() else old[:1].lower()) + old[1:]

    merged = {}
    for a, b in zip(spellings(src), spellings(dst)):
        merged[a] = cased_like(b, a)
    ordered = sorted(merged.items(), key=lambda p: -len(p[0]))
    patterns = [re.compile(r"(?<![A-Za-z0-9])%s(?![A-Za-z0-9])" % re.escape(a)) for a, _ in ordered]

    def tr(s):
        for i, p in enumerate(patterns):
            s = p.sub("\x00%d\x00" % i, s)
        for i, (_a, b) in enumerate(ordered):
            s = s.replace("\x00%d\x00" % i, b)
        return s

    return tr


def deep(v, f):
    """`f` over every string and dictionary key of a plain result."""
    if isinstance(v, str):
        return f(v)
    if isinstance(v, dict):
        return {f(k): deep(x, f) for k, x in v.items()}
    if isinstance(v, list):
        return [deep(x, f) for x in v]
    return v


def example(annotation, k):
    name = annotation.id if isinstance(annotation, ast.Name) else None
    if name == "int":
        return 400 * (k + 1)
    if name == "float":
        return 20.0 * (k + 1)
    if name == "bool":
        return k % 2 == 0
    if name == "str":
        return "a" * (k + 1)
    if name == "bytes":
        return bytes([1, 2, 0, 3][: 4 - k % 2])
    return 1


def conv(v, tr):
    if v is None or isinstance(v, (bool, int, float)):
        return v
    if isinstance(v, (bytes, bytearray)):
        return list(v)
    if isinstance(v, str):
        return tr(v)
    if isinstance(v, enum.Enum):
        return ["enum", conv(v.value, tr)]
    if dataclasses.is_dataclass(v) and not isinstance(v, type):
        return {tr(f.name): conv(getattr(v, f.name), tr) for f in dataclasses.fields(v)}
    if isinstance(v, (list, tuple)):
        return [conv(x, tr) for x in v]
    try:
        return [conv(x, tr) for x in iter(v)]
    except TypeError:
        return type(v).__name__


def load(path, tag):
    spec = importlib.util.spec_from_file_location("sce_under_test_" + tag, path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def schedule(path, tag, tr):
    with open(path, encoding="utf-8") as handle:
        tree = ast.parse(handle.read())
    try:
        mod = load(path, tag)
    except BaseException as e:  # noqa: BLE001
        return [["import", type(e).__name__]]
    results = []
    for node in tree.body:
        if isinstance(node, ast.FunctionDef) and not node.name.startswith("_"):
            args = [example(a.annotation, 0) for a in node.args.args]
            try:
                results.append(["fn", conv(getattr(mod, node.name)(*args), tr)])
            except BaseException as e:  # noqa: BLE001
                results.append(["fn", "raises " + type(e).__name__])
        elif isinstance(node, ast.ClassDef) and not node.name.startswith("_"):
            init = next((f for f in node.body if isinstance(f, ast.FunctionDef) and f.name == "__init__"), None)
            init_args = [example(a.annotation, 0) for a in (init.args.args[1:] if init else [])]
            try:
                obj = getattr(mod, node.name)(*init_args)
            except BaseException as e:  # noqa: BLE001
                results.append(["cls", "init raises " + type(e).__name__])
                continue
            for f in node.body:
                if isinstance(f, ast.FunctionDef) and not f.name.startswith("_"):
                    for k in range(4):
                        args = [example(a.annotation, k) for a in f.args.args[1:]]
                        try:
                            results.append(["m", conv(getattr(obj, f.name)(*args), tr)])
                        except BaseException as e:  # noqa: BLE001
                            results.append(["m", "raises " + type(e).__name__])
    return results


identity = lambda s: s
baselines = {}
failures = []
for base, case, frm, to in pairs:
    if base not in baselines:
        baselines[base] = schedule(base, "b%d" % len(baselines), identity)
    want = baselines[base]
    got = schedule(case, "c", identity)
    # What a rename may legitimately change is a string or a key that CARRIES
    # the name (`rpm_out_of_range`), so the two results are equal when they are
    # the same, or when mapping the new name back to the old in the case's
    # strings, or the old to the new in the baseline's, makes them so. Either
    # direction is enough: a baseline literal that happens to equal the new
    # name (an enum value `STOP` and an input renamed `STOP`) would be
    # rewritten wrongly by one of them, never by both. A behaviour that really
    # differs matches under neither.
    back = translator(to, frm)
    forward = translator(frm, to)
    if got == want or deep(got, back) == want or got == deep(want, forward):
        continue
    failures.append([case, frm, to, str(got)[:150], str(want)[:150]])
print(json.dumps({"pairs": len(pairs), "failures": failures}))
"#;

#[test]
fn an_authors_name_never_decides_what_the_generated_python_of_a_kind_does() {
    if !require_python() {
        return;
    }
    let derived = derive();
    let docs = documents();
    assert!(
        docs.len() >= 40,
        "implausibly few documents of the eight kinds ({}); the scan broke",
        docs.len()
    );

    let proj = std::env::temp_dir().join(format!("sce_py_kind_names_{}", std::process::id()));
    std::fs::create_dir_all(&proj).expect("mkdir");

    let mut pairs: Vec<(String, String, String, String)> = Vec::new();
    let mut attempts = 0usize;
    let mut refused = 0usize;
    let mut skipped_documents: Vec<String> = Vec::new();
    let mut other_refusals: BTreeMap<String, usize> = BTreeMap::new();
    let mut folded_together = 0usize;

    for doc in &docs {
        // A document that does not generate on its own (it needs a sibling
        // module the test does not write) cannot be run, and is not a case.
        let Ok(baseline) = generate(&doc.stem, &doc.text) else {
            skipped_documents.push(doc.stem.clone());
            continue;
        };
        if baseline.contains("\nfrom .") || baseline.contains("\nimport .") {
            skipped_documents.push(doc.stem.clone());
            continue;
        }
        let base_path = proj.join(format!("{}__base.py", doc.stem));
        std::fs::write(&base_path, baseline).expect("write baseline");

        let candidates: BTreeSet<String> = derived
            .per_kind
            .get(&doc.kind)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .chain(SHAPES.iter().map(|s| s.to_string()))
            .collect();
        let folded: BTreeSet<String> = doc.declared.iter().map(|n| snake(n)).collect();
        for declared in &doc.declared {
            for candidate in &candidates {
                if candidate == declared || doc.declared.contains(candidate) {
                    continue;
                }
                // Two author names that snake-case to one spelling (`minRpm`
                // and `min_rpm`) are one parameter to the generated Python, and
                // a duplicate argument to the interpreter. That is a different
                // defect — two of the AUTHOR's names meeting, which nothing in
                // the generator's own names can prevent — and it is counted
                // here, not asked of this oracle.
                if folded.contains(&snake(candidate)) {
                    folded_together += 1;
                    continue;
                }
                attempts += 1;
                let renamed = rename(&doc.text, declared, candidate);
                match generate(&doc.stem, &renamed) {
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
                            || why.contains("unknown"))
                        {
                            let key: String = why.chars().take(70).collect();
                            *other_refusals.entry(key).or_default() += 1;
                        }
                    }
                    Ok(source) => {
                        let case_path =
                            proj.join(format!("{}__{}__{}.py", doc.stem, declared, candidate));
                        std::fs::write(&case_path, source).expect("write case");
                        pairs.push((
                            base_path.to_string_lossy().to_string(),
                            case_path.to_string_lossy().to_string(),
                            declared.clone(),
                            candidate.clone(),
                        ));
                    }
                }
            }
        }
    }

    // A pass over nothing is not a pass: most of what was tried must have been
    // generated and run, and enough documents must have been usable. Some are
    // not, by design — a fixture that exists to be refused, one that imports a
    // sibling module this test does not write — and the floor is on the
    // documents that WERE run, not on a ratio the fixtures' mix would move.
    assert!(
        pairs.len() * 10 >= attempts * 7,
        "only {} of {attempts} renamings generated ({refused} refused); the oracle is mostly \
         asking nothing. Refusals not about a name: {other_refusals:?}",
        pairs.len()
    );
    let documents_run = docs.len() - skipped_documents.len();
    assert!(
        documents_run >= 40 && pairs.len() >= 2000,
        "only {documents_run} documents and {} renamings were run; skipped: {skipped_documents:?}",
        pairs.len()
    );

    let manifest = proj.join("pairs.json");
    std::fs::write(&manifest, serde_json::to_string(&pairs).expect("json")).expect("manifest");
    let out = Command::new("python3")
        .arg("-W")
        .arg("error")
        .arg("-c")
        .arg(RUNNER)
        .arg(&manifest)
        .env("PYTHONPATH", python_runtime())
        .current_dir(&proj)
        .output()
        .expect("python3 runs the schedule");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let _ = std::fs::remove_dir_all(&proj);
    assert!(
        out.status.success(),
        "the runner itself failed:\n{stderr}\n{stdout}"
    );

    let report: serde_json::Value =
        serde_json::from_str(stdout.lines().last().unwrap_or("")).expect("the runner prints JSON");
    let failures = report["failures"].as_array().expect("failures is a list");
    assert!(
        failures.is_empty(),
        "{} of {} accepted renamings make the generated Python do something else \
         ({attempts} tried, {refused} refused, {folded_together} left out because two of the \
         author's own names would snake-case to one):\n{}",
        failures.len(),
        report["pairs"],
        failures
            .iter()
            .take(30)
            .map(|f| format!(
                "  {} : {} -> {}\n      got:  {}\n      want: {}",
                f[0].as_str()
                    .unwrap_or("?")
                    .rsplit('/')
                    .next()
                    .unwrap_or("?"),
                f[1].as_str().unwrap_or("?"),
                f[2].as_str().unwrap_or("?"),
                f[3].as_str().unwrap_or("?"),
                f[4].as_str().unwrap_or("?"),
            ))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
