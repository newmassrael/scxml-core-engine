// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Translation validation of a lifted Rust machine (ADR 0057 §9).
//!
//! A generator that keeps a product line's variability writes one source for
//! every configuration, and is correct when, for every configuration `c`,
//! resolving that source's `#[cfg]` for `c` gives the source it writes for the
//! variant `π_c(D)` alone (ADR 0057 §6). Rather than argue that each template
//! site was lifted correctly, the generator checks the law on each document it
//! generates: for each distinct variant it resolves the lifted source
//! ([`resolve`]) and compares it with the variant's own source
//! ([`first_difference`]). This is translation validation in the sense of
//! Pnueli, Siegel and Singerman ("Translation Validation", TACAS 1998): the
//! output of one run is checked against its reference, so a site not yet
//! lifted refuses the generation instead of compiling into a machine that is
//! wrong for some build.
//!
//! The source is read as Rust syntax, not as text, because an attribute governs
//! a syntax node: an item, a variant, a field, a match arm, a statement, an
//! element of an array. Only a predicate written over the product line's own
//! features is resolved; any other `#[cfg]` (the runtime's profiles, `test`) is
//! left as written on both sides. The comparison is of token streams, so
//! layout and ordinary comments, which no build reads, are not compared.

use std::collections::BTreeMap;

use quote::ToTokens;
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;
use syn::visit::{self, Visit};
use syn::visit_mut::{self, VisitMut};
use syn::{Attribute, Expr, ImplItem, Item, Stmt, TraitItem};

/// A `cfg` predicate as far as resolving it needs: over the product line's
/// features, or over something else.
enum Predicate {
    Feature(String),
    All(Vec<Predicate>),
    Any(Vec<Predicate>),
    Not(Box<Predicate>),
    /// A predicate that names anything but a feature: left unresolved.
    Other,
}

impl Predicate {
    fn read(input: ParseStream) -> syn::Result<Predicate> {
        let name: syn::Ident = input.parse()?;
        if input.peek(syn::Token![=]) {
            input.parse::<syn::Token![=]>()?;
            let value: syn::LitStr = input.parse()?;
            return Ok(if name == "feature" {
                Predicate::Feature(value.value())
            } else {
                Predicate::Other
            });
        }
        if input.peek(syn::token::Paren) {
            let content;
            syn::parenthesized!(content in input);
            let operands: Punctuated<Predicate, syn::Token![,]> =
                content.parse_terminated(Predicate::read, syn::Token![,])?;
            let operands: Vec<Predicate> = operands.into_iter().collect();
            return Ok(match name.to_string().as_str() {
                "all" => Predicate::All(operands),
                "any" => Predicate::Any(operands),
                "not" => match operands.into_iter().next() {
                    Some(operand) => Predicate::Not(Box::new(operand)),
                    None => Predicate::Other,
                },
                _ => Predicate::Other,
            });
        }
        Ok(Predicate::Other)
    }

    /// One spelling per predicate, whichever attribute wrote it.
    fn spelling(&self) -> String {
        let join = |operands: &[Predicate]| {
            operands
                .iter()
                .map(Predicate::spelling)
                .collect::<Vec<_>>()
                .join(",")
        };
        match self {
            Predicate::Feature(name) => format!("feature={name:?}"),
            Predicate::All(operands) => format!("all({})", join(operands)),
            Predicate::Any(operands) => format!("any({})", join(operands)),
            Predicate::Not(operand) => format!("not({})", operand.spelling()),
            Predicate::Other => "?".to_string(),
        }
    }

    /// The predicate's value where exactly the features `on` answers for are on,
    /// or `None` when it names anything but a feature of `features`.
    fn value(&self, features: &dyn Fn(&str) -> Option<bool>) -> Option<bool> {
        match self {
            Predicate::Feature(name) => features(name),
            Predicate::All(operands) => {
                let mut all = true;
                for operand in operands {
                    all &= operand.value(features)?;
                }
                Some(all)
            }
            Predicate::Any(operands) => {
                let mut any = false;
                for operand in operands {
                    any |= operand.value(features)?;
                }
                Some(any)
            }
            Predicate::Not(operand) => operand.value(features).map(|v| !v),
            Predicate::Other => None,
        }
    }
}

