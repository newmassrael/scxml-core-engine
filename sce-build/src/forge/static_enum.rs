// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What an enum value may be used for in a `sce-static` document's expressions
// (docs/SCE_ACCEPTED_SUBSET.md §2.15).
//
// The expression typer declines to claim a type for an enum value
// ([`crate::forge::types::InferredType::from_sce_type`]): the integer an enum
// is carried as belongs to the enum document, which a statechart does not read
// the width of. That is the right answer for the layers that do arithmetic, and
// it leaves nothing to stop `mode == 3` or `mode + 1` or `mode = 7` from being
// accepted and written out, to fail in the generated code's own compiler where
// the author never sees the document.
//
// So this pass says it where the document is read. An enum value is one of
// five things — an `<alias>.<variant>`, a variable declared `enum:<alias>`, a
// conditional whose two branches are values of one enum, the call of an
// imported algorithm that returns one, or `<alias>.fromNumber(n)` — and what it
// may be used for is deliberately small: stored in a variable of its own enum,
// logged, compared with `==` or `!=` to a value of that same enum, handed to an
// imported algorithm whose parameter takes that same enum, and handed to
// `<alias>.toNumber`, which reads the number its variant declares (docs/adr/0047).
// No ordering, no arithmetic, no argument to any other call.
//
// An algorithm's body is held to the same rule ([`check_algorithm`]): its
// parameters, locals, loop items and record fields are typed by the enum they
// are declared as, and its return by the signature. Before that pass an
// algorithm that returned an enum parameter as an `int64`, or an `int64` as an
// enum, passed `check` on every backend, and the Rust it generated
// (`fn status_number(s: Status) -> i64 { return s; }`) did not compile.

use crate::forge::error::{ExprError, ForgeError};
use crate::forge::expr::{dotted_path, parse_to_ast, BinOp, ExprKind, Refusal, TypedExpr};
use crate::forge::expression_site::ExpressionSite;
use crate::forge::generator::{AlgorithmTypes, ImportContext};
use crate::forge::model::{AlgorithmModel, AlgorithmStmt, AlgorithmValueType, SceType};
use crate::forge::type_ctx::INDEXED_ELEMENT;
use crate::forge::types::TypeCtx;

/// The enum each slot of a call of an imported algorithm holds, by the alias the
/// statechart imports it under ([`crate::forge::type_ctx::StaticScope::call_enums`]):
/// what an argument of the call is held to, and what the call is a value of.
#[derive(Debug, Clone)]
pub(crate) struct CallEnums {
    /// The alias the statechart imports the algorithm under, which a refusal names.
    pub callee: String,
    /// One per parameter, in order: the enum it takes, or `None` for a parameter
    /// that is not an enum.
    pub params: Vec<Option<String>>,
    /// The enum it returns, if it returns one.
    pub ret: Option<String>,
}

/// What [`value_enum`] asks of the document it reads.
pub(crate) struct EnumReading<'r> {
    /// Which enum a variable or a field of the document is declared as, for the
    /// identifiers and paths that name one.
    pub var_alias: &'r dyn Fn(&str) -> Option<String>,
    /// The enums of the slots of the call a name is ([`CallEnums`]), for a name
    /// that is the call of an imported algorithm.
    pub calls: &'r dyn Fn(&str) -> Option<CallEnums>,
}

/// The enum alias `expr` is a value of, or `None` when it is not an enum
/// value — refused when it uses one for what an enum value is not for.
pub(crate) fn value_enum(
    expr: &str,
    ctx: &TypeCtx<'_>,
    reading: &EnumReading<'_>,
) -> Result<Option<String>, Refusal> {
    let ast = parse_to_ast(expr)?;
    class_of(&ast, ctx, reading)
}

/// A refusal placed on `node`.
fn refuse(node: &TypedExpr, what: &str) -> Refusal {
    ExprError::UnsupportedConstruct {
        construct: what.to_string(),
        observed: None,
    }
    .at(node.span.clone())
}

