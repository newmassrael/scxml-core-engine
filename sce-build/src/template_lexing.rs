// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Where a comment ends and code begins in the language a template emits.
//!
//! A value a template renders lands in one of three places in the generated
//! source — code, a string literal, or a comment — and each place needs its
//! own encoding for text the author controls. The generator cannot choose that
//! encoding without knowing the place, and the place is a lexical fact about
//! the template: `/* <log expr="{{ action.expr }}"> */` puts the value inside a
//! C block comment, whatever the value is. This module answers that question,
//! and [`crate::comment_text::encode_template_comments`] acts on the answer.
//!
//! It is in the library rather than beside the gates that also read it for two
//! reasons. The generator needs it at template registration, and a test-only
//! copy next to a production copy would be two answers to where a comment
//! ends — the arrangement this repository's Zero Duplication rule forbids.
//!
//! # Every character gets one of four classes
//!
//! [`Class`] rather than a blanked string, because readers want different
//! things from the same walk: the encoder needs to know which class a tag sits
//! in, a gate looking for names in running code blanks comments and keeps
//! literals, and a gate comparing two renderings blanks both.
//!
//! # Template tags are opaque
//!
//! In a template, `{{ … }}` and `{% … %}` are the template's own code, not the
//! emitted language's, so nothing inside one opens a comment or a string: the
//! `"//"` in `{{ x | default("//") }}` is an argument. A tag takes the class of
//! the place it sits in — a tag inside a comment renders its value inside that
//! comment. `{# … #}` is the template's prose and is classified before the
//! emitted language is.
//!
//! ⚠ What a tag RENDERS is invisible here. A macro that spells its comment
//! delimiter from a variable — `{{ open }}sce:req: …` — writes a comment only
//! once rendered, so no reading of its text can call it one. Such a macro
//! encodes its values itself.

use crate::generator::Language;

/// The lexical rules of a language SCE emits. C++ and C11 share one.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Syntax {
    CFamily,
    Go,
    Kotlin,
    Python,
    Rust,
}

impl Syntax {
    /// The syntax of the source a backend emits.
    pub fn of_language(language: Language) -> Syntax {
        match language {
            Language::Rust => Syntax::Rust,
            Language::Cpp | Language::C11 => Syntax::CFamily,
            Language::Kotlin => Syntax::Kotlin,
            Language::Go => Syntax::Go,
            Language::Python => Syntax::Python,
        }
    }

    /// The syntax a template emits, from the name it is registered under and
    /// the backend that loaded it.
    ///
    /// The name's own extension wins: `state_machine.rs.jinja2` is Rust
    /// whichever environment registered it. A name without one —
    /// `actions/log.jinja2`, `_macros/sce_map_marker.jinja2` — is rendered by
    /// the backend that loaded it, so it takes that backend's syntax. That is
    /// also what reads a macro shared by every backend correctly in each: the
    /// branch a backend renders is written in that backend's language.
    ///
    /// Read from the NAME, never from a path on disk. A checkout's own prefix
    /// can contain a backend's directory name (`/home/go/…`), and matching
    /// that already made every template read as foreign once.
    pub fn of_template(name: &str, backend: Language) -> Syntax {
        let stem = name.strip_suffix(".jinja2").unwrap_or(name);
        let file = stem.rsplit('/').next().unwrap_or(stem);
        let extension = file.rsplit_once('.').map(|(_, ext)| ext);
        match extension {
            Some("rs") => Syntax::Rust,
            Some("go") => Syntax::Go,
            Some("kt" | "kts") => Syntax::Kotlin,
            Some("py") => Syntax::Python,
            Some("c" | "h" | "cc" | "cpp" | "hpp" | "inl" | "ld") => Syntax::CFamily,
            _ => Syntax::of_language(backend),
        }
    }

    fn rules(self) -> Rules {
        match self {
            Syntax::CFamily => Rules::c_family(),
            Syntax::Go => Rules::go(),
            Syntax::Kotlin => Rules::kotlin(),
            Syntax::Python => Rules::python(),
            Syntax::Rust => Rules::rust(),
        }
    }
}

