// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A string literal is read once, by the lexer both dialects share, into the text
// it denotes (`decode_string_literal`), and every emitter writes that text
// through its own language's escaper. These tests hold both halves: what each
// escape denotes and which are refused, and what each backend writes for a text
// that would break its literal if it were pasted between quotes.

use super::*;

/// The text the lexer reads `source`, one string literal, as in `mode`.
fn value(source: &str, mode: LexMode) -> String {
    let tokens =
        tokenize_as(source, mode).unwrap_or_else(|e| panic!("{source} tokenizes in {mode:?}: {e}"));
    match tokens.as_slice() {
        [Token::String { value }, Token::Eof] => value.clone(),
        other => panic!("{source} is not one string literal: {other:?}"),
    }
}

/// The escape `source` is refused at in `mode`, as written, and the range of
/// the expression the refusal is placed on.
fn refused(source: &str, mode: LexMode) -> (String, Option<std::ops::Range<usize>>) {
    match tokenize_spanned(source, mode) {
        Err(Spanned {
            error: ExprError::UnsupportedStringEscape { escape, .. },
            span,
        }) => (escape, span),
        other => panic!("{source} was not refused as a string escape in {mode:?}: {other:?}"),
    }
}

#[test]
fn both_dialects_read_the_ecmascript_escapes_into_their_characters() {
    let cases: &[(&str, &str)] = &[
        (r"'a\\b'", "a\\b"),
        (r"'a\'b'", "a'b"),
        (r#"'a\"b'"#, "a\"b"),
        (r"'\b\f\n\r\t\v'", "\u{8}\u{c}\n\r\t\u{b}"),
        (r"'a\0b'", "a\u{0}b"),
        (r"'\x41\x7e'", "A~"),
        (r"'\u{10348}'", "\u{10348}"),
        (r"'\u{0000041}'", "A"),
        (r"'\u{D800}\u{DF48}'", "\u{10348}"),
        // A backslash before a line break continues the literal, and the
        // break is not part of the value.
        ("'a\\\nb'", "ab"),
        ("'a\\\r\nb'", "ab"),
        ("'a\\\rb'", "ab"),
        ("'a\\\u{2028}b'", "ab"),
        // A line break written raw is kept as the character it is.
        ("'a\nb'", "a\nb"),
        // A quote of the other kind needs no escape, and nothing else is
        // special between the quotes.
        ("'say \"hi\"'", "say \"hi\""),
        ("\"it's\"", "it's"),
        ("'$name ${x} \u{e9}'", "$name ${x} \u{e9}"),
        ("''", ""),
    ];
    // A four-digit `\u` escape, assembled at run time: written out whole in
    // this file's source, the editing tools this tree is written with read it
    // as the character it names and write that instead, which is how the
    // first version of these two cases came to test nothing.
    let unit = |hex: &str| format!("\\u{hex}");
    let four_digit = [
        (format!("'{}'", unit("00e9")), "\u{e9}"),
        // The two halves of a surrogate pair are the one character they make,
        // in either spelling of a code unit (the braced one is above).
        (format!("'{}{}'", unit("D800"), unit("DF48")), "\u{10348}"),
    ];
    for mode in [LexMode::Forge, LexMode::EcmaScript] {
        for (source, expected) in cases {
            assert_eq!(value(source, mode), *expected, "{source} in {mode:?}");
        }
        for (source, expected) in &four_digit {
            assert!(
                source.is_ascii(),
                "{source} is an escape, not its character"
            );
            assert_eq!(value(source, mode), *expected, "{source} in {mode:?}");
        }
    }
}

#[test]
fn both_dialects_refuse_a_malformed_escape_and_a_lone_surrogate() {
    let cases: &[(&str, &str)] = &[
        (r"'\x4'", r"\x"),
        (r"'\x4g'", r"\x"),
        (r"'\u12'", r"\u"),
        (r"'\u{}'", r"\u{}"),
        (r"'\u{110000}'", r"\u{110000}"),
        (r"'\u{41'", r"\u{"),
        (r"'\uD800'", r"\uD800"),
        (r"'\uDF48'", r"\uDF48"),
        (r"'\uD800x'", r"\uD800"),
        (r"'\uD800A'", r"\uD800"),
        (r"'\uD800\uD800'", r"\uD800"),
    ];
    for mode in [LexMode::Forge, LexMode::EcmaScript] {
        for (source, escape) in cases {
            assert_eq!(refused(source, mode).0, *escape, "{source} in {mode:?}");
        }
    }
}

/// ECMAScript keeps a legacy octal and a backslash before any other character
/// for compatibility; a forge expression refuses both, and the W3C ECMAScript
/// datamodel reads them as ECMAScript does.
#[test]
fn a_forge_expression_refuses_what_ecmascript_reads_only_for_compatibility() {
    let cases: &[(&str, &str, &str)] = &[
        (r"'c\d'", r"\d", "cd"),
        (r"'a\/b'", r"\/", "a/b"),
        (r"'\101'", r"\1", "A"),
        (r"'\01'", r"\0", "\u{1}"),
        (r"'\08'", r"\0", "\u{0}8"),
        (r"'\8'", r"\8", "8"),
        (r"'\477'", r"\4", "\u{27}7"),
    ];
    for (source, escape, ecmascript) in cases {
        assert_eq!(refused(source, LexMode::Forge).0, *escape, "{source}");
        assert_eq!(value(source, LexMode::EcmaScript), *ecmascript, "{source}");
    }
}

/// The refusal stands on the escape, so a document's row and column name it.
#[test]
fn an_escape_refusal_is_placed_on_the_escape() {
    assert_eq!(refused(r"'c\d'", LexMode::Forge).1, Some(2..4));
    assert_eq!(refused(r"x === 'ab\uD800'", LexMode::Forge).1, Some(9..15));
}

/// The scope every backend is asked in: `label`, a string the document holds.
fn label_ctx() -> TypeCtx<'static> {
    let mut ctx = TypeCtx::new();
    ctx.insert_var("label", InferredType::Str);
    ctx
}

