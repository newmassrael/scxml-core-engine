// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! `<sce:render>` (docs/adr/0087, section 3.1, item 6; docs/SCE_ACCEPTED_SUBSET.md
//! §2.15, "A text rendered from a template"): each rule of the statement refused
//! by its own code, and a render that keeps them lowered by every backend, the
//! Interpreter's included, through the template's unit and the one write of a
//! string.

use std::fs;
use std::path::{Path, PathBuf};

use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, Located};
use sce_build::generator::Language;
use sce_build::{compile_scxml_lang_typed, find_template_dir_for};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/static_datamodel")
}

/// The committed template and the enum it imports, written beside a document
/// whose datamodel is `datamodel` and whose one transition renders with `render`.
fn document(datamodel: &str, render: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="probe">
  <sce:import kind="enum" src="enum_view_mode.scxml" as="ViewMode"/>
  <sce:import kind="text-template" src="text_notice.scxml" as="Notice"/>
  <datamodel>
{datamodel}
  </datamodel>
  <state id="idle">
    <transition event="go" type="internal">
{render}
    </transition>
  </state>
</scxml>
"#
    )
}

const VARIABLES: &str = r#"    <data id="note" sce:type="string[95]" expr="''"/>
    <data id="who" sce:type="string[8]" expr="'ann'"/>
    <data id="wide" sce:type="string[9]" expr="'ann'"/>
    <data id="count" sce:type="int16" expr="0"/>
    <data id="big" sce:type="int32" expr="0"/>
    <data id="view" sce:type="enum:ViewMode" expr="ViewMode.month"/>
    <data id="said" sce:type="string[16]" expr="'hi'"/>"#;

/// A render of the template with `sets` as its `<sce:set>` children, into
/// `location`.
fn render(location: &str, sets: &[(&str, &str)]) -> String {
    let sets: String = sets
        .iter()
        .map(|(name, expr)| format!("        <sce:set name=\"{name}\" expr=\"{expr}\"/>\n"))
        .collect();
    format!(
        "      <sce:render template=\"Notice\" location=\"{location}\">\n{sets}      </sce:render>"
    )
}

const EVERY_HOLE: &[(&str, &str)] = &[
    ("who", "who"),
    ("count", "count"),
    ("view", "view"),
    ("said", "said"),
];

/// The document written beside the template and its enum, compiled for `lang`:
/// every generated file as one text.
fn compile(text: &str, lang: Language) -> Result<String, Located<ForgeError>> {
    let dir = tempfile::TempDir::new().expect("tempdir");
    for sibling in ["text_notice.scxml", "enum_view_mode.scxml"] {
        fs::copy(fixtures().join(sibling), dir.path().join(sibling)).expect("copy sibling");
    }
    let path = dir.path().join("probe.scxml");
    fs::write(&path, text).expect("write document");
    compile_scxml_lang_typed(path.to_str().unwrap(), &find_template_dir_for(lang), lang).map(
        |out| {
            out.files
                .into_iter()
                .map(|(_, content)| content)
                .collect::<Vec<_>>()
                .join("\n")
        },
    )
}

/// The code a refused document is refused with.
fn refused(text: &str) -> (DiagnosticCode, String) {
    let err = compile(text, Language::Rust).expect_err("the render is refused");
    let message = err.error.to_string();
    (err.error.to_diagnostics()[0].code, message)
}

/// A render that keeps every rule is generated on every backend that can name
/// the template's unit, through its `render` and the line that brings it into
/// scope, and Rust, C++, C11 and Swift log it by the template and the variable
/// (Go's line is held by the static lowering's
/// `a_render_is_logged_through_the_machine_runtime_that_has_a_log`).
#[test]
fn a_render_is_lowered_through_the_templates_unit_on_every_backend() {
    let text = document(VARIABLES, &render("note", EVERY_HOLE));
    for &lang in Language::ALL {
        if lang == Language::Go {
            // Go's import path is rooted at a module the build names
            // (`--go-module-prefix`), which a single-file compile has not.
            continue;
        }
        let code = compile(&text, lang).unwrap_or_else(|e| panic!("{lang:?}: {e}"));
        let (call, import, logs) = match lang {
            Language::Rust => (
                "text_notice::render(",
                Some("use super::text_notice;"),
                true,
            ),
            Language::Cpp => (
                "SCE::Generated::TextNotice::render(",
                Some("#include \"text_notice.h\""),
                true,
            ),
            Language::C11 => (
                "text_notice_render(",
                Some("#include \"text_notice.h\""),
                true,
            ),
            Language::Kotlin => (
                "TextNotice.render(",
                Some("import com.sce.generated.text_notice.*"),
                false,
            ),
            Language::Python => (".render(", Some("import text_notice as"), false),
            // Swift logs through its machine runtime's public entry point.
            Language::Swift => ("TextNotice.render(", None, false),
            Language::Go => unreachable!("skipped above"),
        };
        assert!(code.contains(call), "{lang:?} calls {call}:\n{code}");
        if let Some(import) = import {
            assert!(code.contains(import), "{lang:?} imports {import}:\n{code}");
        }
        assert_eq!(
            code.contains("<sce:render template=\\\"Notice\\\"> wrote note"),
            logs,
            "{lang:?} logs the render: {logs}"
        );
        if lang == Language::Swift {
            assert!(
                code.contains(
                    "sceLogRender(machine: \"probe\", template: \"Notice\", location: \"note\")"
                ),
                "Swift logs the render through SceRuntime:\n{code}"
            );
        }
    }
}

