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

/// The kinds whose generated Python is a function an author's names are LOCALS
/// of, and so meet a builtin, an import or a template temporary by name.
const LOCAL_KINDS: &[&str] = &[
    "algorithm",
    "condition",
    "filter",
    "interpolation",
    "lookup",
    "observer",
    "transform",
    "validator",
];

/// Every kind this oracle runs. A procedure and a timer are classes: an
/// author's names are attributes and methods of them, and parameters of the
/// procedure's `execute(…)` wrapper, which is a different set of names to meet
/// (`PYTHON_PROCEDURE_MEMBERS`, `PYTHON_PROCEDURE_WRAPPER_NAMES`).
const KINDS: &[&str] = &[
    "algorithm",
    "condition",
    "filter",
    "interpolation",
    "lookup",
    "observer",
    "procedure",
    "timer",
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
    # A procedure class stores an author's value as `self._<name>` and gives it
    # `set_<name>`, so what an author's name meets there is an ATTRIBUTE or a
    # method of the class with its leading underscore taken off: a bare-name
    # scan sees `State` and `Event` and not `_is_final`.
    stored_as_attribute = m.group(1) in ("procedure", "timer")
    for node in ast.walk(tree):
        if isinstance(node, ast.Name):
            names.add(node.id)
        elif isinstance(node, ast.arg):
            names.add(node.arg)
        elif isinstance(node, (ast.FunctionDef, ast.ClassDef)):
            names.add(node.name)
            if stored_as_attribute:
                names.add(node.name.lstrip("_"))
        elif isinstance(node, ast.Attribute) and stored_as_attribute:
            names.add(node.attr.lstrip("_"))
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

# The base class every generated procedure extends keeps members of its own
# (`_service_handler`, `_done_data`, `_pending_event_data`), which no generated
# module shows unless it happens to use them.
base = ast.parse(open(sys.argv[2], encoding="utf-8").read())
for node in ast.walk(base):
    if isinstance(node, ast.Attribute):
        per_kind.setdefault("procedure", set()).add(node.attr.lstrip("_"))
    elif isinstance(node, ast.FunctionDef):
        per_kind.setdefault("procedure", set()).add(node.name.lstrip("_"))

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
        .arg(python_runtime().join("sce_forge_runtime/procedure.py"))
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
    let private: BTreeSet<String> = LOCAL_KINDS
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

/// What a generated procedure class and its `execute(…)` wrapper keep for
/// themselves, read off the committed output and the runtime base class: the
/// members the class and its base own (the methods the template defines and the
/// attributes the base sets, without their leading `_`), the names the wrapper
/// binds besides its author's parameters, and the public setters of the base.
const DERIVE_PROCEDURE: &str = r#"
import ast, glob, json, re, sys

KIND = re.compile(r'sce:kind="procedure"')
members, wrapper, setters = set(), set(), set()

base = ast.parse(open(sys.argv[2], encoding="utf-8").read())
for cls in [n for n in ast.walk(base) if isinstance(n, ast.ClassDef) and n.name == "ProcedureStateMachine"]:
    for node in ast.walk(cls):
        if isinstance(node, ast.FunctionDef):
            if node.name.startswith("set_"):
                setters.add(node.name[len("set_"):])
            elif node.name.startswith("_") and not node.name.startswith("__"):
                members.add(node.name.lstrip("_"))
        elif (isinstance(node, ast.Attribute) and isinstance(node.value, ast.Name)
              and node.value.id == "self" and node.attr.startswith("_")
              and not node.attr.startswith("__")):
            members.add(node.attr.lstrip("_"))

procedures = 0
for path in sorted(glob.glob(sys.argv[1] + "/*.py")):
    text = open(path, encoding="utf-8").read()
    if not KIND.search(text):
        continue
    procedures += 1
    tree = ast.parse(text)
    for cls in [n for n in tree.body if isinstance(n, ast.ClassDef) and n.name not in ("State", "Event")]:
        for node in cls.body:
            if isinstance(node, ast.FunctionDef) and node.name.startswith("_") and not node.name.startswith("__"):
                members.add(node.name.lstrip("_"))
    for fn in [n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == "execute"]:
        wrapper.add(fn.args.args[0].arg)
        for node in ast.walk(fn):
            if isinstance(node, ast.Name) and isinstance(node.ctx, ast.Store):
                wrapper.add(node.id)

print(json.dumps({"procedures": procedures, "members": sorted(members),
                  "wrapper": sorted(wrapper), "setters": sorted(setters)}))
"#;

/// The names a procedure class and its wrapper keep for themselves are the ones
/// the output shows. A template that adds a method, or a base class that gains
/// a member or a setter, fails here with the name, and the behavioural test
/// below runs each as the author's name.
#[test]
fn the_names_a_generated_procedure_keeps_for_itself_are_the_ones_its_output_shows() {
    if !require_python() {
        return;
    }
    let out = Command::new("python3")
        .arg("-c")
        .arg(DERIVE_PROCEDURE)
        .arg(expected_dir())
        .arg(python_runtime().join("sce_forge_runtime/procedure.py"))
        .output()
        .expect("python3 derives the names");
    assert!(
        out.status.success(),
        "deriving the names failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let json: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("the derivation prints JSON");
    let names = |key: &str| -> BTreeSet<String> {
        json[key]
            .as_array()
            .expect("a list")
            .iter()
            .map(|v| v.as_str().expect("a string").to_string())
            .collect()
    };
    assert!(
        json["procedures"].as_u64().unwrap_or(0) >= 5,
        "implausibly few procedure goldens read: {}",
        json["procedures"]
    );
    let listed =
        |names: &[&str]| -> BTreeSet<String> { names.iter().map(|s| s.to_string()).collect() };
    assert_eq!(
        names("members"),
        listed(sce_build::forge::generator::PYTHON_PROCEDURE_MEMBERS),
        "PYTHON_PROCEDURE_MEMBERS is not what a generated procedure class and its base own"
    );
    assert_eq!(
        names("wrapper"),
        listed(sce_build::forge::generator::PYTHON_PROCEDURE_WRAPPER_NAMES),
        "PYTHON_PROCEDURE_WRAPPER_NAMES is not what the generated execute(…) binds itself"
    );
    assert_eq!(
        names("setters"),
        listed(sce_build::reader_names::PYTHON_PROCEDURE_SETTERS),
        "PYTHON_PROCEDURE_SETTERS is not the set of setters the procedure base class defines"
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
    // What a procedure and a timer call by name beyond their data: a state is
    // an enum member, a helper a member and a setter, and a timer's events and
    // state are spelled into the names of its methods. Each is an identifier
    // the author chose and the generated Python declares. A transition's own
    // `event` is not here: a procedure's `ok` and `fail` are the two answers a
    // service gives, hard-wired to `Event.Ok` and `Event.Fail`, which is a
    // vocabulary and not a name an author is free to change.
    let named_re = Regex::new(
        r#"(?:<(?:state|final)\b[^>]*?\bid|<sce:helper\b[^>]*?\bname|<sce:reset-on\b[^>]*?\bevent|<sce:cancel-on\b[^>]*?\bstate-exit)="([A-Za-z_][A-Za-z0-9_]*)"|<sce:fire-event>([A-Za-z_][A-Za-z0-9_]*)</sce:fire-event>"#,
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
        let mut declared: BTreeSet<String> = declared_re
            .captures_iter(&text)
            .map(|c| c[1].to_string())
            .collect();
        if matches!(kind.as_str(), "procedure" | "timer") {
            declared.extend(
                named_re
                    .captures_iter(&text)
                    .filter_map(|c| c.get(1).or_else(|| c.get(2)))
                    .map(|m| m.as_str().to_string()),
            );
        }
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

from sce_forge_runtime.procedure import ProcedureServiceResponse

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

    # The name as written maps to the other name as written, exactly: a state
    # id is carried into a result as the author spelled it. The other
    # spellings follow the case of the word they replace, and where two of them
    # are one string (a capitalised `Accept` is both the as-written and the
    # Pascal spelling) the first, the exact one, is kept.
    merged = {}
    for i, (a, b) in enumerate(zip(spellings(src), spellings(dst))):
        if i == 0:
            merged[a] = b
        elif a not in merged:
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


class RuntimeFields(dict):
    """The fields of one of the runtime's own result types. Their names are the
    runtime's, so a rename of an author's name never reaches them."""


def deep(v, f):
    """`f` over every string and dictionary key of a plain result."""
    if isinstance(v, str):
        return f(v)
    if isinstance(v, RuntimeFields):
        return RuntimeFields({k: deep(x, f) for k, x in v.items()})
    if isinstance(v, dict):
        return {f(k): deep(x, f) for k, x in v.items()}
    if isinstance(v, list):
        return [deep(x, f) for x in v]
    return v


LOG = []


class Recorder:
    """Stands in for a collaborator the generated class is handed — a timer, a
    fire handler — and records what it was asked, with a callback by its name:
    the order of the calls and the method a firing is routed to are the
    behaviour of a timer, and a name that moved is what this exists to see."""

    def __getattr__(self, name):
        def call(*args):
            LOG.append([name] + [
                "callable:" + getattr(a, "__name__", "?") if callable(a) else a for a in args
            ])
        call.__name__ = name
        return call


def service_handler():
    """A service handler that answers success, failure, success …, so both
    outcomes of a procedure's `<send>` are taken across a run."""
    answers = []

    def handle(request):
        answers.append(1)
        return ProcedureServiceResponse(success=len(answers) % 2 == 1, data="ok")

    return handle


def example(annotation, k):
    if isinstance(annotation, ast.Subscript) and getattr(annotation.value, "id", None) == "Callable":
        returns = annotation.slice.elts[-1] if isinstance(annotation.slice, ast.Tuple) else None
        if getattr(returns, "id", None) == "ProcedureServiceResponse":
            return service_handler()
        return lambda *args: 1
    name = annotation.id if isinstance(annotation, ast.Name) else None
    if name == "Timer" or (name or "").endswith("Handler"):
        return Recorder()
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
        # A field of the runtime's own result type (`final_state`, `done_data`)
        # is the runtime's name, not the author's, and a candidate spelled
        # `state` or `done` must not be read into it.
        if type(v).__module__.startswith("sce_forge_runtime"):
            return RuntimeFields({f.name: conv(getattr(v, f.name), tr) for f in dataclasses.fields(v)})
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
        return [["import raises " + type(e).__name__, None]]
    # Each entry is [what happened, its data]. Only the data carries an
    # author's name, so only the data is ever translated when two results are
    # compared: the harness's own words (`init raises`, `log`) stay as they are,
    # whatever a candidate name happens to be spelled like.
    results = []
    LOG.clear()
    seen = 0

    def observed():
        """What a stand-in collaborator was asked since the last look."""
        nonlocal seen
        new = LOG[seen:]
        seen = len(LOG)
        return ["log", conv(new, tr)] if new else None

    for node in tree.body:
        if isinstance(node, ast.FunctionDef) and not node.name.startswith("_"):
            args = [example(a.annotation, 0) for a in node.args.args]
            try:
                results.append(["fn", conv(getattr(mod, node.name)(*args), tr)])
            except BaseException as e:  # noqa: BLE001
                results.append(["fn raises " + type(e).__name__, None])
        elif isinstance(node, ast.ClassDef) and not node.name.startswith("_"):
            init = next((f for f in node.body if isinstance(f, ast.FunctionDef) and f.name == "__init__"), None)
            init_args = [example(a.annotation, 0) for a in (init.args.args[1:] if init else [])]
            try:
                obj = getattr(mod, node.name)(*init_args)
            except BaseException as e:  # noqa: BLE001
                results.append(["cls init raises " + type(e).__name__, None])
                continue
            for f in node.body:
                if isinstance(f, ast.FunctionDef) and not f.name.startswith("_"):
                    for k in range(4):
                        args = [example(a.annotation, k) for a in f.args.args[1:]]
                        try:
                            results.append(["m", conv(getattr(obj, f.name)(*args), tr)])
                        except BaseException as e:  # noqa: BLE001
                            results.append(["m raises " + type(e).__name__, None])
                        log = observed()
                        if log:
                            results.append(log)
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
    data_only = lambda r, f: [[tag, deep(data, f)] for tag, data in r]
    if got == want or data_only(got, back) == want or got == data_only(want, forward):
        continue
    # Where they part, not how they begin: a schedule is long and the first
    # hundred characters of two results are mostly the same calls.
    at = next((i for i, (a, b) in enumerate(zip(got, want)) if a != b), min(len(got), len(want)))
    show = lambda r: "@%d of %d: %s" % (at, len(r), (str(r[at]) if at < len(r) else "(ends)")[:140])
    failures.append([case, frm, to, show(got), show(want)])
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
    // Which documents and which declarations fail, and how many candidates
    // each, so that a cause shared by many renamings reads as one line. A
    // failure is named for the document, not the file the case was written to.
    let mut by_declaration: BTreeMap<(String, String), (BTreeSet<String>, String)> =
        BTreeMap::new();
    for f in failures {
        let case = f[0]
            .as_str()
            .unwrap_or("?")
            .rsplit('/')
            .next()
            .unwrap_or("?");
        let stem = case.split("__").next().unwrap_or("?").to_string();
        let entry = by_declaration
            .entry((stem, f[1].as_str().unwrap_or("?").to_string()))
            .or_insert_with(|| (BTreeSet::new(), f[3].as_str().unwrap_or("?").to_string()));
        entry.0.insert(f[2].as_str().unwrap_or("?").to_string());
    }
    let grouped: Vec<String> = by_declaration
        .iter()
        .map(|((stem, declared), (names, example))| {
            let shown: Vec<&str> = names.iter().take(14).map(String::as_str).collect();
            format!(
                "  {:>4} x {stem} `{declared}` renamed to {}{}\n         e.g. {example}",
                names.len(),
                shown.join(", "),
                if names.len() > shown.len() {
                    ", ..."
                } else {
                    ""
                }
            )
        })
        .collect();
    assert!(
        failures.is_empty(),
        "{} of {} accepted renamings make the generated Python do something else \
         ({attempts} tried, {refused} refused, {folded_together} left out because two of the \
         author's own names would snake-case to one):\n{}\n\nfirst failures in full:\n{}",
        failures.len(),
        report["pairs"],
        grouped.join("\n"),
        failures
            .iter()
            .take(12)
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

/// A procedure input or helper called `service_handler` is refused, in the
/// document, with the reason: its public setter would be `set_service_handler`,
/// which the procedure base class defines and the generated wrapper calls to
/// hand the service handler over. Every other name that meets the class's own
/// members is stored under a shifted spelling and needs no refusal, and the
/// behavioural test above runs them.
#[test]
fn an_input_or_helper_named_for_an_inherited_setter_is_refused_with_the_reason() {
    use sce_build::forge::diagnostic::ToDiagnostics;

    fn procedure(declaration: &str) -> String {
        format!(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="procedure" name="p_setter" initial="a">
  <datamodel>
    {declaration}
  </datamodel>
  <state id="a"><transition target="done"/></state>
  <final id="done"/>
</scxml>"#
        )
    }
    let refused = |declaration: &str| {
        let label = DocumentLabel {
            identifier: "p_setter",
            diagnostic_label: "p_setter.scxml",
        };
        sce_build::forge::parser::parse_forge_with_imports(&procedure(declaration), label)
            .expect_err("the document is refused")
    };

    // A camelCase spelling is refused too: it is Python's spelling that
    // clashes, and the message names both.
    for written in ["service_handler", "serviceHandler"] {
        let declaration =
            format!(r#"<data id="{written}" sce:type="float32" sce:direction="in"/>"#);
        let refusal = refused(&declaration);
        let diagnostic = &refusal.error.to_diagnostics()[0];
        assert_eq!(
            serde_json::to_string(&diagnostic.code).unwrap(),
            "\"validation/reserved-code-identifier\"",
            "{refusal:?}"
        );
        assert_eq!(diagnostic.actual.as_deref(), Some(written));
        let message = refusal.error.to_string();
        assert!(
            message.contains("set_service_handler") && message.contains("rename it"),
            "{message}"
        );
    }

    // A helper has a setter too.
    let helper = r#"<sce:helper name="service_handler" args="bytes" returns="bytes" sce:returns-max-size="8"/>"#;
    assert!(refused(helper)
        .error
        .to_string()
        .contains("set_service_handler"));

    // An internal has none: it is stored privately, and accepted.
    let internal =
        procedure(r#"<data id="service_handler" sce:type="float32" sce:direction="internal"/>"#);
    let label = DocumentLabel {
        identifier: "p_setter",
        diagnostic_label: "p_setter.scxml",
    };
    sce_build::forge::parser::parse_forge_with_imports(&internal, label)
        .expect("an internal named service_handler has no setter to clash");
}