/// Resolves the product line's `#[cfg]` on every node that can carry one.
struct Resolver<'a> {
    /// For a name: `Some(on)` when it is a feature of the product line, `None`
    /// when it is not one.
    features: &'a dyn Fn(&str) -> Option<bool>,
}

impl Resolver<'_> {
    /// Strip each resolved `cfg` from `attrs`, and answer whether the node they
    /// sit on stays.
    ///
    /// A resolved `cfg_attr` is replaced by the attributes it carries where its
    /// predicate holds and dropped where it does not, as rustc expands it.
    fn keeps(&self, attrs: &mut Vec<Attribute>) -> bool {
        let mut keep = true;
        let mut resolved = Vec::with_capacity(attrs.len());
        for attr in attrs.drain(..) {
            if attr.path().is_ident("cfg") {
                if let Ok(predicate) = attr.parse_args_with(Predicate::read) {
                    if let Some(on) = predicate.value(self.features) {
                        keep &= on;
                        continue;
                    }
                }
            } else if attr.path().is_ident("cfg_attr") {
                if let Ok((predicate, carried)) = attr.parse_args_with(conditional_attributes) {
                    if let Some(on) = predicate.value(self.features) {
                        if on {
                            resolved.extend(carried.into_iter().map(|meta| Attribute {
                                pound_token: attr.pound_token,
                                style: attr.style,
                                bracket_token: attr.bracket_token,
                                meta,
                            }));
                        }
                        continue;
                    }
                }
            }
            resolved.push(attr);
        }
        *attrs = resolved;
        keep
    }
}

/// The arguments of a `cfg_attr`: its predicate and the attributes it carries.
fn conditional_attributes(input: ParseStream) -> syn::Result<(Predicate, Vec<syn::Meta>)> {
    let predicate = Predicate::read(input)?;
    input.parse::<syn::Token![,]>()?;
    let carried: Punctuated<syn::Meta, syn::Token![,]> = Punctuated::parse_terminated(input)?;
    Ok((predicate, carried.into_iter().collect()))
}

fn item_attrs(item: &mut Item) -> Option<&mut Vec<Attribute>> {
    Some(match item {
        Item::Const(i) => &mut i.attrs,
        Item::Enum(i) => &mut i.attrs,
        Item::ExternCrate(i) => &mut i.attrs,
        Item::Fn(i) => &mut i.attrs,
        Item::ForeignMod(i) => &mut i.attrs,
        Item::Impl(i) => &mut i.attrs,
        Item::Macro(i) => &mut i.attrs,
        Item::Mod(i) => &mut i.attrs,
        Item::Static(i) => &mut i.attrs,
        Item::Struct(i) => &mut i.attrs,
        Item::Trait(i) => &mut i.attrs,
        Item::TraitAlias(i) => &mut i.attrs,
        Item::Type(i) => &mut i.attrs,
        Item::Union(i) => &mut i.attrs,
        Item::Use(i) => &mut i.attrs,
        _ => return None,
    })
}

fn impl_item_attrs(item: &mut ImplItem) -> Option<&mut Vec<Attribute>> {
    Some(match item {
        ImplItem::Const(i) => &mut i.attrs,
        ImplItem::Fn(i) => &mut i.attrs,
        ImplItem::Type(i) => &mut i.attrs,
        ImplItem::Macro(i) => &mut i.attrs,
        _ => return None,
    })
}

fn trait_item_attrs(item: &mut TraitItem) -> Option<&mut Vec<Attribute>> {
    Some(match item {
        TraitItem::Const(i) => &mut i.attrs,
        TraitItem::Fn(i) => &mut i.attrs,
        TraitItem::Type(i) => &mut i.attrs,
        TraitItem::Macro(i) => &mut i.attrs,
        _ => return None,
    })
}

