// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! §scxml-5.3: the one place a `<data>` id becomes a target-language
//! identifier.
//!
//! A typed reader is named after the variable it reads, in each backend's
//! own case convention. `<data id>` is an XML name and the data model is an
//! ECMAScript one, so the author may write a name that is legal there and
//! not in the language a backend emits: `box` is a Rust keyword, `object`
//! a Kotlin one, `pass` a Python one, `auto` a C++ one. A reader may also
//! land on a name the generated type already carries — `new` on a Rust
//! policy, `start` on a Kotlin machine, `t` beside C11's `<name>_t` typedef
//! — or on the same spelling as another variable once the case convention
//! has folded `a_b`, `a-b` and `aB` together.
//!
//! Each template used to spell the name itself, and none of them asked any
//! of this, so such a document generated code that did not compile — or,
//! in Python, silently replaced one method with another.
//!
//! The answer is decided here, once:
//!
//! - **Spelling** is per backend. Where the language has an idiomatic
//!   escape for a keyword it is used — Rust `r#`, Kotlin backticks, a
//!   trailing `_` in Python (PEP 8) — because an escaped reader is still
//!   the reader of that variable.
//! - **Existence** is not per backend. A variable that no escape can give a
//!   reader in some backend gets none in any, and says why in the
//!   manifest, for the reason `analyzer::readable_variables` states: a
//!   reader's existence must not depend on which backend a deployment
//!   generated.
//!
//! The names a generated type already carries are read off the templates
//! that emit it and the runtime type it extends, not listed by hand: a
//! member added to a template is reserved by the same edit.
//! `sce-build/tests/reader_names.rs` renders real documents in every
//! backend and checks that each member the output defines is reserved.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use crate::filters;
use crate::generator::Language;

/// Each backend's spelling of one variable's reader.
///
/// Templates read the field for their own backend and nothing else, so a
/// backend cannot spell a reader some other way than the one whose
/// existence was decided here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct ReaderNames {
    pub rust: String,
    pub cpp: String,
    pub kotlin: String,
    pub go: String,
    pub python: String,
    /// The suffix after `<prefix><machine>_`; C11 readers are free
    /// functions, so this is the part the variable contributes.
    pub c11: String,
}

impl ReaderNames {
    /// This variable's reader in `language`.
    pub fn get(&self, language: Language) -> &str {
        match language {
            Language::Rust => &self.rust,
            Language::Cpp => &self.cpp,
            Language::Kotlin => &self.kotlin,
            Language::Go => &self.go,
            Language::Python => &self.python,
            Language::C11 => &self.c11,
        }
    }
}

/// Why a declared, typed variable has no reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnreadableReason {
    /// The name's declarations disagree about its type, so no reader type
    /// is a claim the document made.
    TypeDisagreement,
    /// A `json` reader names the variable inside an expression, and this
    /// name is not one an expression can denote (`screen-rules`).
    NotAnExpression,
    /// The name is a keyword of `language` that the language gives no way
    /// to escape (C++, or Rust `self` / `Self` / `super` / `crate`).
    Keyword,
    /// The spelling is not an identifier in `language` at all — `_` folds
    /// to nothing in Go's PascalCase.
    NotAnIdentifier,
    /// The spelling is a member the generated type already carries.
    MemberCollision,
    /// Another variable folds to the same spelling in `language`.
    DuplicateSpelling,
}

/// One variable that has no reader, and the first backend that refused it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnreadableVariable {
    pub var: String,
    pub reason: UnreadableReason,
    /// The backend whose spelling could not be used; absent for the two
    /// reasons that are about the document rather than a language.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<&'static str>,
    /// The spelling that was refused, when there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spelling: Option<String>,
    /// The other variable, for [`UnreadableReason::DuplicateSpelling`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<crate::forge::error::SourceLocation>,
}

/// Rust keywords that `r#` cannot escape: they are path segments, not
/// identifiers, and the reference rejects `r#self` and its three siblings.
const RUST_UNRAWABLE: &[&str] = &["self", "Self", "super", "crate"];

