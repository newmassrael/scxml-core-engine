// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the unit of a test template decides on every backend
//! (docs/SCE_TEST_PROFILE.md §6, §11.4, §11.9, §12.2): whether a value matches
//! the template, as the tree of the tests its matchers make (ES 201 873-1
//! Annex B.1), and the value a specific template sends (§6.2).
//!
//! The walk over the matchers is written once, here: which test each matcher
//! makes at its position, which matchers read whether an optional position
//! holds a value and which require it present, which bounds of a range hold
//! every value and are left out, which positions a length counts, how a
//! reference binds its template's parameters, and whether a template is
//! specific through the templates it references. A backend spells what the
//! walk decides ([`TemplateSpelling`]): a position, a literal, a parameter, a
//! call of a referenced unit and the parts of a value, and it joins the tree
//! ([`Held`]) into an expression of its language, so only the spelling differs
//! between backends. A backend whose connectives bind as C's do, spelled C's
//! way or Python's ([`Connectives`]), and which writes the tree with what it
//! decides for every value folded away ([`truth`]) spells only the leaves
//! ([`JoinSpelling`]); the join is written here.
//!
//! Matching is on the values the event schema's unit declares, not on an
//! encoding (ADR 0091 C2).

use super::*;
use crate::forge::expr::{wide_literal_spelling, WideLiteral};
use crate::forge::value_type::{
    Bound, ScopedEnum, ScopedRecord, ScopedUnion, TypeOf, TypeScope, ValueType,
};
use crate::generator::Language;

/// An integer literal of the profile (`test_profile::decimal_integer`) as
/// `lang` writes it: the two values no decimal constant of C and C++ holds as
/// written are spelled as every expression of them is there
/// ([`wide_literal_spelling`]); every other language writes the digits. A
/// suite's module parameter default is written by the same spelling.
pub(super) fn integer_literal(lang: Language, text: &str) -> Option<String> {
    let n = crate::forge::test_profile::decimal_integer(text)?;
    let raw = n.to_string();
    let wide = u64::try_from(n)
        .ok()
        .and_then(WideLiteral::of_unsigned)
        .or_else(|| i64::try_from(n).ok().and_then(WideLiteral::of_signed));
    Some(
        wide.and_then(|wide| wide_literal_spelling(lang, wide, &raw))
            .unwrap_or(raw),
    )
}

/// The octets a byte-string literal of the profile writes in hex, two hex
/// digits an octet, each pair in lower case, in order; `None` for a literal
/// that is no hex. A template's literal and a suite's module parameter default
/// of `bytes[N]` are read by it, and each backend writes the pairs as its
/// language writes an octet.
pub(super) fn hex_octets(text: &str) -> Option<Vec<String>> {
    let hex = text.trim();
    if !hex.len().is_multiple_of(2) || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some(
        hex.as_bytes()
            .chunks(2)
            .map(|pair| {
                pair.iter()
                    .map(|b| char::from(b.to_ascii_lowercase()))
                    .collect::<String>()
            })
            .collect(),
    )
}

/// What a value is spelled for: a test that compares it with the value at a
/// position, or a value the template sends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Use {
    Compare,
    Build,
}

/// The function of a referenced template's unit a reference calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TemplateFunction {
    /// Whether a value matches the template.
    Match,
    /// The value a specific template sends.
    Value,
}

/// A comparison of a value, or of a count, with another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Op {
    Eq,
    Ge,
    Le,
}

impl Op {
    /// The operator, as every backend that generates a template writes it.
    pub(super) fn symbol(self) -> &'static str {
        match self {
            Self::Eq => "==",
            Self::Ge => ">=",
            Self::Le => "<=",
        }
    }

    /// The operator, or, where `negated`, the one of the comparison that holds
    /// where it does not.
    pub(super) fn spelled(self, negated: bool) -> &'static str {
        match (self, negated) {
            (op, false) => op.symbol(),
            (Self::Eq, true) => "!=",
            (Self::Ge, true) => "<",
            (Self::Le, true) => ">",
        }
    }
}

/// One position of the value a matcher stands at: its type, read in `scope`,
/// and the backend's spelling of the value there.
#[derive(Clone)]
pub(super) struct Position<'s, A> {
    pub(super) ty: ValueType,
    pub(super) scope: &'s TypeScope,
    pub(super) at: A,
}

impl<A> Position<'_, A> {
    /// Whether the position is an `optional<T>`, which may hold no value.
    pub(super) fn optional(&self) -> bool {
        matches!(self.ty, TypeOf::Optional { .. })
    }
}

/// The record of a union's case and the test it holds, beside the union's
/// test of the case ([`Held::Case`]).
pub(super) type CaseRecord<'s, A> = (Position<'s, A>, Box<Held<'s, A>>);

