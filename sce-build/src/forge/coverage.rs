//! Which values of an input's declared value space does a document
//! never mention?
//!
//! ⚠ WHY THIS IS NOT "the expression is partial". A conditional chain
//! ending in an `else` is TOTAL — it produces a value for every input.
//! The question an author actually needs answered is different and
//! sharper:
//!
//! > I declared this input as an enum with N variants. My rules name K
//! > of them. **Which N−K fall through to the default, and did I mean
//! > them to?**
//!
//! That is not a defect the type system can see, because the document
//! is well-formed either way. It is the shape of an UNASKED QUESTION,
//! and reporting it is the only way the author finds out before the
//! product does.
//!
//! ⚠⚠ THE MEASUREMENT THAT PROMPTED THIS. A real conversion had an
//! operating-logic table covering `0..=221` and `225..=254`, with
//! nothing said about the three values between. The conversion's author
//! (this tree's own NL→IR survey) found that gap BY HAND, by reading a
//! table and noticing an arithmetic hole. Every such hole found by hand
//! is one that a different reader would have missed — so the finding
//! belongs in the tool, not in the reader.
//!
//! ⚠⚠⚠ THIS IS A REPORT, NOT A REFUSAL. Falling through to a default is
//! legal, common and often correct. Making it an error would break every
//! document that has ever used an `else`, and would train authors to
//! silence it. It emits the same NDJSON shape as
//! [`crate::unresolved_check`] so one consumer reads both: here is what
//! the author said they do not know, and here is what they may not have
//! noticed they did not say.
//!
//! ⚠ WHAT IT DELIBERATELY DOES NOT DO: it never guesses what the
//! uncovered value should map to. A tool that proposed an answer would
//! turn a question the author must resolve into one they would confirm
//! by reflex.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};
use std::path::Path;

use serde::Serialize;

use crate::forge::error::{ForgeError, Located};
use crate::forge::expr::{parse_to_ast, BinOp, ExprKind, TypedExpr};
use crate::forge::import_source;
use crate::forge::model::{ForgeDocument, ParsedForge, SceType};

/// One input whose declared value space the document does not name in
/// full, with the exact values that reach only a default branch.
#[derive(Debug, Clone, Serialize)]
pub struct Uncovered {
    /// The document that declares the input.
    pub document: String,
    /// The input field's id, as the author wrote it.
    pub input: String,
    /// The enum alias the input is typed with.
    pub value_space: String,
    /// Variants the document's conditions name, in declaration order.
    pub covered: Vec<String>,
    /// Variants NOTHING in the document tests for AND the author has
    /// not acknowledged. These reach the default branch — which may be
    /// exactly right, and may be an unasked question. The author
    /// decides; this tool does not.
    ///
    /// ⚠ Never empty, and never the whole story on its own: the set
    /// sharing the default is this plus [`Uncovered::acknowledged`].
    /// See the two rules in [`report`].
    pub uncovered: Vec<String>,
    /// Variants that reach the default and that the author has already
    /// signed off by name, via `sce:default-covers`. Not a question any
    /// more — carried so a reader can see WHY the uncovered list is
    /// shorter than the set sharing the default, and so the record can
    /// be diffed against the value space without the gap looking like a
    /// bug in the report.
    ///
    /// Always serialised, even empty, for the reason
    /// [`crate::forge::model::LookupEntry::requirements`] gives: a field
    /// that disappears when empty makes every consumer branch on
    /// presence-versus-empty before it can ask its actual question.
    pub acknowledged: Vec<String>,
}

/// Every identifier-to-literal comparison in an expression, as
/// `{identifier: {literal, …}}`.
///
/// ⚠ Only EQUALITY is collected, not ordering. `x >= 225` does bound a
/// region, but turning comparisons into interval arithmetic means
/// deciding what `x > a && x < b` covers when `a` and `b` are
/// themselves expressions — a solver, not a report. Equality against a
/// named variant is the case that carries the author's intent
/// explicitly, and it is the case the enum surface makes checkable
/// without inference. Ranges are a separate feature with a separate
/// name; conflating them would make this one's output unexplainable.
fn compared_values(ast: &TypedExpr, out: &mut BTreeMap<String, BTreeSet<String>>) {
    if let ExprKind::Binary { op, left, right } = &ast.kind {
        if matches!(op, BinOp::StrictEq | BinOp::StrictNeq) {
            if let (ExprKind::Ident(name), Some(lit)) = (&left.kind, literal_of(right)) {
                out.entry(name.clone()).or_default().insert(lit);
            }
            if let (Some(lit), ExprKind::Ident(name)) = (literal_of(left), &right.kind) {
                out.entry(name.clone()).or_default().insert(lit);
            }
        }
    }
    for child in children(ast) {
        compared_values(child, out);
    }
}

