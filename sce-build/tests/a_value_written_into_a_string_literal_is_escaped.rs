// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A value rendered inside a string literal is escaped for that literal.
//!
//! The sibling of `a_value_written_into_a_comment_is_encoded.rs`, holding the
//! same arrangement for the other class of place: one registration door, a
//! per-syntax census, no second escaper on a value that already carries one,
//! and hostile free text rendered through every syntax.
//!
//! What makes this row necessary rather than theoretical is measured and
//! recorded in `docs/SCE_ACCEPTED_SUBSET.md` §2.10: a `<log label>` holding a
//! line break was written unescaped into a C++ and a Go string literal, so
//! the emitted source did not compile.

mod common;

use common::hostile_document::{code_structure, document, generate, repo_root, BACKENDS, MARKER};
use common::source_lexing::language_of;
use sce_build::literal_text::{
    encode_template_literals, escaper_for, ALREADY_FIT_FOR_A_LITERAL, GUARD,
};
use sce_build::template_lexing::{interpolations, Class, Syntax};
use std::borrow::Cow;
use std::collections::BTreeSet;
use std::path::PathBuf;

/// The generator binary, named HERE rather than in the shared helper.
///
/// `env!` expands at compile time, and `common` compiles into every target in
/// this directory — so a target that reached the binary through the helper
/// alone would both force the `cli` feature on the whole suite and hide the
/// reach from `cli_feature_gating`, which reads this expansion out of the
/// target's own source.
const CODEGEN: &str = env!("CARGO_BIN_EXE_sce-codegen");

/// The door's answer for a template it accepts.
fn encoded(template: &str, syntax: Syntax) -> Cow<'_, str> {
    encode_template_literals(template, syntax)
        .unwrap_or_else(|sites| panic!("refused a template it should accept: {sites:?}"))
}

const SYNTAXES: &[(Syntax, &str)] = &[
    (Syntax::CFamily, "CFamily"),
    (Syntax::Go, "Go"),
    (Syntax::Kotlin, "Kotlin"),
    (Syntax::Python, "Python"),
    (Syntax::Rust, "Rust"),
];

/// Every syntax has an escaper, and no two share one by accident.
///
/// The escaper is what the door writes; a syntax whose escaper is another
/// syntax's would escape for the wrong language silently. `escape_c` and
/// `escape_cpp` ARE the same function, which is why the mapping is keyed on
/// syntax and the C family is one entry rather than two.
#[test]
fn every_syntax_names_an_escaper() {
    let mut seen = BTreeSet::new();
    for (syntax, name) in SYNTAXES {
        let filter = escaper_for(*syntax);
        assert!(!filter.is_empty(), "{name} names no string-literal escaper");
        assert!(
            ALREADY_FIT_FOR_A_LITERAL.contains(&filter),
            "{name}'s escaper {filter} is not itself declared fit for a \
             literal, so the door would wrap its own output a second time"
        );
        seen.insert(filter);
    }
    assert_eq!(
        seen.len(),
        SYNTAXES.len(),
        "every syntax must name its OWN escaper; got {seen:?}"
    );
}

// ⚠ The census that keeps `ALREADY_FIT_FOR_A_LITERAL` complete is NOT here.
// It lives in `filters::literal_escaper_census`, because deciding whether a
// filter escapes for a literal means asking what it DOES, and the functions
// are private to that module. The name cannot answer it — `escape_keyword`
// and `escape_go_keyword` share the prefix and only rename identifiers that
// collide with a language keyword — and neither can a scan of the bodies,
// since `escape_c` and `escape_cpp` forward to `escape_rust` and never
// mention a quote themselves.

/// A value that already carries an escaper is left exactly as written.
#[test]
fn a_site_that_already_escapes_is_left_alone() {
    for (syntax, name) in SYNTAXES {
        let filter = escaper_for(*syntax);
        let template = format!("x = \"{{{{ v | {filter} }}}}\";\n");
        let out = encoded(&template, *syntax);
        assert_eq!(
            out.as_ref(),
            template,
            "{name}: a site already applying {filter} was rewritten"
        );
    }
}

