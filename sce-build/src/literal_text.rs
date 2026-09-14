// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Text placed inside a string literal in generated source.
//!
//! # What was wrong
//!
//! A comment has one encoder ([`crate::comment_text`]) applied at one door. A
//! string literal's encoder is the emitted language's own escaper, and
//! templates applied one at some sites and not at others — so whether an
//! author's text was safe depended on whether the author of that template
//! line remembered.
//!
//! ⚠ The census is DERIVED, by
//! `a_value_written_into_a_string_literal_is_escaped::the_census_the_documentation_cites_is_derived_from_the_tree`,
//! and printed by that test rather than only recorded here — a number a
//! document keeps is a number nothing re-measures. Read 2026-09-14, over the
//! 282 `(template, syntax)` pairs the loaders register: **1868**
//! interpolations land in an escapable literal, **1435** of them carrying no
//! escaper of their own, and a further **52** land in a raw literal.
//!
//! Most of those 1435 are identifiers, which [`crate::scxml_identifier`] has
//! checked against W3C's grammar at parse since 2026-09-14, so they cannot
//! carry a quote or a line break. The remainder is what this module exists
//! for: **349** sites whose value is FREE TEXT or an EXPRESSION — `cond`,
//! `expr`, `location`, `<log label>`, `src`, `namelist` — which no grammar
//! constrains and which W3C does not let SCE constrain.
//!
//! The measured breaking case: a `<log label>` holding a line break was
//! written unescaped into a C++ `SCE_LOG_INFO("…")` and a Go
//! `fmt.Println("…")` literal, so the emitted source did not compile.
//!
//! # Why a door and not a filter per site
//!
//! The same reason [`crate::comment_text::encode_template_comments`] gives:
//! where a value lands is a fact about the TEMPLATE'S TEXT and not about the
//! value, and it is the same fact for every render. A per-site filter is a
//! fact an author restates by hand at each of two thousand sites, and the
//! defect is precisely the site where they did not. Reading the template once
//! at registration and rewriting it makes the guarantee a property of the
//! door every template already passes through — `generator::register_template`.
//!
//! # A raw literal is the other half, and escaping is wrong there
//!
//! Go's `` `…` ``, Rust's `r#"…"#`, C++'s `R"d(…)d"` and Kotlin's `"""` process
//! no escape sequence, so a value escaped for one is CORRUPTED rather than
//! protected: `\"` written into a Go raw string is a backslash and a quote,
//! and `\n` is a backslash and an `n`. Measured 2026-09-14, on the first
//! version of this door: `<data>` XML content came out of the Rust and Go
//! backends as `<books xmlns=\"\">\n  …`, wrong in the generated binary and
//! silent at compile time.
//!
//! What a raw literal needs instead is that the value not carry the sequence
//! that CLOSES it, and there are only two honest answers to that:
//!
//! - **Guard the value** where the closing sequence is one character free text
//!   all but never carries. Go's backtick is the only such form in this tree,
//!   and it holds the seventy sites that put Lua source into readable Go, so
//!   [`GUARD`] passes every value through untouched and refuses the one that
//!   carries a backtick, naming it.
//! - **Refuse the template** where it is not. Rust's `"#`, C++'s `)d"` and
//!   Kotlin's `"""`-and-`$` all appear in ordinary free text — an XML
//!   attribute reading `href="#top"` closes a Rust raw string — so a guard
//!   there would refuse documents that are not hostile at all. An
//!   interpolation inside one of those is a defect in the TEMPLATE, caught
//!   once at registration, and its repair is to write an escapable literal
//!   and let this door escape it.
//!
//! # What it does NOT fix, stated rather than hidden
//!
//! ⚠ A literal that is a FORMAT STRING needs more than this. `SCE_LOG_*`
//! expands to `fmt`, where `{` and `}` are a replacement field, and Go's
//! `fmt.Printf` reads `%` as a verb. Escaping for the LITERAL leaves both
//! alone, correctly — `\{` is not how either is escaped. A value must
//! therefore not be spliced into a format string at all; it belongs in an
//! ARGUMENT, where this module's escaping is the whole of what it needs.
//! [`crate::filters::escape_cpp_format`] exists for the sites that still
//! splice, and is the stronger filter this door defers to when a template
//! applies it.

