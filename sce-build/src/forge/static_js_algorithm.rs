// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//! # Unions
//!
//! A value of `union:U` is the plain object of its wire form (docs/adr/0092,
//! section 2.1 and section 7, the Interpreter row): the tag's key holding the
//! declared name of the case's variant, which is what a value of the tag's enum
//! is here, beside the fields of the case's record. So the tag is `u.kind` and
//! a field of a case `u.f`, the read every proven read is, whichever cases the
//! narrowing leaves; the judge that runs before any backend
//! ([`crate::forge::narrowing::check_algorithm`]) has admitted each read only
//! where its cases are proven, and runs before this lowering too ([`judge`]).
//! A union local is the object built whole as the case its tag names
//! ([`AlgorithmUnions::built`]), never written afterwards, and a union is
//! returned by name. The end of a body no path reaches ([`ends_unreached`])
//! throws, the script engine's statement that the point is not reached, with
//! no failure name: it is no failure an algorithm reports.
//!
//! # Maps and sets
//!
//! A map or a set (docs/adr/0108, D5) is what a statechart holds one as
//! ([`crate::forge::static_js`]): the array of its entries in the order their
//! keys were first put, `[key, value]` of a map and the key of a set, read by
//! `len`, `has` and `m[k]` through the library, and never changed in place. A
//! local starts as the empty array and a `<sce:put>`, a `<sce:remove>` and a
//! `<sce:clear>` write it again, a put held to the bound the local states. A
//! parameter lent to be read is the caller's array itself, which nothing
//! writes. One declared `inout` arrives in a holder, `{map, capacity}`, the
//! caller's array and its variable's bound: the function reads the array into
//! a local of the parameter's name, writes that local as it writes a map local,
//! holding a put to the capacity, and leaves the array it ends with in the
//! holder however it returns, which the caller writes back into its variable
//! once the call has returned (D6 item 2). A `<sce:foreach>` walks the array
//! the map held when the loop began, which a write in its body replaces and
//! never changes, so no copy is taken (D6 item 3).
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
//! `error.execution` is raised (§scxml-5.9.1, 3.12.2).

use crate::forge::algorithm_union::{ends_unreached, unreached_end_reason, AlgorithmUnions};
use crate::forge::const_fold::{self, ConstSite, ConstValue};
use crate::forge::error::{ForgeError, GenerateError};
use crate::forge::expr::{self, ExprTarget};
use crate::forge::generator::{
    judge_text_write, AlgorithmTypes, ImportContext, RecordField, RecordImport, TextRefusal,
    TextSource,
};
use crate::forge::model::{
    AlgorithmBinding, AlgorithmConstType, AlgorithmModel, AlgorithmStmt, AlgorithmValueType,
    ForgeImport, RecordFieldInit, SceType,
};
use crate::forge::narrowing::{
    after_algorithm_statement, loop_entry, present_record_of, proves_written, Chain, Facts,
};
use crate::forge::static_js::{LOWERED_DATAMODEL, RUNTIME_GLOBAL};
use crate::forge::types::{InferredType, OptionalElem, TypeCtx};
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

/// `text` as a single-quoted ECMAScript string: the one ECMAScript quoter
/// ([`crate::filters::ecmascript_string_literal`]), which the expression emitter
/// writes a string literal for the Interpreter through too.
fn quoted(text: &str) -> String {
    crate::filters::ecmascript_string_literal(text)
}

fn refuse(algorithm: &str, what: impl std::fmt::Display) -> GenerateError {
    GenerateError::unsupported(format!(
        "the algorithm `{algorithm}`: {what} has no ecmascript lowering yet"
    ))
}

/// [`refuse`], as the error a statement of the body hands on: a statement's
/// lowering also hands on what the judge of a union local refuses, which keeps
/// its own code ([`AlgorithmUnions::built`]).
fn refusal(algorithm: &str, what: impl std::fmt::Display) -> ForgeError {
    refuse(algorithm, what).into()
}

