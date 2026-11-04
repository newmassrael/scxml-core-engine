// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Swift dialect of the output for every build (ADR 0057 §10).
//!
//! A selector in Swift is a conditional compilation block (The Swift
//! Programming Language, "Compiler Control Statements"): `#if`, `#elseif`,
//! `#else` and `#endif` over compilation conditions a build sets (`-D`,
//! SwiftPM's `.define`). Swift's parser reads a block, so where one may stand
//! is decided by Swift's grammar, and the reader reads that grammar with
//! tree-sitter-swift: its `directive` stands among the statements of the file
//! or of a body and among the declarations of a type's body; not among the
//! elements of an array, the arguments of a call or the accessors of a
//! property. Swift itself admits a block among the `case` clauses of a
//! `switch` too, and tree-sitter-swift 0.7.4 does not: a block before a
//! switch's first clause reads as an error. The reader reads the grammar it
//! has, so clauses are no list, and a clause only some variants write moves the
//! choice to its `switch`. A source tree-sitter-swift reads with an error is
//! refused, never guessed at.
//!
//! The lists are the syntax's: the statements of the file; the members of a
//! `{}` that holds statements alone (a body, a block, a closure that names no
//! parameters) or a type's declarations; and the statements of a `case`
//! clause after its `:`. A member is a child of the list
//! (a statement's label is one member with the statement it labels, as is the
//! `throw` of a top-level throw). A directive is a line of its own, so a member
//! may be selected when it starts a line and ends one. The one member of a
//! body that is an expression is the value an implicit `return` gives, and it
//! cannot be selected: the choice moves to the member that holds it, as it
//! does for the value a Rust block ends with.
//!
//! The merge compares token trees: the tokens tree-sitter-swift reads, comments
//! aside, grouped by their brackets, and a clause's statements grouped after
//! its `:`, so that clauses that differ in their statements are merged inside.
//!
//! The writer writes `#if P` and `#endif` on lines of their own around a
//! member, and the alternatives of one place as one `#if` / `#elseif` chain.
//! The resolver evaluates exactly the blocks written over the features'
//! compilation conditions in the grammar of the selectors (`!`, `&&`, `||`,
//! parentheses, `true` and `false`); the check compares tokens, comments
//! aside, each line's start being one. The guard is an `#error` for each edge
//! of the feature model.

use std::collections::BTreeMap;
use std::ops::Range;

use tree_sitter::{Node as Syntax, Parser, Tree as SyntaxTree};

use super::directives::{self, Condition, Expression, NEWLINE};
use super::Dialect;
use crate::feature_model::macro_spelling;
use crate::feature_selector::{Refusal, Selector};
use crate::generator::Language;
use crate::variant_merge::{Interner, List, Source, Tree, Unmerged};

/// Swift, as the Swift backend writes it, over the features of one product
/// line.
#[derive(Debug, Clone)]
pub struct Swift {
    /// Each feature's compilation condition ([`macro_spelling`]), to the
    /// feature.
    features: BTreeMap<String, String>,
}

impl Swift {
    /// The dialect whose selectors name the features `features`.
    pub fn new<'f>(features: impl IntoIterator<Item = &'f str>) -> Self {
        Swift {
            features: features
                .into_iter()
                .map(|name| (macro_spelling(name), name.to_string()))
                .collect(),
        }
    }
}

impl Dialect for Swift {
    type Lifted = Lifted;
    type Reference = Lifted;

    fn backend(&self) -> Language {
        Language::Swift
    }

