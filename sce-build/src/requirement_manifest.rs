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
//! ⚠⚠ **Every question above assumes the requirement is satisfied
//! HERE.** A layered standard satisfies most of its requirements in
//! another document, and asking *is there a node carrying this id* of
//! those reports them `missing` — true of this document and useless to
//! a reader. [`Disposition`] is how an entry says where it is
//! satisfied, and each answer owes different evidence:
//!
//! | disposition | refused at load unless it carries | a node citing the id |
//! |---|---|---|
//! | `implemented` | nothing more — the annotation is the evidence, per [`Modality`] | is the evidence |
//! | `delegated` | `to_doc` and `to_id`, naming a document other than this one | [`Outcome::Contradicted`] |
//! | `out_of_scope` | a `reason` of more than one word | [`Outcome::Contradicted`] |
//! | `system_level` | `realised_by`, the deployment artefact | [`Outcome::Contradicted`] |
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
//! And it cannot be reached around. `RequirementManifest` does not
//! implement `Deserialize`; the derive sits on a private wire type, so
//! [`RequirementManifest::from_json`] is the only way to obtain one
//! and the sweep is not a step a caller can skip. That was measured
//! rather than assumed: while the derive was public,
//! `serde_json::from_str` loaded a manifest whose section title was a
//! requirement sentence, refusing nothing that `from_json` refused.
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
use crate::provenance::Position;

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
    /// WHERE this requirement is satisfied. Defaults to
    /// [`Disposition::Implemented`], so a manifest written before this
    /// field existed keeps the meaning it had.
    #[serde(default)]
    pub disposition: Disposition,
    /// Which question coverage may ask of this entry. Defaults to
    /// [`Modality::Shall`], so a manifest written before this field
    /// existed keeps the meaning it had.
    #[serde(default)]
    pub modality: Modality,
    /// Division of the source document. Matches `sections[].id` so the
    /// per-division counts of RFC §5.2a can be computed.
    ///
    /// Source-neutral despite the name: a workbook writes its sheet
    /// here, a structured document its package. See [`Position`] for why
    /// this is not folded into that type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    /// Where inside that division, for a reviewer to open. Source-shaped
    /// and therefore a variant — see [`Position`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<Position>,
}

impl RequirementEntry {
    /// Whether a node carrying this id is the evidence that settles it.
    ///
    /// True only for a requirement satisfied HERE ([`Disposition`]) by
    /// something EXISTING ([`Modality`]). Both halves are needed: a
    /// `shall_not` is met by an absence no annotation can mark, and a
    /// `delegated` requirement is met in a document this one is not.
    ///
    /// ⭐ Published because the presence question is asked in more than
    /// one place — [`classify`] asks it, and so does every reading of
    /// the transition table that names requirements with no row. Those
    /// readings used to filter on modality by hand, which was the whole
    /// rule until [`Disposition`] landed and made it half of one: a
    /// caller that kept filtering on modality alone would report every
    /// delegated requirement as missing from the table while the
    /// classifier called it delegated. One predicate is what keeps the
    /// two readings asking the same question.
    pub fn is_settled_by_annotation(&self) -> bool {
        matches!(self.disposition, Disposition::Implemented {}) && self.modality == Modality::Shall
    }
}

/// Whether the source document names its own requirements.
///
/// This is the field the denominator's standing rests on, and the two
/// values are not a quality scale. `native` means the ids in the
/// manifest are the ones the source document itself writes, so a
/// reviewer can go to the document and check that the list is complete.
/// `synthesized` means whatever performed the extraction decided what
/// counted as a requirement, and there is nothing in the source to
/// audit that decision against.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Ids {
    Native,
    Synthesized,
}

/// Whether the source publishes its own requirement-to-artefact map.
///
/// Some documents ship a tracing chapter that names, per requirement,
/// what satisfies it — including the ones nothing satisfies. Where that
/// exists the document supplies its own `missing` column and SCE's
/// comparison is checking a published claim rather than making one.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Trace {
    Published,
    None,
}