/// What `target` writes for `label === <source>`, a literal where a guard
/// stands one.
fn written(source: &str, target: ExprTarget) -> String {
    transpile_typed(
        &format!("label === {source}"),
        target,
        &label_ctx(),
        &HashMap::new(),
        InferredType::Unknown,
    )
    .unwrap_or_else(|e| panic!("`label === {source}` lowers for {target:?}: {e}"))
}

/// One literal, and what each backend writes for it: Rust, Go, Kotlin, Swift,
/// Python, C11, C++ and ECMAScript for the Interpreter, in that order.
type Spellings = (&'static str, [&'static str; 8]);

const TARGETS: [ExprTarget; 8] = [
    ExprTarget::Rust,
    ExprTarget::Go,
    ExprTarget::Kotlin,
    ExprTarget::Swift,
    ExprTarget::Python,
    ExprTarget::C,
    ExprTarget::Cpp,
    ExprTarget::Js,
];

/// Each hazard, written through each backend's escaper: the value lands in the
/// literal as the value, whatever the author's quote and escapes were.
#[test]
fn every_backend_writes_the_literal_s_value_through_its_own_escaper() {
    let rows: &[Spellings] = &[
        // A double quote inside a single-quoted literal.
        (
            r#"'a"b'"#,
            [
                r#""a\"b""#,
                r#""a\"b""#,
                r#""a\"b""#,
                r#""a\"b""#,
                r#""a\"b""#,
                r#""a\"b""#,
                r#""a\"b""#,
                r#"'a"b'"#,
            ],
        ),
        // A single quote inside a double-quoted literal.
        (
            r#""it's""#,
            [
                r#""it's""#,
                r#""it's""#,
                r#""it's""#,
                r#""it's""#,
                r#""it's""#,
                r#""it's""#,
                r#""it's""#,
                r"'it\'s'",
            ],
        ),
        // A backslash.
        (
            r"'a\\b'",
            [
                r#""a\\b""#,
                r#""a\\b""#,
                r#""a\\b""#,
                r#""a\\b""#,
                r#""a\\b""#,
                r#""a\\b""#,
                r#""a\\b""#,
                r"'a\\b'",
            ],
        ),
        // A Kotlin template expression, both forms.
        (
            "'$name'",
            [
                r#""$name""#,
                r#""$name""#,
                r#""\$name""#,
                r#""$name""#,
                r#""$name""#,
                r#""$name""#,
                r#""$name""#,
                "'$name'",
            ],
        ),
        (
            "'${x}'",
            [
                r#""${x}""#,
                r#""${x}""#,
                r#""\${x}""#,
                r#""${x}""#,
                r#""${x}""#,
                r#""${x}""#,
                r#""${x}""#,
                "'${x}'",
            ],
        ),
        // A Swift interpolation: the value is a backslash and `(x)`.
        (
            r"'\\(x)'",
            [
                r#""\\(x)""#,
                r#""\\(x)""#,
                r#""\\(x)""#,
                r#""\\(x)""#,
                r#""\\(x)""#,
                r#""\\(x)""#,
                r#""\\(x)""#,
                r"'\\(x)'",
            ],
        ),
        // A line break and a tab.
        (
            r"'a\nb\tc'",
            [
                r#""a\nb\tc""#,
                r#""a\nb\tc""#,
                r#""a\nb\tc""#,
                r#""a\nb\tc""#,
                r#""a\nb\tc""#,
                r#""a\nb\tc""#,
                r#""a\nb\tc""#,
                r"'a\nb\tc'",
            ],
        ),
        // An escape only some targets have, read into the character.
        (
            r"'\x41'",
            [
                r#""A""#, r#""A""#, r#""A""#, r#""A""#, r#""A""#, r#""A""#, r#""A""#, "'A'",
            ],
        ),
        // A character past ASCII: itself, and its UTF-8 bytes in octal for C
        // and C++.
        (
            "'\u{e9}'",
            [
                "\"\u{e9}\"",
                "\"\u{e9}\"",
                "\"\u{e9}\"",
                "\"\u{e9}\"",
                "\"\u{e9}\"",
                r#""\303\251""#,
                r#""\303\251""#,
                "'\u{e9}'",
            ],
        ),
        // A character past the Basic Multilingual Plane, written as a braced
        // code point.
        (
            r"'\u{10348}'",
            [
                "\"\u{10348}\"",
                "\"\u{10348}\"",
                "\"\u{10348}\"",
                "\"\u{10348}\"",
                "\"\u{10348}\"",
                r#""\360\220\215\210""#,
                r#""\360\220\215\210""#,
                "'\u{10348}'",
            ],
        ),
        // A pair of question marks, which would begin a C trigraph.
        (
            "'a??=b'",
            [
                r#""a??=b""#,
                r#""a??=b""#,
                r#""a??=b""#,
                r#""a??=b""#,
                r#""a??=b""#,
                r#""a?\?=b""#,
                r#""a?\?=b""#,
                "'a??=b'",
            ],
        ),
    ];
    for (source, spellings) in rows {
        for (target, spelling) in TARGETS.iter().zip(spellings.iter()) {
            let text = written(source, *target);
            assert!(
                text.contains(spelling),
                "{target:?} writes `{source}` as {spelling}; got {text}"
            );
        }
    }
}