/// What one character of source is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    /// Text the language executes or declares.
    Code,
    /// A comment, in the emitted language or in the template.
    Comment,
    /// A string literal, delimiters included.
    Literal,
    /// A literal the language reads as documentation because it opens a line:
    /// Python's docstring. Prose to a reader looking for citations, and still a
    /// literal to a reader deciding what a value written into it must be
    /// escaped for — which is why it is neither of its neighbours.
    DocLiteral,
}

/// The class of every character of a source file, one entry per `char`.
pub fn classify(source: &str, syntax: Syntax) -> Vec<Class> {
    let chars: Vec<char> = source.chars().collect();
    classify_chars(&chars, &syntax.rules())
}

/// The class of every character of a template, one entry per `char`: its own
/// prose is a comment, and the rest is read as the emitted language with the
/// template's tags skipped whole.
pub fn classify_template(template: &str, syntax: Syntax) -> Vec<Class> {
    let chars: Vec<char> = template.chars().collect();
    let prose = classify_chars(&chars, &Rules::template_prose());
    let without_prose = blank(&chars, &prose);
    let mut classes = classify_chars(&without_prose, &syntax.rules().with_template_tags());
    for (class, template_class) in classes.iter_mut().zip(prose) {
        if template_class == Class::Comment {
            *class = Class::Comment;
        }
    }
    classes
}

/// A template with its own prose (`{# … #}`) blanked to spaces, and nothing
/// else. Length and line numbering are preserved.
pub fn without_template_prose(template: &str) -> String {
    let chars: Vec<char> = template.chars().collect();
    let prose = classify_chars(&chars, &Rules::template_prose());
    blank(&chars, &prose).into_iter().collect()
}

/// One `{{ … }}` in a template.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Interpolation {
    /// Character offset of the opening `{{`, as `str::chars` counts.
    pub start: usize,
    /// Character offset one past the closing `}}`.
    pub end: usize,
    /// 1-based line of the opening `{{`.
    pub line: usize,
    /// The tag as written, delimiters included.
    pub tag: String,
    /// The class of the place the tag sits in.
    pub context: Class,
}

/// Every interpolation a template renders, with the class of its place.
///
/// Tags that render nothing are not found: those inside the template's prose,
/// and anything inside a `{% raw %}` block, whose content is text.
pub fn interpolations(template: &str, syntax: Syntax) -> Vec<Interpolation> {
    let text: Vec<char> = without_template_prose(template).chars().collect();
    let classes = classify_template(template, syntax);
    let mut found = Vec::new();
    let mut line = 1usize;
    let mut i = 0usize;
    while i < text.len() {
        if starts_with(&text, i, "{%") {
            let end = end_of_span(&text, i, "%}");
            let raw = is_raw_open(&text[i..end]);
            line += count_newlines(&text[i..end]);
            i = end;
            if raw {
                let close = end_of_raw_block(&text, i);
                line += count_newlines(&text[i..close]);
                i = close;
            }
            continue;
        }
        if starts_with(&text, i, "{{") {
            let end = end_of_span(&text, i, "}}");
            let tag: String = text[i..end].iter().collect();
            found.push(Interpolation {
                start: i,
                end,
                line,
                tag,
                context: classes[i],
            });
            line += count_newlines(&text[i..end]);
            i = end;
            continue;
        }
        if text[i] == '\n' {
            line += 1;
        }
        i += 1;
    }
    found
}

/// A tag's expression split at its top-level `|`, head first, delimiters and
/// whitespace-control markers removed.
pub fn tag_segments(tag: &str) -> Vec<String> {
    let body = tag.strip_prefix("{{").unwrap_or(tag);
    let body = body.strip_suffix("}}").unwrap_or(body);
    let body = body
        .trim_start_matches(['-', '+'])
        .trim_end_matches(['-', '+']);
    let mut out = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    for c in body.chars() {
        if let Some(q) = quote {
            current.push(c);
            if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '\'' | '"' => {
                quote = Some(c);
                current.push(c);
            }
            '(' | '[' | '{' => {
                depth += 1;
                current.push(c);
            }
            ')' | ']' | '}' => {
                depth -= 1;
                current.push(c);
            }
            '|' if depth == 0 => out.push(std::mem::take(&mut current)),
            _ => current.push(c),
        }
    }
    out.push(current);
    out
}

