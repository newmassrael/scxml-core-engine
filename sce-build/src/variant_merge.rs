// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! One Rust source for every build of a product line, merged from the source
//! each distinct variant generates (ADR 0057 §9).
//!
//! The variants of a document are a choice between their sources, `⟨v₁, …, vₙ⟩`
//! in the choice calculus (Erwig and Walkingshaw, "The Choice Calculus: A
//! Representation for Software Variation", TOSEM 2011), and the source for
//! every build is that choice with every part the variants share factored out
//! of it, so that what is left of the choice stands at the smallest places
//! where the variants differ. The factoring is a semistructured merge (Apel et
//! al., "Semistructured Merge: Rethinking Merge in Revision Control Systems",
//! ESEC/FSE 2011): the sources are read as Rust, each list of the syntax — the
//! items of a file, the members of an `impl`, the statements of a block, the
//! arms of a `match`, the fields and variants of a type, the fields of a
//! struct expression, the elements of an array — is aligned member by member
//! across the variants, a member they all write alike is written once, a member
//! some write is written under the selector of those that write it, and
//! members that differ only inside one list they share are merged inside it.
//! Where nothing is shared the members are alternatives, each under its own
//! selector.
//!
//! A selector stands before a member of a list, where `#[cfg]` removes it from
//! the builds it does not hold in; a member in a position no `#[cfg]` may take
//! — the value a block ends with — cannot be selected, and the choice moves to
//! the member that holds it. What is written is the variants' own text: the
//! merge cuts and joins it at the members' bounds and inserts each selector
//! before its member, so every comment a generated file carries, its `SCE-MAP`
//! markers among them, stays where the variant wrote it.
//!
//! The merge is not argued correct here; the generation checks it build by
//! build against each variant ([`crate::variant_lift::Variants::first_mismatch`]).

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use proc_macro2::{LineColumn, TokenStream, TokenTree};
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

/// The variants a version of a source stands for.
pub type VariantSet = BTreeSet<usize>;

/// The predicate of a `#[cfg]` that holds in the builds of the first set of
/// variants, as an output read only in the builds of the second asks it.
pub type Spell<'s> = dyn Fn(&VariantSet, &VariantSet) -> String + 's;

/// Why sources could not be merged: a generator defect, or a difference that
/// stands where no selector may.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unmerged(pub String);

/// `sources`, each the text one set of variants generates, merged into one
/// text whose every member stands under the selector `spell` gives for the
/// variants that write it.
pub fn merge(sources: &[(VariantSet, &str)], spell: &Spell<'_>) -> Result<String, Unmerged> {
    // Variants that wrote the same text are one version.
    let mut versions: Vec<(VariantSet, &str)> = Vec::new();
    for (set, text) in sources {
        match versions.iter_mut().find(|(_, written)| written == text) {
            Some((holding, _)) => holding.extend(set.iter().copied()),
            None => versions.push((set.clone(), text)),
        }
    }
    if let [(_, text)] = versions.as_slice() {
        return Ok(text.to_string());
    }
    let read: Vec<Source> = versions
        .iter()
        .map(|(_, text)| Source::read(text))
        .collect::<Result<_, _>>()?;
    let top: Vec<Version> = versions
        .iter()
        .zip(&read)
        .map(|((set, _), source)| Version {
            set: set.clone(),
            source,
            list: &source.lists[&0],
        })
        .collect();
    merge_list(&top, spell)
}

/// A list of the syntax a selector can stand before each member of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListKind {
    Items,
    ImplItems,
    TraitItems,
    Statements,
    Arms,
    Variants,
    Fields,
    FieldValues,
    Elements,
}

impl ListKind {
    /// Whether the members are separated by commas.
    fn separated(self) -> bool {
        matches!(
            self,
            ListKind::Arms
                | ListKind::Variants
                | ListKind::Fields
                | ListKind::FieldValues
                | ListKind::Elements
        )
    }
}

/// A member of a list in one source.
struct Member {
    /// Its text: from its first token, attributes included, to its last, the
    /// comma after it included.
    range: Range<usize>,
    tokens: TokenStream,
    /// Its tokens as one string, which says whether two members are alike.
    key: String,
    /// Whether a `#[cfg]` may stand before it.
    selectable: bool,
}