/// The tests a matcher makes at its position, joined: the tree a backend
/// writes as one boolean expression of its language.
pub(super) enum Held<'s, A> {
    /// Every value matches (`any-value`, `any-value-or-none`).
    True,
    /// The value at `at` compared with `rhs`, the backend's spelling of a
    /// literal or a parameter (`specific`, a bound of `value-range`).
    Compare {
        at: Position<'s, A>,
        op: Op,
        rhs: String,
    },
    /// The count of the value at `at`, compared with `n`: the characters of a
    /// text, the octets of a byte string, the elements of a list (B.1.4.1, and
    /// the count a `list-match` holds).
    Count { at: Position<'s, A>, op: Op, n: u32 },
    /// The call of a referenced template's `match`, spelled whole
    /// (`template-ref`).
    Call(String),
    /// The optional at `at` holds no value (`omit`).
    Absent(Position<'s, A>),
    /// The optional at `at` holds a value, at `inner`, which `then` holds: a
    /// matcher of the value's type on an optional position.
    Present {
        at: Position<'s, A>,
        inner: Position<'s, A>,
        then: Box<Held<'s, A>>,
    },
    /// The optional at `at` holds no value, or holds one, at `inner`, which
    /// `then` holds: a length restriction on an optional position.
    AbsentOr {
        at: Position<'s, A>,
        inner: Position<'s, A>,
        then: Box<Held<'s, A>>,
    },
    /// The union at `at` is in the case `variant`, and the case's record, at
    /// the position of the pair, holds its test (`union-match`); `None` for a
    /// case that holds no record.
    Case {
        at: Position<'s, A>,
        union: &'s ScopedUnion,
        variant: String,
        record: Option<CaseRecord<'s, A>>,
    },
    /// Every part holds.
    All(Vec<Held<'s, A>>),
    /// One part holds at least.
    Any(Vec<Held<'s, A>>),
    /// The part does not hold (`complement`).
    Not(Box<Held<'s, A>>),
}

impl<A> Held<'_, A> {
    /// Whether the tree holds for every value as it is written, every part of a
    /// conjunction holding and a disjunction of one part that does: what a
    /// length restriction that restricts nothing is, and leaves unwrapped.
    pub(super) fn is_true(&self) -> bool {
        match self {
            Self::True => true,
            Self::All(parts) => parts.iter().all(Self::is_true),
            Self::Any(parts) => parts.len() == 1 && parts[0].is_true(),
            Self::Compare { .. }
            | Self::Count { .. }
            | Self::Call(_)
            | Self::Absent(_)
            | Self::Present { .. }
            | Self::AbsentOr { .. }
            | Self::Case { .. }
            | Self::Not(_) => false,
        }
    }
}

/// What `held` comes to for every value, negated where `negated`, where the
/// walk's tree decides it without reading the value: `Some(true)` or
/// `Some(false)`, and `None` where it reads one.
pub(super) fn truth<A>(held: &Held<'_, A>, negated: bool) -> Option<bool> {
    match held {
        Held::True => Some(!negated),
        Held::Not(part) => truth(part, !negated),
        Held::All(parts) => connective_truth(parts, negated, !negated),
        Held::Any(parts) => connective_truth(parts, negated, negated),
        Held::Present { then, .. } => optional_truth(then, negated, negated),
        Held::AbsentOr { then, .. } => optional_truth(then, !negated, negated),
        Held::Case {
            record: Some((_, then)),
            ..
        } if truth(then, false) == Some(false) => Some(negated),
        Held::Compare { .. }
        | Held::Count { .. }
        | Held::Call(_)
        | Held::Absent(_)
        | Held::Case { .. } => None,
    }
}

/// [`truth`] of a join of `parts`, each negated where `negated`: a conjunction
/// where `conjunction`, else a disjunction.
fn connective_truth<A>(parts: &[Held<'_, A>], negated: bool, conjunction: bool) -> Option<bool> {
    // The value that decides the join whichever the others are, and the one
    // that leaves it to them.
    let decides = !conjunction;
    let mut open = false;
    for part in parts {
        match truth(part, negated) {
            Some(held) if held == decides => return Some(decides),
            Some(_) => {}
            None => open = true,
        }
    }
    (!open).then_some(!decides)
}

/// [`truth`] of an optional's test that holds `absent_holds` where the value is
/// absent and its test `then`, negated where `negated`, where it is present.
fn optional_truth<A>(then: &Held<'_, A>, absent_holds: bool, negated: bool) -> Option<bool> {
    (truth(then, negated) == Some(absent_holds)).then_some(absent_holds)
}

/// Where a joined expression stands, which decides whether it is put in
/// parentheses: whole (the body of a function, an arm, a guard), or an
/// operand of `&&` or of `||`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Stands {
    Whole,
    Conjunct,
    Disjunct,
}

/// What the test of a union's case reads of the case's record
/// ([`Held::Case`]), once the walk's tree has decided what it can.
pub(super) enum CaseTest<'h, 's, A> {
    /// The case holds no record.
    Empty,
    /// The case holds a record, which its matcher holds whatever it is.
    AnyRecord,
    /// The case's record, bound at `inner`, holds `then`.
    Holds { inner: &'h A, then: &'h Held<'s, A> },
}

/// The words a backend joins the tests of a matcher by, and the constants it
/// writes a test the walk decides for every value as: C's (`&&`, `||`, `!`,
/// `true`, `false`), which C++, Rust, Kotlin and Go share, Python's (`and`,
/// `or`, `not`, `True`, `False`), or TTCN-3's (`and`, `or`, `not`, `true`,
/// `false`; ES 201 873-1 §7.1), in which the export writes a suite's
/// selection. In each a conjunction binds tighter than a disjunction, and a
/// negation, which the join puts only before a call, a name or a postfix
/// expression, binds that operand whole, so one join writes them all
/// ([`JoinSpelling::join`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Connectives {
    and: &'static str,
    or: &'static str,
    not: &'static str,
    held: &'static str,
    not_held: &'static str,
}

impl Connectives {
    /// C's, and C++'s, Rust's, Kotlin's and Go's.
    pub(super) const C: Self = Self {
        and: " && ",
        or: " || ",
        not: "!",
        held: "true",
        not_held: "false",
    };

    /// Python's.
    pub(super) const PYTHON: Self = Self {
        and: " and ",
        or: " or ",
        not: "not ",
        held: "True",
        not_held: "False",
    };

    /// TTCN-3's (ES 201 873-1 §7.1), which the export of a suite writes its
    /// selections in (docs/SCE_TEST_PROFILE.md §9.5).
    pub(super) const TTCN3: Self = Self {
        and: " and ",
        or: " or ",
        not: "not ",
        held: "true",
        not_held: "false",
    };

    /// The constant of a test that holds for every value where `held`, and for
    /// none otherwise.
    pub(super) fn truth(self, held: bool) -> String {
        if held { self.held } else { self.not_held }.to_string()
    }

    /// `test`, or the test that holds where it does not, where `negated`: the
    /// negation before it, which binds it whole only where it is a call, a name
    /// or a postfix expression.
    pub(super) fn negate(self, test: String, negated: bool) -> String {
        if negated {
            format!("{}{test}", self.not)
        } else {
            test
        }
    }

    /// The word between two operands of a conjunction where `conjunction`, else
    /// of a disjunction, with the spaces around it.
    pub(super) fn joiner(self, conjunction: bool) -> &'static str {
        if conjunction {
            self.and
        } else {
            self.or
        }
    }
}

/// `test`, or the test that holds where it does not, where `negated`, in C's
/// spelling ([`Connectives::C`]): `!` before it.
pub(super) fn negate(test: String, negated: bool) -> String {
    Connectives::C.negate(test, negated)
}

/// How a backend spells what the walk decides. A position's spelling is the
/// backend's own ([`Self::At`]); the walk keeps the type and the scope beside
/// it ([`Position`]).
pub(super) trait TemplateSpelling {
    /// The backend's spelling of the value at one position.
    type At: Clone;

    /// The language the unit is written in.
    fn language(&self) -> Language;

    /// Whether the backend's test profile matches a union
    /// (`codegen_matrix::TEST_PROFILE_UNION_BACKENDS`): a backend that does not
    /// refuses a template whose type reaches one by the union's name, as a
    /// backend that lowers no union refuses a schema unit's union field.
    fn lowers_unions(&self) -> bool {
        crate::forge::codegen_matrix::TEST_PROFILE_UNION_BACKENDS.contains(&self.language())
    }

    /// The template's own value, which `match` is given.
    fn root(&self) -> Self::At;
    /// The value the optional at `at` holds, where it holds one.
    fn present(&self, at: &Self::At) -> Self::At;
    /// The element `index` of the list at `at`.
    fn element(&self, at: &Self::At, index: usize) -> Self::At;
    /// The field `id`, of type `ty`, of the record `record` at `at`.
    fn field(&self, at: &Self::At, record: &ScopedRecord, id: &str, ty: &ValueType) -> Self::At;
    /// The record of the case `variant` of the union `union` at `at`, where a
    /// test proved the value is in that case. Asked only of a backend that
    /// lowers a union ([`Self::lowers_unions`]); one that does not keeps this
    /// refusal.
    fn case(
        &self,
        _at: &Self::At,
        union: &ScopedUnion,
        _variant: &str,
    ) -> Result<Self::At, String> {
        Err(crate::forge::value_type::union_unlowered(&union.name))
    }

    /// The formal parameter `name`, of type `ty`, standing at `at` for `purpose`.
    fn parameter(
        &self,
        name: &str,
        ty: &ValueType,
        at: &Position<'_, Self::At>,
        purpose: Use,
    ) -> String;
    /// The literal `text` of the type of `at` (docs/SCE_TEST_PROFILE.md
    /// §11.4), for `purpose`; the `Err` is the sentence of its refusal.
    fn literal(
        &self,
        text: &str,
        at: &Position<'_, Self::At>,
        purpose: Use,
    ) -> Result<String, String>;
    /// The call of `function` of the unit of the referenced template `import`,
    /// given the value at `first` (for `match`) and `args`, each one of the
    /// template's own formal parameters, in order (-1 §15.1).
    fn call(
        &self,
        import: &ImportContext,
        function: TemplateFunction,
        first: Option<&Self::At>,
        args: &[String],
    ) -> String;

    /// The absent value of the optional position `at`: a backend whose absent
    /// value names its type reads the type there.
    fn absent(&self, at: &Position<'_, Self::At>) -> String;
    /// `value`, a value of the present type of the optional position `at`, as
    /// that position holds it.
    fn present_value(&self, at: &Position<'_, Self::At>, value: String) -> String;
    /// The record `record` at `at`, each field (`(id, value)`) in the order its
    /// schema declares it.
    fn record_value(
        &self,
        at: &Position<'_, Self::At>,
        record: &ScopedRecord,
        fields: Vec<(String, String)>,
    ) -> String;
    /// The list at `at`, its elements in order.
    fn list_value(&self, at: &Position<'_, Self::At>, elements: Vec<String>) -> String;
    /// The value of the case `variant` of the union `union` at `at`, holding
    /// `record` where the case holds one. Asked only of a backend that lowers
    /// a union, as [`Self::case`] is.
    fn case_value(
        &self,
        _at: &Position<'_, Self::At>,
        union: &ScopedUnion,
        _variant: &str,
        _record: Option<String>,
    ) -> Result<String, String> {
        Err(crate::forge::value_type::union_unlowered(&union.name))
    }
}

/// How a backend whose connectives bind as C's do, a conjunction tighter than
/// a disjunction ([`Connectives`]), writes the leaves of the tree;
/// [`Self::join`] writes the tree, once for every such backend. A negation is
/// carried down to the leaves, so a complement is written as the comparisons
/// that do not hold rather than as a negation before a join; a part the tree
/// decides for every value ([`truth`]) is left out of its join, or is the join
/// where it decides it; and a join is put in parentheses only where it is an
/// operand of the other connective.
pub(super) trait JoinSpelling: TemplateSpelling {
    /// The words the backend joins the tests by and the constants it writes a
    /// decided test as: C's, but for a backend whose language spells them
    /// otherwise.
    fn connectives(&self) -> Connectives {
        Connectives::C
    }

    /// The comparison of the value at `at` with `rhs`, or the one that holds
    /// where it does not, where `negated`.
    fn compare(&self, at: &Position<'_, Self::At>, op: Op, negated: bool, rhs: &str) -> String;
    /// The count of the value at `at` compared with `n`, or the comparison
    /// that holds where it does not: the characters of a text, the octets of a
    /// byte string, the elements of a list.
    fn count(&self, at: &Position<'_, Self::At>, op: Op, negated: bool, n: u32) -> String;
    /// Whether the optional at `at` holds no value, or, where `negated`, holds
    /// one.
    fn absence(&self, at: &Self::At, negated: bool) -> String;
    /// The test of the optional at `at` that holds `absent_holds` where it
    /// holds no value and `then`, negated where `negated`, on the value it
    /// holds, bound at `inner`, standing at `stands`. Asked only where both
    /// are read.
    fn bound_optional(
        &self,
        at: &Self::At,
        inner: &Self::At,
        then: &Held<'_, Self::At>,
        absent_holds: bool,
        negated: bool,
        stands: Stands,
    ) -> String;
    /// Whether the union `union` at `at` is in the case `variant`, and what
    /// its record holds there (`record`), or, where `negated`, the test that
    /// holds where it does not: a language whose `!` binds tighter than its
    /// test of a case writes the negation its own way.
    fn case_test(
        &self,
        at: &Self::At,
        union: &ScopedUnion,
        variant: &str,
        record: CaseTest<'_, '_, Self::At>,
        negated: bool,
    ) -> String;

    /// `held` as one expression standing at `stands`, negated where
    /// `negated`.
    fn join(&self, held: &Held<'_, Self::At>, negated: bool, stands: Stands) -> String {
        let connectives = self.connectives();
        match held {
            Held::True => connectives.truth(!negated),
            Held::Not(part) => self.join(part, !negated, stands),
            Held::All(parts) => self.connective(parts, negated, !negated, stands),
            Held::Any(parts) => self.connective(parts, negated, negated, stands),
            Held::Compare { at, op, rhs } => self.compare(at, *op, negated, rhs),
            Held::Count { at, op, n } => self.count(at, *op, negated, *n),
            Held::Call(call) => connectives.negate(call.clone(), negated),
            Held::Absent(at) => self.absence(&at.at, negated),
            Held::Present { at, inner, then } => {
                self.optional(&at.at, &inner.at, then, negated, negated, stands)
            }
            Held::AbsentOr { at, inner, then } => {
                self.optional(&at.at, &inner.at, then, !negated, negated, stands)
            }
            Held::Case {
                at,
                union,
                variant,
                record,
            } => {
                let record = match record {
                    None => CaseTest::Empty,
                    Some((inner, then)) => match truth(then, false) {
                        Some(true) => CaseTest::AnyRecord,
                        Some(false) => return connectives.truth(negated),
                        None => CaseTest::Holds {
                            inner: &inner.at,
                            then,
                        },
                    },
                };
                self.case_test(&at.at, union, variant, record, negated)
            }
        }
    }

    /// The join of `parts`, each negated where `negated`, by a conjunction
    /// where `conjunction` and by a disjunction otherwise: a part that holds
    /// for every value is left out, and one that decides the join is the join.
    fn connective(
        &self,
        parts: &[Held<'_, Self::At>],
        negated: bool,
        conjunction: bool,
        stands: Stands,
    ) -> String {
        let connectives = self.connectives();
        let decides = !conjunction;
        let mut open = Vec::new();
        for part in parts {
            match truth(part, negated) {
                Some(held) if held == decides => return connectives.truth(decides),
                Some(_) => {}
                None => open.push(part),
            }
        }
        match open.as_slice() {
            [] => connectives.truth(!decides),
            [part] => self.join(part, negated, stands),
            _ => {
                let operand = if conjunction {
                    Stands::Conjunct
                } else {
                    Stands::Disjunct
                };
                let joined = open
                    .iter()
                    .map(|part| self.join(part, negated, operand))
                    .collect::<Vec<_>>()
                    .join(connectives.joiner(conjunction));
                let nested = match stands {
                    Stands::Whole => false,
                    Stands::Conjunct => !conjunction,
                    Stands::Disjunct => conjunction,
                };
                if nested {
                    format!("({joined})")
                } else {
                    joined
                }
            }
        }
    }

    /// The test of the optional at `at`: `absent_holds` where it holds no
    /// value, and `then`, negated where `negated`, on the value it holds,
    /// bound at `inner`. Where the tree decides `then` for every value, the
    /// optional's absence or presence alone, or the constant it comes to.
    fn optional(
        &self,
        at: &Self::At,
        inner: &Self::At,
        then: &Held<'_, Self::At>,
        absent_holds: bool,
        negated: bool,
        stands: Stands,
    ) -> String {
        match (truth(then, negated), absent_holds) {
            (Some(true), true) => self.connectives().truth(true),
            (Some(false), false) => self.connectives().truth(false),
            (Some(true), false) => self.absence(at, true),
            (Some(false), true) => self.absence(at, false),
            (None, _) => self.bound_optional(at, inner, then, absent_holds, negated, stands),
        }
    }
}

/// One template, walked: what each matcher of it holds and the value of a
/// specific one, spelled by `spelling`.
pub(super) struct TemplateWalk<'a, B> {
    pub(super) template: &'a TestTemplateModel,
    pub(super) imports: &'a [ImportContext],
    pub(super) spelling: &'a B,
}

impl<B: TemplateSpelling> TemplateWalk<'_, B> {
    /// The refusal of the template, by name, for `why`.
    pub(super) fn refused(&self, why: impl std::fmt::Display) -> ForgeError {
        GenerateError::unsupported_at(
            format!("test-template '{}': {why}", self.template.name),
            self.template.source_location.clone(),
        )
        .into()
    }

    /// Refuse the template where its type or a parameter's reaches a union at
    /// any depth, read in `scope`, on a backend that lowers none, as a schema
    /// unit's union field is refused there
    /// ([`crate::forge::value_type::union_unlowered`]).
    pub(super) fn refuse_unlowered_unions(&self, scope: &TypeScope) -> Result<(), ForgeError> {
        if self.spelling.lowers_unions() {
            return Ok(());
        }
        let m = self.template;
        for ty in
            std::iter::once(&m.template_type).chain(m.parameters.iter().map(|p| &p.parameter_type))
        {
            if let Some(alias) = crate::forge::test_profile::union_reached(ty, scope) {
                return Err(self.refused(crate::forge::value_type::union_unlowered(&alias)));
            }
        }
        Ok(())
    }

    /// Refuse the template where its type or a parameter's holds, at its
    /// leaf, a value of an enum no import of the document reads, in `scope`: a
    /// backend that names an enum through the document that declares it has
    /// no name for one no import reads.
    pub(super) fn refuse_unread_enums(&self, scope: &TypeScope) -> Result<(), ForgeError> {
        let m = self.template;
        for ty in
            std::iter::once(&m.template_type).chain(m.parameters.iter().map(|p| &p.parameter_type))
        {
            if let Some(reference) = ty.enum_leaf() {
                if !scope.enums.contains_key(&reference.alias) {
                    return Err(self.refused(format!(
                        "`enum:{}` names no enum the document imports",
                        reference.alias
                    )));
                }
            }
        }
        Ok(())
    }

    /// The position of the template's own value, whose type is the
    /// template's, read in `scope`.
    pub(super) fn root<'s>(&self, scope: &'s TypeScope) -> Position<'s, B::At> {
        Position {
            ty: self.template.template_type.clone(),
            scope,
            at: self.spelling.root(),
        }
    }

    /// The value an optional position holds where it holds one, and any other
    /// position as it is.
    fn present<'s>(&self, at: &Position<'s, B::At>) -> Position<'s, B::At> {
        match &at.ty {
            TypeOf::Optional { elem } => Position {
                ty: (**elem).clone(),
                scope: at.scope,
                at: self.spelling.present(&at.at),
            },
            _ => at.clone(),
        }
    }

    /// The template the import `alias` names: its import and what the import
    /// read of it.
    fn reference(&self, alias: &str) -> Result<(&ImportContext, &TestTemplateImport), ForgeError> {
        self.imports
            .iter()
            .find(|i| i.alias == alias && i.kind == "test-template")
            .and_then(|i| i.test_template.as_ref().map(|read| (i, read)))
            .ok_or_else(|| {
                self.refused(format!(
                    "<sce:template-ref template=\"{alias}\"> names no test template the document \
                     imports"
                ))
            })
    }

    /// Whether the template is specific (docs/SCE_TEST_PROFILE.md §6.2): its
    /// own matchers are values alone, and every template it references is
    /// specific in turn, as its import read it.
    pub(super) fn specific(&self) -> bool {
        if !self.template.is_specific() {
            return false;
        }
        let mut pending = vec![&self.template.matcher];
        while let Some(matcher) = pending.pop() {
            if let TestMatcherForm::TemplateRef { template, .. } = &matcher.form {
                match self.reference(template) {
                    Ok((_, referenced)) if referenced.specific => {}
                    _ => return false,
                }
            }
            pending.extend(matcher.children());
        }
        true
    }

    /// The formal parameter `name` of the template.
    fn parameter(&self, name: &str) -> Result<&TestTemplateParameter, ForgeError> {
        self.template.parameter(name).ok_or_else(|| {
            self.refused(format!(
                "'{name}' is not a formal parameter of the template"
            ))
        })
    }

    /// The record a record position names.
    fn record<'s>(&self, at: &Position<'s, B::At>) -> Result<&'s ScopedRecord, ForgeError> {
        let TypeOf::Record { alias } = &at.ty else {
            return Err(self.refused(format!(
                "a <sce:record-match> stands where a value of {} is matched",
                at.ty
            )));
        };
        at.scope
            .records
            .get(alias)
            .ok_or_else(|| self.refused(format!("`record:{alias}` names no event schema read")))
    }

    /// The position of the field `id`, of type `ty`, of the record at `at`.
    fn field<'s>(
        &self,
        at: &Position<'s, B::At>,
        record: &'s ScopedRecord,
        id: &str,
        ty: &ValueType,
    ) -> Position<'s, B::At> {
        Position {
            ty: ty.clone(),
            scope: &record.scope,
            at: self.spelling.field(&at.at, record, id, ty),
        }
    }

    /// The matcher a record's matcher holds for the field `id`.
    fn field_matcher<'m>(
        &self,
        fields: &'m [TestMatcher],
        id: &str,
    ) -> Result<&'m TestMatcher, ForgeError> {
        fields
            .iter()
            .find(|f| f.field.as_deref() == Some(id))
            .ok_or_else(|| self.refused(format!("no matcher for the field `{id}`")))
    }

    /// The type of the elements of the list at `at`.
    fn list_element(&self, at: &Position<'_, B::At>) -> Result<ValueType, ForgeError> {
        match &at.ty {
            TypeOf::List { elem, .. } => Ok((**elem).clone()),
            other => Err(self.refused(format!(
                "a <sce:list-match> stands where a value of {other} is matched"
            ))),
        }
    }

    /// The position of the element `index`, of type `elem`, of the list at
    /// `at`.
    fn element<'s>(
        &self,
        at: &Position<'s, B::At>,
        index: usize,
        elem: &ValueType,
    ) -> Position<'s, B::At> {
        Position {
            ty: elem.clone(),
            scope: at.scope,
            at: self.spelling.element(&at.at, index),
        }
    }

    /// The union a union position names, and the record alias of its case
    /// `variant`, `None` for a case that holds no record, on a backend that
    /// lowers a union.
    fn union_case<'s>(
        &self,
        at: &Position<'s, B::At>,
        variant: &str,
    ) -> Result<(&'s ScopedUnion, Option<&'s str>), ForgeError> {
        let refused_here = || {
            self.refused(format!(
                "a <sce:union-match> stands where a value of {} is matched",
                at.ty
            ))
        };
        if !self.spelling.lowers_unions() {
            return Err(refused_here());
        }
        let TypeOf::Union { alias } = &at.ty else {
            return Err(refused_here());
        };
        let union = at
            .scope
            .unions
            .get(alias)
            .ok_or_else(|| self.refused(format!("`union:{alias}` names no union read")))?;
        let case = union
            .cases
            .iter()
            .find(|c| c.variant.name == variant)
            .ok_or_else(|| {
                self.refused(format!(
                    "<sce:union-match variant=\"{variant}\"> names no case of `union:{alias}`"
                ))
            })?;
        Ok((union, case.record.as_deref()))
    }

    /// The position of the record of the case `variant` of the union `union`
    /// at `at`, of the record alias `record` in the union's scope.
    fn case_record<'s>(
        &self,
        at: &Position<'s, B::At>,
        union: &'s ScopedUnion,
        variant: &str,
        record: &str,
    ) -> Result<Position<'s, B::At>, ForgeError> {
        Ok(Position {
            ty: TypeOf::Record {
                alias: record.to_string(),
            },
            scope: &union.scope,
            at: self
                .spelling
                .case(&at.at, union, variant)
                .map_err(|why| self.refused(why))?,
        })
    }

    /// What the value at `at` must hold to match `m`: its form, then its
    /// length restriction, then `ifpresent` (B.1.4.1, B.1.4.2).
    pub(super) fn matches<'s>(
        &self,
        m: &TestMatcher,
        at: &Position<'s, B::At>,
    ) -> Result<Held<'s, B::At>, ForgeError> {
        let mut held = self.form(m, at)?;
        if m.length_min.is_some() || m.length_max.is_some() {
            held = Held::All(vec![held, self.length(m, at)?]);
        }
        if m.ifpresent && at.optional() {
            held = Held::Any(vec![Held::Absent(at.clone()), held]);
        }
        Ok(held)
    }

    /// Whether `m`, standing at an optional position, reads whether the
    /// position holds a value, rather than the value it holds: `omit` and
    /// `any-value-or-none` (B.1.2.4, B.1.2.8), a list or a complement of
    /// matchers at the same position, each of which may be one, a parameter
    /// and a reference whose type is the optional's own.
    fn reads_absence(&self, m: &TestMatcher) -> bool {
        let optional = |ty: &ValueType| matches!(ty, TypeOf::Optional { .. });
        match &m.form {
            TestMatcherForm::Omit
            | TestMatcherForm::AnyValueOrNone
            | TestMatcherForm::ValueList { .. }
            | TestMatcherForm::Complement { .. } => true,
            TestMatcherForm::Specific {
                value: TestValue::Param(name),
            } => self
                .template
                .parameter(name)
                .is_some_and(|p| optional(&p.parameter_type)),
            TestMatcherForm::TemplateRef { template, .. } => self
                .reference(template)
                .is_ok_and(|(_, referenced)| optional(&referenced.template_type)),
            TestMatcherForm::Specific { .. }
            | TestMatcherForm::RecordMatch { .. }
            | TestMatcherForm::ListMatch { .. }
            | TestMatcherForm::UnionMatch { .. }
            | TestMatcherForm::AnyValue
            | TestMatcherForm::ValueRange { .. } => false,
        }
    }

    /// What the value at `at` must hold to match the form of `m`. At an
    /// optional position a matcher of the value's type requires the value
    /// present.
    fn form<'s>(
        &self,
        m: &TestMatcher,
        at: &Position<'s, B::At>,
    ) -> Result<Held<'s, B::At>, ForgeError> {
        if at.optional() && !self.reads_absence(m) {
            let inner = self.present(at);
            let then = self.form(m, &inner)?;
            return Ok(Held::Present {
                at: at.clone(),
                inner,
                then: Box::new(then),
            });
        }
        Ok(match &m.form {
            TestMatcherForm::AnyValue | TestMatcherForm::AnyValueOrNone => Held::True,
            TestMatcherForm::Omit => Held::Absent(at.clone()),
            TestMatcherForm::Specific { value } => Held::Compare {
                at: at.clone(),
                op: Op::Eq,
                rhs: self.operand(value, at, Use::Compare)?,
            },
            TestMatcherForm::RecordMatch { fields } => {
                let record = self.record(at)?;
                let mut parts = Vec::new();
                for (id, ty) in &record.fields {
                    let matcher = self.field_matcher(fields, id)?;
                    let field = self.field(at, record, id, ty);
                    parts.push(self.matches(matcher, &field)?);
                }
                Held::All(parts)
            }
            TestMatcherForm::ListMatch { elements } => {
                let elem = self.list_element(at)?;
                let count = u32::try_from(elements.len())
                    .map_err(|_| self.refused("a <sce:list-match> of more elements than 2^32"))?;
                let mut parts = vec![Held::Count {
                    at: at.clone(),
                    op: Op::Eq,
                    n: count,
                }];
                for (index, element) in elements.iter().enumerate() {
                    parts.push(self.matches(element, &self.element(at, index, &elem))?);
                }
                Held::All(parts)
            }
            TestMatcherForm::ValueList { items } => Held::Any(
                items
                    .iter()
                    .map(|item| self.matches(item, at))
                    .collect::<Result<_, _>>()?,
            ),
            TestMatcherForm::Complement { items } => Held::Not(Box::new(Held::Any(
                items
                    .iter()
                    .map(|item| self.matches(item, at))
                    .collect::<Result<_, _>>()?,
            ))),
            TestMatcherForm::ValueRange { lower, upper, .. } => {
                let range = match &at.ty {
                    TypeOf::Scalar(scalar) => scalar.int_value_range(),
                    _ => None,
                };
                let (least, most) =
                    range.ok_or_else(|| self.refused(format!("a value range over {}", at.ty)))?;
                let mut parts = Vec::new();
                for (bound, op, limit) in [(lower, Op::Ge, least), (upper, Op::Le, most)] {
                    let Some(bound) = bound else {
                        continue;
                    };
                    // A literal bound at the type's own limit holds every
                    // value, and its comparison is one a compiler warns is
                    // always true: it is left out, as a missing bound is.
                    if let TestValue::Value(text) = bound {
                        if crate::forge::test_profile::decimal_integer(text) == Some(limit) {
                            continue;
                        }
                    }
                    parts.push(Held::Compare {
                        at: at.clone(),
                        op,
                        rhs: self.operand(bound, at, Use::Compare)?,
                    });
                }
                Held::All(parts)
            }
            TestMatcherForm::TemplateRef { template, args } => {
                let (import, _) = self.reference(template)?;
                Held::Call(
                    self.spelling
                        .call(import, TemplateFunction::Match, Some(&at.at), args),
                )
            }
            TestMatcherForm::UnionMatch { variant, case } => {
                let (union, record) = self.union_case(at, variant)?;
                let record = match (record, case) {
                    (Some(record), Some(matcher)) => {
                        let inner = self.case_record(at, union, variant, record)?;
                        let then = self.matches(matcher, &inner)?;
                        Some((inner, Box::new(then)))
                    }
                    (None, None) => None,
                    (Some(_), None) | (None, Some(_)) => {
                        return Err(self.refused(format!(
                            "<sce:union-match variant=\"{variant}\"> holds a matcher exactly \
                             where its case holds a record"
                        )))
                    }
                };
                Held::Case {
                    at: at.clone(),
                    union,
                    variant: variant.clone(),
                    record,
                }
            }
        })
    }

    /// The length restriction of `m` on the value at `at` (B.1.4.1):
    /// characters of a text, octets of a byte string, elements of a list. An
    /// absent optional value is held to none: whether it may be absent is the
    /// matcher's own question.
    fn length<'s>(
        &self,
        m: &TestMatcher,
        at: &Position<'s, B::At>,
    ) -> Result<Held<'s, B::At>, ForgeError> {
        let present = self.present(at);
        match &present.ty {
            TypeOf::Text { .. } | TypeOf::Bytes { .. } | TypeOf::List { .. } => {}
            other => return Err(self.refused(format!("a length restriction on {other}"))),
        }
        let mut parts = Vec::new();
        if let Some(least) = m.length_min.filter(|least| *least > 0) {
            parts.push(Held::Count {
                at: present.clone(),
                op: Op::Ge,
                n: least,
            });
        }
        if let Some(most) = m.length_max {
            parts.push(Held::Count {
                at: present.clone(),
                op: Op::Le,
                n: most,
            });
        }
        let held = Held::All(parts);
        Ok(if at.optional() && !held.is_true() {
            Held::AbsentOr {
                at: at.clone(),
                inner: present,
                then: Box::new(held),
            }
        } else {
            held
        })
    }

    /// A value a matcher names, at `at`, for `purpose`: a formal parameter, or
    /// a literal of the type of the value the position holds.
    fn operand(
        &self,
        value: &TestValue,
        at: &Position<'_, B::At>,
        purpose: Use,
    ) -> Result<String, ForgeError> {
        match value {
            TestValue::Param(name) => {
                let parameter = self.parameter(name)?;
                Ok(self
                    .spelling
                    .parameter(name, &parameter.parameter_type, at, purpose))
            }
            TestValue::Value(text) => self
                .spelling
                .literal(text, &self.present(at), purpose)
                .map_err(|why| self.refused(why)),
        }
    }

    /// The value the specific matcher `m` sends at `at`
    /// (docs/SCE_TEST_PROFILE.md §6.2): a record's fields in the order its
    /// schema declares them, a list's elements in order, a union's case with
    /// its record, an absent optional value, and a referenced template's own
    /// `value`. A value of the present type of an optional position is held as
    /// the position holds it.
    pub(super) fn value(
        &self,
        m: &TestMatcher,
        at: &Position<'_, B::At>,
    ) -> Result<String, ForgeError> {
        let present = self.present(at);
        let (built, of_present) = match &m.form {
            TestMatcherForm::Omit => return Ok(self.spelling.absent(at)),
            TestMatcherForm::Specific {
                value: TestValue::Param(name),
            } => {
                let parameter = self.parameter(name)?;
                let whole =
                    at.optional() && matches!(parameter.parameter_type, TypeOf::Optional { .. });
                let stands = if whole { at } else { &present };
                let spelled =
                    self.spelling
                        .parameter(name, &parameter.parameter_type, stands, Use::Build);
                (spelled, !whole)
            }
            TestMatcherForm::Specific {
                value: TestValue::Value(text),
            } => (
                self.spelling
                    .literal(text, &present, Use::Build)
                    .map_err(|why| self.refused(why))?,
                true,
            ),
            TestMatcherForm::RecordMatch { fields } => {
                let record = self.record(&present)?;
                let mut values = Vec::new();
                for (id, ty) in &record.fields {
                    let matcher = self.field_matcher(fields, id)?;
                    let field = self.field(&present, record, id, ty);
                    values.push((id.clone(), self.value(matcher, &field)?));
                }
                (self.spelling.record_value(&present, record, values), true)
            }
            TestMatcherForm::ListMatch { elements } => {
                let elem = self.list_element(&present)?;
                let values: Vec<String> = elements
                    .iter()
                    .enumerate()
                    .map(|(index, element)| {
                        self.value(element, &self.element(&present, index, &elem))
                    })
                    .collect::<Result<_, _>>()?;
                (self.spelling.list_value(&present, values), true)
            }
            TestMatcherForm::UnionMatch { variant, case } => {
                let (union, record) = self.union_case(&present, variant)?;
                let record = match (record, case) {
                    (Some(record), Some(matcher)) => {
                        let inner = self.case_record(&present, union, variant, record)?;
                        Some(self.value(matcher, &inner)?)
                    }
                    (None, None) => None,
                    (Some(_), None) | (None, Some(_)) => {
                        return Err(self.refused(format!(
                            "<sce:union-match variant=\"{variant}\"> holds a matcher exactly \
                             where its case holds a record"
                        )))
                    }
                };
                (
                    self.spelling
                        .case_value(&present, union, variant, record)
                        .map_err(|why| self.refused(why))?,
                    true,
                )
            }
            TestMatcherForm::TemplateRef { template, args } => {
                let (import, referenced) = self.reference(template)?;
                (
                    self.spelling
                        .call(import, TemplateFunction::Value, None, args),
                    !matches!(referenced.template_type, TypeOf::Optional { .. }),
                )
            }
            TestMatcherForm::ValueList { .. }
            | TestMatcherForm::Complement { .. }
            | TestMatcherForm::AnyValue
            | TestMatcherForm::AnyValueOrNone
            | TestMatcherForm::ValueRange { .. } => {
                return Err(self.refused(format!(
                    "<sce:{}> has no value to send: a specific template holds values alone",
                    m.form.element()
                )))
            }
        };
        Ok(if at.optional() && of_present {
            self.spelling.present_value(at, built)
        } else {
            built
        })
    }

    /// What the unit of a template over a record writes of a message of it in
    /// the test log (docs/SCE_TEST_PROFILE.md §12.5): the name of its record
    /// type ([`log_type_name`]) and the expression of its value's canonical
    /// form, spelled by `canonical` ([`canonical_value`]). None for a template
    /// over another type: a message is a record of an event schema (§11.3), so
    /// only such a template's unit writes one
    /// ([`TestTemplateModel::is_over_record`]), which is also what puts the
    /// document's unit on the SCE runtime its header states
    /// (`ForgeDocument::runtime_dep`).
    pub(super) fn message_form<C: CanonicalSpelling>(
        &self,
        canonical: &C,
        scope: &TypeScope,
    ) -> Result<Option<(String, String)>, ForgeError> {
        let TypeOf::Record { alias } = &self.template.template_type else {
            return Ok(None);
        };
        let record = scope.records.get(alias).ok_or_else(|| {
            self.refused(format!(
                "`record:{alias}` names no record the document imports"
            ))
        })?;
        let written = canonical_value(
            canonical,
            &self.template.template_type,
            &canonical.root(),
            scope,
        )
        .map_err(|why| self.refused(why))?;
        Ok(Some((log_type_name(record), written)))
    }
}

