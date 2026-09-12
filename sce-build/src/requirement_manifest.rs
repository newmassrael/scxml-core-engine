//! Requirement-closure RFC ① — the requirement manifest and the
//! four-way comparison against a parsed document.
//!
//! SCE **consumes** a closed requirement set and never derives one.
//! Deriving it would mean reading the source specification, which is
//! the authoring pass's job and outside SCE's mission (RFC §5.1). What
//! this module adds is the comparison that a set makes possible:
//!
//! | outcome | meaning |
//! |---|---|
//! | `implemented` | the manifest id is on a node, and that node is not marked unresolved |
//! | `unresolved` | the manifest id is on a node carrying `<sce:unresolved>` — an honest "I did not know" |
//! | `missing` | the manifest id is on no node at all — silently dropped |
//! | `dangling` | the document cites an id the manifest does not contain — invented, or a stale revision |
//!
//! `missing` is the outcome that does not exist without a manifest,
//! and is the reason this module is worth building: a document can
//! only be measured against a denominator it did not write.
//!
//! ⚠ There is a fifth case no id comparison can see — design elements
//! carrying no `sce:req` at all, which is behaviour the specification
//! never asked for. That one needs the transition table (RFC §6.2),
//! not this file, and is deliberately absent here rather than
//! half-answered.
//!
//! # ⭐ Why the manifest holds no requirement text
//!
//! RFC §5.2 originally put the requirement sentence in the manifest,
//! `text` beside `id`, so review artefacts could print it next to the
//! design element claiming to implement it. That is right about what a
//! reviewer needs and wrong about where it can live.
//!
//! A requirement set is extracted from a specification, and a
//! specification is usually **someone else's copyrighted document**.
//! ISO 13400-2, the first real standard this is aimed at, may be read
//! but its sentences may not be redistributed — and a manifest is a
//! checked-in file in a public repository. A format with a `text`
//! field invites exactly one mistake, makes it silently, and makes it
//! permanent in the history.
//!
//! So the split is structural rather than advisory:
//!
//! ```text
//!   manifest   id, section, page — COORDINATES ONLY.  committed.
//!   sidecar    id -> verbatim sentence.               never committed.
//! ```
//!
//! [`RequirementEntry`] denies unknown fields, so a manifest carrying
//! `text` does not quietly work — it fails to load, for everyone,
//! with a message saying where the sentence belongs. That is the
//! difference between a rule and a guard: nobody has to remember it.
//!
//! The cost this accepts, stated rather than hidden: the acceptance
//! report (RFC §7a) cannot print the sentence from the manifest alone
//! and must read the sidecar beside it. The sidecar is not implemented
//! here, because Atomic A has no consumer for it; what Atomic A owes
//! it is a format that leaves room for it, which is this one.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::model::SCXMLModel;

/// One section of the source document's table of contents.
///
/// Carried because it is the only handle on what was **left out**
/// (RFC §5.2a). A requirement list cannot show that a 51st requirement
/// existed and was dropped; a contents page with a per-section count
/// turns "read sixty pages" into "open the sections that yielded
/// nothing", which is a review a person will actually do.
///
/// ⚠ A pointer, not a proof: a section yielding six requirements may
/// still have had nine.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ManifestSection {
    pub id: String,
    #[serde(default)]
    pub title: String,
}

/// One requirement, by coordinate.
///
/// ⚠ `deny_unknown_fields` is load-bearing, not tidiness. It is what
/// makes the module-level split structural: a manifest carrying `text`
/// is refused rather than accepted-and-committed. See the module doc.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RequirementEntry {
    /// The id as the source document spells it. Compared verbatim
    /// against `sce:req` — SCE never normalises an id, because two
    /// spellings that normalise together are two requirements to the
    /// document that issued them.
    pub id: String,
    /// Section of the source document. Matches `sections[].id` so the
    /// per-section counts of RFC §5.2a can be computed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    /// Page in the source document, for a reviewer to open.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
}

/// A closed requirement set for one `(doc_id, rev)`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RequirementManifest {
    /// Must match the `doc-id` a document's `sce:provenance` cites.
    pub doc_id: String,
    /// Revision. A document citing `@D3` checked against a `D4`
    /// manifest is stale — see [`Classification::revision_note`].
    pub rev: String,
    #[serde(default)]
    pub sections: Vec<ManifestSection>,
    pub requirements: Vec<RequirementEntry>,
}