/// Kotlin hard keywords — the ones that cannot name a declaration without
/// backticks (Kotlin language specification, "Keywords and operators").
const KOTLIN_HARD_KEYWORDS: &[&str] = &[
    "as",
    "break",
    "class",
    "continue",
    "do",
    "else",
    "false",
    "for",
    "fun",
    "if",
    "in",
    "interface",
    "is",
    "null",
    "object",
    "package",
    "return",
    "super",
    "this",
    "throw",
    "true",
    "try",
    "typealias",
    "typeof",
    "val",
    "var",
    "when",
    "while",
];

/// Python keywords (`keyword.kwlist`, Python 3.12). The soft keywords
/// (`match`, `case`, `type`, `_`) name a method legally and are absent.
const PYTHON_KEYWORDS: &[&str] = &[
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue",
    "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import",
    "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while",
    "with", "yield",
];

/// C++20 keywords and alternative tokens (ISO/IEC 14882:2020 [lex.key],
/// [lex.digraph]), plus the standard-library macros a generated header's
/// includes define with lowercase names, which a member function of the
/// same name would be expanded into. C++ has no escape for any of them.
const CPP_RESERVED: &[&str] = &[
    "alignas",
    "alignof",
    "and",
    "and_eq",
    "asm",
    "auto",
    "bitand",
    "bitor",
    "bool",
    "break",
    "case",
    "catch",
    "char",
    "char8_t",
    "char16_t",
    "char32_t",
    "class",
    "compl",
    "concept",
    "const",
    "consteval",
    "constexpr",
    "constinit",
    "const_cast",
    "continue",
    "co_await",
    "co_return",
    "co_yield",
    "decltype",
    "default",
    "delete",
    "do",
    "double",
    "dynamic_cast",
    "else",
    "enum",
    "explicit",
    "export",
    "extern",
    "false",
    "float",
    "for",
    "friend",
    "goto",
    "if",
    "inline",
    "int",
    "long",
    "mutable",
    "namespace",
    "new",
    "noexcept",
    "not",
    "not_eq",
    "nullptr",
    "operator",
    "or",
    "or_eq",
    "private",
    "protected",
    "public",
    "register",
    "reinterpret_cast",
    "requires",
    "return",
    "short",
    "signed",
    "sizeof",
    "static",
    "static_assert",
    "static_cast",
    "struct",
    "switch",
    "template",
    "this",
    "thread_local",
    "throw",
    "true",
    "try",
    "typedef",
    "typeid",
    "typename",
    "union",
    "unsigned",
    "using",
    "virtual",
    "void",
    "volatile",
    "wchar_t",
    "while",
    "xor",
    "xor_eq",
    // <cassert>, <cerrno>, <cstddef>, <csetjmp>, <cstdarg>, <cstdio>
    "assert",
    "errno",
    "offsetof",
    "setjmp",
    "va_arg",
    "va_copy",
    "va_end",
    "va_start",
    "stdin",
    "stdout",
    "stderr",
];

/// C11 keywords C++ does not also reserve (ISO/IEC 9899:2011 §6.4.1).
const C_ONLY_KEYWORDS: &[&str] = &[
    "restrict",
    "_Alignas",
    "_Alignof",
    "_Atomic",
    "_Bool",
    "_Complex",
    "_Generic",
    "_Imaginary",
    "_Noreturn",
    "_Static_assert",
    "_Thread_local",
];

/// Whether `spelled` is a word `language` reserves — a keyword it gives no
/// way to declare as a plain name. One list per language, shared by the
/// reader spelling above and by the forge code-identifier rule.
pub(crate) fn is_reserved_word(language: Language, spelled: &str) -> bool {
    match language {
        Language::Rust => filters::RUST_KEYWORDS.contains(&spelled),
        Language::Kotlin => KOTLIN_HARD_KEYWORDS.contains(&spelled),
        Language::Python => PYTHON_KEYWORDS.contains(&spelled),
        Language::Go => filters::GO_KEYWORDS.contains(&spelled),
        Language::Cpp => CPP_RESERVED.contains(&spelled),
        Language::C11 => CPP_RESERVED.contains(&spelled) || C_ONLY_KEYWORDS.contains(&spelled),
    }
}