/// The name the types module of the suite's TTCN-3 export gives the record
/// `record` (rule E1, docs/SCE_TEST_PROFILE.md §9.5): the unit name of its
/// document's file ([`crate::ttcn3_export::names::unit_name`]), which is what a
/// message of it carries as its `type` in the test log (§12.5). A record read
/// with no document falls back to the name the document gives itself.
pub(super) fn log_type_name(record: &ScopedRecord) -> String {
    let stem = std::path::Path::new(&record.identity)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or(&record.name);
    crate::ttcn3_export::names::unit_name(stem)
}

/// A case of a union in the canonical form: the variant it is for, by its
/// declared name, and, where the case holds a record, the name the record is
/// bound to and its written form.
pub(crate) type CanonicalCase = (String, Option<(String, String)>);

/// How a backend spells the canonical form of a value in the test log
/// (docs/SCE_TEST_PROFILE.md §12.5) with the writers of its runtime: one
/// expression of the backend's language per value, of the form the SCE type
/// takes rather than the language's type, since a byte string and a list of
/// `uint8` are one type in several languages and take two forms. The walk over
/// the type, which form each type takes and which scope each record and union
/// is read in, is [`canonical_value`], written once for every backend.
pub(super) trait CanonicalSpelling {
    /// The backend's spelling of the value the writer reads at one position.
    type At;

