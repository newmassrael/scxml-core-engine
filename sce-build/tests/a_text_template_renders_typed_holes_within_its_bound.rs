// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The `text-template` kind (docs/adr/0087, section 3): what a document of it
//! reads as, each rule of its holes and its fence refused by its own code, and
//! the unit every backend generates, whose `MAX_LEN` is the length of the
//! longest text it renders.

use sce_build::compile_forge_from_string;
use sce_build::forge::codegen_matrix::{self, EmitOutcome, KindClass};
use sce_build::forge::diagnostic::ToDiagnostics;
use sce_build::forge::error::{ForgeError, ValidationError};
use sce_build::forge::model::{
    ForgeDocument, ForgeKind, HoleKind, HoleTrust, SceType, TemplatePart, TextTemplateModel,
};
use sce_build::forge::parser::parse_forge_with_imports;
use sce_build::generator::Language;
use sce_build::DocumentLabel;

const LABEL: DocumentLabel<'static> = DocumentLabel {
    identifier: "notice",
    diagnostic_label: "notice.scxml",
};

/// A template document of `datamodel`, `fence` and `text`, the three parts a
/// template is made of, each as written.
fn document(datamodel: &str, fence: &str, text: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="text-template" name="notice">
  <datamodel>
{datamodel}
  </datamodel>
  {fence}
  <sce:text>{text}</sce:text>
</scxml>"#
    )
}

fn parse(datamodel: &str, fence: &str, text: &str) -> TextTemplateModel {
    let parsed = parse_forge_with_imports(&document(datamodel, fence, text), LABEL)
        .unwrap_or_else(|e| panic!("a valid template parses: {e:?}"))
        .expect("a template is a forge document");
    match parsed.document {
        ForgeDocument::TextTemplate(model) => model,
        other => panic!("a text-template document, got {:?}", other.kind()),
    }
}

/// The validation refusal of the document.
fn refusal(datamodel: &str, fence: &str, text: &str) -> ValidationError {
    let err = parse_forge_with_imports(&document(datamodel, fence, text), LABEL)
        .expect_err("the template is refused");
    match err.error {
        ForgeError::Validation(v) => *v,
        other => panic!("expected a validation error, got {other:?}"),
    }
}

/// The wire code of a refusal, as a consumer routes it.
fn code_of(datamodel: &str, fence: &str, text: &str) -> &'static str {
    let err = parse_forge_with_imports(&document(datamodel, fence, text), LABEL)
        .expect_err("the template is refused");
    err.error.to_diagnostics()[0].code.as_str()
}

const HOLES: &str = r#"    <data id="sender" sce:type="string[8]" sce:trust="trusted"/>
    <data id="count" sce:type="int16"/>
    <data id="said" sce:type="string[12]" sce:trust="untrusted"/>"#;
const FENCE: &str = r#"<sce:fence open="[[" close="]]>"/>"#;
const TEXT: &str = r#"From <sce:put hole="sender"/> (<sce:put hole="count"/>): <sce:put hole="said"/>
again: <sce:put hole="said"/>"#;

#[test]
fn a_template_reads_as_its_holes_its_fence_and_its_text() {
    let model = parse(HOLES, FENCE, TEXT);
    assert_eq!(model.name, "notice");
    assert_eq!(model.revision(), 1);
    let holes: Vec<(&str, &HoleKind)> = model
        .holes
        .iter()
        .map(|h| (h.id.as_str(), &h.kind))
        .collect();
    assert!(matches!(
        holes[0],
        (
            "sender",
            HoleKind::Text {
                trust: HoleTrust::Trusted,
                ..
            }
        )
    ));
    assert_eq!(holes[1], ("count", &HoleKind::Integer(SceType::Int16)));
    assert!(matches!(
        holes[2],
        (
            "said",
            HoleKind::Text {
                trust: HoleTrust::Untrusted,
                ..
            }
        )
    ));
    let fence = model
        .fence
        .as_ref()
        .expect("an untrusted hole has its fence");
    assert_eq!((fence.open.as_str(), fence.close.as_str()), ("[[", "]]>"));
    assert_eq!(
        model.parts,
        vec![
            TemplatePart::Literal("From ".to_string()),
            TemplatePart::Put("sender".to_string()),
            TemplatePart::Literal(" (".to_string()),
            TemplatePart::Put("count".to_string()),
            TemplatePart::Literal("): ".to_string()),
            TemplatePart::Put("said".to_string()),
            TemplatePart::Literal("\nagain: ".to_string()),
            TemplatePart::Put("said".to_string()),
        ]
    );
}

