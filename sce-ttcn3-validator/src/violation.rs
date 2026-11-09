// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the validator finds wrong with an export, each finding with a stable
//! code ([`Violation::code`]) and written on one line.

use std::fmt;

use crate::place::{path_text, Segment};
use crate::rules::Rule;
use crate::witness::Construct;

/// One finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    /// A module holds what is not the TTCN-3 the export writes, or is not
    /// the module the export of the suite names.
    Parse {
        /// The module, by the name the export gives it.
        module: String,
        line: u32,
        message: String,
    },
    /// A module holds one place twice.
    PlaceTwice { module: String, path: Vec<Segment> },
    /// A place of a module that no location of the witness names.
    PlaceUnnamed { module: String, path: Vec<Segment> },
    /// A place of a module that more than one location of the witness names.
    PlaceNamedTwice {
        module: String,
        path: Vec<Segment>,
        times: usize,
    },
    /// A location of the witness, in the entry of `construct` and `rule`,
    /// that names no place of the modules.
    LocationDangling {
        module: String,
        path: Vec<Segment>,
        construct: Construct,
        rule: Rule,
    },
    /// A construct of the documents that `rule` translates and that has no
    /// entry of it.
    ConstructMissing { construct: Construct, rule: Rule },
    /// A construct and a rule with more than one entry.
    EntryDuplicate {
        construct: Construct,
        rule: Rule,
        times: usize,
    },
    /// An entry whose construct the documents do not hold.
    ConstructDangling { construct: Construct, rule: Rule },
}

impl Violation {
    /// The violation's code, which names its kind and does not change.
    pub fn code(&self) -> &'static str {
        match self {
            Violation::Parse { .. } => "parse",
            Violation::PlaceTwice { .. } => "place-twice",
            Violation::PlaceUnnamed { .. } => "place-unnamed",
            Violation::PlaceNamedTwice { .. } => "place-named-twice",
            Violation::LocationDangling { .. } => "location-dangling",
            Violation::ConstructMissing { .. } => "construct-missing",
            Violation::EntryDuplicate { .. } => "entry-duplicate",
            Violation::ConstructDangling { .. } => "construct-dangling",
        }
    }
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = self.code();
        match self {
            Violation::Parse {
                module,
                line,
                message,
            } => write!(f, "{code}: {module}.ttcn, row {line}: {message}"),
            Violation::PlaceTwice { module, path } => write!(
                f,
                "{code}: {module} {}: the module holds the place twice",
                path_text(path)
            ),
            Violation::PlaceUnnamed { module, path } => write!(
                f,
                "{code}: {module} {}: no location of the witness names it",
                path_text(path)
            ),
            Violation::PlaceNamedTwice {
                module,
                path,
                times,
            } => write!(
                f,
                "{code}: {module} {}: {times} locations of the witness name it",
                path_text(path)
            ),
            Violation::LocationDangling {
                module,
                path,
                construct,
                rule,
            } => write!(
                f,
                "{code}: {module} {}: the entry of {construct} by {rule} names it, and the module \
                 holds no such place",
                path_text(path)
            ),
            Violation::ConstructMissing { construct, rule } => {
                write!(
                    f,
                    "{code}: {construct}: {rule} translates it, and no entry says so"
                )
            }
            Violation::EntryDuplicate {
                construct,
                rule,
                times,
            } => write!(f, "{code}: {construct}: {times} entries of {rule}"),
            Violation::ConstructDangling { construct, rule } => write!(
                f,
                "{code}: {construct}: an entry of {rule} names it, and the documents hold no such \
                 construct"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_violation_is_one_line_led_by_its_code() {
        let construct = Construct {
            document: "c.scxml".to_string(),
            node: "state 'a'".to_string(),
            line: Some(3),
        };
        let violations = [
            Violation::PlaceUnnamed {
                module: "s".to_string(),
                path: vec![Segment::Function("f".to_string()), Segment::Statement(2)],
            },
            Violation::ConstructMissing {
                construct: construct.clone(),
                rule: Rule::E7,
            },
            Violation::EntryDuplicate {
                construct,
                rule: Rule::E6,
                times: 2,
            },
        ];
        let lines: Vec<String> = violations.iter().map(ToString::to_string).collect();
        assert_eq!(
            lines,
            [
                r#"place-unnamed: s [{"function":"f"},{"statement":2}]: no location of the witness names it"#,
                "construct-missing: c.scxml state 'a' (row 3): E7 translates it, and no entry says so",
                "entry-duplicate: c.scxml state 'a' (row 3): 2 entries of E6",
            ]
        );
        assert!(lines.iter().all(|line| !line.contains('\n')));
    }
}
