// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A name an author gives a codec field must not decide what the generated
//! Python does.
//!
//! The generated decoder binds each value it reads to a local named after
//! its field, in the same function that calls `bytes(...)`, reads from
//! `cursor` and builds the result with `cls(...)`. A field called `bytes`
//! therefore made `bytes = bytes(raw)` an `UnboundLocalError`, a field called
//! `cursor` reassigned the cursor the next field is read from, and a field
//! called `cls` made `cls(...)` a call on an integer — none of them refused,
//! all of them generated, and none of them visible to a gate that only
//! compiles the output. A wire specification names its fields `len`, `bytes`,
//! `list`, `range`, `data`; a codec generator that works only when the
//! author's names stay clear of its own is a restriction nobody agreed to.
//!
//! # What is measured
//!
//! Every name that could be in the way — each Python builtin and each
//! identifier the committed Python output itself uses — is given to a field of
//! each shape that binds a local (a fixed field, a VLE, both roles of a
//! length-ref pair, a string, a flags carrier a later field is gated on, a
//! tail, a repeat and its count, a TLV chain, an embed). The generated module
//! is run: a frame is decoded and encoded back to the same bytes.
//!
//! The names are DERIVED, not listed: a hand-written list of "names to keep
//! away from" is exactly what drifts when a template starts to use one more
//! builtin. The set is `dir(builtins)` joined with every bare identifier and
//! parameter in `tests/forge/expected/*.py`, so a template that begins to use
//! a name widens the question by itself.
//!
//! A name the parser refuses (`validation/reserved-code-identifier`, a
//! keyword in some target language) is an answer, not a failure: the author is
//! told, in the document, before any code exists. What may not happen is a
//! name that is accepted and then breaks the generated code.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::generator::Language;
use sce_build::{compile_forge_with_imports, DocumentLabel, ForgeCompileOptions};

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

/// The documents a shape imports, in dependency order.
const ENTRY_SET: &[&str] = &[
    "codec_zenoh_ext_unit.scxml",
    "codec_zenoh_ext_zint.scxml",
    "codec_zenoh_ext_zbuf.scxml",
    "codec_zenoh_ext_entry.scxml",
];
const SLICE_SET: &[&str] = &["codec_chain_has_marker_slice.scxml"];

/// One construct that binds a local to the value it decodes, with the name
/// under test standing where an author's name stands.
struct Shape {
    id: &'static str,
    /// Resource documents it imports, generated alongside it.
    imports: &'static [&'static str],
    /// The `<datamodel>` body; `NAME` is replaced by the candidate.
    datamodel: &'static str,
    /// Frames that must decode and encode back to themselves.
    frames: &'static [&'static [u8]],
    /// What the attribute named by the author must hold after the FIRST frame
    /// is decoded, as a Python expression over `{v}` (that attribute). A
    /// round trip alone passes when a field replaces a method of the class that
    /// happens to be unused by it; reading the field back does not.
    check: &'static str,
}

