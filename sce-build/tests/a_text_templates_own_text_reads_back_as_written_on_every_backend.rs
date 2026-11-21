// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The text a text template's author writes, its fence's two markers and the
//! literal parts of its text, reads back from every backend's generated unit as
//! the author wrote it.
//!
//! A marker is any printable ASCII the parser admits
//! (`forge::text_template::fence_refusal`), and the literal text is whatever
//! character data XML hands over, so both can hold a quote, a backslash, a
//! Kotlin template opener (`$`) and a run of `?` that C11 reads as a trigraph.
//! Each lands inside a string literal of the generated source, where no
//! template line escapes it by hand: the registration door
//! (`generator::register_template`, `literal_text::encode_template_literals`)
//! routes every value written into a literal through its language's escaper,
//! and the C and C++ units write theirs through `c_char_array_init`,
//! `c_text_expr` and `cpp_text_expr`. This file holds the result rather than
//! the route: each literal is read out of the generated source, held to end
//! where the template ends it, and read back through the escapes its
//! language's compiler reads (`common::string_literal`).

mod common;

use common::string_literal::{literal_value, Escapes};
use sce_build::compile_forge_from_string;
use sce_build::forge::model::{ForgeDocument, TemplatePart};
use sce_build::forge::parser::parse_forge_with_imports;
use sce_build::generator::Language;
use sce_build::DocumentLabel;

const LABEL: DocumentLabel<'static> = DocumentLabel {
    identifier: "notice",
    diagnostic_label: "notice.scxml",
};

/// The fence's open marker: a trigraph opener, a quote, a backslash and a
/// Kotlin template opener.
const OPEN: &str = r#"[??("\$"#;
/// The close marker: a Kotlin template opener, a backslash, an apostrophe
/// (which every backend's string reads as itself, and Go refuses escaped) and
/// a trigraph closer.
const CLOSE: &str = r#"$\'??)]"#;

/// The literal text before the put: quotes, a backslash, a Kotlin template
/// expression, a C11 trigraph, an apostrophe, braces and a C format verb.
const BEFORE: &str = r#" say "hi" \ ${x} ??= 'y' {z} %s "#;
/// The literal text after the put: an escaped quote as the author wrote it.
const AFTER: &str = r#"\" end"#;

/// The template: one untrusted hole between the two literal parts, fenced by
/// [`OPEN`] and [`CLOSE`], each written in its attribute with the quote as
/// `&quot;`.
fn document() -> String {
    let attribute = |marker: &str| marker.replace('"', "&quot;");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="text-template" name="notice">
  <datamodel>
    <data id="said" sce:type="string[12]" sce:trust="untrusted"/>
  </datamodel>
  <sce:fence open="{}" close="{}"/>
  <sce:text>{BEFORE}<sce:put hole="said"/>{AFTER}</sce:text>
</scxml>"#,
        attribute(OPEN),
        attribute(CLOSE)
    )
}

/// How one backend spells the two marker constants and a literal part of the
/// rendering: the source before each literal's opening quote, and the source
/// after its closing quote to the end of the line.
struct Spelling {
    escapes: Escapes,
    open: &'static str,
    close: &'static str,
    after_marker: &'static str,
    part: &'static str,
    after_part: &'static str,
}