/// A value in a literal with no escaper gains exactly one.
#[test]
fn a_bare_value_in_a_literal_gains_its_syntax_escaper() {
    for (syntax, name) in SYNTAXES {
        let filter = escaper_for(*syntax);
        let out = encoded("x = \"{{ v }}\";\n", *syntax);
        assert!(
            out.contains(filter),
            "{name}: a bare value in a literal did not gain {filter}; got {out}"
        );
        assert_eq!(
            out.matches(filter).count(),
            1,
            "{name}: {filter} applied more than once; got {out}"
        );
    }
}

/// A value in CODE is not touched — escaping it there would corrupt it.
#[test]
fn a_value_outside_a_literal_is_left_alone() {
    for (syntax, name) in SYNTAXES {
        let template = "let x = {{ v }};\n";
        let out = encoded(template, *syntax);
        assert_eq!(
            out.as_ref(),
            template,
            "{name}: a value in code position was escaped for a literal"
        );
    }
}

/// The door's own filter binds to the WHOLE value, not to its last operand.
///
/// A filter binds tighter than every operator, so `{{ a ~ b | f }}` would
/// escape `b` and write `a` raw. The rewrite parenthesises for exactly this,
/// and the case is here because a template that concatenates inside a literal
/// is the one where getting it wrong is invisible: the output still compiles.
#[test]
fn a_concatenation_inside_a_literal_is_escaped_whole() {
    let syntax = Syntax::Rust;
    let out = encoded("x = \"{{ a ~ b }}\";\n", syntax);
    assert!(
        out.contains("(a ~ b)"),
        "the concatenation was not parenthesised before the filter; got {out}"
    );
}

/// A RAW literal is never escaped, in any syntax that has one.
///
/// ⚠ This is the case the first version of this door got wrong, and it failed
/// SILENTLY: escaping is what makes a value safe in an escapable literal and
/// what CORRUPTS it in a raw one, so `<data>` XML content came out of the Rust
/// and Go backends as `<books xmlns=\"\">\n …` — every backslash a literal
/// byte of the parsed document — and the generated source still compiled.
#[test]
fn a_value_in_a_raw_literal_is_never_escaped_for_one() {
    // Each syntax's raw form, spelled as that language spells it.
    let raws = [
        (Syntax::Go, "x := `{{ v }}`\n"),
        (Syntax::Rust, "let x = r#\"{{ v }}\"#;\n"),
        (Syntax::CFamily, "auto x = R\"xml({{ v }})xml\";\n"),
        (Syntax::Kotlin, "val x = \"\"\"{{ v }}\"\"\"\n"),
    ];
    for (syntax, template) in raws {
        let escaper = escaper_for(syntax);
        match encode_template_literals(template, syntax) {
            // Go's backtick is guarded rather than refused: the value passes
            // through, and only a value carrying a backtick is stopped.
            Ok(out) => assert!(
                !out.contains(escaper) && out.contains(GUARD),
                "{syntax:?}: a raw literal was routed through {escaper}; got {out}"
            ),
            // Every other raw form is refused at registration, naming the line.
            Err(sites) => {
                assert_eq!(sites.len(), 1, "{syntax:?}: {sites:?}");
                assert_eq!(sites[0].line, 1, "{syntax:?}: {sites:?}");
            }
        }
    }
}

/// The guard refuses exactly the value that would close the literal.
#[test]
fn the_guard_stops_the_character_that_closes_a_raw_literal() {
    assert!(
        sce_build::literal_text::guard("a plain label".to_string()).is_ok(),
        "a value with no backtick was refused"
    );
    assert!(
        sce_build::literal_text::guard("a `backtick` label".to_string()).is_err(),
        "a value carrying the sequence that closes a Go raw literal was let through"
    );
}