/// One way a backend folds a declared name into an identifier — the folds a
/// keyword could come out equal to.
///
/// A form that adds to the name (`alias_`, `set_<name>`, `TYPE_<NAME>`,
/// `<struct>_<name>`) has no case here: no backend reserves a word shaped
/// like that, so it cannot collide and is left out rather than listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Case {
    /// As written.
    Verbatim,
    /// [`filters::to_snake_case`].
    Snake,
    /// [`filters::to_pascal_case`].
    Pascal,
    /// [`filters::to_camel_case`].
    Camel,
    /// [`filters::to_upper_snake_case`].
    UpperSnake,
}

impl Case {
    /// `name` folded this way.
    pub fn spell(self, name: &str) -> String {
        match self {
            Case::Verbatim => name.to_string(),
            Case::Snake => filters::to_snake_case(name.to_string()),
            Case::Pascal => filters::to_pascal_case(name.to_string()),
            Case::Camel => filters::to_camel_case(name.to_string()),
            Case::UpperSnake => filters::to_upper_snake_case(name.to_string()),
        }
    }
}

/// How each backend spells one kind of declared name, as the folds of
/// [`Case`] it applies, in [`Language::ALL`] order. An empty entry is a
/// backend that emits no identifier from the name at all — it never
/// reaches source, or reaches it only with something added.
pub type Spellings = [&'static [Case]; 6];

/// A name no backend declares: a reference to a name declared elsewhere,
/// which is refused where it is declared, or one used only at build time.
pub const NOT_DECLARED: Spellings = [&[], &[], &[], &[], &[], &[]];

/// The first backend, in [`Language::ALL`] order, that reserves the code
/// identifier `name` as `spellings` says it spells it, with that spelling.
///
/// A name one backend cannot declare is refused at parse for all of them
/// (`validation/reserved-code-identifier`), the same narrowing that refuses
/// `raw-value` — so the question has to be asked of what each backend
/// actually emits. Asking one fold of every name got both directions wrong:
/// a const `DEFAULT` (every backend spells it `DEFAULT`) was refused as
/// C++'s `default`, and a variant `match` (Rust spells it `Match`) as
/// Rust's `match`, while a variant `self` was refused only because the
/// wrong fold happened to agree with Rust's `Self`.
pub fn reserved_in(name: &str, spellings: &Spellings) -> Option<(Language, String)> {
    Language::ALL
        .iter()
        .copied()
        .zip(spellings.iter())
        .find_map(|(language, cases)| {
            cases
                .iter()
                .map(|case| case.spell(name))
                .find(|spelled| is_reserved_word(language, spelled))
                .map(|spelled| (language, spelled))
        })
}

/// What a type already defines: exact names, and the prefixes of names it
/// builds from document ids (`history_<state>`, `on_entry_<state>`), which a
/// reader spelled with that prefix could meet.
#[derive(Debug, Default)]
pub struct Reserved {
    pub names: BTreeSet<String>,
    pub prefixes: BTreeSet<String>,
    /// Names built from the machine's own name — a C++ constructor is
    /// `<machine>()` and `<machine>Policy()` — kept as the suffix after it.
    pub machine_suffixes: BTreeSet<String>,
}

impl Reserved {
    /// Whether `name` is taken on a type generated for `machine`.
    pub fn covers(&self, name: &str, machine: &str) -> bool {
        self.names.contains(name)
            || self
                .prefixes
                .iter()
                .any(|p| name.len() > p.len() && name.starts_with(p.as_str()))
            || name
                .strip_prefix(machine)
                .is_some_and(|suffix| self.machine_suffixes.contains(suffix))
    }

    /// Collect every match of `pattern`'s group 1 in `text`. A match
    /// followed directly by a template expression is the fixed half of a
    /// name built from the document, and is kept as a prefix.
    fn scrape(&mut self, text: &str, pattern: &Regex) {
        for caps in pattern.captures_iter(text) {
            let m = caps
                .get(1)
                .expect("reserved-member patterns capture group 1");
            let name = m.as_str().trim_start_matches("r#");
            if text[m.end()..].starts_with("{{") {
                self.prefixes.insert(name.to_string());
            } else {
                self.names.insert(name.to_string());
            }
        }
    }
}

/// The runtime type each backend's generated machine extends or
/// implements, whose members a reader would hide or duplicate.
const RUST_RUNTIME_POLICY: &str = include_str!("../../backends/rust/runtime/src/policy.rs");
const KOTLIN_RUNTIME_ENGINE: &str = include_str!(
    "../../backends/kotlin/runtime/src/commonMain/kotlin/com/sce/runtime/StateMachineEngine.kt"
);
const PYTHON_RUNTIME_POLICY: &str =
    include_str!("../../backends/python/runtime/sce_runtime/policy.py");
const CPP_RUNTIME_ENGINE: &str = include_str!("../../sce/include/static/StaticExecutionEngine.h");

/// The templates a backend's generated type is emitted from — the backend's
/// own directory, read through the registry the generator loads from.
fn backend_templates(language: Language) -> Vec<&'static str> {
    crate::template_registry::embedded_templates_for(language)
        .into_iter()
        .filter(|(name, _)| match language {
            // Both load the whole tree; C++ owns the root files and C11 `c/`.
            Language::Cpp => !name.contains('/'),
            Language::C11 => name.starts_with("c/"),
            _ => !name.starts_with("_macros/"),
        })
        .map(|(_, content)| content)
        .collect()
}