/// The literal an expression denotes, if it denotes one. A member
/// access (`Fuel.DSL`) counts: that is how an author names an enum
/// variant, and its property is the variant's own name.
fn literal_of(e: &TypedExpr) -> Option<String> {
    match &e.kind {
        ExprKind::NumberLit(v) => Some(v.clone()),
        ExprKind::StringLit { value, .. } => Some(value.clone()),
        // `Fuel.DSL` — a member access is how an author names a variant,
        // and the property IS the variant's name.
        ExprKind::Member { property, .. } => Some(property.clone()),
        _ => None,
    }
}

fn children(e: &TypedExpr) -> Vec<&TypedExpr> {
    match &e.kind {
        ExprKind::Binary { left, right, .. } => vec![left, right],
        ExprKind::Unary { operand, .. } => vec![operand],
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => vec![condition, consequent, alternate],
        ExprKind::Member { object, .. } => vec![object],
        ExprKind::Index { object, index } => vec![object, index],
        ExprKind::Call { callee, args } => {
            let mut v = vec![&**callee];
            v.extend(args.iter());
            v
        }
        _ => Vec::new(),
    }
}

/// What one enum-typed input's value space looks like against the
/// conditions that test it: which variants the document names, and
/// which reach only the default.
///
/// ⚠ Both [`report`] and [`check`] derive their answer from THIS, so
/// "covered" means the same thing to the question and to the claim that
/// answers it. Two walks would be two definitions, and the acknowledgement
/// would then be able to be true for one and false for the other.
struct ValueSpace {
    alias: String,
    variants: Vec<String>,
    covered: Vec<String>,
    uncovered: Vec<String>,
}

/// Every identifier the document's output expressions compare against a
/// literal, as `{identifier: {literal, …}}`.
fn tested_values(m: &crate::forge::model::TransformModel) -> BTreeMap<String, BTreeSet<String>> {
    let mut seen: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for out in &m.outputs {
        let Some(expr) = out.expr.as_deref() else {
            continue;
        };
        if let Ok(ast) = parse_to_ast(expr.trim()) {
            compared_values(&ast, &mut seen);
        }
    }
    seen
}

/// The value space of an enum-typed input, split by what the document
/// tests. `None` for an input that has no resolvable value space —
/// a non-enum type, or an import this pass cannot read (silent for the
/// reason [`import_source::parse_quietly`] gives).
fn value_space_of(
    parsed: &ParsedForge,
    base_dir: &Path,
    inp: &crate::forge::model::ForgeField,
    tested: &BTreeMap<String, BTreeSet<String>>,
) -> Option<ValueSpace> {
    let SceType::Enum(eref) = &inp.sce_type else {
        return None;
    };
    let variants = import_source::enum_variants(parsed, base_dir, &eref.alias)?;
    let named = tested.get(&inp.id).cloned().unwrap_or_default();
    Some(ValueSpace {
        alias: eref.alias.clone(),
        covered: variants
            .iter()
            .filter(|v| named.contains(*v))
            .cloned()
            .collect(),
        uncovered: variants
            .iter()
            .filter(|v| !named.contains(*v))
            .cloned()
            .collect(),
        variants,
    })
}

/// Refuse a `sce:default-covers` claim the document contradicts.
///
/// ⚠ THIS RUNS ON EVERY BUILD, where [`report`] runs only when asked.
/// The asymmetry is the point: the gap is a question, and a question is
/// raised when someone asks for it; the acknowledgement is an assertion
/// the document makes about itself, and an assertion nothing checks is
/// how the roster's own `sce:codec-id` sat unread for so long. An author
/// who can silence the question with a name that means nothing has a
/// worse tool than one with no way to silence it at all.
pub fn check(
    parsed: &ParsedForge,
    base_dir: &Path,
    document: &str,
) -> Result<(), Located<ForgeError>> {
    let ForgeDocument::Transform(m) = &parsed.document else {
        // Every other kind: the attribute has no meaning there, and a
        // field carrying it is caught by the placement arm below only
        // for transforms. Widening is a separate change — but the
        // roster check already refuses unknown NAMES, so the exposure
        // here is one known name in a kind that ignores it.
        return Ok(());
    };
    let tested = tested_values(m);
    for field in m.inputs.iter().chain(m.outputs.iter()) {
        if field.default_covers.is_empty() {
            continue;
        }
        let err =
            |e: crate::forge::error::ValidationError| Located::new(e.into(), document, None, None);
        // An output has conditions of its own but no value space to
        // acknowledge — nothing reaches ITS default from outside.
        let is_input = m.inputs.iter().any(|i| i.id == field.id);
        let Some(space) = value_space_of(parsed, base_dir, field, &tested).filter(|_| is_input)
        else {
            return Err(err(
                crate::forge::error::ValidationError::DefaultCoversWithoutValueSpace {
                    field: field.id.clone(),
                    found: if is_input {
                        field.sce_type.as_attr()
                    } else {
                        "an output".to_string()
                    },
                },
            ));
        };
        for name in &field.default_covers {
            if !space.variants.iter().any(|v| v == name) {
                return Err(err(
                    crate::forge::error::ValidationError::DefaultCoversUnknownVariant {
                        field: field.id.clone(),
                        value_space: space.alias.clone(),
                        name: name.clone(),
                        known: space.variants.clone(),
                    },
                ));
            }
            if space.covered.iter().any(|v| v == name) {
                return Err(err(
                    crate::forge::error::ValidationError::DefaultCoversTestedVariant {
                        field: field.id.clone(),
                        value_space: space.alias.clone(),
                        name: name.clone(),
                    },
                ));
            }
        }
    }
    Ok(())
}