/// Which table of verbal forms decides a sentence's [`Modality`].
///
/// ⭐ A closed set, and that is this type's whole job. RFC §5.2h forbids
/// a manifest field whose values are specification names: `"autosar"`
/// or `"iso"` would make SCE the keeper of a catalogue of standards, and
/// the next source — an OEM document, a spreadsheet, a ticket system —
/// has no entry in it. A closed enum refuses such a value without SCE
/// holding any such list, because a standard's name is simply not a
/// member. Nothing here has to know what standards exist.
///
/// ⚠ The one variant is named for the WORDS, not for the document that
/// codified them. Naming that document would be the same mistake one
/// level down, and it is the mistake
/// `a_standard_named_in_code_is_one_sce_implements` refuses in
/// executable code — a rule the data must not be allowed to route
/// around.
///
/// ⚠⚠ The second variant landed with its table, not before it — see
/// [`normative_markers`], whose entries were counted over a real OEM
/// standard rather than guessed.
///
/// ⚠⚠⚠ A correction worth keeping, because the note that stood here
/// was wrong in a way that would have produced a wrong dictionary. It
/// said the Korean standard "matches **zero** of these verbs". Measured
/// across all 275 pages of `ES95486-02`: `shall` occurs **651** times.
/// The document is MIXED — Korean-normative for its first twenty pages,
/// English-normative from page 21 on — so the earlier figure was true
/// of a slice and read as true of the document. That is why a declared
/// convention adds a vocabulary instead of replacing one.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
pub enum ModalityConvention {
    /// `shall` / `should` / `may` / `must`, read as whole words.
    #[serde(rename = "english-modal-verbs")]
    EnglishModalVerbs,
    /// The Korean auxiliary endings that carry obligation, prohibition,
    /// permission and recommendation — `-어야 한다`, `-지 않아야`,
    /// `-수 있다`, `권장`. Named for the endings themselves, never for a
    /// document that uses them, because
    /// `a_standard_named_in_code_is_one_sce_implements` refuses a
    /// specification's name in executable code and a convention value
    /// is exactly where such a name would try to enter.
    #[serde(rename = "korean-formal-endings")]
    KoreanFormalEndings,
}

/// How the requirement list was produced.
///
/// Recorded because it is what a reviewer needs in order to know how
/// much of the list to re-check, and deliberately NOT part of
/// [`Extraction::denominator_basis`] — see that function for why
/// confidence and auditability are different questions.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
pub enum Method {
    /// Read out of a map the source document publishes.
    #[serde(rename = "derived")]
    Derived,
    /// A first machine pass over the source's prose, unreviewed.
    #[serde(rename = "ai-pass-1")]
    AiPass1,
    /// Typed by a person from the source.
    #[serde(rename = "hand")]
    Hand,
}

/// What is true of the extraction that produced this manifest.
///
/// ⭐ Required, not optional, and that is the point of the field. A
/// coverage figure computed against a standard's own published
/// requirement list and one computed against a list a machine invented
/// from prose are not the same kind of number, and printing both as
/// "94 %" is a lie of omission. A manifest that declines to say which
/// it is cannot be told apart from the trustworthy one, so declining is
/// not allowed: [`RequirementManifest::from_json`] refuses a manifest
/// without this block.
///
/// ⚠ Every field asks what is TRUE of the extraction, never what
/// produced it. There is no `source` field and no `standard` field, by
/// design: `ids: native` + `trace: published` is a property a source SCE
/// has never heard of can satisfy, while a name is a property only a
/// catalogue can interpret.
///
/// ⚠⚠ `reviewed_by` appears in the RFC's sketch of this block and is
/// deliberately absent here. Nothing reads it yet, and a field with no
/// consumer is a field that records whatever the author felt like — it
/// belongs with the acceptance record that will actually ask for it.
///
/// ⚠⚠⚠ The design note this block was sketched from spells the first
/// field two ways — `ids` in its JSON example and `id_scheme` in the
/// paragraph under it. They are one field, and the example's spelling is
/// the one that landed. Written down here because this type is the half
/// of that pair a reader of this repository can open.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Extraction {
    pub ids: Ids,
    pub trace: Trace,
    pub modality_convention: ModalityConvention,
    pub method: Method,
}

/// What a coverage percentage is a percentage OF.
///
/// The distinction the artefact has to carry, because the denominator is
/// the half of a coverage figure that nobody looks at.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DenominatorBasis {
    /// The source document states which requirements exist; the list can
    /// be checked against it.
    Derived,
    /// SCE's own extraction decided what counted; the list cannot be
    /// checked against anything, and a short list reads as high coverage.
    Synthesized,
}

