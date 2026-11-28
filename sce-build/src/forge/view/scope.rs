// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The names an expression of a view reads, and what the build proves of it.
//!
//! An expression of a view is SCE's expression language (`SCE_FORGE.md` §3.4) over
//! the rows in scope: before a projection, each alias's `key` and `row`, the
//! fields of a record row read as `t.row.<field>`; after it, the columns by name.
//! It is typed by the one inference every kind's expressions are typed by, in a
//! closed scope ([`TypeCtx::reject_unknown_identifiers`]), and judged total by the
//! one rule a statement of a `sce-static` machine is (docs/adr/0133): every integer
//! operation proven by the interval analysis, no index read, no call of a
//! function that may fail, and a value stored only in a place that holds every
//! value it may be. A view has no `error.execution` to raise (docs/adr/0151, D1).
//!
//! The right side of a left join is of an optional type: each of its names is
//! an optional value, absent where no right row matched, as an outer join's
//! columns are.

use crate::forge::expr::{self, BinOp, Expected, ExprKind, Refusal, TypedExpr, UnaryOp};
use crate::forge::int_ranges::{self, ProvenOperations};
use crate::forge::model::{parse_int_literal, SceType};
use crate::forge::quantity::NumericBaseType;
use crate::forge::type_ctx::StaticEnumSpelling;
use crate::forge::types::{EnumScope, InferredType, OptionalElem, RecordShape, TypeCtx};
use crate::forge::value_type::{ScopedRecord, TypeScope, ValueType};

/// How deep a record's fields are named through the records they hold.
const MOST_DEPTH: usize = 16;

/// An enum the view's imports name, as an expression reads its variants.
#[derive(Debug, Clone)]
struct ScopeEnum {
    alias: String,
    variants: Vec<String>,
    source_name: String,
    carrier: SceType,
    open: bool,
    spelling: StaticEnumSpelling,
}

/// The names in scope at one operator, and their types.
#[derive(Debug, Clone, Default)]
pub(super) struct Scope {
    vars: Vec<(String, InferredType)>,
    records: Vec<String>,
    texts: Vec<(String, usize)>,
    enums: Vec<ScopeEnum>,
}

/// What a row of a table is: a value of a type of the document, or a record of
/// another view's result, read in that view's own scope (docs/adr/0151, D3).
#[derive(Debug, Clone)]
pub(super) enum RowType {
    Value(ValueType),
    Record(ScopedRecord),
}

/// An expression typed in a scope, and what the build proved of it.
pub(super) struct Judged {
    pub(super) tree: TypedExpr,
    /// Why it may fail, where it may.
    pub(super) may_fail: Option<String>,
    /// The most bytes of the text it denotes, where it is a text the build
    /// bounds.
    pub(super) text_bound: Option<u64>,
}

impl Scope {
    /// A scope of no name but the enums `types` holds by their aliases.
    pub(super) fn new(types: &TypeScope) -> Self {
        let enums = types
            .enums
            .iter()
            .map(|(alias, scoped)| ScopeEnum {
                alias: alias.clone(),
                variants: scoped.variants.iter().map(|v| v.name.clone()).collect(),
                source_name: scoped.name.clone(),
                carrier: scoped.underlying.clone(),
                open: !scoped.strict,
                spelling: StaticEnumSpelling::unlowered(alias),
            })
            .collect();
        Self {
            enums,
            ..Self::default()
        }
    }

    /// Name the rows of `alias`: its `key` of the type `key` gives, where it has
    /// one, and its `row` of the type `row` gives, where it has one, each optional
    /// where the alias is the right side of a left join.
    pub(super) fn bind(
        &mut self,
        alias: &str,
        key: Option<&ValueType>,
        row: Option<&RowType>,
        optional: bool,
        types: &TypeScope,
    ) {
        self.records.push(alias.to_string());
        if let Some(key) = key {
            self.value(&format!("{alias}.key"), key, optional, types, 0);
        }
        if let Some(row) = row {
            self.row(&format!("{alias}.row"), row, optional, types);
        }
    }

    /// Name `path` as a row of `row`.
    fn row(&mut self, path: &str, row: &RowType, optional: bool, types: &TypeScope) {
        match row {
            RowType::Value(ty) => self.value(path, ty, optional, types, 0),
            RowType::Record(record) => self.record(path, record, optional, 0),
        }
    }

    /// Name one row of a table by its own names, `key` and `row`: the scope an
    /// `<sce:visible object>` is read in.
    pub(super) fn bind_unaliased(
        &mut self,
        key: Option<&ValueType>,
        row: Option<&RowType>,
        types: &TypeScope,
    ) {
        if let Some(key) = key {
            self.value("key", key, false, types, 0);
        }
        if let Some(row) = row {
            self.row("row", row, false, types);
        }
    }

