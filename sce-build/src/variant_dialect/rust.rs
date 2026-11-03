// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Rust dialect of the output for every build (ADR 0057 §9 and §10).
//!
//! The reader reads Rust with `syn`. A `#[cfg]` governs a syntax node, so the
//! lists a selector may stand in are the lists of the syntax: the items of a
//! file or a module, the members of an `impl` or a trait, the statements of a
//! block, the arms of a `match`, the fields and variants of a type, the fields
//! of a struct expression, the elements of an array. The value a block ends
//! with is no statement a `#[cfg]` may remove, so it cannot be selected.
//!
//! The writer writes `#[cfg(P)]` before the member, `P` over Cargo features,
//! and the alternatives of one place each under its own. The resolver is
//! [`crate::lift_validation`]; the guard is a `compile_error!` for each edge
//! of the feature model.

use std::ops::Range;

use proc_macro2::{Delimiter, LineColumn, Spacing, TokenStream, TokenTree};
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

use super::Dialect;
use crate::feature_selector::{Refusal, Selector};
use crate::generator::Language;
use crate::lift_validation::{first_difference, Lifted, Reference};
use crate::variant_merge::{Interner, List, Member, Source, Tree, Unmerged};

/// Rust, as the Rust backend writes it.
#[derive(Debug, Clone, Copy, Default)]
pub struct Rust;

impl Dialect for Rust {
    type Lifted = Lifted;
    type Reference = Reference;

    fn backend(&self) -> Language {
        Language::Rust
    }

    fn name(&self) -> &'static str {
        "Rust"
    }

    fn is_source(&self, file: &str) -> bool {
        file.ends_with(".rs")
    }

    fn spell(&self, selector: &Selector) -> String {
        selector.rust()
    }

    fn read<'t>(&self, text: &'t str, trees: &mut Interner) -> Result<Source<'t>, Unmerged> {
        let file = syn::parse_file(text)
            .map_err(|e| Unmerged(format!("a variant's source does not parse: {e}")))?;
        let mut lines = vec![0];
        lines.extend(text.match_indices('\n').map(|(at, _)| at + 1));
        let mut gather = Gather {
            text,
            lines,
            trees,
            source: Source {
                text,
                top: 0,
                lists: Default::default(),
            },
        };
        let items = file
            .items
            .iter()
            .map(|item| gather.member(item, true, false))
            .collect();
        gather.record("items", false, 0..text.len(), items);
        gather.visit_file(&file);
        Ok(gather.source)
    }

    fn select(&self, predicate: &str, member: &str) -> String {
        format!("#[cfg({predicate})] {member}")
    }

    fn alternatives(&self, alternatives: &[(String, String)], indent: &str) -> String {
        alternatives
            .iter()
            .map(|(predicate, member)| self.select(predicate, member))
            .collect::<Vec<_>>()
            .join(&format!("\n{indent}"))
    }

    fn guarded_file(&self, stem: &str) -> String {
        format!("{stem}_sm.rs")
    }

    fn guard(&self, refusals: &[Refusal], model: &str) -> String {
        let mut guard = format!(
            "// The builds the feature model `{model}` admits (docs/adr/0057 §6): a build\n\
             // that turns a feature on and leaves out one it requires compiles none of this.\n"
        );
        for refusal in refusals {
            guard.push_str(&refusal.rust(model));
        }
        guard
    }

    fn lift(&self, text: &str, is_feature: &dyn Fn(&str) -> bool) -> Result<Lifted, String> {
        Lifted::parse(text, is_feature)
    }

    fn reference(&self, text: &str) -> Result<Reference, String> {
        Reference::parse(text)
    }

    fn valuation(&self, lifted: &Lifted, features: &dyn Fn(&str) -> Option<bool>) -> Vec<bool> {
        lifted.valuation(features)
    }

    fn difference(
        &self,
        lifted: &Lifted,
        features: &dyn Fn(&str) -> Option<bool>,
        reference: &Reference,
    ) -> Option<String> {
        first_difference(&lifted.resolve(features), reference)
    }
}

/// Reads every list of a file into its source.
struct Gather<'g, 't> {
    text: &'t str,
    /// Where each line of `text` starts.
    lines: Vec<usize>,
    trees: &'g mut Interner,
    source: Source<'t>,
}

impl Gather<'_, '_> {
    fn record(
        &mut self,
        kind: &'static str,
        separated: bool,
        content: Range<usize>,
        members: Vec<Member>,
    ) {
        self.source.lists.insert(
            content.start,
            List {
                kind,
                separated,
                content,
                members,
            },
        );
    }

