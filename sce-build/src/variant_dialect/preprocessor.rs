// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C and C++ dialects of the output for every build (ADR 0057 §10).
//!
//! A selector in C and C++ is a conditional of the preprocessor, so the grammar
//! that decides where one may stand is the preprocessor's: ISO C translation
//! phases 1 to 3 and §6.10, the same phases in ISO C++. The reader splices
//! lines, reads preprocessing tokens and comments, and splits the text into
//! logical lines; a directive is a logical line whose first token is `#`. A
//! `#if` stands at the start of a logical line, so this grammar decides where
//! one may stand. The language grammar does not: SCE's C and C++ are written
//! with macros it does not know without their definitions.
//!
//! A member of a list is a run of whole logical lines:
//!
//! - a conditional group (`#if` to its `#endif`) is one member, and each of
//!   its groups is a list the merge may merge inside;
//! - any other directive is a member of its own, except `#line`, which names
//!   the line after it and is one member with the member that line starts, so
//!   no selector comes between them;
//! - a run of other lines extends until its `()`, `[]` and `{}` are balanced
//!   and every conditional it opens is closed.
//!
//! So every selected member encloses whole token trees: the discipline of
//! Liebig et al. stated on the tokens, which is as far as a grammar that does
//! not expand macros can state it. A member that is not balanced on its own
//! (`#ifdef __cplusplus` / `extern "C" {` / `#endif`) cannot be selected. The
//! lines between a `{` that ends a line and its `}` that starts one are a list.
//!
//! The writer writes `#if P` and `#endif` on lines of their own around a
//! member, and the alternatives of one place as one `#if` / `#elif` chain. The
//! resolver evaluates exactly the conditionals written over the features'
//! macros in the grammar of the selectors (`defined`, `!`, `&&`, `||`,
//! parentheses, `0` and `1`), and keeps every other directive as written; the
//! check compares preprocessing tokens, comments aside, the end of a
//! directive's line being one.
//!
//! A selector's lines move the lines after it: `__LINE__` and the line a
//! `#line` scope gives a statement after its first are not those of the
//! variant. The first line of a scope is the one its `#line` names, and that
//! one is kept, since the two are one member.

use std::collections::BTreeMap;
use std::ops::Range;

use super::directives::{self, Condition, Expression, NEWLINE};
use super::Dialect;
use crate::feature_model::macro_spelling;
use crate::feature_selector::{Refusal, Selector};
use crate::generator::Language;
use crate::variant_merge::{Interner, List, Member, Source, Tree, Unmerged};

/// C or C++, as the backend of that language writes it, over the features of
/// one product line.
#[derive(Debug, Clone)]
pub struct Preprocessor {
    language: Language,
    /// Each feature's macro ([`macro_spelling`]), to the feature.
    features: BTreeMap<String, String>,
}

impl Preprocessor {
    /// The dialect of `language`, C11 or C++, whose selectors name the
    /// features `features`.
    pub fn new<'f>(language: Language, features: impl IntoIterator<Item = &'f str>) -> Self {
        assert!(
            matches!(language, Language::C11 | Language::Cpp),
            "the preprocessor dialect is C11's and C++'s"
        );
        Preprocessor {
            language,
            features: features
                .into_iter()
                .map(|name| (macro_spelling(name), name.to_string()))
                .collect(),
        }
    }

    fn cpp(&self) -> bool {
        matches!(self.language, Language::Cpp)
    }
}

impl Dialect for Preprocessor {
    type Lifted = Lifted;
    type Reference = Lifted;

    fn backend(&self) -> Language {
        self.language
    }