/// The filter names a tag applies, in order.
pub fn tag_filters(tag: &str) -> Vec<String> {
    tag_segments(tag)
        .into_iter()
        .skip(1)
        .map(|segment| {
            segment
                .trim()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect()
        })
        .collect()
}

/// Whether `filter` is the last thing a tag applies, and applies to the whole
/// value.
///
/// A filter binds tighter than every operator, so `{{ a ~ b | f }}` applies
/// `f` to `b` alone. Any segment holding an operator outside its brackets and
/// quotes therefore means the filter does not cover the value.
pub fn applies_last_to_the_whole_value(tag: &str, filter: &str) -> bool {
    tag_filters(tag).last().is_some_and(|f| f == filter)
        && !tag_segments(tag).iter().any(|s| has_top_level_operator(s))
}

fn has_top_level_operator(segment: &str) -> bool {
    let mut flat = String::new();
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    for c in segment.chars() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '\'' | '"' => quote = Some(c),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            _ if depth == 0 => flat.push(c),
            _ => {}
        }
    }
    if flat.contains(['~', '+', '*', '/', '%', '<', '>', '=', '!']) || flat.contains(" - ") {
        return true;
    }
    flat.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .any(|word| matches!(word, "if" | "else" | "and" | "or" | "not" | "in" | "is"))
}

/// Whether `needle` sits at `at`.
///
/// Written without collecting the needle: this runs once per character of
/// every template on every registration.
pub fn starts_with(s: &[char], at: usize, needle: &str) -> bool {
    for (i, nc) in (at..).zip(needle.chars()) {
        if i >= s.len() || s[i] != nc {
            return false;
        }
    }
    true
}

fn count_newlines(span: &[char]) -> usize {
    span.iter().filter(|c| **c == '\n').count()
}

/// One past the `close` that ends the span opening at `at`, or the end of the
/// text when it never closes.
fn end_of_span(s: &[char], at: usize, close: &str) -> usize {
    let mut i = at + 2;
    while i < s.len() {
        if starts_with(s, i, close) {
            return i + close.chars().count();
        }
        i += 1;
    }
    s.len()
}

/// Whether a `{% … %}` tag opens a raw block.
fn is_raw_open(tag: &[char]) -> bool {
    tag_keyword(tag) == "raw"
}

/// The first word of a statement tag, whitespace control removed.
fn tag_keyword(tag: &[char]) -> String {
    let body: String = tag.iter().collect();
    let body = body.strip_prefix("{%").unwrap_or(&body).to_string();
    let body = body.strip_suffix("%}").unwrap_or(&body).to_string();
    body.trim_matches(|c: char| c == '-' || c == '+' || c.is_whitespace())
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string()
}

/// One past the `{% endraw %}` that closes a raw block whose content starts
/// at `at`.
fn end_of_raw_block(s: &[char], at: usize) -> usize {
    let mut i = at;
    while i < s.len() {
        if starts_with(s, i, "{%") {
            let end = end_of_span(s, i, "%}");
            if tag_keyword(&s[i..end]) == "endraw" {
                return end;
            }
            i = end;
            continue;
        }
        i += 1;
    }
    s.len()
}

/// `chars` with every comment character blanked to a space; newlines survive.
fn blank(chars: &[char], classes: &[Class]) -> Vec<char> {
    chars
        .iter()
        .zip(classes)
        .map(|(ch, class)| {
            if *class == Class::Comment && *ch != '\n' {
                ' '
            } else {
                *ch
            }
        })
        .collect()
}

