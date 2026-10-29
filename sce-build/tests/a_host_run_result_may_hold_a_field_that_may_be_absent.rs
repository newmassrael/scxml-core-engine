// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A host-run `<invoke>`'s typed result may hold a field that may be absent
//! (docs/adr/0055, section 4, and step 10 of "How it lands"): the host completes
//! the invocation with the record's JSON, which spells an absent value `null`, so
//! the record the generated interface hands the host holds the field in the
//! backend's one optional type and writes `null` for an absent value. A request
//! is text, which has no spelling for absence, and refuses such a field (see
//! `a_typed_host_invoke_is_held_to_its_schemas`).
//!
//! Each case writes a statechart whose one invoke completes with a record of a
//! number, a `bool`, a text and a byte string that may be absent beside a field
//! that never is, and asks every backend that emits a statechart to check it and
//! to generate it. The needles are fragments of what each renderer writes
//! (`sce-build/src/forge/host_invoker_interface.rs`), never a whole statement:
//! a formatter wraps a long one.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::codegen_matrix::emitting;
use sce_build::forge::model::ForgeKind;
use tempfile::tempdir;

/// The record the host answers with. The text and the byte string declare their
/// bound, which the C11 buffer they are held in is declared over.
const ANSWER: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       name="answer" sce:kind="event-schema" sce:event-name="answer.record">
  <datamodel>
    <data id="id" sce:type="uint32" sce:direction="in"/>
    <data id="due" sce:type="optional&lt;int64&gt;" sce:direction="in"/>
    <data id="done" sce:type="optional&lt;bool&gt;" sce:direction="in"/>
    <data id="note" sce:type="optional&lt;string&gt;" sce:max-size="16" sce:direction="in"/>
    <data id="blob" sce:type="optional&lt;bytes&gt;" sce:max-size="8" sce:direction="in"/>
  </datamodel>
</scxml>"#;

/// The same record with a text that declares no bound.
const UNBOUNDED_ANSWER: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       name="answer" sce:kind="event-schema" sce:event-name="answer.record">
  <datamodel>
    <data id="id" sce:type="uint32" sce:direction="in"/>
    <data id="note" sce:type="optional&lt;string&gt;" sce:direction="in"/>
  </datamodel>
</scxml>"#;

/// A record holding a 64-bit integer that never is absent beside one that may be.
const WIDE_ANSWER: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       name="answer" sce:kind="event-schema" sce:event-name="answer.record">
  <datamodel>
    <data id="total" sce:type="uint64" sce:direction="in"/>
    <data id="due" sce:type="optional&lt;int64&gt;" sce:direction="in"/>
  </datamodel>
</scxml>"#;

/// A machine of the data model `datamodel` whose one state runs a host invoke
/// completing with the record, named `probe` as the file is. `outcome` counts
/// the completions, which is all the machine does with one.
fn machine(datamodel: &str) -> String {
    let declared = if datamodel == "sce-static" {
        r#"<data id="outcome" sce:type="uint8" expr="0" sce:direction="out"/>"#
    } else {
        r#"<data id="outcome" expr="0"/>"#
    };
    format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="asking" datamodel="{datamodel}" name="probe">
  <sce:import kind="event-schema" src="answer.scxml" as="Answer"/>
  <datamodel>
    {declared}
  </datamodel>
  <state id="asking">
    <invoke type="x-app-host" id="ask" sce:result="Answer"/>
    <transition event="done.invoke.ask" target="finished">
      <assign location="outcome" expr="1"/>
    </transition>
  </state>
  <final id="finished"/>
</scxml>
"#
    )
}

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// What `sce-codegen <subcommand> <args…>` did with the machine `doc` and the
/// schema `schema` beside it: whether it succeeded, everything it said, and each
/// file it wrote, by name.
fn run(
    subcommand: &str,
    language: &str,
    doc: &str,
    schema: &str,
) -> (bool, String, Vec<(String, String)>) {
    let dir = tempdir().expect("tempdir");
    std::fs::write(dir.path().join("answer.scxml"), schema).expect("write schema");
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, doc).expect("write probe");
    let out_dir = tempdir().expect("tempdir");
    let mut command = Command::new(sce_codegen_bin());
    command
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .arg(subcommand)
        .arg(&path)
        .args(["-l", language])
        .args(["--host-invoker", "x-app-host"]);
    if subcommand == "generate" {
        command.arg("-o").arg(out_dir.path());
    }
    let out = command.output().expect("invoke sce-codegen");
    let mut written = Vec::new();
    for entry in std::fs::read_dir(out_dir.path()).expect("the output directory") {
        let path = entry.expect("an entry").path();
        if path.is_file() {
            let name = path
                .file_name()
                .expect("a file name")
                .to_string_lossy()
                .into_owned();
            written.push((name, std::fs::read_to_string(&path).unwrap_or_default()));
        }
    }
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
        written,
    )
}

