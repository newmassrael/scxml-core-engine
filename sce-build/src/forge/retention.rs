//! Is a field's initial value read by anything, and one its type can hold?
//!
//! `sce:initial="<value>"` is a field's value before anything has ever
//! written it. Two things ask for it. A store does: `sce:retain="<scope>"`
//! says the value outlives the program, and SCE cannot implement that —
//! where a value is kept between runs belongs to the host. And
//! `previous(<field>)` does, on a transform's first activation
//! ([`crate::forge::previous_value`]). An initial value neither of them
//! reads is refused as an orphan; one that is read leaves the question SCE
//! can answer: **could that initial value ever be this field's value?**
//!
//! ⚠ WHY THIS IS WORTH A PASS OF ITS OWN. The initial value is the one
//! a system takes on the day it is first switched on, and on no other
//! day. It is therefore the value least likely to be exercised by any
//! test, and the one whose mistake survives longest. A misspelled enum
//! variant there does not fail at build time in a language whose
//! generated code carries the string verbatim, and does not fail at run
//! time either until a factory-fresh unit boots.
//!
//! ⚠⚠ WHAT IT DELIBERATELY DOES NOT CHECK: the scope label. That is
//! opaque by design (see [`crate::forge::model::ForgeField::retain`]) — SCE
//! records it and a domain adapter gives it meaning. Checking it here
//! would mean holding a list of legal scopes, which is the automotive
//! vocabulary this surface exists to keep out of a general tool.
//!
//! ⚠⚠⚠ AND IT DOES NOT CHECK THAT THE STORE EXISTS. Whether the host
//! actually provides a `"battery"` store is a deployment fact SCE has no
//! way to see. Saying so is the point: the gap is stated rather than
//! left for a reader to assume the check is wider than it is.

use std::collections::HashSet;
use std::path::Path;

use crate::forge::error::{ForgeError, Located, ValidationError};
use crate::forge::model::{ForgeDocument, ForgeField, ParsedForge, SceType};
use crate::forge::{import_source, previous_value};

/// Every field of a document, whatever kind it is.
///
/// ⚠ Exhaustive over the kinds that HAVE typed fields rather than over
/// `ForgeDocument`, and the `_` arm is why this is honest: a kind with
/// no field list has no retention to check, and adding one to this list
/// is a deliberate act. The alternative — matching every variant — would
/// make the roster look like it had decided eighteen things when it has
/// decided one.
fn fields_of(doc: &ForgeDocument) -> Vec<&ForgeField> {
    match doc {
        ForgeDocument::Transform(m) => m.inputs.iter().chain(m.outputs.iter()).collect(),
        // A procedure has no output FIELDS — it returns through its
        // internals, which is where a retained one would sit anyway.
        ForgeDocument::Procedure(m) => m.inputs.iter().chain(m.internals.iter()).collect(),
        ForgeDocument::EventSchema(m) => m.fields.iter().collect(),
        _ => Vec::new(),
    }
}

/// Refuse an initial value nothing reads, or one the field's type could
/// never hold.
pub fn check(
    parsed: &ParsedForge,
    base_dir: &Path,
    document: &str,
) -> Result<(), Located<ForgeError>> {
    // `None` when a transform's expression cannot be read: whether it reads
    // a field through `previous()` is then unknown, and the expression's own
    // refusal is the one that names the fault. Every other kind reads no
    // field that way.
    let read_previously = match &parsed.document {
        ForgeDocument::Transform(m) => previous_value::fields_read_previously(m),
        _ => Some(HashSet::new()),
    };
    for field in fields_of(&parsed.document) {
        let Some(value) = field.initial.as_deref() else {
            continue;
        };
        let element = format!("field '{}'", field.id);
        // ⚠ On the attribute's own row. This used to carry no location at
        // all, which the wire then answers by searching the whole document
        // for `actual` — and an initial value is usually a `0` or a `false`,
        // the least findable token a document holds.
        let (line, col) = field
            .initial_spelling
            .as_ref()
            .map_or((None, None), |s| (Some(s.row()), Some(s.col())));
        // ⚠ The orphan rule moved here from the parser, unchanged for every
        // field that is not read through `previous()`. An ordinary field is
        // computed afresh every cycle, so an initial value that no store and
        // no `previous()` asks for is read by nothing.
        let unread = field.retain.is_none()
            && read_previously
                .as_ref()
                .is_some_and(|read| !read.contains(&field.id));
        if unread {
            return Err(Located::new(
                ValidationError::AttributeRuleViolated {
                    element,
                    attr: "sce:initial".into(),
                    value: value.to_string(),
                    rule: "an initial value is read by `sce:retain=\"<scope>\"` on the same \
                           element, as the store's first value, or by `previous(<field>)` in a \
                           transform's expression, on its first activation — this field has \
                           neither, and a field that is not retained is computed afresh every \
                           cycle, so nothing reads it"
                        .into(),
                }
                .into(),
                document,
                line,
                col,
            ));
        }
        // A closed set where the type has one — an enum's variants, a
        // bool's two literals — so the record offers them as candidates;
        // an integer's range is a rule no list states.
        let not_one_of = |allowed: Vec<String>| {
            Err(Located::new(
                ValidationError::InvalidAttribute {
                    element: element.clone(),
                    attr: "sce:initial".into(),
                    value: value.to_string(),
                    allowed,
                }
                .into(),
                document,
                line,
                col,
            ))
        };
        match &field.sce_type {
            SceType::Enum(eref) => {
                // An unreadable import is silent here; see
                // `import_source::parse_quietly`.
                let Some(variants) = import_source::enum_variants(parsed, base_dir, &eref.alias)
                else {
                    continue;
                };
                if !variants.iter().any(|v| v == value) {
                    return not_one_of(variants);
                }
            }
            SceType::Bool => {
                if value != "true" && value != "false" {
                    return not_one_of(vec!["true".into(), "false".into()]);
                }
            }
            // A number's range is a rule no list states; the reading is
            // the one every typed numeric attribute shares, so hex and
            // binary initial values are accepted as they are elsewhere.
            //
            // ⚠ Floats are held too. This arm used to cover integers only,
            // beside a comment saying every literal an author can write is
            // one a float can hold — but `sce:initial="warm"` on a float64
            // is not a number, and it reached the emitted code as written.
            ty if ty.is_numeric() => {
                if let Err(rule) = ty.numeric_literal(value) {
                    return Err(Located::new(
                        ValidationError::AttributeRuleViolated {
                            element,
                            attr: "sce:initial".into(),
                            value: value.to_string(),
                            rule,
                        }
                        .into(),
                        document,
                        line,
                        col,
                    ));
                }
            }
            // String and bytes: nothing here to refuse by value — stated
            // rather than silently skipped, and the parser has already
            // rejected an empty `sce:initial`.
            _ => {}
        }
    }
    Ok(())
}