    fn name(&self) -> &'static str {
        if self.cpp() {
            "C++"
        } else {
            "C"
        }
    }

    fn is_source(&self, file: &str) -> bool {
        let extensions: &[&str] = if self.cpp() {
            &[".h", ".hpp", ".inl", ".cpp", ".cc"]
        } else {
            &[".h", ".c"]
        };
        extensions.iter().any(|extension| file.ends_with(extension))
    }

    fn spell(&self, selector: &Selector) -> String {
        selector.c()
    }

    fn read<'t>(&self, text: &'t str, trees: &mut Interner) -> Result<Source<'t>, Unmerged> {
        let lines = lex(text, self.cpp())
            .map_err(|e| Unmerged(format!("a variant's source does not read: {e}")))?;
        let count = lines.len();
        let mut reader = Reader {
            lines,
            trees,
            source: Source {
                text,
                top: 0,
                lists: BTreeMap::new(),
            },
        };
        reader.list("file", 0..count, 0..text.len());
        Ok(reader.source)
    }

    fn select(&self, predicate: &str, member: &str) -> String {
        format!("#if {predicate}\n{member}\n#endif")
    }

    fn alternatives(&self, alternatives: &[(String, String)], _indent: &str) -> String {
        let mut out = String::new();
        for (index, (predicate, member)) in alternatives.iter().enumerate() {
            let directive = if index == 0 { "#if" } else { "#elif" };
            out.push_str(&format!("{directive} {predicate}\n{member}\n"));
        }
        out.push_str("#endif");
        out
    }

    fn guarded_file(&self, stem: &str) -> String {
        format!("{stem}_sm.h")
    }

    fn guard(&self, refusals: &[Refusal], model: &str) -> String {
        let mut guard = format!(
            "/* The builds the feature model `{model}` admits (docs/adr/0057 §6): a build\n   \
             that turns a feature on and leaves out one it requires compiles none of this. */\n"
        );
        for refusal in refusals {
            guard.push_str(&refusal.c(model));
        }
        guard
    }

    fn lift(&self, text: &str, _is_feature: &dyn Fn(&str) -> bool) -> Result<Lifted, String> {
        Lifted::read(text, self.cpp(), &self.features)
    }

    fn reference(&self, text: &str) -> Result<Lifted, String> {
        Lifted::read(text, self.cpp(), &self.features)
    }

    fn valuation(&self, lifted: &Lifted, features: &dyn Fn(&str) -> Option<bool>) -> Vec<bool> {
        directives::valuation(&lifted.conditions, features)
    }

    fn difference(
        &self,
        lifted: &Lifted,
        features: &dyn Fn(&str) -> Option<bool>,
        reference: &Lifted,
    ) -> Option<String> {
        directives::difference(
            &lifted.text,
            &lifted.resolve(features),
            &reference.text,
            &reference.resolve(&|_| None),
        )
    }
}

// ---------------------------------------------------------------------------
// Translation phases 1 to 3: logical lines of preprocessing tokens.
// ---------------------------------------------------------------------------

/// A preprocessing token: its text in the source, and its spelling once lines
/// are spliced.
#[derive(Debug, Clone)]
struct Token {
    at: Range<usize>,
    spelling: String,
}

/// A logical line: from the start of its first physical line to the newline
/// that ends it, or to the end of the text.
#[derive(Debug, Clone)]
struct Line {
    start: usize,
    end: usize,
    tokens: Vec<Token>,
}

impl Line {
    /// The name of the directive the line is: `Some("")` for the null
    /// directive, `None` for a line that is no directive.
    fn directive(&self) -> Option<&str> {
        let first = self.tokens.first()?;
        if first.spelling != "#" && first.spelling != "%:" {
            return None;
        }
        Some(
            self.tokens
                .get(1)
                .map_or("", |token| token.spelling.as_str()),
        )
    }

    fn opens_conditional(&self) -> bool {
        matches!(self.directive(), Some("if" | "ifdef" | "ifndef"))
    }

    fn continues_conditional(&self) -> bool {
        matches!(
            self.directive(),
            Some("elif" | "elifdef" | "elifndef" | "else")
        )
    }

    fn closes_conditional(&self) -> bool {
        self.directive() == Some("endif")
    }

    /// Whether the line is a `#line` directive, or the line marker `# 37
    /// "file"` that writes one: a directive that names the line after it.
    fn names_next_line(&self) -> bool {
        self.directive()
            .is_some_and(|name| name == "line" || name.starts_with(|c: char| c.is_ascii_digit()))
    }
}

/// The punctuators of C longer than one character, longest first (ISO C
/// 6.4.6).
const PUNCTUATORS: &[&str] = &[
    "%:%:", "...", "<<=", ">>=", "##", "%:", "<:", ":>", "<%", "%>", "->", "++", "--", "<<", ">>",
    "<=", ">=", "==", "!=", "&&", "||", "*=", "/=", "%=", "+=", "-=", "&=", "^=", "|=",
];

/// The punctuators C++ adds to C's, longest first ([lex.operators]). Each is
/// tried before C's, so that `->*` is not read as `->` and `*`.
const CPP_PUNCTUATORS: &[&str] = &["->*", "<=>", "::", ".*"];

