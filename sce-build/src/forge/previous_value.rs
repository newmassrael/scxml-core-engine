//! `previous(x)` — what a transform output reads from the activation before
//! this one.
//!
//! A transform is a function of one activation's inputs. A specification very
//! often says something that is not: "when A becomes B" needs the input's
//! value one activation ago, and "keep the previous value while the source
//! reports a sentinel" needs the output's. Until this, the only way to say
//! either was for the BINDING to feed the value back in, which left the
//! document claiming a purity it did not have and left the memory to whoever
//! hosts the generated code. `previous(x)` puts the memory in the document,
//! where the rest of the logic already is.
//!
//! The law, stated once:
//!
//! * `x` names an input or an output of the same document, and nothing else.
//! * `previous(x)` is `x` as it stood at the end of the previous activation,
//!   and on the first activation it is `x`'s `sce:initial` — which is
//!   therefore REQUIRED on any field read this way. No silent first value.
//! * A read through `previous()` is not a dependency: `x = previous(x) + 1`
//!   is legal, where `x = x + 1` is a cycle
//!   ([`crate::forge::transform_dep_check`]).
//!
//! ⚠ WHAT THIS MODULE DOES NOT DO YET. It validates. No backend lowers
//! `previous()` today, so a document that uses it is refused by every
//! backend's codegen with `generate/unsupported-feature`
//! ([`first_read`]) — the same shape `<sce:action>` took while its lowering
//! reached one backend at a time. A document the validator accepts and a
//! generator silently mis-emits is the outcome that refusal exists to rule
//! out.

use std::collections::HashSet;
use std::ops::Range;

use crate::forge::error::{ExprError, ForgeError, Located, ValidationError};
use crate::forge::expr::{expr_children, parse_to_ast, ExprKind, TypedExpr};
use crate::forge::model::{ForgeDocument, ForgeField, ParsedForge, TransformModel};

/// The built-in's name, as an expression spells it.
pub(crate) const PREVIOUS: &str = "previous";

/// What one expression reads.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Reads {
    /// Every bare name read THIS activation, outside any `previous(…)`, in
    /// the order first read and each once. Names that are not fields —
    /// an import alias, an enum's alias — are included; a caller keeps the
    /// ones it has a use for.
    pub now: Vec<String>,
    /// Every `previous(<name>)`, in source order, with the call's span in
    /// the expression as it was handed to [`reads`].
    pub previous: Vec<PreviousRead>,
}

/// One `previous(<name>)`.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PreviousRead {
    pub name: String,
    pub span: Option<Range<usize>>,
}

/// A `previous(…)` that is not a call of it on one bare name — `previous()`,
/// `previous(a, b)`, `previous(1)`, `previous(a + b)`, `previous(a.b)`.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Malformed {
    pub span: Option<Range<usize>>,
}

/// What `expr` reads, or `None` when it does not parse.
///
/// ⚠ `None` is SILENCE, not a verdict. An expression that does not parse is
/// refused by the expression stage with its own diagnostic, and every
/// validation pass that re-parses stays quiet on it so the same text is not
/// reported twice (`quantity_check` follows the same rule).
pub(crate) fn reads(expr: &str) -> Option<Result<Reads, Malformed>> {
    if expr.trim().is_empty() {
        return Some(Ok(Reads::default()));
    }
    let ast = parse_to_ast(expr).ok()?;
    let mut out = Reads::default();
    Some(walk(&ast, &mut out).map(|()| out))
}

fn walk(e: &TypedExpr, out: &mut Reads) -> Result<(), Malformed> {
    match &e.kind {
        ExprKind::Ident(name) => {
            if !out.now.iter().any(|n| n == name) {
                out.now.push(name.clone());
            }
            return Ok(());
        }
        ExprKind::Call { callee, args } => {
            if matches!(&callee.kind, ExprKind::Ident(n) if n == PREVIOUS) {
                return match args.as_slice() {
                    [arg] => match &arg.kind {
                        ExprKind::Ident(name) => {
                            out.previous.push(PreviousRead {
                                name: name.clone(),
                                span: e.span.clone(),
                            });
                            Ok(())
                        }
                        _ => Err(Malformed {
                            span: e.span.clone(),
                        }),
                    },
                    _ => Err(Malformed {
                        span: e.span.clone(),
                    }),
                };
            }
            // A callee is a function's name, never a read of a field that
            // happens to share it.
            for arg in args {
                walk(arg, out)?;
            }
            return Ok(());
        }
        _ => {}
    }
    for child in expr_children(e) {
        walk(child, out)?;
    }
    Ok(())
}

/// Every field some output of `m` reads through `previous()`, or `None`
/// when an output's expression does not parse or misuses `previous()`.
///
/// `None` means "not known", and a caller must treat it so: a rule that
/// needs to know whether a field is read cannot be answered while an
/// expression that might read it is unreadable, and the expression stage or
/// [`check`] is already going to refuse that document for the real reason.
pub(crate) fn fields_read_previously(m: &TransformModel) -> Option<HashSet<String>> {
    let mut found = HashSet::new();
    for out in &m.outputs {
        let read = reads(out.expr.as_deref().unwrap_or(""))?.ok()?;
        found.extend(read.previous.into_iter().map(|p| p.name));
    }
    Some(found)
}

