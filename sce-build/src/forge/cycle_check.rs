//! Does every stop on a `<sce:cycle>` name a value that exists?
//!
//! The cycle declares an ORDER over alternatives drawn from a value
//! space (see [`crate::forge::model::Cycle`] for why the order is the
//! document's to state and not the enum's to imply). What the document
//! cannot check for itself is whether each `<sce:step name="…">` is
//! really one of that space's values — and a step that is not is a stop
//! no cursor can ever land on.
//!
//! ⚠ WHY A MISSPELLED STOP IS WORSE THAN AN ORDINARY TYPO. The cycle's
//! steps are positions, so an unreachable stop does not merely do
//! nothing: it lengthens the sequence. `next` from the stop before it
//! lands on a value that is not in the value space at all, and every
//! position after it shifts by one. The defect shows up as "the wrong
//! mode" three stops away from the line that caused it.
//!
//! ⚠⚠ AND IT CANNOT BE CAUGHT DOWNSTREAM. The name is emitted as a
//! variant reference, so a target compiler catches it only in the
//! languages whose enums are nominal — and the ones that spell a variant
//! as a plain identifier or an integer constant would compile it. A
//! check that fires on some backends is the shape this tree refuses.

use std::collections::BTreeSet;
use std::path::Path;

use crate::forge::error::{ForgeError, Located, ValidationError};
use crate::forge::import_source;
use crate::forge::model::{ForgeKind, ParsedForge};

/// Refuse a cycle whose value space or stops do not resolve.
pub fn check(
    parsed: &ParsedForge,
    base_dir: &Path,
    document: &str,
) -> Result<(), Located<ForgeError>> {
    for cycle in &parsed.cycles {
        let element = format!("<sce:cycle id=\"{}\">", cycle.id);
        let refuse = |error: ValidationError| {
            Err(Located::new(
                error.into(),
                document,
                // ⚠ The cycle's own line, captured at parse time. This was
                // `None`, so the record named a file and nothing else --
                // and a consumer holding `actual` was left searching the
                // whole document for a token that occurs more than once.
                cycle.line,
                None,
            ))
        };

        // ⚠ `None` is NOT silent here, unlike the same read in
        // `coverage` and `retention`. There a missing import already has
        // its own diagnostic from the import pass; here the alias is named
        // by `of=`, which the import pass never looks at — so nothing else
        // in the build has an opinion about it, and staying quiet would
        // accept a cycle over a value space that does not exist.
        let Some(variants) = import_source::enum_variants(parsed, base_dir, &cycle.of) else {
            // Only an enum import can be cycled over, so the enum aliases
            // are the set — not every import, which is what this listed.
            let enum_aliases: Vec<String> = parsed
                .imports
                .iter()
                .filter(|i| i.kind == ForgeKind::Enum)
                .map(|i| i.alias.clone())
                .collect();
            return refuse(if enum_aliases.is_empty() {
                ValidationError::AttributeRuleViolated {
                    element,
                    attr: "of".into(),
                    value: cycle.of.clone(),
                    rule: "an `<sce:import kind=\"enum\">` alias — this document imports none"
                        .into(),
                }
            } else {
                ValidationError::InvalidAttribute {
                    element,
                    attr: "of".into(),
                    value: cycle.of.clone(),
                    allowed: enum_aliases,
                }
            });
        };

        let declared: BTreeSet<&str> = variants.iter().map(String::as_str).collect();
        for step in &cycle.steps {
            if !declared.contains(step.name.as_str()) {
                return refuse(ValidationError::InvalidAttribute {
                    element,
                    attr: "step name".into(),
                    value: step.name.clone(),
                    allowed: variants,
                });
            }
        }
    }
    Ok(())
}
