// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! An imported algorithm, lowered to the ECMAScript function a lowered
//! `sce-static` document calls (docs/SCE_ACCEPTED_SUBSET.md §2.15,
//! [`crate::forge::static_js`]).
//!
//! A `sce-static` statechart calls an algorithm from a guard or an assignment,
//! and a generated machine reaches it where the algorithm's own generation put
//! it. The Interpreter has no generated unit to link, so the function travels
//! in the lowered document, installed by the same `<data>` that installs the
//! runtime library, and is called as `SceStatic.algorithms.<name>`.
//!
//! ⚠ One rule for what a body means: its expressions go through the forge
//! expression lowerer against the typing every backend uses
//! ([`AlgorithmTypes`]), so a name, an operand type and a checked integer
//! operation are judged here exactly as they are for Kotlin. What this module
//! owns is the statement shapes, and it refuses, by name, every statement and
//! every signature it does not yet spell.
//!
//! # Values
//!
//! A number, a truth value and a string are what the script engine calls them,
//! and an integer is exact: a Number where a Number holds it, a BigInt where it
//! does not, so a function computes in the width of its types and a hash whose
//! intermediate product passes 2^53 is the hash. A BigInt leaves a function as
//! the value it is, and a statechart that takes it from a call is the one that
//! checks it ([`TypeCtx::exact_integers`]). A list and `bytes` are arrays, a
//! record a plain object of its schema's fields, and none is changed in place:
//! an `<sce:append>` and a field assignment are an assignment of the whole
//! value, written again (`SceStatic.append`, `SceStatic.set`), so a record or a
//! list a caller handed in is never changed under it — a parameter is read-only
//! on every backend, and a copy a body took of one is its own. A constant is a
//! `var` of a function that closes over it, built once when the algorithm is
//! installed.
//!
//! # Imports
//!
//! What a body's expressions are typed against includes the documents it
//! imports: a record's fields come from its event-schema, and a call is typed
//! by the signature of the algorithm it names. [`Imports`] reads them the way
//! every backend does, and gives each callable algorithm its spelling here,
//! `SceStatic.algorithms.<name>`; the callee itself is lowered and installed
//! beside the function that calls it ([`crate::forge::static_js`]).
//!
//! # Failures
//!
//! A failure is a throw. A `may-fail` algorithm's checked operation throws from
//! the library, a `<sce:require>` throws where its condition does not hold, a
//! call of an algorithm that fails throws through the caller, and the
//! expression of the statechart that called the algorithm fails as it does for
//! an overflow of its own: the statement is skipped, the guard is false, and
//! `error.execution` is raised (W3C SCXML 5.9.1, 3.12.2).

use crate::forge::const_fold::{self, ConstSite, ConstValue};
use crate::forge::error::GenerateError;
use crate::forge::expr::{self, ExprTarget};
use crate::forge::generator::{AlgorithmTypes, ImportContext, RecordImport};
use crate::forge::model::{
    AlgorithmConstType, AlgorithmModel, AlgorithmStmt, AlgorithmValueType, ForgeImport,
    ListElemType, SceType,
};
use crate::forge::static_js::RUNTIME_GLOBAL;
use crate::forge::types::{InferredType, TypeCtx};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Words a name may not be in ECMAScript, and the two this lowering itself
/// reads from the scope a function body runs in.
const RESERVED: &[&str] = &[
    "arguments",
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "eval",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "function",
    "if",
    "implements",
    "import",
    "in",
    "instanceof",
    "interface",
    "let",
    "new",
    "null",
    "package",
    "private",
    "protected",
    "public",
    "return",
    "static",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
    "yield",
    "Math",
    RUNTIME_GLOBAL,
];

/// The language the imports are read in. An import is read once, for every
/// fact the typing needs — a record's fields, a callee's parameters and whether
/// it can fail, an enum's variants — and those are the same for every backend.
/// The enrichment also builds what only a backend's own rendering reads (an
/// `include`, a qualified type name), and there is no `Language` for the
/// Interpreter on purpose: one would put it in every forge kind and every
/// conformance run ([`crate::forge::expr::ExprTarget::ALL`]). So the reading is
/// asked in one language, and what is language-specific in its answer is never
/// read here: the only such field the lowering needs, a callee's spelling, is
/// replaced ([`Imports::resolve`]).
const ENRICHMENT_LANGUAGE: crate::generator::Language = crate::generator::Language::Kotlin;

