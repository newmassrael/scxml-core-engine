// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Where a comment ends and code begins, for the gates that sweep the tree.
//!
//! The lexer itself is the library's, [`sce_build::template_lexing`]: the
//! generator encodes every template value by the context that module answers,
//! and a copy here would be a second answer to where a comment ends. What
//! lives in this file is what only a sweep needs:
//!
//! - **which language a tracked path is written in** ([`language_of`]). The
//!   generator is asked about a template by its registered name and the
//!   backend that loaded it; a sweep holds repository paths and no backend, so
//!   it answers from the directory the path sits in, and
//!   `every_template_resolves_to_a_target_language` refuses a template in a
//!   directory this does not know;
//! - **Rust source files**, which [`super::rust_source`] already lexes — raw
//!   strings, lifetimes, nested comments — better than a template's view of
//!   Rust needs to.
//!
//! [`comments_blanked`] and [`structure_mask`] are the two renderings the
//! sweeps compare. Both preserve line numbering, so a line of either is a line
//! of the source. ⚠ Only the non-Rust arms preserve LENGTH: the Rust arm is
//! [`super::rust_source`]'s, which drops a comment's text instead of blanking
//! it, so an offset into a Rust rendering is not an offset into the source.

#![allow(dead_code)]

use super::rust_source::{code_mask, code_only};
use sce_build::template_lexing::{classify, classify_template};

pub use sce_build::template_lexing::{Class, Syntax};

/// Source extensions a sweep reads, each mapped to its lexical rules.
///
/// A file whose extension is absent is not read, and that is the honest half
/// of a coverage claim: a caller that pins how many files each language
/// contributes turns a language dropping out of the enumeration into a red
/// rather than a quieter green.
pub fn language_of(path: &str) -> Option<Lang> {
    let ext = path.rsplit('.').next()?;
    match ext {
        "rs" => Some(Lang::Rust),
        "c" | "h" | "cpp" | "hpp" | "cc" | "inl" => Some(Lang::CFamily),
        "go" => Some(Lang::Go),
        "kt" | "kts" => Some(Lang::Kotlin),
        "py" => Some(Lang::Python),
        "jinja2" => Some(target_language_of_template(path)),
        _ => None,
    }
}

/// What a swept file is written in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    Rust,
    CFamily,
    Go,
    Kotlin,
    Python,
    /// A template: `{# #}` on top of the emitted language's own rules.
    Template(Syntax),
}

/// The directory that says which language a template emits.
///
/// Derived rather than declared, because the template tree already says it:
/// every backend has its own directory.
const TEMPLATE_DIRS: &[(&str, Syntax)] = &[
    ("/forge/c/", Syntax::CFamily),
    ("/forge/cpp/", Syntax::CFamily),
    ("/forge/go/", Syntax::Go),
    ("/forge/kotlin/", Syntax::Kotlin),
    ("/forge/python/", Syntax::Python),
    ("/forge/rust/", Syntax::Rust),
    ("/mesh/cpp/", Syntax::CFamily),
    ("/templates/c/", Syntax::CFamily),
    ("/templates/go/", Syntax::Go),
    ("/templates/kotlin/", Syntax::Kotlin),
    ("/templates/python/", Syntax::Python),
    ("/templates/rust/", Syntax::Rust),
];

/// Where the C++ backend's own templates live. They predate the
/// per-language directories and so have none of their own.
const CPP_TEMPLATE_ROOTS: &[&str] = &[
    "tools/codegen/templates/_macros/",
    "tools/codegen/templates/actions/",
];

/// Which language a template emits, when its directory says.
fn declared_template_target(path: &str) -> Option<Syntax> {
    TEMPLATE_DIRS
        .iter()
        .find(|(segment, _)| path.contains(segment))
        .map(|(_, syntax)| *syntax)
}

/// Whether a template is one of the C++ backend's, which carry no directory
/// of their own: the templates directly in the templates root, and the two
/// shared trees beside them.
fn is_cpp_backend_template(path: &str) -> bool {
    if CPP_TEMPLATE_ROOTS.iter().any(|r| path.starts_with(r)) {
        return true;
    }
    path.strip_prefix("tools/codegen/templates/")
        .is_some_and(|rest| !rest.contains('/'))
}

/// Which language a template emits.
fn target_language_of_template(path: &str) -> Lang {
    Lang::Template(declared_template_target(path).unwrap_or(Syntax::CFamily))
}

/// Whether the derivation above knows this template, rather than having
/// fallen through to the C++ default by accident.
///
/// `path` is repository-relative, as `git ls-files` spells it.
pub fn template_target_is_declared(path: &str) -> bool {
    declared_template_target(path).is_some() || is_cpp_backend_template(path)
}

/// The class of every character of `source`, one entry per `char` — for every
/// language but Rust.
///
/// `None` for [`Lang::Rust`]: its lexer is [`super::rust_source`], whose
/// renderings drop a comment's text rather than blanking it, so no
/// per-character class can be read back from them. The first version of this
/// function read them back anyway, behind an assertion that the renderings
/// were as long as the source — which fails for every Rust file with a comment.
pub fn classes(source: &str, lang: Lang) -> Option<Vec<Class>> {
    Some(match lang {
        Lang::Rust => return None,
        Lang::CFamily => classify(source, Syntax::CFamily),
        Lang::Go => classify(source, Syntax::Go),
        Lang::Kotlin => classify(source, Syntax::Kotlin),
        Lang::Python => classify(source, Syntax::Python),
        Lang::Template(syntax) => classify_template(source, syntax),
    })
}

/// The source with every comment blanked and literals kept. Line numbering is
/// preserved in every arm, and length in every arm but Rust's. A docstring and
/// a toolchain directive count as comments here.
pub fn comments_blanked(source: &str, lang: Lang) -> String {
    match classes(source, lang) {
        None => code_only(source),
        Some(classes) => blank(source, &classes, |c| {
            matches!(c, Class::Comment | Class::DocLiteral | Class::Directive)
        }),
    }
}

/// The source with comments AND literals blanked: what is left is the
/// program's structure. Line numbering is preserved in every arm, and length
/// in every arm but Rust's.
pub fn structure_mask(source: &str, lang: Lang) -> String {
    match classes(source, lang) {
        None => code_mask(source),
        Some(classes) => blank(source, &classes, |c| c != Class::Code),
    }
}

/// `source` with every character whose class `blanked` selects turned into a
/// space. Newlines survive, so line numbering does too.
fn blank(source: &str, classes: &[Class], blanked: impl Fn(Class) -> bool) -> String {
    source
        .chars()
        .zip(classes)
        .map(|(ch, class)| {
            if blanked(*class) && ch != '\n' {
                ' '
            } else {
                ch
            }
        })
        .collect()
}
