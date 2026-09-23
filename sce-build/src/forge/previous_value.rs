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
//! This module owns the law and the shape every backend lowers it to: each
//! value read this way is a [`Cell`], which is one more parameter of every
//! output function (`previous_<x>`, [`param_name`]) and one kept value of
//! the transform's holder. The rendering itself is `render_transform`'s.
//!
//! ⚠ The cells no backend lowers — `bytes` anywhere, `string` on C11 — are
//! refused before rendering, at the read [`first_read`] finds, as
//! `generate/unsupported-feature`. A document the validator accepts and
//! a generator silently mis-emits is the outcome that refusal exists to
//! rule out.

use std::collections::HashSet;
use std::ops::Range;

use crate::forge::error::{ExprError, ForgeError, Located, ValidationError};
use crate::forge::expr::{expr_children, parse_to_ast, ExprKind, TypedExpr};
use crate::forge::expression_site::ExpressionSite;
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
///
/// Spans index `expr` TRIMMED, as every entry point of the expression
/// pipeline hands its text to the parser, so one rule places them all
/// ([`ExpressionSite`]).
pub(crate) fn reads(expr: &str) -> Option<Result<Reads, Malformed>> {
    let expr = expr.trim();
    if expr.is_empty() {
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
        ExprKind::Call { callee, args, .. } => {
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

/// A field some output reads through `previous()`, and the parameter that
/// carries its previous value into every output function.
///
/// ⚠ The parameter IS the field, renamed: same type, same quantity, same
/// `sce:initial`, direction `in`. So everything downstream of the lowering —
/// typing, the unit checker, the sibling-call rename, every emitter, the
/// parameter list — treats it as one more input, which is what it is to the
/// function that reads it. Only the holder knows it is a memory.
#[derive(Debug, Clone)]
pub struct Cell {
    /// The field `previous(<of>)` names.
    pub of: String,
    /// The parameter every output function takes for it.
    pub param: ForgeField,
}

/// The parameter a read of `previous(<of>)` lowers to.
pub fn param_name(of: &str) -> String {
    format!("previous_{of}")
}

/// The cells of `m`: inputs first, then outputs, each in declaration order.
///
/// Empty when nothing is read through `previous()` — and when an expression
/// cannot be read, which `check` and the expression stage refuse before any
/// of this is asked.
pub fn cells(m: &TransformModel) -> Vec<Cell> {
    let Some(read) = fields_read_previously(m) else {
        return Vec::new();
    };
    m.inputs
        .iter()
        .chain(m.outputs.iter())
        .filter(|f| read.contains(&f.id))
        .map(|f| {
            let mut param = f.clone();
            param.id = param_name(&f.id);
            param.direction = crate::forge::model::Direction::In;
            param.expr = None;
            param.expr_spelling = None;
            Cell {
                of: f.id.clone(),
                param,
            }
        })
        .collect()
}

/// The first `previous()` read in a transform — what a refusal names and
/// points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstRead {
    /// The output whose expression reads it.
    pub output: String,
    /// The field it reads.
    pub field: String,
    /// Where in the transform's own document, when known.
    pub line: Option<u32>,
    pub col: Option<u32>,
}

/// See [`FirstRead`]. `which` chooses the fields asked about; `|_| true`
/// asks about any.
pub(crate) fn first_read(m: &TransformModel, which: impl Fn(&str) -> bool) -> Option<FirstRead> {
    for out in &m.outputs {
        let Some(Ok(read)) = reads(out.expr.as_deref().unwrap_or("")) else {
            continue;
        };
        if let Some(p) = read.previous.into_iter().find(|p| which(&p.name)) {
            let at = site(out).locate(p.span);
            return Some(FirstRead {
                output: out.id.clone(),
                field: p.name,
                line: at.line,
                col: at.col,
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
                let at = site(out).locate(span.clone());
                let got = at.observed().unwrap_or_else(|| slice(out, span));
                return Err(Located::new(
                    ExprError::ParseMismatch {
                        expected: "exactly one field name in 'previous(…)'".into(),
                        got,
                    }
                    .into(),
                    label,
                    at.line,
                    at.col,
                ));
            }
        };
        for p in read.previous {
            let at = site(out).locate(p.span.clone());
            let (line, col) = (at.line, at.col);
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
            // The read lowers to a parameter with a name of its own, and a
            // field already spelled that way would be bound twice in every
            // output function's signature.
            //
            // ⚠ No row, as `forge::namespace` reports every duplicate: the
            // record's `actual` is the colliding NAME, which is spelled on
            // the other field's element and nowhere near this read — and a
            // record without a row is found by the one place that name is
            // written.
            let param = param_name(&field.id);
            if fields.iter().any(|f| f.id == param) {
                return Err(Located::new(
                    ValidationError::DuplicateId {
                        kind: crate::forge::model::ForgeKind::Transform,
                        what: format!(
                            "name (field, and the parameter `previous({})` lowers to)",
                            field.id
                        ),
                        id: param,
                    }
                    .into(),
                    label,
                    None,
                    None,
                ));
            }
        }
    }
    Ok(())
}

/// `out`'s expression, as a refusal of a piece of it is placed against.
fn site(out: &ForgeField) -> ExpressionSite<'_> {
    ExpressionSite::new(
        out.expr.as_deref().unwrap_or(""),
        out.expr_spelling.as_ref(),
    )
}

/// `span` of `out`'s expression as the reader decoded it — what a record
/// carries when the spelling cannot place it. Trimmed first, as [`reads`]
/// parses it, so the span slices the string it indexes.
fn slice(out: &ForgeField, span: Option<Range<usize>>) -> String {
    let expr = out.expr.as_deref().unwrap_or("").trim();
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