/// `source` with every comment and literal blanked, so a scan for member
/// names reads definitions and not the prose explaining them. Template tags
/// are code, and stay: C11's names are spelled through them.
pub fn code_only(source: &str, classes: &[crate::template_lexing::Class]) -> String {
    source
        .chars()
        .zip(classes)
        .map(|(c, class)| match class {
            crate::template_lexing::Class::Code => c,
            _ if c == '\n' => '\n',
            _ => ' ',
        })
        .collect()
}

fn regex(pattern: &str) -> Regex {
    Regex::new(pattern).expect("reserved-member patterns are fixed and must compile")
}

/// What a member definition looks like in each backend's source — a
/// function, and a field where the language keeps fields in one namespace
/// with methods. Deliberately loose: a name reserved that the type does not
/// carry costs one reader for an unusual variable name, while a member
/// missed is generated code that does not compile.
///
/// C11 is absent: its names are free functions and typedefs spelled with the
/// machine's symbol prefix, which [`scan_template`] and [`rendered_members`]
/// each build from what they are reading.
fn member_patterns(language: Language) -> Vec<Regex> {
    match language {
        Language::Rust => vec![regex(r"\bfn\s+((?:r#)?[A-Za-z_][A-Za-z0-9_]*)")],
        // A definition, a declaration or a call: every unqualified name
        // followed by `(` that is not reached through `.`, `->` or `::`,
        // and every name ending in `_`, which is how the policy spells its
        // data members.
        Language::Cpp => vec![
            regex(r"(?:^|[^.:>A-Za-z0-9_])([A-Za-z_][A-Za-z0-9_]*)\s*\("),
            regex(r"\b([A-Za-z][A-Za-z0-9]*(?:_[A-Za-z0-9]+)*_)(?:[^A-Za-z0-9_]|\{\{)"),
        ],
        Language::Kotlin => vec![regex(r"\bfun\s+(?:<[^>]*>\s*)?([A-Za-z_][A-Za-z0-9_]*)")],
        // Methods; the policy struct's fields are read from its body alone
        // (`GO_POLICY_FIELD`), because a package-level constant shares no
        // namespace with a method and the same line shape declares one.
        Language::Go => vec![regex(r"func\s*\([^)]*\)\s*([A-Z][A-Za-z0-9_]*)")],
        // Methods, and instance attributes, which shadow a method of the
        // same name.
        Language::Python => vec![
            regex(r"\bdef\s+([A-Za-z_][A-Za-z0-9_]*)"),
            regex(r"\bself\.([A-Za-z_][A-Za-z0-9_]*)\s*(?::[^=\n]*)?="),
        ],
        Language::C11 => Vec::new(),
    }
}

/// The body of each Go policy struct in `code` — rendered, or in a template
/// whose type name is `{{ machine_name }}Policy` — and a field line inside
/// one.
static GO_POLICY_STRUCT: LazyLock<Regex> = LazyLock::new(|| {
    regex(r"(?s)type\s+(?:\{\{[^}]*\}\}|[A-Za-z0-9_])*Policy\s+struct\s*\{(.*?)\n\}")
});
static GO_POLICY_FIELD: LazyLock<Regex> =
    LazyLock::new(|| regex(r"(?m)^\s+([A-Z][A-Za-z0-9_]*)\s+[\[\]*A-Za-z]"));

