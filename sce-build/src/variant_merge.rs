// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! One source for every build of a product line, merged from the source each
//! distinct variant generates (ADR 0057 §9 and §10).
//!
//! The variants of a document are a choice between their sources, `⟨v₁, …, vₙ⟩`
//! in the choice calculus (Erwig and Walkingshaw, "The Choice Calculus: A
//! Representation for Software Variation", TOSEM 2011), and the source for
//! every build is that choice with every part the variants share factored out
//! of it, so that what is left of the choice stands at the smallest places
//! where the variants differ. The factoring is a semistructured merge (Apel et
//! al., "Semistructured Merge: Rethinking Merge in Revision Control Systems",
//! ESEC/FSE 2011): each list of a source a selector may stand in is aligned
//! member by member across the variants, a member they all write alike is
//! written once, a member some write is written under the selector of those
//! that write it, and members that differ only inside one list they share are
//! merged inside it. Where nothing is shared the members are alternatives,
//! each under its own selector.
//!
//! This module is the part of the merge no language decides. What a language
//! decides is a [`Dialect`]: its reader says which lists a source holds and
//! which members a selector may stand before, and its writer spells a member
//! under a selector. A member that some variants write, in a position no
//! selector may take, cannot be selected, and the choice moves to the member
//! that holds it. What is written is the variants' own text: the merge cuts and
//! joins it at the members' bounds and the writer puts each selector at its
//! member, so every comment a generated file carries, its `SCE-MAP` markers
//! among them, stays where the variant wrote it.
//!
//! The merge is not argued correct here; the generation checks it build by
//! build against each variant ([`crate::variant_lift::Variants::first_mismatch`]).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ops::Range;

use crate::variant_dialect::Dialect;

/// The variants a version of a source stands for.
pub type VariantSet = BTreeSet<usize>;

/// The predicate, as the dialect spells it, that holds in the builds of the
/// first set of variants, as an output read only in the builds of the second
/// asks it.
pub type Spell<'s> = dyn Fn(&VariantSet, &VariantSet) -> String + 's;

/// Why sources could not be merged: a generator defect, or a difference that
/// stands where no selector may.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unmerged(pub String);

/// `sources`, each the text one set of variants generates in `dialect`,
/// merged into one text whose every member stands under the selector `spell`
/// gives for the variants that write it.
pub fn merge<D: Dialect + ?Sized>(
    sources: &[(VariantSet, &str)],
    dialect: &D,
    spell: &Spell<'_>,
) -> Result<String, Unmerged> {
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
    let mut trees = Interner::default();
    let read: Vec<Source> = versions
        .iter()
        .map(|(_, text)| dialect.read(text, &mut trees))
        .collect::<Result<_, _>>()?;
    let top: Vec<Version> = versions
        .iter()
        .zip(&read)
        .map(|((set, _), source)| Version {
            set: set.clone(),
            source,
            list: &source.lists[&source.top],
        })
        .collect();
    Merge { dialect, spell }.list(&top)
}

/// The identity of a token tree among all the sources one merge reads: two
/// trees have one id exactly when they are the same tokens, so the merge
/// compares members and the groups inside them by id (hash-consing).
pub type TreeId = u32;

/// The ids of the token trees of the sources one merge reads.
#[derive(Debug, Default)]
pub struct Interner {
    ids: HashMap<Shape, TreeId>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum Shape {
    Token(String),
    Group(char, Vec<TreeId>),
    Run(Vec<TreeId>),
}

impl Interner {
    fn intern(&mut self, shape: Shape) -> TreeId {
        let next = TreeId::try_from(self.ids.len()).expect("fewer token trees than ids");
        *self.ids.entry(shape).or_insert(next)
    }

    /// A token, by a spelling the dialect gives each token that two tokens
    /// share exactly when they are the same token.
    pub fn token(&mut self, spelling: String) -> Tree {
        Tree {
            id: self.intern(Shape::Token(spelling)),
            group: None,
        }
    }