/// A list of one source.
struct List {
    kind: ListKind,
    /// Its text between its delimiters: all of the file for its items.
    content: Range<usize>,
    members: Vec<Member>,
}

/// One source, read: its text, where each line starts, and every list in it,
/// by the offset its content starts at.
struct Source<'t> {
    text: &'t str,
    lines: Vec<usize>,
    lists: BTreeMap<usize, List>,
}

impl<'t> Source<'t> {
    fn read(text: &'t str) -> Result<Self, Unmerged> {
        let file = syn::parse_file(text)
            .map_err(|e| Unmerged(format!("a variant's source does not parse: {e}")))?;
        let mut lines = vec![0];
        lines.extend(text.match_indices('\n').map(|(at, _)| at + 1));
        let mut source = Source {
            text,
            lines,
            lists: BTreeMap::new(),
        };
        let items = file
            .items
            .iter()
            .map(|item| source.member(item, true, false))
            .collect();
        source.lists.insert(
            0,
            List {
                kind: ListKind::Items,
                content: 0..text.len(),
                members: items,
            },
        );
        let mut gather = Gather {
            source: &mut source,
        };
        gather.visit_file(&file);
        Ok(source)
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

    fn member(&self, node: &dyn ToTokens, selectable: bool, separated: bool) -> Member {
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
        Member {
            range: start..end,
            key: tokens.to_string(),
            tokens,
            selectable,
        }
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

/// Records every list of a file in its source.
struct Gather<'s, 't> {
    source: &'s mut Source<'t>,
}

impl Gather<'_, '_> {
    fn record(&mut self, kind: ListKind, content: Range<usize>, members: Vec<Member>) {
        self.source.lists.insert(
            content.start,
            List {
                kind,
                content,
                members,
            },
        );
    }
}

impl<'ast> Visit<'ast> for Gather<'_, '_> {
    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        let content = self.source.content_of(&node.brace_token.span);
        let members = node
            .items
            .iter()
            .map(|item| self.source.member(item, true, false))
            .collect();
        self.record(ListKind::ImplItems, content, members);
        visit::visit_item_impl(self, node);
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        let content = self.source.content_of(&node.brace_token.span);
        let members = node
            .items
            .iter()
            .map(|item| self.source.member(item, true, false))
            .collect();
        self.record(ListKind::TraitItems, content, members);
        visit::visit_item_trait(self, node);
    }

    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if let Some((brace, items)) = &node.content {
            let content = self.source.content_of(&brace.span);
            let members = items
                .iter()
                .map(|item| self.source.member(item, true, false))
                .collect();
            self.record(ListKind::Items, content, members);
        }
        visit::visit_item_mod(self, node);
    }

    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        let content = self.source.content_of(&node.brace_token.span);
        let members = node
            .variants
            .iter()
            .map(|variant| self.source.member(variant, true, true))
            .collect();
        self.record(ListKind::Variants, content, members);
        visit::visit_item_enum(self, node);
    }

    fn visit_fields_named(&mut self, node: &'ast syn::FieldsNamed) {
        let content = self.source.content_of(&node.brace_token.span);
        let members = node
            .named
            .iter()
            .map(|field| self.source.member(field, true, true))
            .collect();
        self.record(ListKind::Fields, content, members);
        visit::visit_fields_named(self, node);
    }