use std::borrow::Cow;
use std::collections::BTreeSet;
use std::fmt;

use crate::template_lexing::{
    applies_last_to_the_whole_value, interpolations, routed_through, set_bindings,
    tag_renders_bare_name, without_template_prose, Class, Syntax,
};

/// Filters whose output is already fit to sit inside a string literal.
///
/// A site applying one of these last is left alone: wrapping it again would
/// escape the escape, turning a rendered `\n` back into a literal backslash
/// and an `n`.
///
/// ⚠ This is a DECLARED set, not a heuristic over names, because the two
/// groups in it are safe for opposite reasons and a name cannot tell them
/// apart. `escape_*` escapes a value FOR a literal. The `*_string_expr` and
/// `*_literal` group returns a COMPLETE expression, quotes included, which is
/// a value that was never inside the literal's quotes to begin with.
/// `sce-build/tests/a_value_written_into_a_string_literal_is_escaped.rs`
/// holds the census that stops the set going stale.
pub const ALREADY_FIT_FOR_A_LITERAL: &[&str] = &[
    // Escape a value for the literal it lands in.
    "escape_c",
    "escape_cpp",
    "escape_cpp_format",
    "escape_go",
    "escape_json_string",
    "escape_kotlin",
    "escape_lua",
    "escape_python",
    "escape_rust",
    // Return a complete expression, delimiters included.
    "py_string_literal",
    "to_go_literal",
    "to_go_string_expr",
    "to_kotlin_string_expr",
    "to_python_const",
    "to_rust_literal",
    "to_rust_string_expr",
    "to_script_source_expr",
    // Lower an expression into a scripting language's own source text, which
    // carries its own quoting rules and is not this door's to second-guess.
    "to_lua_assign_content",
    "to_lua_data_content",
    "to_lua_expr",
    "to_lua_guard",
    "to_lua_location",
    "to_lua_script",
];

/// The filter that escapes a value for a string literal of `syntax`.
///
/// Keyed on [`Syntax`] rather than on `Language` because the escaping a
/// literal needs is a property of the SYNTAX: `escape_c` and `escape_cpp` are
/// the same function, and the one thing that separates the five is Kotlin's
/// `$`, which starts a template expression inside its literals.
pub fn escaper_for(syntax: Syntax) -> &'static str {
    match syntax {
        Syntax::CFamily => "escape_cpp",
        Syntax::Go => "escape_go",
        Syntax::Kotlin => "escape_kotlin",
        Syntax::Python => "escape_python",
        Syntax::Rust => "escape_rust",
    }
}

/// The name [`guard`] is registered under, and the one the rewrite writes at a
/// raw-literal site.
pub const GUARD: &str = "raw_literal_text";

/// The raw-literal form a template may still interpolate into, and what closes
/// it.
///
/// A syntax absent from this table has no admissible raw form, and an
/// interpolation inside one is refused at registration — see the module docs
/// for why that is the honest answer rather than a stricter one.
///
/// ⚠ [`guard`] refuses every terminator in this table, not the one its own
/// syntax names, and that is deliberate: a filter is registered on an
/// environment by name and sees only the value, never which template is
/// rendering. With one entry the two readings coincide. A second entry would
/// make the guard refuse a Go backtick in a Kotlin literal — conservative, but
/// wrong enough to be worth carrying the terminator on the interpolation
/// instead, which is the change to make then rather than now.
const GUARDED_RAW_FORMS: &[(Syntax, &str)] = &[(Syntax::Go, "`")];

/// What closes `syntax`'s guarded raw literal, if it has one.
fn guarded_terminator(syntax: Syntax) -> Option<&'static str> {
    GUARDED_RAW_FORMS
        .iter()
        .find(|(s, _)| *s == syntax)
        .map(|(_, t)| *t)
}