/// The templates this repository ships hold literal interpolations, per
/// syntax, and the door reaches them.
///
/// A census rather than a total: a change that stopped the lexer seeing one
/// syntax's literals would still leave the others looking busy, and the sweep
/// as a whole would report a healthy number.
#[test]
fn the_engine_finds_the_literals_the_templates_hold() {
    let files = shipped_templates();
    assert!(
        files.len() >= 200,
        "only {} template(s) tracked under {TEMPLATE_ROOT}; the walk has stopped reading them",
        files.len(),
    );

    for (syntax, name) in SYNTAXES {
        let mut in_literal = 0usize;
        for path in &files {
            let text = std::fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            in_literal += interpolations(&text, *syntax)
                .into_iter()
                .filter(|s| matches!(s.context, Class::Literal | Class::DocLiteral))
                .count();
        }
        // Every arm carries its own floor. Measured 2026-09-14: the smallest
        // syntax reading of this corpus finds well over a thousand.
        assert!(
            in_literal > 500,
            "{name}: only {in_literal} interpolation(s) read as sitting \
             inside a string literal; the classifier has stopped seeing them"
        );
    }
}

/// Model fields whose value is the AUTHOR's — the population this door exists
/// for, as against ids and derived names which §1 of the accepted subset
/// checks against W3C's grammar at parse.
///
/// The expression half is the generator's own list rather than a copy:
/// `MODEL_EXPRESSION_FIELDS` already answers "which fields carry text the
/// author wrote", and the Lua seam reads it for the same reason. What is added
/// here is the FREE-TEXT half, which that list has no cause to hold — nothing
/// evaluates a `<log label>`, which is exactly why no grammar constrains it
/// and why it was the measured breaking case.
fn author_text_fields() -> Vec<&'static str> {
    let mut fields = sce_build::generator::MODEL_EXPRESSION_FIELDS.to_vec();
    fields.extend([".label", ".src", ".namelist", ".name"]);
    fields
}

/// The census this door's documentation cites, DERIVED here rather than
/// remembered there.
///
/// ⚠ Over the registrations, not over the files. A template is read as the
/// language it emits, and one file can be registered by more than one backend
/// — so a walk that pairs each `.jinja2` with a syntax of its own choosing
/// counts a tree the generator never sees. The sweep above is deliberately the
/// other shape: it reads every file in every syntax, because its question is
/// whether the CLASSIFIER still sees literals at all, and a per-syntax floor
/// answers that where a single total would not.
///
/// The numbers are PRINTED rather than asserted equal. An exact count pinned
/// in a test is a number two places have to agree on, and the tree is the one
/// that gets to say — so this prints what it derived (`-- --nocapture`) and
/// asserts only the shape a reader's conclusion actually rests on: that free
/// text reaches a literal at all, and that the door leaves nothing in an
/// escapable literal unescaped.
#[test]
fn the_census_the_documentation_cites_is_derived_from_the_tree() {
    let root = repo_root();
    let registered = common::template_registration::registrations(&root);
    assert!(
        registered.len() >= 200,
        "only {} registration(s); the loaders have stopped answering",
        registered.len()
    );

    let (mut escapable, mut unescaped, mut raw, mut author_text) = (0usize, 0usize, 0usize, 0usize);
    for (path, syntax) in registered.keys() {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        // The door's OWN verdicts, not a second reading of the same files: a
        // count derived from a look-alike predicate is a count about the
        // look-alike.
        let plan = sce_build::literal_text::plan(&text, *syntax);
        let routed_to_an_escaper = plan
            .route
            .iter()
            .filter(|(_, filter)| *filter != sce_build::literal_text::GUARD)
            .count();
        escapable += plan.already_escaped.len() + routed_to_an_escaper;
        unescaped += routed_to_an_escaper;
        raw += plan.route.len() - routed_to_an_escaper + plan.refuse.len();
        let author_fields = author_text_fields();
        author_text += plan
            .route
            .iter()
            .filter(|(_, filter)| *filter != sce_build::literal_text::GUARD)
            .filter(|(site, _)| author_fields.iter().any(|field| site.tag.contains(field)))
            .count();
    }

    println!(
        "literal census over {} registration(s): {escapable} interpolation(s) \
         in an escapable literal, of which {unescaped} carried no escaper of \
         their own and {author_text} of those name a field the author writes; \
         {raw} in a raw literal",
        registered.len()
    );

    // Floors, not equalities. Each is a claim the door's documentation makes,
    // and each fails when the thing it describes stops existing.
    assert!(
        escapable > 500,
        "only {escapable} interpolation(s) land in an escapable literal; the \
         population this door exists for has gone"
    );
    assert!(
        unescaped > 0,
        "every literal interpolation already carries an escaper of its own, so \
         the door adds nothing — either the templates went back to per-site \
         filters, or this census stopped reading their tags"
    );
    assert!(
        raw > 0,
        "no interpolation reads as landing in a raw literal, so the half of \
         this door that tells the two apart is measured by nothing"
    );
    assert!(
        author_text > 0,
        "no site the door escapes names a field the author writes, so every \
         one of them is a derived name the parse-time grammar already holds — \
         and this door would be guarding a population that does not exist"
    );
}