/// The text is the character data XML hands over: entities expanded, a CDATA
/// section's text as written, a comment not part of it, and nothing trimmed.
#[test]
fn the_text_is_its_character_data_as_xml_hands_it() {
    let model = parse(
        r#"    <data id="n" sce:type="uint8"/>"#,
        "",
        "  a&amp;b<!-- not text --><![CDATA[<c>]]> <sce:put hole=\"n\"/>  ",
    );
    assert_eq!(
        model.parts,
        vec![
            TemplatePart::Literal("  a&b<c> ".to_string()),
            TemplatePart::Put("n".to_string()),
            TemplatePart::Literal("  ".to_string()),
        ]
    );
}

/// Each rule of holes refuses its own case as `validation/text-template-hole`,
/// naming the hole: a type whose text differs by backend, `sce:trust` missing
/// on a text or written on another hole, a hole declared twice or twice in snake
/// case, a hole no backend can name, a put naming no hole, and a hole never put.
#[test]
fn each_rule_of_holes_refuses_its_case() {
    let cases: [(&str, &str, &str, &str); 9] = [
        (
            r#"    <data id="on" sce:type="bool"/>"#,
            r#"<sce:put hole="on"/>"#,
            "on",
            "not the same on every backend",
        ),
        (
            r#"    <data id="ratio" sce:type="float64"/>"#,
            r#"<sce:put hole="ratio"/>"#,
            "ratio",
            "not the same on every backend",
        ),
        (
            r#"    <data id="title" sce:type="string[8]"/>"#,
            r#"<sce:put hole="title"/>"#,
            "title",
            "sce:trust=\"trusted\" or sce:trust=\"untrusted\"",
        ),
        (
            r#"    <data id="count" sce:type="uint8" sce:trust="trusted"/>"#,
            r#"<sce:put hole="count"/>"#,
            "count",
            "only a text hole",
        ),
        (
            r#"    <data id="n" sce:type="uint8"/>
    <data id="n" sce:type="uint16"/>"#,
            r#"<sce:put hole="n"/>"#,
            "n",
            "declared twice",
        ),
        (
            r#"    <data id="dayCount" sce:type="uint8"/>
    <data id="day_count" sce:type="uint8"/>"#,
            r#"<sce:put hole="dayCount"/><sce:put hole="day_count"/>"#,
            "day_count",
            "again in snake case",
        ),
        (
            r#"    <data id="_hidden" sce:type="uint8"/>"#,
            r#"<sce:put hole="_hidden"/>"#,
            "_hidden",
            "a letter and then",
        ),
        (
            r#"    <data id="n" sce:type="uint8"/>"#,
            r#"<sce:put hole="n"/><sce:put hole="m"/>"#,
            "m",
            "declared by no <data>",
        ),
        (
            r#"    <data id="n" sce:type="uint8"/>
    <data id="unused" sce:type="uint8"/>"#,
            r#"<sce:put hole="n"/>"#,
            "unused",
            "never put",
        ),
    ];
    for (datamodel, text, hole, says) in cases {
        match refusal(datamodel, "", text) {
            ValidationError::TextTemplateHole {
                template,
                hole: refused,
                detail,
            } => {
                assert_eq!(template, "notice");
                assert_eq!(refused, hole, "{datamodel}");
                assert!(detail.contains(says), "{hole}: {detail}");
            }
            other => panic!("{hole}: expected text-template-hole, got {other:?}"),
        }
        assert_eq!(
            code_of(datamodel, "", text),
            "validation/text-template-hole"
        );
    }
}

