// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//! tail, a repeat and its count, a TLV chain, an embed), and to an entry of a
//! CBOR map, a property of a content-line component and a parameter of the
//! lines of a repeated one, each an attribute of a class whose body evaluates
//! names of its own. The generated module is run: a frame is decoded and
//! encoded back to the same bytes, the field is read back, and a value made by
//! default is asked what it holds under the name, which is never a method of
//! its class.
//!
//! The names are DERIVED, not listed: a hand-written list of "names to keep
//! away from" is exactly what drifts when a template starts to use one more
//! builtin. The set is `dir(builtins)` joined with every bare identifier and
//! parameter in `tests/forge/expected/*.py` and in the Python the documents
//! of `GENERATED_HERE` generate, so a template that
//! begins to use a name widens the question by itself.
//!
//! A name the parser refuses (`validation/reserved-code-identifier`, a
//! keyword in some target language) is an answer, not a failure: the author is
//! told, in the document, before any code exists. What may not happen is a
//! name that is accepted and then breaks the generated code.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::generator::Language;
use sce_build::{
    compile_forge_file, compile_forge_with_imports, DocumentLabel, ForgeCompileOptions,
};

fn resource_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources")
}

fn expected_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/expected")
}

fn python_runtime() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../backends/python/forge-runtime")
}

/// The documents the names are also read off, as paths from the crate. Their
/// Python is committed nowhere, so each is generated where it is read. The
/// CBOR and content-line ones hold every class body those two printers write:
/// a map's value and the `Protocol` its source satisfies, a component's value,
/// its line records and theirs, a list of values, an enum entry and an enum
/// parameter, and a map that reads a list of records. The positional ones hold
/// a field of an enum's type, alone, gated, and a member of a group's arm,
/// each a default the class body would otherwise make by the enum's module.
const GENERATED_HERE: &[&str] = &[
    "../tests/forge/resources/codec_cbor_map.scxml",
    "../tests/forge/resources/codec_content_line_enums.scxml",
    "../tests/forge/resources/codec_content_line_event.scxml",
    "../tests/forge/resources/codec_content_line_open.scxml",
    "../tests/forge/resources/codec_content_line_records.scxml",
    "../tests/forge/resources/codec_content_line_required_lists.scxml",
    "../tests/forge/conformance/broadcast_log_text/broadcast_log_conformance_text_event.scxml",
    "../tests/forge/conformance/codec_cbor_open_enum/codec_cbor_open_enum.scxml",
    "tests/fixtures/cbor_codec/codec_cbor_record_list.scxml",
    "tests/fixtures/cbor_codec/codec_cbor_record_list_bytes.scxml",
    "tests/fixtures/cbor_codec/codec_cbor_record_list_least.scxml",
    "tests/fixtures/cbor_codec/codec_cbor_signed_enum.scxml",
    "../stdlib/mesh/envelope.scxml",
    "../tests/forge/resources/codec_enum_fields.scxml",
    "../tests/forge/resources/codec_enum_gated.scxml",
    "../tests/forge/resources/codec_group_enum.scxml",
];

/// The Python each of [`GENERATED_HERE`] generates, as `<stem>.py` in a new
/// directory, beside a copy of the document as `<stem>.scxml`, which the
/// derivation reads the names the document declares from.
fn generated_here_modules() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a directory for the generated modules");
    for document in GENERATED_HERE {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(document);
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("a document has a stem")
            .to_string();
        std::fs::copy(&path, dir.path().join(format!("{stem}.scxml"))).expect("copy the document");
        let output = compile_forge_file(
            &path,
            Language::Python,
            &[],
            &ForgeCompileOptions::default(),
        )
        .unwrap_or_else(|e| panic!("{document} generates on Python: {:?}", e.error));
        let (_, source) = output
            .files
            .into_iter()
            .find(|(file, _)| *file == format!("{stem}.py"))
            .unwrap_or_else(|| panic!("{document} generates no `{stem}.py`"));
        std::fs::write(dir.path().join(format!("{stem}.py")), source).expect("write the module");
    }
    dir
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
/// An enum a field is typed by, imported as an enum. A closed one whose first
/// variant, the default of a field of its type, is 1 and not the zero.
const ENUM_SET: &[&str] = &["enum_session_state.scxml"];