/// A language's comment and string delimiters.
struct Rules {
    line: &'static [&'static str],
    block: &'static [(&'static str, &'static str)],
    /// Whether a block comment may contain another.
    nests: bool,
    /// String delimiters, longest first, paired with whether a backslash
    /// escapes inside them.
    ///
    /// ⚠ The single quote is absent from every C-family arm, and that is a
    /// measured decision rather than an oversight. C++ writes digit separators
    /// with it — `0x0000'FFFF'FFFF'FFFFULL` in `sce/src/common/Uuid.cpp` — and a
    /// lexer that read the third of those as a character literal ran the
    /// literal to the end of the file, swallowing eighteen lines of comment
    /// into what it called code.
    ///
    /// Dropping it costs nothing here: a character literal holds ONE
    /// character, so it can contain neither `//` nor `/*` nor `#`, and no
    /// comment can hide inside one. Python keeps its `'` because there it
    /// delimits a full string, and Python has no digit separator spelled that
    /// way.
    strings: &'static [(&'static str, bool)],
    /// Delimiters whose content reads as prose when the literal begins a line
    /// — Python's docstring.
    prose_when_line_initial: &'static [&'static str],
    /// C++ raw strings: `R"delim( ... )delim"`.
    cpp_raw: bool,
    /// Whether a backslash immediately before a line break joins the next line
    /// to this one, comments included. C and C++ splice lines before they look
    /// for comments at all, so `// note \` makes the NEXT line a comment too —
    /// the shape an author value ending in `\` produced in generated C++.
    line_splices: bool,
    /// Spans that belong to another language and are skipped whole: the
    /// template's own tags, when the text is a template.
    opaque: &'static [(&'static str, &'static str)],
}

/// The template tags whose content is the template's code.
const TEMPLATE_TAGS: &[(&str, &str)] = &[("{{", "}}"), ("{%", "%}")];

impl Rules {
    fn c_family() -> Self {
        Rules {
            line: &["//"],
            block: &[("/*", "*/")],
            nests: false,
            strings: &[("\"", true)],
            prose_when_line_initial: &[],
            cpp_raw: true,
            line_splices: true,
            opaque: &[],
        }
    }
    fn go() -> Self {
        Rules {
            line: &["//"],
            block: &[("/*", "*/")],
            nests: false,
            strings: &[("`", false), ("\"", true)],
            prose_when_line_initial: &[],
            cpp_raw: false,
            line_splices: false,
            opaque: &[],
        }
    }
    fn kotlin() -> Self {
        // Kotlin's `"""` is an ordinary string; its documentation comment is
        // `/** */`, which the block arm already covers.
        Rules {
            line: &["//"],
            block: &[("/*", "*/")],
            nests: true,
            strings: &[("\"\"\"", false), ("\"", true)],
            prose_when_line_initial: &[],
            cpp_raw: false,
            line_splices: false,
            opaque: &[],
        }
    }
    fn python() -> Self {
        Rules {
            line: &["#"],
            block: &[],
            nests: false,
            strings: &[("\"\"\"", false), ("'''", false), ("\"", true), ("'", true)],
            prose_when_line_initial: &["\"\"\"", "'''"],
            cpp_raw: false,
            line_splices: false,
            opaque: &[],
        }
    }
    /// Rust as a template emits it. Raw strings and lifetimes are not
    /// modelled: a `'` never delimits here, for the reason C-family's does not.
    fn rust() -> Self {
        Rules {
            line: &["//"],
            block: &[("/*", "*/")],
            nests: true,
            strings: &[("\"", true)],
            prose_when_line_initial: &[],
            cpp_raw: false,
            line_splices: false,
            opaque: &[],
        }
    }
    /// Jinja's own prose. `{{ }}` and `{% %}` are code and are left alone;
    /// only `{# #}` is a comment.
    fn template_prose() -> Self {
        Rules {
            line: &[],
            block: &[("{#", "#}")],
            nests: false,
            strings: &[],
            prose_when_line_initial: &[],
            cpp_raw: false,
            line_splices: false,
            opaque: &[],
        }
    }
    /// The same language, read through a template.
    fn with_template_tags(self) -> Self {
        Rules {
            opaque: TEMPLATE_TAGS,
            ..self
        }
    }
}