impl Extraction {
    /// Which kind of denominator this manifest's requirement list is.
    ///
    /// Decided by [`Ids`] alone. [`Method`] is deliberately not consulted,
    /// and the reason is worth stating because the opposite looks
    /// reasonable: `method` records how much care went into building the
    /// list, which is a claim about CONFIDENCE, while this function
    /// answers whether the list can be AUDITED at all. A hand-typed list
    /// of invented ids is careful and still unauditable; a machine pass
    /// that collected the document's own tags is unreviewed and still
    /// checkable against the document. Folding the two together would
    /// let care substitute for evidence, which is the failure the whole
    /// block exists to make visible.
    pub fn denominator_basis(&self) -> DenominatorBasis {
        match self.ids {
            Ids::Native => DenominatorBasis::Derived,
            Ids::Synthesized => DenominatorBasis::Synthesized,
        }
    }
}

/// Where a requirement is satisfied, and the evidence that disposition
/// owes.
///
/// ⭐ A layered standard satisfies most of its requirements SOMEWHERE
/// ELSE, and that is the normal case rather than an exception. Measured
/// in ISO 13400-2:2019, a middle layer: it defers to the layer below
/// ("as specified in ISO 13400-3"), to the layer above ("as specified
/// in ISO 14229-1"), to IETF documents seven times, and declares
/// something out of scope once. If every one of those reported
/// `missing`, the risk column would fill with correct-but-uninteresting
/// rows until a reader stopped looking at it — which is the same
/// failure [`Outcome::NeedsScenario`] exists to prevent, arriving by a
/// different route.
///
/// ⚠⚠ **Each variant carries its own evidence, and that is why this is
/// an enum with fields rather than a flag beside optional columns.**
/// `delegated` is the dangerous one: written alone it is a
/// not-our-problem stamp — the requirement leaves this manifest and
/// nothing checks that it arrived anywhere. Binding the target INTO the
/// variant means a delegation with no destination cannot be written at
/// all; the wire format refuses it rather than a reviewer having to
/// notice. Had the evidence lived in sibling `Option` fields, the stamp
/// would parse cleanly and the guard would be a runtime check somebody
/// could forget to call.
///
/// The evidence is held in two steps, and both run inside
/// [`RequirementManifest::from_json`], so neither is a step a caller
/// can skip:
///
/// - the FORMAT refuses a variant with a field absent, and a field that
///   belongs to another variant — a delegation carrying a `reason`
///   instead of a destination has no spelling;
/// - `Disposition::missing_evidence` refuses a field that is present
///   and says nothing. The format admits `"to_doc": ""`, and a key
///   filled in blank is the same stamp with its label left empty.
///
/// ⚠ `out_of_scope` carries prose for the same reason: a requirement
/// silently dropped into it is indistinguishable from one that was
/// never read. The line is one word, and it is the line the prose sweep
/// already draws between a token and a sentence (`is_one_word`): a
/// one-word reason is a flag spelled as a string — `n/a`, `tbd` — and
/// no reason is one word. Whether a longer reason is a GOOD one is a
/// reviewer's judgement, which this does not claim to make.
///
/// ⚠⚠ The reason is still swept for specification prose, and that is
/// deliberate rather than an oversight to exempt. It is the one field
/// meant to hold words, which makes it the likeliest place for a
/// specification's own "not covered by this document" sentence to be
/// pasted. So a reason is the author's words: it may not end in
/// sentence punctuation or carry a normative marker, and the refusal
/// names the field, so a false one is an inconvenience and not a
/// silence. Exempting the field would be the per-field defect the sweep
/// exists to remove.
///
/// ⚠⚠⚠ **What is NOT checked: that a delegation ARRIVED.** Naming
/// `to_doc` and `to_id` says a delegation was intended. Only the
/// target's own manifest can say it carries the requirement — RFC
/// §5.2e's second clause, inter-document traceability — and this crate
/// is handed one manifest at a time. A two-manifest check was drafted
/// and removed before landing, because it answered the wrong question:
/// given one target, it reported every delegation to any OTHER document
/// as broken, and a middle-layer document delegates to several at once.
/// It also left "arrived" undefined for a target entry that is itself
/// `out_of_scope` or delegated onward. Both are decisions for a real
/// second manifest to force, not for a fixture to guess.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Disposition {
    /// Satisfied in this artefact. The evidence is the annotation, read
    /// through [`Modality`] exactly as before this field existed.
    ///
    /// ⚠ An EMPTY STRUCT variant rather than a unit one, and the braces
    /// are load-bearing. serde applies `deny_unknown_fields` to the
    /// fields of a struct variant, but not to a unit variant of an
    /// internally tagged enum. Written as a unit, `{"kind":
    /// "implemented", "to_doc": "lower", "to_id": "L-1"}` LOADED, and
    /// the destination was dropped without a word. A delegation whose
    /// `kind` was mistyped became a requirement satisfied here: the
    /// silent drop this type exists to refuse, arriving through the
    /// default. This was measured, not supposed:
    /// `a_disposition_owes_its_own_evidence.rs` went red on it before
    /// the braces were added.
    Implemented {},
    /// Satisfied by another document or component.
    Delegated {
        /// The `doc_id` of the manifest that carries it.
        to_doc: String,
        /// The requirement id it becomes over there. Named separately
        /// because a layered standard renumbers: the same obligation is
        /// `3.DoIP-131` in one document and something else in the next,
        /// and a delegation that only named the document would leave a
        /// reader searching.
        to_id: String,
    },
    /// This toolchain cannot carry it.
    OutOfScope {
        /// Why. Prose, not a flag — see above.
        reason: String,
    },
    /// Realised by the deployment rather than by any behaviour.
    SystemLevel {
        /// The deployment artefact that realises it.
        ///
        /// ⚠ Weaker evidence than the others by necessity, and the
        /// weakness is stated rather than hidden: RFC §5.2f measured
        /// that the system level has **no annotation surface at all**,
        /// so nothing can confirm the requirement arrived there. Naming
        /// the artefact is what a reviewer can act on today; confirming
        /// it needs an annotation surface that does not exist.
        realised_by: String,
    },
}