fn expr_attrs(expr: &mut Expr) -> Option<&mut Vec<Attribute>> {
    Some(match expr {
        Expr::Array(e) => &mut e.attrs,
        Expr::Assign(e) => &mut e.attrs,
        Expr::Async(e) => &mut e.attrs,
        Expr::Await(e) => &mut e.attrs,
        Expr::Binary(e) => &mut e.attrs,
        Expr::Block(e) => &mut e.attrs,
        Expr::Break(e) => &mut e.attrs,
        Expr::Call(e) => &mut e.attrs,
        Expr::Cast(e) => &mut e.attrs,
        Expr::Closure(e) => &mut e.attrs,
        Expr::Const(e) => &mut e.attrs,
        Expr::Continue(e) => &mut e.attrs,
        Expr::Field(e) => &mut e.attrs,
        Expr::ForLoop(e) => &mut e.attrs,
        Expr::Group(e) => &mut e.attrs,
        Expr::If(e) => &mut e.attrs,
        Expr::Index(e) => &mut e.attrs,
        Expr::Let(e) => &mut e.attrs,
        Expr::Lit(e) => &mut e.attrs,
        Expr::Loop(e) => &mut e.attrs,
        Expr::Macro(e) => &mut e.attrs,
        Expr::Match(e) => &mut e.attrs,
        Expr::MethodCall(e) => &mut e.attrs,
        Expr::Paren(e) => &mut e.attrs,
        Expr::Path(e) => &mut e.attrs,
        Expr::Range(e) => &mut e.attrs,
        Expr::Reference(e) => &mut e.attrs,
        Expr::Repeat(e) => &mut e.attrs,
        Expr::Return(e) => &mut e.attrs,
        Expr::Struct(e) => &mut e.attrs,
        Expr::Try(e) => &mut e.attrs,
        Expr::TryBlock(e) => &mut e.attrs,
        Expr::Tuple(e) => &mut e.attrs,
        Expr::Unary(e) => &mut e.attrs,
        Expr::Unsafe(e) => &mut e.attrs,
        Expr::While(e) => &mut e.attrs,
        Expr::Yield(e) => &mut e.attrs,
        _ => return None,
    })
}

fn stmt_attrs(stmt: &mut Stmt) -> Option<&mut Vec<Attribute>> {
    match stmt {
        Stmt::Local(local) => Some(&mut local.attrs),
        Stmt::Item(item) => item_attrs(item),
        Stmt::Expr(expr, _) => expr_attrs(expr),
        Stmt::Macro(m) => Some(&mut m.attrs),
    }
}

/// Keep the members of `list` whose attributes, as `attrs` finds them, survive.
fn retain_in<T, P>(
    resolver: &Resolver,
    list: &mut Punctuated<T, P>,
    attrs: impl Fn(&mut T) -> Option<&mut Vec<Attribute>>,
) where
    P: Default,
{
    let members: Vec<T> = std::mem::take(list).into_iter().collect();
    for mut member in members {
        let keep = attrs(&mut member).is_none_or(|a| resolver.keeps(a));
        if keep {
            list.push(member);
        }
    }
}