/// Why a manifest could not be used.
#[derive(Debug)]
pub enum ManifestError {
    Read {
        path: String,
        source: std::io::Error,
    },
    Parse {
        path: String,
        source: serde_json::Error,
    },
    DuplicateId {
        path: String,
        id: String,
    },
    Empty {
        path: String,
    },
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManifestError::Read { path, source } => {
                write!(f, "cannot read requirement manifest {path}: {source}")
            }
            ManifestError::Parse { path, source } => write!(
                f,
                "requirement manifest {path} is not valid: {source}. \
                 Note that a requirement entry carries COORDINATES only \
                 (`id`, `section`, `page`); the requirement sentence \
                 lives in an uncommitted sidecar, because a manifest is \
                 checked in and specification text is usually somebody \
                 else's copyright"
            ),
            ManifestError::DuplicateId { path, id } => write!(
                f,
                "requirement manifest {path} lists `{id}` more than \
                 once. A denominator that counts one requirement twice \
                 is not a set, and every ratio taken against it is wrong"
            ),
            ManifestError::Empty { path } => write!(
                f,
                "requirement manifest {path} lists no requirements. \
                 Every comparison against it would be vacuously clean, \
                 which reads exactly like a document that implements \
                 everything"
            ),
        }
    }
}

impl std::error::Error for ManifestError {}

impl RequirementManifest {
    /// Load and validate a manifest.
    ///
    /// Refuses an empty requirement list and duplicate ids. Both would
    /// still "work" — the first reports everything clean, the second
    /// double-counts — and both are the shape this repository already
    /// knows reads as success while measuring nothing.
    pub fn load(path: &Path) -> Result<Self, ManifestError> {
        let display = path.display().to_string();
        let raw = std::fs::read_to_string(path).map_err(|source| ManifestError::Read {
            path: display.clone(),
            source,
        })?;
        Self::from_json(&raw, &display)
    }

    /// The parsing and validation half, without the filesystem.
    ///
    /// Split from [`Self::load`] so the refusals below are reachable
    /// from a test without writing a file for each one. They are the
    /// part worth testing: each is a manifest that would otherwise
    /// "work" while measuring nothing.
    pub fn from_json(raw: &str, label: &str) -> Result<Self, ManifestError> {
        let manifest: RequirementManifest =
            serde_json::from_str(raw).map_err(|source| ManifestError::Parse {
                path: label.to_string(),
                source,
            })?;
        if manifest.requirements.is_empty() {
            return Err(ManifestError::Empty {
                path: label.to_string(),
            });
        }
        let mut seen = BTreeSet::new();
        for entry in &manifest.requirements {
            if !seen.insert(entry.id.as_str()) {
                return Err(ManifestError::DuplicateId {
                    path: label.to_string(),
                    id: entry.id.clone(),
                });
            }
        }
        Ok(manifest)
    }
}

/// Which of the four buckets a requirement landed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    Implemented,
    Unresolved,
    Missing,
    Dangling,
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Implemented => "implemented",
            Outcome::Unresolved => "unresolved",
            Outcome::Missing => "missing",
            Outcome::Dangling => "dangling",
        }
    }
}

/// One requirement's verdict, with the nodes that cite it.
#[derive(Debug, Clone, Serialize)]
pub struct RequirementOutcome {
    pub id: String,
    pub outcome: Outcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    /// `node_path`s from the requirement report, so a reader can find
    /// the nodes in the very report this classification walked. Empty
    /// for `missing` by definition.
    pub node_paths: Vec<String>,
}

/// The four-way comparison of a document against a manifest.
#[derive(Debug, Clone)]
pub struct Classification {
    pub outcomes: Vec<RequirementOutcome>,
    /// Per-section requirement counts, sections with none included —
    /// RFC §5.2a's only handle on omission.
    pub section_counts: Vec<(String, usize)>,
    /// Set when the document cites the manifest's `doc_id` at a
    /// different revision. A stale denominator is worth more than a
    /// clean comparison against it.
    pub revision_note: Option<String>,
}

impl Classification {
    pub fn count(&self, outcome: Outcome) -> usize {
        self.outcomes
            .iter()
            .filter(|o| o.outcome == outcome)
            .count()
    }
}