/// A template that writes a value into a raw literal this door cannot make
/// safe.
///
/// Carries where, so the repair is a line the author can open rather than a
/// search: `register_template` adds the template's name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawLiteralSite {
    /// 1-based line of the interpolation, as the template's author numbers it.
    pub line: usize,
    /// The tag as written.
    pub tag: String,
    /// The syntax the template emits.
    pub syntax: Syntax,
}

impl fmt::Display for RawLiteralSite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "line {}: {} is written into a raw string literal, which processes \
             no escape sequence — escaping the value there would corrupt it, \
             and leaving it unescaped lets it close the literal. Write an \
             escapable literal instead ({}) and this door escapes the value \
             for it.",
            self.line,
            self.tag,
            match self.syntax {
                Syntax::Rust => "\"…\" rather than r#\"…\"#",
                Syntax::CFamily => "\"…\" rather than R\"d(…)d\"",
                Syntax::Kotlin => "\"…\" rather than \"\"\"…\"\"\"",
                Syntax::Go => "\"…\" rather than `…`",
                Syntax::Python => "\"…\"",
            }
        )
    }
}

/// Whether `tag` already ends in a filter that makes it fit for a literal.
///
/// Public because the census in
/// `sce-build/tests/a_value_written_into_a_string_literal_is_escaped.rs`
/// counts the sites this door adds an escaper to, and a count that asked a
/// look-alike predicate — `tag.contains(filter)`, say — would report a
/// different population from the one the door actually acts on. There is one
/// answer to "does this site already escape", and it is this function.
pub fn already_fit(tag: &str) -> bool {
    ALREADY_FIT_FOR_A_LITERAL
        .iter()
        .any(|f| applies_last_to_the_whole_value(tag, f))
}

/// Names a `{% set %}` bound to a value that is already fit for a literal.
///
/// ⚠ Without this the door reads only the interpolation's own filter chain,
/// and a template that HOISTS its escaper into a binding escapes twice.
/// Measured 2026-09-14 in `c/scriptengine.jinja2`, which does it both ways
/// twenty lines apart: `"{{ item | to_lua_location | escape_c }} = nil"` in
/// one macro, and `{% set item_loc = item | to_lua_location | escape_c %}`
/// followed by `"{{ item_loc }} = …"` in the next. The second came out of
/// test457 as `(error(\\\"…\\\"))`, and the artifact then carried a refusal
/// message no reader could match against the one the walker reports.
///
/// A fixpoint, because a template is free to bind one alias from another and
/// a scan that followed a single hop would lose the value at the second — the
/// same reason the Lua-seam scan's `laundering_alias_names` is one.
fn names_bound_to_a_fit_value(template: &str) -> BTreeSet<String> {
    // Jinja's own prose is blanked first: a `{% set %}` inside `{# … #}`
    // binds nothing, and reading one would declare a site fit on the strength
    // of a comment.
    let text = without_template_prose(template);
    let lines: Vec<&str> = text.lines().collect();
    let bindings = set_bindings(&lines);
    let mut fit: BTreeSet<String> = BTreeSet::new();
    loop {
        let before = fit.len();
        for binding in &bindings {
            if fit.contains(&binding.name) {
                continue;
            }
            let as_tag = format!("{{{{{}}}}}", binding.value);
            let inherits = tag_renders_bare_name(&as_tag).is_some_and(|n| fit.contains(&n));
            if already_fit(&as_tag) || inherits {
                fit.insert(binding.name.clone());
            }
        }
        if fit.len() == before {
            return fit;
        }
    }
}