const TEMPLATE_ROOT: &str = "tools/codegen/templates";

/// Every template the repository tracks under [`TEMPLATE_ROOT`].
///
/// Tracked rather than walked: the claim is about what ships, and a walk of
/// the directory also read whatever an editor or a half-finished
/// regeneration had left beside the templates.
fn shipped_templates() -> Vec<PathBuf> {
    let root = repo_root();
    common::repository::paths_git_tracks(&[TEMPLATE_ROOT])
        .into_iter()
        .filter(|p| p.ends_with(".jinja2"))
        .map(|p| root.join(p))
        .collect()
}

// ─────────────────────────────────────────────────────────────────────
// The end-to-end half: a document, six backends, and what came out.
// ─────────────────────────────────────────────────────────────────────

/// What a value can do to the string literal it is written into.
#[derive(Clone, Copy, Debug)]
enum Hazard {
    /// A quote closes the literal, and what follows it is code.
    ClosesTheLiteral,
    /// A backslash escapes the quote that was to close the literal, so the
    /// literal runs on into the rest of the line.
    EscapesTheClosingQuote,
    /// A line break: no backend's ordinary literal may carry one, so an
    /// unescaped one does not compile. This is the case
    /// `docs/SCE_ACCEPTED_SUBSET.md` §2.10 records as measured.
    BreaksTheLine,
    /// Kotlin reads `$` inside a literal as the start of a template
    /// expression, so an unescaped one is a reference to a name that is not
    /// there — a literal broken without a quote in sight.
    OpensATemplateExpression,
}

const HAZARDS: [Hazard; 4] = [
    Hazard::ClosesTheLiteral,
    Hazard::EscapesTheClosingQuote,
    Hazard::BreaksTheLine,
    Hazard::OpensATemplateExpression,
];

/// The hazard characters, and the inert ones the control puts in their place.
///
/// Same length and same position in both, so the two documents differ in what
/// the characters MEAN and in nothing else.
fn hazard_chars(hazard: Hazard, hostile: bool) -> &'static str {
    match (hazard, hostile) {
        (Hazard::ClosesTheLiteral, true) => "&quot;",
        (Hazard::EscapesTheClosingQuote, true) => "\\",
        (Hazard::BreaksTheLine, true) => "&#10;",
        (Hazard::OpensATemplateExpression, true) => "$x",
        (_, false) => "z",
    }
}

/// A free-text value carrying the hazard — `<log label>`, which no grammar
/// constrains.
///
/// ⚠ A backslash goes at the END and the others in the MIDDLE, and the
/// difference is the whole hazard rather than a detail of layout: a backslash
/// anywhere else escapes an ordinary letter, which every compiler here accepts.
/// It is the one immediately before the closing quote that escapes THAT quote
/// and lets the literal run on into the code. Measured while falsifying this
/// gate: with the door removed, a mid-text backslash changed nothing and the
/// gate stayed green.
fn free_text(hazard: Hazard, hostile: bool) -> String {
    let chars = hazard_chars(hazard, hostile);
    match hazard {
        Hazard::EscapesTheClosingQuote => format!("LABEL{MARKER}LABEL{chars}"),
        _ => format!("LABEL{chars}{MARKER}LABEL"),
    }
}

