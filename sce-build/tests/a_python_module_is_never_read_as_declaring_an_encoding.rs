// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A document's name must not decide whether the generated Python can be
//! imported.
//!
//! The first line of every generated module is the source-map marker,
//! `# SCE-MAP: <document>:<line> :: <symbol>`. PEP 263 reads an encoding out
//! of a comment on the first two lines of a file wherever `coding` is followed
//! by `:` or `=` and a name, so a document called `zenoh_encoding`,
//! `transcoding` or `recoding` made that line `…coding:23`, the declaration of
//! an encoding called `23`. The library hands that module back as it is, and
//! every import of it is a `SyntaxError: unknown encoding: 23`.
//!
//! Nothing that compiles the output as a string sees it, because a str source
//! ignores the declaration; the file has to be read as bytes, which is what an
//! import does. So the oracle is the interpreter's own reading of the file:
//! `tokenize.detect_encoding` over the bytes written, then an import.
//!
//! # What is measured
//!
//! Names that end in `coding` are generated for two kinds (a codec, which is
//! then decoded and encoded back to the same bytes, and a lookup, which is
//! imported) and each module is read as a file. Names that contain `coding`
//! without the declaration shape are the control: nothing may be added to
//! those, so only a module that was unreadable moves. Every committed Python
//! golden is read the same way, which is how `codec_zenoh_encoding.py` was
//! found: it had been committed in the unreadable state.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::generator::Language;
use sce_build::{compile_forge_with_imports, DocumentLabel, ForgeCompileOptions};

/// What a name is prefixed with, to end in `coding`. The bare word is the case
/// with nothing before it.
const STEMS: &[&str] = &["", "zenoh_en", "trans", "re", "de", "x_"];

/// Names that hold `coding` without the shape PEP 263 reads (`coding`, then
/// `:` or `=`): the marker line puts `:` after the document name, so none of
/// these ends in `coding`.
const CONTROLS: &[&str] = &[
    "coding_notes",
    "encoding2",
    "my_coding_table",
    "codec_plain",
];

/// The line a module is declared UTF-8 by when its first two lines would
/// otherwise be read as a declaration nobody wrote.
const DECLARATION: &str = "# -*- coding: utf-8 -*-";

fn resource_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources")
}

fn python_runtime() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../backends/python/forge-runtime")
}

fn python_present() -> bool {
    Command::new("python3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn dangerous_names() -> Vec<String> {
    STEMS.iter().map(|s| format!("{s}coding")).collect()
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Codec,
    Lookup,
}

impl Kind {
    fn label(self) -> &'static str {
        match self {
            Kind::Codec => "codec",
            Kind::Lookup => "lookup",
        }
    }

    fn document(self, name: &str) -> String {
        match self {
            Kind::Codec => format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" name="{name}">
  <datamodel>
    <data id="value" sce:type="uint64" sce:byte="0" sce:bit-size="vle"/>
  </datamodel>
</scxml>"#
            ),
            Kind::Lookup => format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="lookup" name="{name}">
  <datamodel>
    <data id="engSta" sce:type="uint8" sce:direction="in"/>
    <data id="status" sce:type="string" sce:direction="out"/>
    <data id="mapping" sce:default="STOP">
      <sce:entry key="0x00" value="STOP"/>
      <sce:entry key="0x03" value="RUNNING"/>
    </data>
  </datamodel>
</scxml>"#
            ),
        }
    }
}

/// The module the library generates for `name`: its file name and source.
fn generate(kind: Kind, name: &str) -> (String, String) {
    let output = compile_forge_with_imports(
        &kind.document(name),
        DocumentLabel::symmetric(name),
        Language::Python,
        &resource_dir(),
        &ForgeCompileOptions::default(),
    )
    .unwrap_or_else(|e| panic!("generating {} `{name}`: {:?}", kind.label(), e.error));
    output
        .files
        .into_iter()
        .find(|(file, _)| file.ends_with(".py"))
        .unwrap_or_else(|| panic!("{} `{name}`: no .py file generated", kind.label()))
}