    /// The unit's `canonical`'s parameter, the template's own value.
    fn root(&self) -> Self::At;
    /// The field `id` of the record at `at`.
    fn field(&self, at: &Self::At, id: &str) -> Self::At;
    /// The name a function or a pattern binds a value inside the one at `at`
    /// to, numbered by how many bindings `at` stands inside after `prefix`,
    /// and the value it binds.
    fn bound(&self, at: &Self::At, prefix: &str) -> (String, Self::At);
    /// The place that holds a union case's record, once the walk has bound the
    /// case as `bound` ([`Self::bound`] with the prefix `case`). The bound value
    /// itself by default, where a pattern binds the case's record; a backend
    /// whose binding holds the union, read as the case's class, names the
    /// record within it.
    fn case_record(&self, bound: Self::At) -> Self::At {
        bound
    }
    /// The place that holds an element of the list at `list`, once the walk
    /// has bound it as `bound` ([`Self::bound`] with the prefix `item`). The
    /// bound value itself by default, where a function binds each element; a
    /// backend whose binding is the element's index names the element by it.
    fn list_element(&self, _list: &Self::At, bound: Self::At) -> Self::At {
        bound
    }
    /// The place that holds the value of the optional field or list element at
    /// `field`, where it holds one, once the walk has bound it as `bound`
    /// ([`Self::bound`] with the prefix `present`). The bound value itself by
    /// default, where a function binds the value; a backend that binds none
    /// reads it from the field or the element.
    fn optional_value(&self, _field: &Self::At, bound: Self::At) -> Self::At {
        bound
    }