/// `text` as logical lines of preprocessing tokens (phases 1 to 3). `cpp`
/// reads C++'s raw string literals, whose text no splice changes.
fn lex(text: &str, cpp: bool) -> Result<Vec<Line>, String> {
    // The second translation phase of ISO C 5.1.1.2: a backslash that ends a
    // physical line joins it to the next.
    let mut chars: Vec<(usize, char)> = Vec::with_capacity(text.len());
    let mut all = text.char_indices().peekable();
    while let Some((at, c)) = all.next() {
        if c == '\\' {
            let rest = &text[at + 1..];
            if rest.starts_with('\n') {
                all.next();
                continue;
            }
            if rest.starts_with("\r\n") {
                all.next();
                all.next();
                continue;
            }
        }
        chars.push((at, c));
    }
    let char_at = |i: usize| chars.get(i).map(|(_, c)| *c);
    let end_of = |i: usize| chars[i].0 + chars[i].1.len_utf8();
    let mut lines = Vec::new();
    let mut tokens = Vec::new();
    let mut line_start = 0;
    let mut i = 0;
    while i < chars.len() {
        let (at, c) = chars[i];
        match c {
            '\n' => {
                lines.push(Line {
                    start: line_start,
                    end: at,
                    tokens: std::mem::take(&mut tokens),
                });
                line_start = at + 1;
                i += 1;
            }
            c if c.is_whitespace() => i += 1,
            '/' if char_at(i + 1) == Some('*') => {
                let mut j = i + 2;
                while j + 1 < chars.len() && !(chars[j].1 == '*' && chars[j + 1].1 == '/') {
                    j += 1;
                }
                if j + 1 >= chars.len() {
                    return Err(format!("the comment at byte {at} does not end"));
                }
                i = j + 2;
            }
            '/' if char_at(i + 1) == Some('/') => {
                while i < chars.len() && chars[i].1 != '\n' {
                    i += 1;
                }
            }
            _ => {
                let next = token(text, &chars, i, cpp)?;
                let range = at..end_of(next - 1);
                let spliced: String = chars[i..next].iter().map(|(_, c)| *c).collect();
                // A raw string is the source's own text: the splicing of the
                // second translation phase is undone in one ([lex.pptoken]).
                let raw = cpp
                    && ["R\"", "LR\"", "uR\"", "UR\"", "u8R\""]
                        .iter()
                        .any(|prefix| spliced.starts_with(prefix));
                tokens.push(Token {
                    spelling: if raw {
                        text[range.clone()].to_string()
                    } else {
                        spliced
                    },
                    at: range,
                });
                i = next;
            }
        }
    }
    lines.push(Line {
        start: line_start,
        end: text.len(),
        tokens,
    });
    Ok(lines)
}

/// The index of the first character after the token that starts at `i`.
fn token(text: &str, chars: &[(usize, char)], i: usize, cpp: bool) -> Result<usize, String> {
    let char_at = |i: usize| chars.get(i).map(|(_, c)| *c);
    let identifier = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$' || !c.is_ascii();
    let c = chars[i].1;
    if identifier(c) && !c.is_ascii_digit() {
        let mut j = i;
        while char_at(j).is_some_and(identifier) {
            j += 1;
        }
        let name: String = chars[i..j].iter().map(|(_, c)| *c).collect();
        return Ok(match char_at(j) {
            Some(quote @ ('"' | '\'')) if matches!(name.as_str(), "L" | "u" | "U" | "u8") => {
                literal(chars, j, quote)
            }
            Some('"') if cpp && matches!(name.as_str(), "R" | "LR" | "uR" | "UR" | "u8R") => {
                raw_string(text, chars, j)?
            }
            _ => j,
        });
    }
    if c.is_ascii_digit() || (c == '.' && char_at(i + 1).is_some_and(|c| c.is_ascii_digit())) {
        // A pp-number: digits, letters, `_`, `.`, an exponent's sign, and a
        // digit separator.
        let mut j = i + 1;
        while let Some(c) = char_at(j) {
            let next = char_at(j + 1);
            let exponent = matches!(c, 'e' | 'E' | 'p' | 'P') && matches!(next, Some('+' | '-'));
            let separator =
                c == '\'' && next.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
            if exponent || separator {
                j += 2;
            } else if c.is_ascii_alphanumeric() || c == '_' || c == '.' {
                j += 1;
            } else {
                break;
            }
        }
        return Ok(j);
    }
    if c == '"' || c == '\'' {
        return Ok(literal(chars, i, c));
    }
    let at = |spelling: &str| {
        spelling
            .chars()
            .enumerate()
            .all(|(k, p)| char_at(i + k) == Some(p))
    };
    // [lex.pptoken]: `<::` not followed by `:` or `>` is `<` and `::`, not the
    // alternative token `<:`, so that `vector<::T>` names `::T`.
    if cpp && at("<::") && !matches!(char_at(i + 3), Some(':' | '>')) {
        return Ok(i + 1);
    }
    let added: &[&str] = if cpp { CPP_PUNCTUATORS } else { &[] };
    for punctuator in added.iter().chain(PUNCTUATORS) {
        if at(punctuator) {
            return Ok(i + punctuator.chars().count());
        }
    }
    Ok(i + 1)
}

/// The end of the character constant or string literal whose quote is at
/// `i`; a quote no quote closes on its line is a token of its own.
fn literal(chars: &[(usize, char)], i: usize, quote: char) -> usize {
    let mut j = i + 1;
    while j < chars.len() {
        match chars[j].1 {
            '\\' => j += 2,
            '\n' => return i + 1,
            c if c == quote => return j + 1,
            _ => j += 1,
        }
    }
    i + 1
}

/// The end of the raw string literal whose quote is at `i`, read on the
/// source's own text: a splice in it is part of it.
fn raw_string(text: &str, chars: &[(usize, char)], i: usize) -> Result<usize, String> {
    let open = chars[i].0 + 1;
    let delimiter_end = text[open..]
        .find('(')
        .map(|at| open + at)
        .ok_or_else(|| format!("the raw string at byte {open} has no `(`"))?;
    let delimiter = &text[open..delimiter_end];
    let close = format!("){delimiter}\"");
    let end = text[delimiter_end..]
        .find(&close)
        .map(|at| delimiter_end + at + close.len())
        .ok_or_else(|| format!("the raw string at byte {open} does not end"))?;
    Ok(chars.partition_point(|(at, _)| *at < end))
}