/// Report every enum-typed input whose variants the document does not
/// name in full. Empty when every declared value space is fully named —
/// which is the state an author can only reach deliberately.
pub fn report(
    parsed: &ParsedForge,
    base_dir: &Path,
    document: &str,
) -> Result<Vec<Uncovered>, Located<ForgeError>> {
    let ForgeDocument::Transform(m) = &parsed.document else {
        // Other kinds have value spaces too; this surface starts where
        // the measurement that prompted it lives. Widening it is a
        // separate change with its own fixtures — not a silent default.
        return Ok(Vec::new());
    };

    let seen = tested_values(m);

    let mut report = Vec::new();
    for inp in &m.inputs {
        let Some(space) = value_space_of(parsed, base_dir, inp, &seen) else {
            continue;
        };
        let alias = &space.alias;
        let covered = space.covered;
        // ⚠⚠ AN ACKNOWLEDGED VARIANT IS NO LONGER A QUESTION. The author
        // has said, by name, that this one reaches the default on
        // purpose. What it does NOT do is leave the value space: a
        // variant added to the enum document tomorrow is unacknowledged,
        // and asking again then is the whole reason the claim names
        // variants instead of being a flag.
        let (acknowledged, uncovered): (Vec<String>, Vec<String>) = space
            .uncovered
            .iter()
            .cloned()
            .partition(|v| inp.default_covers.contains(v));
        if uncovered.is_empty() {
            continue;
        }
        // ⚠⚠ ONE UNNAMED VARIANT IS NOT A QUESTION. If a document names
        // all but one, the default branch belongs to that one
        // unambiguously — the author DID say what happens to it, by
        // saying what happens to everything else. Reporting it would be
        // a false alarm, and a false alarm is not a small cost here: the
        // measurement that prompted this rule found **34** of them in a
        // single corpus, against a handful of real gaps. An author who
        // learns the list contains noise stops reading the list, and
        // then the real gap is invisible too.
        //
        // The question only exists when TWO OR MORE variants share the
        // default, because then nothing in the document distinguishes
        // them from each other.
        //
        // ⚠ This does not weaken the case that prompted the feature: a
        // four-variant enum with one named leaves three sharing.
        //
        // ⚠⚠⚠ THE COUNT IS OF WHAT SHARES THE DEFAULT, NOT OF WHAT IS
        // STILL UNANSWERED — `space.uncovered`, before acknowledgements
        // come out. Ambiguity is a property of the DOCUMENT: if four
        // variants reach the default and the author has signed off three,
        // the fourth still does not own that branch alone, so the premise
        // of this rule ("the author said what happens to it by saying
        // what happens to everything else") is false and the question is
        // real. Applying the rule to the post-acknowledgement count
        // instead made a newly-added variant silent — caught by
        // `an_acknowledgement_does_not_cover_a_variant_added_later`,
        // which is the exact case the whole surface exists for.
        if space.uncovered.len() < 2 {
            continue;
        }
        report.push(Uncovered {
            document: document.to_string(),
            input: inp.id.clone(),
            value_space: alias.clone(),
            covered,
            uncovered,
            acknowledged,
        });
    }
    Ok(report)
}

/// One NDJSON record per uncovered input, in declaration order.
pub fn emit_ndjson<W: Write + ?Sized>(rows: &[Uncovered], writer: &mut W) -> io::Result<()> {
    for row in rows {
        let line = serde_json::to_string(row)
            .expect("Uncovered serialises; every field is an owned String or Vec<String>");
        writeln!(writer, "{line}")?;
    }
    Ok(())
}