const SHAPES: &[Shape] = &[
    Shape {
        id: "fixed",
        imports: &[],
        datamodel: r#"
    <sce:field id="NAME" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:field id="b" sce:type="uint16" sce:byte="1" sce:bit-size="16"/>"#,
        frames: &[&[1, 0, 2]],
        check: "{v} == 1",
    },
    Shape {
        id: "vle",
        imports: &[],
        datamodel: r#"
    <sce:field id="NAME" sce:type="uint32" sce:byte="0" sce:bit-size="vle"/>
    <sce:field id="z" sce:type="uint8" sce:byte="1" sce:bit-size="8"/>"#,
        frames: &[&[5, 3]],
        check: "{v} == 5",
    },
    Shape {
        id: "bytes_named",
        imports: &[],
        datamodel: r#"
    <sce:field id="n" sce:type="uint64" sce:byte="0" sce:bit-size="vle"/>
    <sce:field id="NAME" sce:type="bytes" sce:byte="1" sce:bit-size="length-ref"
               sce:length-field="n" sce:max-size="16"/>"#,
        frames: &[&[2, 0xAA, 0xBB]],
        check: "{v} == bytes([0xAA, 0xBB])",
    },
    Shape {
        id: "length_named",
        imports: &[],
        datamodel: r#"
    <sce:field id="NAME" sce:type="uint64" sce:byte="0" sce:bit-size="vle"/>
    <sce:field id="data" sce:type="bytes" sce:byte="1" sce:bit-size="length-ref"
               sce:length-field="NAME" sce:max-size="16"/>"#,
        frames: &[&[2, 0xAA, 0xBB]],
        check: "{v} == 2",
    },
    Shape {
        id: "string_named",
        imports: &[],
        datamodel: r#"
    <sce:field id="n" sce:type="uint64" sce:byte="0" sce:bit-size="vle"/>
    <sce:field id="NAME" sce:type="string" sce:byte="1" sce:bit-size="length-ref"
               sce:length-field="n" sce:max-size="16"/>"#,
        frames: &[&[2, 0x68, 0x69]],
        check: "{v} == 'hi'",
    },
    Shape {
        id: "flags_gate",
        imports: &[],
        datamodel: r#"
    <sce:flags id="NAME" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="on" bit="0"/>
    </sce:flags>
    <sce:field id="x" sce:type="uint8" sce:byte="1" sce:bit-size="8"
               sce:present-if="NAME.on"/>"#,
        frames: &[&[1, 7], &[0]],
        check: "{v} == 1",
    },
    Shape {
        id: "tail",
        imports: &[],
        datamodel: r#"
    <sce:field id="h" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:field id="NAME" sce:type="bytes" sce:byte="1" sce:bit-size="tail"
               sce:max-size="16"/>"#,
        frames: &[&[1, 2, 3]],
        check: "{v} == bytes([2, 3])",
    },
    Shape {
        id: "repeat_count",
        imports: SLICE_SET,
        datamodel: r#"
    <sce:field id="NAME" sce:type="uint32" sce:byte="0" sce:bit-size="vle"/>
    <sce:repeat id="items" type="codec_chain_has_marker_slice" sce:byte="1"
                count="NAME" max-count="4"/>"#,
        frames: &[&[1, 0, 2, 0xAA, 0xBB], &[0]],
        check: "{v} == 1",
    },
    Shape {
        id: "repeat_named",
        imports: SLICE_SET,
        datamodel: r#"
    <sce:field id="n" sce:type="uint32" sce:byte="0" sce:bit-size="vle"/>
    <sce:repeat id="NAME" type="codec_chain_has_marker_slice" sce:byte="1"
                count="n" max-count="4"/>"#,
        frames: &[&[1, 0, 2, 0xAA, 0xBB], &[0]],
        check: "len({v}) == 1",
    },
    Shape {
        id: "chain_named",
        imports: ENTRY_SET,
        datamodel: r#"
    <sce:tlv-chain id="NAME" type="codec_zenoh_ext_entry" sce:byte="0"
                   max-depth="4" on-overflow="reject"
                   terminate-on="entry-flag" entry-flag-name="Z"/>
    <sce:field id="z" sce:type="uint8" sce:byte="1" sce:bit-size="8"/>"#,
        frames: &[&[0x03, 9], &[0x83, 0x04, 9]],
        check: "len({v}) == 1",
    },
    Shape {
        id: "embed_named",
        imports: SLICE_SET,
        datamodel: r#"
    <sce:field id="h" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:embed id="NAME" type="codec_chain_has_marker_slice" sce:byte="1"/>"#,
        frames: &[&[1, 0, 2, 0xAA, 0xBB]],
        check: "{v} is not None",
    },
];