/// The name the lowered document calls `document_name` by: its own name, with
/// anything an identifier cannot hold made `_`.
pub(crate) fn symbol(document_name: &str) -> String {
    document_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// `text` as a single-quoted ECMAScript string.
fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('\'');
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            other => out.push(other),
        }
    }
    out.push('\'');
    out
}

fn refuse(algorithm: &str, what: impl std::fmt::Display) -> GenerateError {
    GenerateError::unsupported(format!(
        "the algorithm `{algorithm}`: {what} has no ecmascript lowering yet"
    ))
}

/// Whether a scalar type is a value this lowering can hold in a local, an
/// element of a list or a field: a number, a truth value or a string. A `bytes`
/// value is a scalar to the model but a list of numbers here, held by a
/// parameter, a buffer and a return and by nothing else, and an enum is the enum
/// document's type, which the lowered document does not import.
fn has_a_value_form(ty: &SceType) -> bool {
    !matches!(ty, SceType::Bytes | SceType::Enum(_))
}

/// A build-time value as ECMAScript writes it: an integer a Number holds
/// exactly as a Number and another as a BigInt, as an expression's literal is.
fn const_literal(algorithm: &str, name: &str, value: ConstValue) -> Result<String, GenerateError> {
    Ok(match value {
        ConstValue::U8(v) => v.to_string(),
        ConstValue::U16(v) => v.to_string(),
        ConstValue::U32(v) => v.to_string(),
        ConstValue::I8(v) => v.to_string(),
        ConstValue::I16(v) => v.to_string(),
        ConstValue::I32(v) => v.to_string(),
        ConstValue::U64(v) if v <= 9_007_199_254_740_991 => v.to_string(),
        ConstValue::U64(v) => format!("{v}n"),
        ConstValue::I64(v) if v.unsigned_abs() <= 9_007_199_254_740_991 => v.to_string(),
        ConstValue::I64(v) => format!("{v}n"),
        ConstValue::F32(v) if v.is_finite() => format!("{v:?}"),
        ConstValue::F64(v) if v.is_finite() => format!("{v:?}"),
        ConstValue::F32(_) | ConstValue::F64(_) => {
            return Err(refuse(
                algorithm,
                format!("the constant `{name}`, which holds a value ECMAScript has no literal for"),
            ))
        }
        ConstValue::Bool(v) => v.to_string(),
    })
}

/// The constants of `m`, each as a `var` — a table folded at build time as an
/// array, a scalar as its value — evaluated here as every backend's are
/// ([`crate::forge::const_fold`]).
fn consts(m: &AlgorithmModel) -> Result<String, GenerateError> {
    let mut budget = const_fold::Budget::default();
    let mut out = String::new();
    for constant in &m.consts {
        check_name(&m.name, &constant.name)?;
        let site = ConstSite {
            algorithm: &m.name,
            const_name: &constant.name,
        };
        let literal = |value: ConstValue| const_literal(&m.name, &constant.name, value);
        let value = match (&constant.sce_type, &constant.fold, &constant.init) {
            (AlgorithmConstType::Array { len, .. }, Some(fold), None) => {
                let values = const_fold::evaluate_fold(fold, &mut budget, site)?;
                if values.len() as u32 != *len {
                    return Err(refuse(
                        &m.name,
                        format!(
                            "the constant `{}`, whose fold gives {} elements where it declares {len}",
                            constant.name,
                            values.len()
                        ),
                    ));
                }
                let elements = values
                    .into_iter()
                    .map(literal)
                    .collect::<Result<Vec<_>, _>>()?;
                format!("[{}]", elements.join(", "))
            }
            (AlgorithmConstType::Scalar(ty), None, Some(init)) => {
                literal(const_fold::evaluate_scalar_init(init, ty, site)?)?
            }
            _ => {
                return Err(refuse(
                    &m.name,
                    format!(
                        "the constant `{}`, which is neither a fold nor a scalar",
                        constant.name
                    ),
                ))
            }
        };
        out.push_str(&format!("var {} = {value}; ", constant.name));
    }
    Ok(out)
}