fn classify_chars(s: &[char], rules: &Rules) -> Vec<Class> {
    let mut class = vec![Class::Code; s.len()];
    let mut i = 0usize;
    while i < s.len() {
        if let Some(end) = end_of_opaque(s, i, rules) {
            i = end;
            continue;
        }
        // A string literal first: a `//` inside one is data, and `http://` in a
        // URL is the case that proves it.
        if let Some((delim, escapes)) = opening_string(s, i, rules) {
            let end = end_of_string(s, i, delim, escapes, rules);
            let kind = if rules.prose_when_line_initial.contains(&delim) && begins_a_line(s, i) {
                Class::DocLiteral
            } else {
                Class::Literal
            };
            mark(&mut class, i, end, kind);
            i = end;
            continue;
        }
        if rules.cpp_raw {
            if let Some(end) = end_of_cpp_raw_string(s, i) {
                mark(&mut class, i, end, Class::Literal);
                i = end;
                continue;
            }
        }
        if rules.line.iter().any(|m| starts_with(s, i, m)) {
            let start = i;
            while i < s.len() && s[i] != '\n' {
                if rules.line_splices && s[i] == '\\' {
                    match (s.get(i + 1), s.get(i + 2)) {
                        (Some('\n'), _) => {
                            i += 2;
                            continue;
                        }
                        (Some('\r'), Some('\n')) => {
                            i += 3;
                            continue;
                        }
                        _ => {}
                    }
                }
                i = end_of_opaque(s, i, rules).unwrap_or(i + 1);
            }
            mark(&mut class, start, i, Class::Comment);
            continue;
        }
        if let Some((open, close)) = rules.block.iter().find(|(o, _)| starts_with(s, i, o)) {
            let start = i;
            let mut depth = 1usize;
            i += open.chars().count();
            while i < s.len() && depth > 0 {
                if let Some(end) = end_of_opaque(s, i, rules) {
                    i = end;
                } else if rules.nests && starts_with(s, i, open) {
                    depth += 1;
                    i += open.chars().count();
                } else if starts_with(s, i, close) {
                    depth -= 1;
                    i += close.chars().count();
                } else {
                    i += 1;
                }
            }
            mark(&mut class, start, i, Class::Comment);
            continue;
        }
        i += 1;
    }
    class
}

fn mark(class: &mut [Class], start: usize, end: usize, kind: Class) {
    for slot in class.iter_mut().take(end).skip(start) {
        *slot = kind;
    }
}

/// Where an opaque span opening at `at` ends, one past its closing delimiter.
fn end_of_opaque(s: &[char], at: usize, rules: &Rules) -> Option<usize> {
    let (open, close) = rules.opaque.iter().find(|(o, _)| starts_with(s, at, o))?;
    let mut i = at + open.chars().count();
    while i < s.len() {
        if starts_with(s, i, close) {
            return Some(i + close.chars().count());
        }
        i += 1;
    }
    Some(s.len())
}

/// Whether only whitespace precedes `at` on its line.
fn begins_a_line(s: &[char], at: usize) -> bool {
    let mut i = at;
    while i > 0 {
        i -= 1;
        if s[i] == '\n' {
            return true;
        }
        if !s[i].is_whitespace() {
            return false;
        }
    }
    true
}

/// The string delimiter opening at `at`, if one does.
fn opening_string(s: &[char], at: usize, rules: &Rules) -> Option<(&'static str, bool)> {
    rules
        .strings
        .iter()
        .find(|(d, _)| starts_with(s, at, d))
        .copied()
}

/// Where the string opening at `at` ends, one past its closing delimiter.
fn end_of_string(s: &[char], at: usize, delim: &str, escapes: bool, rules: &Rules) -> usize {
    let width = delim.chars().count();
    let mut i = at + width;
    while i < s.len() {
        if let Some(end) = end_of_opaque(s, i, rules) {
            i = end;
            continue;
        }
        if escapes && s[i] == '\\' {
            i += 2;
            continue;
        }
        if starts_with(s, i, delim) {
            return i + width;
        }
        i += 1;
    }
    s.len()
}