/// What an enum value used where none may stand is refused as.
const NOT_AN_OPERAND: &str = "an enum value used as an operand or an argument (an enum value is \
     compared with === and !== to one of its own enum, stored in a variable of it, logged, \
     handed to an imported algorithm whose parameter takes that enum, or read as the number its \
     variant declares by `<alias>.toNumber(value)`)";

fn class_of(
    node: &TypedExpr,
    ctx: &TypeCtx<'_>,
    reading: &EnumReading<'_>,
) -> Result<Option<String>, Refusal> {
    match &node.kind {
        // `<alias>.<variant>`: the variant itself was judged against the
        // enum's own set by the expression pass that ran before this one.
        ExprKind::Member { object, property } => {
            if let ExprKind::Ident(alias) = &object.kind {
                if ctx.lookup_enum(alias).is_some() {
                    return Ok(Some(alias.clone()));
                }
            }
            // `<record>.<field>` or `_event.data.<field>`: the path of a field of
            // a record, or of the payload, that holds an enum — whatever the
            // number of names it is spelled in.
            if let Some(held) = dotted_path(node).and_then(|path| (reading.var_alias)(&path)) {
                return Ok(Some(held));
            }
            // `<list>[<index>].<field>`: the field of the element a list of records
            // is indexed at, held under `<list>[].<field>` whatever the index is.
            if let ExprKind::Index {
                object: list,
                index,
            } = &object.kind
            {
                if let ExprKind::Ident(name) = &list.kind {
                    if let Some(held) =
                        (reading.var_alias)(&format!("{name}{INDEXED_ELEMENT}{property}"))
                    {
                        if class_of(index, ctx, reading)?.is_some() {
                            return Err(refuse(index, "an enum value as an index"));
                        }
                        return Ok(Some(held));
                    }
                }
            }
            nothing_but(node, ctx, reading)
        }
        ExprKind::Ident(name) => Ok((reading.var_alias)(name)),
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            if class_of(condition, ctx, reading)?.is_some() {
                return Err(refuse(condition, "an enum value as a condition"));
            }
            let then = class_of(consequent, ctx, reading)?;
            let otherwise = class_of(alternate, ctx, reading)?;
            match (then, otherwise) {
                (None, None) => Ok(None),
                (Some(a), Some(b)) if a == b => Ok(Some(a)),
                _ => Err(refuse(
                    node,
                    "a conditional whose branches are not both values of one enum, or both \
                     not enum values",
                )),
            }
        }
        ExprKind::Binary {
            op: BinOp::StrictEq | BinOp::StrictNeq,
            left,
            right,
        } => {
            let l = class_of(left, ctx, reading)?;
            let r = class_of(right, ctx, reading)?;
            match (l, r) {
                (None, None) => Ok(None),
                (Some(a), Some(b)) if a == b => Ok(None),
                _ => Err(refuse(
                    node,
                    "a comparison of an enum value with anything but a value of the same enum",
                )),
            }
        }
        // A conversion of an imported enum: `toNumber` takes a value of the enum
        // and is the number its variant declares, and `fromNumber` takes a number
        // and is a value of the enum (docs/adr/0047).
        ExprKind::Call { callee, args, .. }
            if crate::forge::expr::is_enum_conversion(callee, ctx) =>
        {
            let ExprKind::Member { object, property } = &callee.kind else {
                return nothing_but(node, ctx, reading);
            };
            let ExprKind::Ident(alias) = &object.kind else {
                return nothing_but(node, ctx, reading);
            };
            let to_number = property == crate::forge::types::ENUM_TO_NUMBER;
            for arg in args {
                let held = class_of(arg, ctx, reading)?;
                let wrong = match (to_number, held) {
                    (true, Some(held)) if held == *alias => None,
                    (true, Some(held)) => Some(format!(
                        "a value of the enum `{held}` handed to `{alias}.{property}`, which takes \
                         a value of the enum `{alias}`"
                    )),
                    (true, None) => Some(format!(
                        "a value that is not of the enum `{alias}` handed to `{alias}.{property}`, \
                         which takes a value of that enum (write `{alias}.<variant>`, or a \
                         variable or a field of the enum)"
                    )),
                    (false, None) => None,
                    (false, Some(held)) => Some(format!(
                        "a value of the enum `{held}` handed to `{alias}.{property}`, which takes \
                         a number (`{held}.{}(value)` reads the number its variant declares)",
                        crate::forge::types::ENUM_TO_NUMBER
                    )),
                };
                if let Some(wrong) = wrong {
                    return Err(refuse(arg, &wrong));
                }
            }
            Ok((!to_number).then(|| alias.clone()))
        }
        // The call of an imported algorithm: each argument is held to the enum its
        // parameter takes, which the statechart and the algorithm import from one
        // document (docs/SCE_ACCEPTED_SUBSET.md §2.15), and the call is a value of
        // the enum it returns.
        ExprKind::Call { callee, args, .. } => {
            let slots = match &callee.kind {
                ExprKind::Ident(name) => (reading.calls)(name),
                _ => None,
            };
            let Some(slots) = slots else {
                return nothing_but(node, ctx, reading);
            };
            for (index, arg) in args.iter().enumerate() {
                let held = class_of(arg, ctx, reading)?;
                let wanted = slots.params.get(index).cloned().flatten();
                match (wanted, held) {
                    (None, None) => {}
                    (Some(wanted), Some(held)) if wanted == held => {}
                    (Some(wanted), Some(held)) => {
                        return Err(refuse(
                            arg,
                            &format!(
                                "a value of the enum `{held}` handed to `{}`, whose parameter \
                                 takes a value of the enum `{wanted}`",
                                slots.callee
                            ),
                        ))
                    }
                    (Some(wanted), None) => {
                        return Err(refuse(
                            arg,
                            &format!(
                                "a value that is not of the enum `{wanted}` handed to `{}`, \
                                 whose parameter takes one (write `{wanted}.<variant>`, or a \
                                 variable or a field of that enum)",
                                slots.callee
                            ),
                        ))
                    }
                    (None, Some(_)) => return Err(refuse(arg, NOT_AN_OPERAND)),
                }
            }
            Ok(slots.ret)
        }
        _ => nothing_but(node, ctx, reading),
    }
}