    /// A `bool`.
    fn boolean(&self, at: &Self::At) -> String;
    /// A value of an enum, by the name its document declares for its variant;
    /// `strict` is false for an enum of an open set, whose number no variant
    /// declares is written as its digits.
    fn enumerated(&self, at: &Self::At, strict: bool) -> String;
    /// A value of the enum `declared` at `at`: [`Self::enumerated`] by
    /// default, where the value names its variant's name and number itself; a
    /// backend whose value names neither, as a C enumerator does not, reads them
    /// through the functions of the enum's own unit, which `declared` names.
    fn enumerated_of(&self, at: &Self::At, declared: &ScopedEnum) -> String {
        self.enumerated(at, declared.strict)
    }
    /// An integer of any width.
    fn integer(&self, at: &Self::At) -> String;
    /// A text.
    fn text(&self, at: &Self::At) -> String;
    /// A byte string.
    fn bytes(&self, at: &Self::At) -> String;
    /// The list at `at`, each element bound to `binding` and written as
    /// `element`.
    fn list(&self, at: &Self::At, binding: &str, element: String) -> String;

    /// A field the record always holds, written as `written`.
    fn held_field(&self, written: String) -> String;
    /// A field that may be absent, at `field`: the value it holds bound to
    /// `binding` and written as `written`, and nothing where it holds none.
    fn optional_field(&self, field: &Self::At, binding: &str, written: String) -> String;
    /// An element of a list that may be absent, at `element`: the value it
    /// holds bound to `binding` and written as `written`, and JSON `null`
    /// where it holds none. Unlike a field it is never left out, since an
    /// element is named by its position in the array, which leaving one out
    /// would move for every element after it.
    fn optional_element(&self, element: &Self::At, binding: &str, written: String) -> String;
    /// A record of `fields`, each its declared id and its field as
    /// [`Self::held_field`] or [`Self::optional_field`] wrote it, in the order
    /// the schema declares them.
    fn record(&self, fields: Vec<(String, String)>) -> String;