/// `implemented`, so a manifest written before [`Disposition`] existed
/// keeps the meaning it had. Written by hand because `#[derive(Default)]`
/// accepts only a unit variant, and the variant must not be one — see
/// [`Disposition::Implemented`].
impl Default for Disposition {
    fn default() -> Self {
        Disposition::Implemented {}
    }
}

impl Disposition {
    /// The field that is present and says nothing, and why that is a
    /// refusal — or `None` when the disposition carries what it owes.
    ///
    /// `doc_id` is the document the manifest describes, because one
    /// delegation is empty without being blank: a `to_doc` naming the
    /// very document it leaves. RFC §5.2e defines `delegated` as
    /// satisfied by ANOTHER document, and a requirement satisfied in its
    /// own document is implemented here — or, if it is satisfied by
    /// other requirements beside it, decomposed, which is a parent/child
    /// relation and not a delegation. Admitting it would let the stamp
    /// point at itself.
    ///
    /// Ids are compared verbatim, as everywhere in this module.
    fn missing_evidence(&self, doc_id: &str) -> Option<(&'static str, &'static str)> {
        const BLANK: &str = "is blank";
        match self {
            Disposition::Implemented {} => None,
            Disposition::Delegated { to_doc, to_id } => {
                if to_doc.trim().is_empty() {
                    Some(("to_doc", BLANK))
                } else if to_doc == doc_id {
                    Some((
                        "to_doc",
                        "names the document this manifest describes, and a \
                         requirement satisfied in its own document is \
                         implemented there, not delegated",
                    ))
                } else if to_id.trim().is_empty() {
                    Some(("to_id", BLANK))
                } else {
                    None
                }
            }
            Disposition::OutOfScope { reason } => {
                if reason.trim().is_empty() {
                    Some(("reason", BLANK))
                } else if is_one_word(reason) {
                    Some((
                        "reason",
                        "is one word, and a flag spelled as a string is not a \
                         reason — silence is not a disposition",
                    ))
                } else {
                    None
                }
            }
            Disposition::SystemLevel { realised_by } => realised_by
                .trim()
                .is_empty()
                .then_some(("realised_by", BLANK)),
        }
    }
}

/// Whether `value` holds at most one word.
///
/// ⭐ The module's one line between a token and words, used from both
/// sides so it cannot be drawn twice. The prose sweep reads a one-word
/// string as a token and never as a sentence ([`prose_reason`]); an
/// `out_of_scope` reason of one word is a flag and not a reason
/// ([`Disposition`]). Two definitions would let a string be too short to
/// count as a reason and long enough to be swept as prose, or the
/// reverse, and neither answer would say which rule had moved.
fn is_one_word(value: &str) -> bool {
    value.split_whitespace().nth(1).is_none()
}