fn spelling(lang: Language) -> Spelling {
    match lang {
        Language::Rust => Spelling {
            escapes: Escapes::Rust,
            open: "pub const FENCE_OPEN: &str = ",
            close: "pub const FENCE_CLOSE: &str = ",
            after_marker: ";",
            part: "out.push_str(",
            after_part: ");",
        },
        Language::Go => Spelling {
            escapes: Escapes::Go,
            open: "const FenceOpen string = ",
            close: "const FenceClose string = ",
            after_marker: "",
            part: "out.WriteString(",
            after_part: ")",
        },
        Language::Kotlin => Spelling {
            escapes: Escapes::Kotlin,
            open: "public const val FENCE_OPEN: String = ",
            close: "public const val FENCE_CLOSE: String = ",
            after_marker: "",
            part: "out.append(",
            after_part: ")",
        },
        Language::Python => Spelling {
            escapes: Escapes::Python,
            open: "FENCE_OPEN: Final[str] = ",
            close: "FENCE_CLOSE: Final[str] = ",
            after_marker: "",
            part: "out.append(",
            after_part: ")",
        },
        Language::Cpp => Spelling {
            escapes: Escapes::CFamily,
            open: "inline constexpr std::string_view FENCE_OPEN = ",
            close: "inline constexpr std::string_view FENCE_CLOSE = ",
            after_marker: ";",
            part: "out += ",
            after_part: ";",
        },
        Language::C11 => Spelling {
            escapes: Escapes::CFamily,
            open: "#define NOTICE_FENCE_OPEN ",
            close: "#define NOTICE_FENCE_CLOSE ",
            after_marker: "",
            part: "sce_forge_text_put(&text, ",
            after_part: ");",
        },
        Language::Swift => Spelling {
            escapes: Escapes::Swift,
            open: "public static let fenceOpen: String = ",
            close: "public static let fenceClose: String = ",
            after_marker: "",
            part: "out += ",
            after_part: "",
        },
    }
}

/// The string literal that opens `text`: its body, the characters between
/// its quotes, and what follows its closing quote. The literal closes at the
/// first quote no backslash escapes; `None` when `text` opens no literal or
/// the literal never closes.
fn literal(text: &str) -> Option<(&str, &str)> {
    let inner = text.strip_prefix('"')?;
    let mut escaped = false;
    for (at, ch) in inner.char_indices() {
        match ch {
            _ if escaped => escaped = false,
            '\\' => escaped = true,
            '"' => return Some((&inner[..at], &inner[at + 1..])),
            _ => {}
        }
    }
    None
}

/// Every string literal a line of `code` opens right after `head`, as its
/// body and what follows it on the line.
fn literals_after<'a>(code: &'a str, head: &str) -> Vec<(&'a str, &'a str)> {
    code.lines()
        .filter_map(|line| line.trim_start().strip_prefix(head))
        .filter_map(literal)
        .collect()
}

/// What in a literal's body the escapes alone do not answer for: an
/// unescaped character the language reads as other than itself. Kotlin reads
/// a bare `$` as the start of a template, and C11 a bare `??` as the start
/// of a trigraph. `None` when the body holds neither.
fn read_otherwise(escapes: Escapes, body: &str) -> Option<String> {
    let mut escaped = false;
    let mut bare_question = false;
    for ch in body.chars() {
        if escaped {
            escaped = false;
            bare_question = false;
            continue;
        }
        match (escapes, ch) {
            (_, '\\') => escaped = true,
            (Escapes::Kotlin, '$') => {
                return Some(format!(
                    "a bare `$`, which Kotlin reads as a template: `{body}`"
                ))
            }
            (Escapes::CFamily, '?') if bare_question => {
                return Some(format!(
                    "a bare `??`, which C11 reads as a trigraph: `{body}`"
                ))
            }
            _ => {}
        }
        bare_question = ch == '?';
    }
    None
}

/// The text one literal of `lang` compiles to, once it is held to end where
/// the template ends it (`after`) and to carry nothing its language reads as
/// other than itself.
fn read_back(lang: Language, escapes: Escapes, body: &str, rest: &str, after: &str) -> String {
    assert_eq!(
        rest, after,
        "{lang:?}: the literal `{body}` ends before the template ends it, at a \
         quote left unescaped"
    );
    if let Some(why) = read_otherwise(escapes, body) {
        panic!("{lang:?}: {why}");
    }
    literal_value(escapes, body)
}

