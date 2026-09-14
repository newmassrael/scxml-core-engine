// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A value a template writes into a comment is encoded for that comment.
//!
//! Generated source echoes the document back in its comments — a `<log>`'s
//! `expr`, a transition's `cond`, a `<data>` initialiser — and that text is the
//! author's. Measured 2026-09-13, before the arrangement below existed: 963
//! interpolations across the template tree sat inside a comment of the language
//! their template emits, and none was encoded. Rendered with hostile values,
//! the C11 block comment that echoes a `<log>` element's `expr` closed at the
//! value's `*/` and put the rest of the expression into code, and the Go line
//! comment that echoes a `<data>` element's `expr` put the second line of a
//! value into code.
//!
//! # The generator encodes by context
//!
//! Templates do not write the encoder at each site; that is manual escaping, and
//! it is what had forgotten all 963. Every template enters an environment through
//! `generator::register_template`, which reads it with
//! `sce_build::template_lexing` and routes each value inside a comment of the
//! emitted language through `comment_text`. This gate holds the parts of that
//! arrangement that could break without anything else noticing:
//!
//! - [`every_template_enters_through_the_one_door`] — a template registered any
//!   other way renders its comment values raw.
//! - [`the_engine_finds_the_comments_the_templates_hold`] — the census, per
//!   emitted syntax, over the registrations the generator makes. A lexer that
//!   stopped seeing comments would encode nothing and report nothing.
//! - [`no_template_encodes_for_a_place_its_value_is_not_in`] — a literal's
//!   escaper inside a comment runs in addition to the comment's encoder, an
//!   explicit `comment_text` inside one says the engine is not trusted to do its
//!   job, and one outside a comment encodes for a place the value is not in.
//! - [`hostile_text_in_an_echoed_field_stays_out_of_code`] — the whole chain,
//!   rendered through all six backends.
//!
//! # What this gate does not hold
//!
//! - **A template that spells its comment delimiters at render time.** Which
//!   files do is derived ([`renders_for_several_backends`]), and each is named
//!   in [`PER_BACKEND_MACROS`] with what holds it instead.
//! - **A value written into a string literal.** Its encoder is the literal's
//!   escaper, which differs per language, and its gate is the sibling
//!   `a_value_written_into_a_string_literal_is_escaped`. The two share one
//!   hostile document ([`common::hostile_document`]) and differ in the hazards
//!   they put into it.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use common::hostile_document::{code_structure, generate, BACKENDS, MARKER};
use common::rust_source::code_only;
use common::source_lexing::{comments_blanked, language_of, structure_mask, Lang};
use sce_build::comment_text::FILTER;
use sce_build::template_lexing::{interpolations, tag_filters, Class, Syntax};

/// What a string-literal escaper's name starts with. Derived rather than
/// listed, so an escaper added for a new language is refused inside a comment
/// the day it is written.
const LITERAL_ESCAPER_PREFIX: &str = "escape_";

/// A template the static rules cannot read, and what holds it instead.
struct PerBackendMacro {
    path: &'static str,
    held_by: &'static str,
}

/// Every template [`renders_for_several_backends`] selects, each with the
/// argument for why reading around it is not a hole nobody knows about.
/// [`the_templates_the_rules_cannot_read_are_exactly_the_named_ones`] holds the
/// list to the derivation in both directions.
const PER_BACKEND_MACROS: &[PerBackendMacro] = &[
    PerBackendMacro {
        path: "tools/codegen/templates/_macros/sce_annotation_marker.jinja2",
        held_by: "spells its comment delimiters from a variable, so no reading of its \
                  text can see its comments and it writes `| comment_text` itself; \
                  sce_annotation_emission.rs::hostile_annotation_text_stays_inside_its_comment \
                  renders it through all six backends with every hazard",
    },
    PerBackendMacro {
        path: "tools/codegen/templates/_macros/sce_map_marker.jinja2",
        held_by: "writes a literal delimiter in each backend's branch, so the engine \
                  encodes it under the backend that renders that branch. Open, and \
                  registered in docs/SCE_ACCEPTED_SUBSET.md: the sourcemap readers that \
                  parse these lines back do not decode, and the same path also lands in \
                  string literals (`#line`, `#[doc]`)",
    },
];