    fn visit_block(&mut self, node: &'ast syn::Block) {
        let content = self.source.content_of(&node.brace_token.span);
        let last = node.stmts.len().saturating_sub(1);
        let members = node
            .stmts
            .iter()
            .enumerate()
            .map(|(index, stmt)| {
                // §9: the value a block ends with is no statement a `#[cfg]`
                // may remove.
                let tail = index == last && matches!(stmt, syn::Stmt::Expr(_, None));
                self.source.member(stmt, !tail, false)
            })
            .collect();
        self.record(ListKind::Statements, content, members);
        visit::visit_block(self, node);
    }

    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        let content = self.source.content_of(&node.brace_token.span);
        let members = node
            .arms
            .iter()
            .map(|arm| self.source.member(arm, true, true))
            .collect();
        self.record(ListKind::Arms, content, members);
        visit::visit_expr_match(self, node);
    }

    fn visit_expr_struct(&mut self, node: &'ast syn::ExprStruct) {
        let content = self.source.content_of(&node.brace_token.span);
        let members = node
            .fields
            .iter()
            .map(|field| self.source.member(field, true, true))
            .collect();
        self.record(ListKind::FieldValues, content, members);
        visit::visit_expr_struct(self, node);
    }

    fn visit_expr_array(&mut self, node: &'ast syn::ExprArray) {
        let content = self.source.content_of(&node.bracket_token.span);
        let members = node
            .elems
            .iter()
            .map(|elem| self.source.member(elem, true, true))
            .collect();
        self.record(ListKind::Elements, content, members);
        visit::visit_expr_array(self, node);
    }
}

/// One version of a list: the variants that write it alike, the source of one
/// of them, and the list in that source.
#[derive(Clone)]
struct Version<'v> {
    set: VariantSet,
    source: &'v Source<'v>,
    list: &'v List,
}

/// One position of the merged list: each distinct member written there, with
/// the variants that write it.
struct Slot {
    groups: Vec<Group>,
}

/// A member several versions write alike.
struct Group {
    set: VariantSet,
    /// The member in each version that writes it: (version, member).
    at: Vec<(usize, usize)>,
}

impl Slot {
    fn key<'a>(&self, versions: &'a [Version<'_>], group: usize) -> &'a str {
        let (version, member) = self.groups[group].at[0];
        &versions[version].list.members[member].key
    }

    fn has_key(&self, versions: &[Version<'_>], key: &str) -> Option<usize> {
        (0..self.groups.len()).find(|&group| self.key(versions, group) == key)
    }
}

/// The merged content of the list `versions` are versions of.
fn merge_list(versions: &[Version<'_>], spell: &Spell<'_>) -> Result<String, Unmerged> {
    let kind = versions[0].list.kind;
    if versions.iter().any(|version| version.list.kind != kind) {
        return Err(Unmerged(
            "the variants write different lists at one place".into(),
        ));
    }
    let within: VariantSet = versions
        .iter()
        .flat_map(|version| version.set.iter().copied())
        .collect();
    let slots = align(versions);
    // The version that stands for the most variants lends the text between
    // members: the layout and comments the most builds read.
    let lender = (0..versions.len())
        .max_by_key(|&version| versions[version].set.len())
        .expect("a list has a version");
    let mut out = String::new();
    for slot in &slots {
        let (version, member) = slot_lead_source(slot);
        let lead = lead_of(&versions[version], member);
        let indent = lead
            .rsplit('\n')
            .next()
            .filter(|last| lead.contains('\n') && last.chars().all(char::is_whitespace))
            .unwrap_or("");
        let written = write_slot(versions, slot, &within, spell, indent)?;
        // Two members that each start their own version's list have no text
        // between them; they are put on lines of their own.
        if lead.is_empty() && out.chars().last().is_some_and(|c| !c.is_whitespace()) {
            out.push('\n');
        }
        out.push_str(lead);
        out.push_str(&written);
        if kind.separated() && !written.trim_end().ends_with(',') {
            out.push(',');
        }
    }
    let lent = &versions[lender];
    let tail_start = lent
        .list
        .members
        .last()
        .map_or(lent.list.content.start, |member| member.range.end);
    out.push_str(&lent.source.text[tail_start..lent.list.content.end]);
    Ok(out)
}

/// The member whose leading text a slot is written after: that of the group
/// that stands for the most variants.
fn slot_lead_source(slot: &Slot) -> (usize, usize) {
    let group = slot
        .groups
        .iter()
        .max_by_key(|group| group.set.len())
        .expect("a slot has a group");
    group.at[0]
}

/// The text between a member and the one before it in its list.
fn lead_of<'v>(version: &Version<'v>, member: usize) -> &'v str {
    let members = &version.list.members;
    let start = if member == 0 {
        version.list.content.start
    } else {
        members[member - 1].range.end
    };
    &version.source.text[start..members[member].range.start]
}

fn member_text<'v>(versions: &[Version<'v>], (version, member): (usize, usize)) -> &'v str {
    let version = &versions[version];
    &version.source.text[version.list.members[member].range.clone()]
}