impl VisitMut for Resolver<'_> {
    fn visit_file_mut(&mut self, file: &mut syn::File) {
        file.items
            .retain_mut(|item| item_attrs(item).is_none_or(|a| self.keeps(a)));
        visit_mut::visit_file_mut(self, file);
    }

    fn visit_item_mod_mut(&mut self, module: &mut syn::ItemMod) {
        if let Some((_, items)) = &mut module.content {
            items.retain_mut(|item| item_attrs(item).is_none_or(|a| self.keeps(a)));
        }
        visit_mut::visit_item_mod_mut(self, module);
    }

    fn visit_item_impl_mut(&mut self, block: &mut syn::ItemImpl) {
        block
            .items
            .retain_mut(|item| impl_item_attrs(item).is_none_or(|a| self.keeps(a)));
        visit_mut::visit_item_impl_mut(self, block);
    }

    fn visit_item_trait_mut(&mut self, block: &mut syn::ItemTrait) {
        block
            .items
            .retain_mut(|item| trait_item_attrs(item).is_none_or(|a| self.keeps(a)));
        visit_mut::visit_item_trait_mut(self, block);
    }

    fn visit_item_enum_mut(&mut self, item: &mut syn::ItemEnum) {
        let resolver = Resolver {
            features: self.features,
        };
        retain_in(&resolver, &mut item.variants, |v| Some(&mut v.attrs));
        visit_mut::visit_item_enum_mut(self, item);
    }

    fn visit_fields_named_mut(&mut self, fields: &mut syn::FieldsNamed) {
        let resolver = Resolver {
            features: self.features,
        };
        retain_in(&resolver, &mut fields.named, |f| Some(&mut f.attrs));
        visit_mut::visit_fields_named_mut(self, fields);
    }

    fn visit_expr_match_mut(&mut self, expr: &mut syn::ExprMatch) {
        expr.arms.retain_mut(|arm| self.keeps(&mut arm.attrs));
        // An arm's comma is optional after a block and after the last arm, so
        // which arm is last depends on the build; both sides are given every
        // comma, the one spelling valid wherever an arm stands.
        for arm in &mut expr.arms {
            arm.comma.get_or_insert_with(Default::default);
        }
        visit_mut::visit_expr_match_mut(self, expr);
    }

    fn visit_block_mut(&mut self, block: &mut syn::Block) {
        block
            .stmts
            .retain_mut(|stmt| stmt_attrs(stmt).is_none_or(|a| self.keeps(a)));
        visit_mut::visit_block_mut(self, block);
    }

    fn visit_expr_array_mut(&mut self, expr: &mut syn::ExprArray) {
        let resolver = Resolver {
            features: self.features,
        };
        retain_in(&resolver, &mut expr.elems, expr_attrs);
        visit_mut::visit_expr_array_mut(self, expr);
    }

    fn visit_expr_struct_mut(&mut self, expr: &mut syn::ExprStruct) {
        let resolver = Resolver {
            features: self.features,
        };
        retain_in(&resolver, &mut expr.fields, |f| Some(&mut f.attrs));
        visit_mut::visit_expr_struct_mut(self, expr);
    }
}

/// Gathers the distinct product-line predicates a source's `#[cfg]` write.
struct Predicates<'a> {
    features: &'a dyn Fn(&str) -> Option<bool>,
    found: BTreeMap<String, Predicate>,
}

impl<'ast> Visit<'ast> for Predicates<'_> {
    fn visit_attribute(&mut self, attr: &'ast Attribute) {
        let predicate = if attr.path().is_ident("cfg") {
            attr.parse_args_with(Predicate::read).ok()
        } else if attr.path().is_ident("cfg_attr") {
            attr.parse_args_with(conditional_attributes)
                .ok()
                .map(|(predicate, _)| predicate)
        } else {
            None
        };
        if let Some(predicate) = predicate.filter(|p| p.value(self.features).is_some()) {
            self.found.entry(predicate.spelling()).or_insert(predicate);
        }
        visit::visit_attribute(self, attr);
    }
}

/// A lifted machine, read once and resolved for each build asked about.
pub struct Lifted {
    file: syn::File,
    /// Each distinct predicate over the product line's features that the
    /// source's `#[cfg]` write.
    predicates: Vec<Predicate>,
}

