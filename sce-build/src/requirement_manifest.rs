//! Requirement-closure RFC ① — the requirement manifest and the set
//! comparison against a parsed document.
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
//! ⚠⚠ **Those four are the `shall` column, and saying so is not a
//! caveat — it is the correction a real standard forced.** All four
//! ask one question, *is there a node carrying this id*, and that
//! question is evidence only for a requirement met by something
//! **existing**. A requirement met by something being **absent** —
//! ISO 13400-2:2019 `3.DoIP-131`, "shall not be routed before the
//! connection is Registered [Routing Active]" — has no honest answer
//! among them, and the tool answered anyway: it read `implemented`
//! from an annotation on the one handler that *was* allowed, and went
//! on reading `implemented` after a second, forbidden handler was
//! added elsewhere. See [`Modality`], which is how an entry now says
//! which question may be asked of it, and [`Outcome::NeedsScenario`],
//! which is the answer when none of the four may.
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
//! ## The guard covers every string, not the fields someone listed
//!
//! `deny_unknown_fields` refuses `text` by NAME, and that is what let
//! the hole in: [`ManifestSection::title`], in the struct beside it,
//! was unchecked, so a whole requirement sentence pasted there loaded
//! without a word. A rule written per field is a rule the next field
//! does not inherit — and the next field is the one nobody remembers.
//!
//! So the repair is not a title check. [`prose_reason`] is applied by
//! walking the parsed JSON tree, to **every string a manifest
//! commits**, including strings in fields that do not exist yet.
//!
//! ### What decides it is shape, and the size rule is refuted
//!
//! A length bound was tried first. It cannot work: over
//! ISO 13400-2:2019 the two populations overlap, so every threshold
//! is wrong in both directions, and a guard wrong in both directions
//! is worse than none because it advertises that the field is
//! checked. Do not re-propose it.
//!
//! What was measured instead, on the same document — 243
//! heading-shaped strings (78 contents-page headings, 165 REQ box
//! headings) against the 166 requirement sentences:
//!
//! ```text
//!                                    headings    sentences
//!                                    (accept)    (refuse)
//!   ends with sentence punctuation      0/243      156/166
//!   carries a normative modal           0/243      164/166
//!   either                              0/243      164/166
//! ```
//!
//! Zero false refusals. The two sentences that escape are an artefact
//! of the extraction, not of the rule — both are split across a page
//! boundary, and both carry `shall` and a full stop in the source.
//!
//! ⚠ Measured on one document, a zero is a property of that document.
//! So it was re-measured on two more, and the accepting half holds:
//!
//! ```text
//!   ISO 13400-2:2019   243 headings   0 falsely refused
//!   ISO 13400-1:2011    24 headings   0 falsely refused
//!   ISO 14229-1:2013    62 headings   0 falsely refused
//! ```
//!
//! 329 real contents-page headings across three standards, none of
//! which this guard rejects.
//!
//! ### ⚠ The residue, written down rather than left silent
//!
//! - **A heading that carries a normative modal is refused.** None of
//!   the 243 measured does, but *"Features a server may omit"* is a
//!   legal heading and this guard would reject it. The refusal says so
//!   and names the field, so the author can reword; it is not silent.
//! - **The rule is about Latin script and English modals.** A
//!   specification written in another language passes the modal test
//!   unread. The punctuation half still applies.
//! - **A single token is never prose** ([`prose_reason`] returns early
//!   on it). That is what keeps the format's own vocabulary — a
//!   `modality` of `shall_not`, an id of `3.DoIP-152` — out of the
//!   sweep, and it means a one-word string is never guarded. No
//!   sentence is one word, so nothing is lost.
//! - **`title` still exists.** Dropping it — §5.2b's own rule is
//!   coordinates only — remains the stricter repair, and is still the
//!   right one to weigh when the sidecar is built and the labels have
//!   somewhere else to live.
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