// ---------------------------------------------------------------------------
// The reader: the lists of a source and their members.
// ---------------------------------------------------------------------------

/// How far a token moves the depth of brackets: `(`, `[` and `{`, and their
/// digraphs, open; their matches close.
fn opening(spelling: &str) -> Option<char> {
    match spelling {
        "(" => Some('('),
        "[" | "<:" => Some('['),
        "{" | "<%" => Some('{'),
        _ => None,
    }
}

fn closing(spelling: &str) -> Option<char> {
    match spelling {
        ")" => Some('('),
        "]" | ":>" => Some('['),
        "}" | "%>" => Some('{'),
        _ => None,
    }
}

/// A group of brackets open while a member's trees are read.
struct Open {
    delimiter: char,
    token: Token,
    /// The line it opens on, and whether it is that line's last token.
    line: usize,
    ends_line: bool,
    trees: Vec<Tree>,
}

/// A `{}` whose lines between are a list: the lines, and the text between
/// the braces.
struct Braces {
    lines: Range<usize>,
    content: Range<usize>,
}

struct Reader<'r, 't> {
    lines: Vec<Line>,
    trees: &'r mut Interner,
    source: Source<'t>,
}

impl Reader<'_, '_> {
    /// Read the lines `span` as a list whose text is `content`, and every list
    /// inside its members.
    fn list(&mut self, kind: &'static str, span: Range<usize>, content: Range<usize>) {
        let mut members = Vec::new();
        let mut i = span.start;
        while i < span.end {
            if self.lines[i].tokens.is_empty() {
                i += 1;
                continue;
            }
            // A `#line` is one member with the member of the line it names.
            let mut head = i;
            while self.lines[head].names_next_line() {
                match (head + 1..span.end).find(|&k| !self.lines[k].tokens.is_empty()) {
                    Some(next) => head = next,
                    None => break,
                }
            }
            let (end, selectable) = self.extent(head, span.end);
            members.push(self.member(i..end, selectable));
            i = end;
        }
        self.source.lists.insert(
            content.start,
            List {
                kind,
                separated: false,
                content,
                members,
            },
        );
    }

    /// The end of the member whose first line (after its `#line`) is `i`,
    /// within lines ending at `end`, and whether it is balanced, so that a
    /// selector may stand around it.
    fn extent(&self, i: usize, end: usize) -> (usize, bool) {
        let line = &self.lines[i];
        if line.opens_conditional() {
            return match self.endif_of(i, end) {
                Some(endif) => (endif + 1, self.balanced(i..endif + 1)),
                None => (i + 1, false),
            };
        }
        if line.continues_conditional() || line.closes_conditional() {
            return (i + 1, false);
        }
        if line.directive().is_some() {
            return (i + 1, true);
        }
        let (mut depth, mut open) = (0usize, 0usize);
        for (j, line) in self.lines.iter().enumerate().take(end).skip(i) {
            if line.opens_conditional() {
                open += 1;
            } else if line.continues_conditional() || line.closes_conditional() {
                if open == 0 {
                    // A conditional of a list this run is not in.
                    return (j, false);
                }
                if line.closes_conditional() {
                    open -= 1;
                }
            } else if line.directive().is_none() {
                for token in &line.tokens {
                    if opening(&token.spelling).is_some() {
                        depth += 1;
                    } else if closing(&token.spelling).is_some() {
                        if depth == 0 {
                            return (j + 1, false);
                        }
                        depth -= 1;
                    }
                }
            }
            if depth == 0 && open == 0 {
                return (j + 1, true);
            }
        }
        (end, false)
    }

    /// The `#endif` of the conditional `#if` line `i` opens, before `end`.
    fn endif_of(&self, i: usize, end: usize) -> Option<usize> {
        let mut depth = 0usize;
        for (j, line) in self.lines.iter().enumerate().take(end).skip(i) {
            if line.opens_conditional() {
                depth += 1;
            } else if line.closes_conditional() {
                depth -= 1;
                if depth == 0 {
                    return Some(j);
                }
            }
        }
        None
    }

    /// Whether the lines' brackets close what they open and nothing else.
    fn balanced(&self, lines: Range<usize>) -> bool {
        let mut depth = 0usize;
        for line in &self.lines[lines] {
            if line.directive().is_some() {
                continue;
            }
            for token in &line.tokens {
                if opening(&token.spelling).is_some() {
                    depth += 1;
                } else if closing(&token.spelling).is_some() {
                    match depth.checked_sub(1) {
                        Some(less) => depth = less,
                        None => return false,
                    }
                }
            }
        }
        depth == 0
    }