impl Lifted {
    /// Read `source`; `is_feature` names the product line's features. An error
    /// is the text `syn` gave for a source it could not read, which is a
    /// generator defect.
    pub fn parse(source: &str, is_feature: &dyn Fn(&str) -> bool) -> Result<Lifted, String> {
        let file = syn::parse_file(source).map_err(|e| e.to_string())?;
        let any = |name: &str| is_feature(name).then_some(false);
        let mut gather = Predicates {
            features: &any,
            found: BTreeMap::new(),
        };
        gather.visit_file(&file);
        Ok(Lifted {
            file,
            predicates: gather.found.into_values().collect(),
        })
    }

    /// The value each of the source's predicates takes in a build: two builds
    /// alike in it resolve the source alike, so the check resolves it once
    /// for each valuation a build gives it.
    pub fn valuation(&self, features: &dyn Fn(&str) -> Option<bool>) -> Vec<bool> {
        self.predicates
            .iter()
            .map(|predicate| predicate.value(features).unwrap_or(false))
            .collect()
    }

    /// The source with the `#[cfg]` over the product line's features
    /// resolved: `features` answers `Some(on)` for a feature of the product
    /// line and `None` for any other name.
    pub fn resolve(&self, features: &dyn Fn(&str) -> Option<bool>) -> syn::File {
        let mut file = self.file.clone();
        Resolver { features }.visit_file_mut(&mut file);
        file
    }
}

/// The machine a variant generates on its own, read once.
pub struct Reference(syn::File);

impl Reference {
    /// Read `source`. An error is the text `syn` gave for a source it could not
    /// read, which is a generator defect.
    pub fn parse(source: &str) -> Result<Reference, String> {
        let mut file = syn::parse_file(source).map_err(|e| e.to_string())?;
        // A variant carries no product-line predicate, so resolving it changes
        // nothing a build reads; it gives the two sides one spelling of what
        // the resolver normalises (an arm's comma).
        Resolver {
            features: &|_| None,
        }
        .visit_file_mut(&mut file);
        Ok(Reference(file))
    }
}

/// How many tokens of each side a reported difference quotes around it.
const QUOTED_CONTEXT: usize = 12;

/// How many differing units a report names.
const REPORTED_DIFFERENCES: usize = 5;

/// Where `resolved` differs from `reference`, or `None` when the two are the
/// same tokens: each unit that differs — a top-level item, or a member of an
/// `impl` — named, with the tokens around the first token that differs on each
/// side, the first [`REPORTED_DIFFERENCES`] of them. Units are matched by
/// name, so one missing on a side does not shift every comparison after it.
pub fn first_difference(resolved: &syn::File, reference: &Reference) -> Option<String> {
    let reference = &reference.0;
    if resolved.to_token_stream().to_string() == reference.to_token_stream().to_string() {
        return None;
    }
    let (ours, theirs) = (units(resolved), units(reference));
    let theirs_by_name: BTreeMap<&str, &[String]> = theirs
        .iter()
        .map(|(name, tokens)| (name.as_str(), tokens.as_slice()))
        .collect();
    let ours_by_name: BTreeMap<&str, &[String]> = ours
        .iter()
        .map(|(name, tokens)| (name.as_str(), tokens.as_slice()))
        .collect();
    let mut reported = Vec::new();
    for (name, tokens) in &ours {
        let other = theirs_by_name.get(name.as_str()).copied().unwrap_or(&[]);
        if let Some(at) = first_unequal(tokens, other) {
            reported.push(format!(
                "in {name}: the lifted machine has `{}` where the variant has `{}`",
                quoted(tokens, at),
                quoted(other, at)
            ));
        }
    }
    for (name, tokens) in &theirs {
        if !ours_by_name.contains_key(name.as_str()) {
            reported.push(format!(
                "the variant has {name}, `{}`, which the lifted machine does not",
                quoted(tokens, 0)
            ));
        }
    }
    if reported.is_empty() {
        return Some("the attributes of the file differ".to_string());
    }
    let more = reported.len().saturating_sub(REPORTED_DIFFERENCES);
    reported.truncate(REPORTED_DIFFERENCES);
    let mut said = reported.join("; ");
    if more > 0 {
        said.push_str(&format!("; and {more} more"));
    }
    Some(said)
}