/// What kind of requirement an entry is, which decides **what
/// question coverage is allowed to ask of it**.
///
/// The rule underneath is one line:
///
/// > Presence can be labelled. Absence can only be tested.
///
/// A `shall` is met by a design element being *there*, and a node
/// carrying `sce:req` is evidence of exactly that. A `shall_not` is
/// met by a transition being *absent*, and absence cannot carry an
/// annotation — so no annotation is evidence for it, and the
/// comparison in [`classify`] must not treat one as if it were.
///
/// ⚠⚠ This enum exists because of a measurement, not a prediction.
/// ISO 13400-2:2019 §12.6.1.3 `3.DoIP-131` forbids routing a
/// diagnostic message before the connection reaches
/// "Registered [Routing Active]". The statechart satisfies it by
/// handling `diagnostic_message` only inside `routing_active` — so
/// the natural annotation goes on that handler, and it read
/// `implemented`. Adding a second `diagnostic_message` handler to
/// `initialized` **violates the requirement outright** and the
/// classification still read `implemented`, because the annotated
/// node was still there. Coverage was green across a document that
/// does the opposite of what the standard says.
///
/// ⚠ Only the two modalities the classifier actually routes are
/// spelled here. `shall_within` and `may` are named by the
/// Requirement-closure RFC §5.2c/§5.2d and are deliberately absent:
/// measured over ISO 13400-2:2019 §12.6, that section contains
/// neither — its timing requirements are *triggers* ("if the timeout
/// elapsed, close the socket"), not deadlines on the entity's own
/// response, and it carries no `may` requirement box at all. A
/// variant no manifest can populate and no branch can route is a
/// declared field with no check, which is the shape this repository
/// keeps finding underneath a false green.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Modality {
    /// Met by something existing. Traced through the `sce:req`
    /// annotation, which is what the four outcomes below measure.
    #[default]
    Shall,
    /// Met by something being absent. Traceable only through a
    /// scenario that asserts the thing does not occur — the RFC's
    /// third trace column, which does not exist yet.
    ShallNot,
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
    /// Which question coverage may ask of this entry. Defaults to
    /// [`Modality::Shall`], so a manifest written before this field
    /// existed keeps the meaning it had.
    #[serde(default)]
    pub modality: Modality,
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
    /// A string the manifest commits reads as specification prose
    /// rather than as a coordinate. `field` is the JSON path to it.
    Prose {
        path: String,
        field: String,
        reason: &'static str,
    },
}

/// Words that make a sentence normative. ISO/IEC Directives Part 2 and
/// RFC 2119 both build requirement prose out of these, so their
/// presence is the surest sign a string is a requirement rather than a
/// label.
///
/// ⚠ Measured over ISO 13400-2:2019: `shall` alone carries all 164 of
/// the sentences this catches and the other three appear in none of
/// them. They are kept because the class is "normative modal" and not
/// "the word this one standard happened to use" — and because they
/// cost nothing measurable: across 243 heading-shaped strings from the
/// same document (78 contents-page headings and 165 REQ box headings)
/// not one contains any of the four.
const NORMATIVE_MODALS: [&str; 4] = ["shall", "should", "must", "may"];

/// Whether `value` carries one of [`NORMATIVE_MODALS`] as a whole word.
///
/// Whole-word rather than substring, or `may` would fire on "Maybe"
/// and `must` on "mustard". Byte indexing is safe here: a non-ASCII
/// byte is not `is_ascii_alphanumeric`, so it reads as a boundary,
/// which is the answer a word check wants anyway.
fn carries_normative_modal(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    NORMATIVE_MODALS.iter().any(|modal| {
        lower.match_indices(modal).any(|(start, _)| {
            let end = start + modal.len();
            let before = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
            let after = end == bytes.len() || !bytes[end].is_ascii_alphanumeric();
            before && after
        })
    })
}