/// An ECMAScript expression carrying the hazard inside a string of its own.
///
/// The hazard sits INSIDE the ECMAScript string literal, because that is where
/// an author's text is: the expression around it must stay a valid expression
/// or the document would be refused for a reason that is not this gate's.
fn expression(field: &str, hazard: Hazard, hostile: bool) -> String {
    match hazard {
        // A line break cannot sit inside an ECMAScript string literal either,
        // so it goes between two operands, as the comment gate's does.
        Hazard::BreaksTheLine => format!(
            "'{field}' +{}'{MARKER}{field}'",
            if hostile { "&#10;" } else { " " }
        ),
        // A trailing backslash would escape the expression's own closing
        // quote; it is the free-text field that carries this one.
        Hazard::EscapesTheClosingQuote => format!("'{field} {MARKER}{field}'"),
        _ => format!("'{field}{}{MARKER}{field}'", hazard_chars(hazard, hostile)),
    }
}

/// What a language forbids INSIDE a `"`-delimited literal, and what it would
/// mean if it were there.
///
/// ⚠ The structural differential cannot see either of these, which is why they
/// are a second oracle rather than a second hazard. A raw line break inside a
/// literal leaves the lexer's idea of where that literal ends exactly where it
/// was — the next `"` still closes it — so the hostile and control renderings
/// mask to the same structure while only one of them compiles. A Kotlin `$` is
/// worse: it opens a TEMPLATE EXPRESSION, so the value escapes into code with
/// no quote in sight at all.
fn forbidden_in_a_literal(syntax: Syntax, body: &str) -> Option<String> {
    if body.contains('\n') && syntax != Syntax::Rust {
        // Rust is the one syntax whose `"` literal may span lines, so a break
        // there is not a defect and claiming it would be a false red.
        return Some("a raw line break, which no literal of this syntax may carry".into());
    }
    if syntax == Syntax::Kotlin {
        let chars: Vec<char> = body.chars().collect();
        for (i, c) in chars.iter().enumerate() {
            if *c != '$' {
                continue;
            }
            // An escaped `$` is `\$`; count the run before it, because `\\$`
            // is an escaped backslash followed by a live `$`.
            let escaped = chars[..i].iter().rev().take_while(|p| **p == '\\').count() % 2 == 1;
            let opens = chars
                .get(i + 1)
                .is_some_and(|n| n.is_alphabetic() || *n == '_' || *n == '{');
            if !escaped && opens {
                return Some("an unescaped `$`, which opens a Kotlin template expression".into());
            }
        }
    }
    None
}

/// The `"`-delimited literal bodies of a generated file, in order.
///
/// Triple-quoted spans are skipped: they carry a line break legitimately, and
/// the rule above is about the form that cannot.
fn literal_bodies(source: &str, syntax: Syntax) -> Vec<String> {
    let chars: Vec<char> = source.chars().collect();
    let classes = sce_build::template_lexing::classify(source, syntax);
    let mut bodies = Vec::new();
    let mut at = 0usize;
    while at < classes.len() {
        if classes[at] != Class::Literal {
            at += 1;
            continue;
        }
        let start = at;
        while at < classes.len() && classes[at] == Class::Literal {
            at += 1;
        }
        let span: String = chars[start..at].iter().collect();
        if span.starts_with("\"\"\"") || span.starts_with("'''") {
            continue;
        }
        bodies.push(
            span.trim_start_matches(['"', '\''])
                .trim_end_matches(['"', '\''])
                .to_string(),
        );
    }
    bodies
}