/// One construct that binds a local to the value it decodes, with the name
/// under test standing where an author's name stands.
struct Shape {
    id: &'static str,
    /// The root's attributes after `sce:kind="codec"`: the encoding the
    /// document is written in, and what that encoding asks the root to say.
    root: &'static str,
    /// Resource documents it imports, generated alongside it.
    imports: &'static [&'static str],
    /// Whether the document states that its encode can refuse a value
    /// (`sce:encode-may-fail`, docs/adr/0065): a list or a chain can be given
    /// more entries than its bound, and the build holds the statement to that.
    encode_may_fail: bool,
    /// The `<datamodel>` body; `NAME` is replaced by the candidate.
    datamodel: &'static str,
    /// Frames that must decode and encode back to themselves.
    frames: &'static [&'static [u8]],
    /// What the attribute named by the author must hold after the FIRST frame
    /// is decoded, as a Python expression over `{v}` (that attribute). A
    /// round trip alone passes when a field replaces a method of the class that
    /// happens to be unused by it; reading the field back does not. Where the
    /// name is of an element the encode derives, which the value does not hold
    /// (docs/adr/0081 §3), the expression is over `{value}`, the decoded value,
    /// and `{name}`, the name as a Python string.
    check: &'static str,
}

/// The root of a codec of fields at their positions, big-endian.
const POSITIONAL: &str = r#" sce:default-endian="big""#;
/// The root of a codec that writes one CBOR map (SCE_FORGE.md, section 4.6.1).
const CBOR: &str = r#" sce:encoding="cbor" sce:codec-format="2""#;
/// The root of a codec that writes one component of content lines
/// (SCE_FORGE.md, section 4.6.4).
const CONTENT_LINE: &str =
    r#" sce:encoding="content-line" sce:codec-format="2" sce:component="VEVENT""#;