fn check_name(algorithm: &str, name: &str) -> Result<(), GenerateError> {
    if RESERVED.contains(&name) {
        return Err(GenerateError::unsupported(format!(
            "the algorithm `{algorithm}`: the name `{name}` is one ECMAScript keeps, so it \
             cannot be a variable of the function it becomes; rename it"
        )));
    }
    Ok(())
}

/// The documents an algorithm imports, read for what its body is typed against.
pub(crate) struct Imports {
    contexts: Vec<ImportContext>,
}

impl Imports {
    /// `imports`, resolved from `base_dir` — the directory of the document that
    /// wrote them — and read as every backend reads them.
    ///
    /// A callable algorithm is called here as the library installs it:
    /// `SceStatic.algorithms.<symbol>`. A call of it that fails is a throw, so
    /// nothing wraps it ([`crate::forge::expr`]).
    pub(crate) fn resolve(
        algorithm: &str,
        imports: &[ForgeImport],
        base_dir: &Path,
    ) -> Result<Self, GenerateError> {
        let refuse = |why: String| {
            GenerateError::unsupported(format!(
                "the algorithm `{algorithm}`: its imports cannot be read: {why}"
            ))
        };
        let options = crate::ForgeCompileOptions::default();
        let mut contexts =
            crate::forge::generator::resolve_imports(imports, &ENRICHMENT_LANGUAGE, &options)
                .map_err(|e| refuse(e.to_string()))?;
        crate::validate_and_enrich_imports(
            &mut contexts,
            imports,
            base_dir,
            &ENRICHMENT_LANGUAGE,
            &options,
            algorithm,
        )
        .map_err(|e| refuse(e.error.to_string()))?;
        for context in &mut contexts {
            if context.is_callable_algorithm() {
                context.qualified_call = format!(
                    "{RUNTIME_GLOBAL}.algorithms.{}",
                    symbol(&context.document_name)
                );
            }
        }
        Ok(Self { contexts })
    }

    /// Each callable algorithm's alias, and what a call of it is written as.
    fn renames(&self) -> HashMap<&str, &str> {
        self.contexts
            .iter()
            .filter(|context| context.is_callable_algorithm())
            .map(|context| (context.alias.as_str(), context.qualified_call.as_str()))
            .collect()
    }

    /// The record type the import `alias` names.
    fn record(&self, alias: &str) -> Option<&RecordImport> {
        self.contexts
            .iter()
            .find(|context| context.alias == alias)
            .and_then(|context| context.record.as_ref())
    }
}

/// What a buffer local holds: the elements it is appended to with.
#[derive(Clone, Copy)]
enum BufferElement<'a> {
    /// A byte.
    Byte,
    /// A fixed-width number or a truth value.
    Scalar(&'a SceType),
    /// A record, appended by name.
    Record,
}

/// A `bytes` or `list<T>` local: it starts empty, is appended to forward-only,
/// and is returned by name (SCE_FORGE.md §4.12).
#[derive(Clone, Copy)]
struct Buffer<'a> {
    capacity: u32,
    element: BufferElement<'a>,
}

/// Every buffer local of `stmts`, nested bodies included, by name.
fn collect_buffers<'a>(stmts: &'a [AlgorithmStmt], out: &mut HashMap<&'a str, Buffer<'a>>) {
    for stmt in stmts {
        match stmt {
            AlgorithmStmt::Var {
                name,
                sce_type,
                capacity: Some(capacity),
                ..
            } => {
                let element = match sce_type {
                    AlgorithmValueType::Scalar(SceType::Bytes) => Some(BufferElement::Byte),
                    AlgorithmValueType::List {
                        elem: ListElemType::Scalar(elem),
                    } => Some(BufferElement::Scalar(elem)),
                    AlgorithmValueType::List {
                        elem: ListElemType::Record { .. },
                    } => Some(BufferElement::Record),
                    _ => None,
                };
                if let Some(element) = element {
                    out.insert(
                        name,
                        Buffer {
                            capacity: *capacity,
                            element,
                        },
                    );
                }
            }
            AlgorithmStmt::If {
                then_body,
                else_body,
                ..
            } => {
                collect_buffers(then_body, out);
                if let Some(else_body) = else_body {
                    collect_buffers(else_body, out);
                }
            }
            AlgorithmStmt::While { body, .. } | AlgorithmStmt::Foreach { body, .. } => {
                collect_buffers(body, out)
            }
            _ => {}
        }
    }
}