/// The manifest exactly as JSON spells it, before any check has run.
///
/// ⭐ Private, and that is the point. [`RequirementManifest`] does
/// **not** implement `Deserialize`, so the only way to obtain one is
/// [`RequirementManifest::from_json`], which runs the prose sweep. The
/// derive lives here instead, where nobody outside this module can
/// reach it.
///
/// ⚠ This exists because of a measurement, not a worry. With the
/// derive on the public type, `serde_json::from_str::<RequirementManifest>`
/// **loaded a manifest whose section title was a requirement
/// sentence**, while `from_json` refused the same bytes — so the
/// module's claim that such a manifest "fails to load, for everyone"
/// was true only of callers who picked the guarded spelling. That is
/// the same defect as the one this guard was written to close, one
/// level up: the protection was attached to a call path rather than to
/// the type, exactly as `text` was guarded by field name while the
/// field beside it was not. The next consumer is the acceptance
/// report, which has to load manifests, and `from_str` is the obvious
/// thing to reach for.
///
/// The cost is four field names written twice. It is paid at compile
/// time: add a field to [`RequirementManifest`] and forget it here and
/// the conversion below does not build.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestWire {
    doc_id: String,
    rev: String,
    /// No `#[serde(default)]`, unlike `sections`: a manifest that does
    /// not say how it was made is refused rather than assumed. See
    /// [`Extraction`].
    extraction: Extraction,
    #[serde(default)]
    sections: Vec<ManifestSection>,
    requirements: Vec<RequirementEntry>,
}