/// Whether a scalar type is a value this lowering can hold in a local, an
/// element of a list or a field: a number, a truth value, a string or a value of
/// an enum. A `bytes` value is a scalar to the model but a list of numbers here,
/// held by a parameter, a buffer and a return and by nothing else. A value of an
/// enum is the name its document declares for the variant, a string: what the
/// Interpreter holds a statechart's enum value as, and what `<alias>.<variant>`
/// is lowered to ([`crate::forge::expr`]), so a statechart hands an algorithm the
/// value it holds and takes back the one the algorithm returns as it is
/// (docs/SCE_ACCEPTED_SUBSET.md §2.15).
fn has_a_value_form(ty: &SceType) -> bool {
    !matches!(ty, SceType::Bytes)
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
            // A value of an enum is its declared name here, so the enum's two
            // conversions read the library's table of its names and numbers.
            if context.kind == "enum" {
                let variants: Vec<(String, i128)> = context
                    .enum_variants
                    .iter()
                    .cloned()
                    .zip(context.enum_texts.iter().map(|text| text.value))
                    .collect();
                (context.enum_to_number, context.enum_from_number) =
                    crate::forge::static_js::enum_number_symbols(&variants);
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

    /// Whether the import `alias` names a bounded-collection.
    fn is_bounded_collection(&self, alias: &str) -> bool {
        self.contexts
            .iter()
            .any(|context| context.alias == alias && context.kind == "bounded-collection")
    }

    /// The record type the import `alias` names.
    fn record(&self, alias: &str) -> Option<&RecordImport> {
        self.contexts
            .iter()
            .find(|context| context.alias == alias)
            .and_then(|context| context.record.as_ref())
    }

    /// Why the schema the import `alias` names is no record an algorithm holds
    /// yet, when its field is a composition ([`ImportContext::record_composed`]).
    fn record_composed(&self, alias: &str) -> Option<&str> {
        self.contexts
            .iter()
            .find(|context| context.alias == alias)
            .and_then(|context| context.record_composed.as_deref())
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
    /// A union, appended by the name of the parameter, local or loop item that
    /// holds it (docs/adr/0099, "Still open" item 2).
    Union,
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
            AlgorithmStmt::Var { name, sce_type, .. } => {
                // A buffer states its bound in its type (docs/adr/0086).
                let Some(capacity) = sce_type.stated_outer_bound().map(|bound| bound.max) else {
                    continue;
                };
                let element = match sce_type {
                    AlgorithmValueType::Bytes { .. } => Some(BufferElement::Byte),
                    AlgorithmValueType::List { elem, .. } => match &**elem {
                        AlgorithmValueType::Scalar(elem) => Some(BufferElement::Scalar(elem)),
                        AlgorithmValueType::Record { .. } => Some(BufferElement::Record),
                        AlgorithmValueType::Union { .. } => Some(BufferElement::Union),
                        _ => None,
                    },
                    _ => None,
                };
                if let Some(element) = element {
                    out.insert(name, Buffer { capacity, element });
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

/// A map or a set the body holds (docs/adr/0108, D5): a parameter, `inout` or
/// not, or a local, each the array of its entries ([`lower`]).
struct MapHeld {
    /// The most entries a put leaves in it, as ECMAScript reads it: a local's
    /// stated bound, or the capacity the holder of an `inout` parameter carries
    /// from the caller's variable, since the parameter states none. `None` for
    /// a parameter lent to be read, which no statement writes.
    capacity: Option<String>,
    /// Whether it is a set, each entry of which is the key itself.
    set: bool,
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
    /// Every optional record parameter and local (docs/adr/0055, step 9), by
    /// name, and the alias of the schema that types it: `null` or the plain
    /// object a record is, held whole, and written a field at a time only
    /// where a test proves it present.
    optional_records: HashMap<&'a str, &'a str>,
    /// Every buffer local.
    buffers: HashMap<&'a str, Buffer<'a>>,
    /// The parameters a `foreach` reads: a list, or bytes.
    iterable: HashSet<&'a str>,
    /// What a `return` hands back by name, without judging it as an
    /// expression: a buffer, a record, a list or bytes parameter, a union
    /// parameter or local. None of them is an operand, and each is returned
    /// whole (SCE_FORGE.md §4.12).
    whole_values: HashSet<&'a str>,
    /// The unions the parameters and locals hold, and the case a union local
    /// builds (docs/adr/0092, Landing order step 5).
    unions: AlgorithmUnions<'a>,
    /// Every map and set parameter and local, by name (docs/adr/0108, D5).
    maps: HashMap<&'a str, MapHeld>,
}

impl Body<'_> {
    /// `expr` as ECMAScript, in the slot of type `slot`, read where nothing is
    /// proven present: a text written to a record's field.
    fn slot(&self, expr_text: &str, slot: InferredType) -> Result<String, GenerateError> {
        self.slot_at(expr_text, slot, &Facts::none())
    }

    /// `expr` as ECMAScript, in the slot of type `slot`, read where `proven`
    /// holds (docs/adr/0055): a `.value` the narrowing admitted is the read of
    /// the present value it is.
    fn slot_at(
        &self,
        expr_text: &str,
        slot: InferredType,
        proven: &Facts,
    ) -> Result<String, GenerateError> {
        let proving;
        let ctx = if proven.is_empty() {
            self.ctx
        } else {
            proving = self.ctx.proving(proven.clone());
            &proving
        };
        expr::transpile_into(expr_text, ExprTarget::Js, ctx, self.renames, slot).map_err(
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

    /// The value written to the `string` field `dest` of a record, judged as
    /// every backend judges it ([`judge_text_write`]) and held to the field's
    /// bound where it is written, so the Interpreter fails a text past it as
    /// the generated backends refuse one (SCE_FORGE.md §4.12).
    fn text_write(&self, rhs: &str, dest: &RecordField) -> Result<String, GenerateError> {
        let bound = dest
            .max_size
            .expect("resolve_record refuses a string field with no bound");
        let record_of = |name: &str| {
            self.records
                .get(name)
                .and_then(|alias| self.imports.record(alias))
        };
        let source = judge_text_write(rhs, dest, &record_of).map_err(|refusal| match refusal {
            TextRefusal::Unreadable(refusal) => refuse(
                self.algorithm,
                format!("`{}`: {}", rhs.trim(), refusal.error),
            ),
            TextRefusal::Shape(what) => refuse(
                self.algorithm,
                format!(
                    "{what} written to the string field `{}` (v1 writes a record's text from a \
                     string literal or from the string field of a record)",
                    dest.id
                ),
            ),
            TextRefusal::TooLong { held, what } => refuse(
                self.algorithm,
                format!(
                    "{what} of up to {held} bytes written to the string field `{}`, which holds \
                     at most {bound}",
                    dest.id
                ),
            ),
        })?;
        let text = match source {
            // The absent value of a field that may be absent holds no text: it is
            // `null`, and nothing is bounded (docs/adr/0055).
            TextSource::Absent => return Ok("null".to_string()),
            TextSource::Literal => self.slot(rhs, InferredType::Str)?,
            TextSource::Field { record, field } => {
                format!("{record}[{}]", quoted(&field.id))
            }
        };
        Ok(format!("{RUNTIME_GLOBAL}.bounded({text}, {bound})"))
    }

    /// The members of a plain object that holds a value of `record`, one per
    /// field in the schema's order, each `'<id>': <value>` with the value the
    /// `<sce:set>` among `given` that names the field gives, read where
    /// `proven` holds: what a record local is, and what a union local holds
    /// beside its tag. `local` names the local a refusal is about.
    fn field_values(
        &self,
        local: &str,
        record: &RecordImport,
        given: &[&RecordFieldInit],
        proven: &Facts,
    ) -> Result<Vec<String>, GenerateError> {
        let mut members = Vec::with_capacity(record.fields.len());
        for field in &record.fields {
            let id = &field.id;
            let Some(set) = given.iter().find(|set| &set.name == id) else {
                return Err(refuse(
                    self.algorithm,
                    format!("{local}, which gives no value for `{id}`"),
                ));
            };
            let value = if field.holds_text() {
                self.text_write(&set.expr, field)?
            } else {
                self.slot_at(&set.expr, InferredType::of_field(&field.ty), proven)?
            };
            members.push(format!("{}: {value}", quoted(id)));
        }
        Ok(members)
    }

    /// The statements of one block, entered with `entry` proven present
    /// (docs/adr/0055): each read where what the ones before it prove holds, as
    /// [`crate::forge::narrowing::check_algorithm`] judged them.
    fn statements(
        &self,
        stmts: &[AlgorithmStmt],
        depth: usize,
        entry: &Facts,
    ) -> Result<Vec<String>, ForgeError> {
        let mut proven = entry.clone();
        let mut lowered = Vec::with_capacity(stmts.len());
        for stmt in stmts {
            lowered.push(self.statement(stmt, depth, &proven)?);
            proven = after_algorithm_statement(&proven, stmt);
        }
        Ok(lowered)
    }

    /// The record type the schema `alias` names, or the refusal that says it
    /// is not one this body imports.
    fn record_import(&self, alias: &str) -> Result<&RecordImport, GenerateError> {
        // A schema whose field is a list, a record or an optional over either is
        // a record no algorithm holds yet, refused by name (docs/adr/0086,
        // section 7.3).
        if let Some(composed) = self.imports.record_composed(alias) {
            return Err(GenerateError::unsupported(composed.to_string()));
        }
        self.imports.record(alias).ok_or_else(|| {
            refuse(
                self.algorithm,
                format!("the record `{alias}`, which no event-schema import names"),
            )
        })
    }

    /// The map or set `target` a `<sce:put>`, a `<sce:remove>` or a
    /// `<sce:clear>` writes (docs/adr/0108, D5): its place, the type the
    /// library reads it by ([`crate::forge::static_js::map_descriptor`]), and
    /// the most entries it holds. Only a local and an `inout` parameter are
    /// written, which the parser held every such statement to; any other name
    /// is refused here all the same rather than written.
    fn map_written(
        &self,
        element: &str,
        target: &str,
    ) -> Result<(crate::forge::value_path::Place, String, &str), ForgeError> {
        let capacity = self
            .maps
            .get(target)
            .and_then(|held| held.capacity.as_deref())
            .ok_or_else(|| {
                refusal(
                    self.algorithm,
                    format!(
                        "{element}, whose target is no map or set local and no inout map or set \
                         parameter"
                    ),
                )
            })?;
        let place = self
            .ctx
            .value_of(target)
            .filter(crate::forge::value_path::Place::is_keyed)
            .ok_or_else(|| {
                refusal(
                    self.algorithm,
                    format!("{element}, whose target this build did not type"),
                )
            })?;
        let descriptor = crate::forge::static_js::map_descriptor(&place, self.ctx.value_scope)
            .ok_or_else(|| {
                refusal(
                    self.algorithm,
                    format!(
                        "{element}, whose key of type `{}` has no ecmascript spelling",
                        place.ty
                    ),
                )
            })?;
        Ok((place, descriptor, capacity))
    }

    /// `written`, a key or a value of type `ty` an `<sce:put>` or an
    /// `<sce:remove>` of the map `element` names hands the library, read where
    /// `proven` holds: a number, a truth value, a value of an enum (its
    /// declared name) and a text as the expressions they are, a text held to
    /// the bound its type states as a record's field is, and a record by the
    /// name of the parameter, local or loop item that holds it, the plain
    /// object no algorithm changes in place. A byte string is the array of
    /// numbers an algorithm holds it as, which a statechart's map does not
    /// hold, and is refused by name with every other type.
    fn map_part(
        &self,
        element: &str,
        written: &str,
        ty: &crate::forge::value_type::ValueType,
        proven: &Facts,
    ) -> Result<String, ForgeError> {
        use crate::forge::value_type::ValueType;
        let written = written.trim();
        match ty {
            ValueType::Record { .. } => {
                let named = self.records.contains_key(written)
                    || self
                        .ctx
                        .value_of(written)
                        .is_some_and(|place| matches!(place.ty, ValueType::Record { .. }));
                if named {
                    Ok(written.to_string())
                } else {
                    Err(refusal(
                        self.algorithm,
                        format!(
                            "{element} of `{written}`, which is not a record parameter, local or \
                             loop item"
                        ),
                    ))
                }
            }
            ValueType::Text { max } => Ok(format!(
                "{RUNTIME_GLOBAL}.bounded({}, {})",
                self.slot_at(written, InferredType::Str, proven)?,
                max.max
            )),
            ValueType::Scalar(scalar) if has_a_value_form(scalar) => self
                .slot_at(written, InferredType::from_sce_type(scalar), proven)
                .map_err(ForgeError::from),
            ValueType::Scalar(_)
            | ValueType::Bytes { .. }
            | ValueType::Optional { .. }
            | ValueType::Union { .. }
            | ValueType::List { .. }
            | ValueType::Map { .. }
            | ValueType::Set { .. } => Err(refusal(
                self.algorithm,
                format!("{element} of `{written}`, a key or a value of type `{ty}`"),
            )),
        }
    }

    /// `statement`, with `depth` the number of `<sce:foreach>` it sits in, which
    /// names the loop's counter, read where `proven` holds (docs/adr/0055).
    fn statement(
        &self,
        stmt: &AlgorithmStmt,
        depth: usize,
        proven: &Facts,
    ) -> Result<String, ForgeError> {
        let lowered = |text: &str, ty: InferredType| self.slot_at(text, ty, proven);
        Ok(match stmt {
            // A map or a set local starts as the array of no entries
            // (docs/adr/0108, D5); its bound is what a put is held to.
            AlgorithmStmt::Var { name, .. } if self.maps.contains_key(name.as_str()) => {
                check_name(self.algorithm, name)?;
                format!("var {name} = [];")
            }
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
                // A local that may be absent is `null` or its present value
                // (docs/adr/0055).
                let ty = match (sce_type.scalar(), sce_type.optional_elem()) {
                    (Some(ty), _) if has_a_value_form(ty) => InferredType::from_sce_type(ty),
                    (None, Some(elem)) if holds(sce_type) => {
                        InferredType::Optional(OptionalElem::of_value(elem))
                    }
                    _ => {
                        return Err(refusal(
                            self.algorithm,
                            format!("the local `{name}` of type `{}`", sce_type.as_attr()),
                        ))
                    }
                };
                format!("var {name} = {};", lowered(init, ty)?)
            }
            AlgorithmStmt::Var { name, sce_type, .. } => {
                return Err(refusal(
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
                let given: Vec<&RecordFieldInit> = fields.iter().collect();
                let members = self.field_values(
                    &format!("the record local `{name}`"),
                    record,
                    &given,
                    proven,
                )?;
                if fields.len() != record.fields.len() {
                    return Err(refusal(
                        self.algorithm,
                        format!("the record local `{name}`, which gives a field `{alias}` does not declare"),
                    ));
                }
                format!("var {name} = {{{}}};", members.join(", "))
            }
            // A union built whole as the case its tag names (docs/adr/0092,
            // section 6.1 item 5): the plain object of its wire form, the tag's
            // key holding the declared name of the case's variant, beside the
            // fields of the case's record, each given as a record local's is.
            // The judge held the build to its case and fields, and refuses a
            // write of the local, so it is declared once and never changed.
            AlgorithmStmt::UnionVar {
                name,
                name_spelling,
                alias,
                fields,
                ..
            } => {
                check_name(self.algorithm, name)?;
                let built = self
                    .unions
                    .built(name, name_spelling.as_ref(), alias, fields)?;
                let tag = self
                    .unions
                    .union(alias)
                    .map(|union| union.tag.as_str())
                    .ok_or_else(|| {
                        refusal(
                            self.algorithm,
                            format!(
                                "the union local `{name}` of `union:{alias}`, a union whose \
                                 document was not read"
                            ),
                        )
                    })?;
                let mut members = vec![format!(
                    "{}: {}",
                    quoted(tag),
                    quoted(&built.case.variant.name)
                )];
                if let Some(record_alias) = built.record.as_deref() {
                    let record = self.record_import(record_alias)?;
                    members.extend(self.field_values(
                        &format!("the union local `{name}`"),
                        record,
                        &built.fields,
                        proven,
                    )?);
                }
                format!("var {name} = {{{}}};", members.join(", "))
            }
            // A union copied whole from an element of a list of its union at an
            // index (docs/adr/0099, section 5.2): the library's checked read,
            // which throws for an index outside the list before the local is
            // declared. The object is the list's own, which no algorithm writes,
            // as a union handed to a call is ("Decided while landing step 5"
            // item 3 of docs/adr/0092).
            AlgorithmStmt::UnionCopied { name, init, .. } => {
                check_name(self.algorithm, name)?;
                format!("var {name} = {};", lowered(init, InferredType::Unknown)?)
            }
            // A record received whole from a call.
            AlgorithmStmt::RecordFromCall {
                name, alias, init, ..
            } => {
                check_name(self.algorithm, name)?;
                let record = self.record_import(alias)?;
                format!(
                    "var {name} = {};",
                    lowered(init, InferredType::Record(record.id))?
                )
            }
            AlgorithmStmt::Assign { target, expr, .. } => {
                let target = target.trim();
                // A field of a record is the record, written again with that
                // field changed; a field of the present value of an optional
                // record (`x.value.f`, written where a test proves it present) is
                // the object `x` holds, written again the same way.
                let held = match present_record_of(target)
                    .filter(|holder| self.optional_records.contains_key(holder))
                {
                    Some(root) => target
                        .rsplit_once('.')
                        .map(|(_, field)| (root, field.trim(), self.optional_records.get(root))),
                    None => target.split_once('.').map(|(root, field)| {
                        let root = root.trim();
                        (root, field.trim(), self.records.get(root))
                    }),
                };
                if let Some((root, field, alias)) = held {
                    let Some(alias) = alias.copied() else {
                        return Err(refusal(
                            self.algorithm,
                            format!("an assignment to the member `{target}`"),
                        ));
                    };
                    if field.contains(['.', '[']) {
                        return Err(refusal(
                            self.algorithm,
                            format!("an assignment to the member `{target}`"),
                        ));
                    }
                    let text_field = self
                        .imports
                        .record(alias)
                        .and_then(|record| record.field(field))
                        .filter(|f| f.holds_text());
                    let value = match text_field {
                        Some(text_field) => self.text_write(expr, text_field)?,
                        None => {
                            let slot = expr::infer_expr_type(target, self.ctx)
                                .unwrap_or(InferredType::Unknown);
                            lowered(expr, slot)?
                        }
                    };
                    return Ok(format!(
                        "{root} = {RUNTIME_GLOBAL}.set({root}, {}, {value});",
                        quoted(field),
                    ));
                }
                // A record put into an optional record is the object it is, by
                // its name: there is no record expression (SCE_FORGE.md §4.12).
                if self.optional_records.contains_key(target) {
                    let name = expr.trim();
                    if self.records.contains_key(name) || self.optional_records.contains_key(name) {
                        return Ok(format!("{target} = {name};"));
                    }
                }
                if target.contains('[') {
                    return Err(refusal(
                        self.algorithm,
                        format!("an assignment to the element `{target}`"),
                    ));
                }
                let slot = expr::infer_expr_type(target, self.ctx).unwrap_or(InferredType::Unknown);
                format!("{target} = {};", lowered(expr, slot)?)
            }
            // The buffer written again with the element at its end. One that is
            // already full fails `capacity-exceeded`, as the bounded backends do
            // (SCE_FORGE.md §4.12): a body that outgrows the bound it declared
            // is wrong there, and passing it here would hide that.
            AlgorithmStmt::Append { target, expr, .. } => {
                let target = target.trim();
                let Some(buffer) = self.buffers.get(target) else {
                    return Err(refusal(
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
                                lowered(expr, InferredType::Bytes)?
                            ));
                        }
                        lowered(
                            expr,
                            InferredType::Int {
                                signed: false,
                                bits: 8,
                            },
                        )?
                    }
                    BufferElement::Scalar(elem) => {
                        lowered(expr, InferredType::from_sce_type(elem))?
                    }
                    // A record is appended by name: there is no record
                    // expression (SCE_FORGE.md §4.12).
                    BufferElement::Record => {
                        let name = expr.trim();
                        if !self.records.contains_key(name) {
                            return Err(refusal(
                                self.algorithm,
                                format!(
                                    "<sce:append target=\"{target}\"> of `{name}`, which is not a \
                                     record parameter, local or foreach item"
                                ),
                            ));
                        }
                        name.to_string()
                    }
                    // A union is appended by the name that holds it, the object
                    // no algorithm writes, which the judge held to the list's
                    // union where the append was lowered on every backend.
                    BufferElement::Union => {
                        let name = expr.trim();
                        if !self.unions.holds(name) {
                            return Err(refusal(
                                self.algorithm,
                                format!(
                                    "<sce:append target=\"{target}\"> of `{name}`, which is not a \
                                     union parameter, local or foreach item"
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
            // The block is read where the condition holds, the `<sce:else>`
            // where it does not (docs/adr/0055).
            AlgorithmStmt::If {
                cond,
                then_body,
                else_body,
                ..
            } => {
                let chain = Chain::of(proven, &[cond.as_str()]);
                let mut text = format!(
                    "if ({}) {{ {} }}",
                    lowered(cond, InferredType::Bool)?,
                    self.statements(then_body, depth, &chain.block(0))?
                        .join(" ")
                );
                if let Some(else_body) = else_body {
                    text.push_str(&format!(
                        " else {{ {} }}",
                        self.statements(else_body, depth, &chain.otherwise())?
                            .join(" ")
                    ));
                }
                text
            }
            // The condition is read where every turn starts, and the body where
            // it holds besides.
            AlgorithmStmt::While { cond, body, .. } => {
                let turn = loop_entry(proven, body);
                format!(
                    "while ({}) {{ {} }}",
                    self.slot_at(cond, InferredType::Bool, &turn)?,
                    self.statements(body, depth, &turn.and(&proves_written(cond, true)))?
                        .join(" ")
                )
            }
            // Each element of a list or of bytes, in order. The item is a local
            // of the function, and a record item is changed, as any record is,
            // by writing it again.
            AlgorithmStmt::Foreach {
                item,
                source,
                key,
                body,
                ..
            } => {
                check_name(self.algorithm, item)?;
                let source = source.trim();
                if self.imports.is_bounded_collection(source) {
                    // Not a gap to close: a `sce-static` statechart has no
                    // variable of this kind, so nothing could hand the
                    // function one to loop over; the table a statechart holds
                    // is a map, walked below (docs/adr/0108, D9).
                    return Err(refusal(
                        self.algorithm,
                        format!("<sce:foreach item=\"{item}\" in=\"{source}\">, a loop over a bounded-collection import, which a sce-static statechart cannot hold"),
                    ));
                }
                // A map or a set is walked by its entries in the order their
                // keys were first put (docs/adr/0108, D5): a map's item is each
                // value and its key each key, a set's item each key. The walk
                // reads the array the map held when the loop began, which a
                // put, a remove or a clear in the body replaces and never
                // changes (D6 item 3), so the array is held under a name of the
                // loop's own and no copy is taken.
                if let Some(held) = self.maps.get(source) {
                    let walked = format!("__w{depth}");
                    let counter = format!("__i{depth}");
                    let entry = format!("{walked}[{counter}]");
                    let mut bound = vec![if held.set {
                        format!("var {item} = {entry};")
                    } else {
                        format!("var {item} = {entry}[1];")
                    }];
                    if let Some(key) = key.as_deref().map(str::trim) {
                        check_name(self.algorithm, key)?;
                        if held.set {
                            return Err(refusal(
                                self.algorithm,
                                format!(
                                    "<sce:foreach in=\"{source}\" key=\"{key}\">, a key of a walk \
                                     over a set, whose item is its key"
                                ),
                            ));
                        }
                        bound.push(format!("var {key} = {entry}[0];"));
                    }
                    return Ok(format!(
                        "var {walked} = {source}; for (var {counter} = 0; {counter} < \
                         {walked}.length; {counter}++) {{ {} {} }}",
                        bound.join(" "),
                        self.statements(body, depth + 1, &loop_entry(proven, body))?
                            .join(" ")
                    ));
                }
                if let Some(key) = key {
                    return Err(refusal(
                        self.algorithm,
                        format!(
                            "<sce:foreach in=\"{source}\" key=\"{}\">, a key of a walk over no \
                             map",
                            key.trim()
                        ),
                    ));
                }
                if !self.iterable.contains(source) {
                    return Err(refusal(
                        self.algorithm,
                        format!("<sce:foreach item=\"{item}\" in=\"{source}\">, which is not a list or bytes parameter or a list buffer"),
                    ));
                }
                let counter = format!("__i{depth}");
                format!(
                    "for (var {counter} = 0; {counter} < {source}.length; {counter}++) \
                     {{ var {item} = {source}[{counter}]; {} }}",
                    self.statements(body, depth + 1, &loop_entry(proven, body))?
                        .join(" ")
                )
            }
            // A map's statements write the map local or the `inout` parameter
            // again, as a buffer's append does: the library answers a new
            // array and never changes the one it is handed (docs/adr/0108, D3,
            // D5 and D10). One that names an outcome writes the map and the
            // outcome from the one answer; a put that names none fails
            // `capacity-exceeded` on a full map, as an append to a full buffer
            // does, and a remove that names none of a key that is absent
            // writes the map as it was.
            AlgorithmStmt::Put {
                target,
                key,
                expr,
                outcome,
                ..
            } => {
                let map = target.trim();
                let element = format!("<sce:put target=\"{map}\">");
                let (place, descriptor, capacity) = self.map_written(&element, map)?;
                let key_place = place.map_key().ok_or_else(|| {
                    refusal(
                        self.algorithm,
                        format!("{element}, whose target has no key"),
                    )
                })?;
                let key = self.map_part(&element, key, &key_place.ty, proven)?;
                let value = match (place.map_value(), expr) {
                    (Some(value), Some(expr)) => {
                        format!(", {}", self.map_part(&element, expr, &value.ty, proven)?)
                    }
                    (None, None) => String::new(),
                    (Some(_), None) | (None, Some(_)) => {
                        return Err(refusal(
                            self.algorithm,
                            format!("{element}, which puts a value into a set or none into a map"),
                        ))
                    }
                };
                match outcome.as_deref().map(str::trim) {
                    Some(outcome) => format!(
                        "{{ var __wrote = {RUNTIME_GLOBAL}.mapPut({map}, {descriptor}, \
                         {capacity}, {key}{value}); {map} = __wrote.map; {outcome} = \
                         __wrote.outcome; }}"
                    ),
                    None => format!(
                        "{map} = {RUNTIME_GLOBAL}.mapPutOrFail({map}, {descriptor}, {capacity}, \
                         {key}{value});"
                    ),
                }
            }
            AlgorithmStmt::Remove {
                target,
                key,
                outcome,
                ..
            } => {
                let map = target.trim();
                let element = format!("<sce:remove target=\"{map}\">");
                let (place, descriptor, _) = self.map_written(&element, map)?;
                let key_place = place.map_key().ok_or_else(|| {
                    refusal(
                        self.algorithm,
                        format!("{element}, whose target has no key"),
                    )
                })?;
                let key = self.map_part(&element, key, &key_place.ty, proven)?;
                let removed = format!("{RUNTIME_GLOBAL}.mapRemove({map}, {descriptor}, {key})");
                match outcome.as_deref().map(str::trim) {
                    Some(outcome) => format!(
                        "{{ var __wrote = {removed}; {map} = __wrote.map; {outcome} = \
                         __wrote.outcome; }}"
                    ),
                    None => format!("{map} = {removed}.map;"),
                }
            }
            AlgorithmStmt::Clear { target, .. } => {
                let map = target.trim();
                self.map_written(&format!("<sce:clear target=\"{map}\">"), map)?;
                format!("{map} = [];")
            }
            // The body goes on only where the condition holds, and fails
            // `precondition` where it does not (SCE_FORGE.md §3.4.1).
            AlgorithmStmt::Require { cond, .. } => format!(
                "{RUNTIME_GLOBAL}.require({}, {});",
                lowered(cond, InferredType::Bool)?,
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
                format!("{};", lowered(&call, InferredType::Unknown)?)
            }
            AlgorithmStmt::Return {
                expr: Some(expr), ..
            } => {
                let name = expr.trim();
                if self.whole_values.contains(name) {
                    format!("return {name};")
                } else {
                    format!("return {};", lowered(expr, self.return_ty)?)
                }
            }
            AlgorithmStmt::Return { expr: None, .. } => "return;".to_string(),
        })
    }
}

/// Whether `ty` is a parameter or a return this lowering can hold.
///
/// A composition no algorithm's slot holds yet
/// ([`AlgorithmValueType::algorithm_view`]: a union inside a list or an
/// optional value among them) is not held here either, and is refused by name
/// where it is met.
fn holds(ty: &AlgorithmValueType) -> bool {
    if ty.algorithm_view().is_err() {
        return false;
    }
    match ty {
        AlgorithmValueType::Scalar(_) | AlgorithmValueType::Text { .. } => {
            ty.scalar().is_some_and(has_a_value_form)
        }
        // A byte string is the array of numbers an algorithm holds it as.
        AlgorithmValueType::Bytes { .. } => true,
        AlgorithmValueType::List { elem, .. } => match &**elem {
            AlgorithmValueType::Scalar(elem) => has_a_value_form(elem),
            // A list of unions is an array of the plain objects of their wire
            // form (docs/adr/0099, "Still open" item 2), as a list of records is
            // an array of theirs.
            AlgorithmValueType::Record { .. } | AlgorithmValueType::Union { .. } => true,
            _ => false,
        },
        AlgorithmValueType::Record { .. } => true,
        // A union is the plain object of its wire form (docs/adr/0092, section
        // 7, the Interpreter row), held whole.
        AlgorithmValueType::Union { .. } => true,
        // A map or a set (docs/adr/0108, D5 and D10) is the array of its
        // entries, `[key, value]` of a map and the key of a set, a parameter
        // written in place lent in a holder of it ([`lower`]). A key or a value
        // this lowering does not spell is refused by name where a statement
        // writes one.
        AlgorithmValueType::Map { .. } | AlgorithmValueType::Set { .. } => true,
        // A value that may be absent is `null` or its present value
        // (docs/adr/0055): a number, a `bool`, a text, a value of an enum (its
        // declared name) or a record (a plain object). A byte string is the array
        // of numbers an algorithm holds it as, which a variable of a statechart
        // is not, and has no form here.
        AlgorithmValueType::Optional { elem } => holds_optional(elem),
    }
}

/// Whether this lowering holds an optional value of `elem`
/// ([`holds`]).
fn holds_optional(elem: &AlgorithmValueType) -> bool {
    match elem {
        AlgorithmValueType::Scalar(_)
        | AlgorithmValueType::Text { .. }
        | AlgorithmValueType::Bytes { .. } => elem.scalar().is_some_and(has_a_value_form),
        AlgorithmValueType::Record { .. } => true,
        AlgorithmValueType::List { .. }
        | AlgorithmValueType::Map { .. }
        | AlgorithmValueType::Set { .. }
        | AlgorithmValueType::Optional { .. }
        | AlgorithmValueType::Union { .. } => false,
    }
}

/// The judges of the body, run before [`lower`] reads it, as every backend's
/// route runs them before the backend renders the body
/// ([`crate::judge_algorithm_body`]; docs/adr/0092, "Decided while landing step
/// 5" item 1): a value that may be absent read where no test proves it present,
/// a field of a union read where no test proves its case, a write of a union, a
/// local built as no case, an end a path reaches without a return, an integer
/// operation the range analysis cannot prove safe in a body that does not
/// declare `may-fail`, a number where an enum value is declared, and an
/// optional record of another schema are each refused with the judge's own
/// diagnostic, in the document. The lowering reads the body as they admitted it.
pub(crate) fn judge(m: &AlgorithmModel, imports: &Imports) -> Result<(), ForgeError> {
    crate::judge_algorithm_body(m, &imports.contexts, &crate::ForgeCompileOptions::default())
}

/// The ECMAScript function `m` lowers to, as an expression —
/// `function (a, b) { … }` — on one line, for the attribute that carries it.
///
/// A constant is a `var` of a function that closes over it, so the table is
/// built once, when the algorithm is installed, and not on every call.
///
/// Refused by name, never passed on half lowered: a parameter or a return of a
/// type with no form here, and every statement it does not spell. The body is
/// one [`judge`] has admitted: the lowering reads a union's fields where the
/// judge proved their cases, and closes its end where the judge proved it
/// unreached.
pub(crate) fn lower(m: &AlgorithmModel, imports: &Imports) -> Result<String, ForgeError> {
    let mut params = Vec::new();
    let mut iterable: HashSet<&str> = HashSet::new();
    let mut whole_values: HashSet<&str> = HashSet::new();
    let mut maps: HashMap<&str, MapHeld> = HashMap::new();
    // Each `inout` map or set parameter, by name, with the holder the caller
    // lends it in (docs/adr/0108, D6 item 2): `{map, capacity}`, the array of
    // its entries and the bound of the variable it is (`JsTarget::in_place_call`).
    let mut lent: Vec<(&str, String)> = Vec::new();
    for (index, param) in m.signature.params.iter().enumerate() {
        check_name(&m.name, &param.name)?;
        if !holds(&param.sce_type) {
            return Err(refusal(
                &m.name,
                format!(
                    "the parameter `{}` of type `{}`",
                    param.name,
                    param.sce_type.as_attr()
                ),
            ));
        }
        let is_bytes = matches!(param.sce_type, AlgorithmValueType::Bytes { .. });
        if param.sce_type.list_elem().is_some() || is_bytes {
            iterable.insert(&param.name);
        }
        // A value that may be absent is an operand, judged where it is
        // returned as any other value is (docs/adr/0055).
        if is_bytes
            || !matches!(
                param.sce_type,
                AlgorithmValueType::Scalar(_)
                    | AlgorithmValueType::Text { .. }
                    | AlgorithmValueType::Optional { .. }
            )
        {
            whole_values.insert(&param.name);
        }
        // A map or a set is the array of its entries (docs/adr/0108, D10). One
        // lent to be read is that array, and one written in place arrives in
        // the holder its caller lends, read into a local of the parameter's
        // name and left there again when the function returns.
        if param.sce_type.map_key().is_some() {
            let set = matches!(param.sce_type, AlgorithmValueType::Set { .. });
            if param.written_in_place() {
                let holder = format!("__lent{index}");
                maps.insert(
                    param.name.as_str(),
                    MapHeld {
                        capacity: Some(format!("{holder}.capacity")),
                        set,
                    },
                );
                params.push(holder.clone());
                lent.push((param.name.as_str(), holder));
                continue;
            }
            maps.insert(
                param.name.as_str(),
                MapHeld {
                    capacity: None,
                    set,
                },
            );
        }
        params.push(param.name.clone());
    }
    // A map or a set local starts empty, and a put into it is held to the bound
    // its type states (docs/adr/0108, D5).
    for binding in m.body_bindings() {
        let AlgorithmBinding::Local { name, sce_type } = binding else {
            continue;
        };
        if sce_type.map_key().is_none() {
            continue;
        }
        let Some(capacity) = sce_type.stated_outer_bound().map(|bound| bound.max) else {
            return Err(refusal(
                &m.name,
                format!(
                    "the local `{name}` of type `{}`, which states no bound",
                    sce_type.as_attr()
                ),
            ));
        };
        check_name(&m.name, name)?;
        maps.insert(
            name,
            MapHeld {
                capacity: Some(capacity.to_string()),
                set: matches!(sce_type, AlgorithmValueType::Set { .. }),
            },
        );
        whole_values.insert(name);
    }
    let mut buffers = HashMap::new();
    collect_buffers(&m.body, &mut buffers);
    whole_values.extend(buffers.keys().copied());
    // A list buffer is walked as a list parameter is (SCE_FORGE.md §4.12, v2):
    // the parser has refused a loop that appends to the buffer it walks, so
    // the array the loop reads is not written while it reads it. A byte
    // buffer is walked by no backend.
    iterable.extend(
        buffers
            .iter()
            .filter(|(_, buffer)| !matches!(buffer.element, BufferElement::Byte))
            .map(|(name, _)| *name),
    );

    let options = crate::ForgeCompileOptions::default();
    let types = AlgorithmTypes::collect(m, &imports.contexts, &options)
        .map_err(|e| GenerateError::unsupported(format!("the algorithm `{}`: {e}", m.name)))?;
    types.refuse_optional(LOWERED_DATAMODEL, &holds_optional)?;
    let mut ctx = types.type_ctx(m, &imports.contexts);
    // An integer here is exact and may be a BigInt; one leaves the function as
    // the value it is, and a statechart that takes it from a call is the one
    // that checks it ([`TypeCtx::exact_integers`]).
    ctx.exact_integers = true;
    let renames = imports.renames();

    let return_ty = match &m.signature.return_type {
        None => InferredType::Unknown,
        // A `bytes` return is a buffer returned by name, like a list.
        Some(AlgorithmValueType::Bytes { .. }) => InferredType::Unknown,
        Some(ty) if !holds(ty) => {
            return Err(refusal(
                &m.name,
                format!("a return of type `{}`", ty.as_attr()),
            ))
        }
        Some(ty @ (AlgorithmValueType::Scalar(_) | AlgorithmValueType::Text { .. })) => ty
            .scalar()
            .map_or(InferredType::Unknown, InferredType::from_sce_type),
        // A list and a union are returned by name, never judged as an operand
        // (SCE_FORGE.md §4.12; docs/adr/0092, "Decided while landing step 5"
        // item 4), and so is a map or a set, the array of a local's entries
        // (docs/adr/0108, D5).
        Some(
            AlgorithmValueType::List { .. }
            | AlgorithmValueType::Union { .. }
            | AlgorithmValueType::Map { .. }
            | AlgorithmValueType::Set { .. },
        ) => InferredType::Unknown,
        // An optional return takes `null`, a value of its element or an optional
        // value (docs/adr/0055).
        Some(AlgorithmValueType::Optional { elem }) => {
            InferredType::Optional(OptionalElem::of_value(elem))
        }
        Some(AlgorithmValueType::Record { alias }) => {
            let Some(record) = imports.record(alias) else {
                return Err(refusal(
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
    // A union parameter or local is returned by name, as a record is.
    whole_values.extend(types.unions.iter().map(|&(name, _)| name));
    let optional_records: HashMap<&str, &str> = types
        .optional_records
        .iter()
        .map(|&(name, alias, _)| (name, alias))
        .collect();

    let body = Body {
        algorithm: &m.name,
        ctx: &ctx,
        renames: &renames,
        return_ty,
        imports,
        records,
        optional_records,
        buffers,
        iterable,
        whole_values,
        unions: types.union_rules(),
        maps,
    };
    let mut statements = body.statements(&m.body, 0, &Facts::none())?;
    // docs/adr/0092, section 6.1 item 4: a body that ends where the tests of a
    // union's tag have returned on every case ends at a point the judge proved
    // no path passes. A function that runs past its last statement answers
    // `undefined`, which a caller would take as the value; the point is closed
    // by a throw that says it is not reached, as every backend closes it.
    if ends_unreached(m) {
        statements.push(format!(
            "throw new Error({});",
            quoted(&unreached_end_reason(&m.name))
        ));
    }
    // docs/adr/0108, D6 item 2: each map written in place is read out of the
    // holder it was lent in and left in it again however the body ends, so the
    // caller, which writes its variable from the holder only once the call has
    // returned, sees every write of the call at once. The body cannot fail
    // (`algorithm/inout-on-may-fail`), and a holder the call threw out of is
    // read by no one.
    let body_text = if lent.is_empty() {
        statements.join(" ")
    } else {
        let taken: Vec<String> = lent
            .iter()
            .map(|(name, holder)| format!("var {name} = {holder}.map;"))
            .collect();
        let left: Vec<String> = lent
            .iter()
            .map(|(name, holder)| format!("{holder}.map = {name};"))
            .collect();
        format!(
            "{} try {{ {} }} finally {{ {} }}",
            taken.join(" "),
            statements.join(" "),
            left.join(" ")
        )
    };
    let function = format!("function ({}) {{ {body_text} }}", params.join(", "));
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

    /// The function `document`, an algorithm document that imports nothing,
    /// lowers to.
    fn lowered_document(document: &str) -> Result<String, String> {
        let parsed =
            crate::forge::parser::parse_forge(document, crate::DocumentLabel::symmetric("probe"))
                .unwrap_or_else(|e| panic!("the probe does not parse: {e:?}"))
                .expect("an algorithm document");
        let ForgeDocument::Algorithm(algorithm) = parsed else {
            panic!("the probe is not an algorithm");
        };
        lower(&algorithm, &no_imports()).map_err(|e| e.to_string())
    }

    /// docs/adr/0108, D5, D6 and D10: a map or a set is the array of its
    /// entries. A parameter lent to be read is the caller's array, read through
    /// the library. One declared `inout` arrives in a holder: it is read out
    /// of it into a local of its name, written again by each statement, and
    /// left in the holder however the function returns. A local starts empty,
    /// and a walk reads the array the map held when the loop began, its value
    /// and its key from each entry, under names of the loop's own.
    #[test]
    fn a_map_is_an_array_lent_whole_and_written_back_through_its_holder() {
        let read = lowered_document(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="probe" version="1.0">
  <sce:signature>
    <sce:param name="m" type="map&lt;uint32, uint32&gt;"/>
    <sce:param name="k" type="uint32"/>
    <sce:return type="bool"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="has(m, k)"/>
  </sce:body>
</scxml>"#,
        )
        .expect("lowers");
        assert_eq!(
            read,
            "function (m, k) { return SceStatic.mapHas(m, {key: 'integer'}, k); }"
        );

        let written = lowered_document(
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="probe" version="1.0">
  <sce:signature>
    <sce:param name="m" type="map&lt;uint32, uint32&gt;" inout="true"/>
    <sce:param name="k" type="uint32"/>
    <sce:return type="uint32"/>
  </sce:signature>
  <sce:body>
    <sce:var name="seen" type="set&lt;uint32&gt;[4]"/>
    <sce:var name="last" type="uint32" init="0"/>
    <sce:remove target="m" key="k"/>
    <sce:foreach in="m" item="v" key="at">
      <sce:assign target="last" expr="at"/>
    </sce:foreach>
    <sce:clear target="seen"/>
    <sce:return expr="last"/>
  </sce:body>
</scxml>"#,
        )
        .expect("lowers");
        for part in [
            "function (__lent0, k) { var m = __lent0.map; try { ",
            "var seen = [];",
            "m = SceStatic.mapRemove(m, {key: 'integer'}, k).map;",
            "var __w0 = m; for (var __i0 = 0; __i0 < __w0.length; __i0++) { var v = \
             __w0[__i0][1]; var at = __w0[__i0][0];",
            "seen = [];",
            " } finally { __lent0.map = m; } }",
        ] {
            assert!(written.contains(part), "`{part}` in {written}");
        }
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

        // A word only ECMAScript reserves: one every backend reserves (`default`
        // is a C++ keyword) is refused by the parser before it reaches here, and
        // this is the guard for the rest.
        let keyword = lowered(
            false,
            r#"<sce:param name="debugger" type="uint8"/>"#,
            r#"<sce:return expr="debugger"/>"#,
        )
        .expect_err("a keyword cannot be a parameter");
        assert!(keyword.contains("`debugger`"), "{keyword}");
    }

    #[test]
    fn a_symbol_holds_only_what_an_identifier_can() {
        assert_eq!(symbol("days_in_month"), "days_in_month");
        assert_eq!(symbol("sync-failure.v2"), "sync_failure_v2");
        assert_eq!(quoted("it's a\\b"), r"'it\'s a\\b'");
        // The one quoter writes a line separator (U+2028) and a control
        // character (U+0001) as `\u` escapes, which this module's own quoter
        // had left raw. The expected text is assembled so that no escape of
        // this file's own source stands for either character.
        let line_separator = char::from_u32(0x2028).expect("a character");
        let expected = ["'a", "\\", "u2028b", "\\", "u0001'"].concat();
        assert_eq!(quoted(&format!("a{line_separator}b\u{1}")), expected);
    }
}