    /// The value of the union `union` at `at`, one of `cases`, each its
    /// variant and, where the case holds a record, the name the record is bound
    /// to and its written form. Asked only of a backend that lowers a union
    /// (`TemplateSpelling::lowers_unions`); one that does not keeps this
    /// refusal, which the walk of its matcher has given first.
    fn union(
        &self,
        _at: &Self::At,
        union: &ScopedUnion,
        _cases: Vec<CanonicalCase>,
    ) -> Result<String, String> {
        Err(crate::forge::value_type::union_unlowered(&union.name))
    }
}

/// The expression that writes the value `at`, of the type `ty` read in
/// `scope`, in the canonical form of the test log (docs/SCE_TEST_PROFILE.md
/// §12.5), spelled by `spelling`: [`canonical_form`], the one walk of the
/// form, in the direction of the writer ([`Writing`]). An optional value has a
/// form where it stands in a record, which omits it where it is absent, and in
/// a list, which writes `null` in its place
/// ([`CanonicalSpelling::optional_element`]). Refused, naming why: a number
/// that is no integer, and an optional value anywhere else.
pub(super) fn canonical_value<S: CanonicalSpelling>(
    spelling: &S,
    ty: &ValueType,
    at: &S::At,
    scope: &TypeScope,
) -> Result<String, String> {
    canonical_form(&Writing(spelling), ty, at, scope)
}

/// How a backend spells one direction of the canonical form of a value
/// (docs/SCE_TEST_PROFILE.md §12.5): the writer of a value's form, which every
/// template over a record's unit declares ([`CanonicalSpelling`], through
/// [`Writing`]), or its reader, which reads a recorded input trace's values
/// back (docs/SCE_TTCN3_LANE.md §10.3, §10.5). One expression of the backend's
/// language per value, of the form the SCE type takes rather than the
/// language's type.
///
/// The walk over the type, which form each type takes and which scope each
/// record and union is read in, is [`canonical_form`], written once for both
/// directions, so a reader cannot read a form its writer does not write. Each
/// method is told what a reader needs and a writer does not read: an integer's
/// width, a text's, a byte string's and a list's bounds, and the record a set
/// of fields builds, since a value that does not belong to its type is refused
/// where it is read.
pub(crate) trait CanonicalForm {
    /// The backend's spelling of the value the walk stands at.
    type At;

    /// The value the unit's function takes: the template's own value, or the
    /// value read.
    fn root(&self) -> Self::At;
    /// The field `id` of the record at `at`.
    fn field(&self, at: &Self::At, id: &str) -> Self::At;
    /// The name a function or a pattern binds a value inside the one at `at`
    /// to, numbered by how many bindings `at` stands inside after `prefix`,
    /// and the value it binds.
    fn bound(&self, at: &Self::At, prefix: &str) -> (String, Self::At);
    /// The place that holds a union case's record, `record:<record>` in the
    /// union's scope, once the walk has bound the case as `bound`
    /// ([`Self::bound`] with the prefix `case`): what a direction that names
    /// the type of each value it reads reads the case's fields by.
    fn case_record(&self, bound: Self::At, record: &str) -> Self::At;
    /// The place that holds an element of the list at `list`, once the walk
    /// has bound it as `bound` ([`Self::bound`] with the prefix `item`).
    fn list_element(&self, list: &Self::At, bound: Self::At) -> Self::At;
    /// The place that holds the value of the optional field or list element at
    /// `field`, where it holds one, once the walk has bound it as `bound`
    /// ([`Self::bound`] with the prefix `present`).
    fn optional_value(&self, field: &Self::At, bound: Self::At) -> Self::At;

    /// A `bool`.
    fn boolean(&self, at: &Self::At) -> String;
    /// A value of the enum `declared`.
    fn enumerated(&self, at: &Self::At, declared: &ScopedEnum) -> String;
    /// An integer of the width `width`.
    fn integer(&self, at: &Self::At, width: &SceType) -> String;
    /// A text of at most `max` bytes.
    fn text(&self, at: &Self::At, max: &Bound) -> String;
    /// A byte string of at most `max` octets.
    fn bytes(&self, at: &Self::At, max: &Bound) -> String;
    /// The list at `at`, of `max`'s count of elements, each element bound to
    /// `binding` and spelled as `element`.
    fn list(&self, at: &Self::At, max: &Bound, binding: &str, element: String) -> String;

    /// A field the record always holds, spelled as `written`.
    fn held_field(&self, written: String) -> String;
    /// A field that may be absent, at `field`: the value it holds bound to
    /// `binding` and spelled as `written`, and nothing where it holds none.
    fn optional_field(&self, field: &Self::At, binding: &str, written: String) -> String;
    /// An element of a list that may be absent, at `element`: the value it
    /// holds bound to `binding` and spelled as `written`, and JSON `null`
    /// where it holds none.
    fn optional_element(&self, element: &Self::At, binding: &str, written: String) -> String;
    /// The record `record` at `at`, of `fields`, each its declared id and its
    /// field as [`Self::held_field`] or [`Self::optional_field`] spelled it, in
    /// the order the schema declares them.
    fn record(&self, at: &Self::At, record: &ScopedRecord, fields: Vec<(String, String)>)
        -> String;
    /// The value of the union `union` at `at`, one of `cases`, each its
    /// variant and, where the case holds a record, the name the record is
    /// bound to and its spelled form.
    fn union(
        &self,
        at: &Self::At,
        union: &ScopedUnion,
        cases: Vec<CanonicalCase>,
    ) -> Result<String, String>;
}

/// The writer of a value's canonical form, a backend's [`CanonicalSpelling`],
/// as the direction [`canonical_form`] walks: it reads no width and no bound,
/// since a value it writes is one of its type already.
pub(super) struct Writing<'s, S>(pub(super) &'s S);

impl<S: CanonicalSpelling> CanonicalForm for Writing<'_, S> {
    type At = S::At;

    fn root(&self) -> S::At {
        self.0.root()
    }
    fn field(&self, at: &S::At, id: &str) -> S::At {
        self.0.field(at, id)
    }
    fn bound(&self, at: &S::At, prefix: &str) -> (String, S::At) {
        self.0.bound(at, prefix)
    }
    fn case_record(&self, bound: S::At, _record: &str) -> S::At {
        self.0.case_record(bound)
    }
    fn list_element(&self, list: &S::At, bound: S::At) -> S::At {
        self.0.list_element(list, bound)
    }
    fn optional_value(&self, field: &S::At, bound: S::At) -> S::At {
        self.0.optional_value(field, bound)
    }
    fn boolean(&self, at: &S::At) -> String {
        self.0.boolean(at)
    }
    fn enumerated(&self, at: &S::At, declared: &ScopedEnum) -> String {
        self.0.enumerated_of(at, declared)
    }
    fn integer(&self, at: &S::At, _width: &SceType) -> String {
        self.0.integer(at)
    }
    fn text(&self, at: &S::At, _max: &Bound) -> String {
        self.0.text(at)
    }
    fn bytes(&self, at: &S::At, _max: &Bound) -> String {
        self.0.bytes(at)
    }
    fn list(&self, at: &S::At, _max: &Bound, binding: &str, element: String) -> String {
        self.0.list(at, binding, element)
    }
    fn held_field(&self, written: String) -> String {
        self.0.held_field(written)
    }
    fn optional_field(&self, field: &S::At, binding: &str, written: String) -> String {
        self.0.optional_field(field, binding, written)
    }
    fn optional_element(&self, element: &S::At, binding: &str, written: String) -> String {
        self.0.optional_element(element, binding, written)
    }
    fn record(&self, _at: &S::At, _record: &ScopedRecord, fields: Vec<(String, String)>) -> String {
        self.0.record(fields)
    }
    fn union(
        &self,
        at: &S::At,
        union: &ScopedUnion,
        cases: Vec<CanonicalCase>,
    ) -> Result<String, String> {
        self.0.union(at, union, cases)
    }
}

