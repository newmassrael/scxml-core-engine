// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a backend's language decides about the output for every build of a
//! product line (ADR 0057 §10).
//!
//! The derivation of the variants, their generation, the alignment of their
//! sources and the selectors of the builds are the same for every backend that
//! keeps the variability ([`crate::variant_lift`], [`crate::variant_merge`]).
//! A [`Dialect`] is the rest, each part decided by the grammar of the language
//! the backend writes:
//!
//! - the *reader*: which lists of a source a selector may stand in, and before
//!   which of their members;
//! - the *writer*: how a member is written under a selector, and how
//!   alternatives of one place are;
//! - the *resolver*: how a build reads the output, which the check of the
//!   output against each variant performs;
//! - the *guard*: how the output refuses a build the feature model does not
//!   admit (§6).

use crate::feature_selector::{Refusal, Selector};
use crate::generator::Language;
use crate::variant_merge::{Interner, Source, Unmerged};

pub mod preprocessor;
pub mod rust;

/// How the sources one backend writes carry the variability of a product line.
pub trait Dialect {
    /// A source written for every build, read once and resolved for each.
    type Lifted;
    /// A source one variant generates alone, read once.
    type Reference;

    /// The backend whose sources these are.
    fn backend(&self) -> Language;

    /// The language, as a refusal names it.
    fn name(&self) -> &'static str;

    /// Whether `file`, a file the backend writes, is a source this dialect
    /// reads. Any other file is written by every variant alike or refused.
    fn is_source(&self, file: &str) -> bool;

    /// `selector` as a condition of the language over a build's features.
    fn spell(&self, selector: &Selector) -> String;

    /// `text` read as the merge reads a source: its lists, each member's
    /// tokens and whether a selector may stand at it. A text the reader cannot
    /// read is refused, never guessed at.
    fn read<'t>(&self, text: &'t str, trees: &mut Interner) -> Result<Source<'t>, Unmerged>;

    /// `member` written so that it is in exactly the builds where `predicate`
    /// holds.
    fn select(&self, predicate: &str, member: &str) -> String;

    /// The members of one place, each written for the builds where its
    /// predicate holds; the predicates hold in disjoint sets of builds. Each
    /// member after the first starts a line at `indent`.
    fn alternatives(&self, alternatives: &[(String, String)], indent: &str) -> String;

    /// The file of the output for every build that a build of the statechart
    /// `stem` cannot leave out, which carries the guard.
    fn guarded_file(&self, stem: &str) -> String;

    /// The guard of the feature model `model`, whose direct edges are
    /// `refusals`: the text that makes a build that breaks an edge fail, and
    /// that no build the model admits reads.
    fn guard(&self, refusals: &[Refusal], model: &str) -> String;

    /// Read a source written for every build; `is_feature` names the product
    /// line's features. An error is a source the resolver cannot read, which
    /// is a generator defect.
    fn lift(&self, text: &str, is_feature: &dyn Fn(&str) -> bool) -> Result<Self::Lifted, String>;

    /// Read a source one variant generates alone.
    fn reference(&self, text: &str) -> Result<Self::Reference, String>;

    /// The value of each condition `lifted` writes in a build where
    /// `features` answers `Some(on)` for each of the product line's features:
    /// two builds alike in it resolve `lifted` alike.
    fn valuation(
        &self,
        lifted: &Self::Lifted,
        features: &dyn Fn(&str) -> Option<bool>,
    ) -> Vec<bool>;

    /// Where `lifted`, resolved for the build `features` answers for, is not
    /// `reference`, or `None` when the two are the same tokens.
    fn difference(
        &self,
        lifted: &Self::Lifted,
        features: &dyn Fn(&str) -> Option<bool>,
        reference: &Self::Reference,
    ) -> Option<String>;
}