    fn name(&self) -> &'static str {
        "Swift"
    }

    fn is_source(&self, file: &str) -> bool {
        file.ends_with(".swift")
    }

    fn spell(&self, selector: &Selector) -> String {
        selector.swift()
    }

    fn read<'t>(&self, text: &'t str, trees: &mut Interner) -> Result<Source<'t>, Unmerged> {
        let unread = |e: String| Unmerged(format!("a variant's source does not read: {e}"));
        let syntax = parse(text).map_err(unread)?;
        let read = Read::of(text, &syntax).map_err(unread)?;
        if let Some(&(first, _)) = read.directives.first() {
            return Err(Unmerged(format!(
                "a variant's source writes the compiler directive `{}` at byte {}, which the \
                 output for every build would have to select inside",
                read.tokens[first].spelling, read.tokens[first].at.start
            )));
        }
        let mut reader = Reader {
            read,
            trees,
            source: Source {
                text,
                top: 0,
                lists: BTreeMap::new(),
            },
        };
        reader.lists(syntax.root_node());
        Ok(reader.source)
    }

    fn select(&self, predicate: &str, member: &str) -> String {
        format!("#if {predicate}\n{member}\n#endif")
    }

    fn alternatives(&self, alternatives: &[(String, String)], _indent: &str) -> String {
        let mut out = String::new();
        for (index, (predicate, member)) in alternatives.iter().enumerate() {
            let directive = if index == 0 { "#if" } else { "#elseif" };
            out.push_str(&format!("{directive} {predicate}\n{member}\n"));
        }
        out.push_str("#endif");
        out
    }

    fn guarded_file(&self, stem: &str) -> String {
        format!("{stem}_sm.swift")
    }

    fn guard(&self, refusals: &[Refusal], model: &str) -> String {
        let mut guard = format!(
            "// The builds the feature model `{model}` admits (docs/adr/0057 §6): a build\n\
             // that turns a feature on and leaves out one it requires compiles none of this.\n"
        );
        for refusal in refusals {
            guard.push_str(&refusal.swift(model));
        }
        guard
    }

    fn lift(&self, text: &str, _is_feature: &dyn Fn(&str) -> bool) -> Result<Lifted, String> {
        Lifted::read(text, &self.features)
    }

    fn reference(&self, text: &str) -> Result<Lifted, String> {
        Lifted::read(text, &self.features)
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

/// Where tree-sitter-swift reads `text` with an error, or `None` when it
/// reads all of it: the test every generated Swift source the tree commits is
/// held to, so that a template that writes what the reader cannot read fails
/// there and not in a consumer's build.
pub fn unreadable(text: &str) -> Option<String> {
    parse(text).err()
}

// ---------------------------------------------------------------------------
// The syntax, as tree-sitter-swift reads it, and its tokens.
// ---------------------------------------------------------------------------

/// `text` read by tree-sitter-swift, refused where it reads an error.
fn parse(text: &str) -> Result<SyntaxTree, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_swift::LANGUAGE.into())
        .map_err(|e| format!("tree-sitter-swift does not load: {e}"))?;
    let syntax = parser
        .parse(text, None)
        .ok_or_else(|| "tree-sitter-swift read nothing".to_string())?;
    match first_error(syntax.root_node()) {
        None => Ok(syntax),
        Some(error) => {
            let at = error.start_position();
            let what = if error.is_missing() {
                format!("a missing `{}`", error.kind())
            } else {
                format!("`{}`", &text[error.byte_range()])
            };
            Err(format!(
                "tree-sitter-swift reads an error at line {}, column {}: {what}",
                at.row + 1,
                at.column + 1
            ))
        }
    }
}