/// U+0000 is a character of the value on every backend whose string holds
/// one; C11's ends at it, so the literal is refused there by name rather than
/// cut short.
#[test]
fn a_zero_character_is_held_whole_or_refused_by_name() {
    let held: &[(ExprTarget, &str)] = &[
        (ExprTarget::Rust, r#""a\0b""#),
        (ExprTarget::Go, r#""a\u0000b""#),
        (ExprTarget::Kotlin, r#""a\u0000b""#),
        (ExprTarget::Swift, r#""a\0b""#),
        (ExprTarget::Python, r#""a\x00b""#),
        (ExprTarget::Cpp, r"(std::string{'a', '\x00', 'b'})"),
        (ExprTarget::Js, r"'a\u0000b'"),
    ];
    for (target, spelling) in held {
        let text = written(r"'a\0b'", *target);
        assert!(text.contains(spelling), "{target:?}: {text}");
    }
    let refusal = transpile_typed(
        r"label === 'a\0b'",
        ExprTarget::C,
        &label_ctx(),
        &HashMap::new(),
        InferredType::Unknown,
    )
    .expect_err("C11 holds no string with a zero in it");
    assert!(
        matches!(
            &refusal.error,
            ExprError::UnsupportedConstruct { construct, .. } if construct.contains("U+0000")
        ),
        "{refusal:?}"
    );
}

/// An escape the subset does not read is refused before any backend is asked,
/// the same on all of them.
#[test]
fn a_refused_escape_is_refused_for_every_target() {
    for target in TARGETS {
        let refusal = transpile_typed(
            r"label === 'a\uD800'",
            target,
            &label_ctx(),
            &HashMap::new(),
            InferredType::Unknown,
        )
        .expect_err("a lone surrogate is refused");
        assert!(
            matches!(
                &refusal.error,
                ExprError::UnsupportedStringEscape { escape, .. } if escape == r"\uD800"
            ),
            "{target:?}: {refusal:?}"
        );
    }
}

/// A C11 literal holds 4095 bytes of text and no more; past that the text is a
/// compound literal of its bytes.
#[test]
fn a_c_text_past_one_literal_is_a_compound_literal_of_its_bytes() {
    let fits = "x".repeat(crate::filters::C_STRING_LITERAL_MAX_BYTES);
    assert_eq!(fits.len(), 4095);
    assert_eq!(
        crate::filters::c_text_expression(&fits),
        format!("\"{fits}\"")
    );
    let long = "x".repeat(4096);
    let text = crate::filters::c_text_expression(&long);
    assert!(
        text.starts_with("((const char[]){'x', 'x', "),
        "{}",
        &text[..40]
    );
    assert!(
        text.ends_with("'x', '\\0'})"),
        "{}",
        &text[text.len() - 40..]
    );
    assert_eq!(text.matches("'x'").count(), 4096);
    // The emitter writes a literal through the same door.
    let lowered = written(&format!("'{long}'"), ExprTarget::C);
    assert!(
        lowered.contains("((const char[]){'x', "),
        "{}",
        &lowered[..60]
    );
    assert!(written(&format!("'{fits}'"), ExprTarget::C).contains(&format!("\"{fits}\"")));
}

/// A C++ literal holds 65,535 bytes of text and no more; past that, and
/// wherever the text holds a zero, the text is the `std::string` its bytes
/// initialize.
#[test]
fn a_cpp_text_past_one_literal_is_the_string_of_its_bytes() {
    let fits = "y".repeat(crate::filters::CPP_STRING_LITERAL_MAX_BYTES);
    assert_eq!(fits.len(), 65_535);
    assert_eq!(
        crate::filters::cpp_text_expression(&fits),
        format!("\"{fits}\"")
    );
    let long = "y".repeat(65_536);
    let text = crate::filters::cpp_text_expression(&long);
    assert!(
        text.starts_with("(std::string{'y', 'y', "),
        "{}",
        &text[..40]
    );
    assert!(text.ends_with("'y', 'y'})"), "{}", &text[text.len() - 40..]);
    assert_eq!(text.matches("'y'").count(), 65_536);
    let lowered = written(&format!("'{long}'"), ExprTarget::Cpp);
    assert!(lowered.contains("(std::string{'y', "), "{}", &lowered[..60]);
}

/// A Kotlin literal holds one class-file constant; a longer text is built from
/// pieces that each fit.
#[test]
fn a_kotlin_text_past_one_constant_is_built_from_pieces() {
    let long = "z".repeat(crate::filters::JVM_CONSTANT_MAX_BYTES + 1);
    let text = crate::filters::kotlin_text_expression(&long);
    assert!(
        text.starts_with("buildString { append(\"zzz"),
        "{}",
        &text[..40]
    );
    assert_eq!(text.matches("append(").count(), 2);
    let fits = "z".repeat(crate::filters::JVM_CONSTANT_MAX_BYTES);
    assert_eq!(
        crate::filters::kotlin_text_expression(&fits),
        format!("\"{fits}\"")
    );
}

/// A byte string compared with a literal is the literal's text, its escapes
/// read; Kotlin writes it through its escaper, so a `$` in it opens nothing.
#[test]
fn a_bytes_literal_is_its_text_escaped_for_the_target() {
    let mut ctx = TypeCtx::new();
    ctx.insert_var("raw", InferredType::Bytes);
    let lower = |source: &str, target: ExprTarget| {
        transpile_typed(source, target, &ctx, &HashMap::new(), InferredType::Unknown)
            .unwrap_or_else(|e| panic!("{source} lowers for {target:?}: {e}"))
    };
    assert_eq!(
        lower("raw === 'a$b'", ExprTarget::Kotlin),
        "raw.contentEquals(\"a\\$b\".toByteArray())"
    );
    assert_eq!(lower(r"raw === '\x41'", ExprTarget::Rust), "raw == b\"A\"");
    assert_eq!(
        lower("raw === 'a??=b'", ExprTarget::C),
        "raw_len == 5 && memcmp(raw, \"a?\\?=b\", 5) == 0"
    );
}

/// Each escaper, on the characters its language reads specially.
#[test]
fn each_escaper_writes_what_its_language_reads_back() {
    use crate::filters::{
        ecmascript_string_literal, escape_c, escape_go, escape_kotlin, escape_rust,
    };
    assert_eq!(
        escape_rust("a\"\\\n\r\t\u{0}\u{1}\u{7f}\u{202e}\u{e9}".to_string()),
        "a\\\"\\\\\\n\\r\\t\\0\\u{1}\\u{7f}\\u{202e}\u{e9}"
    );
    assert_eq!(
        escape_go("a\"\\\n\u{0}\u{1}\u{feff}\u{e9}".to_string()),
        "a\\\"\\\\\\n\\u0000\\u0001\\ufeff\u{e9}"
    );
    assert_eq!(
        escape_kotlin("$a${b}\"\\\n\u{0}\u{e9}".to_string()),
        "\\$a\\${b}\\\"\\\\\\n\\u0000\u{e9}"
    );
    assert_eq!(
        escape_c("a\"\\\n\u{0}\u{1}\u{e9}???".to_string()),
        "a\\\"\\\\\\n\\000\\001\\303\\251?\\?\\?"
    );
    assert_eq!(
        ecmascript_string_literal("it's\\\n\u{0}\u{2028}\u{e9}"),
        "'it\\'s\\\\\\n\\u0000\\u2028\u{e9}'"
    );
}