/// `node` is none of the shapes an enum value can take: no part of it may be
/// one either, since an operator or a call has no meaning for an enum value.
fn nothing_but(
    node: &TypedExpr,
    ctx: &TypeCtx<'_>,
    reading: &EnumReading<'_>,
) -> Result<Option<String>, Refusal> {
    for child in node.children() {
        if class_of(child, ctx, reading)?.is_some() {
            return Err(refuse(child, NOT_AN_OPERAND));
        }
    }
    Ok(None)
}

/// What a place of an algorithm's body holds: a value of one enum, by the alias the
/// algorithm imports it under; a number, a bool, a string or bytes; or a record or a
/// list, which is named and not computed, and holds no enum value whole.
enum Slot {
    Enum(String),
    Scalar,
    Named,
}

impl Slot {
    /// The place a value of `ty` is declared to stand in.
    fn of(ty: &AlgorithmValueType) -> Self {
        match ty.scalar() {
            Some(SceType::Enum(reference)) => Self::Enum(reference.alias.clone()),
            Some(_) => Self::Scalar,
            None => Self::Named,
        }
    }

    /// The place a parameter of a call that holds the enum `wanted` (`None` for a
    /// parameter that holds none) is.
    fn of_call(wanted: Option<&str>) -> Self {
        wanted.map_or(Self::Scalar, |alias| Self::Enum(alias.to_string()))
    }
}

