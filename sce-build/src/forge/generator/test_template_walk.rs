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
//! between backends.
//!
//! Matching is on the values the event schema's unit declares, not on an
//! encoding (ADR 0091 C2).

use super::*;
use crate::forge::expr::{wide_literal_spelling, WideLiteral};
use crate::forge::value_type::{ScopedRecord, ScopedUnion, TypeOf, TypeScope, ValueType};
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

/// How a backend spells what the walk decides. A position's spelling is the
/// backend's own ([`Self::At`]); the walk keeps the type and the scope beside
/// it ([`Position`]).
pub(super) trait TemplateSpelling {
    /// The backend's spelling of the value at one position.
    type At: Clone;

    /// The language the unit is written in.
    fn language(&self) -> Language;

    /// Whether the backend lowers a union (`codegen_matrix::UNION_BACKENDS`):
    /// a backend that does not refuses a template whose type reaches one, as
    /// it refuses a schema unit's union field.
    fn lowers_unions(&self) -> bool {
        crate::forge::codegen_matrix::UNION_BACKENDS.contains(&self.language())
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

    /// An absent optional value.
    fn absent(&self) -> String;
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
            TestMatcherForm::ValueRange { lower, upper } => {
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
            TestMatcherForm::Omit => return Ok(self.spelling.absent()),
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
}