/// Measured floors on how many values the engine finds inside comments, per
/// emitted syntax.
///
/// A lexer that stops seeing comments encodes nothing and this gate would
/// otherwise stay green. Each syntax carries its own floor, because a total
/// lets one language go blind while another's count keeps the sum respectable.
///
/// Measured 2026-09-13, at roughly 90 % of what the tree holds: CFamily 494,
/// Go 153, Kotlin 106, Python 50, Rust 253.
const FLOORS: &[(Syntax, usize)] = &[
    (Syntax::CFamily, 445),
    (Syntax::Go, 138),
    (Syntax::Kotlin, 95),
    (Syntax::Python, 45),
    (Syntax::Rust, 228),
];

use common::hostile_document::repo_root;

/// The generator binary, named HERE rather than in the shared helper.
///
/// `env!` expands at compile time, and `common` compiles into every target in
/// this directory — so a target that reached the binary through the helper
/// alone would both force the `cli` feature on the whole suite and hide the
/// reach from `cli_feature_gating`, which reads this expansion out of the
/// target's own source.
const CODEGEN: &str = env!("CARGO_BIN_EXE_sce-codegen");

use common::template_registration::registrations;

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Whether a template writes comment delimiters it chooses at render time.
///
/// The derivation is the macro's signature: a macro that takes the backend as a
/// parameter renders for several languages from one text.
fn renders_for_several_backends(source: &str) -> bool {
    let mut rest = source;
    while let Some(at) = rest.find("macro ") {
        let after = &rest[at + "macro ".len()..];
        let (Some(open), Some(close)) = (after.find('('), after.find(')')) else {
            break;
        };
        if open < close
            && after[open + 1..close].split(',').any(|param| {
                param
                    .split('=')
                    .next()
                    .is_some_and(|name| name.trim() == "backend")
            })
        {
            return true;
        }
        rest = &after[close.max(open) + 1..];
    }
    false
}

// ---------------------------------------------------------------------------
// The door
// ---------------------------------------------------------------------------

/// No template reaches a minijinja environment except through
/// `generator::register_template`.
///
/// Read from the library's code with comments stripped, so prose that names the
/// minijinja call is not counted.
#[test]
fn every_template_enters_through_the_one_door() {
    let src = repo_root().join("sce-build/src");
    let mut pending = vec![src.clone()];
    let mut calls: Vec<String> = Vec::new();
    let mut door_holds_it = false;
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).expect("read sce-build/src") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("read source");
            let code = code_only(&text);
            let door = code
                .find("pub fn register_template(")
                .map(|start| (start, start + code[start..].find("\n}\n").unwrap_or(0)));
            let mut from = 0usize;
            while let Some(found) = code[from..].find("add_template") {
                let at = from + found;
                let inside_the_door = door.is_some_and(|(start, end)| at > start && at < end);
                if inside_the_door {
                    door_holds_it = true;
                } else {
                    let line = code[..at].matches('\n').count() + 1;
                    calls.push(format!("  {}:{line}", relative(&repo_root(), &path)));
                }
                from = at + "add_template".len();
            }
        }
    }
    assert!(
        door_holds_it,
        "register_template no longer registers anything itself, so this guard is \
         looking for a door that is not there"
    );
    assert!(
        calls.is_empty(),
        "templates registered without generator::register_template:\n{}\n\n\
         A template that skips the door renders every value it writes into a comment \
         raw — the defect the door exists to remove.",
        calls.join("\n")
    );
}

// ---------------------------------------------------------------------------
// The census and the static rules
// ---------------------------------------------------------------------------