/// The first node under `node`, in the text's order, that is an error or a
/// token the parser supplied as missing.
fn first_error(node: Syntax<'_>) -> Option<Syntax<'_>> {
    if !node.has_error() {
        return None;
    }
    if node.is_error() || node.is_missing() {
        return Some(node);
    }
    let mut cursor = node.walk();
    let found = node.children(&mut cursor).find_map(first_error);
    found.or(Some(node))
}

fn is_comment(node: Syntax<'_>) -> bool {
    matches!(node.kind(), "comment" | "multiline_comment")
}

/// A token: its text, and whether no other token stands before it on its line.
#[derive(Debug, Clone)]
struct Token {
    at: Range<usize>,
    spelling: String,
    starts_line: bool,
}

/// A source's tokens and what the merge and the resolver read of them.
struct Read {
    tokens: Vec<Token>,
    /// For each token that opens a group, the token that closes it: a bracket
    /// its match; a clause's `:` the last token of its statements, which the
    /// group holds.
    closes: BTreeMap<usize, usize>,
    /// Each directive, as its first and last token.
    directives: Vec<(usize, usize)>,
}

impl Read {
    fn of(text: &str, syntax: &SyntaxTree) -> Result<Read, String> {
        let mut read = Read {
            tokens: Vec::new(),
            closes: BTreeMap::new(),
            directives: Vec::new(),
        };
        let mut last_row = None;
        read.gather(text, syntax.root_node(), &mut last_row);
        let mut open: Vec<usize> = Vec::new();
        for (index, token) in read.tokens.iter().enumerate() {
            let spelling = token.spelling.as_str();
            if matches!(spelling, "(" | "[" | "{")
                || (spelling.starts_with('\\') && spelling.ends_with('('))
            {
                open.push(index);
            } else if matches!(spelling, ")" | "]" | "}") {
                let Some(opener) = open.pop() else {
                    return Err(format!(
                        "`{spelling}` at byte {} closes nothing",
                        token.at.start
                    ));
                };
                read.closes.insert(opener, index);
            }
        }
        if let Some(&opener) = open.last() {
            let token = &read.tokens[opener];
            return Err(format!(
                "`{}` at byte {} does not close",
                token.spelling, token.at.start
            ));
        }
        read.clauses(syntax.root_node());
        Ok(read)
    }

    /// The tokens under `node`, comments aside; `last_row` is the row the
    /// token before ends on.
    fn gather(&mut self, text: &str, node: Syntax<'_>, last_row: &mut Option<usize>) {
        if is_comment(node) {
            return;
        }
        if node.kind() == "directive" {
            let first = self.tokens.len();
            self.gather_children(text, node, last_row);
            if self.tokens.len() > first {
                self.directives.push((first, self.tokens.len() - 1));
            }
            return;
        }
        if node.child_count() > 0 {
            self.gather_children(text, node, last_row);
            return;
        }
        if node.start_byte() == node.end_byte() {
            return;
        }
        let row = node.start_position().row;
        self.tokens.push(Token {
            at: node.byte_range(),
            spelling: text[node.byte_range()].to_string(),
            starts_line: last_row.is_none_or(|last| last < row),
        });
        *last_row = Some(node.end_position().row);
    }

    fn gather_children(&mut self, text: &str, node: Syntax<'_>, last_row: &mut Option<usize>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.gather(text, child, last_row);
        }
    }

    /// Record each clause's `:` as the opening of a group that holds its
    /// statements.
    fn clauses(&mut self, node: Syntax<'_>) {
        if let Some((colon, statements)) = clause_body(node) {
            if let (Some((colon, _)), Some((_, last))) = (self.span(colon), self.span(statements)) {
                self.closes.insert(colon, last);
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.clauses(child);
        }
    }

    /// The first and last token of `node`, or `None` when it has none.
    fn span(&self, node: Syntax<'_>) -> Option<(usize, usize)> {
        let first = self
            .tokens
            .partition_point(|token| token.at.start < node.start_byte());
        let end = self
            .tokens
            .partition_point(|token| token.at.end <= node.end_byte());
        (first < end).then(|| (first, end - 1))
    }
}

/// A `case` clause's `:` and the statements after it.
fn clause_body(node: Syntax<'_>) -> Option<(Syntax<'_>, Syntax<'_>)> {
    if node.kind() != "switch_entry" {
        return None;
    }
    let mut cursor = node.walk();
    let children: Vec<Syntax<'_>> = node.children(&mut cursor).collect();
    let colon = children.iter().position(|child| child.kind() == ":")?;
    let statements = children[colon + 1..]
        .iter()
        .find(|child| child.kind() == "statements")?;
    Some((children[colon], *statements))
}

// ---------------------------------------------------------------------------
// The reader: the lists of a source and their members.
// ---------------------------------------------------------------------------

/// The kinds of the syntax that are no expression: declarations, and the
/// statements tree-sitter-swift does not read as expressions (`if` and
/// `switch` it does, as Swift does since 5.9).
const NOT_EXPRESSIONS: &[&str] = &[
    "associatedtype_declaration",
    "class_declaration",
    "control_transfer_statement",
    "deinit_declaration",
    "do_statement",
    "for_statement",
    "function_declaration",
    "guard_statement",
    "import_declaration",
    "init_declaration",
    "macro_declaration",
    "operator_declaration",
    "precedence_group_declaration",
    "property_declaration",
    "protocol_declaration",
    "repeat_while_statement",
    "subscript_declaration",
    "typealias_declaration",
    "while_statement",
];

/// The kinds of a `{}` whose members are declarations.
const DECLARATION_BODIES: &[&str] = &["class_body", "enum_class_body", "protocol_body"];

/// The kinds of the syntax that are one member with the node after them: a
/// statement's label, and the `throw` of a top-level throw (tree-sitter-swift
/// writes each as a sibling of what it belongs to).
const LEADING: &[&str] = &["statement_label", "throw_keyword"];

struct Reader<'r, 't> {
    read: Read,
    trees: &'r mut Interner,
    source: Source<'t>,
}

impl Reader<'_, '_> {
    /// Read every list under `node`.
    fn lists(&mut self, node: Syntax<'_>) {
        if node.kind() == "source_file" {
            let members = children(node);
            self.list("file", 0..self.source.text.len(), &members, false);
        }
        if let Some((colon, statements)) = clause_body(node) {
            if let Some((_, last)) = self.read.span(statements) {
                let content = colon.end_byte()..self.read.tokens[last].at.end;
                self.list("statements", content, &children(statements), true);
            }
        }
        let mut cursor = node.walk();
        let all: Vec<Syntax<'_>> = node.children(&mut cursor).collect();
        let mut at = 0;
        while let Some(open) = (at..all.len()).find(|&i| all[i].kind() == "{") {
            let Some(close) = (open + 1..all.len()).find(|&i| all[i].kind() == "}") else {
                break;
            };
            let between: Vec<Syntax<'_>> = all[open + 1..close]
                .iter()
                .copied()
                .filter(|child| !is_comment(*child))
                .collect();
            let content = all[open].end_byte()..all[close].start_byte();
            if DECLARATION_BODIES.contains(&node.kind()) {
                self.list("declarations", content, &grouped(&between), false);
            } else if let [statements] = between.as_slice() {
                if statements.kind() == "statements" {
                    self.list("statements", content, &children(*statements), true);
                }
            } else if between.is_empty() {
                self.list("statements", content, &[], true);
            }
            at = close + 1;
        }
        for child in all {
            self.lists(child);
        }
    }

    /// Record the list whose text is `content` and whose members are
    /// `members`, each the nodes it is made of. A `body` is one an implicit
    /// `return` may give the value of.
    fn list(
        &mut self,
        kind: &'static str,
        content: Range<usize>,
        members: &[Vec<Syntax<'_>>],
        body: bool,
    ) {
        let members = members
            .iter()
            .filter_map(|nodes| {
                let (first, _) = self.read.span(nodes[0])?;
                let (_, last) = self
                    .read
                    .span(*nodes.last().expect("a member has a node"))?;
                let value = body
                    && members.len() == 1
                    && nodes.len() == 1
                    && !NOT_EXPRESSIONS.contains(&nodes[0].kind());
                let selectable = !value && self.stands_alone(first, last);
                let trees = self.trees_of(first..last + 1);
                let range = self.read.tokens[first].at.start..self.read.tokens[last].at.end;
                Some(self.trees.member(range, trees, selectable))
            })
            .collect();
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

    /// Whether the tokens from `first` to `last` start a line and end one, so
    /// that a directive's lines may stand around them.
    fn stands_alone(&self, first: usize, last: usize) -> bool {
        let tokens = &self.read.tokens;
        tokens[first].starts_line && tokens.get(last + 1).is_none_or(|next| next.starts_line)
    }

    /// The trees of the tokens `range`.
    fn trees_of(&mut self, range: Range<usize>) -> Vec<Tree> {
        let mut out = Vec::new();
        let mut at = range.start;
        while at < range.end {
            let token = &self.read.tokens[at];
            match self
                .read
                .closes
                .get(&at)
                .copied()
                .filter(|&close| close < range.end)
            {
                Some(close) => {
                    let delimiter = token.spelling.chars().last().expect("a token has text");
                    let content = token.at.end;
                    // A clause's `:` holds its last token; a bracket does not
                    // hold its match.
                    let inner = if delimiter == ':' {
                        at + 1..close + 1
                    } else {
                        at + 1..close
                    };
                    let inner = self.trees_of(inner);
                    out.push(self.trees.group(delimiter, content, inner));
                    at = close + 1;
                }
                None => {
                    let spelling = token.spelling.clone();
                    out.push(self.trees.token(spelling));
                    at += 1;
                }
            }
        }
        out
    }
}

/// The members of `node`'s list: its named children, comments aside, each
/// that leads another one member with it.
fn children(node: Syntax<'_>) -> Vec<Vec<Syntax<'_>>> {
    let mut cursor = node.walk();
    let named: Vec<Syntax<'_>> = node
        .named_children(&mut cursor)
        .filter(|child| !is_comment(*child))
        .collect();
    grouped(&named)
}

/// `nodes`, named ones alone, each that leads another one member with it.
fn grouped<'s>(nodes: &[Syntax<'s>]) -> Vec<Vec<Syntax<'s>>> {
    let mut members: Vec<Vec<Syntax<'s>>> = Vec::new();
    let mut leading = false;
    for node in nodes.iter().filter(|node| node.is_named()) {
        match members.last_mut() {
            Some(member) if leading => member.push(*node),
            _ => members.push(vec![*node]),
        }
        leading = LEADING.contains(&node.kind());
    }
    members
}

// ---------------------------------------------------------------------------
// The resolver: a source as one build reads it.
// ---------------------------------------------------------------------------

/// The condition of an `#if` or `#elseif` in the grammar of the selectors,
/// whose atoms are `true`, `false` and a feature's compilation condition.
fn expression(tokens: &[Token], features: &BTreeMap<String, String>) -> Option<Condition> {
    let spellings: Vec<&str> = tokens.iter().map(|token| token.spelling.as_str()).collect();
    Expression::read(&spellings, &|expression: &mut Expression<'_>| {
        if expression.eat("true") {
            return Some(Condition::Constant(true));
        }
        if expression.eat("false") {
            return Some(Condition::Constant(false));
        }
        let feature = features.get(expression.peek()?)?;
        expression.next();
        Some(Condition::Feature(feature.clone()))
    })
}