/// The expression of the value `at`, of the type `ty` read in `scope`, in the
/// canonical form of the test log (docs/SCE_TEST_PROFILE.md §12.5), spelled by
/// `spelling` in its direction: the one walk of the form, which both the
/// writer of a value's form ([`canonical_value`]) and its reader take. The
/// form is the SCE type's, never the language's. Each record and union is read
/// in the scope of its own document, as the walk of its matcher reads it. An
/// optional value has a form where it stands in a record, which omits it where
/// it is absent, and in a list, which writes `null` in its place
/// ([`CanonicalForm::optional_element`]). Refused, naming why: a number that
/// is no integer, and an optional value anywhere else.
pub(crate) fn canonical_form<S: CanonicalForm>(
    spelling: &S,
    ty: &ValueType,
    at: &S::At,
    scope: &TypeScope,
) -> Result<String, String> {
    Ok(match ty {
        TypeOf::Scalar(SceType::Bool) => spelling.boolean(at),
        TypeOf::Scalar(SceType::Enum(reference)) => {
            let declared = scope
                .enums
                .get(&reference.alias)
                .ok_or_else(|| unimported("enum", &reference.alias))?;
            spelling.enumerated(at, declared)
        }
        TypeOf::Scalar(scalar) if scalar.int_value_range().is_some() => {
            spelling.integer(at, scalar)
        }
        TypeOf::Scalar(_) => return Err(formless(ty)),
        TypeOf::Text { max } => spelling.text(at, max),
        TypeOf::Bytes { max } => spelling.bytes(at, max),
        TypeOf::Optional { .. } => return Err(optional_out_of_place(ty)),
        TypeOf::List { elem, max } => {
            let (binding, bound) = spelling.bound(at, "item");
            let element = spelling.list_element(at, bound);
            let written = match elem.as_ref() {
                TypeOf::Optional { elem: held } => {
                    let (present_binding, present_bound) = spelling.bound(&element, "present");
                    let present = spelling.optional_value(&element, present_bound);
                    let value = canonical_form(spelling, held, &present, scope)?;
                    spelling.optional_element(&element, &present_binding, value)
                }
                other => canonical_form(spelling, other, &element, scope)?,
            };
            spelling.list(at, max, &binding, written)
        }
        TypeOf::Record { alias } => {
            let record = scope
                .records
                .get(alias)
                .ok_or_else(|| unimported("record", alias))?;
            let fields = record
                .fields
                .iter()
                .map(|(id, field_type)| {
                    let field = spelling.field(at, id);
                    let value = match field_type {
                        TypeOf::Optional { elem } => {
                            let (binding, bound) = spelling.bound(&field, "present");
                            let present = spelling.optional_value(&field, bound);
                            let written = canonical_form(spelling, elem, &present, &record.scope)?;
                            spelling.optional_field(&field, &binding, written)
                        }
                        other => spelling.held_field(canonical_form(
                            spelling,
                            other,
                            &field,
                            &record.scope,
                        )?),
                    };
                    Ok((id.clone(), value))
                })
                .collect::<Result<Vec<(String, String)>, String>>()?;
            spelling.record(at, record, fields)
        }
        TypeOf::Union { alias } => {
            let union = scope
                .unions
                .get(alias)
                .ok_or_else(|| unimported("union", alias))?;
            let cases = union
                .cases
                .iter()
                .map(|case| {
                    let held = match &case.record {
                        Some(record) => {
                            let (binding, bound) = spelling.bound(at, "case");
                            let held = spelling.case_record(bound, record);
                            let written = canonical_form(
                                spelling,
                                &TypeOf::Record {
                                    alias: record.clone(),
                                },
                                &held,
                                &union.scope,
                            )?;
                            Some((binding, written))
                        }
                        None => None,
                    };
                    Ok((case.variant.name.clone(), held))
                })
                .collect::<Result<Vec<CanonicalCase>, String>>()?;
            spelling.union(at, union, cases)?
        }
    })
}

/// Why the type `<kind>:<alias>` has no form where it is read: the scope it is
/// read in names no such type, as no document imports it under that alias.
pub(crate) fn unimported(kind: &str, alias: &str) -> String {
    format!("`{kind}:{alias}` names no {kind} the document imports")
}

/// Why a value of the scalar `ty` has no form in the test log
/// (docs/SCE_TEST_PROFILE.md §12.5): it is no integer, no `bool` and no value
/// of an enum.
pub(crate) fn formless(ty: &ValueType) -> String {
    format!("a value of {ty} has no form in the test log")
}

/// Why a value of `ty`, which may be absent, has no form where it stands: the
/// form writes an absent value only as a record omits a field or as a list
/// writes `null` for an element.
pub(crate) fn optional_out_of_place(ty: &ValueType) -> String {
    format!(
        "a value of {ty} has a form in the test log only as a field of a record, which omits \
         it where it is absent, or as an element of a list, which is null where it is absent"
    )
}

/// A backend's spelling that lowers a union whatever
/// `codegen_matrix::UNION_BACKENDS` says, and spells everything else as the
/// backend does: what a backend's union arm is tested through before the
/// backend joins the list, which it joins once the union unit that arm names
/// is written.
#[cfg(test)]
pub(super) struct LowersUnions<S>(pub(super) S);

#[cfg(test)]
impl<S: TemplateSpelling> TemplateSpelling for LowersUnions<S> {
    type At = S::At;

    fn language(&self) -> Language {
        self.0.language()
    }
    fn lowers_unions(&self) -> bool {
        true
    }
    fn root(&self) -> Self::At {
        self.0.root()
    }
    fn present(&self, at: &Self::At) -> Self::At {
        self.0.present(at)
    }
    fn element(&self, at: &Self::At, index: usize) -> Self::At {
        self.0.element(at, index)
    }
    fn field(&self, at: &Self::At, record: &ScopedRecord, id: &str, ty: &ValueType) -> Self::At {
        self.0.field(at, record, id, ty)
    }
    fn case(&self, at: &Self::At, union: &ScopedUnion, variant: &str) -> Result<Self::At, String> {
        self.0.case(at, union, variant)
    }
    fn parameter(
        &self,
        name: &str,
        ty: &ValueType,
        at: &Position<'_, Self::At>,
        purpose: Use,
    ) -> String {
        self.0.parameter(name, ty, at, purpose)
    }
    fn literal(
        &self,
        text: &str,
        at: &Position<'_, Self::At>,
        purpose: Use,
    ) -> Result<String, String> {
        self.0.literal(text, at, purpose)
    }
    fn call(
        &self,
        import: &ImportContext,
        function: TemplateFunction,
        first: Option<&Self::At>,
        args: &[String],
    ) -> String {
        self.0.call(import, function, first, args)
    }
    fn absent(&self, at: &Position<'_, Self::At>) -> String {
        self.0.absent(at)
    }
    fn present_value(&self, at: &Position<'_, Self::At>, value: String) -> String {
        self.0.present_value(at, value)
    }
    fn record_value(
        &self,
        at: &Position<'_, Self::At>,
        record: &ScopedRecord,
        fields: Vec<(String, String)>,
    ) -> String {
        self.0.record_value(at, record, fields)
    }
    fn list_value(&self, at: &Position<'_, Self::At>, elements: Vec<String>) -> String {
        self.0.list_value(at, elements)
    }
    fn case_value(
        &self,
        at: &Position<'_, Self::At>,
        union: &ScopedUnion,
        variant: &str,
        record: Option<String>,
    ) -> Result<String, String> {
        self.0.case_value(at, union, variant, record)
    }
}