/// Scan `code` for the members `language`'s patterns name, and — for Go —
/// the fields of every policy struct in it.
fn scan_code(language: Language, code: &str, into: &mut Reserved) {
    for pattern in &member_patterns(language) {
        into.scrape(code, pattern);
    }
    if language == Language::Go {
        for body in GO_POLICY_STRUCT.captures_iter(code) {
            into.scrape(&body[1], &GO_POLICY_FIELD);
        }
    }
}

/// Scan one template of `language`'s for the names its generated type
/// carries.
fn scan_template(language: Language, template: &str, into: &mut Reserved) {
    let syntax = crate::template_lexing::Syntax::of_language(language);
    let code = code_only(
        template,
        &crate::template_lexing::classify_template(template, syntax),
    );
    match language {
        Language::C11 => {
            // `<prefix><machine>_<name>`, spelled either as the two tags or
            // through a `{% set %}` binding of them (`_t` in microstep).
            let mut spellings = vec![r"csym_prefix\s*\}\}\{\{\s*model\.name".to_string()];
            let lines: Vec<&str> = template.lines().collect();
            for binding in crate::template_lexing::set_bindings(&lines) {
                if binding.value.contains("csym_prefix") && binding.value.contains("model.name") {
                    spellings.push(regex::escape(&binding.name));
                }
            }
            let pattern = regex(&format!(
                r"\{{\{{\s*(?:{})\s*\}}\}}_([A-Za-z0-9_]+)",
                spellings.join("|")
            ));
            into.scrape(&code, &pattern);
        }
        Language::Cpp => {
            scan_code(language, &code, into);
            // `<machine>()` and `<machine>Policy()` — constructors, named by
            // the document.
            static MACHINE_MEMBER: LazyLock<Regex> =
                LazyLock::new(|| regex(r"\{\{\s*model\.name\s*\}\}([A-Za-z0-9_]*)\s*\("));
            for caps in MACHINE_MEMBER.captures_iter(&code) {
                into.machine_suffixes.insert(caps[1].to_string());
            }
        }
        _ => scan_code(language, &code, into),
    }
}

/// The member names a generated source file defines, read the way
/// [`reserved`] reads the templates: `source` is rendered output, scanned as
/// code alone. C11 spells its names through template tags that rendering
/// has replaced, so for it the scan is for `c11_symbols` — the rendered
/// `<prefix><machine>_` — instead.
///
/// The guard test holds every name found here to [`reserved`], which is how
/// a member built from a document id, invisible in the template text, is
/// found reserved by prefix or not at all.
pub fn rendered_members(language: Language, source: &str, c11_symbols: &str) -> BTreeSet<String> {
    let syntax = crate::template_lexing::Syntax::of_language(language);
    let code = code_only(source, &crate::template_lexing::classify(source, syntax));
    let mut found = Reserved::default();
    if language == Language::C11 {
        let pattern = regex(&format!(r"\b{}([A-Za-z0-9_]+)", regex::escape(c11_symbols)));
        found.scrape(&code, &pattern);
    } else {
        scan_code(language, &code, &mut found);
    }
    found.names
}

/// The members each backend's generated type carries.
pub fn reserved(language: Language) -> &'static Reserved {
    static RESERVED: LazyLock<BTreeMap<&'static str, Reserved>> = LazyLock::new(|| {
        Language::ALL
            .iter()
            .map(|&language| {
                let mut reserved = Reserved::default();
                for template in backend_templates(language) {
                    scan_template(language, template, &mut reserved);
                }
                let runtime = match language {
                    Language::Rust => Some(RUST_RUNTIME_POLICY),
                    Language::Kotlin => Some(KOTLIN_RUNTIME_ENGINE),
                    Language::Python => Some(PYTHON_RUNTIME_POLICY),
                    Language::Cpp => Some(CPP_RUNTIME_ENGINE),
                    Language::Go | Language::C11 => None,
                };
                if let Some(source) = runtime {
                    let syntax = crate::template_lexing::Syntax::of_language(language);
                    let code = code_only(source, &crate::template_lexing::classify(source, syntax));
                    scan_code(language, &code, &mut reserved);
                }
                if language == Language::C11 {
                    // A local `<invoke>`'s child machine is
                    // `<parent>__sce_synth_invoke__<id>`, and C11 has one
                    // namespace for the parent's functions and the child's.
                    let infix = crate::mesh::deploy::SYNTH_INVOKE_INFIX;
                    reserved
                        .prefixes
                        .insert(infix.strip_prefix('_').unwrap_or(infix).to_string());
                }
                (language.canonical_name(), reserved)
            })
            .collect()
    });
    &RESERVED[language.canonical_name()]
}