    fn member(&mut self, lines: Range<usize>, selectable: bool) -> Member {
        let range = self.lines[lines.start].start..self.lines[lines.end - 1].end;
        let mut braces = Vec::new();
        let trees = self.trees_of(lines, &mut braces);
        // Each `{}` list outside every other is read here; the lists inside
        // one are read with its members.
        let outermost: Vec<&Braces> = braces
            .iter()
            .filter(|inner| {
                !braces.iter().any(|outer| {
                    outer.lines != inner.lines
                        && outer.lines.start <= inner.lines.start
                        && inner.lines.end <= outer.lines.end
                })
            })
            .collect();
        for list in outermost {
            self.list("braces", list.lines.clone(), list.content.clone());
        }
        self.trees.member(range, trees, selectable)
    }

    /// The trees of the lines `lines`, each `{}` whose lines between are a
    /// list added to `braces`. A conditional the lines start and end at their
    /// own level is a token for each of its directives and a group for each
    /// of its groups, each group read as a list.
    fn trees_of(&mut self, lines: Range<usize>, braces: &mut Vec<Braces>) -> Vec<Tree> {
        let mut stack: Vec<Open> = Vec::new();
        let mut out: Vec<Tree> = Vec::new();
        let mut j = lines.start;
        while j < lines.end {
            if stack.is_empty() && self.lines[j].opens_conditional() {
                if let Some(endif) = self.endif_of(j, lines.end) {
                    self.conditional(j, endif, &mut out);
                    j = endif + 1;
                    continue;
                }
            }
            let line = self.lines[j].clone();
            if line.tokens.is_empty() {
                j += 1;
                continue;
            }
            let into =
                |stack: &mut Vec<Open>, out: &mut Vec<Tree>, tree: Tree| match stack.last_mut() {
                    Some(open) => open.trees.push(tree),
                    None => out.push(tree),
                };
            if line.directive().is_some() {
                for token in &line.tokens {
                    let tree = self.trees.token(token.spelling.clone());
                    into(&mut stack, &mut out, tree);
                }
                let end = self.trees.token(NEWLINE.to_string());
                into(&mut stack, &mut out, end);
                j += 1;
                continue;
            }
            let last = line.tokens.len() - 1;
            for (index, token) in line.tokens.iter().enumerate() {
                if let Some(delimiter) = opening(&token.spelling) {
                    stack.push(Open {
                        delimiter,
                        token: token.clone(),
                        line: j,
                        ends_line: index == last,
                        trees: Vec::new(),
                    });
                    continue;
                }
                let matched = closing(&token.spelling).filter(|delimiter| {
                    stack
                        .last()
                        .is_some_and(|open| open.delimiter == *delimiter)
                });
                if matched.is_none() {
                    let tree = self.trees.token(token.spelling.clone());
                    into(&mut stack, &mut out, tree);
                    continue;
                }
                let open = stack.pop().expect("a matched bracket is open");
                if open.delimiter == '{' && open.ends_line && index == 0 && open.line < j {
                    braces.push(Braces {
                        lines: open.line + 1..j,
                        content: open.token.at.end..token.at.start,
                    });
                }
                let group = self
                    .trees
                    .group(open.delimiter, open.token.at.end, open.trees);
                into(&mut stack, &mut out, group);
            }
            j += 1;
        }
        // A bracket nothing closes is a token like any other.
        while let Some(open) = stack.pop() {
            let opener = self.trees.token(open.token.spelling.clone());
            let target = match stack.last_mut() {
                Some(parent) => &mut parent.trees,
                None => &mut out,
            };
            target.push(opener);
            target.extend(open.trees);
        }
        out
    }

    /// The trees of the conditional from `#if` line `start` to `#endif` line
    /// `endif`, pushed onto `out`, each of its groups read as a list.
    fn conditional(&mut self, start: usize, endif: usize, out: &mut Vec<Tree>) {
        // The directives of this conditional, at its own depth.
        let mut directives = vec![start];
        let mut depth = 0usize;
        for (j, line) in self.lines.iter().enumerate().take(endif).skip(start + 1) {
            if line.opens_conditional() {
                depth += 1;
            } else if line.closes_conditional() {
                depth -= 1;
            } else if line.continues_conditional() && depth == 0 {
                directives.push(j);
            }
        }
        directives.push(endif);
        for (index, &directive) in directives.iter().enumerate() {
            let tokens = self.lines[directive].tokens.clone();
            for token in tokens {
                let tree = self.trees.token(token.spelling);
                out.push(tree);
            }
            out.push(self.trees.token(NEWLINE.to_string()));
            let Some(&next) = directives.get(index + 1) else {
                break;
            };
            // The group is read once, as a list, and its trees are its
            // members': reading it again for the trees would read a
            // conditional inside it twice, and one inside that four times.
            let content = self.lines[directive].end..self.lines[next].start;
            self.list("group", directive + 1..next, content.clone());
            let trees: Vec<Tree> = self.source.lists[&content.start]
                .members
                .iter()
                .flat_map(|member| member.trees().iter().cloned())
                .collect();
            out.push(self.trees.group('#', content.start, trees));
        }
    }
}