/// Hostile free text in every echoed field changes nothing but literals and
/// comments, in all six backends.
///
/// The differential is [`common::hostile_document`]'s and the reasoning is its
/// module documentation's: a search for markers in code reports nothing when
/// the value SWALLOWED the code that followed it, which is exactly what an
/// unterminated string literal does.
///
/// ⚠ This is the half that a green row predicate cannot stand in for. The
/// ledger's C2 predicate asks whether the door exists and whether one Go site
/// still reads `action.label` bare; neither question is answered by generating
/// a document, and neither answers this one.
#[test]
fn hostile_free_text_stays_inside_the_literal_in_every_backend() {
    let scratch =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("a_value_written_into_a_string_literal");
    let _ = std::fs::remove_dir_all(&scratch);
    let mut failures = Vec::new();
    for hazard in HAZARDS {
        for lang in BACKENDS {
            let base = scratch.join(format!("{hazard:?}")).join(lang);
            let hostile_dir = generate(
                CODEGEN,
                lang,
                &base.join("hostile"),
                &document(&free_text(hazard, true), |f| expression(f, hazard, true)),
            );
            let control_dir = generate(
                CODEGEN,
                lang,
                &base.join("control"),
                &document(&free_text(hazard, false), |f| expression(f, hazard, false)),
            );
            let mut names: Vec<String> = std::fs::read_dir(&hostile_dir)
                .expect("read generated output")
                .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
            let mut lexed = 0usize;
            let mut literals = 0usize;
            for name in &names {
                let Some(file_lang) = language_of(name) else {
                    continue;
                };
                lexed += 1;
                let hostile = std::fs::read_to_string(hostile_dir.join(name))
                    .unwrap_or_else(|e| panic!("read {name}: {e}"));
                let control = std::fs::read_to_string(control_dir.join(name))
                    .unwrap_or_else(|e| panic!("{lang}: the control rendered no {name}: {e}"));
                if code_structure(&hostile, file_lang) != code_structure(&control, file_lang) {
                    failures.push(format!(
                        "  {hazard:?} / {lang} / {name}: the hostile rendering's code \
                         differs from the control's, so the value left the literal it \
                         was written into."
                    ));
                }
                let syntax = match file_lang {
                    common::source_lexing::Lang::Rust => Syntax::Rust,
                    common::source_lexing::Lang::CFamily => Syntax::CFamily,
                    common::source_lexing::Lang::Go => Syntax::Go,
                    common::source_lexing::Lang::Kotlin => Syntax::Kotlin,
                    common::source_lexing::Lang::Python => Syntax::Python,
                    common::source_lexing::Lang::Template(_) => continue,
                };
                for body in literal_bodies(&hostile, syntax) {
                    literals += 1;
                    if let Some(why) = forbidden_in_a_literal(syntax, &body) {
                        failures.push(format!(
                            "  {hazard:?} / {lang} / {name}: a literal carries {why}: {body:?}"
                        ));
                    }
                }
            }
            // A floor per backend: an empty output directory compares equal to
            // another empty one, and a file with no literal passes the second
            // oracle by having nothing to read.
            assert!(
                lexed >= 1,
                "{hazard:?} / {lang}: no generated file was lexed; \
                 the probe produced {names:?}"
            );
            assert!(
                literals >= 1,
                "{hazard:?} / {lang}: no string literal was read out of the \
                 generated source; the second oracle measured nothing"
            );
        }
    }
    assert!(
        failures.is_empty(),
        "hostile free text escaped its string literal:\n{}",
        failures.join("\n")
    );
}

/// No template this repository ships writes a value into a raw literal the
/// door cannot make safe.
///
/// The refusal is a template defect and the generator reports it at
/// registration — which means the first thing to notice it would otherwise be
/// a backend failing to generate anything at all. Reading it here names the
/// template and the line instead.
#[test]
fn no_shipped_template_writes_into_an_inadmissible_raw_literal() {
    let files = shipped_templates();
    let mut refused = Vec::new();
    let mut read = 0usize;
    for path in &files {
        let name = path
            .strip_prefix(repo_root())
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned();
        let Some(lang) = language_of(&name) else {
            continue;
        };
        let common::source_lexing::Lang::Template(syntax) = lang else {
            continue;
        };
        read += 1;
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        if let Err(sites) = encode_template_literals(&text, syntax) {
            for site in sites {
                refused.push(format!("  {name}: {site}"));
            }
        }
    }
    assert!(
        read >= 200,
        "only {read} template(s) were read; the walk has stopped finding them"
    );
    assert!(
        refused.is_empty(),
        "template(s) write a value into a raw string literal:\n{}",
        refused.join("\n")
    );
}