/// The members `language`'s generator defines for `model` beside its
/// templates ([`crate::generator::emitted_beside_templates`]), read the way
/// [`rendered_members`] reads a generated file.
///
/// [`reserved`] reads the templates, and cannot see a member spelled in Rust
/// — the typed payload channel's, a native action interface's, a typed
/// host-run invoke's. Those are built from the document, so they are
/// reserved for the document rather than for every machine.
pub fn emitted_members(language: Language, model: &crate::model::SCXMLModel) -> BTreeSet<String> {
    let code = crate::generator::emitted_beside_templates(model, language);
    rendered_members(language, &code, &format!("{}_", model.name))
}

/// A spelling, or why there is none.
enum Spelling {
    Name(String),
    Refused(UnreadableReason, String),
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && name != "_"
}

/// Spell `id`'s reader in `language`, escaping a keyword where the language
/// can.
fn spell(id: &str, language: Language) -> Spelling {
    let base = match language {
        Language::Rust | Language::Python | Language::C11 => filters::to_snake_case(id.to_string()),
        Language::Go => filters::to_pascal_case(id.to_string()),
        Language::Kotlin => filters::to_camel_case(id.to_string()),
        // C++ keeps the author's spelling, as it always has; only the two
        // delimiters an XML name allows and an identifier does not change.
        Language::Cpp => id.replace(['.', '-'], "_"),
    };
    if !is_identifier(&base) {
        return Spelling::Refused(UnreadableReason::NotAnIdentifier, base);
    }
    match language {
        Language::Rust if RUST_UNRAWABLE.contains(&base.as_str()) => {
            Spelling::Refused(UnreadableReason::Keyword, base)
        }
        Language::Rust if filters::RUST_KEYWORDS.contains(&base.as_str()) => {
            Spelling::Name(format!("r#{base}"))
        }
        Language::Kotlin if KOTLIN_HARD_KEYWORDS.contains(&base.as_str()) => {
            Spelling::Name(format!("`{base}`"))
        }
        Language::Python if PYTHON_KEYWORDS.contains(&base.as_str()) => {
            Spelling::Name(format!("{base}_"))
        }
        Language::Cpp if CPP_RESERVED.contains(&base.as_str()) => {
            Spelling::Refused(UnreadableReason::Keyword, base)
        }
        _ => Spelling::Name(base),
    }
}

/// The bare identifier a spelling declares, without its escape.
fn declared(name: &str) -> &str {
    name.trim_start_matches("r#").trim_matches('`')
}