/// A token of a source, or a conditional compilation block the resolver may
/// decide.
#[derive(Debug)]
enum Piece {
    Token(usize),
    Conditional {
        /// Each clause: its directive's tokens, its condition when the
        /// resolver can read it (`Some(Constant(true))` for `#else`), and its
        /// pieces.
        clauses: Vec<(Range<usize>, Option<Condition>, Vec<Piece>)>,
        endif: Option<Range<usize>>,
    },
}

/// A source read for the resolver: its text, its tokens, and each distinct
/// condition over the features its blocks write.
#[derive(Debug)]
pub struct Lifted {
    text: String,
    tokens: Vec<Token>,
    pieces: Vec<Piece>,
    conditions: BTreeMap<String, Condition>,
}

impl Lifted {
    fn read(text: &str, features: &BTreeMap<String, String>) -> Result<Lifted, String> {
        let syntax = parse(text)?;
        let read = Read::of(text, &syntax)?;
        let directives: BTreeMap<usize, usize> = read.directives.iter().copied().collect();
        let mut at = 0;
        let mut conditions = BTreeMap::new();
        let mut reading = Pieces {
            tokens: &read.tokens,
            directives: &directives,
            features,
            conditions: &mut conditions,
        };
        let pieces = reading.pieces(&mut at, true);
        Ok(Lifted {
            text: text.to_string(),
            tokens: read.tokens,
            pieces,
            conditions,
        })
    }