/// Refuse a value that carries the sequence closing the raw literal it is
/// written into; pass every other value through untouched.
///
/// Untouched is the whole point: a raw literal reproduces its bytes, so the
/// value is already correct there. What it cannot survive is the closing
/// sequence, and no escape exists for that inside the literal — so the only
/// answer left is to say so, loudly, at the document that carries it.
pub fn guard(value: String) -> Result<String, minijinja::Error> {
    for (_, terminator) in GUARDED_RAW_FORMS {
        if value.contains(terminator) {
            return Err(minijinja::Error::new(
                minijinja::ErrorKind::InvalidOperation,
                format!(
                    "a value written into a raw string literal contains {terminator}, \
                     which closes it, and a raw literal admits no escape for it: {value:?}"
                ),
            ));
        }
    }
    Ok(value)
}

/// Route every interpolation landing inside a string literal through the
/// encoding that literal needs — the escaper for an escapable one, [`guard`]
/// for a raw one this door admits.
///
/// Returns the template borrowed and byte-identical when there is nothing to
/// route, so a template with no literal interpolation costs nothing. Fails,
/// naming every site at once, when a template writes into a raw literal that
/// has no admissible encoding: reporting them one per run would make a
/// template with four such sites take four runs to repair.
pub fn encode_template_literals(
    template: &str,
    syntax: Syntax,
) -> Result<Cow<'_, str>, Vec<RawLiteralSite>> {
    let plan = plan(template, syntax);
    if !plan.refuse.is_empty() {
        return Err(plan.refuse);
    }
    if plan.route.is_empty() {
        return Ok(Cow::Borrowed(template));
    }
    let chars: Vec<char> = template.chars().collect();
    let mut out = String::with_capacity(template.len() + plan.route.len() * 16);
    let mut at = 0usize;
    for (site, filter) in &plan.route {
        out.extend(&chars[at..site.start]);
        out.push_str(&routed_through(&site.tag, filter));
        at = site.end;
    }
    out.extend(&chars[at..]);
    Ok(Cow::Owned(out))
}

/// What the door decided about one template, before it rewrote anything.
///
/// Published so a census can count what the door DID rather than re-deriving
/// it from a look-alike predicate. A number about this door that was computed
/// by a second reading of the same templates is a number about the second
/// reading.
#[derive(Debug, Default)]
pub struct Plan {
    /// Sites to route, each with the filter to route it through.
    pub route: Vec<(crate::template_lexing::Interpolation, &'static str)>,
    /// Sites in an escapable literal that already carry an escaper of their
    /// own — at the interpolation, or at the `{% set %}` that bound the value.
    pub already_escaped: Vec<crate::template_lexing::Interpolation>,
    /// Sites in a raw literal this door cannot make safe.
    pub refuse: Vec<RawLiteralSite>,
}

/// Read `template` and decide what each of its literal interpolations needs.
///
/// One walk, one verdict per site, and the only place those verdicts are
/// made. [`encode_template_literals`] acts on it; the gate counts it.
pub fn plan(template: &str, syntax: Syntax) -> Plan {
    let escaper = escaper_for(syntax);
    let guarded = guarded_terminator(syntax).is_some();
    let fit_names = names_bound_to_a_fit_value(template);
    let fit_here = |tag: &str| {
        already_fit(tag) || tag_renders_bare_name(tag).is_some_and(|n| fit_names.contains(&n))
    };
    let mut plan = Plan::default();
    for site in interpolations(template, syntax) {
        match site.context {
            Class::Literal | Class::DocLiteral => {
                if fit_here(&site.tag) {
                    plan.already_escaped.push(site);
                } else {
                    plan.route.push((site, escaper));
                }
            }
            // A raw literal takes the guard whatever filters the tag already
            // applies: the guard rewrites nothing, so it cannot double-encode,
            // and a value an earlier filter escaped can still carry the
            // character that closes this literal.
            Class::RawLiteral if guarded => {
                if !applies_last_to_the_whole_value(&site.tag, GUARD) {
                    plan.route.push((site, GUARD));
                }
            }
            Class::RawLiteral => plan.refuse.push(RawLiteralSite {
                line: site.line,
                tag: site.tag.clone(),
                syntax,
            }),
            _ => {}
        }
    }
    plan
}