/// Every expression of the algorithm `m`'s body held to the rule this module
/// keeps for a statechart: an enum value stands only where a value of that enum
/// is declared — a parameter, a local, a record field or a slot of a call of that
/// enum, or the return of an algorithm that returns it — and a number never does.
/// `imports` is the algorithm's own, enriched ([`ImportContext::enum_slots`] names
/// the enum each slot of an imported algorithm holds). Judged once, before any
/// backend renders the body, so the verdict does not depend on the backend.
pub(crate) fn check_algorithm(
    m: &AlgorithmModel,
    imports: &[ImportContext],
    options: &crate::ForgeCompileOptions,
) -> Result<(), ForgeError> {
    let types = AlgorithmTypes::collect(m, imports, options)?;
    let ctx = types.type_ctx(m, imports);
    let var_alias = |path: &str| types.enum_at(path).map(str::to_string);
    let calls = |name: &str| callee_enums(imports, name);
    let reading = EnumReading {
        var_alias: &var_alias,
        calls: &calls,
    };
    let walk = AlgorithmWalk {
        m,
        types: &types,
        imports,
        ctx: &ctx,
        reading: &reading,
    };
    walk.block(&m.body)
}

/// The enums of the slots of the algorithm `name` names — its alias, or the
/// `<alias>.<name>` spelling a `<sce:call>` may write — among `imports`.
fn callee_enums(imports: &[ImportContext], name: &str) -> Option<CallEnums> {
    let alias = name.split_once('.').map_or(name, |(alias, _)| alias);
    let import = imports
        .iter()
        .find(|i| i.alias == alias && i.kind == "algorithm")?;
    let slots = import.enum_slots.as_ref()?;
    Some(CallEnums {
        callee: alias.to_string(),
        params: slots.params.clone(),
        ret: slots.ret.clone(),
    })
}

/// The walk of one algorithm's body ([`check_algorithm`]).
struct AlgorithmWalk<'w, 'c> {
    m: &'w AlgorithmModel,
    types: &'w AlgorithmTypes<'w>,
    imports: &'w [ImportContext],
    ctx: &'w TypeCtx<'c>,
    reading: &'w EnumReading<'w>,
}

impl AlgorithmWalk<'_, '_> {
    fn block(&self, body: &[AlgorithmStmt]) -> Result<(), ForgeError> {
        body.iter().try_for_each(|stmt| self.statement(stmt))
    }