    /// The source's tokens, each with its offset and each that starts a line
    /// after a [`NEWLINE`], in a build where `features` answers `Some(on)` for
    /// each feature: a block all of whose conditions are read and answered
    /// keeps the clause of the first that holds and loses its directives; any
    /// other is kept as written.
    fn resolve(&self, features: &dyn Fn(&str) -> Option<bool>) -> Vec<(String, usize)> {
        let mut out = Vec::new();
        self.resolve_into(&self.pieces, features, &mut out);
        out
    }

    fn resolve_into(
        &self,
        pieces: &[Piece],
        features: &dyn Fn(&str) -> Option<bool>,
        out: &mut Vec<(String, usize)>,
    ) {
        for piece in pieces {
            match piece {
                Piece::Token(token) => self.push(*token, out),
                Piece::Conditional { clauses, endif } => {
                    let decided: Option<Vec<bool>> = endif.as_ref().and(
                        clauses
                            .iter()
                            .map(|(_, condition, _)| condition.as_ref()?.value(features))
                            .collect(),
                    );
                    match decided {
                        Some(values) => {
                            if let Some(kept) = values.iter().position(|holds| *holds) {
                                self.resolve_into(&clauses[kept].2, features, out);
                            }
                        }
                        None => {
                            for (directive, _, body) in clauses {
                                directive.clone().for_each(|token| self.push(token, out));
                                self.resolve_into(body, features, out);
                            }
                            if let Some(endif) = endif {
                                endif.clone().for_each(|token| self.push(token, out));
                            }
                        }
                    }
                }
            }
        }
    }

    fn push(&self, token: usize, out: &mut Vec<(String, usize)>) {
        let token = &self.tokens[token];
        if token.starts_line {
            out.push((NEWLINE.to_string(), token.at.start));
        }
        out.push((token.spelling.clone(), token.at.start));
    }
}

/// Reads a source's tokens into pieces.
struct Pieces<'p> {
    tokens: &'p [Token],
    /// Each directive's first token, to its last.
    directives: &'p BTreeMap<usize, usize>,
    features: &'p BTreeMap<String, String>,
    conditions: &'p mut BTreeMap<String, Condition>,
}