    /// Name a column of a projection.
    pub(super) fn column(&mut self, name: &str, ty: &ValueType, types: &TypeScope) {
        self.value(name, ty, false, types, 0);
    }

    /// Name `path` as a value of `ty`, a record through each of its fields, and
    /// each optional where `optional` says the whole is.
    fn value(
        &mut self,
        path: &str,
        ty: &ValueType,
        optional: bool,
        types: &TypeScope,
        depth: usize,
    ) {
        if depth > MOST_DEPTH {
            return;
        }
        match ty {
            ValueType::Record { alias } => match types.records.get(alias) {
                Some(record) => self.record(path, record, optional, depth),
                None => self.records.push(path.to_string()),
            },
            other => {
                let ty = if optional {
                    lifted(other)
                } else {
                    InferredType::of_value(other)
                };
                self.vars.push((path.to_string(), ty));
                if let Some(bytes) = text_capacity(other) {
                    self.texts.push((path.to_string(), bytes));
                }
            }
        }
    }

    /// Name `path` as a record of `record`, through each of its fields.
    fn record(&mut self, path: &str, record: &ScopedRecord, optional: bool, depth: usize) {
        self.records.push(path.to_string());
        for (field, field_ty) in &record.fields {
            let field_path = format!("{path}.{field}");
            if optional {
                self.vars.push((field_path.clone(), lifted(field_ty)));
                if let Some(bytes) = text_capacity(field_ty) {
                    self.texts.push((field_path, bytes));
                }
            } else {
                self.value(&field_path, field_ty, false, &record.scope, depth + 1);
            }
        }
    }

    /// The bound a text the place `path` names is held to, where it is one.
    fn text_of(&self, path: &str) -> Option<usize> {
        self.texts
            .iter()
            .find(|(name, _)| name == path)
            .map(|(_, bytes)| *bytes)
    }

    /// The context an expression of this scope is typed in: closed, so a name
    /// it does not carry is refused, as in every forge kind.
    pub(super) fn ctx(&self) -> TypeCtx<'_> {
        let mut ctx = TypeCtx::new();
        ctx.reject_unknown_identifiers = true;
        ctx.reject_unknown_callees = true;
        for record in &self.records {
            ctx.insert_record(record.as_str(), RecordShape::Closed);
        }
        for (name, ty) in &self.vars {
            ctx.insert_var(name.as_str(), *ty);
        }
        for (name, bytes) in &self.texts {
            ctx.insert_string_capacity(name.as_str(), *bytes);
        }
        for e in &self.enums {
            ctx.insert_enum(
                e.alias.as_str(),
                EnumScope {
                    variants: &e.variants,
                    qualified_type: e.spelling.qualified_type.as_str(),
                    source_name: e.source_name.as_str(),
                    carrier: InferredType::from_sce_type(&e.carrier),
                    to_number: e.spelling.to_number.as_str(),
                    from_number: e.spelling.from_number.as_str(),
                    open: e.open,
                },
            );
        }
        ctx
    }

    /// `text` typed in this scope against `expected`, its optional values read only
    /// where the build proves them present (docs/adr/0055), and judged total.
    pub(super) fn judge(&self, text: &str, expected: Expected) -> Result<Judged, Refusal> {
        let text = text.trim();
        let ctx = self.ctx();
        let tree = expr::judged(text, &ctx, expected)?;
        expr::judge_narrowed_use(text, &ctx, expected)?;
        let proven = int_ranges::proven_operations(&tree, &ctx);
        let may_fail = first_failure(&tree, &proven, text);
        let text_bound = match tree.ty {
            InferredType::Optional(OptionalElem::Str) => expr::dotted_path(&tree)
                .and_then(|path| self.text_of(&path))
                .map(|b| b as u64),
            _ => crate::forge::text_bound::text_bound(&tree, &ctx).bounded(),
        };
        Ok(Judged {
            tree,
            may_fail,
            text_bound,
        })
    }
}

/// The type a value of `ty` is read as where it may be absent.
fn lifted(ty: &ValueType) -> InferredType {
    match ty {
        ValueType::Optional { .. } => InferredType::of_value(ty),
        other => InferredType::Optional(OptionalElem::of_element(other)),
    }
}

/// The bound of a text `ty` holds, through an optional value.
fn text_capacity(ty: &ValueType) -> Option<usize> {
    match ty {
        ValueType::Text { max } => usize::try_from(max.max).ok(),
        ValueType::Optional { elem } => text_capacity(elem),
        _ => None,
    }
}

