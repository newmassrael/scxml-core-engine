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
use crate::forge::model::{ForgeDocument, ParsedForge};

/// The variants an `enum:<alias>` import declares, or `None` when the
/// import names no readable enum.
///
/// ⚠ The `None` case is NOT silent here, unlike the same read in
/// [`crate::forge::coverage`]. There, a missing import already has its
/// own diagnostic from the import pass and a second voice would
/// double-emit. Here the alias is named by `of=` on the cycle, which the
/// import pass never looks at — so nothing else in the build has an
/// opinion about it, and staying quiet would accept a cycle over a value
/// space that does not exist.
fn variants_of(parsed: &ParsedForge, base_dir: &Path, alias: &str) -> Option<Vec<String>> {
    let imp = parsed.imports.iter().find(|i| i.alias == alias)?;
    let src = base_dir.join(&imp.src);
    let content = std::fs::read_to_string(&src).ok()?;
    let stem = src.file_stem()?.to_str()?;
    let basename = src.file_name()?.to_str()?;
    let label = crate::DocumentLabel {
        identifier: stem,
        diagnostic_label: basename,
    };
    match crate::forge::parser::parse_forge(&content, label).ok()?? {
        ForgeDocument::Enum(e) => Some(e.variants.into_iter().map(|v| v.name).collect()),
        _ => None,
    }
}

/// Refuse a cycle whose value space or stops do not resolve.
pub fn check(
    parsed: &ParsedForge,
    base_dir: &Path,
    document: &str,
) -> Result<(), Located<ForgeError>> {
    for cycle in &parsed.cycles {
        let refuse = |attr: &str, value: String, expected: String| {
            Err(Located::new(
                ValidationError::InvalidAttribute {
                    element: format!("<sce:cycle id=\"{}\">", cycle.id),
                    attr: attr.into(),
                    value,
                    expected,
                }
                .into(),
                document,
                None,
                None,
            ))
        };

        let Some(variants) = variants_of(parsed, base_dir, &cycle.of) else {
            let known: Vec<&str> = parsed.imports.iter().map(|i| i.alias.as_str()).collect();
            return refuse(
                "of",
                cycle.of.clone(),
                if known.is_empty() {
                    "an `<sce:import kind=\"enum\">` alias — this document imports none".to_string()
                } else {
                    format!(
                        "an alias this document imports as an enum: {}",
                        known.join(", ")
                    )
                },
            );
        };

        let declared: BTreeSet<&str> = variants.iter().map(String::as_str).collect();
        for step in &cycle.steps {
            if !declared.contains(step.name.as_str()) {
                return refuse(
                    "step name",
                    step.name.clone(),
                    format!(
                        "one of the values {} declares: {}",
                        cycle.of,
                        variants.join(", ")
                    ),
                );
            }
        }
    }
    Ok(())
}