/// What lowering one body reads and does not change.
struct Body<'a> {
    algorithm: &'a str,
    ctx: &'a TypeCtx<'a>,
    renames: &'a HashMap<&'a str, &'a str>,
    return_ty: InferredType,
    imports: &'a Imports,
    /// Every record parameter, local and foreach item, by name, and the alias
    /// of the schema that types it.
    records: HashMap<&'a str, &'a str>,
    /// Every buffer local.
    buffers: HashMap<&'a str, Buffer<'a>>,
    /// The parameters a `foreach` reads: a list, or bytes.
    iterable: HashSet<&'a str>,
    /// What a `return` hands back by name, without judging it as an
    /// expression: a buffer, a record, a list or bytes parameter. None of them
    /// is an operand, and each is returned whole (SCE_FORGE.md §4.12).
    whole_values: HashSet<&'a str>,
}

impl Body<'_> {
    /// `expr` as ECMAScript, in the slot of type `slot`.
    fn slot(&self, expr_text: &str, slot: InferredType) -> Result<String, GenerateError> {
        expr::transpile_into(expr_text, ExprTarget::Js, self.ctx, self.renames, slot).map_err(
            |refusal| {
                GenerateError::unsupported(format!(
                    "the algorithm `{}`: `{}` has no ecmascript lowering: {}",
                    self.algorithm,
                    expr_text.trim(),
                    refusal.error
                ))
            },
        )
    }

    fn statements(
        &self,
        stmts: &[AlgorithmStmt],
        depth: usize,
    ) -> Result<Vec<String>, GenerateError> {
        stmts.iter().map(|s| self.statement(s, depth)).collect()
    }

    /// The record type the schema `alias` names, or the refusal that says it
    /// is not one this body imports.
    fn record_import(&self, alias: &str) -> Result<&RecordImport, GenerateError> {
        self.imports.record(alias).ok_or_else(|| {
            refuse(
                self.algorithm,
                format!("the record `{alias}`, which no event-schema import names"),
            )
        })
    }

    /// `statement`, with `depth` the number of `<sce:foreach>` it sits in, which
    /// names the loop's counter.
    fn statement(&self, stmt: &AlgorithmStmt, depth: usize) -> Result<String, GenerateError> {
        Ok(match stmt {
            // A buffer starts empty; its capacity is what an append is held to.
            AlgorithmStmt::Var { name, .. } if self.buffers.contains_key(name.as_str()) => {
                check_name(self.algorithm, name)?;
                format!("var {name} = [];")
            }
            AlgorithmStmt::Var {
                name,
                sce_type,
                init: Some(init),
                ..
            } => {
                check_name(self.algorithm, name)?;
                let Some(ty) = sce_type.scalar().filter(|ty| has_a_value_form(ty)) else {
                    return Err(refuse(
                        self.algorithm,
                        format!("the local `{name}` of type `{}`", sce_type.as_attr()),
                    ));
                };
                format!(
                    "var {name} = {};",
                    self.slot(init, InferredType::from_sce_type(ty))?
                )
            }
            AlgorithmStmt::Var { name, sce_type, .. } => {
                return Err(refuse(
                    self.algorithm,
                    format!("the buffer `{name}` of type `{}`", sce_type.as_attr()),
                ))
            }
            // A record built whole: one value per field of the schema, written
            // in the schema's order.
            AlgorithmStmt::RecordVar {
                name,
                alias,
                fields,
                ..
            } => {
                check_name(self.algorithm, name)?;
                let record = self.record_import(alias)?;
                let mut members = Vec::with_capacity(record.fields.len());
                for (id, ty) in &record.fields {
                    let Some(given) = fields.iter().find(|field| &field.name == id) else {
                        return Err(refuse(
                            self.algorithm,
                            format!("the record local `{name}`, which gives no value for `{id}`"),
                        ));
                    };
                    members.push(format!(
                        "{}: {}",
                        quoted(id),
                        self.slot(&given.expr, InferredType::from_sce_type(ty))?
                    ));
                }
                if fields.len() != record.fields.len() {
                    return Err(refuse(
                        self.algorithm,
                        format!("the record local `{name}`, which gives a field `{alias}` does not declare"),
                    ));
                }
                format!("var {name} = {{{}}};", members.join(", "))
            }
            // A record received whole from a call.
            AlgorithmStmt::RecordFromCall {
                name, alias, init, ..
            } => {
                check_name(self.algorithm, name)?;
                let record = self.record_import(alias)?;
                format!(
                    "var {name} = {};",
                    self.slot(init, InferredType::Record(record.id))?
                )
            }
            AlgorithmStmt::Assign { target, expr, .. } => {
                let target = target.trim();
                // A field of a record is the record, written again with that
                // field changed.
                if let Some((root, field)) = target.split_once('.') {
                    let (root, field) = (root.trim(), field.trim());
                    if !self.records.contains_key(root) || field.contains(['.', '[']) {
                        return Err(refuse(
                            self.algorithm,
                            format!("an assignment to the member `{target}`"),
                        ));
                    }
                    let slot =
                        expr::infer_expr_type(target, self.ctx).unwrap_or(InferredType::Unknown);
                    return Ok(format!(
                        "{root} = {RUNTIME_GLOBAL}.set({root}, {}, {});",
                        quoted(field),
                        self.slot(expr, slot)?
                    ));
                }
                if target.contains('[') {
                    return Err(refuse(
                        self.algorithm,
                        format!("an assignment to the element `{target}`"),
                    ));
                }
                let slot = expr::infer_expr_type(target, self.ctx).unwrap_or(InferredType::Unknown);
                format!("{target} = {};", self.slot(expr, slot)?)
            }
            // The buffer written again with the element at its end. One that is
            // already full fails `capacity-exceeded`, as the bounded backends do
            // (SCE_FORGE.md §4.12): a body that outgrows the bound it declared
            // is wrong there, and passing it here would hide that.
            AlgorithmStmt::Append { target, expr, .. } => {
                let target = target.trim();
                let Some(buffer) = self.buffers.get(target) else {
                    return Err(refuse(
                        self.algorithm,
                        format!("<sce:append target=\"{target}\">, which is not a buffer"),
                    ));
                };
                let element = match buffer.element {
                    // A `bytes` value extends the buffer and a byte is
                    // pushed, as the expression's own type says.
                    BufferElement::Byte => {
                        let rhs = expr::infer_expr_type(expr, self.ctx)
                            .map_err(|e| refuse(self.algorithm, e.error))?;
                        if matches!(rhs, InferredType::Bytes) {
                            return Ok(format!(
                                "{target} = {RUNTIME_GLOBAL}.extend({target}, {}, {});",
                                buffer.capacity,
                                self.slot(expr, InferredType::Bytes)?
                            ));
                        }
                        self.slot(
                            expr,
                            InferredType::Int {
                                signed: false,
                                bits: 8,
                            },
                        )?
                    }
                    BufferElement::Scalar(elem) => {
                        self.slot(expr, InferredType::from_sce_type(elem))?
                    }
                    // A record is appended by name: there is no record
                    // expression (SCE_FORGE.md §4.12).
                    BufferElement::Record => {
                        let name = expr.trim();
                        if !self.records.contains_key(name) {
                            return Err(refuse(
                                self.algorithm,
                                format!(
                                    "<sce:append target=\"{target}\"> of `{name}`, which is not a \
                                     record parameter, local or foreach item"
                                ),
                            ));
                        }
                        name.to_string()
                    }
                };
                format!(
                    "{target} = {RUNTIME_GLOBAL}.append({target}, {}, {element});",
                    buffer.capacity
                )
            }
            AlgorithmStmt::If {
                cond,
                then_body,
                else_body,
                ..
            } => {
                let mut text = format!(
                    "if ({}) {{ {} }}",
                    self.slot(cond, InferredType::Bool)?,
                    self.statements(then_body, depth)?.join(" ")
                );
                if let Some(else_body) = else_body {
                    text.push_str(&format!(
                        " else {{ {} }}",
                        self.statements(else_body, depth)?.join(" ")
                    ));
                }
                text
            }
            AlgorithmStmt::While { cond, body, .. } => format!(
                "while ({}) {{ {} }}",
                self.slot(cond, InferredType::Bool)?,
                self.statements(body, depth)?.join(" ")
            ),
            // Each element of a list or of bytes, in order. The item is a local
            // of the function, and a record item is changed, as any record is,
            // by writing it again.
            AlgorithmStmt::Foreach {
                item, source, body, ..
            } => {
                check_name(self.algorithm, item)?;
                let source = source.trim();
                if !self.iterable.contains(source) {
                    return Err(refuse(
                        self.algorithm,
                        format!("<sce:foreach item=\"{item}\" in=\"{source}\">, which is not a list or bytes parameter"),
                    ));
                }
                let counter = format!("__i{depth}");
                format!(
                    "for (var {counter} = 0; {counter} < {source}.length; {counter}++) \
                     {{ var {item} = {source}[{counter}]; {} }}",
                    self.statements(body, depth + 1)?.join(" ")
                )
            }
            // The body goes on only where the condition holds, and fails
            // `precondition` where it does not (SCE_FORGE.md §3.4.1).
            AlgorithmStmt::Require { cond, .. } => format!(
                "{RUNTIME_GLOBAL}.require({}, {});",
                self.slot(cond, InferredType::Bool)?,
                quoted(cond.trim())
            ),
            // A call is typed as the expression it is, by the signature of the
            // algorithm it names; what it answers is not kept, and what it
            // fails with is thrown through this function.
            AlgorithmStmt::Call { target, args, .. } => {
                let call = format!(
                    "{}({})",
                    target.trim(),
                    args.iter()
                        .map(|arg| arg.expr.trim())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                format!("{};", self.slot(&call, InferredType::Unknown)?)
            }
            AlgorithmStmt::Return {
                expr: Some(expr), ..
            } => {
                let name = expr.trim();
                if self.whole_values.contains(name) {
                    format!("return {name};")
                } else {
                    format!("return {};", self.slot(expr, self.return_ty)?)
                }
            }
            AlgorithmStmt::Return { expr: None, .. } => "return;".to_string(),
        })
    }
}

