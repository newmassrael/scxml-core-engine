// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A name an author gives a codec field must not decide what the generated Go
//! does.
//!
//! The Python generator broke on this once: an author's field `bytes` shadowed
//! the builtin the decoder calls in the same function, and none of it was
//! refused, all of it was generated, and none of it was visible to a gate that
//! only compiles the output (`a_python_codec_keeps_an_authors_names_apart_from_its_own`).
//! Go writes a field Pascal and keeps its own locals lower case, which is why
//! the same defect was not expected here and was not measured. What Go does
//! share with the other backends is one namespace per struct: a field and a
//! method of one name are one name, and every codec struct carries `Encode`,
//! `EncodeToBytes` and the accessor and setter of each flag. This asks the
//! generator, rather than assuming, whether an accepted name can break any of
//! it.
//!
//! # What is measured
//!
//! Every name that could be in the way is given to a field of each shape that
//! binds a name (a fixed field, a VLE, both roles of a length-ref pair, a
//! string, a flags carrier a later field is gated on, a tail, a repeat and its
//! count, a TLV chain, an embed). Each generated package is built; the ones
//! that build are run: a frame is decoded and encoded back to the same bytes,
//! and the field is read back through reflection, because a round trip alone
//! passes when a field replaces something the round trip does not use.
//!
//! The names are DERIVED, not listed. A hand-written list of "names to keep
//! away from" is what drifts when a template starts to use one more name. The
//! set is every identifier that appears in at least three committed Go outputs
//! (a name only one document uses is that document's own), each in the spelling
//! it has there and in the snake_case an author would write for it, joined with
//! Go's universe scope and its keywords, which the language fixes.
//!
//! A name the parser refuses (`validation/reserved-code-identifier`, or one that
//! folds into another name the shape declares) is an answer, not a failure: the
//! author is told, in the document, before any code exists. What may not happen
//! is a name that is accepted and then breaks the generated code.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::generator::Language;
use sce_build::{compile_forge_with_imports, DocumentLabel, ForgeCompileOptions};

const GO_MODULE_PREFIX: &str = "example.com/sce-forge";

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

/// The documents a shape imports, in dependency order.
const ENTRY_SET: &[&str] = &[
    "codec_zenoh_ext_unit.scxml",
    "codec_zenoh_ext_zint.scxml",
    "codec_zenoh_ext_zbuf.scxml",
    "codec_zenoh_ext_entry.scxml",
];
const SLICE_SET: &[&str] = &["codec_chain_has_marker_slice.scxml"];