/// The units a difference is reported in, each named and flattened: every
/// top-level item, an `impl` split into its header and its members. A name
/// written twice — two inherent `impl` blocks of one type — is numbered.
fn units(file: &syn::File) -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    let mut push = |name: String, tokens: Vec<String>| {
        let count = seen.entry(name.clone()).or_insert(0);
        *count += 1;
        let name = if *count == 1 {
            name
        } else {
            format!("{name} #{count}")
        };
        out.push((name, tokens));
    };
    for item in &file.items {
        let name = item_name(item);
        match item {
            Item::Impl(block) => {
                let mut header = block.clone();
                header.items.clear();
                push(name.clone(), flat_tokens(header.to_token_stream()));
                for member in &block.items {
                    push(
                        format!("{name}, {}", impl_item_name(member)),
                        flat_tokens(member.to_token_stream()),
                    );
                }
            }
            other => push(name, flat_tokens(other.to_token_stream())),
        }
    }
    out
}

/// The tokens of `stream`, groups opened, each spelled as `quote` prints it.
fn flat_tokens(stream: proc_macro2::TokenStream) -> Vec<String> {
    fn walk(stream: proc_macro2::TokenStream, out: &mut Vec<String>) {
        for tree in stream {
            match tree {
                proc_macro2::TokenTree::Group(group) => {
                    let (open, close) = match group.delimiter() {
                        proc_macro2::Delimiter::Parenthesis => ("(", ")"),
                        proc_macro2::Delimiter::Brace => ("{", "}"),
                        proc_macro2::Delimiter::Bracket => ("[", "]"),
                        proc_macro2::Delimiter::None => ("", ""),
                    };
                    out.push(open.to_string());
                    walk(group.stream(), out);
                    out.push(close.to_string());
                }
                other => out.push(other.to_string()),
            }
        }
    }
    let mut out = Vec::new();
    walk(stream, &mut out);
    out
}

/// How a report names a member of an `impl`.
fn impl_item_name(member: &ImplItem) -> String {
    match member {
        ImplItem::Const(i) => format!("const `{}`", i.ident),
        ImplItem::Fn(i) => format!("fn `{}`", i.sig.ident),
        ImplItem::Type(i) => format!("type `{}`", i.ident),
        ImplItem::Macro(i) => format!("macro `{}!`", i.mac.path.to_token_stream()),
        other => format!("`{}`", flat_tokens(other.to_token_stream()).join(" ")),
    }
}

fn first_unequal(left: &[String], right: &[String]) -> Option<usize> {
    let shared = left.len().min(right.len());
    (0..shared)
        .find(|&i| left[i] != right[i])
        .or((left.len() != right.len()).then_some(shared))
}

fn quoted(tokens: &[String], at: usize) -> String {
    let start = at.saturating_sub(QUOTED_CONTEXT);
    let end = (at + QUOTED_CONTEXT).min(tokens.len());
    if start >= end {
        return "<nothing>".to_string();
    }
    tokens[start..end].join(" ")
}