    fn statement(&self, stmt: &AlgorithmStmt) -> Result<(), ForgeError> {
        match stmt {
            AlgorithmStmt::Var {
                sce_type,
                init: Some(init),
                init_spelling,
                ..
            } => self.expect(init, init_spelling.as_ref(), &Slot::of(sce_type)),
            AlgorithmStmt::Var { init: None, .. } => Ok(()),
            AlgorithmStmt::RecordVar { name, fields, .. } => {
                let record = self
                    .types
                    .records
                    .iter()
                    .find(|(held, _, _)| *held == name.as_str())
                    .map(|(_, _, record)| *record);
                for field in fields {
                    let slot = match record.and_then(|r| r.field(&field.name)) {
                        Some(declared) => Slot::of(&declared.ty.value_type()),
                        // A field the schema does not declare is refused where the
                        // record is resolved; its value is judged as a number's.
                        None => Slot::Scalar,
                    };
                    self.expect(&field.expr, field.expr_spelling.as_ref(), &slot)?;
                }
                Ok(())
            }
            AlgorithmStmt::RecordFromCall {
                init,
                init_spelling,
                ..
            } => self.expect(init, init_spelling.as_ref(), &Slot::Named),
            AlgorithmStmt::Assign {
                target,
                expr,
                expr_spelling,
                ..
            } => {
                let slot = self
                    .types
                    .enum_at(target.trim())
                    .map_or(Slot::Scalar, |alias| Slot::Enum(alias.to_string()));
                self.expect(expr, expr_spelling.as_ref(), &slot)
            }
            AlgorithmStmt::Append {
                target,
                expr,
                expr_spelling,
                ..
            } => {
                let slot = self.buffer_element(target.trim());
                self.expect(expr, expr_spelling.as_ref(), &slot)
            }
            AlgorithmStmt::If {
                cond,
                cond_spelling,
                then_body,
                else_body,
            } => {
                self.expect(cond, cond_spelling.as_ref(), &Slot::Scalar)?;
                self.block(then_body)?;
                else_body.as_deref().map_or(Ok(()), |body| self.block(body))
            }
            AlgorithmStmt::While {
                cond,
                cond_spelling,
                body,
                ..
            } => {
                self.expect(cond, cond_spelling.as_ref(), &Slot::Scalar)?;
                self.block(body)
            }
            AlgorithmStmt::Foreach { body, .. } => self.block(body),
            AlgorithmStmt::Return {
                expr: Some(expr),
                expr_spelling,
            } => {
                let slot = self
                    .m
                    .signature
                    .return_type
                    .as_ref()
                    .map_or(Slot::Scalar, Slot::of);
                self.expect(expr, expr_spelling.as_ref(), &slot)
            }
            AlgorithmStmt::Return { expr: None, .. } => Ok(()),
            AlgorithmStmt::Call { target, args, .. } => {
                let slots = callee_enums(self.imports, target.trim());
                for (index, arg) in args.iter().enumerate() {
                    let wanted = slots
                        .as_ref()
                        .and_then(|s| s.params.get(index))
                        .and_then(|held| held.as_deref());
                    self.expect(&arg.expr, arg.spelling.as_ref(), &Slot::of_call(wanted))?;
                }
                Ok(())
            }
            AlgorithmStmt::Require {
                cond,
                cond_spelling,
            } => self.expect(cond, cond_spelling.as_ref(), &Slot::Scalar),
        }
    }

    /// The place an element appended to the buffer local `target` stands in.
    fn buffer_element(&self, target: &str) -> Slot {
        use crate::forge::model::{AlgorithmBinding, ListElemType};
        self.m
            .body_bindings()
            .into_iter()
            .find_map(|binding| match binding {
                AlgorithmBinding::Local { name, sce_type } if name == target => {
                    Some(match sce_type.list_elem() {
                        Some(ListElemType::Scalar(elem)) => {
                            Slot::of(&AlgorithmValueType::Scalar(elem.clone()))
                        }
                        Some(ListElemType::Record { .. }) => Slot::Named,
                        None => Slot::Scalar,
                    })
                }
                _ => None,
            })
            .unwrap_or(Slot::Scalar)
    }

    /// `expr`, written at `spelling`, held to the place `slot` it stands in.
    fn expect(
        &self,
        expr: &str,
        spelling: Option<&crate::attribute_spelling::AttributeSpelling>,
        slot: &Slot,
    ) -> Result<(), ForgeError> {
        let site = ExpressionSite::new(expr, spelling);
        let held = value_enum(expr, self.ctx, self.reading).map_err(|r| site.place(r))?;
        let construct = match (slot, held) {
            (Slot::Named, _) | (Slot::Scalar, None) => return Ok(()),
            (Slot::Enum(wanted), Some(held)) if *wanted == held => return Ok(()),
            (Slot::Scalar, Some(held)) => format!(
                "a value of the enum `{held}` where a number or a bool is expected \
                 (`{held}.toNumber(value)` is the number its variant declares)"
            ),
            (Slot::Enum(wanted), Some(held)) => format!(
                "a value of the enum `{held}` where a value of the enum `{wanted}` is declared"
            ),
            (Slot::Enum(wanted), None) => format!(
                "a value that is not of the enum `{wanted}` where a value of it is declared \
                 (write `{wanted}.<variant>`, or a parameter, a local or a field of that enum)"
            ),
        };
        Err(site.place(
            ExprError::UnsupportedConstruct {
                construct,
                observed: Some(expr.trim().to_string()),
            }
            .at(None),
        ))
    }
}