/// What a template gets wrong about where its value is encoded.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Wrong {
    /// A literal's escaper inside a comment, where the engine also encodes.
    LiteralEscaperInComment,
    /// An explicit `comment_text` inside a comment the engine can see.
    ExplicitWhereTheEngineEncodes,
    /// The comment encoder, somewhere that is not a comment.
    EncoderOutsideComment,
}

/// The single place the static rules are decided.
fn violations_in(source: &str, syntax: Syntax) -> Vec<(usize, String, Wrong)> {
    interpolations(source, syntax)
        .into_iter()
        .filter_map(|site| {
            let applied = tag_filters(&site.tag);
            let wrong = if site.context == Class::Comment {
                if applied
                    .iter()
                    .any(|f| f.starts_with(LITERAL_ESCAPER_PREFIX))
                {
                    Some(Wrong::LiteralEscaperInComment)
                } else if applied.iter().any(|f| f == FILTER) {
                    Some(Wrong::ExplicitWhereTheEngineEncodes)
                } else {
                    None
                }
            } else if applied.iter().any(|f| f == FILTER) {
                Some(Wrong::EncoderOutsideComment)
            } else {
                None
            };
            wrong.map(|w| (site.line, site.tag, w))
        })
        .collect()
}

/// The engine finds the comments the templates hold, in every syntax.
#[test]
fn the_engine_finds_the_comments_the_templates_hold() {
    let root = repo_root();
    let mut per_syntax: BTreeMap<String, usize> = BTreeMap::new();
    let mut counted: BTreeMap<Syntax, usize> = BTreeMap::new();
    for ((path, syntax), _) in registrations(&root) {
        let source = std::fs::read_to_string(&path).expect("read template");
        let in_comments = interpolations(&source, syntax)
            .iter()
            .filter(|site| site.context == Class::Comment)
            .count();
        *counted.entry(syntax).or_default() += in_comments;
        *per_syntax.entry(format!("{syntax:?}")).or_default() += in_comments;
    }
    println!("values the engine encodes inside comments, per emitted syntax: {per_syntax:?}");
    let short: Vec<String> = FLOORS
        .iter()
        .filter_map(|(syntax, floor)| {
            let seen = counted.get(syntax).copied().unwrap_or(0);
            (seen < *floor).then(|| format!("  {syntax:?}: {seen} found, floor {floor}"))
        })
        .collect();
    assert!(
        short.is_empty(),
        "the engine found fewer values inside comments than it is meant to:\n{}\n\n\
         A lexer that stopped seeing comments encodes nothing. Either it broke, or \
         the templates genuinely shed comment echoes and the floor should be \
         re-derived and lowered in the same commit.",
        short.join("\n"),
    );
}