/// The first `previous()` any output of `m` writes — what code generation
/// names when it refuses the document.
pub(crate) struct FirstRead {
    pub output: String,
    pub field: String,
    pub line: Option<u32>,
    pub col: Option<u32>,
}

/// See [`FirstRead`].
pub(crate) fn first_read(m: &TransformModel) -> Option<FirstRead> {
    for out in &m.outputs {
        let Some(Ok(read)) = reads(out.expr.as_deref().unwrap_or("")) else {
            continue;
        };
        if let Some(p) = read.previous.into_iter().next() {
            let (line, col, _) = locate(out, p.span);
            return Some(FirstRead {
                output: out.id.clone(),
                field: p.name,
                line,
                col,
            });
        }
    }
    None
}

/// Refuse a `previous()` the law above does not admit.
pub fn check(parsed: &ParsedForge, label: &str) -> Result<(), Located<ForgeError>> {
    let ForgeDocument::Transform(m) = &parsed.document else {
        return Ok(());
    };
    let fields: Vec<&ForgeField> = m.inputs.iter().chain(m.outputs.iter()).collect();
    for out in &m.outputs {
        let Some(read) = reads(out.expr.as_deref().unwrap_or("")) else {
            continue;
        };
        let read = match read {
            Ok(read) => read,
            Err(Malformed { span }) => {
                let (line, col, written) = locate(out, span.clone());
                let got = written.unwrap_or_else(|| slice(out, span));
                return Err(Located::new(
                    ExprError::ParseMismatch {
                        expected: "exactly one field name in 'previous(…)'".into(),
                        got,
                    }
                    .into(),
                    label,
                    line,
                    col,
                ));
            }
        };
        for p in read.previous {
            let (line, col, _) = locate(out, p.span.clone());
            let Some(field) = fields.iter().find(|f| f.id == p.name) else {
                return Err(Located::new(
                    ExprError::UnknownIdentifier {
                        name: p.name.clone(),
                        candidates: crate::near_miss::near_misses(
                            &p.name,
                            fields.iter().map(|f| f.id.as_str()),
                        ),
                    }
                    .into(),
                    label,
                    line,
                    col,
                ));
            };
            // ⚠ Located at the READ, not at the field. The field is where
            // the attribute goes, and the fix says so; the read is why it
            // is needed, and a refusal pointing at a declaration nobody has
            // looked at since it was written does not say which line made
            // the value necessary.
            if field.initial.is_none() {
                return Err(Located::new(
                    ValidationError::MissingAttribute {
                        element: format!("field '{}'", field.id),
                        attr: "sce:initial".into(),
                    }
                    .into(),
                    label,
                    line,
                    col,
                ));
            }
        }
    }
    Ok(())
}

/// Where `span` of `out`'s expression was written: row, column and the text
/// as the document spells it when that text lies on one row.
fn locate(
    out: &ForgeField,
    span: Option<Range<usize>>,
) -> (Option<u32>, Option<u32>, Option<String>) {
    let written = out
        .expr_spelling
        .as_ref()
        .zip(span)
        .and_then(|(spelling, span)| spelling.locate(span));
    match (written, out.expr_spelling.as_ref()) {
        (Some(w), _) => (Some(w.row), Some(w.col), w.on_one_row().map(str::to_string)),
        (None, Some(spelling)) => (Some(spelling.row()), Some(spelling.col()), None),
        (None, None) => (None, None, None),
    }
}

/// `span` of `out`'s expression as the reader decoded it — what a record
/// carries when the spelling cannot place it.
fn slice(out: &ForgeField, span: Option<Range<usize>>) -> String {
    let expr = out.expr.as_deref().unwrap_or("");
    span.and_then(|s| expr.get(s))
        .unwrap_or(expr)
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(expr: &str) -> Reads {
        reads(expr)
            .expect("the expression parses")
            .expect("previous() is well formed")
    }

    fn names(r: &Reads) -> Vec<&str> {
        r.previous.iter().map(|p| p.name.as_str()).collect()
    }

    #[test]
    fn a_name_inside_previous_is_not_read_now() {
        let r = ok("latched !== 'NULL' ? latched : previous(shown)");
        assert_eq!(r.now, ["latched"]);
        assert_eq!(names(&r), ["shown"]);
    }

    #[test]
    fn a_string_literal_is_not_a_read() {
        // The text search this replaced read `'b'` as a read of `b`.
        let r = ok("x === 'b' ? 1 : 0");
        assert_eq!(r.now, ["x"]);
    }

    #[test]
    fn a_callee_is_not_a_read() {
        let r = ok("floor(raw) + len(name)");
        assert_eq!(r.now, ["raw", "name"]);
    }

    #[test]
    fn every_misuse_is_refused_as_one_shape() {
        for expr in [
            "previous()",
            "previous(a, b)",
            "previous(1)",
            "previous(a + b)",
            "previous(a.b)",
            "1 + previous(previous(a))",
        ] {
            assert!(
                matches!(reads(expr), Some(Err(Malformed { .. }))),
                "{expr} was admitted"
            );
        }
    }

    #[test]
    fn an_unparsable_expression_is_silence() {
        assert_eq!(reads("a +"), None);
    }

    #[test]
    fn the_span_covers_the_call() {
        let expr = "1 + previous(a)";
        let r = ok(expr);
        let span = r.previous[0].span.clone().expect("a span");
        assert_eq!(&expr[span], "previous(a)");
    }
}