/// Why `node` or an expression under it may fail: an integer operation the
/// interval analysis does not prove, an index read, which may always miss, or a
/// call of a function that declares `may-fail` (docs/adr/0133, D1).
fn first_failure(node: &TypedExpr, proven: &ProvenOperations, text: &str) -> Option<String> {
    let spelled = || {
        node.span
            .clone()
            .and_then(|span| text.get(span))
            .unwrap_or(text)
            .to_string()
    };
    let integer = matches!(node.ty, InferredType::Int { .. } | InferredType::UntypedInt);
    let why = match &node.kind {
        ExprKind::Binary {
            op: BinOp::Add | BinOp::Sub | BinOp::Mul,
            ..
        } if integer && !proven.holds(node) => Some(format!(
            "the integer operation `{}` may leave its type",
            spelled()
        )),
        ExprKind::Binary {
            op: BinOp::Div | BinOp::Mod,
            ..
        } if integer && !proven.holds(node) => Some(format!(
            "the integer division `{}` may divide by zero or leave its type",
            spelled()
        )),
        ExprKind::Unary {
            op: UnaryOp::Neg, ..
        } if integer && !proven.holds(node) => {
            Some(format!("the negation `{}` may leave its type", spelled()))
        }
        ExprKind::Index { .. } => Some(format!(
            "the index read `{}` may miss its list, whose bound holds its length from above \
             only",
            spelled()
        )),
        ExprKind::Call { fails: true, .. } => Some(format!(
            "the call `{}` is of a function that declares may-fail",
            spelled()
        )),
        _ => None,
    };
    why.or_else(|| {
        node.children()
            .into_iter()
            .find_map(|child| first_failure(child, proven, text))
    })
}

/// The integers a value of `ty` may be, through an optional value.
pub(super) fn value_range(ty: InferredType) -> Option<(i128, i128)> {
    match ty {
        InferredType::Int { signed, bits }
        | InferredType::Optional(OptionalElem::Number(NumericBaseType::Int { signed, bits })) => {
            Some(int_range(signed, bits))
        }
        _ => None,
    }
}

/// The integers a place of `ty` holds, through an optional value.
pub(super) fn place_range(ty: &ValueType) -> Option<(i128, i128)> {
    let (signed, bits) = match ty {
        ValueType::Optional { elem } => return place_range(elem),
        ValueType::Scalar(SceType::Uint8) => (false, 8),
        ValueType::Scalar(SceType::Uint16) => (false, 16),
        ValueType::Scalar(SceType::Uint32) => (false, 32),
        ValueType::Scalar(SceType::Uint64) => (false, 64),
        ValueType::Scalar(SceType::Int8) => (true, 8),
        ValueType::Scalar(SceType::Int16) => (true, 16),
        ValueType::Scalar(SceType::Int32) => (true, 32),
        ValueType::Scalar(SceType::Int64) => (true, 64),
        _ => return None,
    };
    Some(int_range(signed, bits))
}

/// The integers of a type of `bits` bits, signed or not.
fn int_range(signed: bool, bits: u8) -> (i128, i128) {
    let bits = u32::from(bits.min(64));
    if signed {
        (-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1)
    } else {
        (0, (1i128 << bits) - 1)
    }
}

/// Whether a value of `ty` is a floating number, through an optional value.
pub(super) fn floating(ty: InferredType) -> bool {
    ty.is_float_like()
        || matches!(
            ty,
            InferredType::Optional(OptionalElem::Number(NumericBaseType::Float { .. }))
        )
}

/// Why storing the value `judged` in a place of `place` may fail, where it may:
/// an integer the place's type does not hold every value of, or a text longer
/// than the place's bound or of no bound the build proves (docs/adr/0133, D1
/// items 4 and 5).
pub(super) fn store_failure(judged: &Judged, place: &ValueType) -> Option<String> {
    let held = match place {
        ValueType::Optional { elem } => elem.as_ref(),
        other => other,
    };
    if let Some((low, high)) = place_range(held) {
        if let ExprKind::NumberLit(literal) = &judged.tree.kind {
            return match parse_int_literal(literal.trim()) {
                Some(n) if n >= low && n <= high => None,
                _ => Some(format!("the literal {literal} is no value of {held}")),
            };
        }
        let (value_low, value_high) = value_range(judged.tree.ty)?;
        return (value_low < low || value_high > high).then(|| {
            format!(
                "a value of {} is stored in {held}, which does not hold every one",
                judged.tree.ty.describe()
            )
        });
    }
    if let ValueType::Text { max } = held {
        if matches!(judged.tree.kind, ExprKind::NullLit) {
            return None;
        }
        return match judged.text_bound {
            Some(bytes) if bytes <= u64::from(max.max) => None,
            Some(bytes) => Some(format!(
                "its text may be {bytes} bytes, and {held} holds {}",
                max.max
            )),
            None => Some(format!(
                "the build proves no bound of its text, and {held} holds {} bytes",
                max.max
            )),
        };
    }
    None
}
