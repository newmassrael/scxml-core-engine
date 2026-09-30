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
//! owns is the statement shapes — `var`, `if`, `while`, `return` — and it
//! refuses, by name, every statement and every signature it does not yet spell:
//! a buffer, a record, a list, an imported callee.
//!
//! A failure is a throw. A `may-fail` algorithm's checked operation throws from
//! the library, a `<sce:require>` throws where its condition does not hold, and
//! the expression of the statechart that called the algorithm fails as it does
//! for an overflow of its own: the statement is skipped, the guard is false, and
//! `error.execution` is raised (W3C SCXML 5.9.1, 3.12.2).

use crate::forge::error::GenerateError;
use crate::forge::expr::{self, ExprTarget};
use crate::forge::generator::AlgorithmTypes;
use crate::forge::model::{AlgorithmModel, AlgorithmStmt, SceType};
use crate::forge::static_js::RUNTIME_GLOBAL;
use crate::forge::types::{InferredType, TypeCtx};
use std::collections::HashMap;

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

/// Whether a scalar type is a value this lowering can hold: a number, a truth
/// value or a string. A `bytes` value is a scalar to the model and has no form
/// here, and an enum is the enum document's type, which the lowered document
/// does not import.
fn has_a_value_form(ty: &SceType) -> bool {
    !matches!(ty, SceType::Bytes | SceType::Enum(_))
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

/// What lowering one body reads and does not change.
struct Body<'a> {
    algorithm: &'a str,
    ctx: &'a TypeCtx<'a>,
    renames: &'a HashMap<&'a str, &'a str>,
    return_ty: InferredType,
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

    fn statements(&self, stmts: &[AlgorithmStmt]) -> Result<Vec<String>, GenerateError> {
        stmts.iter().map(|s| self.statement(s)).collect()
    }

    fn statement(&self, stmt: &AlgorithmStmt) -> Result<String, GenerateError> {
        Ok(match stmt {
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
            AlgorithmStmt::Assign { target, expr, .. } => {
                let target = target.trim();
                if target.contains(['.', '[']) {
                    return Err(refuse(
                        self.algorithm,
                        format!("an assignment to the member `{target}`"),
                    ));
                }
                let slot = expr::infer_expr_type(target, self.ctx).unwrap_or(InferredType::Unknown);
                format!("{target} = {};", self.slot(expr, slot)?)
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
                    self.statements(then_body)?.join(" ")
                );
                if let Some(else_body) = else_body {
                    text.push_str(&format!(
                        " else {{ {} }}",
                        self.statements(else_body)?.join(" ")
                    ));
                }
                text
            }
            AlgorithmStmt::While { cond, body, .. } => format!(
                "while ({}) {{ {} }}",
                self.slot(cond, InferredType::Bool)?,
                self.statements(body)?.join(" ")
            ),
            // The body goes on only where the condition holds, and fails
            // `precondition` where it does not (SCE_FORGE.md §3.4.1).
            AlgorithmStmt::Require { cond, .. } => format!(
                "{RUNTIME_GLOBAL}.require({}, {});",
                self.slot(cond, InferredType::Bool)?,
                quoted(cond.trim())
            ),
            AlgorithmStmt::Return {
                expr: Some(expr), ..
            } => {
                format!("return {};", self.slot(expr, self.return_ty)?)
            }
            AlgorithmStmt::Return { expr: None, .. } => "return;".to_string(),
            AlgorithmStmt::RecordVar { name, .. } | AlgorithmStmt::RecordFromCall { name, .. } => {
                return Err(refuse(self.algorithm, format!("the record local `{name}`")))
            }
            AlgorithmStmt::Append { target, .. } => {
                return Err(refuse(
                    self.algorithm,
                    format!("<sce:append target=\"{target}\">"),
                ))
            }
            AlgorithmStmt::Foreach { item, .. } => {
                return Err(refuse(
                    self.algorithm,
                    format!("<sce:foreach item=\"{item}\">"),
                ))
            }
            AlgorithmStmt::Call { target, .. } => {
                return Err(refuse(
                    self.algorithm,
                    format!("<sce:call target=\"{target}\">"),
                ))
            }
        })
    }
}

/// The ECMAScript function `m` lowers to, as an expression —
/// `function (a, b) { … }` — on one line, for the attribute that carries it.
///
/// Refused by name, never passed on half lowered: a parameter or a return that
/// is not a scalar, a constant, and every statement outside the scalar core.
pub(crate) fn lower(m: &AlgorithmModel) -> Result<String, GenerateError> {
    if let Some(constant) = m.consts.first() {
        return Err(refuse(&m.name, format!("the constant `{}`", constant.name)));
    }
    let mut params = Vec::new();
    for param in &m.signature.params {
        check_name(&m.name, &param.name)?;
        if !param.sce_type.scalar().is_some_and(has_a_value_form) {
            return Err(refuse(
                &m.name,
                format!(
                    "the parameter `{}` of type `{}`",
                    param.name,
                    param.sce_type.as_attr()
                ),
            ));
        }
        params.push(param.name.clone());
    }
    let return_ty = match &m.signature.return_type {
        None => InferredType::Unknown,
        Some(ty) => match ty.scalar().filter(|scalar| has_a_value_form(scalar)) {
            Some(scalar) => InferredType::from_sce_type(scalar),
            None => {
                return Err(refuse(
                    &m.name,
                    format!("a return of type `{}`", ty.as_attr()),
                ))
            }
        },
    };
    let options = crate::ForgeCompileOptions::default();
    let types = AlgorithmTypes::collect(m, &[], &options)
        .map_err(|e| GenerateError::unsupported(format!("the algorithm `{}`: {e}", m.name)))?;
    let ctx = types.type_ctx(m, &[]);
    let renames = HashMap::new();
    let body = Body {
        algorithm: &m.name,
        ctx: &ctx,
        renames: &renames,
        return_ty,
    };
    let statements = body.statements(&m.body)?;
    Ok(format!(
        "function ({}) {{ {} }}",
        params.join(", "),
        statements.join(" ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::model::ForgeDocument;

    /// An algorithm document whose `<sce:body>` is `body`, taking a `uint8`
    /// `kind` and returning a `uint8`.
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
        lower(&algorithm).map_err(|e| e.to_string())
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

    /// A construct with no lowering yet is named, and the algorithm is not
    /// passed on half lowered.
    #[test]
    fn what_it_cannot_spell_yet_is_refused_by_name() {
        let bytes = lowered(
            false,
            r#"<sce:param name="data" type="bytes"/>"#,
            r#"<sce:return expr="0"/>"#,
        )
        .expect_err("a bytes value has no form here");
        assert!(
            bytes.contains("probe") && bytes.contains("the parameter `data` of type `bytes`"),
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