    /// The tokens `trees` between the delimiter `delimiter` and its match, the
    /// text between the two starting at the offset `content`.
    pub fn group(&mut self, delimiter: char, content: usize, trees: Vec<Tree>) -> Tree {
        let id = self.intern(Shape::Group(
            delimiter,
            trees.iter().map(|tree| tree.id).collect(),
        ));
        Tree {
            id,
            group: Some(Group {
                delimiter,
                content,
                trees,
            }),
        }
    }

    /// The member of a list whose text is `range` and whose tokens are
    /// `trees`; `selectable` says whether a selector may stand at it.
    pub fn member(&mut self, range: Range<usize>, trees: Vec<Tree>, selectable: bool) -> Member {
        let key = self.intern(Shape::Run(trees.iter().map(|tree| tree.id).collect()));
        Member {
            range,
            trees,
            key,
            selectable,
        }
    }
}

/// A token, or tokens between a delimiter and its match.
#[derive(Debug, Clone)]
pub struct Tree {
    id: TreeId,
    group: Option<Group>,
}

#[derive(Debug, Clone)]
struct Group {
    delimiter: char,
    /// The offset the text between the delimiters starts at: where a list
    /// whose content it is is recorded in its source.
    content: usize,
    trees: Vec<Tree>,
}

/// A member of a list in one source.
#[derive(Debug)]
pub struct Member {
    /// Its text, from its first token to its last, the separator after it
    /// included.
    range: Range<usize>,
    trees: Vec<Tree>,
    /// The id of its tokens, which says whether two members are alike.
    key: TreeId,
    selectable: bool,
}

impl Member {
    /// The id of its tokens: two members of the sources of one merge have one
    /// key exactly when they are the same tokens.
    pub fn key(&self) -> TreeId {
        self.key
    }

    /// Its tokens, as the dialect read them.
    pub fn trees(&self) -> &[Tree] {
        &self.trees
    }
}

/// A list of one source a selector may stand in.
#[derive(Debug)]
pub struct List {
    /// What list of the language it is; the versions of one list are lists
    /// of one kind.
    pub kind: &'static str,
    /// Whether its members are separated by commas.
    pub separated: bool,
    /// Its text between its delimiters: all of the file for the top list.
    pub content: Range<usize>,
    pub members: Vec<Member>,
}

/// One source, read: its text, and every list in it by the offset its content
/// starts at.
#[derive(Debug)]
pub struct Source<'t> {
    pub text: &'t str,
    /// The offset of the list that holds the rest.
    pub top: usize,
    pub lists: BTreeMap<usize, List>,
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
    groups: Vec<Alike>,
}

/// A member several versions write alike.
struct Alike {
    set: VariantSet,
    /// The member in each version that writes it: (version, member).
    at: Vec<(usize, usize)>,
}