/// Each rule of fences refuses its own case as `validation/text-template-fence`:
/// an untrusted hole with no fence, a fence with no untrusted hole, a second
/// fence, and a marker that is empty, too long, not printable ASCII, inside the
/// other, or in the template's own text.
#[test]
fn each_rule_of_fences_refuses_its_case() {
    let untrusted = r#"    <data id="said" sce:type="string[8]" sce:trust="untrusted"/>"#;
    let trusted = r#"    <data id="said" sce:type="string[8]" sce:trust="trusted"/>"#;
    let put = r#"x <sce:put hole="said"/>"#;
    let long = format!(r#"<sce:fence open="{}" close="]]"/>"#, "x".repeat(33));
    let cases: Vec<(&str, String, &str, &str)> = vec![
        (untrusted, String::new(), put, "declares no <sce:fence"),
        (trusted, FENCE.to_string(), put, "no untrusted hole"),
        (
            untrusted,
            format!("{FENCE}{FENCE}"),
            put,
            "a second <sce:fence>",
        ),
        (
            untrusted,
            r#"<sce:fence open="" close="]]"/>"#.to_string(),
            put,
            "empty",
        ),
        (untrusted, long, put, "33 bytes long"),
        (
            untrusted,
            "<sce:fence open=\"\u{ab}\" close=\"]]\"/>".to_string(),
            put,
            "printable ASCII",
        ),
        (
            untrusted,
            r#"<sce:fence open="TEXT" close="TEXT&gt;"/>"#.to_string(),
            put,
            "inside the other",
        ),
        (
            untrusted,
            FENCE.to_string(),
            r#"see [[ <sce:put hole="said"/>"#,
            "the template's own text",
        ),
    ];
    for (datamodel, fence, text, says) in &cases {
        match refusal(datamodel, fence, text) {
            ValidationError::TextTemplateFence { detail, .. } => {
                assert!(detail.contains(says), "{fence}: {detail}");
            }
            other => panic!("{fence}: expected text-template-fence, got {other:?}"),
        }
        assert_eq!(
            code_of(datamodel, fence, text),
            "validation/text-template-fence"
        );
    }
    // A marker of exactly 32 bytes is admitted.
    let at_bound = format!(r#"<sce:fence open="{}" close="]]"/>"#, "x".repeat(32));
    assert!(parse(untrusted, &at_bound, put).fence.is_some());
}

/// The longest text the template renders, by the rule of docs/adr/0087 section
/// 3.1 item 5 written out for this document: the literal bytes, and each put at
/// its longest.
fn longest_rendering_of_holes_fence_and_text() -> usize {
    let literals = "From ".len() + " (".len() + "): ".len() + "\nagain: ".len();
    let sender = 8;
    let count = "-32768".len();
    let said = "[[".len() + 12 + "]]>".len();
    literals + sender + count + 2 * said
}

/// Every backend generates the unit, declares the same `MAX_LEN`, the length of
/// the longest rendering, and removes the fence's markers from the untrusted
/// hole through its forge runtime, and from no other hole.
#[test]
fn every_backend_declares_the_longest_rendering_and_fences_the_untrusted_hole() {
    let max_len = longest_rendering_of_holes_fence_and_text();
    let xml = document(HOLES, FENCE, TEXT);
    for &lang in Language::ALL {
        let output = compile_forge_from_string(&xml, LABEL, lang)
            .unwrap_or_else(|e| panic!("{lang:?}: the template generates: {e:?}"));
        let (_, code) = output
            .files
            .first()
            .unwrap_or_else(|| panic!("{lang:?}: the unit is one file"));
        let (declared, removes) = match lang {
            Language::Rust => (
                format!("pub const MAX_LEN: usize = {max_len};"),
                "remove_markers(",
            ),
            Language::Go => (
                format!("const MaxLen int = {max_len}"),
                "texttemplate.RemoveMarkers(",
            ),
            Language::Kotlin => (
                format!("public const val MAX_LEN: Int = {max_len}"),
                "TextTemplate.removeMarkers(",
            ),
            Language::Python => (
                format!("MAX_LEN: Final[int] = {max_len}"),
                "remove_markers(",
            ),
            Language::Cpp => (
                format!("inline constexpr std::size_t MAX_LEN = {max_len}u;"),
                "::SCE::Forge::removeMarkers(",
            ),
            Language::C11 => (
                format!("#define NOTICE_MAX_LEN ((size_t){max_len}u)"),
                "sce_forge_text_put_fenced(",
            ),
            Language::Swift => (
                format!("public static let maxLen: Int = {max_len}"),
                "TextTemplate.removeMarkers(",
            ),
        };
        assert!(
            code.contains(&declared),
            "{lang:?} declares {declared}:\n{code}"
        );
        // The untrusted hole is put twice, and each put removes the markers;
        // the trusted text and the integer are put as they are.
        let calls = code.matches(removes).count();
        let expected = match lang {
            // The Rust unit removes them in its one `put_fenced`, called twice.
            Language::Rust => 1,
            _ => 2,
        };
        assert_eq!(
            calls, expected,
            "{lang:?} removes markers {calls} times:\n{code}"
        );
    }
}

/// A template with no untrusted hole has no fence and calls nothing of the
/// forge runtime: its unit depends on nothing.
#[test]
fn a_template_of_trusted_holes_needs_no_runtime() {
    let xml = document(
        r#"    <data id="sender" sce:type="string[8]" sce:trust="trusted"/>"#,
        "",
        r#"Hi <sce:put hole="sender"/>"#,
    );
    for &lang in Language::ALL {
        let output = compile_forge_from_string(&xml, LABEL, lang)
            .unwrap_or_else(|e| panic!("{lang:?}: the template generates: {e:?}"));
        let code = &output.files[0].1;
        assert!(
            !code.to_lowercase().contains("removemarkers"),
            "{lang:?}:\n{code}"
        );
        assert!(!code.contains("remove_markers"), "{lang:?}:\n{code}");
    }
}

/// A hole's named bound is the number the build gives its name (`--limit`,
/// docs/adr/0050), read by the type reader every bound is read by, so `MAX_LEN`
/// moves with it: a trusted `string[note_text=8]` hole adds 8 bytes as written
/// and 20 under `note_text=20`.
#[test]
fn a_limit_on_a_holes_bound_moves_max_len() {
    let xml = document(
        r#"    <data id="note" sce:type="string[note_text=8]" sce:trust="trusted"/>"#,
        "",
        r#"Note: <sce:put hole="note"/>"#,
    );
    let max_len_of = |code: &str| -> String {
        let at = code
            .find("pub const MAX_LEN: usize = ")
            .expect("the Rust unit declares MAX_LEN");
        code[at..].lines().next().unwrap_or_default().to_string()
    };
    let written = compile_forge_from_string(&xml, LABEL, Language::Rust).expect("generates");
    assert_eq!(
        max_len_of(&written.files[0].1),
        format!("pub const MAX_LEN: usize = {};", "Note: ".len() + 8)
    );
    let limits = sce_build::limits::Limits::none()
        .with("note_text", 20)
        .expect("a limit");
    let _scope = sce_build::limits::scope(&limits);
    let moved = compile_forge_from_string(&xml, LABEL, Language::Rust).expect("generates");
    assert_eq!(
        max_len_of(&moved.files[0].1),
        format!("pub const MAX_LEN: usize = {};", "Note: ".len() + 20)
    );
}

#[test]
fn the_kind_is_a_generic_stateless_kind_every_backend_generates() {
    assert_eq!(
        ForgeKind::from_attr("text-template"),
        Some(ForgeKind::TextTemplate)
    );
    assert_eq!(ForgeKind::TextTemplate.to_string(), "text-template");
    assert!(ForgeKind::TextTemplate.is_supported());
    assert!(!ForgeKind::TextTemplate.needs_instance());
    assert!(!ForgeKind::TextTemplate.is_inline_eligible());
    assert_eq!(
        codegen_matrix::kind_class(ForgeKind::TextTemplate),
        KindClass::Generic
    );
    for &lang in Language::ALL {
        assert_eq!(
            codegen_matrix::lookup(ForgeKind::TextTemplate, lang),
            EmitOutcome::Emit,
            "{lang:?}"
        );
    }
}