/// One construct that binds a name, with the name under test standing where an
/// author's name stands.
struct Shape {
    id: &'static str,
    /// Resource documents it imports, generated alongside it.
    imports: &'static [&'static str],
    /// The `<datamodel>` body; `NAME` is replaced by the candidate.
    datamodel: &'static str,
    /// Frames that must decode and encode back to themselves.
    frames: &'static [&'static [u8]],
    /// Where the author's field sits among the struct's fields, in declaration
    /// order, so it is read back without restating how Go spells it.
    field_index: usize,
    /// What the field must hold after the FIRST frame is decoded, as a Go
    /// boolean over `f`, the field as a `reflect.Value`.
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
        field_index: 0,
        check: "f.Uint() == 1",
    },
    Shape {
        id: "vle",
        imports: &[],
        datamodel: r#"
    <sce:field id="NAME" sce:type="uint32" sce:byte="0" sce:bit-size="vle"/>
    <sce:field id="z" sce:type="uint8" sce:byte="1" sce:bit-size="8"/>"#,
        frames: &[&[5, 3]],
        field_index: 0,
        check: "f.Uint() == 5",
    },
    Shape {
        id: "bytes_named",
        imports: &[],
        datamodel: r#"
    <sce:field id="n" sce:type="uint64" sce:byte="0" sce:bit-size="vle"/>
    <sce:field id="NAME" sce:type="bytes" sce:byte="1" sce:bit-size="length-ref"
               sce:length-field="n" sce:max-size="16"/>"#,
        frames: &[&[2, 0xAA, 0xBB]],
        field_index: 1,
        check: "bytes.Equal(f.Bytes(), []byte{0xAA, 0xBB})",
    },
    Shape {
        id: "length_named",
        imports: &[],
        datamodel: r#"
    <sce:field id="NAME" sce:type="uint64" sce:byte="0" sce:bit-size="vle"/>
    <sce:field id="data" sce:type="bytes" sce:byte="1" sce:bit-size="length-ref"
               sce:length-field="NAME" sce:max-size="16"/>"#,
        frames: &[&[2, 0xAA, 0xBB]],
        field_index: 0,
        check: "f.Uint() == 2",
    },
    Shape {
        id: "string_named",
        imports: &[],
        datamodel: r#"
    <sce:field id="n" sce:type="uint64" sce:byte="0" sce:bit-size="vle"/>
    <sce:field id="NAME" sce:type="string" sce:byte="1" sce:bit-size="length-ref"
               sce:length-field="n" sce:max-size="16"/>"#,
        frames: &[&[2, 0x68, 0x69]],
        field_index: 1,
        check: "f.String() == \"hi\"",
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
        field_index: 0,
        check: "f.Uint() == 1",
    },
    Shape {
        id: "tail",
        imports: &[],
        datamodel: r#"
    <sce:field id="h" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:field id="NAME" sce:type="bytes" sce:byte="1" sce:bit-size="tail"
               sce:max-size="16"/>"#,
        frames: &[&[1, 2, 3]],
        field_index: 1,
        check: "bytes.Equal(f.Bytes(), []byte{2, 3})",
    },
    Shape {
        id: "repeat_count",
        imports: SLICE_SET,
        datamodel: r#"
    <sce:field id="NAME" sce:type="uint32" sce:byte="0" sce:bit-size="vle"/>
    <sce:repeat id="items" type="codec_chain_has_marker_slice" sce:byte="1"
                count="NAME" max-count="4"/>"#,
        frames: &[&[1, 0, 2, 0xAA, 0xBB], &[0]],
        field_index: 0,
        check: "f.Uint() == 1",
    },
    Shape {
        id: "repeat_named",
        imports: SLICE_SET,
        datamodel: r#"
    <sce:field id="n" sce:type="uint32" sce:byte="0" sce:bit-size="vle"/>
    <sce:repeat id="NAME" type="codec_chain_has_marker_slice" sce:byte="1"
                count="n" max-count="4"/>"#,
        frames: &[&[1, 0, 2, 0xAA, 0xBB], &[0]],
        field_index: 1,
        check: "f.Len() == 1",
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
        field_index: 0,
        check: "f.Len() == 1",
    },
    Shape {
        id: "embed_named",
        imports: SLICE_SET,
        datamodel: r#"
    <sce:field id="h" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:embed id="NAME" type="codec_chain_has_marker_slice" sce:byte="1"/>"#,
        frames: &[&[1, 0, 2, 0xAA, 0xBB]],
        field_index: 1,
        check: "f.Kind() == reflect.Struct",
    },
];

/// Go's predeclared identifiers: the universe scope the language fixes.
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
];

/// Go's keywords, which the language fixes.
const KEYWORDS: &[&str] = &[
    "break",
    "case",
    "chan",
    "const",
    "continue",
    "default",
    "defer",
    "else",
    "fallthrough",
    "for",
    "func",
    "go",
    "goto",
    "if",
    "import",
    "interface",
    "map",
    "package",
    "range",
    "return",
    "select",
    "struct",
    "switch",
    "type",
    "var",
];

/// How many committed Go outputs a name has to appear in to be a name of the
/// generator's and not of one document.
const SHARED_BY_AT_LEAST: usize = 3;

/// The identifiers of Go source, with comments and literals left out: a word in
/// a comment is prose, and a word in a string is data.
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