const SHAPES: &[Shape] = &[
    Shape {
        id: "fixed",
        root: POSITIONAL,
        imports: &[],
        encode_may_fail: false,
        datamodel: r#"
    <sce:field id="NAME" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:field id="b" sce:type="uint16" sce:byte="1" sce:bit-size="16"/>"#,
        frames: &[&[1, 0, 2]],
        check: "{v} == 1",
    },
    Shape {
        id: "vle",
        root: POSITIONAL,
        imports: &[],
        encode_may_fail: false,
        datamodel: r#"
    <sce:field id="NAME" sce:type="uint32" sce:byte="0" sce:bit-size="vle"/>
    <sce:field id="z" sce:type="uint8" sce:byte="1" sce:bit-size="8"/>"#,
        frames: &[&[5, 3]],
        check: "{v} == 5",
    },
    Shape {
        id: "bytes_named",
        root: POSITIONAL,
        imports: &[],
        encode_may_fail: false,
        datamodel: r#"
    <sce:field id="n" sce:type="uint64" sce:byte="0" sce:bit-size="vle"/>
    <sce:field id="NAME" sce:type="bytes" sce:byte="1" sce:bit-size="length-ref"
               sce:length-field="n" sce:max-size="16"/>"#,
        frames: &[&[2, 0xAA, 0xBB]],
        check: "{v} == bytes([0xAA, 0xBB])",
    },
    // The length is derived (docs/adr/0081 §3): the value printed from its
    // plan holds no attribute of the author's name, and the bytes it measures
    // read back.
    Shape {
        id: "length_named",
        root: POSITIONAL,
        imports: &[],
        encode_may_fail: false,
        datamodel: r#"
    <sce:field id="NAME" sce:type="uint64" sce:byte="0" sce:bit-size="vle"/>
    <sce:field id="data" sce:type="bytes" sce:byte="1" sce:bit-size="length-ref"
               sce:length-field="NAME" sce:max-size="16"/>"#,
        frames: &[&[2, 0xAA, 0xBB]],
        check: "not hasattr({value}, {name}) and {value}.data == bytes([0xAA, 0xBB])",
    },
    Shape {
        id: "string_named",
        root: POSITIONAL,
        imports: &[],
        encode_may_fail: false,
        datamodel: r#"
    <sce:field id="n" sce:type="uint64" sce:byte="0" sce:bit-size="vle"/>
    <sce:field id="NAME" sce:type="string" sce:byte="1" sce:bit-size="length-ref"
               sce:length-field="n" sce:max-size="16"/>"#,
        frames: &[&[2, 0x68, 0x69]],
        check: "{v} == 'hi'",
    },
    Shape {
        id: "flags_gate",
        root: POSITIONAL,
        imports: &[],
        encode_may_fail: false,
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
        root: POSITIONAL,
        imports: &[],
        encode_may_fail: false,
        datamodel: r#"
    <sce:field id="h" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:field id="NAME" sce:type="bytes" sce:byte="1" sce:bit-size="tail"
               sce:max-size="16"/>"#,
        frames: &[&[1, 2, 3]],
        check: "{v} == bytes([2, 3])",
    },
    Shape {
        id: "repeat_count",
        root: POSITIONAL,
        imports: SLICE_SET,
        encode_may_fail: false,
        datamodel: r#"
    <sce:field id="NAME" sce:type="uint32" sce:byte="0" sce:bit-size="vle"/>
    <sce:repeat id="items" type="codec_chain_has_marker_slice" sce:byte="1"
                count="NAME" max-count="4"/>"#,
        frames: &[&[1, 0, 2, 0xAA, 0xBB], &[0]],
        // The count is derived (docs/adr/0081 §3) where the Python backend
        // prints the codec, and an attribute of the value where it does not.
        check: "len({value}.items) == 1 and getattr({value}, {name}, 1) == 1",
    },
    Shape {
        id: "repeat_named",
        root: POSITIONAL,
        imports: SLICE_SET,
        encode_may_fail: false,
        datamodel: r#"
    <sce:field id="n" sce:type="uint32" sce:byte="0" sce:bit-size="vle"/>
    <sce:repeat id="NAME" type="codec_chain_has_marker_slice" sce:byte="1"
                count="n" max-count="4"/>"#,
        frames: &[&[1, 0, 2, 0xAA, 0xBB], &[0]],
        check: "len({v}) == 1",
    },
    Shape {
        id: "chain_named",
        root: POSITIONAL,
        imports: ENTRY_SET,
        encode_may_fail: false,
        datamodel: r#"
    <sce:tlv-chain id="NAME" type="codec_zenoh_ext_entry" sce:byte="0"
                   max-depth="4"                   terminate-on="entry-flag" entry-flag-name="Z"/>
    <sce:field id="z" sce:type="uint8" sce:byte="1" sce:bit-size="8"/>"#,
        frames: &[&[0x03, 9], &[0x83, 0x04, 9]],
        check: "len({v}) == 1",
    },
    Shape {
        id: "embed_named",
        root: POSITIONAL,
        imports: SLICE_SET,
        encode_may_fail: false,
        datamodel: r#"
    <sce:field id="h" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:embed id="NAME" type="codec_chain_has_marker_slice" sce:byte="1"/>"#,
        frames: &[&[1, 0, 2, 0xAA, 0xBB]],
        check: "{v} is not None",
    },
    // An entry of a CBOR map, declared before another: the value's dataclass
    // holds both and the `Protocol` its source satisfies presents both, each
    // made by a decorator its class body evaluates.
    Shape {
        id: "map_entry",
        root: CBOR,
        imports: &[],
        encode_may_fail: false,
        datamodel: r#"
    <data id="raw" sce:type="bytes" sce:direction="in"/>
    <data id="NAME" sce:type="uint8" sce:key="0" sce:direction="out"/>
    <data id="z" sce:type="uint8" sce:key="1" sce:direction="out"/>"#,
        frames: &[&[0xA2, 0x00, 0x01, 0x01, 0x02]],
        check: "{v} == 1",
    },
    // A property of a content-line component declared before a repeated one,
    // whose list each value holds anew: the default of that list is made in
    // the class body the author's name is bound in.
    Shape {
        id: "line_property",
        root: CONTENT_LINE,
        imports: &[],
        encode_may_fail: true,
        datamodel: r#"
    <data id="raw" sce:type="bytes" sce:direction="in"/>
    <data id="NAME" sce:type="uint8" sce:property="X-A"/>
    <data id="z" sce:type="optional&lt;string&gt;" sce:property="X-Z" sce:repeated="true"
          sce:max-count="2" sce:max-size="8"/>"#,
        frames: &[
            b"BEGIN:VEVENT\r\nX-A:1\r\nX-Z:ab\r\nEND:VEVENT\r\n",
            b"BEGIN:VEVENT\r\nX-A:1\r\nEND:VEVENT\r\n",
        ],
        check: "{v} == 1",
    },
    // A parameter of a repeated property, a member of the record each line is
    // read into beside the line's own value: the record's class body holds
    // both, and both are read back off the first line, and off a record made
    // by default.
    Shape {
        id: "line_record_param",
        root: CONTENT_LINE,
        imports: &[],
        encode_may_fail: true,
        datamodel: r#"
    <data id="raw" sce:type="bytes" sce:direction="in"/>
    <data id="z" sce:type="string" sce:property="X-Z" sce:repeated="true"
          sce:max-count="2" sce:max-size="8"/>
    <data id="NAME" sce:type="optional&lt;string&gt;" sce:property="X-Z" sce:param="P"
          sce:max-size="8"/>"#,
        frames: &[b"BEGIN:VEVENT\r\nX-Z;P=ab:cd\r\nEND:VEVENT\r\n"],
        check: "getattr({value}.z[0], {name}) == 'ab' and {value}.z[0].value == 'cd' \
                and not callable(getattr(type({value}.z[0])(), {name}))",
    },
    // A field of an enum's type declared after the one under test: its default,
    // the enum's first variant, is made through the module the enum is
    // imported as, which a field declared before it and spelled as that module
    // would stand in for in the class body.
    Shape {
        id: "enum_after",
        root: POSITIONAL,
        imports: ENUM_SET,
        encode_may_fail: false,
        datamodel: r#"
    <sce:field id="NAME" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:field id="e" sce:type="enum:enum_session_state" sce:byte="1" sce:bit-size="8"/>"#,
        frames: &[&[1, 1]],
        check: "{v} == 1",
    },
    // The same pair as members of one arm of a gate group (docs/adr/0065 §6
    // R4b), whose class body makes the enum's default as the value's does.
    Shape {
        id: "group_enum",
        root: POSITIONAL,
        imports: ENUM_SET,
        encode_may_fail: false,
        datamodel: r#"
    <sce:flags id="f" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="on" bit="0"/>
    </sce:flags>
    <sce:field id="NAME" sce:type="uint8" sce:byte="1" sce:bit-size="8"
               sce:present-if="f.on"/>
    <sce:field id="e" sce:type="enum:enum_session_state" sce:byte="2" sce:bit-size="8"
               sce:present-if="f.on"/>"#,
        frames: &[&[1, 7, 1], &[0]],
        check: "{value} is not None",
    },
];