/// The parser admits the fence and the text as written, so what the tests
/// below hold is generation and not a refusal.
#[test]
fn the_parser_admits_a_fence_and_a_text_of_quotes_and_backslashes() {
    let parsed = parse_forge_with_imports(&document(), LABEL)
        .unwrap_or_else(|e| panic!("the template parses: {e:?}"))
        .expect("a template is a forge document");
    let ForgeDocument::TextTemplate(model) = parsed.document else {
        panic!("a text-template document");
    };
    let fence = model.fence.expect("an untrusted hole has its fence");
    assert_eq!((fence.open.as_str(), fence.close.as_str()), (OPEN, CLOSE));
    assert_eq!(
        model.parts,
        vec![
            TemplatePart::Literal(BEFORE.to_string()),
            TemplatePart::Put("said".to_string()),
            TemplatePart::Literal(AFTER.to_string()),
        ]
    );
}

/// Every backend declares each marker once, as a literal that compiles to the
/// marker.
#[test]
fn every_backend_writes_a_fence_of_quotes_and_backslashes_as_the_fence() {
    let xml = document();
    for &lang in Language::ALL {
        let output = compile_forge_from_string(&xml, LABEL, lang)
            .unwrap_or_else(|e| panic!("{lang:?}: the template generates: {e:?}"));
        let code = &output.files[0].1;
        let spelling = spelling(lang);
        for (head, marker) in [(spelling.open, OPEN), (spelling.close, CLOSE)] {
            let found = literals_after(code, head);
            let [(body, rest)] = found.as_slice() else {
                panic!(
                    "{lang:?}: one literal after `{head}`, found {}:\n{code}",
                    found.len()
                );
            };
            assert_eq!(
                read_back(lang, spelling.escapes, body, rest, spelling.after_marker),
                marker,
                "{lang:?}: `{head}` does not compile to the marker:\n{code}"
            );
        }
    }
}

/// Every backend writes the literal parts of the text as literals that compile
/// to them, in order.
#[test]
fn every_backend_writes_a_text_of_quotes_and_backslashes_as_the_text() {
    let xml = document();
    for &lang in Language::ALL {
        let output = compile_forge_from_string(&xml, LABEL, lang)
            .unwrap_or_else(|e| panic!("{lang:?}: the template generates: {e:?}"));
        let code = &output.files[0].1;
        let spelling = spelling(lang);
        let parts: Vec<String> = literals_after(code, spelling.part)
            .into_iter()
            .map(|(body, rest)| read_back(lang, spelling.escapes, body, rest, spelling.after_part))
            .collect();
        assert_eq!(
            parts,
            [BEFORE, AFTER],
            "{lang:?}: the literal parts do not compile to the text:\n{code}"
        );
    }
}

/// The readers above find what they are written to find, on text built here:
/// a literal closes at its first unescaped quote, and a bare `$` (Kotlin) or
/// `??` (C) is reported where an escaped one is not.
#[test]
fn the_readers_close_a_literal_at_its_quote_and_report_a_bare_opener() {
    assert_eq!(literal(r#""a\"b" + 1"#), Some((r#"a\"b"#, " + 1")));
    assert_eq!(literal(r#""a\\" b""#), Some((r#"a\\"#, r#" b""#)));
    assert_eq!(literal(r#""never closed\""#), None);
    assert_eq!(literal("no quote"), None);
    assert_eq!(
        literals_after("    x = \"1\";\n    x = y;\n  x = \"2\";\n", "x = "),
        [("1", ";"), ("2", ";")]
    );
    assert!(read_otherwise(Escapes::Kotlin, "a${b}").is_some());
    assert!(read_otherwise(Escapes::Kotlin, r"a\${b}").is_none());
    assert!(read_otherwise(Escapes::CFamily, "[??(").is_some());
    assert!(read_otherwise(Escapes::CFamily, r"[?\?(").is_none());
    assert!(read_otherwise(Escapes::Rust, "${x} ??(").is_none());
}