/// Every name that could be in the way, derived. See the module comment.
fn candidate_names() -> Vec<String> {
    let script = r#"
import ast, builtins, glob, re, sys
names = set(dir(builtins))
for path in glob.glob(sys.argv[1] + "/*.py"):
    tree = ast.parse(open(path, encoding="utf-8").read())
    for node in ast.walk(tree):
        if isinstance(node, ast.Name):
            names.add(node.id)
        elif isinstance(node, ast.arg):
            names.add(node.arg)
        elif isinstance(node, (ast.FunctionDef, ast.ClassDef)):
            names.add(node.name)
print("\n".join(sorted(n for n in names if re.fullmatch(r"[a-z_][a-z0-9_]*", n))))
"#;
    let out = Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(expected_dir())
        .output()
        .expect("python3 runs the name-derivation script");
    assert!(
        out.status.success(),
        "deriving the candidate names failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let ids_the_shapes_use: BTreeSet<&str> = ["n", "b", "z", "x", "h", "data", "items"]
        .into_iter()
        .collect();
    String::from_utf8(out.stdout)
        .expect("utf-8 names")
        .lines()
        .map(str::to_string)
        // A shape's own fixed ids are not candidates: a field cannot be
        // declared twice, and that is a different refusal.
        .filter(|n| !ids_the_shapes_use.contains(n.as_str()))
        .collect()
}

enum Outcome {
    /// The parser refused the name, in the document.
    Refused,
    /// Generated: the module name and its source.
    Generated(String, String),
}

/// The document is named by position, never by the candidate: a document
/// whose name ends in `coding` has a first line Python reads as a source
/// encoding declaration (`# SCE-MAP: …coding:23`). The generator guards that
/// (`a_python_module_is_never_read_as_declaring_an_encoding`), and it stays a
/// separate matter from what this oracle asks: were it not guarded, it would
/// hide the answer behind a `SyntaxError`.
fn generate(shape: &Shape, index: usize, name: &str) -> Result<Outcome, String> {
    let doc_name = format!("p_{}_{}", shape.id, index);
    let imports: String = shape
        .imports
        .iter()
        .filter(|f| !ENTRY_SET[..3].contains(f))
        .map(|f| {
            let stem = f.trim_end_matches(".scxml");
            format!("  <sce:import src=\"{f}\" kind=\"codec\" as=\"{stem}\"/>\n")
        })
        .collect();
    let scxml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="{doc_name}">
{imports}  <datamodel>{}
  </datamodel>
</scxml>"#,
        shape.datamodel.replace("NAME", name)
    );
    match compile_forge_with_imports(
        &scxml,
        DocumentLabel::symmetric(&doc_name),
        Language::Python,
        &resource_dir(),
        &ForgeCompileOptions::default(),
    ) {
        Ok(output) => {
            let (module, source) = output
                .files
                .into_iter()
                .find(|(file, _)| file.ends_with(".py"))
                .ok_or_else(|| format!("{doc_name}: no .py file generated"))?;
            Ok(Outcome::Generated(
                Path::new(&module)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&doc_name)
                    .to_string(),
                source,
            ))
        }
        Err(e) => {
            let message = e.error.to_string();
            // `validation/reserved-code-identifier`, whichever the reason: a
            // keyword, or a name the generated class or call already uses.
            if message.contains("cannot declare it — rename it") {
                Ok(Outcome::Refused)
            } else {
                Err(format!("{doc_name}: {message}"))
            }
        }
    }
}