/// `EncodeToBytes` as an author would write it: `encode_to_bytes`.
fn snake(token: &str) -> String {
    let chars: Vec<char> = token.chars().collect();
    let mut out = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 && (chars[i - 1].is_ascii_lowercase() || chars[i - 1].is_ascii_digit()) {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// Every name that could be in the way, derived. See the module comment.
fn candidate_names() -> Vec<String> {
    let mut files_with: BTreeMap<String, usize> = BTreeMap::new();
    let mut outputs = 0usize;
    let mut entries: Vec<PathBuf> = std::fs::read_dir(expected_dir())
        .expect("read the committed outputs")
        .map(|e| e.expect("directory entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "go"))
        .collect();
    entries.sort();
    for path in entries {
        outputs += 1;
        let source = std::fs::read_to_string(&path).expect("read a committed Go output");
        for name in identifiers_of(&source) {
            *files_with.entry(name).or_default() += 1;
        }
    }
    assert!(
        outputs > 100,
        "implausibly few committed Go outputs ({outputs}); the derivation broke"
    );
    let mut names: BTreeSet<String> = BTreeSet::new();
    for (token, count) in &files_with {
        if *count >= SHARED_BY_AT_LEAST && token != "_" {
            names.insert(token.clone());
            names.insert(snake(token));
        }
    }
    names.extend(UNIVERSE.iter().map(|s| s.to_string()));
    names.extend(KEYWORDS.iter().map(|s| s.to_string()));
    // A shape's own fixed ids are not candidates: a field cannot be declared
    // twice, and that is a different refusal.
    let ids_the_shapes_use: BTreeSet<&str> = ["n", "b", "z", "x", "h", "data", "items"]
        .into_iter()
        .collect();
    names
        .into_iter()
        .filter(|n| !ids_the_shapes_use.contains(n.as_str()) && n != "_")
        .collect()
}

/// A generated Go file: the package directory it belongs in, and its source.
struct GoFile {
    package: String,
    source: String,
}

enum Outcome {
    /// The parser refused the name, in the document.
    Refused,
    /// Generated: the files of the document, and its Go identifiers.
    Generated {
        files: Vec<GoFile>,
        decode: String,
        type_name: String,
    },
}

fn is_refusal(message: &str) -> bool {
    // `validation/reserved-code-identifier`, whichever the reason;
    // `validation/colliding-code-identifier`: the candidate and a name the
    // shape declares are one name to some backend; or a field named as an
    // import is (`codec_zenoh_ext_entry` is an alias the document declares).
    // Each is an answer given in the document before any code exists.
    message.contains("cannot declare it — rename it")
        || message.contains("so it would declare one name twice")
        || message.contains("duplicate name (import alias and field)")
}

fn package_of(file_name: &str) -> String {
    Path::new(file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(file_name)
        .to_string()
}

/// What a generated document is called: by position and never by the
/// candidate. The position is written `c<n>` and not as a bare number because
/// Go reads a file name that ends in `_<GOARCH>` as a build constraint, and a
/// document numbered 386 was `g_<shape>_386.go`, a file no build of ours
/// includes.
fn document_name(shape: &Shape, index: usize) -> String {
    format!("g_{}_c{}", shape.id, index)
}

/// Generate one document, naming it by position and never by the candidate.
fn generate(shape: &Shape, index: usize, name: &str) -> Result<Outcome, String> {
    let doc_name = document_name(shape, index);
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
        Language::Go,
        &resource_dir(),
        &options(),
    ) {
        Ok(output) => {
            let files: Vec<GoFile> = output
                .files
                .into_iter()
                .filter(|(file, _)| file.ends_with(".go"))
                .map(|(file, source)| GoFile {
                    package: package_of(&file),
                    source,
                })
                .collect();
            let own = files
                .iter()
                .find(|f| f.package == doc_name)
                .ok_or_else(|| format!("{doc_name}: no Go file for the document itself"))?;
            let decode = own
                .source
                .lines()
                .find_map(|l| {
                    l.strip_prefix("func Decode")
                        .and_then(|rest| rest.split('(').next())
                        .map(|n| format!("Decode{n}"))
                })
                .ok_or_else(|| format!("{doc_name}: no Decode function generated"))?;
            let type_name = decode.trim_start_matches("Decode").to_string();
            Ok(Outcome::Generated {
                files,
                decode,
                type_name,
            })
        }
        Err(e) => {
            let message = e.error.to_string();
            if is_refusal(&message) {
                Ok(Outcome::Refused)
            } else {
                Err(format!("{doc_name}: {message}"))
            }
        }
    }
}

/// The test program of one shape: every module that built, decoded, encoded
/// back and read through reflection.
fn runner_source(shape: &Shape, modules: &[(String, String, String, String)]) -> String {
    let mut imports = String::new();
    let mut cases = String::new();
    for (i, (package, name, decode, _type_name)) in modules.iter().enumerate() {
        imports.push_str(&format!("\tm{i} \"{GO_MODULE_PREFIX}/{package}\"\n"));
        cases.push_str(&format!(
            "\t{{\"{name}\", func(frame []byte) (interface{{}}, []byte, error) {{\n\
             \t\tc := codec.NewSceCursor(frame)\n\
             \t\tv, err := m{i}.{decode}(&c)\n\
             \t\tif err != nil {{\n\t\t\treturn nil, nil, err\n\t\t}}\n\
             \t\treturn v, v.EncodeToBytes(), nil\n\
             \t}}}},\n"
        ));
    }
    let frames: String = shape
        .frames
        .iter()
        .map(|f| {
            let bytes: Vec<String> = f.iter().map(|b| format!("0x{b:02X}")).collect();
            format!("\t{{{}}},\n", bytes.join(", "))
        })
        .collect();
    format!(
        r#"package run_{id}

import (
	"bytes"
	"fmt"
	"reflect"
	"testing"

	"github.com/newmassrael/sce-forge-runtime/codec"
{imports})

var _ = bytes.Equal
var _ = reflect.Struct

type decodeCase struct {{
	name   string
	decode func(frame []byte) (interface{{}}, []byte, error)
}}

var cases = []decodeCase{{
{cases}}}

var frames = [][]byte{{
{frames}}}

func holds(f reflect.Value) bool {{
	return {check}
}}

func try(c decodeCase, frame []byte) (v interface{{}}, again []byte, err error) {{
	defer func() {{
		if r := recover(); r != nil {{
			err = fmt.Errorf("panic: %v", r)
		}}
	}}()
	return c.decode(frame)
}}

func TestNames(t *testing.T) {{
	for _, c := range cases {{
		for i, frame := range frames {{
			v, again, err := try(c, frame)
			if err != nil {{
				t.Errorf("ORACLE\t{id}\t%s\tframe %d: %v", c.name, i, err)
				break
			}}
			if !bytes.Equal(again, frame) {{
				t.Errorf("ORACLE\t{id}\t%s\tround trip % x != % x", c.name, again, frame)
				break
			}}
			if i == 0 {{
				f := reflect.ValueOf(v).Elem().Field({index})
				if !holds(f) {{
					t.Errorf("ORACLE\t{id}\t%s\tthe field reads back as %v", c.name, f)
				}}
			}}
		}}
	}}
	fmt.Printf("ORACLE-RAN\t{id}\t%d\n", len(cases))
}}
"#,
        id = shape.id,
        imports = imports,
        cases = cases,
        frames = frames,
        check = shape.check,
        index = shape.field_index,
    )
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

#[test]
fn an_authors_field_name_never_decides_what_the_generated_go_does() {
    let Some(go) = sce_build::toolchain::require_or_skip("go", "the Go name oracle") else {
        return;
    };

    let names = candidate_names();
    assert!(
        names.len() > 150,
        "the derived candidate set is implausibly small ({}); the derivation broke",
        names.len()
    );

    let proj = std::env::temp_dir().join(format!("sce_go_names_{}", std::process::id()));
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

    let write_package = |file: &GoFile| {
        let dir = proj.join(&file.package);
        std::fs::create_dir_all(&dir).expect("mkdir package");
        std::fs::write(dir.join(format!("{}.go", file.package)), &file.source)
            .expect("write package");
    };

    // The imported documents, generated once.
    let mut written: BTreeSet<String> = BTreeSet::new();
    for set in [ENTRY_SET, SLICE_SET] {
        for file in set {
            let src = std::fs::read_to_string(resource_dir().join(file)).expect("read import");
            let stem = file.trim_end_matches(".scxml");
            let output = compile_forge_with_imports(
                &src,
                DocumentLabel::symmetric(stem),
                Language::Go,
                &resource_dir(),
                &options(),
            )
            .unwrap_or_else(|e| panic!("generating {file}: {:?}", e.error));
            for (module, source) in output.files {
                if module.ends_with(".go") {
                    let go_file = GoFile {
                        package: package_of(&module),
                        source,
                    };
                    if written.insert(go_file.package.clone()) {
                        write_package(&go_file);
                    }
                }
            }
        }
    }

    // (package, name, decode function, type) per module generated, by shape.
    let mut generated: BTreeMap<&str, Vec<(String, String, String, String)>> = BTreeMap::new();
    let mut refused: BTreeSet<String> = BTreeSet::new();
    let mut generation_failures: Vec<String> = Vec::new();
    for shape in SHAPES {
        for (index, name) in names.iter().enumerate() {
            match generate(shape, index, name) {
                Ok(Outcome::Refused) => {
                    refused.insert(name.clone());
                }
                Ok(Outcome::Generated {
                    files,
                    decode,
                    type_name,
                }) => {
                    for file in &files {
                        if !written.contains(&file.package) {
                            write_package(file);
                        }
                    }
                    generated.entry(shape.id).or_default().push((
                        document_name(shape, index),
                        name.clone(),
                        decode,
                        type_name,
                    ));
                }
                Err(e) => generation_failures.push(e),
            }
        }
    }

    // Build every generated package. `go build` keeps going past a package
    // that does not compile, and says which.
    let build = Command::new(&go)
        .args(["build", "./..."])
        .current_dir(&proj)
        .output()
        .expect("go builds the generated packages");
    let build_stderr = String::from_utf8_lossy(&build.stderr).to_string();
    let broken = failed_packages(&build_stderr);

    let mut failures: Vec<String> = Vec::new();
    for shape in SHAPES {
        for (package, name, _, _) in generated.get(shape.id).map(Vec::as_slice).unwrap_or(&[]) {
            if let Some(why) = broken.get(package) {
                failures.push(format!("{}\t{name}\tdoes not compile: {why}", shape.id));
            }
        }
    }
    assert!(
        build.status.success() || !broken.is_empty(),
        "go build failed without naming a package:\n{build_stderr}"
    );

    // Run what built, a program per shape.
    let mut ran: BTreeMap<&str, usize> = BTreeMap::new();
    for shape in SHAPES {
        let modules: Vec<(String, String, String, String)> = generated
            .get(shape.id)
            .map(|all| {
                all.iter()
                    .filter(|(package, ..)| !broken.contains_key(package))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        if modules.is_empty() {
            continue;
        }
        let dir = proj.join(format!("run_{}", shape.id));
        std::fs::create_dir_all(&dir).expect("mkdir runner");
        std::fs::write(dir.join("run_test.go"), runner_source(shape, &modules))
            .expect("write runner");
    }
    let run = Command::new(&go)
        .args(["test", "-v", "-count=1", "./run_..."])
        .current_dir(&proj)
        .output()
        .expect("go runs the generated modules");
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    let stderr = String::from_utf8_lossy(&run.stderr).to_string();
    for line in stdout.lines().chain(stderr.lines()) {
        let line = line.trim();
        if let Some(rest) = line.split("ORACLE\t").nth(1) {
            failures.push(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("ORACLE-RAN\t") {
            let mut parts = rest.split('\t');
            if let (Some(id), Some(count)) = (parts.next(), parts.next()) {
                if let Some(shape) = SHAPES.iter().find(|s| s.id == id) {
                    ran.insert(shape.id, count.parse().unwrap_or(0));
                }
            }
        }
    }
    let _ = std::fs::remove_dir_all(&proj);

    assert!(
        generation_failures.is_empty(),
        "a document built for this oracle was refused for a reason other than a reserved word:\n{}",
        generation_failures.join("\n")
    );
    assert_eq!(
        ran.len(),
        SHAPES.len(),
        "a shape's program did not run ({ran:?}); go test said:\n{stdout}\n{stderr}"
    );
    let programs: usize = ran.values().sum();
    // The count is the evidence that it ran: a green that ran nothing reads the
    // same as one that ran everything.
    eprintln!(
        "go name oracle: {} candidate names, {} refused by the parser, {} generated modules \
         built and run, {} did not compile",
        names.len(),
        refused.len(),
        programs,
        failures
            .iter()
            .filter(|f| f.contains("does not compile"))
            .count()
    );
    eprintln!(
        "go name oracle: refused: {}",
        refused.iter().cloned().collect::<Vec<_>>().join(" ")
    );
    assert!(
        programs > SHAPES.len() * 100,
        "implausibly few generated modules ran ({programs}); most names were refused or the \
         build lost them"
    );
    assert!(
        failures.is_empty(),
        "{} accepted field name(s) break the generated Go ({} candidate names, {} refused by the \
         parser, {} programs ran):\n{}",
        failures.len(),
        names.len(),
        refused.len(),
        programs,
        failures.join("\n")
    );
}