/// How a report names an item: its kind and identifier.
fn item_name(item: &Item) -> String {
    match item {
        Item::Const(i) => format!("const `{}`", i.ident),
        Item::Enum(i) => format!("enum `{}`", i.ident),
        Item::Fn(i) => format!("fn `{}`", i.sig.ident),
        Item::Impl(i) => match &i.trait_ {
            Some((_, path, _)) => format!(
                "impl `{}` for `{}`",
                path.to_token_stream(),
                i.self_ty.to_token_stream()
            ),
            None => format!("impl `{}`", i.self_ty.to_token_stream()),
        },
        Item::Macro(i) => format!("macro `{}!`", i.mac.path.to_token_stream()),
        Item::Mod(i) => format!("mod `{}`", i.ident),
        Item::Static(i) => format!("static `{}`", i.ident),
        Item::Struct(i) => format!("struct `{}`", i.ident),
        Item::Trait(i) => format!("trait `{}`", i.ident),
        Item::Type(i) => format!("type `{}`", i.ident),
        Item::Use(i) => format!("use `{}`", i.tree.to_token_stream()),
        other => format!("`{}`", flat_tokens(other.to_token_stream()).join(" ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn product_line(name: &str) -> Option<bool> {
        match name {
            "accept" => Some(false),
            "unicast" => Some(true),
            _ => None,
        }
    }

    #[test]
    fn a_product_line_cfg_is_resolved_on_every_node_it_can_sit_on() {
        let lifted = r#"
            pub enum State {
                Idle,
                #[cfg(feature = "accept")]
                Accepting,
                #[cfg(feature = "unicast")]
                Open,
            }
            #[cfg(not(feature = "accept"))]
            const DEFAULT: State = State::Idle;
            #[cfg(feature = "accept")]
            const DEFAULT: State = State::Accepting;
            fn name(s: State) -> &'static str {
                match s {
                    State::Idle => "idle",
                    #[cfg(feature = "accept")]
                    State::Accepting => "accepting",
                    #[cfg(all(feature = "unicast", not(feature = "accept")))]
                    State::Open => "open",
                }
            }
            const ALL: &[State] = &[State::Idle, #[cfg(feature = "accept")] State::Accepting];
            fn act(h: &mut H) {
                h.a();
                #[cfg(feature = "accept")]
                h.b();
            }
        "#;
        let variant = r#"
            pub enum State { Idle, Open }
            const DEFAULT: State = State::Idle;
            fn name(s: State) -> &'static str {
                match s { State::Idle => "idle", State::Open => "open" }
            }
            const ALL: &[State] = &[State::Idle];
            fn act(h: &mut H) { h.a(); }
        "#;
        assert_eq!(
            first_difference(&resolved(lifted), &reference(variant)),
            None
        );
    }

    #[test]
    fn a_cfg_over_anything_but_the_product_line_is_left_as_written() {
        let lifted = r#"
            #[cfg(feature = "std")]
            fn with_std() {}
            #[cfg(test)]
            mod tests {}
        "#;
        assert_eq!(
            first_difference(&resolved(lifted), &reference(lifted)),
            None
        );
    }

    #[test]
    fn a_site_left_unlifted_is_named_where_it_differs() {
        let lifted = "fn other() {} pub enum State { Idle, Accepting, Open }";
        let variant = "fn other() {} pub enum State { Idle, Open }";
        let difference = first_difference(&resolved(lifted), &reference(variant));
        assert_eq!(
            difference.as_deref(),
            Some(
                "in enum `State`: the lifted machine has `pub enum State { Idle , Accepting , Open }` \
                 where the variant has `pub enum State { Idle , Open }`"
            )
        );
    }

    /// The check resolves the source once for each valuation of its
    /// predicates, so the valuation must see each distinct predicate, and
    /// only those over the product line.
    #[test]
    fn a_valuation_holds_each_distinct_product_line_predicate_once() {
        let lifted = Lifted::parse(
            r#"
            #[cfg(feature = "accept")] fn a() {}
            #[cfg(feature = "accept")] fn b() {}
            #[cfg(feature = "unicast")] fn c() {}
            #[cfg(feature = "std")] fn d() {}
            #[cfg(all(feature = "std", feature = "accept"))] fn e() {}
            "#,
            &|name| product_line(name).is_some(),
        )
        .expect("reads");
        assert_eq!(lifted.valuation(&product_line), vec![false, true]);
    }

    fn resolved(lifted: &str) -> syn::File {
        Lifted::parse(lifted, &|name| product_line(name).is_some())
            .expect("reads")
            .resolve(&product_line)
    }

    fn reference(variant: &str) -> Reference {
        Reference::parse(variant).expect("reads")
    }
}