/// Where a C++ raw string `R"delim(` opening at `at` ends, if one does.
fn end_of_cpp_raw_string(s: &[char], at: usize) -> Option<usize> {
    if s[at] != 'R' || at + 1 >= s.len() || s[at + 1] != '"' {
        return None;
    }
    if at > 0 && (s[at - 1].is_alphanumeric() || s[at - 1] == '_') {
        return None;
    }
    let mut j = at + 2;
    let mut delim = String::new();
    while j < s.len() && s[j] != '(' {
        delim.push(s[j]);
        j += 1;
    }
    if j >= s.len() {
        return None;
    }
    let closing = format!("){delim}\"");
    let mut k = j + 1;
    while k < s.len() {
        if starts_with(s, k, &closing) {
            return Some(k + closing.chars().count());
        }
        k += 1;
    }
    Some(s.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contexts(template: &str, syntax: Syntax) -> Vec<(String, Class)> {
        interpolations(template, syntax)
            .into_iter()
            .map(|i| (i.tag, i.context))
            .collect()
    }

    #[test]
    fn a_c_line_comment_continues_across_a_backslash_newline() {
        let t = "// a \\\nint b = {{ x }};\nint c = {{ y }};\n";
        let c = contexts(t, Syntax::CFamily);
        assert_eq!((c[0].1, c[1].1), (Class::Comment, Class::Code), "{c:?}");
        // Go does not splice, so the same text is code on its second line.
        let go = contexts(t, Syntax::Go);
        assert_eq!(go[0].1, Class::Code, "{go:?}");
    }

    #[test]
    fn a_tag_takes_the_class_of_the_place_it_sits_in() {
        let t = "int a = {{ x }}; /* {{ y }} */ const char *s = \"{{ z }}\";\n";
        assert_eq!(
            contexts(t, Syntax::CFamily),
            vec![
                ("{{ x }}".to_string(), Class::Code),
                ("{{ y }}".to_string(), Class::Comment),
                ("{{ z }}".to_string(), Class::Literal),
            ]
        );
    }

    #[test]
    fn nothing_inside_a_tag_opens_a_comment_or_a_string() {
        let t = "int a = {{ x | default(\"//\") }}; int b = {{ y }};\n";
        assert!(contexts(t, Syntax::CFamily)
            .iter()
            .all(|(_, c)| *c == Class::Code));
    }

    #[test]
    fn tags_that_render_nothing_are_not_found() {
        let t = "{# {{ a }} #}\n{% raw %}// {{ b }}{% endraw %}\n// {{ c }}\n";
        let found = interpolations(t, Syntax::CFamily);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].tag, "{{ c }}");
        assert_eq!(found[0].line, 3);
    }

    #[test]
    fn the_name_decides_the_syntax_before_the_backend_does() {
        assert_eq!(
            Syntax::of_template("state_machine.rs.jinja2", Language::Cpp),
            Syntax::Rust
        );
        assert_eq!(
            Syntax::of_template("actions/log.jinja2", Language::Cpp),
            Syntax::CFamily
        );
        assert_eq!(
            Syntax::of_template("_macros/sce_map_marker.jinja2", Language::Python),
            Syntax::Python
        );
        assert_eq!(
            Syntax::of_template("forge/c/buffer_pool.ld.jinja2", Language::C11),
            Syntax::CFamily
        );
    }

    #[test]
    fn a_filter_covers_the_value_only_without_a_top_level_operator() {
        assert!(applies_last_to_the_whole_value(
            "{{ x | comment_text }}",
            "comment_text"
        ));
        assert!(applies_last_to_the_whole_value(
            "{{- (a ~ b) | comment_text -}}",
            "comment_text"
        ));
        assert!(!applies_last_to_the_whole_value(
            "{{ a ~ b | comment_text }}",
            "comment_text"
        ));
        assert!(!applies_last_to_the_whole_value(
            "{{ a if c else b | comment_text }}",
            "comment_text"
        ));
        assert!(!applies_last_to_the_whole_value(
            "{{ x | comment_text | upper }}",
            "comment_text"
        ));
        assert!(applies_last_to_the_whole_value(
            "{{ x | default('a - b') | comment_text }}",
            "comment_text"
        ));
    }
}