impl Pieces<'_> {
    /// The directive at `at`: its name and its tokens.
    fn directive(&self, at: usize) -> Option<(&str, Range<usize>)> {
        let last = *self.directives.get(&at)?;
        Some((self.tokens[at].spelling.as_str(), at..last + 1))
    }

    /// The tokens from `at` as pieces, until the end or, below the top, a
    /// directive that continues or closes the block they are a clause of.
    fn pieces(&mut self, at: &mut usize, top: bool) -> Vec<Piece> {
        let tokens = self.tokens;
        let mut out = Vec::new();
        while *at < tokens.len() {
            let opens = match self.directive(*at) {
                None => false,
                Some(("#if", _)) => true,
                // A directive no `#if` opened is a token like any other.
                Some(_) if top => false,
                Some(_) => return out,
            };
            if !opens {
                out.push(Piece::Token(*at));
                *at += 1;
                continue;
            }
            let mut clauses = Vec::new();
            let mut directive = self.directive(*at).expect("a directive").1;
            let endif = loop {
                let operands = &tokens[directive.start + 1..directive.end];
                let condition = match tokens[directive.start].spelling.as_str() {
                    "#else" => operands.is_empty().then_some(Condition::Constant(true)),
                    _ => expression(operands, self.features),
                };
                if let (Some(condition), false) = (&condition, operands.is_empty()) {
                    let spelling: Vec<&str> = operands
                        .iter()
                        .map(|token| token.spelling.as_str())
                        .collect();
                    self.conditions
                        .entry(spelling.join(" "))
                        .or_insert_with(|| condition.clone());
                }
                *at = directive.end;
                let body = self.pieces(at, false);
                clauses.push((directive.clone(), condition, body));
                match self.directive(*at) {
                    Some(("#elseif" | "#else", next)) => directive = next,
                    Some(("#endif", endif)) => {
                        *at = endif.end;
                        break Some(endif);
                    }
                    _ => break None,
                }
            };
            out.push(Piece::Conditional { clauses, endif });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variant_merge::{merge, VariantSet};

    fn swift() -> Swift {
        Swift::new(["a", "b"])
    }

    fn set(variants: &[usize]) -> VariantSet {
        variants.iter().copied().collect()
    }

    /// The selector of a set of variants: variant 0 is the build with `a` on.
    fn spell(holding: &VariantSet, _within: &VariantSet) -> String {
        if holding.contains(&0) {
            "SCE_FEATURE_A".to_string()
        } else {
            "!SCE_FEATURE_A".to_string()
        }
    }

    fn merged(with_a: &str, without: &str) -> String {
        merge(
            &[(set(&[0]), with_a), (set(&[1]), without)],
            &swift(),
            &spell,
        )
        .unwrap()
    }

    fn refused(with_a: &str, without: &str) -> String {
        merge(
            &[(set(&[0]), with_a), (set(&[1]), without)],
            &swift(),
            &spell,
        )
        .unwrap_err()
        .0
    }

    /// The merge, which tree-sitter-swift reads with no error, resolved for
    /// each build, is that build's own.
    fn assert_resolves(merged: &str, with_a: &str, without: &str) {
        let dialect = swift();
        let lifted = dialect
            .lift(merged, &|_| true)
            .unwrap_or_else(|e| panic!("{e}:\n{merged}"));
        for (a, own) in [(true, with_a), (false, without)] {
            let features = |name: &str| Some(name == "a" && a);
            let reference = dialect.reference(own).unwrap();
            assert_eq!(
                dialect.difference(&lifted, &features, &reference),
                None,
                "a = {a}:\n{merged}"
            );
        }
    }

    fn spellings(text: &str) -> Vec<String> {
        let syntax = parse(text).unwrap();
        Read::of(text, &syntax)
            .unwrap()
            .tokens
            .into_iter()
            .map(|token| token.spelling)
            .collect()
    }

    #[test]
    fn a_comment_is_no_token() {
        assert_eq!(
            spellings("let a /* b /* c */ d */ = 1 // f\nlet g = 2\n"),
            ["let", "a", "=", "1", "let", "g", "=", "2"]
        );
    }

    #[test]
    fn a_source_with_an_error_is_refused() {
        assert!(unreadable("let a = (\n").is_some());
        assert!(unreadable("let a = 1\n").is_none());
    }

    #[test]
    fn a_declaration_some_variants_write_is_written_under_their_selector() {
        let with_a = "enum E {\n    case idle\n    case open\n}\n";
        let without = "enum E {\n    case idle\n}\n";
        let merged = merged(with_a, without);
        assert_eq!(
            merged,
            "enum E {\n    case idle\n    #if SCE_FEATURE_A\ncase open\n#endif\n}\n"
        );
        assert_resolves(&merged, with_a, without);
    }

    /// tree-sitter-swift reads no block among the clauses of a `switch`, so a
    /// clause only some variants write moves the choice to the `switch`.
    #[test]
    fn a_case_clause_some_variants_write_moves_the_choice_to_its_switch() {
        let with_a = "switch s {\ncase .idle:\n    a()\ncase .open:\n    b()\n}\n";
        let without = "switch s {\ncase .idle:\n    a()\n}\n";
        let merged = merged(with_a, without);
        assert_eq!(
            merged,
            format!("#if SCE_FEATURE_A\n{with_a}#elseif !SCE_FEATURE_A\n{without}#endif\n")
        );
        assert_resolves(&merged, with_a, without);
    }

    /// What a `switch` alone in a body moves the choice to is the member that
    /// holds the body: tree-sitter-swift reads a `switch` as an expression,
    /// which may be the value an implicit `return` gives.
    #[test]
    fn a_switch_alone_in_a_body_moves_the_choice_past_it() {
        let with_a = "func f(s: S) -> Int {\n    switch s {\n    case .idle:\n        return 0\n    case .open:\n        return 1\n    }\n}\n";
        let without =
            "func f(s: S) -> Int {\n    switch s {\n    case .idle:\n        return 0\n    }\n}\n";
        let merged = merged(with_a, without);
        assert_eq!(
            merged,
            format!("#if SCE_FEATURE_A\n{with_a}#elseif !SCE_FEATURE_A\n{without}#endif\n")
        );
        assert_resolves(&merged, with_a, without);
    }

    /// A block before a `switch`'s first clause is what tree-sitter-swift
    /// 0.7.4 reads as an error, which is why clauses are no list.
    #[test]
    fn the_reader_reads_no_block_among_the_clauses_of_a_switch() {
        assert!(unreadable(
            "switch s {\n#if A\ncase .open:\n    b()\n#endif\ncase .idle:\n    a()\n}\n"
        )
        .is_some());
    }

    #[test]
    fn a_statement_a_clause_holds_in_some_variants_is_selected_inside_it() {
        let with_a = "switch s {\ncase .idle:\n    a()\n    b()\ndefault:\n    break\n}\n";
        let without = "switch s {\ncase .idle:\n    a()\ndefault:\n    break\n}\n";
        let merged = merged(with_a, without);
        assert!(
            merged.contains("    a()\n    #if SCE_FEATURE_A\nb()\n#endif"),
            "{merged}"
        );
        assert_resolves(&merged, with_a, without);
    }

    #[test]
    fn the_first_statement_of_a_clause_is_selected_inside_it() {
        let with_a = "switch s {\ncase .idle:\n    b()\n    a()\ndefault:\n    break\n}\n";
        let without = "switch s {\ncase .idle:\n    a()\ndefault:\n    break\n}\n";
        let merged = merged(with_a, without);
        assert!(
            merged.contains("case .idle:\n    #if SCE_FEATURE_A\nb()\n#endif"),
            "{merged}"
        );
        assert_resolves(&merged, with_a, without);
    }

    #[test]
    fn values_that_differ_are_each_under_its_selector() {
        let with_a = "enum E: Int {\n    case done = 0\n    case open = 1\n}\n";
        let without = "enum E: Int {\n    case done = 1\n}\n";
        let merged = merged(with_a, without);
        for selected in [
            "#if SCE_FEATURE_A\ncase done = 0\n#endif",
            "#if !SCE_FEATURE_A\ncase done = 1\n#endif",
        ] {
            assert!(merged.contains(selected), "{merged}");
        }
        assert_resolves(&merged, with_a, without);
    }

    /// Members that differ inside a list no conditional may stand in are
    /// alternatives, one chain of them.
    #[test]
    fn members_that_differ_inside_no_list_are_one_chain_of_alternatives() {
        let with_a = "let n = f(1)\n";
        let without = "let n = f(2)\n";
        let merged = merged(with_a, without);
        assert_eq!(
            merged,
            "#if SCE_FEATURE_A\nlet n = f(1)\n#elseif !SCE_FEATURE_A\nlet n = f(2)\n#endif\n"
        );
        assert_resolves(&merged, with_a, without);
    }

    /// No conditional may stand among the elements of an array: the choice
    /// moves to the declaration that holds it.
    #[test]
    fn an_element_of_an_array_moves_the_choice_to_its_declaration() {
        let with_a = "let all: [S] = [\n    .idle,\n    .open,\n]\n";
        let without = "let all: [S] = [\n    .idle,\n]\n";
        let merged = merged(with_a, without);
        assert_eq!(
            merged,
            format!("#if SCE_FEATURE_A\n{with_a}#elseif !SCE_FEATURE_A\n{without}#endif\n")
        );
        assert_resolves(&merged, with_a, without);
    }

    #[test]
    fn a_statement_written_over_lines_is_one_member() {
        let with_a = "func f() {\n    let x = a\n        .b()\n    guard x else {\n        return\n    }\n    c()\n}\n";
        let without = "func f() {\n    c()\n}\n";
        let merged = merged(with_a, without);
        for selected in [
            "#if SCE_FEATURE_A\nlet x = a\n        .b()\n#endif",
            "#if SCE_FEATURE_A\nguard x else {\n        return\n    }\n#endif",
        ] {
            assert!(merged.contains(selected), "{merged}");
        }
        assert_resolves(&merged, with_a, without);
    }

    #[test]
    fn a_label_is_one_member_with_its_statement() {
        let with_a = "func f() {\n    a()\n    block: do {\n        b()\n    }\n}\n";
        let without = "func f() {\n    a()\n}\n";
        let merged = merged(with_a, without);
        assert!(
            merged.contains("#if SCE_FEATURE_A\nblock: do {\n        b()\n    }\n#endif"),
            "{merged}"
        );
        assert_resolves(&merged, with_a, without);
    }

    #[test]
    fn a_body_some_variants_leave_empty_is_merged_inside() {
        let with_a = "struct S {\n    @discardableResult\n    func f() -> Int { 1 }\n}\n";
        let without = "struct S {\n}\n";
        let merged = merged(with_a, without);
        assert!(
            merged.contains(
                "#if SCE_FEATURE_A\n@discardableResult\n    func f() -> Int { 1 }\n#endif"
            ),
            "{merged}"
        );
        assert_resolves(&merged, with_a, without);
    }

    /// The one expression of a body is the value its implicit `return` gives,
    /// and a conditional around it would leave the body without one.
    #[test]
    fn the_value_a_body_returns_moves_the_choice_to_its_member() {
        let with_a = "func f() -> Int {\n    1\n}\n";
        let without = "func f() -> Int {\n    2\n}\n";
        let merged = merged(with_a, without);
        assert_eq!(
            merged,
            format!("#if SCE_FEATURE_A\n{with_a}#elseif !SCE_FEATURE_A\n{without}#endif\n")
        );
        assert_resolves(&merged, with_a, without);
    }

    /// No conditional may stand among the accessors of a property: the choice
    /// moves to the property.
    #[test]
    fn an_accessor_moves_the_choice_to_its_property() {
        let with_a = "struct S {\n    var x: Int {\n        get { 1 }\n        set { }\n    }\n}\n";
        let without = "struct S {\n    var x: Int {\n        get { 1 }\n    }\n}\n";
        let merged = merged(with_a, without);
        assert!(
            merged.contains(
                "    #if SCE_FEATURE_A\nvar x: Int {\n        get { 1 }\n        set { }\n    }\n#elseif !SCE_FEATURE_A\nvar x: Int {"
            ),
            "{merged}"
        );
        assert_resolves(&merged, with_a, without);
    }

    #[test]
    fn a_statement_that_shares_its_line_cannot_be_selected() {
        assert!(refused("a(); b()\n", "a();\n").contains("stands where no selector may"));
    }

    #[test]
    fn a_variant_that_writes_a_directive_is_refused() {
        assert!(refused("#if os(Linux)\nlet a = 1\n#endif\n", "let a = 1\n")
            .contains("compiler directive `#if`"));
    }

    #[test]
    fn only_a_condition_over_the_features_is_resolved() {
        let dialect = swift();
        let lifted = dialect
            .lift(
                "#if (SCE_FEATURE_A && !SCE_FEATURE_B)\nx()\n#elseif os(Linux)\ny()\n#endif\n",
                &|_| true,
            )
            .unwrap();
        let features = |name: &str| Some(name == "a");
        let read: Vec<String> = lifted
            .resolve(&features)
            .into_iter()
            .map(|(spelling, _)| spelling)
            .collect();
        // `os(Linux)` is no condition over the features, so the block is kept
        // as written in every build.
        assert!(read.contains(&"#elseif".to_string()), "{read:?}");
        assert_eq!(lifted.conditions.len(), 1);
    }

    #[test]
    fn the_guard_refuses_a_build_that_breaks_an_edge() {
        let guard = swift().guard(
            &[Refusal {
                feature: "b".into(),
                requires: "a".into(),
            }],
            "m",
        );
        assert!(
            guard.ends_with(
                "#if (SCE_FEATURE_B && !SCE_FEATURE_A)\n#error(\"feature `b` requires `a`, which \
                 this build leaves out (feature model `m`)\")\n#endif\n"
            ),
            "{guard}"
        );
        let dialect = swift();
        let lifted = dialect.lift(&guard, &|_| true).unwrap();
        let admitted = |name: &str| Some(name == "a");
        assert!(dialect
            .difference(&lifted, &admitted, &dialect.reference("").unwrap())
            .is_none());
        let broken = |name: &str| Some(name == "b");
        assert!(dialect
            .difference(&lifted, &broken, &dialect.reference("").unwrap())
            .is_some());
    }
}