/// A slot, written: each of its members under its selector, each on a line of
/// its own at `indent`, or the members merged inside the one list they differ
/// in.
fn write_slot(
    versions: &[Version<'_>],
    slot: &Slot,
    within: &VariantSet,
    spell: &Spell<'_>,
    indent: &str,
) -> Result<String, Unmerged> {
    let selected = |set: &VariantSet, at: (usize, usize), text: &str| {
        if set.is_superset(within) {
            return Ok(text.to_string());
        }
        let member = &versions[at.0].list.members[at.1];
        if !member.selectable {
            return Err(Unmerged(format!(
                "a member only some variants write stands where no selector may: {}",
                member.key
            )));
        }
        Ok(format!("#[cfg({})] {text}", spell(set, within)))
    };
    if let [group] = slot.groups.as_slice() {
        return selected(&group.set, group.at[0], member_text(versions, group.at[0]));
    }
    let union: VariantSet = slot
        .groups
        .iter()
        .flat_map(|group| group.set.iter().copied())
        .collect();
    if let Some(merged) = merge_inside(versions, slot, spell)? {
        let lender = slot_lead_source(slot);
        return selected(&union, lender, &merged);
    }
    let mut out = String::new();
    for (index, group) in slot.groups.iter().enumerate() {
        if index > 0 {
            out.push('\n');
            out.push_str(indent);
        }
        out.push_str(&selected(
            &group.set,
            group.at[0],
            member_text(versions, group.at[0]),
        )?);
        if versions[group.at[0].0].list.kind.separated() && !out.trim_end().ends_with(',') {
            out.push(',');
        }
    }
    Ok(out)
}

/// The slot's members merged inside the deepest list outside which they are
/// alike, written as the member that holds the most variants writes it with
/// that list's content replaced; `None` when they differ outside every list.
fn merge_inside(
    versions: &[Version<'_>],
    slot: &Slot,
    spell: &Spell<'_>,
) -> Result<Option<String>, Unmerged> {
    let members: Vec<(usize, &Member)> = slot
        .groups
        .iter()
        .map(|group| {
            let (version, member) = group.at[0];
            (version, &versions[version].list.members[member])
        })
        .collect();
    // Each list the members share, outermost first, as the offset its content
    // starts at in each member's source.
    let mut shared: Vec<Vec<usize>> = Vec::new();
    let mut streams: Vec<TokenStream> = members.iter().map(|(_, m)| m.tokens.clone()).collect();
    loop {
        let trees: Vec<Vec<TokenTree>> = streams
            .iter()
            .map(|s| s.clone().into_iter().collect())
            .collect();
        let length = trees[0].len();
        if trees.iter().any(|t| t.len() != length) {
            break;
        }
        let differing: Vec<usize> = (0..length)
            .filter(|&i| {
                trees
                    .iter()
                    .any(|t| t[i].to_string() != trees[0][i].to_string())
            })
            .collect();
        let [position] = differing.as_slice() else {
            break;
        };
        let groups: Option<Vec<&proc_macro2::Group>> = trees
            .iter()
            .map(|t| match &t[*position] {
                TokenTree::Group(group) => Some(group),
                _ => None,
            })
            .collect();
        let Some(groups) = groups else {
            break;
        };
        if groups
            .iter()
            .any(|g| g.delimiter() != groups[0].delimiter())
        {
            break;
        }
        shared.push(
            groups
                .iter()
                .zip(&members)
                .map(|(group, (version, _))| {
                    versions[*version].source.offset(group.span_open().end())
                })
                .collect(),
        );
        streams = groups.iter().map(|group| group.stream()).collect();
    }
    for offsets in shared.iter().rev() {
        let lists: Option<Vec<&List>> = offsets
            .iter()
            .zip(&members)
            .map(|(offset, (version, _))| versions[*version].source.lists.get(offset))
            .collect();
        let Some(lists) = lists else {
            continue;
        };
        let inner: Vec<Version> = slot
            .groups
            .iter()
            .zip(&lists)
            .zip(&members)
            .map(|((group, list), (version, _))| Version {
                set: group.set.clone(),
                source: versions[*version].source,
                list,
            })
            .collect();
        let Ok(content) = merge_list(&inner, spell) else {
            continue;
        };
        let lender = (0..members.len())
            .max_by_key(|&i| slot.groups[i].set.len())
            .expect("a slot has a group");
        let (version, member) = members[lender];
        let text = versions[version].source.text;
        let list = lists[lender];
        return Ok(Some(format!(
            "{}{content}{}",
            &text[member.range.start..list.content.start],
            &text[list.content.end..member.range.end]
        )));
    }
    Ok(None)
}

/// The positions of the merged list: the versions' members aligned, each
/// version in turn against those before it. Members alike are aligned by the
/// longest common subsequence of their keys; between two aligned members, a
/// member of the new version is put beside one it differs from only inside a
/// list they share, which the write merges inside it, and the rest are
/// positions of their own. The order of every version is kept: two members of
/// one version are in its order, and two that no version writes together are
/// in no build together, so either order is the order of every build.
fn align(versions: &[Version<'_>]) -> Vec<Slot> {
    let mut slots: Vec<Slot> = (0..versions[0].list.members.len())
        .map(|member| Slot {
            groups: vec![Group {
                set: versions[0].set.clone(),
                at: vec![(0, member)],
            }],
        })
        .collect();
    for version in 1..versions.len() {
        let members = &versions[version].list.members;
        let pairs = longest_common(slots.len(), members.len(), |s, m| {
            slots[s].has_key(versions, &members[m].key).is_some()
        });
        let mut next: Vec<Slot> = Vec::new();
        let (mut s, mut m) = (0, 0);
        let mut taken: Vec<Option<Slot>> = slots.into_iter().map(Some).collect();
        for &(ps, pm) in pairs
            .iter()
            .chain(std::iter::once(&(taken.len(), members.len())))
        {
            // The gap before this pair: unmatched slots s..ps, members m..pm.
            let gap_slots: Vec<Slot> = (s..ps).map(|i| taken[i].take().expect("a slot")).collect();
            next.extend(place_gap(versions, version, gap_slots, m..pm));
            if ps < taken.len() {
                let mut slot = taken[ps].take().expect("a slot");
                let group = slot
                    .has_key(versions, &members[pm].key)
                    .expect("an aligned slot has the key");
                slot.groups[group]
                    .set
                    .extend(versions[version].set.iter().copied());
                slot.groups[group].at.push((version, pm));
                next.push(slot);
            }
            s = ps + 1;
            m = pm + 1;
        }
        slots = next;
    }
    slots
}

/// The slots of a gap between two aligned positions: `gap` of the slots so
/// far, and members `members` of `version`, which none of them writes alike.
fn place_gap(
    versions: &[Version<'_>],
    version: usize,
    gap: Vec<Slot>,
    members: Range<usize>,
) -> Vec<Slot> {
    let new: Vec<usize> = members.collect();
    let pairs = longest_common(gap.len(), new.len(), |s, m| {
        differs_inside_a_list(versions, &gap[s], (version, new[m]))
    });
    let mut out = Vec::new();
    let mut gap: Vec<Option<Slot>> = gap.into_iter().map(Some).collect();
    let (mut s, mut m) = (0, 0);
    for &(ps, pm) in pairs.iter().chain(std::iter::once(&(gap.len(), new.len()))) {
        for slot in gap.iter_mut().take(ps).skip(s) {
            out.push(slot.take().expect("a slot"));
        }
        for &member in &new[m..pm] {
            out.push(Slot {
                groups: vec![Group {
                    set: versions[version].set.clone(),
                    at: vec![(version, member)],
                }],
            });
        }
        if ps < gap.len() {
            let mut slot = gap[ps].take().expect("a slot");
            slot.groups.push(Group {
                set: versions[version].set.clone(),
                at: vec![(version, new[pm])],
            });
            out.push(slot);
        }
        s = ps + 1;
        m = pm + 1;
    }
    out
}

/// Whether a member differs from a slot's first member only inside a group of
/// tokens they share at the same place.
fn differs_inside_a_list(versions: &[Version<'_>], slot: &Slot, member: (usize, usize)) -> bool {
    let (version, index) = slot.groups[0].at[0];
    let ours: Vec<TokenTree> = versions[version].list.members[index]
        .tokens
        .clone()
        .into_iter()
        .collect();
    let theirs: Vec<TokenTree> = versions[member.0].list.members[member.1]
        .tokens
        .clone()
        .into_iter()
        .collect();
    if ours.len() != theirs.len() {
        return false;
    }
    let differing: Vec<usize> = (0..ours.len())
        .filter(|&i| ours[i].to_string() != theirs[i].to_string())
        .collect();
    match differing.as_slice() {
        [position] => matches!(
            (&ours[*position], &theirs[*position]),
            (TokenTree::Group(a), TokenTree::Group(b)) if a.delimiter() == b.delimiter()
        ),
        _ => false,
    }
}

/// The pairs `(i, j)`, increasing in both, of a longest common subsequence of
/// two sequences of lengths `n` and `m` under `alike`.
fn longest_common(n: usize, m: usize, alike: impl Fn(usize, usize) -> bool) -> Vec<(usize, usize)> {
    let mut table = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            table[i][j] = if alike(i, j) {
                table[i + 1][j + 1] + 1
            } else {
                table[i + 1][j].max(table[i][j + 1])
            };
        }
    }
    let mut pairs = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if alike(i, j) && table[i][j] == table[i + 1][j + 1] + 1 {
            pairs.push((i, j));
            i += 1;
            j += 1;
        } else if table[i + 1][j] >= table[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn a_member_some_variants_write_is_written_under_their_selector() {
        let with = "pub enum State {\n    Idle,\n    Open,\n}\n";
        let without = "pub enum State {\n    Idle,\n}\n";
        let merged = merge(&[(set(&[0]), with), (set(&[1]), without)], &spell).unwrap();
        assert_eq!(
            merged,
            "pub enum State {\n    Idle,\n    #[cfg(feature = \"v0\")] Open,\n}\n"
        );
    }

    #[test]
    fn members_that_differ_inside_a_body_are_merged_inside_it() {
        let a = "impl P {\n    fn f(&self) {\n        a();\n        // SCE-MAP: d.scxml:3\n        b();\n    }\n}\n";
        let b = "impl P {\n    fn f(&self) {\n        a();\n    }\n}\n";
        let merged = merge(&[(set(&[0]), a), (set(&[1]), b)], &spell).unwrap();
        assert_eq!(
            merged,
            "impl P {\n    fn f(&self) {\n        a();\n        // SCE-MAP: d.scxml:3\n        #[cfg(feature = \"v0\")] b();\n    }\n}\n"
        );
    }

    #[test]
    fn values_that_differ_are_alternatives_under_their_selectors() {
        let a = "const N: usize = 1;\n";
        let b = "const N: usize = 2;\n";
        let merged = merge(&[(set(&[0]), a), (set(&[1]), b)], &spell).unwrap();
        assert_eq!(
            merged,
            "#[cfg(feature = \"v0\")] const N: usize = 1;\n#[cfg(feature = \"v1\")] const N: usize = 2;\n"
        );
    }

    #[test]
    fn a_value_a_block_ends_with_moves_the_choice_to_its_member() {
        let a = "fn f() -> u8 {\n    1\n}\n";
        let b = "fn f() -> u8 {\n    2\n}\n";
        let merged = merge(&[(set(&[0]), a), (set(&[1]), b)], &spell).unwrap();
        assert_eq!(
            merged,
            "#[cfg(feature = \"v0\")] fn f() -> u8 {\n    1\n}\n#[cfg(feature = \"v1\")] fn f() -> u8 {\n    2\n}\n"
        );
    }
}