/// No template encodes a value for a place the value is not in.
#[test]
fn no_template_encodes_for_a_place_its_value_is_not_in() {
    let root = repo_root();
    let exempt: BTreeSet<String> = PER_BACKEND_MACROS
        .iter()
        .map(|m| m.path.to_string())
        .collect();
    let mut wrong = BTreeSet::new();
    for ((path, syntax), _) in registrations(&root) {
        let rel = relative(&root, &path);
        if exempt.contains(&rel) {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("read template");
        for (line, tag, kind) in violations_in(&source, syntax) {
            wrong.insert(format!("  {rel}:{line} ({syntax:?}) {kind:?}: {tag}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} template value(s) encoded for the wrong place:\n{}\n\n\
         LiteralEscaperInComment / ExplicitWhereTheEngineEncodes: remove the filter; \
         the generator encodes every value inside a comment when the template is \
         registered. EncoderOutsideComment: the value is not in a comment, so the \
         comment encoder is the wrong one there.",
        wrong.len(),
        wrong.iter().cloned().collect::<Vec<_>>().join("\n"),
    );
}

/// The files the static rules skip are exactly the named ones, and each names
/// what holds it.
#[test]
fn the_templates_the_rules_cannot_read_are_exactly_the_named_ones() {
    let root = repo_root();
    let derived: BTreeSet<String> = registrations(&root)
        .into_keys()
        .map(|(path, _)| path)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|path| {
            renders_for_several_backends(&std::fs::read_to_string(path).expect("read template"))
        })
        .map(|path| relative(&root, &path))
        .collect();
    let named: BTreeSet<String> = PER_BACKEND_MACROS
        .iter()
        .map(|m| m.path.to_string())
        .collect();
    assert_eq!(
        derived, named,
        "the templates that render for several backends from one text are not the \
         ones PER_BACKEND_MACROS names. A new one is read by no static rule until it \
         is named with what holds it; a stale entry claims a hold nobody needs."
    );
    for m in PER_BACKEND_MACROS {
        assert!(
            m.held_by.split_whitespace().count() >= 8,
            "{} is exempted without an argument",
            m.path
        );
    }
    // The derivation must be able to say no, or the equality above would hold
    // for a predicate that selects every file.
    assert!(!renders_for_several_backends(
        "{%- macro emit(action, indent=\"\") -%}\n// {{ action.expr }}\n{%- endmacro -%}\n"
    ));
    assert!(renders_for_several_backends(
        "{%- macro marker(loc, backend, module_level=false) -%}{%- endmacro -%}\n"
    ));
}

/// The static rules, exercised in both directions over templates written here.
///
/// The sweep is meant to be green, so it can never show that the chain behind it
/// — lex, find the tag, read its class, parse its filters — still refuses
/// anything. Rows are paired: a lexer that called everything a comment passes
/// the comment rows and fails the literal and code rows, and one that called
/// nothing a comment does the reverse.
#[test]
fn the_rules_are_exercised_in_both_directions() {
    use Wrong::*;
    struct Case {
        syntax: Syntax,
        what: &'static str,
        source: &'static str,
        expected: &'static [Wrong],
    }
    let cases = [
        Case {
            syntax: Syntax::CFamily,
            what: "a plain value in a block comment is the engine's to encode",
            source: "/* <log expr=\"{{ e }}\"> */\n",
            expected: &[],
        },
        Case {
            syntax: Syntax::CFamily,
            what: "a literal escaper in a block comment",
            source: "/* <log expr=\"{{ e | escape_c }}\"> */\n",
            expected: &[LiteralEscaperInComment],
        },
        Case {
            syntax: Syntax::CFamily,
            what: "an explicit encoder the engine would have supplied",
            source: "// {{ e | comment_text }}\n",
            expected: &[ExplicitWhereTheEngineEncodes],
        },
        Case {
            syntax: Syntax::CFamily,
            what: "the comment encoder in a string literal",
            source: "const char *s = \"{{ x | comment_text }}\";\n",
            expected: &[EncoderOutsideComment],
        },
        Case {
            syntax: Syntax::CFamily,
            what: "the comment encoder in code",
            source: "int {{ x | comment_text }} = 0;\n",
            expected: &[EncoderOutsideComment],
        },
        Case {
            syntax: Syntax::CFamily,
            what: "a literal escaper in a string literal is that literal's business",
            source: "const char *s = \"{{ x | escape_c }}\";\n",
            expected: &[],
        },
        Case {
            syntax: Syntax::CFamily,
            what: "a // inside a tag opens no comment",
            source: "int a = {{ x | default(\"//\") | comment_text }};\n",
            expected: &[EncoderOutsideComment],
        },
        Case {
            syntax: Syntax::CFamily,
            what: "a tag in template prose renders nothing",
            source: "{# /* {{ x | escape_c }} */ #}\nint a;\n",
            expected: &[],
        },
        Case {
            syntax: Syntax::CFamily,
            what: "C block comments do not nest",
            source: "/* /* */ int {{ x | comment_text }}; /* */\n",
            expected: &[EncoderOutsideComment],
        },
        Case {
            syntax: Syntax::Kotlin,
            what: "Kotlin block comments do",
            source: "/* /* */ {{ x | escape_kotlin }} */\n",
            expected: &[LiteralEscaperInComment],
        },
        Case {
            syntax: Syntax::Rust,
            what: "a literal and a trailing line comment on one line",
            source: "let s = \"{{ x | escape_rust }}\"; // {{ y | escape_rust }}\n",
            expected: &[LiteralEscaperInComment],
        },
        Case {
            syntax: Syntax::Go,
            what: "a Go raw string is a literal",
            source: "s := `{{ x | comment_text }}`\n",
            expected: &[EncoderOutsideComment],
        },
        Case {
            syntax: Syntax::Python,
            what: "a hash after a literal",
            source: "s = \"{{ x }}\"  # {{ y | comment_text }}\n",
            expected: &[ExplicitWhereTheEngineEncodes],
        },
        Case {
            syntax: Syntax::Python,
            what: "a docstring is a literal, not a comment",
            source: "    \"\"\"{{ x | comment_text }}\"\"\"\n",
            expected: &[EncoderOutsideComment],
        },
    ];
    let mut failures = Vec::new();
    for case in &cases {
        let got: Vec<Wrong> = violations_in(case.source, case.syntax)
            .into_iter()
            .map(|(_, _, w)| w)
            .collect();
        if got != case.expected {
            failures.push(format!(
                "  {} ({:?}): expected {:?}, got {got:?}",
                case.what, case.syntax, case.expected
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "the decision behind the sweep is wrong:\n{}",
        failures.join("\n")
    );
}

// ---------------------------------------------------------------------------
// The rendered half
// ---------------------------------------------------------------------------

/// The characters that end, open or extend a comment in some backend.
#[derive(Clone, Copy, Debug)]
enum Hazard {
    ClosesABlock,
    OpensANestedBlock,
    BreaksALine,
    SplicesTheNextLine,
}

const HAZARDS: [Hazard; 4] = [
    Hazard::ClosesABlock,
    Hazard::OpensANestedBlock,
    Hazard::BreaksALine,
    Hazard::SplicesTheNextLine,
];

/// The value an ECMAScript expression field carries: `hostile` puts the hazard
/// in, the control puts inert characters of the same kind where it was, so the
/// two renderings differ in those characters and nothing else.
fn expression(field: &str, hazard: Hazard, hostile: bool) -> String {
    match hazard {
        Hazard::ClosesABlock => format!(
            "'{field}{}{MARKER}{field}'",
            if hostile { "*/" } else { "xx" }
        ),
        Hazard::OpensANestedBlock => format!(
            "'{field}{}{MARKER}{field}'",
            if hostile { "/*" } else { "xx" }
        ),
        // A string literal cannot hold a raw line break, so the break sits
        // between two operands.
        Hazard::BreaksALine => format!(
            "'{field}' +{}'{MARKER}{field}'",
            if hostile { "&#10;" } else { " " }
        ),
        // An expression cannot end in a bare backslash; free text carries it.
        Hazard::SplicesTheNextLine => format!("'{field} {MARKER}{field}'"),
    }
}

/// The value a free-text field carries — `<log label>`.
fn free_text(field: &str, hazard: Hazard, hostile: bool) -> String {
    match hazard {
        Hazard::ClosesABlock => format!(
            "{field}{}{MARKER}{field}",
            if hostile { "*/" } else { "xx" }
        ),
        Hazard::OpensANestedBlock => format!(
            "{field}{}{MARKER}{field}",
            if hostile { "/*" } else { "xx" }
        ),
        // A line break, now that a value written into a string literal is
        // escaped for it (`sce_build::literal_text`). Until 2026-09-14 this
        // arm carried a space instead, because C++ and Go wrote the label into
        // a literal unescaped and the resulting source did not compile — a
        // defect this gate is not about, carved out here rather than hidden.
        Hazard::BreaksALine => format!("{field}&#10;{MARKER}{field}"),
        Hazard::SplicesTheNextLine => {
            format!(
                "{MARKER}{field} {field}{}",
                if hostile { "\\" } else { "x" }
            )
        }
    }
}

/// A document echoing a hostile value from every field a backend comments on.
///
/// The document, the six backends and the structural comparison are
/// [`common::hostile_document`]'s: the string-literal door's gate runs the same
/// experiment with its own hazards, and two copies would be two answers to
/// what "the value stayed where it was written" means.
fn document(hazard: Hazard, hostile: bool) -> String {
    common::hostile_document::document(&free_text("LABEL", hazard, hostile), |f| {
        expression(f, hazard, hostile)
    })
}

/// Marker occurrences that sit inside a comment.
///
/// Counted per line, not per offset: every rendering keeps the source's lines,
/// but the Rust one drops a comment's text rather than blanking it, so an
/// offset into it is not an offset into the source. A marker the comment-free
/// line no longer carries was inside a comment.
fn markers_in_comments(source: &str, lang: Lang) -> usize {
    source
        .lines()
        .zip(comments_blanked(source, lang).lines())
        .map(|(line, code)| {
            line.matches(MARKER)
                .count()
                .saturating_sub(code.matches(MARKER).count())
        })
        .sum()
}

/// Hostile text in every field a backend echoes into a comment changes nothing
/// but comments.
///
/// Differential rather than a search for markers in code: a hostile document
/// and a control that differs only in the hazard characters are generated under
/// one basename, and every generated source file's structure — the text with
/// comments AND literals blanked — must be identical between the two. That
/// catches text leaving a comment and code a comment swallowed alike, and it is
/// indifferent to the markers legitimately appearing in string literals, where
/// every backend also writes the expression for its script engine.
///
/// ⚠ It is exercised, not assumed: each backend must show at least one marker
/// inside a comment in each rendering, or the echoes this is about were not
/// produced and a green would measure nothing.
#[test]
fn hostile_text_in_an_echoed_field_stays_out_of_code() {
    let scratch = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("a_value_written_into_a_comment");
    let _ = std::fs::remove_dir_all(&scratch);
    let mut failures = Vec::new();
    for hazard in HAZARDS {
        let mut exercised = 0usize;
        let mut census = Vec::new();
        for lang in BACKENDS {
            let base = scratch.join(format!("{hazard:?}")).join(lang);
            let hostile_dir = generate(
                CODEGEN,
                lang,
                &base.join("hostile"),
                &document(hazard, true),
            );
            let control_dir = generate(
                CODEGEN,
                lang,
                &base.join("control"),
                &document(hazard, false),
            );
            let mut names: Vec<String> = std::fs::read_dir(&hostile_dir)
                .expect("read generated output")
                .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
            let mut lexed = 0usize;
            let mut in_comments = 0usize;
            for name in names {
                let Some(file_lang) = language_of(&name) else {
                    continue;
                };
                lexed += 1;
                let hostile = std::fs::read_to_string(hostile_dir.join(&name))
                    .unwrap_or_else(|e| panic!("read {name}: {e}"));
                let control = std::fs::read_to_string(control_dir.join(&name))
                    .unwrap_or_else(|e| panic!("{lang}: the control rendered no {name}: {e}"));
                in_comments += markers_in_comments(&hostile, file_lang);
                if code_structure(&hostile, file_lang) != code_structure(&control, file_lang) {
                    let hostile_structure = structure_mask(&hostile, file_lang);
                    let leaked: Vec<String> = hostile_structure
                        .lines()
                        .zip(hostile.lines())
                        .enumerate()
                        .filter(|(_, (structure, _))| structure.contains(MARKER))
                        .map(|(n, (_, line))| format!("      {}: {}", n + 1, line.trim()))
                        .take(8)
                        .collect();
                    failures.push(format!(
                        "  {hazard:?} / {lang} / {name}: the hostile rendering's code differs \
                         from the control's. Hostile lines whose CODE carries a marker:\n{}",
                        if leaked.is_empty() {
                            "      (none — code was swallowed rather than injected)".to_string()
                        } else {
                            leaked.join("\n")
                        }
                    ));
                }
            }
            assert!(
                lexed > 0,
                "{hazard:?} / {lang}: no generated file was a source file this test can lex"
            );
            exercised += in_comments;
            census.push(format!("{lang}={in_comments}"));
        }
        // Not every backend echoes these fields into a comment — measured
        // 2026-09-13, C++, Kotlin and Python echo none of them — so the
        // exercise is demanded of the hazard, not of each backend. A hazard no
        // backend carried into a comment was not tested at all.
        println!(
            "{hazard:?}: markers inside comments, per backend: {}",
            census.join(" ")
        );
        if exercised == 0 {
            failures.push(format!(
                "  {hazard:?}: no marker reached a comment in any backend, so no echo was \
                 exercised and the clean result means nothing"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "author text reached generated code through a comment:\n{}",
        failures.join("\n")
    );
}

/// The differential sees a broken comment, and sees nothing else.
///
/// The rendered half is meant to be green, so on the tree its comparison can
/// never show that it would notice a leak. Every row here runs the same
/// [`code_structure`] over a pair written by hand: the leaking rows differ in
/// code the way a broken comment makes them differ — text injected after the
/// comment ends, or code swallowed by one that never does — and the inert rows
/// differ only inside comments and literals.
#[test]
fn the_differential_sees_a_broken_comment_and_nothing_else() {
    struct Pair {
        lang: Lang,
        what: &'static str,
        hostile: &'static str,
        control: &'static str,
        leaks: bool,
    }
    let pairs = [
        Pair {
            lang: Lang::CFamily,
            what: "a */ closes a C block comment early",
            hostile: "/* x A*/INJ */\nint a;\n",
            control: "/* x AxxINJ */\nint a;\n",
            leaks: true,
        },
        Pair {
            lang: Lang::CFamily,
            what: "a backslash-newline pulls the next C line into a comment",
            hostile: "// x A\\\nint a;\n",
            control: "// x Ax\nint a;\n",
            leaks: true,
        },
        Pair {
            lang: Lang::CFamily,
            what: "an encoded */ stays inside its comment",
            hostile: "/* x A*\\x2FINJ */\nint a;\n",
            control: "/* x AxxINJ */\nint a;\n",
            leaks: false,
        },
        Pair {
            lang: Lang::Go,
            what: "a line break ends a Go line comment",
            hostile: "// x A\nINJ()\n",
            control: "// x A INJ()\n",
            leaks: true,
        },
        Pair {
            lang: Lang::Kotlin,
            what: "a /* opens a nested Kotlin comment that swallows code",
            hostile: "/* x /*INJ */\nval a = 1\n",
            control: "/* x xxINJ */\nval a = 1\n",
            leaks: true,
        },
        Pair {
            lang: Lang::Rust,
            what: "a /* opens a nested Rust comment that swallows code",
            hostile: "/* x /*INJ */\nfn a() {}\n",
            control: "/* x xxINJ */\nfn a() {}\n",
            leaks: true,
        },
        Pair {
            lang: Lang::Rust,
            what: "a string literal may hold anything",
            hostile: "let s = \"A*/INJ\";\n",
            control: "let s = \"AxxINJ\";\n",
            leaks: false,
        },
        Pair {
            lang: Lang::Python,
            what: "a line break ends a Python comment",
            hostile: "# x A\nINJ = 1\n",
            control: "# x A INJ = 1\n",
            leaks: true,
        },
    ];
    let wrong: Vec<String> = pairs
        .iter()
        .filter(|p| {
            (code_structure(p.hostile, p.lang) != code_structure(p.control, p.lang)) != p.leaks
        })
        .map(|p| format!("  {} ({:?}): expected leaks={}", p.what, p.lang, p.leaks))
        .collect();
    assert!(
        wrong.is_empty(),
        "the differential behind the rendered half is wrong:\n{}",
        wrong.join("\n")
    );
    // The per-line marker count, on the one arm whose rendering is not
    // offset-aligned.
    assert_eq!(
        markers_in_comments("let a = 1; // INJ_A\nlet s = \"INJ_B\";\n", Lang::Rust),
        1
    );
}