// ---------------------------------------------------------------------------
// The resolver: a source as one build reads it.
// ---------------------------------------------------------------------------

/// The expression of a `#if` in the grammar of the selectors, whose atoms are
/// `0`, `1` and `defined` of a feature's macro, parenthesised or not.
fn expression(tokens: &[Token], features: &BTreeMap<String, String>) -> Option<Condition> {
    let spellings: Vec<&str> = tokens.iter().map(|token| token.spelling.as_str()).collect();
    Expression::read(&spellings, &|expression: &mut Expression<'_>| {
        if expression.eat("0") {
            return Some(Condition::Constant(false));
        }
        if expression.eat("1") {
            return Some(Condition::Constant(true));
        }
        if !expression.eat("defined") {
            return None;
        }
        let parenthesised = expression.eat("(");
        let feature = features.get(expression.next()?)?;
        if parenthesised && !expression.eat(")") {
            return None;
        }
        Some(Condition::Feature(feature.clone()))
    })
}

/// A line of a source, or a conditional the resolver may decide.
#[derive(Debug)]
enum Node {
    Line(usize),
    Conditional {
        /// Each group: its directive's line, its condition when the resolver
        /// can read it (`Some(Constant(true))` for `#else`), and its lines.
        groups: Vec<(usize, Option<Condition>, Vec<Node>)>,
        endif: Option<usize>,
    },
}

/// A source read for the resolver: its text, its lines, and each distinct
/// condition over the features its conditionals write.
#[derive(Debug)]
pub struct Lifted {
    text: String,
    lines: Vec<Line>,
    nodes: Vec<Node>,
    conditions: BTreeMap<String, Condition>,
}

impl Lifted {
    fn read(text: &str, cpp: bool, features: &BTreeMap<String, String>) -> Result<Lifted, String> {
        let lines = lex(text, cpp)?;
        let mut at = 0;
        let mut conditions = BTreeMap::new();
        let nodes = nodes(&lines, &mut at, true, features, &mut conditions);
        Ok(Lifted {
            text: text.to_string(),
            lines,
            nodes,
            conditions,
        })
    }

    /// The source's tokens, each with its offset, in a build where `features`
    /// answers `Some(on)` for each feature: a conditional all of whose
    /// conditions are read and answered keeps the group of the first that
    /// holds and loses its directives; any other is kept as written.
    fn resolve(&self, features: &dyn Fn(&str) -> Option<bool>) -> Vec<(String, usize)> {
        let mut out = Vec::new();
        self.resolve_into(&self.nodes, features, &mut out);
        out
    }

    fn resolve_into(
        &self,
        nodes: &[Node],
        features: &dyn Fn(&str) -> Option<bool>,
        out: &mut Vec<(String, usize)>,
    ) {
        for node in nodes {
            match node {
                Node::Line(line) => self.push_line(*line, out),
                Node::Conditional { groups, endif } => {
                    let decided: Option<Vec<bool>> = endif.and(
                        groups
                            .iter()
                            .map(|(_, condition, _)| condition.as_ref()?.value(features))
                            .collect(),
                    );
                    match decided {
                        Some(values) => {
                            if let Some(kept) = values.iter().position(|holds| *holds) {
                                self.resolve_into(&groups[kept].2, features, out);
                            }
                        }
                        None => {
                            for (directive, _, body) in groups {
                                self.push_line(*directive, out);
                                self.resolve_into(body, features, out);
                            }
                            if let Some(endif) = endif {
                                self.push_line(*endif, out);
                            }
                        }
                    }
                }
            }
        }
    }

    fn push_line(&self, line: usize, out: &mut Vec<(String, usize)>) {
        let line = &self.lines[line];
        for token in &line.tokens {
            out.push((token.spelling.clone(), token.at.start));
        }
        if line.directive().is_some() {
            out.push((NEWLINE.to_string(), line.end));
        }
    }
}

/// The lines from `at` as nodes, until the end or, below the top, a
/// directive that continues or closes the conditional they are a group of.
fn nodes(
    lines: &[Line],
    at: &mut usize,
    top: bool,
    features: &BTreeMap<String, String>,
    conditions: &mut BTreeMap<String, Condition>,
) -> Vec<Node> {
    let mut out = Vec::new();
    while *at < lines.len() {
        let line = &lines[*at];
        if line.opens_conditional() {
            let mut groups = Vec::new();
            let mut directive = *at;
            loop {
                let condition = condition_of(&lines[directive], features);
                if let Some(condition) = &condition {
                    let spelling: Vec<&str> = lines[directive].tokens[2..]
                        .iter()
                        .map(|token| token.spelling.as_str())
                        .collect();
                    if lines[directive].directive() != Some("else") {
                        conditions
                            .entry(spelling.join(" "))
                            .or_insert_with(|| condition.clone());
                    }
                }
                *at = directive + 1;
                let body = nodes(lines, at, false, features, conditions);
                groups.push((directive, condition, body));
                match lines.get(*at) {
                    Some(next) if next.continues_conditional() => directive = *at,
                    Some(next) if next.closes_conditional() => {
                        out.push(Node::Conditional {
                            groups,
                            endif: Some(*at),
                        });
                        *at += 1;
                        break;
                    }
                    _ => {
                        out.push(Node::Conditional {
                            groups,
                            endif: None,
                        });
                        break;
                    }
                }
            }
            continue;
        }
        if !top && (line.continues_conditional() || line.closes_conditional()) {
            return out;
        }
        out.push(Node::Line(*at));
        *at += 1;
    }
    out
}