    fn offset(&self, at: LineColumn) -> usize {
        let start = self.lines[at.line - 1];
        let rest = &self.text[start..];
        start
            + rest
                .char_indices()
                .nth(at.column)
                .map_or(rest.len(), |(offset, _)| offset)
    }

    /// A member: its text from its first token, attributes included, to its
    /// last, the comma after it included when the list is `separated`.
    fn member(&mut self, node: &dyn ToTokens, selectable: bool, separated: bool) -> Member {
        let tokens = node.to_token_stream();
        let span = tokens_span(&tokens);
        let start = self.offset(span.0);
        let mut end = self.offset(span.1);
        if separated {
            let after = &self.text[end..];
            let skipped = after.len() - after.trim_start().len();
            if after[skipped..].starts_with(',') {
                end += skipped + 1;
            }
        }
        let trees = self.trees_of(tokens);
        self.trees.member(start..end, trees, selectable)
    }

    /// `tokens` as the merge compares them: two tokens are one when they print
    /// alike and, for punctuation, join the next token alike.
    fn trees_of(&mut self, tokens: TokenStream) -> Vec<Tree> {
        tokens
            .into_iter()
            .map(|tree| match tree {
                TokenTree::Group(group) => {
                    let delimiter = match group.delimiter() {
                        Delimiter::Parenthesis => '(',
                        Delimiter::Brace => '{',
                        Delimiter::Bracket => '[',
                        Delimiter::None => ' ',
                    };
                    let content = self.offset(group.span_open().end());
                    let inner = self.trees_of(group.stream());
                    self.trees.group(delimiter, content, inner)
                }
                TokenTree::Punct(punct) => self.trees.token(match punct.spacing() {
                    Spacing::Joint => format!("{}\u{1}", punct.as_char()),
                    Spacing::Alone => punct.as_char().to_string(),
                }),
                other => self.trees.token(other.to_string()),
            })
            .collect()
    }

    fn content_of(&self, delimiters: &proc_macro2::extra::DelimSpan) -> Range<usize> {
        self.offset(delimiters.open().end())..self.offset(delimiters.close().start())
    }
}

/// Where the first token of `tokens` starts and the last ends.
fn tokens_span(tokens: &TokenStream) -> (LineColumn, LineColumn) {
    let mut trees = tokens.clone().into_iter();
    let first = trees.next().map(|tree| tree.span());
    let last = tokens.clone().into_iter().last().map(|tree| tree.span());
    match (first, last) {
        (Some(first), Some(last)) => (first.start(), last.end()),
        _ => {
            let span = tokens.span();
            (span.start(), span.end())
        }
    }
}