/// The names the backend keeps for itself are the ones the output shows.
///
/// A list written by hand is what drifts, so each one is derived from the
/// committed Python output and compared:
///
/// - the methods every generated codec class defines (`decode`, `encode`, …);
/// - the names a class body evaluates while it is built — a decorator, a
///   default — outside any function and outside the annotations, which are
///   lazy under `from __future__ import annotations`;
/// - the first parameters of `decode` and `encode`;
/// - that no local the generator names itself begins `f_`, the prefix an
///   author's field becomes in a decoder (`python_field_local`): each local
///   bound under that prefix is `f_` and the name of an attribute of its own
///   class.
///
/// A template edit that adds a method, a class-body reference or an `f_` name
/// makes this fail, and the failure says which list to extend.
#[test]
fn the_names_the_generated_python_keeps_for_itself_are_the_ones_its_output_shows() {
    if !python_present() {
        assert!(
            std::env::var_os("SCE_REQUIRE_ALL_COMPILERS").is_none(),
            "python3 is required (SCE_REQUIRE_ALL_COMPILERS is set) and is absent"
        );
        eprintln!("python3 absent: the derived-names check was not run");
        return;
    }
    let script = r#"
import ast, glob, json, sys

def loads(node):
    return {n.id for n in ast.walk(node) if isinstance(n, ast.Name)}

methods = None
class_body = set()
first_params = set()
unowned_f_locals = []
codec_classes = 0
for path in sorted(glob.glob(sys.argv[1] + "/*.py")):
    tree = ast.parse(open(path, encoding="utf-8").read())
    for cls in tree.body:
        if not isinstance(cls, ast.ClassDef):
            continue
        fns = {f.name: f for f in cls.body if isinstance(f, ast.FunctionDef)}
        if "decode" not in fns or "encode" not in fns:
            continue
        codec_classes += 1
        names = set(fns)
        methods = names if methods is None else methods & names
        for stmt in cls.body:
            if isinstance(stmt, ast.FunctionDef):
                for d in stmt.decorator_list:
                    class_body |= loads(d)
                for default in stmt.args.defaults + [k for k in stmt.args.kw_defaults if k]:
                    class_body |= loads(default)
            elif isinstance(stmt, ast.AnnAssign) and stmt.value is not None:
                class_body |= loads(stmt.value)
            elif isinstance(stmt, ast.Assign):
                class_body |= loads(stmt.value)
        for fname in ("decode", "encode"):
            args = fns[fname].args.args
            first_params |= {a.arg for a in args[:2]}
        attrs = {s.target.id for s in cls.body if isinstance(s, ast.AnnAssign)}
        for node in ast.walk(fns["decode"]):
            if isinstance(node, ast.Name) and isinstance(node.ctx, ast.Store):
                if node.id.startswith("f_") and node.id[2:] not in attrs:
                    unowned_f_locals.append(f"{path}:{cls.name}:{node.id}")
print(json.dumps({
    "codec_classes": codec_classes,
    "methods": sorted(methods or []),
    "class_body": sorted(n for n in class_body if n[:1].islower()),
    "first_params": sorted(first_params),
    "unowned_f_locals": unowned_f_locals,
}))
"#;
    let out = Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(expected_dir())
        .output()
        .expect("python3 derives the names");
    assert!(
        out.status.success(),
        "deriving the names failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let derived: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("the derivation prints JSON");
    let strings = |key: &str| -> BTreeSet<String> {
        derived[key]
            .as_array()
            .unwrap_or_else(|| panic!("{key} is a list"))
            .iter()
            .map(|v| v.as_str().expect("a string").to_string())
            .collect()
    };
    let listed =
        |names: &[&str]| -> BTreeSet<String> { names.iter().map(|s| s.to_string()).collect() };

    assert!(
        derived["codec_classes"].as_u64().unwrap_or(0) > 100,
        "implausibly few generated codec classes ({}); the derivation broke",
        derived["codec_classes"]
    );
    assert_eq!(
        strings("methods"),
        listed(sce_build::reader_names::PYTHON_CODEC_METHODS),
        "the methods every generated Python codec class defines are not the ones \
         reader_names::PYTHON_CODEC_METHODS lists"
    );
    assert_eq!(
        strings("class_body"),
        listed(sce_build::reader_names::PYTHON_CLASS_BODY_NAMES),
        "the names a generated Python class body evaluates are not the ones \
         reader_names::PYTHON_CLASS_BODY_NAMES lists"
    );
    let parameters = listed(sce_build::reader_names::PYTHON_CALL_PARAMETERS);
    assert!(
        strings("first_params").is_subset(&parameters),
        "decode and encode take a parameter reader_names::PYTHON_CALL_PARAMETERS does not list: {:?}",
        strings("first_params").difference(&parameters).collect::<Vec<_>>()
    );
    assert_eq!(
        parameters
            .difference(&strings("first_params"))
            .cloned()
            .collect::<BTreeSet<_>>(),
        listed(&["parent_flags", "tag"]),
        "PYTHON_CALL_PARAMETERS carries a name besides the always-present parameters \
         and the two a variant arm's decode threads"
    );
    assert!(
        derived["unowned_f_locals"]
            .as_array()
            .is_some_and(Vec::is_empty),
        "a decoder binds a local under the f_ prefix that is not an author's field: {}",
        derived["unowned_f_locals"]
    );
}

#[test]
fn an_authors_field_name_never_decides_what_the_generated_python_does() {
    if !python_present() {
        assert!(
            std::env::var_os("SCE_REQUIRE_ALL_COMPILERS").is_none(),
            "python3 is required (SCE_REQUIRE_ALL_COMPILERS is set) and is absent"
        );
        eprintln!("python3 absent: the Python name oracle was not run");
        return;
    }

    let names = candidate_names();
    assert!(
        names.len() > 100,
        "the derived candidate set is implausibly small ({}); the derivation broke",
        names.len()
    );

    let proj = std::env::temp_dir().join(format!("sce_py_names_{}", std::process::id()));
    let pkg = proj.join("names_pkg");
    std::fs::create_dir_all(&pkg).expect("mkdir");
    std::fs::write(pkg.join("__init__.py"), "").expect("write __init__");

    // The imported documents, generated once.
    let mut written: BTreeSet<String> = BTreeSet::new();
    for set in [ENTRY_SET, SLICE_SET] {
        for file in set {
            let src = std::fs::read_to_string(resource_dir().join(file)).expect("read import");
            let stem = file.trim_end_matches(".scxml");
            let output = compile_forge_with_imports(
                &src,
                DocumentLabel::symmetric(stem),
                Language::Python,
                &resource_dir(),
                &ForgeCompileOptions::default(),
            )
            .unwrap_or_else(|e| panic!("generating {file}: {:?}", e.error));
            for (module, source) in output.files {
                if module.ends_with(".py") && written.insert(module.clone()) {
                    std::fs::write(pkg.join(&module), source).expect("write import module");
                }
            }
        }
    }

    // One line per module the run must exercise: module, shape, name, frames.
    let mut manifest = String::new();
    let mut refused: BTreeSet<String> = BTreeSet::new();
    let mut generation_failures: Vec<String> = Vec::new();
    for shape in SHAPES {
        for (index, name) in names.iter().enumerate() {
            match generate(shape, index, name) {
                Ok(Outcome::Refused) => {
                    refused.insert(name.clone());
                }
                Ok(Outcome::Generated(module, source)) => {
                    std::fs::write(pkg.join(format!("{module}.py")), source).expect("write module");
                    let frames: Vec<String> = shape
                        .frames
                        .iter()
                        .map(|f| f.iter().map(|b| format!("{b:02x}")).collect::<String>())
                        .collect();
                    manifest.push_str(&format!(
                        "{module}\t{}\t{name}\t{}\t{}\n",
                        shape.id,
                        frames.join(","),
                        shape.check
                    ));
                }
                Err(e) => generation_failures.push(e),
            }
        }
    }
    std::fs::write(proj.join("manifest.tsv"), manifest).expect("write manifest");

    let runner = r#"
import importlib, inspect, sys, traceback
sys.path.insert(0, sys.argv[1])
from sce_forge_runtime.codec import SceCursor

failures = []
count = 0
for line in open(sys.argv[2], encoding="utf-8").read().splitlines():
    module, shape, name, frames, check = line.split("\t")
    count += 1
    try:
        mod = importlib.import_module("names_pkg." + module)
        cls = next(
            c for _, c in inspect.getmembers(mod, inspect.isclass)
            if c.__module__ == mod.__name__ and hasattr(c, "decode") and hasattr(c, "encode_to_bytes")
        )
        for number, hexframe in enumerate(frames.split(",")):
            frame = bytes.fromhex(hexframe)
            value = cls.decode(SceCursor(frame))
            assert value is not None, "decode returned None"
            again = value.encode_to_bytes()
            assert again == frame, f"round trip {again.hex()} != {frame.hex()}"
            if number == 0:
                held = getattr(value, name)
                assert eval(check.replace("{v}", "held")), f"the field reads back as {held!r}"
    except BaseException as e:
        last = traceback.format_exc().strip().splitlines()[-1]
        failures.append(f"{shape}\t{name}\t{last}")
print(f"ran {count} generated modules")
for f in failures:
    print("FAIL\t" + f)
"#;
    let out = Command::new("python3")
        .arg("-W")
        .arg("error")
        .arg("-c")
        .arg(runner)
        .arg(&proj)
        .arg(proj.join("manifest.tsv"))
        .env("PYTHONPATH", python_runtime())
        .current_dir(&proj)
        .output()
        .expect("python3 runs the generated modules");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let _ = std::fs::remove_dir_all(&proj);

    assert!(
        out.status.success(),
        "the runner itself failed:\n{stderr}\n{stdout}"
    );
    assert!(
        generation_failures.is_empty(),
        "a document built for this oracle was refused for a reason other than a reserved word:\n{}",
        generation_failures.join("\n")
    );
    let failures: Vec<&str> = stdout.lines().filter(|l| l.starts_with("FAIL\t")).collect();
    assert!(
        failures.is_empty(),
        "{} accepted field name(s) break the generated Python ({} candidate names, {} refused by \
         the parser):\n{}",
        failures.len(),
        names.len(),
        refused.len(),
        failures.join("\n")
    );
}