/// Every file `language` generated for the machine of `datamodel`.
fn generated(language: &str, datamodel: &str) -> Vec<(String, String)> {
    let (ok, said, written) = run("generate", language, &machine(datamodel), ANSWER);
    assert!(
        ok,
        "{language} generates a result whose fields may be absent ({datamodel}):\n{said}"
    );
    written
}

/// The one file of `written` whose name ends with `suffix`.
fn file<'w>(written: &'w [(String, String)], suffix: &str) -> &'w str {
    written
        .iter()
        .find(|(name, _)| name.ends_with(suffix))
        .map(|(_, text)| text.as_str())
        .unwrap_or_else(|| panic!("a file named *{suffix} among {:?}", names(written)))
}

fn names(written: &[(String, String)]) -> Vec<&str> {
    written.iter().map(|(name, _)| name.as_str()).collect()
}

#[test]
fn every_backend_checks_a_result_whose_fields_may_be_absent() {
    for datamodel in ["ecmascript", "sce-static"] {
        for language in emitting(ForgeKind::Statechart).map(|language| language.canonical_name()) {
            let (ok, said, _) = run("check", language, &machine(datamodel), ANSWER);
            assert!(
                ok,
                "{language} refused a result whose fields may be absent ({datamodel}): {said}"
            );
        }
    }
}

#[test]
fn rust_holds_an_absent_field_as_an_option_and_writes_it_as_null() {
    for datamodel in ["ecmascript", "sce-static"] {
        let written = generated("rust", datamodel);
        let text = file(&written, "_sm.rs");
        for needle in [
            "pub due: Option<i64>",
            "pub done: Option<bool>",
            "pub note: Option<::sce_rust_runtime::SceString>",
            "pub blob: Option<::sce_rust_runtime::SceBytes<8>>",
            // An absent value is `null` and a present one what the field is
            // written as, the key written either way.
            "optional_json(",
            "pub id: u32",
        ] {
            assert!(
                text.contains(needle),
                "the Rust record holds `{needle}` ({datamodel}):\n{text}"
            );
        }
    }
}

#[test]
fn kotlin_holds_an_absent_field_as_a_nullable_and_writes_it_as_null() {
    let written = generated("kotlin", "ecmascript");
    let text = file(&written, "Sm.kt");
    for needle in [
        "val due: Long?",
        "val done: Boolean?",
        "val note: String?",
        "val blob: ByteArray?",
        "val id: UInt,",
        // The encoder writes a `null` as `null`.
        "\"due\" to due",
    ] {
        assert!(
            text.contains(needle),
            "the Kotlin record holds `{needle}`:\n{text}"
        );
    }
}

#[test]
fn python_holds_an_absent_field_as_an_optional_and_writes_it_as_null() {
    let written = generated("python", "ecmascript");
    let text = file(&written, "_sm.py");
    for needle in [
        "due: Optional[int]",
        "done: Optional[bool]",
        "note: Optional[str]",
        "blob: Optional[bytes]",
        "id: int\n",
        // A `None` is what `json.dumps` writes as `null`; a byte string is
        // written as its Latin-1 text, and a 64-bit integer as the text of its
        // digits, only when it is present.
        "\"due\": None if self.due is None else str(self.due)",
        "None if self.blob is None else self.blob.decode(\"latin-1\")",
    ] {
        assert!(
            text.contains(needle),
            "the Python record holds `{needle}`:\n{text}"
        );
    }
}

#[test]
fn cpp_holds_an_absent_field_as_a_std_optional_and_writes_it_as_null() {
    let written = generated("cpp", "ecmascript");
    let text = file(&written, "_sm.h");
    for needle in [
        "std::optional<int64_t> due{}",
        "std::optional<bool> done{}",
        "std::optional<std::string> note{}",
        "std::optional<std::vector<uint8_t>> blob{}",
        "due.has_value()",
        "absent(\"due\")",
        "absent(\"blob\")",
    ] {
        assert!(
            text.contains(needle),
            "the C++ record holds `{needle}`:\n{text}"
        );
    }
}

#[test]
fn go_holds_an_absent_field_as_an_optional_of_and_imports_its_package() {
    for datamodel in ["ecmascript", "sce-static"] {
        let written = generated("go", datamodel);
        let text = file(&written, "_sm.go");
        for needle in [
            "optional.Of[int64]",
            "optional.Of[bool]",
            "optional.Of[string]",
            "optional.Of[[]byte]",
            // An absent value is `nil`, which `PayloadJSON` writes as `null`.
            "optional.Wire(r.Due",
            "optional.Wire(r.Blob",
            "sce.BytesAsPayloadText(v)",
            // A machine of any data model names the package, so it imports it.
            "sce-forge-runtime/optional\"",
        ] {
            assert!(
                text.contains(needle),
                "the Go record holds `{needle}` ({datamodel}):\n{text}"
            );
        }
    }
}