#[cfg(test)]
impl<S: JoinSpelling> JoinSpelling for LowersUnions<S> {
    fn connectives(&self) -> Connectives {
        self.0.connectives()
    }
    fn compare(&self, at: &Position<'_, Self::At>, op: Op, negated: bool, rhs: &str) -> String {
        self.0.compare(at, op, negated, rhs)
    }
    fn count(&self, at: &Position<'_, Self::At>, op: Op, negated: bool, n: u32) -> String {
        self.0.count(at, op, negated, n)
    }
    fn absence(&self, at: &Self::At, negated: bool) -> String {
        self.0.absence(at, negated)
    }
    fn bound_optional(
        &self,
        at: &Self::At,
        inner: &Self::At,
        then: &Held<'_, Self::At>,
        absent_holds: bool,
        negated: bool,
        stands: Stands,
    ) -> String {
        self.0
            .bound_optional(at, inner, then, absent_holds, negated, stands)
    }
    fn case_test(
        &self,
        at: &Self::At,
        union: &ScopedUnion,
        variant: &str,
        record: CaseTest<'_, '_, Self::At>,
        negated: bool,
    ) -> String {
        self.0.case_test(at, union, variant, record, negated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::value_type::{Bound, ScopedCase, ScopedVariant};

    /// A spelling of the canonical form that keeps the default of every
    /// method that has one, its positions the names it reads.
    struct Plain;

    impl CanonicalSpelling for Plain {
        type At = String;

        fn root(&self) -> String {
            "value".to_string()
        }
        fn field(&self, at: &String, id: &str) -> String {
            format!("{at}.{id}")
        }
        fn bound(&self, at: &String, prefix: &str) -> (String, String) {
            let name = format!("{prefix}_of_{at}");
            (name.clone(), name)
        }
        fn boolean(&self, at: &String) -> String {
            at.clone()
        }
        fn enumerated(&self, at: &String, strict: bool) -> String {
            format!("{at}:{}", if strict { "closed" } else { "open" })
        }
        fn integer(&self, at: &String) -> String {
            at.clone()
        }
        fn text(&self, at: &String) -> String {
            at.clone()
        }
        fn bytes(&self, at: &String) -> String {
            at.clone()
        }
        fn list(&self, at: &String, binding: &str, element: String) -> String {
            format!("{at}[{binding}: {element}]")
        }
        fn held_field(&self, written: String) -> String {
            written
        }
        fn optional_field(&self, field: &String, binding: &str, written: String) -> String {
            format!("{field}?[{binding}: {written}]")
        }
        fn optional_element(&self, element: &String, binding: &str, written: String) -> String {
            format!("{element}?null[{binding}: {written}]")
        }
        fn record(&self, fields: Vec<(String, String)>) -> String {
            fields
                .into_iter()
                .map(|(id, value)| format!("{id}={value}"))
                .collect::<Vec<String>>()
                .join(",")
        }
    }

    /// The record of a bound union case is, by default, the value the binding
    /// names, which is where a pattern that binds the case's record puts it:
    /// a backend that keeps the default writes what it wrote before the hook.
    #[test]
    fn a_case_record_is_the_bound_value_by_default() {
        let (binding, bound) = Plain.bound(&Plain.root(), "case");
        assert_eq!(binding, "case_of_value");
        assert_eq!(Plain.case_record(bound.clone()), bound);
    }

    /// The element of a bound list and the value of a bound optional field
    /// are, by default, the value the binding names, which is where a function
    /// that binds them puts them: a backend that keeps the defaults writes what
    /// it wrote before the hooks.
    #[test]
    fn a_list_element_and_an_optional_value_are_the_bound_value_by_default() {
        let root = Plain.root();
        let (_, item) = Plain.bound(&root, "item");
        assert_eq!(item, "item_of_value");
        assert_eq!(Plain.list_element(&root, item.clone()), item);
        let field = Plain.field(&root, "note");
        let (_, present) = Plain.bound(&field, "present");
        assert_eq!(present, "present_of_value.note");
        assert_eq!(Plain.optional_value(&field, present.clone()), present);
    }

    /// A value of an enum is, by default, what `enumerated` writes of it,
    /// closed or open as its enum declares: a backend that keeps the default
    /// writes what it wrote before the hook, whatever enum it is of.
    #[test]
    fn an_enum_value_is_spelled_by_enumerated_by_default() {
        let root = Plain.root();
        for strict in [true, false] {
            let declared = ScopedEnum {
                name: "paint_color".to_string(),
                variants: vec![ScopedVariant {
                    name: "red".to_string(),
                    value: 0,
                }],
                underlying: SceType::Uint8,
                strict,
            };
            assert_eq!(
                Plain.enumerated_of(&root, &declared),
                Plain.enumerated(&root, strict)
            );
        }
        assert_eq!(
            Plain.enumerated(&root, true),
            "value:closed",
            "the spelling tells a closed enum from an open one"
        );
    }

    /// An element of a list that may be absent is written through
    /// `optional_element`, its value bound inside the element as a field's is
    /// inside the field, and an optional value anywhere but in a record or a
    /// list is still refused, naming both places it has a form.
    #[test]
    fn an_optional_element_of_a_list_is_written_through_optional_element() {
        let uint8 = || Box::new(TypeOf::Scalar(SceType::Uint8));
        let tags: ValueType = TypeOf::List {
            elem: Box::new(TypeOf::Optional { elem: uint8() }),
            max: Bound::of(2),
        };
        let scope = TypeScope::default();
        assert_eq!(
            canonical_value(&Plain, &tags, &Plain.root(), &scope),
            Ok(
                "value[item_of_value: item_of_value?null[present_of_item_of_value: \
                present_of_item_of_value]]"
                    .to_string()
            )
        );
        let alone: ValueType = TypeOf::Optional { elem: uint8() };
        let refused = canonical_value(&Plain, &alone, &Plain.root(), &scope)
            .expect_err("an optional value alone has no form");
        assert!(
            refused.contains("only as a field of a record")
                && refused.contains("as an element of a list"),
            "{refused}"
        );
    }

    /// A direction of the form that spells what the walk tells it beside the
    /// place: each width, bound and record, which a reader holds a value to.
    struct Typed;

    impl CanonicalForm for Typed {
        type At = String;

        fn root(&self) -> String {
            "value".to_string()
        }
        fn field(&self, at: &String, id: &str) -> String {
            format!("{at}.{id}")
        }
        fn bound(&self, at: &String, prefix: &str) -> (String, String) {
            let name = format!("{prefix}_of_{at}");
            (name.clone(), name)
        }
        fn case_record(&self, bound: String, record: &str) -> String {
            format!("{bound}:{record}")
        }
        fn list_element(&self, _list: &String, bound: String) -> String {
            bound
        }
        fn optional_value(&self, _field: &String, bound: String) -> String {
            bound
        }
        fn boolean(&self, at: &String) -> String {
            format!("bool({at})")
        }
        fn enumerated(&self, at: &String, declared: &ScopedEnum) -> String {
            format!("{}({at})", declared.name)
        }
        fn integer(&self, at: &String, width: &SceType) -> String {
            format!("{}({at})", width.as_attr())
        }
        fn text(&self, at: &String, max: &Bound) -> String {
            format!("text<={}({at})", max.max)
        }
        fn bytes(&self, at: &String, max: &Bound) -> String {
            format!("bytes<={}({at})", max.max)
        }
        fn list(&self, at: &String, max: &Bound, binding: &str, element: String) -> String {
            format!("list{}..{}({at}, {binding}: {element})", max.min, max.max)
        }
        fn held_field(&self, written: String) -> String {
            written
        }
        fn optional_field(&self, field: &String, binding: &str, written: String) -> String {
            format!("{field}?[{binding}: {written}]")
        }
        fn optional_element(&self, element: &String, binding: &str, written: String) -> String {
            format!("{element}?null[{binding}: {written}]")
        }
        fn record(
            &self,
            at: &String,
            record: &ScopedRecord,
            fields: Vec<(String, String)>,
        ) -> String {
            let fields: Vec<String> = fields
                .into_iter()
                .map(|(id, value)| format!("{id}={value}"))
                .collect();
            format!("{}@{at}{{{}}}", record.name, fields.join(","))
        }
        fn union(
            &self,
            at: &String,
            union: &ScopedUnion,
            cases: Vec<CanonicalCase>,
        ) -> Result<String, String> {
            let cases: Vec<String> = cases
                .into_iter()
                .map(|(variant, held)| match held {
                    Some((binding, written)) => format!("{variant}({binding}: {written})"),
                    None => variant,
                })
                .collect();
            Ok(format!("{}@{at}[{}]", union.name, cases.join("|")))
        }
    }

    /// The one walk hands a direction of the form each integer's width, each
    /// text's, byte string's and list's bounds, and the record a set of fields
    /// builds, through every depth: what a reader refuses a value that does not
    /// belong to its type by, and what the writer's spelling is not asked.
    #[test]
    fn the_walk_tells_a_direction_each_width_bound_and_record() {
        let mut inner = TypeScope::default();
        inner.enums.insert(
            "Paint".to_string(),
            ScopedEnum {
                name: "paint_color".to_string(),
                variants: vec![ScopedVariant {
                    name: "red".to_string(),
                    value: 0,
                }],
                underlying: SceType::Uint8,
                strict: true,
            },
        );
        let line = ScopedRecord {
            identity: "order_line.scxml".to_string(),
            name: "order_line".to_string(),
            fields: vec![
                (
                    "color".to_string(),
                    TypeOf::Scalar(SceType::Enum(EnumRef {
                        alias: "Paint".to_string(),
                    })),
                ),
                ("quantity".to_string(), TypeOf::Scalar(SceType::Uint16)),
            ],
            scope: inner,
        };
        let mut scope = TypeScope::default();
        scope.records.insert("Line".to_string(), line);
        let order = ScopedRecord {
            identity: "order.scxml".to_string(),
            name: "order".to_string(),
            fields: vec![
                ("note".to_string(), TypeOf::Text { max: Bound::of(8) }),
                ("key".to_string(), TypeOf::Bytes { max: Bound::of(4) }),
                (
                    "rush".to_string(),
                    TypeOf::Optional {
                        elem: Box::new(TypeOf::Scalar(SceType::Bool)),
                    },
                ),
                (
                    "lines".to_string(),
                    TypeOf::List {
                        elem: Box::new(TypeOf::Record {
                            alias: "Line".to_string(),
                        }),
                        max: Bound::of(3).at_least(1),
                    },
                ),
            ],
            scope,
        };
        let mut outer = TypeScope::default();
        outer.records.insert("Order".to_string(), order);
        let ty: ValueType = TypeOf::Record {
            alias: "Order".to_string(),
        };
        assert_eq!(
            canonical_form(&Typed, &ty, &Typed.root(), &outer),
            Ok(concat!(
                "order@value{note=text<=8(value.note),key=bytes<=4(value.key),",
                "rush=value.rush?[present_of_value.rush: bool(present_of_value.rush)],",
                "lines=list1..3(value.lines, item_of_value.lines: order_line@item_of_value.lines{",
                "color=paint_color(item_of_value.lines.color),",
                "quantity=uint16(item_of_value.lines.quantity)})}"
            )
            .to_string())
        );
    }

    /// The walk hands a direction the alias of each case's record where it
    /// binds the case, the record it then reads in the union's own scope: what
    /// a direction that names the type of each value it reads reads the case's
    /// fields by. A case that holds no record is its variant alone.
    #[test]
    fn the_walk_tells_a_direction_the_record_of_each_case() {
        let card = ScopedRecord {
            identity: "card_payment.scxml".to_string(),
            name: "card_payment".to_string(),
            fields: vec![("amount".to_string(), TypeOf::Scalar(SceType::Uint32))],
            scope: TypeScope::default(),
        };
        let mut union_scope = TypeScope::default();
        union_scope.records.insert("Card".to_string(), card);
        let case = |name: &str, value: i128, record: Option<&str>| ScopedCase {
            variant: ScopedVariant {
                name: name.to_string(),
                value,
            },
            record: record.map(str::to_string),
        };
        let payment = ScopedUnion {
            identity: "order_payment.scxml".to_string(),
            name: "order_payment".to_string(),
            tag: "kind".to_string(),
            tag_enum: "Kind".to_string(),
            cases: vec![case("card", 0, Some("Card")), case("cash", 1, None)],
            scope: union_scope,
        };
        let mut scope = TypeScope::default();
        scope.unions.insert("Payment".to_string(), payment);
        let ty: ValueType = TypeOf::Union {
            alias: "Payment".to_string(),
        };
        assert_eq!(
            canonical_form(&Typed, &ty, &Typed.root(), &scope),
            Ok(concat!(
                "order_payment@value[card(case_of_value: card_payment@case_of_value:Card{",
                "amount=uint32(case_of_value:Card.amount)})|cash]"
            )
            .to_string())
        );
    }
}