/// Why `value` reads as specification prose, or `None` if it reads as
/// a coordinate or a label.
///
/// # The discriminator, and why it is shape and not size
///
/// A length bound was tried first and is **refuted**: over
/// ISO 13400-2:2019 the two populations overlap, so any threshold is
/// wrong in both directions, and a guard wrong in both directions is
/// worse than none because it advertises that the field is checked.
///
/// Measured instead, on the same document:
///
/// ```text
///                                    headings    requirement
///                                    (accept)    sentences (refuse)
///   ends with sentence punctuation      0/243          156/166
///   carries a normative modal           0/243          164/166
///   either of the two                   0/243          164/166
/// ```
///
/// The heading population is 243 strings — every contents-page heading
/// and every REQ box heading in the standard. **Not one** of them
/// trips either test, so the rule has no measured false refusal at
/// all. The two sentences that escape are an artefact of the
/// extraction rather than of the rule: both are split across a page
/// boundary, and both carry `shall` and a full stop in the source, so
/// on the real text the rule refuses all 166.
///
/// Both tests are kept even though the modal subsumes the punctuation
/// one here, because they catch different prose. A descriptive
/// sentence lifted from the same specification — a definition, a note
/// — carries no modal and is still somebody else's copyrighted text.
/// Refuse the first string anywhere under `tree` that reads as prose.
///
/// ⭐ Walks the untyped tree rather than named fields, and that is the
/// whole repair. The hole this closes was not "`title` was forgotten"
/// — it was that the guard keyed on a field NAME (`text`, refused by
/// `deny_unknown_fields`) while the field next to it in the same
/// struct went unchecked. A rule written per field is a rule the next
/// field does not inherit, and the next field is the one nobody
/// remembers. Every string a manifest commits is subject to this,
/// including strings in fields that do not exist yet.
///
/// Object KEYS are deliberately not checked: they are the format's own
/// vocabulary, not content copied from a specification.
fn reject_prose_anywhere(
    tree: &serde_json::Value,
    field: &mut String,
    label: &str,
) -> Result<(), ManifestError> {
    match tree {
        serde_json::Value::String(value) => {
            if let Some(reason) = prose_reason(value) {
                return Err(ManifestError::Prose {
                    path: label.to_string(),
                    field: if field.is_empty() {
                        "<root>".to_string()
                    } else {
                        field.clone()
                    },
                    reason,
                });
            }
        }
        serde_json::Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                let mark = field.len();
                field.push_str(&format!("[{index}]"));
                reject_prose_anywhere(item, field, label)?;
                field.truncate(mark);
            }
        }
        serde_json::Value::Object(members) => {
            for (key, item) in members {
                let mark = field.len();
                if !field.is_empty() {
                    field.push('.');
                }
                field.push_str(key);
                reject_prose_anywhere(item, field, label)?;
                field.truncate(mark);
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn prose_reason(value: &str) -> Option<&'static str> {
    // ⭐ Prose is multi-word. A string with no whitespace is a token —
    // an id, a section number, a revision, or one of the format's own
    // closed vocabulary words — and no amount of it is a sentence.
    //
    // This precondition is not a softening; it is what makes the rule
    // correct. Without it the sweep refused this repository's own ISO
    // manifest, because `modality` serialises as `"shall_not"` and the
    // whole-word modal test fired on the `shall` in it. The fix that
    // suggests itself — exempt the `modality` field — would have been
    // the per-field defect this guard exists to remove, written the
    // other way round. Asking "does this have words at all" separates
    // the format's vocabulary from the author's prose without naming
    // either.
    //
    // Measured on ISO 13400-2:2019: **0 of 166** requirement sentences
    // lack whitespace, so the precondition gives up no refusal power;
    // 4 of 242 headings are single words, and those are strings the
    // rule wants to accept anyway.
    if !value.chars().any(char::is_whitespace) {
        return None;
    }
    let trimmed = value.trim_end();
    if trimmed.ends_with('.') || trimmed.ends_with('!') || trimmed.ends_with('?') {
        return Some("it ends in sentence punctuation");
    }
    if carries_normative_modal(value) {
        return Some("it carries a normative modal (shall / should / must / may)");
    }
    None
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
            ManifestError::Prose {
                path,
                field,
                reason,
            } => write!(
                f,
                "requirement manifest {path}: `{field}` reads as \
                 specification prose, because {reason}. A manifest is \
                 checked in and specification text is usually somebody \
                 else's copyright, so the committed side carries \
                 COORDINATES only — an id, a section, a page, or the \
                 heading a contents page prints. The sentence belongs \
                 in the uncommitted sidecar. If this is a real heading \
                 the check has misread, shorten it to the wording the \
                 contents page uses"
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
        // Parsed twice on purpose: once into the typed shape, whose
        // `deny_unknown_fields` is what refuses a `text` field with the
        // message that names the sidecar, and once as an untyped tree
        // so the prose sweep below can reach EVERY string without being
        // told where the strings are. Field-by-field checking is what
        // produced this hole — `text` was guarded by name while
        // `title`, sitting in the struct beside it, was not.
        let tree: serde_json::Value =
            serde_json::from_str(raw).map_err(|source| ManifestError::Parse {
                path: label.to_string(),
                source,
            })?;
        let manifest: RequirementManifest =
            serde_json::from_value(tree.clone()).map_err(|source| ManifestError::Parse {
                path: label.to_string(),
                source,
            })?;
        reject_prose_anywhere(&tree, &mut String::new(), label)?;
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

/// Which bucket a requirement landed in.
///
/// The first four are the set comparison a `shall` admits. The fifth
/// is not a refinement of them but the admission that they do not
/// apply: for a [`Modality::ShallNot`] entry, *every* one of the four
/// would be a false answer. `implemented` would certify a prohibition
/// from the presence of a node, which is what let a violating document
/// score green; `missing` would report a correctly-implemented
/// prohibition as silently dropped, which is how a risk column fills
/// with false alarms until people learn to ignore it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    Implemented,
    Unresolved,
    Missing,
    Dangling,
    /// The annotation cannot decide this requirement, and nothing
    /// else here can either.
    ///
    /// Not a severity between `implemented` and `missing` — a
    /// different axis. It says the only column that could carry this
    /// requirement is the third one (a scenario asserting the thing
    /// does not occur, and passing), and that column does not exist
    /// yet. When it does, a `shall_not` with a passing scenario
    /// becomes `implemented` and this bucket empties from the top.
    NeedsScenario,
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Implemented => "implemented",
            Outcome::Unresolved => "unresolved",
            Outcome::Missing => "missing",
            Outcome::Dangling => "dangling",
            Outcome::NeedsScenario => "needs-scenario",
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

/// The comparison of a document against a manifest, one
/// [`RequirementOutcome`] per requirement.
///
/// ⚠ Deliberately not "the four-way comparison", which is what this
/// line said until [`Outcome::NeedsScenario`] landed and made it
/// false. [`Outcome`] is the list; a doc comment that repeats its
/// length has to be corrected every time a variant lands, and the one
/// time it is not, it misinforms silently.
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
    // Walked once, read three times below — the citations, the
    // dangling ids, and the revision check all answer from this.
    let nodes = crate::requirements_report::annotated_nodes(model);

    // id -> node_paths citing it, and whether every citing node is
    // marked unresolved.
    let mut cited: BTreeMap<&str, (Vec<String>, bool)> = BTreeMap::new();
    for node in &nodes {
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
        let citation = cited.get(entry.id.as_str());
        // Carried for EVERY modality, including the one whose verdict
        // does not depend on it. For a `shall_not` the citation is not
        // evidence, but it is still the author saying where they
        // believe the prohibition is enforced — which is the first
        // place a reviewer, or the scenario that finally settles it,
        // has to look. Dropping it would make the honest verdict less
        // actionable than the false one it replaced.
        let node_paths = citation.map_or_else(Vec::new, |(paths, _)| paths.clone());
        let outcome = match entry.modality {
            // Presence can be labelled: the annotation is the evidence,
            // and its absence is the finding.
            Modality::Shall => match citation {
                None => Outcome::Missing,
                Some((_, all_unresolved)) => {
                    if *all_unresolved {
                        Outcome::Unresolved
                    } else {
                        Outcome::Implemented
                    }
                }
            },
            // Absence can only be tested. Nothing about which nodes
            // carry this id can settle a prohibition — a document that
            // keeps the annotated node AND adds the forbidden
            // behaviour beside it is more annotated, not more
            // compliant. So the verdict deliberately does not read the
            // citation, and is the same whether or not one exists.
            Modality::ShallNot => Outcome::NeedsScenario,
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

    // Over the SAME traversal as everything else above, and not a
    // hand-rolled walk of states and transitions. The first version of
    // this was exactly that, and it could not see an anchor on an
    // `<onentry>` action or an `<invoke>` — so a document whose only
    // `sce:provenance` sat on an action would be compared against a
    // stale manifest in silence. The staleness check is worth more than
    // the comparison it qualifies, which makes a partial walk here the
    // worst place in the file to have one.
    let revision_note = nodes
        .iter()
        .flat_map(|node| node.record.spec_provenance.iter())
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