/// C11 renders into a buffer of `MAX_LEN + 1` bytes, read through the runtime's
/// receiver of a rendering, so a text that did not fit is a failure the
/// statement receives.
#[test]
fn c11_renders_into_a_buffer_of_the_templates_longest_text() {
    let code = compile(
        &document(VARIABLES, &render("note", EVERY_HOLE)),
        Language::C11,
    )
    .expect("the render generates");
    assert!(
        code.contains(
            "sce_forge_text_rendered(&sce_failure_, text_notice_render((char[96]){0}, 96u"
        ),
        "{code}"
    );
}

/// Go names the template's unit by a module path the build is given, as it
/// names a called document's package, and asks for that path where it is not
/// given (`generate/invalid-config`), as it does for a callee: a machine that
/// renders a template is a machine that imports a package.
#[test]
fn go_asks_for_the_module_a_rendered_templates_package_lives_under() {
    let err = compile(
        &document(VARIABLES, &render("note", EVERY_HOLE)),
        Language::Go,
    )
    .expect_err("no module prefix");
    assert_eq!(
        err.error.to_diagnostics()[0].code,
        DiagnosticCode::GenerateInvalidConfig,
        "{err}"
    );
    assert!(err.error.to_string().contains("go_module_prefix"), "{err}");
}

/// The Interpreter's lowering replaces the element with an `<assign>` of the
/// library's `render`, handed the template's parts: the untrusted text fenced,
/// the integer as its digits, and the markers.
#[test]
fn the_interpreter_assigns_the_librarys_rendering_of_the_parts() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    for sibling in ["text_notice.scxml", "enum_view_mode.scxml"] {
        fs::copy(fixtures().join(sibling), dir.path().join(sibling)).expect("copy sibling");
    }
    let path = dir.path().join("probe.scxml");
    fs::write(&path, document(VARIABLES, &render("note", EVERY_HOLE))).expect("write");
    let lowered = sce_build::forge::static_js::lower_file(path.to_str().unwrap(), Vec::new())
        .expect("lowers");
    assert!(lowered.contains("<assign location=\"note\""), "{lowered}");
    assert!(lowered.contains("SceStatic.render(["), "{lowered}");
    assert!(lowered.contains("{fenced: "), "{lowered}");
    assert!(lowered.contains("String("), "{lowered}");
    // The markers, as single-quoted strings written in an XML attribute.
    assert!(
        lowered.contains("&apos;[[TEXT&apos;, &apos;TEXT]]&apos;"),
        "{lowered}"
    );
    assert!(!lowered.contains("<sce:render"), "{lowered}");
}

/// Each rule of the statement refuses its own case: a template the document
/// does not import, a location that is no string variable, a location below
/// `MAX_LEN`, a set naming no hole, a hole given twice or not at all, a text
/// past its hole's bound, a number of another width, and a value of another
/// enum or none.
#[test]
fn each_rule_of_the_statement_refuses_its_case() {
    let unknown = document(
        VARIABLES,
        &render("note", EVERY_HOLE).replace("template=\"Notice\"", "template=\"Other\""),
    );
    assert_eq!(
        refused(&unknown).0,
        DiagnosticCode::ValidationInvalidAttribute
    );

    let not_text = document(VARIABLES, &render("count", EVERY_HOLE));
    assert_eq!(
        refused(&not_text).0,
        DiagnosticCode::ValidationInvalidAttribute
    );

    let short = document(
        &VARIABLES.replace("string[95]", "string[94]"),
        &render("note", EVERY_HOLE),
    );
    let (code, message) = refused(&short);
    assert_eq!(code, DiagnosticCode::ValidationRenderBound);
    assert!(
        message.contains("94") && message.contains("95"),
        "{message}"
    );

    let cases: [(Vec<(&str, &str)>, &str); 7] = [
        (
            vec![
                ("who", "who"),
                ("count", "count"),
                ("view", "view"),
                ("said", "said"),
                ("extra", "who"),
            ],
            "no hole of the template",
        ),
        (
            vec![
                ("who", "who"),
                ("who", "who"),
                ("count", "count"),
                ("view", "view"),
                ("said", "said"),
            ],
            "given twice",
        ),
        (
            vec![("who", "who"), ("count", "count"), ("view", "view")],
            "not given",
        ),
        (
            vec![
                ("who", "wide"),
                ("count", "count"),
                ("view", "view"),
                ("said", "said"),
            ],
            "up to 9 bytes",
        ),
        (
            vec![
                ("who", "who"),
                ("count", "big"),
                ("view", "view"),
                ("said", "said"),
            ],
            "fits only its own type",
        ),
        (
            vec![
                ("who", "who"),
                ("count", "count"),
                ("view", "count"),
                ("said", "said"),
            ],
            "a value of no enum",
        ),
        (
            vec![
                ("who", "count"),
                ("count", "count"),
                ("view", "view"),
                ("said", "said"),
            ],
            "is a text of string[8]",
        ),
    ];
    for (sets, says) in &cases {
        let (code, message) = refused(&document(VARIABLES, &render("note", sets)));
        assert_eq!(
            code,
            DiagnosticCode::ValidationRenderHoles,
            "{says}: {message}"
        );
        assert!(message.contains(says), "{says}: {message}");
    }
}

/// Under another data model the element is the static data model's.
#[test]
fn a_render_is_refused_under_another_data_model() {
    let text = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="ecmascript" name="probe">
  <state id="idle">
    <transition event="go">
      <sce:render template="Notice" location="note"/>
    </transition>
  </state>
</scxml>
"#;
    assert_eq!(refused(text).0, DiagnosticCode::ScxmlStaticDatamodelRule);
}