/// Decide which of `candidates` get readers, and name them.
///
/// `candidates` are the variables that passed the document-level rules, in
/// emission order; `refused` already holds those that did not. A candidate
/// refused in any backend is refused in all of them, and the first backend
/// in [`Language::ALL`] order to refuse it is the one recorded. `model` is the
/// document: its `name` is what some backends build members from, and what
/// it declares decides the members the generator adds beside the templates
/// ([`emitted_members`]).
pub fn assign(
    candidates: Vec<crate::model::Variable>,
    model: &crate::model::SCXMLModel,
    refused: &mut Vec<UnreadableVariable>,
) -> Vec<crate::model::Variable> {
    let machine = model.name.as_str();
    let emitted: BTreeMap<&'static str, BTreeSet<String>> = if candidates.is_empty() {
        BTreeMap::new()
    } else {
        Language::ALL
            .iter()
            .map(|&language| (language.canonical_name(), emitted_members(language, model)))
            .collect()
    };
    let mut named: Vec<(crate::model::Variable, ReaderNames)> = Vec::new();
    'candidate: for var in candidates {
        let mut spellings: BTreeMap<&'static str, String> = BTreeMap::new();
        for &language in Language::ALL {
            let refusal = match spell(&var.id, language) {
                Spelling::Refused(reason, spelling) => Some((reason, spelling, None)),
                Spelling::Name(name) => {
                    let bare = declared(&name);
                    if reserved(language).covers(bare, machine)
                        || emitted
                            .get(language.canonical_name())
                            .is_some_and(|names| names.contains(bare))
                    {
                        Some((UnreadableReason::MemberCollision, name, None))
                    } else if let Some((other, _)) = named
                        .iter()
                        .find(|(_, names)| declared(names.get(language)) == bare)
                    {
                        Some((
                            UnreadableReason::DuplicateSpelling,
                            name,
                            Some(other.id.clone()),
                        ))
                    } else {
                        spellings.insert(language.canonical_name(), name);
                        None
                    }
                }
            };
            if let Some((reason, spelling, with)) = refusal {
                refused.push(UnreadableVariable {
                    var: var.id.clone(),
                    reason,
                    language: Some(language.canonical_name()),
                    // Go folds `_` to nothing, which is no spelling at all.
                    spelling: (!spelling.is_empty()).then_some(spelling),
                    with,
                    location: var.source_location.clone(),
                });
                continue 'candidate;
            }
        }
        let take = |spellings: &mut BTreeMap<&'static str, String>, language: Language| {
            spellings
                .remove(language.canonical_name())
                .expect("every backend spelled an accepted reader")
        };
        let names = ReaderNames {
            rust: take(&mut spellings, Language::Rust),
            cpp: take(&mut spellings, Language::Cpp),
            kotlin: take(&mut spellings, Language::Kotlin),
            go: take(&mut spellings, Language::Go),
            python: take(&mut spellings, Language::Python),
            c11: take(&mut spellings, Language::C11),
        };
        named.push((var, names));
    }
    named
        .into_iter()
        .map(|(mut var, names)| {
            var.reader = Some(names);
            var
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(id: &str, language: Language) -> Option<String> {
        match spell(id, language) {
            Spelling::Name(n) => Some(n),
            Spelling::Refused(..) => None,
        }
    }

    #[test]
    fn a_keyword_is_escaped_where_the_language_can() {
        assert_eq!(name("box", Language::Rust).as_deref(), Some("r#box"));
        assert_eq!(
            name("object", Language::Kotlin).as_deref(),
            Some("`object`")
        );
        assert_eq!(name("pass", Language::Python).as_deref(), Some("pass_"));
        assert_eq!(name("box", Language::Cpp).as_deref(), Some("box"));
    }

    #[test]
    fn a_keyword_no_escape_reaches_is_refused() {
        assert_eq!(name("auto", Language::Cpp), None);
        assert_eq!(name("self", Language::Rust), None);
        assert_eq!(name("_", Language::Go), None);
    }

    #[test]
    fn an_xml_name_delimiter_is_not_passed_to_cpp() {
        assert_eq!(
            name("screen-rules", Language::Cpp).as_deref(),
            Some("screen_rules")
        );
    }

    #[test]
    fn the_members_a_type_carries_are_reserved() {
        let m = "machine";
        assert!(reserved(Language::Rust).covers("new", m));
        assert!(reserved(Language::Rust).covers("is_state_active", m));
        assert!(reserved(Language::Kotlin).covers("start", m));
        assert!(reserved(Language::Go).covers("ScriptEngine", m));
        assert!(reserved(Language::C11).covers("t", m));
        assert!(reserved(Language::C11).covers("entry_set_t", m));
        assert!(reserved(Language::Python).covers("initial_state", m));
        assert!(reserved(Language::Cpp).covers("initialize", m));
        assert!(reserved(Language::Cpp).covers("sessionId_", m));
        assert!(reserved(Language::Cpp).covers("machine", m));
        assert!(reserved(Language::Cpp).covers("machinePolicy", m));
        assert!(!reserved(Language::Rust).covers("slot", m));
        assert!(!reserved(Language::Go).covers("AiLoopStateRun", m));
    }
}