/// A closed requirement set for one `(doc_id, rev)`.
///
/// Deliberately not `Deserialize` — see [`ManifestWire`].
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RequirementManifest {
    /// Must match the `doc-id` a document's `sce:provenance` cites.
    pub doc_id: String,
    /// Revision. A document citing `@D3` checked against a `D4`
    /// manifest is stale — see [`Classification::revision_note`].
    pub rev: String,
    /// What is true of the extraction that produced `requirements`.
    /// Travels with the manifest into every artefact, because the
    /// denominator's standing is not recoverable from the numbers.
    pub extraction: Extraction,
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
    /// A requirement disposed somewhere other than this document, whose
    /// disposition carries a field that says nothing. `field` names it
    /// inside the disposition; `reason` says why it does not count.
    DispositionWithoutEvidence {
        path: String,
        id: String,
        field: &'static str,
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

/// The normative vocabulary of one convention.
///
/// ⭐ Measured, not intuited. `ES95486-02` — an OEM engineering
/// standard, read but never copied here — writes obligation with the
/// auxiliary `-어야/아야/여야 한다` rather than with any single verb, so
/// the entries below are the ENDINGS and not a list of verbs. Counted
/// over its 275 pages: `야 한다` 55, `야 함` 4, `야 하며` 1 (obligation);
/// `서는 안` 2, `지 않아야` 1 (prohibition); `수 있다` 29 (permission);
/// `권장` 1 (recommendation). Listing verbs instead would have matched
/// `하여야 한다` (13) and missed `해야 한다` (20) — the commoner form.
///
/// ⚠ `이내에` ("within …") is deliberately absent although it occurs.
/// It is a quantity phrase, not a modality: in "N ms 이내에 응답하여야
/// 한다" the obligation is carried by `야 한다` and `이내에` only says
/// how much. Admitting it would make every timing bound read as
/// normative prose.
///
/// ⚠⚠ This is a SET of markers, not a map from marker to [`Modality`],
/// and the difference is a measurement rather than a shortcut. Nothing
/// in this crate derives a modality from text: there is no function
/// returning [`Modality`] anywhere in `sce-build/src`, because SCE
/// consumes a manifest whose `modality` is already declared and never
/// sees the sentence at all — the sentence lives in the uncommitted
/// sidecar, which is the copyright split this format is built on.
///
/// A marker-to-modality map therefore has no consumer HERE. It belongs
/// to whatever authoring pass turns sentences into manifest entries,
/// and that pass is upstream of this repository. Building the map here
/// would be a table no test could exercise and no caller could be wrong
/// about — the "declared but unread" shape this module already refuses
/// once, where `RequirementId::validate` sat uncalled for 113 days.
fn normative_markers(convention: ModalityConvention) -> &'static [&'static str] {
    match convention {
        ModalityConvention::EnglishModalVerbs => &NORMATIVE_MODALS,
        ModalityConvention::KoreanFormalEndings => &[
            "야 한다",
            "야 함",
            "야 하며",
            "서는 안",
            "지 않아야",
            "수 있다",
            "권장",
        ],
    }
}

/// Whether `value` carries one of [`NORMATIVE_MODALS`] as a whole word.
///
/// Whole-word rather than substring, or `may` would fire on "Maybe"
/// and `must` on "mustard". Byte indexing is safe here: a non-ASCII
/// byte is not `is_ascii_alphanumeric`, so it reads as a boundary,
/// which is the answer a word check wants anyway.
/// ⭐ The declared convention ADDS a vocabulary; it never removes one.
///
/// A manifest that declares Korean is still checked against the English
/// markers, and the reason is a measurement rather than caution:
/// `ES95486-02` is the very document this convention was added for, and
/// it is MIXED — its first twenty pages are Korean-normative and carry
/// no `shall` at all, while the body from page 21 on carries 651 of
/// them. A guard that consulted only the declared table would have gone
/// blind to two thirds of that one document.
///
/// The direction matters more than the symmetry. This guard's failure
/// mode is committing somebody else's copyrighted sentence into a
/// public repository, which is permanent; its false-positive mode is a
/// refused heading, which is an inconvenience. So declaring a
/// convention may only ever make it see more.
fn carries_normative_modal(value: &str, convention: ModalityConvention) -> bool {
    let lower = value.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut tables = vec![normative_markers(ModalityConvention::EnglishModalVerbs)];
    if convention != ModalityConvention::EnglishModalVerbs {
        tables.push(normative_markers(convention));
    }
    tables.into_iter().flatten().any(|modal| {
        // ASCII markers are matched whole-word, or `may` fires on
        // "Maybe" and `must` on "mustard". A marker carrying non-ASCII
        // is a Korean ending, which is a distinctive sequence rather
        // than a word between spaces — Korean does not delimit these
        // the way English does, so the boundary test does not apply and
        // a substring match is the correct reading.
        if !modal.is_ascii() {
            return lower.contains(modal);
        }
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
    convention: ModalityConvention,
) -> Result<(), ManifestError> {
    match tree {
        serde_json::Value::String(value) => {
            if let Some(reason) = prose_reason(value, convention) {
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
                reject_prose_anywhere(item, field, label, convention)?;
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
                reject_prose_anywhere(item, field, label, convention)?;
                field.truncate(mark);
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn prose_reason(value: &str, convention: ModalityConvention) -> Option<&'static str> {
    // ⭐ Prose is multi-word. A string of one word is a token — an id,
    // a section number, a revision, or one of the format's own closed
    // vocabulary words — and no amount of it is a sentence. The line is
    // `is_one_word`, which an `out_of_scope` reason is held to from the
    // other side, so the two cannot be drawn in different places.
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
    if is_one_word(value) {
        return None;
    }
    let trimmed = value.trim_end();
    if trimmed.ends_with('.') || trimmed.ends_with('!') || trimmed.ends_with('?') {
        return Some("it ends in sentence punctuation");
    }
    if carries_normative_modal(value, convention) {
        return Some("it carries a normative marker of the declared convention");
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
                 (`id`, `section`, `at`); the requirement sentence \
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
                 COORDINATES only — an id, a division, a position \
                 inside it, or the heading a contents page prints. The \
                 sentence belongs \
                 in the uncommitted sidecar. If this is a real heading \
                 the check has misread, shorten it to the wording the \
                 contents page uses. If it is an `out_of_scope` reason, \
                 say why in your own words rather than the document's"
            ),
            ManifestError::DispositionWithoutEvidence {
                path,
                id,
                field,
                reason,
            } => write!(
                f,
                "requirement manifest {path}: `{id}` is disposed outside \
                 this document, but its `disposition.{field}` {reason}. A \
                 disposition other than `implemented` takes the \
                 requirement out of the `missing` column, and what it owes \
                 in exchange is something a reviewer can follow: the \
                 document and id it was delegated to, the reason this \
                 toolchain cannot carry it, or the deployment artefact \
                 that realises it"
            ),
        }
    }
}

impl std::error::Error for ManifestError {}

impl ManifestError {
    /// Which refusal this is, in words that do not carry the path.
    ///
    /// The sentence [`Display`](std::fmt::Display) writes names the file,
    /// so a diagnostic keyed on it would change identity with the
    /// directory a manifest was checked out into. This is the part of the
    /// refusal that belongs to the manifest.
    pub fn kind(&self) -> &'static str {
        match self {
            ManifestError::Read { .. } => "read",
            ManifestError::Parse { .. } => "parse",
            ManifestError::DuplicateId { .. } => "duplicate-id",
            ManifestError::Empty { .. } => "empty",
            ManifestError::Prose { .. } => "prose",
            ManifestError::DispositionWithoutEvidence { .. } => "disposition-without-evidence",
        }
    }
}

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
        let wire: ManifestWire =
            serde_json::from_value(tree.clone()).map_err(|source| ManifestError::Parse {
                path: label.to_string(),
                source,
            })?;
        // The sweep runs with the convention the manifest DECLARED, which
        // is why the wire parse has to come first: a Korean-authored
        // manifest is checked against Korean prose markers, and a
        // manifest that never said would be checked against English
        // alone — which is the state every manifest was in before this.
        reject_prose_anywhere(
            &tree,
            &mut String::new(),
            label,
            wire.extraction.modality_convention,
        )?;
        let manifest = RequirementManifest {
            doc_id: wire.doc_id,
            rev: wire.rev,
            extraction: wire.extraction,
            sections: wire.sections,
            requirements: wire.requirements,
        };
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
            // Here and not in the classifier: a disposition that owes
            // evidence and lacks it is a manifest that must not be used,
            // not a verdict to print beside the others. Refusing at load
            // is what makes every consumer inherit it, the same way the
            // prose sweep above is not a step a caller can skip.
            if let Some((field, reason)) = entry.disposition.missing_evidence(&manifest.doc_id) {
                return Err(ManifestError::DispositionWithoutEvidence {
                    path: label.to_string(),
                    id: entry.id.clone(),
                    field,
                    reason,
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
    /// The manifest says another document carries this, and named it.
    ///
    /// ⚠ This is a claim, not a confirmation. The name was required at
    /// load, so a reviewer can follow it; nothing here has followed it.
    /// Whether the target carries the requirement needs the target's
    /// own manifest — see [`Disposition`] for why that check is not
    /// here yet. A reader who treats this bucket as "handled" is making
    /// exactly the mistake the disposition exists to prevent.
    Delegated,
    /// The manifest says this toolchain cannot carry it, and said why.
    OutOfScope,
    /// The manifest says the deployment realises it, and named the
    /// artefact. Unconfirmable today — see [`Disposition::SystemLevel`].
    SystemLevel,
    /// The manifest places this requirement outside the document, and
    /// the document cites it on a node anyway.
    ///
    /// ⭐ For a disposition other than `implemented`, an annotation is
    /// not evidence. It is a second claim about where the requirement
    /// is met, and it disagrees with the first. Reporting the
    /// disposition's own bucket would certify the manifest's claim
    /// while the document's contrary claim sat in `node_paths`, a column
    /// nobody sorts a risk list by.
    ///
    /// One of the two is wrong. Either the annotation was invented, or
    /// the disposition is stale, or the requirement is met partly here
    /// and partly elsewhere. The last is decomposition, which the
    /// manifest cannot express yet (RFC §5.2f); naming the disagreement
    /// helps a reader more than a bucket that hides it. The row carries
    /// both claims, the disposition and the citing nodes, so the reader
    /// can tell which.
    Contradicted,
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Implemented => "implemented",
            Outcome::Unresolved => "unresolved",
            Outcome::Missing => "missing",
            Outcome::Dangling => "dangling",
            Outcome::NeedsScenario => "needs-scenario",
            Outcome::Delegated => "delegated",
            Outcome::OutOfScope => "out-of-scope",
            Outcome::SystemLevel => "system-level",
            Outcome::Contradicted => "contradicted",
        }
    }
}

/// One requirement's verdict, with the nodes that cite it.
#[derive(Debug, Clone, Serialize)]
pub struct RequirementOutcome {
    pub id: String,
    pub outcome: Outcome,
    /// The disposition's own evidence, carried into the artefact.
    ///
    /// ⭐ Without this the report says `"outcome":"delegated"` and stops
    /// — which is the not-our-problem stamp rebuilt one layer out.
    /// Binding the destination into [`Disposition`] stops the stamp
    /// being WRITTEN; carrying it here is what stops the stamp being
    /// READ. A reviewer meets the verdict and the destination on the
    /// same line, or the verdict is unactionable.
    ///
    /// `None` for [`Outcome::Dangling`]. The manifest has no entry for
    /// that id, so it has no disposition to report. Reporting the
    /// default instead would say the manifest had placed a requirement
    /// it never listed.
    ///
    /// Omitted from the artefact when `None`, and when
    /// [`Disposition::Implemented`]. That one is the default and says
    /// nothing a reader needs: every row would otherwise carry a word
    /// that only means "not elsewhere", and a column that is nearly
    /// always the same word is one a reader learns to skip, including
    /// on the rows where it is not.
    #[serde(skip_serializing_if = "places_nothing_elsewhere")]
    pub disposition: Option<Disposition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    /// Carried through from the manifest entry unchanged, so the report
    /// tells a reviewer where to open whatever shape the source is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<Position>,
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
    /// Carried from the manifest, unchanged. A classification that did
    /// not hold this would let a consumer print a percentage with the
    /// standing of its denominator left behind in a file it never read.
    pub extraction: Extraction,
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

/// Whether a row's disposition would tell a reader nothing — see
/// [`RequirementOutcome::disposition`].
fn places_nothing_elsewhere(disposition: &Option<Disposition>) -> bool {
    matches!(disposition, None | Some(Disposition::Implemented {}))
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
        Extraction {
            /// What the manifest said about itself, verbatim.
            declared: &'a Extraction,
            /// What SCE concluded from it. Nested beside `declared`
            /// rather than flattened into it so the artefact says which
            /// half is a claim and which half is a reading of that
            /// claim — a reader who disagrees with the conclusion can
            /// see the input it was drawn from on the same line.
            ///
            /// Written out rather than left for the consumer to infer:
            /// the rule that maps one to the other lives in exactly one
            /// place ([`Extraction::denominator_basis`]), and a consumer
            /// re-deriving it would be a second copy free to drift.
            denominator: DenominatorBasis,
        },
        Requirement(&'a RequirementOutcome),
        SectionCoverage {
            section: &'a str,
            requirements: usize,
        },
        RevisionMismatch {
            detail: &'a str,
        },
    }

    // First, before any count. A reader who stops after one line has the
    // standing of everything below it; a reader who stops before
    // reaching it has no numbers yet either.
    writeln!(
        writer,
        "{}",
        serde_json::to_string(&Line::Extraction {
            declared: &classification.extraction,
            denominator: classification.extraction.denominator_basis(),
        })
        .expect("Extraction serialises; every field is a unit enum")
    )?;

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
        // ⭐ Disposition is asked BEFORE modality, and the order is the
        // point. Modality decides what evidence would settle a
        // requirement satisfied HERE; disposition decides whether it is
        // satisfied here at all. Asking modality first would report a
        // delegated requirement as `missing` — technically true of this
        // document and useless to a reader, because for a middle-layer
        // standard that is most of the rows.
        //
        // For the three dispositions that place a requirement elsewhere,
        // their evidence was required at load and a citation here is
        // not evidence but a contrary claim — see
        // [`Outcome::Contradicted`]. The citation is read for that and
        // for nothing else.
        let placed_elsewhere = |outcome: Outcome| {
            if citation.is_some() {
                Outcome::Contradicted
            } else {
                outcome
            }
        };
        let outcome = match &entry.disposition {
            Disposition::Delegated { .. } => placed_elsewhere(Outcome::Delegated),
            Disposition::OutOfScope { .. } => placed_elsewhere(Outcome::OutOfScope),
            Disposition::SystemLevel { .. } => placed_elsewhere(Outcome::SystemLevel),
            Disposition::Implemented {} => match entry.modality {
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
            },
        };
        outcomes.push(RequirementOutcome {
            id: entry.id.clone(),
            outcome,
            disposition: Some(entry.disposition.clone()),
            section: entry.section.clone(),
            at: entry.at.clone(),
            node_paths,
        });
    }

    for (id, (paths, _)) in &cited {
        if !declared.contains(id) {
            outcomes.push(RequirementOutcome {
                id: (*id).to_string(),
                outcome: Outcome::Dangling,
                // A dangling id is one the manifest does not contain, so
                // it has no disposition, and saying so is not the same as
                // saying `implemented`.
                disposition: None,
                section: None,
                at: None,
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
        extraction: manifest.extraction,
        outcomes,
        section_counts,
        revision_note,
    }
}