impl<'ast> Visit<'ast> for Gather<'_, '_> {
    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        let content = self.content_of(&node.brace_token.span);
        let members = node
            .items
            .iter()
            .map(|item| self.member(item, true, false))
            .collect();
        self.record("impl items", false, content, members);
        visit::visit_item_impl(self, node);
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        let content = self.content_of(&node.brace_token.span);
        let members = node
            .items
            .iter()
            .map(|item| self.member(item, true, false))
            .collect();
        self.record("trait items", false, content, members);
        visit::visit_item_trait(self, node);
    }

    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if let Some((brace, items)) = &node.content {
            let content = self.content_of(&brace.span);
            let members = items
                .iter()
                .map(|item| self.member(item, true, false))
                .collect();
            self.record("items", false, content, members);
        }
        visit::visit_item_mod(self, node);
    }

    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        let content = self.content_of(&node.brace_token.span);
        let members = node
            .variants
            .iter()
            .map(|variant| self.member(variant, true, true))
            .collect();
        self.record("variants", true, content, members);
        visit::visit_item_enum(self, node);
    }

    fn visit_fields_named(&mut self, node: &'ast syn::FieldsNamed) {
        let content = self.content_of(&node.brace_token.span);
        let members = node
            .named
            .iter()
            .map(|field| self.member(field, true, true))
            .collect();
        self.record("fields", true, content, members);
        visit::visit_fields_named(self, node);
    }

    fn visit_block(&mut self, node: &'ast syn::Block) {
        let content = self.content_of(&node.brace_token.span);
        let last = node.stmts.len().saturating_sub(1);
        let members = node
            .stmts
            .iter()
            .enumerate()
            .map(|(index, stmt)| {
                // §9: the value a block ends with is no statement a `#[cfg]`
                // may remove.
                let tail = index == last && matches!(stmt, syn::Stmt::Expr(_, None));
                self.member(stmt, !tail, false)
            })
            .collect();
        self.record("statements", false, content, members);
        visit::visit_block(self, node);
    }

    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        let content = self.content_of(&node.brace_token.span);
        let members = node
            .arms
            .iter()
            .map(|arm| self.member(arm, true, true))
            .collect();
        self.record("arms", true, content, members);
        visit::visit_expr_match(self, node);
    }

    fn visit_expr_struct(&mut self, node: &'ast syn::ExprStruct) {
        let content = self.content_of(&node.brace_token.span);
        let members = node
            .fields
            .iter()
            .map(|field| self.member(field, true, true))
            .collect();
        self.record("field values", true, content, members);
        visit::visit_expr_struct(self, node);
    }

    fn visit_expr_array(&mut self, node: &'ast syn::ExprArray) {
        let content = self.content_of(&node.bracket_token.span);
        let members = node
            .elems
            .iter()
            .map(|elem| self.member(elem, true, true))
            .collect();
        self.record("elements", true, content, members);
        visit::visit_expr_array(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variant_merge::{merge, VariantSet};

    fn set(variants: &[usize]) -> VariantSet {
        variants.iter().copied().collect()
    }

    /// The selector of a set of variants, spelled by their indices.
    fn spell(holding: &VariantSet, _within: &VariantSet) -> String {
        let names: Vec<String> = holding
            .iter()
            .map(|v| format!("feature = \"v{v}\""))
            .collect();
        if names.len() == 1 {
            names[0].clone()
        } else {
            format!("any({})", names.join(", "))
        }
    }

    fn merged(sources: &[(VariantSet, &str)]) -> String {
        merge(sources, &Rust, &spell).unwrap()
    }

    #[test]
    fn a_member_some_variants_write_is_written_under_their_selector() {
        let with = "pub enum State {\n    Idle,\n    Open,\n}\n";
        let without = "pub enum State {\n    Idle,\n}\n";
        assert_eq!(
            merged(&[(set(&[0]), with), (set(&[1]), without)]),
            "pub enum State {\n    Idle,\n    #[cfg(feature = \"v0\")] Open,\n}\n"
        );
    }

    #[test]
    fn members_that_differ_inside_a_body_are_merged_inside_it() {
        let a = "impl P {\n    fn f(&self) {\n        a();\n        // SCE-MAP: d.scxml:3\n        b();\n    }\n}\n";
        let b = "impl P {\n    fn f(&self) {\n        a();\n    }\n}\n";
        assert_eq!(
            merged(&[(set(&[0]), a), (set(&[1]), b)]),
            "impl P {\n    fn f(&self) {\n        a();\n        // SCE-MAP: d.scxml:3\n        #[cfg(feature = \"v0\")] b();\n    }\n}\n"
        );
    }

    #[test]
    fn values_that_differ_are_alternatives_under_their_selectors() {
        let a = "const N: usize = 1;\n";
        let b = "const N: usize = 2;\n";
        assert_eq!(
            merged(&[(set(&[0]), a), (set(&[1]), b)]),
            "#[cfg(feature = \"v0\")] const N: usize = 1;\n#[cfg(feature = \"v1\")] const N: usize = 2;\n"
        );
    }

    #[test]
    fn a_value_a_block_ends_with_moves_the_choice_to_its_member() {
        let a = "fn f() -> u8 {\n    1\n}\n";
        let b = "fn f() -> u8 {\n    2\n}\n";
        assert_eq!(
            merged(&[(set(&[0]), a), (set(&[1]), b)]),
            "#[cfg(feature = \"v0\")] fn f() -> u8 {\n    1\n}\n#[cfg(feature = \"v1\")] fn f() -> u8 {\n    2\n}\n"
        );
    }

    /// A member's tokens are those `syn` prints for its syntax, so two members
    /// are alike exactly when their syntax is, whatever their layout: `1 --1`
    /// and `1 - -1` are one expression, and a member that names another value
    /// or another path is another member.
    #[test]
    fn members_are_alike_exactly_when_their_syntax_is() {
        let mut trees = Interner::default();
        let mut key =
            |text: &'static str| Rust.read(text, &mut trees).unwrap().lists[&0].members[0].key();
        assert_eq!(key("const A : u8 = 1 ;\n"), key("const A: u8 = 1;\n"));
        assert_eq!(
            key("const A: i8 = 1 --1;\n"),
            key("const A: i8 = 1 - -1;\n")
        );
        assert_ne!(key("const A: u8 = 1;\n"), key("const A: u8 = 2;\n"));
        assert_ne!(key("type T = a::b;\n"), key("type T = a::c;\n"));
    }
}