/// Whether `ty` is a parameter or a return this lowering can hold.
fn holds(ty: &AlgorithmValueType) -> bool {
    match ty {
        AlgorithmValueType::Scalar(scalar) => {
            has_a_value_form(scalar) || matches!(scalar, SceType::Bytes)
        }
        AlgorithmValueType::List {
            elem: ListElemType::Scalar(elem),
        } => has_a_value_form(elem),
        AlgorithmValueType::List {
            elem: ListElemType::Record { .. },
        }
        | AlgorithmValueType::Record { .. } => true,
    }
}

/// The ECMAScript function `m` lowers to, as an expression —
/// `function (a, b) { … }` — on one line, for the attribute that carries it.
///
/// A constant is a `var` of a function that closes over it, so the table is
/// built once, when the algorithm is installed, and not on every call.
///
/// Refused by name, never passed on half lowered: a parameter or a return of a
/// type with no form here, and every statement it does not spell.
pub(crate) fn lower(m: &AlgorithmModel, imports: &Imports) -> Result<String, GenerateError> {
    let mut params = Vec::new();
    let mut iterable: HashSet<&str> = HashSet::new();
    let mut whole_values: HashSet<&str> = HashSet::new();
    for param in &m.signature.params {
        check_name(&m.name, &param.name)?;
        if !holds(&param.sce_type) {
            return Err(refuse(
                &m.name,
                format!(
                    "the parameter `{}` of type `{}`",
                    param.name,
                    param.sce_type.as_attr()
                ),
            ));
        }
        let is_bytes = matches!(param.sce_type, AlgorithmValueType::Scalar(SceType::Bytes));
        if param.sce_type.list_elem().is_some() || is_bytes {
            iterable.insert(&param.name);
        }
        if is_bytes || !matches!(param.sce_type, AlgorithmValueType::Scalar(_)) {
            whole_values.insert(&param.name);
        }
        params.push(param.name.clone());
    }
    let mut buffers = HashMap::new();
    collect_buffers(&m.body, &mut buffers);
    whole_values.extend(buffers.keys().copied());

    let options = crate::ForgeCompileOptions::default();
    let types = AlgorithmTypes::collect(m, &imports.contexts, &options)
        .map_err(|e| GenerateError::unsupported(format!("the algorithm `{}`: {e}", m.name)))?;
    let mut ctx = types.type_ctx(m, &imports.contexts);
    // An integer here is exact and may be a BigInt; one leaves the function as
    // the value it is, and a statechart that takes it from a call is the one
    // that checks it ([`TypeCtx::exact_integers`]).
    ctx.exact_integers = true;
    let renames = imports.renames();

    let return_ty = match &m.signature.return_type {
        None => InferredType::Unknown,
        // A `bytes` return is a buffer returned by name, like a list.
        Some(AlgorithmValueType::Scalar(SceType::Bytes)) => InferredType::Unknown,
        Some(ty) if !holds(ty) => {
            return Err(refuse(
                &m.name,
                format!("a return of type `{}`", ty.as_attr()),
            ))
        }
        Some(AlgorithmValueType::Scalar(scalar)) => InferredType::from_sce_type(scalar),
        // A list or a record is returned by name, never judged as an operand.
        Some(AlgorithmValueType::List { .. }) => InferredType::Unknown,
        Some(AlgorithmValueType::Record { alias }) => {
            let Some(record) = imports.record(alias) else {
                return Err(refuse(
                    &m.name,
                    format!("a return of the record `{alias}`, which no event-schema import names"),
                ));
            };
            InferredType::Record(record.id)
        }
    };
    let records: HashMap<&str, &str> = types
        .records
        .iter()
        .map(|(name, alias, _)| (*name, *alias))
        .collect();
    whole_values.extend(records.keys().copied());

    let body = Body {
        algorithm: &m.name,
        ctx: &ctx,
        renames: &renames,
        return_ty,
        imports,
        records,
        buffers,
        iterable,
        whole_values,
    };
    let statements = body.statements(&m.body, 0)?;
    let function = format!(
        "function ({}) {{ {} }}",
        params.join(", "),
        statements.join(" ")
    );
    if m.consts.is_empty() {
        return Ok(function);
    }
    Ok(format!(
        "(function () {{ {}return {function}; }})()",
        consts(m)?
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::model::ForgeDocument;

    /// An algorithm that imports nothing.
    fn no_imports() -> Imports {
        Imports {
            contexts: Vec::new(),
        }
    }

    /// An algorithm document whose `<sce:body>` is `body`, taking `params` and
    /// returning a `uint8`.
    fn document(may_fail: bool, params: &str, body: &str) -> String {
        format!(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="probe" version="1.0">
  <sce:signature>
    {params}
    <sce:return type="uint8"{fail}/>
  </sce:signature>
  <sce:body>
    {body}
  </sce:body>
</scxml>"#,
            fail = if may_fail { r#" may-fail="true""# } else { "" }
        )
    }

    fn lowered(may_fail: bool, params: &str, body: &str) -> Result<String, String> {
        let text = document(may_fail, params, body);
        let parsed =
            crate::forge::parser::parse_forge(&text, crate::DocumentLabel::symmetric("probe"))
                .unwrap_or_else(|e| panic!("the probe does not parse: {e:?}"))
                .expect("an algorithm document");
        let ForgeDocument::Algorithm(algorithm) = parsed else {
            panic!("the probe is not an algorithm");
        };
        lower(&algorithm, &no_imports()).map_err(|e| e.to_string())
    }

    const KIND: &str = r#"<sce:param name="kind" type="uint8"/>"#;

    /// The scalar core, as one function on one line: a local, a conditional
    /// assignment, a return — the shape of `days_in_month` and every `sync_*`
    /// algorithm.
    #[test]
    fn the_scalar_core_becomes_a_function() {
        let function = lowered(
            false,
            KIND,
            r#"<sce:var name="days" type="uint8" init="31"/>
    <sce:if cond="kind === 2"><sce:assign target="days" expr="28"/></sce:if>
    <sce:return expr="days"/>"#,
        )
        .expect("lowers");
        assert_eq!(
            function,
            "function (kind) { var days = 31; if (kind === 2) { days = 28; } return days; }"
        );
        assert!(!function.contains('\n'));
    }

    /// A `may-fail` algorithm checks every integer operation, and a
    /// `<sce:require>` throws where its condition does not hold — the function
    /// fails, which the statechart that called it answers as it does an
    /// overflow of its own.
    #[test]
    fn a_may_fail_algorithm_checks_its_operations_and_its_preconditions() {
        let function = lowered(
            true,
            KIND,
            r#"<sce:require cond="kind &gt;= 1 &amp;&amp; kind &lt;= 9"/>
    <sce:return expr="kind + 1"/>"#,
        )
        .expect("lowers");
        assert!(
            function
                .contains("SceStatic.require(kind >= 1 && kind <= 9, 'kind >= 1 && kind <= 9');"),
            "{function}"
        );
        assert!(
            function.contains("return SceStatic.U8.add(kind, 1);"),
            "{function}"
        );
    }

    /// A list parameter is an array read by `foreach`, `len` and an index, and
    /// the item of a `foreach` is a local of the function. The counter is named
    /// by the depth, so a loop inside a loop has its own.
    #[test]
    fn a_list_parameter_is_read_by_foreach_len_and_an_index() {
        let function = lowered(
            true,
            r#"<sce:param name="xs" type="list&lt;int32&gt;"/>"#,
            r#"<sce:var name="total" type="uint8" init="0"/>
    <sce:foreach item="x" in="xs">
      <sce:foreach item="y" in="xs">
        <sce:assign target="total" expr="total + 1"/>
      </sce:foreach>
    </sce:foreach>
    <sce:return expr="total"/>"#,
        )
        .expect("lowers");
        assert!(
            function.contains(
                "for (var __i0 = 0; __i0 < xs.length; __i0++) { var x = xs[__i0]; \
                 for (var __i1 = 0; __i1 < xs.length; __i1++) { var y = xs[__i1];"
            ),
            "{function}"
        );
    }

    /// A buffer is an array that starts empty and is written again with each
    /// element appended, held to the capacity it declared, and is returned by
    /// name.
    #[test]
    fn a_buffer_is_appended_to_within_its_capacity_and_returned_by_name() {
        let text = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="probe" version="1.0">
  <sce:signature>
    <sce:param name="n" type="uint8"/>
    <sce:return type="list&lt;uint8&gt;" returns-max-size="4"/>
  </sce:signature>
  <sce:body>
    <sce:var name="out" type="list&lt;uint8&gt;" capacity="4"/>
    <sce:append target="out" expr="n"/>
    <sce:return expr="out"/>
  </sce:body>
</scxml>"#;
        let parsed =
            crate::forge::parser::parse_forge(text, crate::DocumentLabel::symmetric("probe"))
                .unwrap_or_else(|e| panic!("the probe does not parse: {e:?}"))
                .expect("an algorithm document");
        let ForgeDocument::Algorithm(algorithm) = parsed else {
            panic!("the probe is not an algorithm");
        };
        let function = lower(&algorithm, &no_imports()).expect("lowers");
        assert_eq!(
            function,
            "function (n) { var out = []; out = SceStatic.append(out, 4, n); return out; }"
        );
    }

    /// A construct with no lowering yet is named, and the algorithm is not
    /// passed on half lowered.
    #[test]
    fn what_it_cannot_spell_yet_is_refused_by_name() {
        let bytes = lowered(
            false,
            r#"<sce:param name="data" type="bytes"/>"#,
            r#"<sce:if cond="data === 'abc'"><sce:return expr="1"/></sce:if>
    <sce:return expr="0"/>"#,
        )
        .expect_err("a bytes literal has no form here");
        assert!(
            bytes.contains("probe") && bytes.contains("`bytes` value"),
            "{bytes}"
        );

        let keyword = lowered(
            false,
            r#"<sce:param name="default" type="uint8"/>"#,
            r#"<sce:return expr="default"/>"#,
        )
        .expect_err("a keyword cannot be a parameter");
        assert!(keyword.contains("`default`"), "{keyword}");
    }

    #[test]
    fn a_symbol_holds_only_what_an_identifier_can() {
        assert_eq!(symbol("days_in_month"), "days_in_month");
        assert_eq!(symbol("sync-failure.v2"), "sync_failure_v2");
        assert_eq!(quoted("it's a\\b"), r"'it\'s a\\b'");
    }
}