impl Slot {
    fn key(&self, versions: &[Version<'_>], group: usize) -> TreeId {
        let (version, member) = self.groups[group].at[0];
        versions[version].list.members[member].key
    }

    fn has_key(&self, versions: &[Version<'_>], key: TreeId) -> Option<usize> {
        (0..self.groups.len()).find(|&group| self.key(versions, group) == key)
    }
}

/// A merge in one dialect, under one spelling of the selectors.
struct Merge<'m, D: ?Sized> {
    dialect: &'m D,
    spell: &'m Spell<'m>,
}

impl<D: Dialect + ?Sized> Merge<'_, D> {
    /// The merged content of the list `versions` are versions of.
    fn list(&self, versions: &[Version<'_>]) -> Result<String, Unmerged> {
        let list = versions[0].list;
        if versions
            .iter()
            .any(|version| version.list.kind != list.kind)
        {
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
        let mut previous: Option<&Slot> = None;
        for slot in &slots {
            let (version, member) = lead_source(slot, previous);
            previous = Some(slot);
            let lead = lead_of(&versions[version], member);
            let indent = lead
                .rsplit('\n')
                .next()
                .filter(|last| lead.contains('\n') && last.chars().all(char::is_whitespace))
                .unwrap_or("");
            let written = self.slot(versions, slot, &within, indent)?;
            // Two members that each start their own version's list have no text
            // between them; they are put on lines of their own.
            if lead.is_empty() && out.chars().last().is_some_and(|c| !c.is_whitespace()) {
                out.push('\n');
            }
            out.push_str(lead);
            out.push_str(&written);
            if list.separated && !written.trim_end().ends_with(',') {
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

    /// A slot, written: each of its members under its selector, each on a line
    /// of its own at `indent`, or the members merged inside the one list they
    /// differ in.
    fn slot(
        &self,
        versions: &[Version<'_>],
        slot: &Slot,
        within: &VariantSet,
        indent: &str,
    ) -> Result<String, Unmerged> {
        if let [group] = slot.groups.as_slice() {
            let text = member_text(versions, group.at[0]);
            return self.selected(versions, &group.set, within, group.at[0], text);
        }
        let union: VariantSet = slot
            .groups
            .iter()
            .flat_map(|group| group.set.iter().copied())
            .collect();
        let deeper = match self.inside(versions, slot) {
            Ok(merged) => {
                let lender = slot_lead_source(slot);
                return self.selected(versions, &union, within, lender, &merged);
            }
            Err(deeper) => deeper,
        };
        // The groups of a slot are written by disjoint sets of variants, so
        // none of two or more holds in every build of `within`.
        let alternatives = slot
            .groups
            .iter()
            .map(|group| {
                let (version, _) = group.at[0];
                self.refuse_unselectable(versions, group.at[0])?;
                let mut text = member_text(versions, group.at[0]).to_string();
                if versions[version].list.separated && !text.trim_end().ends_with(',') {
                    text.push(',');
                }
                Ok(((self.spell)(&group.set, within), text))
            })
            .collect::<Result<Vec<_>, Unmerged>>()
            // The members could not be selected whole because a list inside
            // them could not be merged: that list's refusal is the cause.
            .map_err(|whole| deeper.unwrap_or(whole))?;
        Ok(self.dialect.alternatives(&alternatives, indent))
    }

    /// `text`, the member `at` or what it is merged into, written for the
    /// builds of `set` within those of `within`.
    fn selected(
        &self,
        versions: &[Version<'_>],
        set: &VariantSet,
        within: &VariantSet,
        at: (usize, usize),
        text: &str,
    ) -> Result<String, Unmerged> {
        if set.is_superset(within) {
            return Ok(text.to_string());
        }
        self.refuse_unselectable(versions, at)?;
        Ok(self.dialect.select(&(self.spell)(set, within), text))
    }

    fn refuse_unselectable(
        &self,
        versions: &[Version<'_>],
        (version, member): (usize, usize),
    ) -> Result<(), Unmerged> {
        let version = &versions[version];
        let member = &version.list.members[member];
        if member.selectable {
            return Ok(());
        }
        Err(Unmerged(format!(
            "a member only some variants write stands where no selector may: {}",
            &version.source.text[member.range.clone()]
        )))
    }

    /// The slot's members merged inside the deepest list outside which they
    /// are alike, written as the member that holds the most variants writes it
    /// with that list's content replaced. The error is `None` when they differ
    /// outside every list, and otherwise the refusal of the deepest list they
    /// differ in.
    fn inside(&self, versions: &[Version<'_>], slot: &Slot) -> Result<String, Option<Unmerged>> {
        let members: Vec<(usize, &Member)> = slot
            .groups
            .iter()
            .map(|group| {
                let (version, member) = group.at[0];
                (version, &versions[version].list.members[member])
            })
            .collect();
        // Each list the members share, outermost first, as the offset its
        // content starts at in each member's source.
        let mut shared: Vec<Vec<usize>> = Vec::new();
        let mut trees: Vec<&[Tree]> = members.iter().map(|(_, m)| m.trees.as_slice()).collect();
        while let Some(position) = only_difference(&trees) {
            let Some(groups) = trees
                .iter()
                .map(|t| t[position].group.as_ref())
                .collect::<Option<Vec<&Group>>>()
            else {
                break;
            };
            if groups.iter().any(|g| g.delimiter != groups[0].delimiter) {
                break;
            }
            shared.push(groups.iter().map(|group| group.content).collect());
            trees = groups.iter().map(|group| group.trees.as_slice()).collect();
        }
        let mut deepest = None;
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
            let content = match self.list(&inner) {
                Ok(content) => content,
                Err(refused) => {
                    deepest.get_or_insert(refused);
                    continue;
                }
            };
            let lender = (0..members.len())
                .max_by_key(|&i| slot.groups[i].set.len())
                .expect("a slot has a group");
            let (version, member) = members[lender];
            let text = versions[version].source.text;
            let list = lists[lender];
            return Ok(format!(
                "{}{content}{}",
                &text[member.range.start..list.content.start],
                &text[list.content.end..member.range.end]
            ));
        }
        Err(deepest)
    }
}

/// The one position at which runs of trees of one length differ, or `None`
/// when they differ in length, at no position, or at more than one.
fn only_difference(runs: &[&[Tree]]) -> Option<usize> {
    let length = runs[0].len();
    if runs.iter().any(|run| run.len() != length) {
        return None;
    }
    let mut differing = (0..length).filter(|&i| runs.iter().any(|run| run[i].id != runs[0][i].id));
    let position = differing.next()?;
    differing.next().is_none().then_some(position)
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

/// The member whose leading text a slot is written after, `previous` the slot
/// written before it: one that follows a member of `previous` in its own
/// version, or starts its version's list where `previous` is none, so that the
/// text between the two is a version's own; of those, the one that stands for
/// the most variants. A slot no member of which so follows is led as the
/// slot's members are ([`slot_lead_source`]).
///
/// Leading text a version writes before a member that is first only in it (a
/// comment over a run of members, say) is then written once, before the slot
/// it leads in that version, and not again before the next.
fn lead_source(slot: &Slot, previous: Option<&Slot>) -> (usize, usize) {
    let follows = |&(version, member): &(usize, usize)| match previous {
        None => member == 0,
        Some(previous) => {
            member > 0
                && previous
                    .groups
                    .iter()
                    .any(|group| group.at.contains(&(version, member - 1)))
        }
    };
    slot.groups
        .iter()
        .filter_map(|group| {
            let at = group.at.iter().find(|&at| follows(at))?;
            Some((group.set.len(), *at))
        })
        .max_by_key(|(variants, _)| *variants)
        .map_or_else(|| slot_lead_source(slot), |(_, at)| at)
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
            groups: vec![Alike {
                set: versions[0].set.clone(),
                at: vec![(0, member)],
            }],
        })
        .collect();
    for version in 1..versions.len() {
        let members = &versions[version].list.members;
        let pairs = longest_common(slots.len(), members.len(), |s, m| {
            slots[s].has_key(versions, members[m].key).is_some()
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
                    .has_key(versions, members[pm].key)
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
                groups: vec![Alike {
                    set: versions[version].set.clone(),
                    at: vec![(version, member)],
                }],
            });
        }
        if ps < gap.len() {
            let mut slot = gap[ps].take().expect("a slot");
            slot.groups.push(Alike {
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
    let ours = &versions[version].list.members[index].trees;
    let theirs = &versions[member.0].list.members[member.1].trees;
    only_difference(&[ours, theirs]).is_some_and(|position| {
        matches!(
            (&ours[position].group, &theirs[position].group),
            (Some(a), Some(b)) if a.delimiter == b.delimiter
        )
    })
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