/// The condition of a conditional's directive line over the features, when
/// it is written in the grammar of the selectors; `#else` holds.
fn condition_of(line: &Line, features: &BTreeMap<String, String>) -> Option<Condition> {
    let operands = &line.tokens[2.min(line.tokens.len())..];
    let defined = |operands: &[Token]| match operands {
        [name] => features
            .get(&name.spelling)
            .map(|feature| Condition::Feature(feature.clone())),
        _ => None,
    };
    match line.directive()? {
        "if" | "elif" => expression(operands, features),
        "ifdef" | "elifdef" => defined(operands),
        "ifndef" | "elifndef" => defined(operands).map(|c| Condition::Not(Box::new(c))),
        "else" => operands.is_empty().then_some(Condition::Constant(true)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variant_merge::{merge, VariantSet};

    fn c() -> Preprocessor {
        Preprocessor::new(Language::C11, ["a", "b"])
    }

    fn set(variants: &[usize]) -> VariantSet {
        variants.iter().copied().collect()
    }

    /// Variant 0 is the build with `a` on, variant 1 the build with it off.
    fn spell(holding: &VariantSet, _within: &VariantSet) -> String {
        if holding.contains(&0) {
            "defined(SCE_FEATURE_A)".into()
        } else {
            "!defined(SCE_FEATURE_A)".into()
        }
    }

    fn merged(with_a: &str, without: &str) -> String {
        merge(&[(set(&[0]), with_a), (set(&[1]), without)], &c(), &spell).unwrap()
    }

    fn spellings(lifted: &Lifted, a: bool) -> Vec<String> {
        lifted
            .resolve(&|name| (name == "a").then_some(a).or(Some(false)))
            .into_iter()
            .map(|(spelling, _)| spelling)
            .collect()
    }

    /// The merge resolved for each build is that build's own source, token
    /// for token (ADR 0057 §9).
    fn assert_resolves(merged: &str, with_a: &str, without: &str) {
        let dialect = c();
        let lifted = dialect.lift(merged, &|_| true).unwrap();
        for (a, own) in [(true, with_a), (false, without)] {
            let own = dialect.reference(own).unwrap();
            assert_eq!(
                spellings(&lifted, a),
                spellings(&own, a),
                "a = {a}:\n{merged}"
            );
        }
    }

    #[test]
    fn a_logical_line_joins_what_a_splice_and_a_comment_span() {
        let lines = lex("#define X \\\n  1\nint /* a\n b */ y;\n", false).unwrap();
        let spelled: Vec<Vec<&str>> = lines
            .iter()
            .map(|line| line.tokens.iter().map(|t| t.spelling.as_str()).collect())
            .collect();
        assert_eq!(
            spelled,
            vec![vec!["#", "define", "X", "1"], vec!["int", "y", ";"], vec![]]
        );
    }

    #[test]
    fn a_raw_string_is_one_token_of_cpp() {
        let lines = lex("auto s = R\"x(a\n\\\nb)x\";\n", true).unwrap();
        assert_eq!(lines[0].tokens[3].spelling, "R\"x(a\n\\\nb)x\"");
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn a_less_than_before_a_scope_is_no_bracket_of_cpp() {
        let spelled = |text: &str, cpp: bool| -> Vec<String> {
            lex(text, cpp).unwrap()[0]
                .tokens
                .iter()
                .map(|t| t.spelling.clone())
                .collect()
        };
        assert_eq!(
            spelled("p<::T> a<::>b<:::c->*d", true),
            ["p", "<", "::", "T", ">", "a", "<:", ":>", "b", "<:", "::", "c", "->*", "d"]
        );
        // C has neither `::` nor the rule: `<:` is `[` wherever it is written.
        assert_eq!(spelled("p<::T>", false), ["p", "<:", ":", "T", ">"]);
    }

    #[test]
    fn a_member_is_a_balanced_run_of_lines() {
        let text = "int f(void) {\n    return 1;\n}\nint g;\n";
        let source = c().read(text, &mut Interner::default()).unwrap();
        assert_eq!(source.lists[&0].members.len(), 2);
        let body = source.lists.values().find(|l| l.kind == "braces").unwrap();
        assert_eq!(body.members.len(), 1);
    }

    #[test]
    fn a_conditional_that_opens_a_brace_it_does_not_close_cannot_be_selected() {
        let text =
            "#ifdef __cplusplus\nextern \"C\" {\n#endif\nint a;\n#ifdef __cplusplus\n}\n#endif\n";
        let with = text.replace("int a;\n", "int a;\nint b;\n");
        let out = merged(&with, text);
        assert!(
            out.contains("#if defined(SCE_FEATURE_A)\nint b;\n#endif\n"),
            "{out}"
        );
        assert_resolves(&out, &with, text);
    }

    #[test]
    fn a_comment_over_a_member_some_variants_write_is_written_once() {
        let with = "/* the checks */\nint a;\nint b;\n";
        let without = "/* the checks */\nint b;\n";
        let out = merged(with, without);
        assert_eq!(
            out,
            "/* the checks */\n#if defined(SCE_FEATURE_A)\nint a;\n#endif\nint b;\n"
        );
        assert_resolves(&out, with, without);
    }

    #[test]
    fn a_statement_some_variants_write_is_written_under_their_selector() {
        let with = "void f(void) {\n    a();\n    /* note */\n    b();\n}\n";
        let without = "void f(void) {\n    a();\n}\n";
        let out = merged(with, without);
        assert_eq!(
            out,
            "void f(void) {\n    a();\n    /* note */\n#if defined(SCE_FEATURE_A)\n    b();\n#endif\n}\n"
        );
        assert_resolves(&out, with, without);
    }

    #[test]
    fn a_directive_that_differs_is_written_under_the_selector_of_each_variant() {
        let with = "#define N 2\nint x;\n";
        let without = "#define N 1\nint x;\n";
        let out = merged(with, without);
        assert_eq!(
            out,
            "#if defined(SCE_FEATURE_A)\n#define N 2\n#endif\n\
             #if !defined(SCE_FEATURE_A)\n#define N 1\n#endif\nint x;\n"
        );
        assert_resolves(&out, with, without);
    }

    /// Members that differ inside a group of tokens that is no list are
    /// alternatives of one place: one chain, since exactly one is compiled.
    #[test]
    fn members_that_differ_inside_no_list_are_one_chain_of_alternatives() {
        let with = "int f(void) { return 2; }\n";
        let without = "int f(void) { return 1; }\n";
        let out = merged(with, without);
        assert_eq!(
            out,
            "#if defined(SCE_FEATURE_A)\nint f(void) { return 2; }\n\
             #elif !defined(SCE_FEATURE_A)\nint f(void) { return 1; }\n#endif\n"
        );
        assert_resolves(&out, with, without);
    }

    #[test]
    fn a_line_directive_stays_with_the_line_it_names() {
        let with = "void f(void) {\n#line 7 \"d.scxml\"\n    b();\n}\n";
        let without = "void f(void) {\n}\n";
        let out = merged(with, without);
        assert!(
            out.contains("#if defined(SCE_FEATURE_A)\n#line 7 \"d.scxml\"\n    b();\n#endif"),
            "{out}"
        );
        assert_resolves(&out, with, without);
    }

    #[test]
    fn a_conditional_the_variants_write_is_merged_inside() {
        let with = "#if CHECKED\nint a;\nint b;\n#else\nint c;\n#endif\n";
        let without = "#if CHECKED\nint a;\n#else\nint c;\n#endif\n";
        let out = merged(with, without);
        assert_eq!(
            out,
            "#if CHECKED\nint a;\n#if defined(SCE_FEATURE_A)\nint b;\n#endif\n#else\nint c;\n#endif\n"
        );
        assert_resolves(&out, with, without);
    }

    #[test]
    fn only_a_condition_over_the_features_is_resolved() {
        let text = "#if defined(SCE_FEATURE_A) && !defined(SCE_FEATURE_B)\nx;\n#endif\n#if defined(OTHER)\ny;\n#endif\n";
        let lifted = c().lift(text, &|_| true).unwrap();
        let on = |a: bool, b: bool| {
            lifted
                .resolve(&move |name| match name {
                    "a" => Some(a),
                    "b" => Some(b),
                    _ => None,
                })
                .into_iter()
                .map(|(spelling, _)| spelling)
                .collect::<Vec<_>>()
                .join(" ")
        };
        assert_eq!(
            on(true, false),
            "x ; # if defined ( OTHER ) \n y ; # endif \n"
        );
        assert_eq!(on(true, true), "# if defined ( OTHER ) \n y ; # endif \n");
        assert_eq!(c().valuation(&lifted, &|_| Some(true)), vec![false]);
    }

    #[test]
    fn the_guard_refuses_a_build_that_breaks_an_edge() {
        let refusal = Refusal {
            feature: "b".into(),
            requires: "a".into(),
        };
        let guard = c().guard(&[refusal], "features");
        let lifted = c().lift(&guard, &|_| true).unwrap();
        let reads = |a: bool, b: bool| {
            lifted
                .resolve(&move |name| Some(if name == "a" { a } else { b }))
                .into_iter()
                .any(|(spelling, _)| spelling == "error")
        };
        assert!(reads(false, true));
        assert!(!reads(true, true) && !reads(true, false) && !reads(false, false));
    }
}