#[test]
fn a_name_that_ends_in_coding_does_not_make_the_module_unreadable() {
    if !python_present() {
        assert!(
            std::env::var_os("SCE_REQUIRE_ALL_COMPILERS").is_none(),
            "python3 is required (SCE_REQUIRE_ALL_COMPILERS is set) and is absent"
        );
        eprintln!("python3 absent: the Python encoding oracle was not run");
        return;
    }

    let proj = tempfile::tempdir().expect("temp dir");
    let pkg = proj.path().join("enc_pkg");
    std::fs::create_dir_all(&pkg).expect("mkdir");
    std::fs::write(pkg.join("__init__.py"), "").expect("write __init__");

    // One line per module the run reads: module, kind.
    let mut manifest = String::new();
    for kind in [Kind::Codec, Kind::Lookup] {
        for name in dangerous_names() {
            let (file, source) = generate(kind, &name);
            let first = source.lines().next().unwrap_or_default();
            assert_eq!(
                first,
                DECLARATION,
                "{} `{name}`: a first line the interpreter would read as an \
                 encoding declaration was left to stand:\n{first}",
                kind.label()
            );
            // The module keeps its own marker, one line down.
            assert!(
                source
                    .lines()
                    .nth(1)
                    .is_some_and(|l| l.starts_with(&format!("# SCE-MAP: {name}:"))),
                "{} `{name}`: the marker is no longer the line after the declaration",
                kind.label()
            );
            // Two kinds generate a module of the same name, so the kind is
            // part of where each is kept.
            let stored = format!("{}_{file}", kind.label());
            std::fs::write(pkg.join(&stored), source).expect("write module");
            let module = stored.trim_end_matches(".py");
            manifest.push_str(&format!("{module}\t{}\n", kind.label()));
        }
    }
    std::fs::write(proj.path().join("manifest.tsv"), manifest).expect("write manifest");

    let runner = r#"
import importlib, inspect, sys, tokenize, traceback
sys.path.insert(0, sys.argv[1])
from sce_forge_runtime.codec import SceCursor

failures = []
count = 0
for line in open(sys.argv[2], encoding="utf-8").read().splitlines():
    module, kind = line.split("\t")
    count += 1
    try:
        # As a file is read: the bytes, so the declaration on the first two
        # lines is honoured. A str source never is.
        path = f"{sys.argv[1]}/enc_pkg/{module}.py"
        with open(path, "rb") as f:
            encoding, _ = tokenize.detect_encoding(f.readline)
        assert encoding == "utf-8", f"read as {encoding}"
        compile(open(path, "rb").read(), path, "exec")
        mod = importlib.import_module("enc_pkg." + module)
        if kind == "codec":
            cls = next(
                (c for _, c in inspect.getmembers(mod, inspect.isclass)
                 if c.__module__ == mod.__name__ and hasattr(c, "decode") and hasattr(c, "encode_to_bytes")),
                None,
            )
            assert cls is not None, f"{module} defines no codec class"
            for hexframe in ("05", "8001"):
                frame = bytes.fromhex(hexframe)
                value = cls.decode(SceCursor(frame))
                assert value.encode_to_bytes() == frame, f"round trip of {hexframe}"
    except BaseException:
        failures.append(f"{kind}\t{module}\t" + traceback.format_exc().strip().splitlines()[-1])
print(f"read {count} generated modules")
for f in failures:
    print("FAIL\t" + f)
"#;
    let out = Command::new("python3")
        .arg("-W")
        .arg("error")
        .arg("-c")
        .arg(runner)
        .arg(proj.path())
        .arg(proj.path().join("manifest.tsv"))
        .env("PYTHONPATH", python_runtime())
        .current_dir(proj.path())
        .output()
        .expect("python3 reads the generated modules");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();

    assert!(
        out.status.success(),
        "the runner itself failed:\n{stderr}\n{stdout}"
    );
    let expected = 2 * dangerous_names().len();
    assert!(
        stdout.contains(&format!("read {expected} generated modules")),
        "the runner did not read every module:\n{stdout}"
    );
    let failures: Vec<&str> = stdout.lines().filter(|l| l.starts_with("FAIL\t")).collect();
    assert!(
        failures.is_empty(),
        "{} generated module(s) could not be read as the file they are:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// A golden is a file somebody imports, not a string to compare. One was
/// committed in a state the interpreter refuses (`codec_zenoh_encoding.py`,
/// `unknown encoding: 68`) and every conformance test passed, because each
/// compares text. The check is the interpreter's own reading of each one, so
/// the next such golden fails here instead of in a consumer.
#[test]
fn every_committed_python_golden_is_read_as_the_utf8_it_is() {
    if !python_present() {
        assert!(
            std::env::var_os("SCE_REQUIRE_ALL_COMPILERS").is_none(),
            "python3 is required (SCE_REQUIRE_ALL_COMPILERS is set) and is absent"
        );
        eprintln!("python3 absent: the committed Python goldens were not read");
        return;
    }
    let script = r#"
import pathlib, sys, tokenize
bad, count = [], 0
for path in sorted(pathlib.Path(sys.argv[1]).glob("*.py")):
    count += 1
    try:
        with open(path, "rb") as f:
            encoding, _ = tokenize.detect_encoding(f.readline)
        if encoding != "utf-8":
            bad.append(f"{path.name}\tread as {encoding}")
    except SyntaxError as e:
        bad.append(f"{path.name}\t{e}")
print(f"read {count} goldens")
for b in bad:
    print("FAIL\t" + b)
"#;
    let out = Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/expected"))
        .output()
        .expect("python3 reads the committed goldens");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        out.status.success(),
        "the reader itself failed:\n{}\n{stdout}",
        String::from_utf8_lossy(&out.stderr)
    );
    let count: usize = stdout
        .lines()
        .find_map(|l| {
            l.strip_prefix("read ")?
                .strip_suffix(" goldens")?
                .parse()
                .ok()
        })
        .unwrap_or(0);
    assert!(
        count > 100,
        "implausibly few Python goldens were read ({count}); the directory moved\n{stdout}"
    );
    let failures: Vec<&str> = stdout.lines().filter(|l| l.starts_with("FAIL\t")).collect();
    assert!(
        failures.is_empty(),
        "{} committed Python golden(s) cannot be read as the file they are:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Nothing is added where the interpreter reads no declaration. Only a module
/// that was unreadable moves, and this needs no interpreter.
#[test]
fn a_module_python_reads_no_declaration_from_is_left_as_it_is() {
    for kind in [Kind::Codec, Kind::Lookup] {
        for name in CONTROLS {
            let (_, source) = generate(kind, name);
            let first = source.lines().next().unwrap_or_default();
            assert!(
                first.starts_with(&format!("# SCE-MAP: {name}:")),
                "{} `{name}`: a module the interpreter reads no declaration from was \
                 changed; its first line is\n{first}",
                kind.label()
            );
            assert!(
                !source.contains(DECLARATION),
                "{} `{name}`: the declaration was added where nothing needed it",
                kind.label()
            );
        }
    }
}