/// The classification as NDJSON, one record per requirement.
///
/// NDJSON because that is what this subcommand already speaks, so a
/// consumer that reads the annotation report line by line reads this
/// the same way. The per-section counts and the revision note ride on
/// their own record kinds rather than repeating on every line.
pub fn emit_classification_ndjson<W: std::io::Write + ?Sized>(
    classification: &Classification,
    writer: &mut W,
) -> std::io::Result<()> {
    #[derive(Serialize)]
    #[serde(tag = "kind", rename_all = "kebab-case")]
    enum Line<'a> {
        Requirement(&'a RequirementOutcome),
        SectionCoverage {
            section: &'a str,
            requirements: usize,
        },
        RevisionMismatch {
            detail: &'a str,
        },
    }

    for outcome in &classification.outcomes {
        writeln!(
            writer,
            "{}",
            serde_json::to_string(&Line::Requirement(outcome))
                .expect("RequirementOutcome serialises; all fields are owned primitives")
        )?;
    }
    for (section, requirements) in &classification.section_counts {
        writeln!(
            writer,
            "{}",
            serde_json::to_string(&Line::SectionCoverage {
                section,
                requirements: *requirements,
            })
            .expect("section coverage serialises")
        )?;
    }
    if let Some(detail) = &classification.revision_note {
        writeln!(
            writer,
            "{}",
            serde_json::to_string(&Line::RevisionMismatch { detail })
                .expect("revision note serialises")
        )?;
    }
    Ok(())
}

/// Compare `model` against `manifest`.
///
/// Walks [`crate::requirements_report::annotated_nodes`] rather than
/// the IR directly — the same traversal the `requirements` report
/// prints. That is deliberate: the classification names nodes by
/// `node_path`, and those paths are only useful if they are the ones a
/// reader finds in the report. A private second walk would be free to
/// drift.
pub fn classify(model: &SCXMLModel, manifest: &RequirementManifest) -> Classification {
    // id -> node_paths citing it, and whether every citing node is
    // marked unresolved.
    let mut cited: BTreeMap<&str, (Vec<String>, bool)> = BTreeMap::new();
    for node in crate::requirements_report::annotated_nodes(model) {
        for id in &node.record.requirement_ids {
            let entry = cited.entry(id).or_insert_with(|| (Vec::new(), true));
            entry.0.push(node.record.node_path.clone());
            // ⚠ ALL, not ANY. A requirement cited once on an
            // unresolved node and once on a settled one has been
            // answered somewhere, and reporting it as an open question
            // would send a reviewer to look for a decision already
            // made. `unresolved` is for the requirement nothing
            // settled.
            entry.1 = entry.1 && node.unresolved;
        }
    }

    let declared: BTreeSet<&str> = manifest
        .requirements
        .iter()
        .map(|entry| entry.id.as_str())
        .collect();

    let mut outcomes = Vec::new();
    for entry in &manifest.requirements {
        let (outcome, node_paths) = match cited.get(entry.id.as_str()) {
            None => (Outcome::Missing, Vec::new()),
            Some((paths, all_unresolved)) => (
                if *all_unresolved {
                    Outcome::Unresolved
                } else {
                    Outcome::Implemented
                },
                paths.clone(),
            ),
        };
        outcomes.push(RequirementOutcome {
            id: entry.id.clone(),
            outcome,
            section: entry.section.clone(),
            page: entry.page,
            node_paths,
        });
    }

    for (id, (paths, _)) in &cited {
        if !declared.contains(id) {
            outcomes.push(RequirementOutcome {
                id: (*id).to_string(),
                outcome: Outcome::Dangling,
                section: None,
                page: None,
                node_paths: paths.clone(),
            });
        }
    }

    // Sections in the manifest's own order, so the report reads like
    // the document's contents page rather than like a hash map.
    let mut section_counts: Vec<(String, usize)> = manifest
        .sections
        .iter()
        .map(|section| {
            let count = manifest
                .requirements
                .iter()
                .filter(|entry| entry.section.as_deref() == Some(section.id.as_str()))
                .count();
            (section.id.clone(), count)
        })
        .collect();
    section_counts.sort_by(|a, b| a.0.cmp(&b.0));

    let revision_note = model
        .states
        .values()
        .flat_map(|state| state.provenance.iter())
        .chain(
            model
                .states
                .values()
                .flat_map(|state| state.transitions.iter())
                .flat_map(|transition| transition.provenance.iter()),
        )
        .find(|anchor| {
            anchor.doc_id == manifest.doc_id && anchor.rev.as_deref() != Some(&manifest.rev)
        })
        .map(|anchor| {
            format!(
                "document cites {}@{} but the manifest is {}@{}",
                anchor.doc_id,
                anchor.rev.as_deref().unwrap_or("(no rev)"),
                manifest.doc_id,
                manifest.rev,
            )
        });

    Classification {
        outcomes,
        section_counts,
        revision_note,
    }
}