#[test]
fn c11_holds_an_absent_field_in_the_struct_of_its_element_and_includes_it() {
    for datamodel in ["ecmascript", "sce-static"] {
        let written = generated("c11", datamodel);
        let header = file(&written, "_sm.h");
        for needle in [
            "#include <sce/forge/optional.h>",
            "sce_optional_int64_t due;",
            "sce_optional_bool_t done;",
            "sce_optional_string_16_t note;",
            "sce_optional_bytes_8_t blob;",
            // The buffers the text and the byte string are held in, and the
            // optional values over them, are declared where the record is.
            "SCE_OPTIONAL_DECLARE_STRING(16)",
            "SCE_OPTIONAL_DECLARE_BYTES(8)",
            "uint32_t id;",
        ] {
            assert!(
                header.contains(needle),
                "the C11 header holds `{needle}` ({datamodel}):\n{header}"
            );
        }
        let source = file(&written, "_sm.c");
        for needle in [
            "#include <sce/forge/wire.h>",
            // An absent value is `null`, written through the wire writer, whose
            // key is written either way.
            "sce_forge_wire_null()",
        ] {
            assert!(
                source.contains(needle),
                "the C11 source holds `{needle}` ({datamodel}):\n{source}"
            );
        }
    }
}

/// A 64-bit integer crosses as the text of its digits (ARCHITECTURE.md, "JSON
/// Number Text"), in a record the host completes an invoke with as in every other
/// payload: a reader that takes a JSON number through a double carries one past
/// 2^53 - 1 with its low bits wrong, and every engine refuses such a number. Each
/// backend's record writer spells a present `uint64` and `int64` so, and a
/// present optional one too; the Kotlin writer hands the integer to the encoder
/// that quotes it (`EventPayload.literal`), and its runtime test pins the text.
#[test]
fn every_backend_writes_a_64_bit_result_field_as_the_text_of_its_digits() {
    let wide = |language: &str, datamodel: &str| {
        let (ok, said, written) = run("generate", language, &machine(datamodel), WIDE_ANSWER);
        assert!(
            ok,
            "{language} generates a 64-bit result field ({datamodel}):\n{said}"
        );
        written
    };
    let holds = |text: &str, needles: &[&str], what: &str| {
        for needle in needles {
            assert!(text.contains(needle), "{what} holds `{needle}`:\n{text}");
        }
    };
    for datamodel in ["ecmascript", "sce-static"] {
        let written = wide("rust", datamodel);
        holds(
            file(&written, "_sm.rs"),
            &[
                "\\\"total\\\":\\\"{}\\\"",
                "|sce_present| format!(\"\\\"{}\\\"\", sce_present)",
            ],
            &format!("the Rust record ({datamodel})"),
        );
        let written = wide("go", datamodel);
        holds(
            file(&written, "_sm.go"),
            &[
                "sce.Uint64PayloadText(r.Total)",
                "func(v int64) any { return sce.Int64PayloadText(v) }",
            ],
            &format!("the Go record ({datamodel})"),
        );
        let written = wide("c11", datamodel);
        holds(
            file(&written, "_sm.c"),
            // A field that is never absent is written through the format of the
            // record, an optional one through the wire writer.
            &[
                "\\\"total\\\":\\\"%llu\\\"",
                "sce_forge_wire_wide_int(result->due.value)",
            ],
            &format!("the C11 record ({datamodel})"),
        );
    }
    let written = wide("python", "ecmascript");
    holds(
        file(&written, "_sm.py"),
        &[
            "\"total\": str(self.total)",
            "\"due\": None if self.due is None else str(self.due)",
        ],
        "the Python record",
    );
    let written = wide("cpp", "ecmascript");
    holds(
        file(&written, "_sm.h"),
        &[
            "field(\"total\", ::std::to_string(total))",
            "field(\"due\", ::std::to_string(*due))",
        ],
        "the C++ record",
    );
    let written = wide("kotlin", "ecmascript");
    holds(
        file(&written, "Sm.kt"),
        &["\"total\" to total", "\"due\" to due"],
        "the Kotlin record",
    );
}

/// A text with no `sce:max-size` has no buffer for the C11 struct that holds an
/// optional one, so C11 refuses it by name, as it refuses it in a payload; every
/// other backend holds it.
#[test]
fn c11_refuses_an_optional_text_that_declares_no_bound_and_the_others_hold_it() {
    let (ok, said, _) = run("generate", "c11", &machine("ecmascript"), UNBOUNDED_ANSWER);
    assert!(
        !ok,
        "C11 has no buffer for an unbounded optional text:\n{said}"
    );
    assert!(
        said.contains("generate/unsupported-feature")
            && said.contains("an optional value has no c11 lowering yet")
            && said.contains("no sce:max-size"),
        "refused by name:\n{said}"
    );
    for language in ["rust", "cpp", "kotlin", "go", "python"] {
        let (ok, said, _) = run(
            "generate",
            language,
            &machine("ecmascript"),
            UNBOUNDED_ANSWER,
        );
        assert!(ok, "{language} holds an unbounded optional text:\n{said}");
    }
}