impl Shape {
    /// Whether the shape's own datamodel declares `name`, besides the name
    /// under test.
    fn declares(&self, name: &str) -> bool {
        self.datamodel.contains(&format!("id=\"{name}\""))
    }
}

/// Every name that could be in the way, derived. See the module comment.
/// `generated` holds the Python of [`GENERATED_HERE`] ([`generated_here_modules`]).
fn candidate_names(generated: &Path) -> Vec<String> {
    let script = r#"
import ast, builtins, glob, re, sys
names = set(dir(builtins))
for directory in sys.argv[1:]:
    for path in glob.glob(directory + "/*.py"):
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
        .arg(generated)
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
            let kind = if ENUM_SET.contains(f) {
                "enum"
            } else {
                "codec"
            };
            format!("  <sce:import src=\"{f}\" kind=\"{kind}\" as=\"{stem}\"/>\n")
        })
        .collect();
    let declared = if shape.encode_may_fail {
        r#" sce:encode-may-fail="true""#
    } else {
        ""
    };
    let root = shape.root;
    let scxml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec"{declared}{root} name="{doc_name}">
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
            // Or `validation/colliding-code-identifier`: the candidate and one
            // of the shape's own names are one name to some backend (`_n`
            // beside `n` is Go's `N` twice), which is also an answer given in
            // the document before any code exists.
            if message.contains("cannot declare it — rename it")
                || message.contains("so it would declare one name twice")
            {
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
/// committed Python output, and from the Python the documents of
/// [`GENERATED_HERE`] generate, which is committed nowhere, and
/// compared:
///
/// - the methods a codec class, or a dataclass of the codec's module, defines
///   and does not spell from a name its document declares (`decode`, `encode`,
///   …, and the `_decode_at` of a map's and a component's class), any of which
///   an author's field of the same name would replace, or be replaced by;
/// - the names a class body evaluates while it is built — a decorator, a
///   default — outside any function and outside the annotations, which are
///   lazy under `from __future__ import annotations`, and outside a lambda's
///   body or a comprehension past its first iterable, which read the module's
///   names when they run: of every class whose members are the codec's
///   elements, the value's dataclass, each other dataclass (a group's arm, the
///   variant's default arm) and the `Protocol` a source satisfies, since a
///   field of each is spelled as the author named it;
/// - the first parameters of `decode` and `encode`;
/// - that no local the generator names itself begins `f_`, the prefix an
///   author's field becomes in a decoder (`python_field_local`): each local
///   bound under that prefix is `f_` and, snake_cased, a name the codec's
///   document declares. Not an attribute of the class: an element the encode
///   derives is none (docs/adr/0081 §3), and a group's member is one of its
///   arm's class.
///
/// A template edit that adds a method, a class-body reference or an `f_` name
/// makes this fail, and the failure says which list to extend. So does a CBOR
/// map that reads a record by a `@staticmethod` of its class
/// (`codec_cbor.py.jinja2`), a content-line list whose default names `list`
/// (`python_source::content_line::EMPTY_LIST`), and an enum entry or field,
/// alone or in a group's arm, whose default names its module in the class body
/// (`python_source::class_body_default`).
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
import ast, glob, json, os, sys
import xml.etree.ElementTree as ET

SCE = "{http://sce.dev/ext}"
SCXML = "{http://www.w3.org/2005/07/scxml}"
DECLARING = {SCE + t for t in ("field", "flags", "repeat", "tlv-chain", "embed")} | {SCXML + "data"}

def snake(name):
    # filters::to_snake_case, the spelling a Python decoder's local takes.
    name = name.replace(".", "_").replace("-", "_")
    out = []
    for i, ch in enumerate(name):
        if i > 0 and ch.isascii() and ch.isupper() and (name[i - 1].islower() or name[i - 1].isdigit()):
            out.append("_")
        out.append(ch)
    return "".join(out).lower()

def declared(path):
    # Every name the codec's document declares: a field, a flag set and each
    # of its flags, a list, a chain, an embed.
    names = set()
    for node in ET.parse(path).getroot().iter():
        if node.tag in DECLARING and node.get("id"):
            names.add(snake(node.get("id")))
        elif node.tag == SCE + "flag" and node.get("name"):
            names.add(snake(node.get("name")))
    return names

def loads(node):
    # The names node reads in the class body. A lambda's body and a
    # comprehension past its first iterable are scopes of their own, read
    # when they run, and resolve a name in the module, never in the class.
    found = set()

    def walk(n):
        if isinstance(n, ast.Lambda):
            for default in n.args.defaults + [k for k in n.args.kw_defaults if k]:
                walk(default)
            return
        if isinstance(n, (ast.GeneratorExp, ast.ListComp, ast.SetComp, ast.DictComp)):
            walk(n.generators[0].iter)
            return
        if isinstance(n, ast.Name):
            found.add(n.id)
        for child in ast.iter_child_nodes(n):
            walk(child)

    walk(node)
    return found

def is_dataclass(cls):
    # The value, a group's arm, the variant's default arm, a line record.
    return any(isinstance(d, ast.Name) and d.id == "dataclass" for d in cls.decorator_list)

def is_protocol(cls):
    # What a source of the codec, or of a line record, satisfies.
    return any(isinstance(b, ast.Name) and b.id == "Protocol" for b in cls.bases)

def declares_fields(cls):
    # A class whose members are the codec's elements, named as the author
    # named them.
    return is_dataclass(cls) or is_protocol(cls)

def is_codec(cls):
    fns = {f.name for f in cls.body if isinstance(f, ast.FunctionDef)}
    return "decode" in fns and "encode" in fns

def spelled_from(name, names):
    # A method an author's name makes: a flag's accessor and its `set_` twin,
    # a quantity's `<id>_phys` pair.
    stem = name[len("set_"):] if name.startswith("set_") else name
    stem = stem[: -len("_phys")] if stem.endswith("_phys") else stem
    return name in names or stem in names

methods = set()
class_body = set()
first_params = set()
unowned_f_locals = []
codec_classes = 0
generated_here_classes = 0
# The committed output, its documents among the resources; and the Python of
# the documents committed nowhere, generated beside a copy of each.
for py_dir, doc_dir in ((sys.argv[1], sys.argv[2]), (sys.argv[3], sys.argv[3])):
    for path in sorted(glob.glob(py_dir + "/*.py")):
        tree = ast.parse(open(path, encoding="utf-8").read())
        classes = [c for c in tree.body if isinstance(c, ast.ClassDef)]
        # The names a codec's document declares, which a module that holds no
        # codec class has none of.
        stem = os.path.splitext(os.path.basename(path))[0]
        codec_module = any(is_codec(c) for c in classes)
        names = declared(os.path.join(doc_dir, stem + ".scxml")) if codec_module else set()
        for cls in classes:
            fns = {f.name: f for f in cls.body if isinstance(f, ast.FunctionDef)}
            codec = is_codec(cls)
            # Every method the generator gives a class an author's names are
            # attributes of, whichever encoding wrote it: a field of the
            # same name is bound in the same class, and one of the two wins.
            # A `__x__` name is refused as a form; a flag's accessor is the
            # author's own name.
            if codec_module and (codec or is_dataclass(cls)):
                methods |= {
                    f for f in fns
                    if not (f.startswith("__") and f.endswith("__")) and not spelled_from(f, names)
                }
            if codec or declares_fields(cls):
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
            if not codec:
                continue
            codec_classes += 1
            if py_dir == sys.argv[3]:
                generated_here_classes += 1
            for fname in ("decode", "encode"):
                args = fns[fname].args.args
                first_params |= {a.arg for a in args[:2]}
            for node in ast.walk(fns["decode"]):
                if isinstance(node, ast.Name) and isinstance(node.ctx, ast.Store):
                    if node.id.startswith("f_") and node.id[2:] not in names:
                        unowned_f_locals.append(f"{path}:{cls.name}:{node.id}")
print(json.dumps({
    "codec_classes": codec_classes,
    "generated_here_classes": generated_here_classes,
    "methods": sorted(methods),
    "class_body": sorted(n for n in class_body if n[:1].islower()),
    "first_params": sorted(first_params),
    "unowned_f_locals": unowned_f_locals,
}))
"#;
    let generated = generated_here_modules();
    let out = Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(expected_dir())
        .arg(resource_dir())
        .arg(generated.path())
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
        derived["generated_here_classes"].as_u64(),
        Some(GENERATED_HERE.len() as u64),
        "each document generated here generates one codec class the names are read off"
    );
    assert_eq!(
        strings("methods"),
        listed(sce_build::reader_names::PYTHON_CODEC_METHODS),
        "the methods a generated Python codec class, or a dataclass of its module, defines \
         are not the ones reader_names::PYTHON_CODEC_METHODS lists"
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

    let generated = generated_here_modules();
    let names = candidate_names(generated.path());
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
    for set in [ENTRY_SET, SLICE_SET, ENUM_SET] {
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
            // A shape's own ids are not candidates for it: a document cannot
            // declare an id twice, and that is a different refusal.
            if shape.declares(name) {
                continue;
            }
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
        # A value made by default holds a value of each field, never what the
        # class names itself: a field declared before a method of its name
        # takes the method as its default, and a round trip, which sets
        # every field it reads, never shows it.
        made = getattr(cls(), name, None)
        assert not callable(made), f"a value made by default holds {made!r} as {name}"
        for number, hexframe in enumerate(frames.split(",")):
            frame = bytes.fromhex(hexframe)
            value = cls.decode(SceCursor(frame))
            assert value is not None, "decode returned None"
            again = value.encode_to_bytes()
            assert again == frame, f"round trip {again.hex()} != {frame.hex()}"
            if number == 0:
                held = getattr(value, name) if "{v}" in check else value
                spelled = check.replace("{v}", "held").replace("{value}", "value").replace("{name}", repr(name))
                assert eval(spelled), f"{spelled} is false of {held!r}"
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
