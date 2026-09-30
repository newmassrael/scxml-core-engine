// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The authoring profile: what a specification owner configures about how a
//! design is authored for them, stated in a file instead of said once in a
//! conversation.
//!
//! # Why a file, and why not a flag
//!
//! The product judges a document by grammar and by what SCXML means, and it
//! cannot know what its author was asked for. A statechart with no schema and
//! no `sce:interface` is a W3C conformance document, a legacy machine, or a
//! design an owner is about to be shown; nothing in it says which. Guessing
//! from a proxy — a recorded kind basis, an import — reads intent into a
//! feature that means something else, and a second guess for the next
//! expectation is a second way to say the same thing. The owner's
//! expectation belongs where the owner can state it, change it, and have it
//! remembered: a profile, beside the specification.
//!
//! So intent is explicit. A run without a profile judges nothing new, and a
//! run under one is held to exactly what the file says, by the same parse the
//! build compiles.
//!
//! # Three classes, fixed by the schema
//!
//! Every setting belongs to one class, and the class is not the file's to
//! choose ([`SETTINGS`]):
//!
//! ```text
//!   enforced   a draft that breaks it is refused
//!   reported   every departure is listed, and the owner decides
//!   guidance   handed to the author, checked by nothing, and said so
//! ```
//!
//! The settings a build knows are the rows of [`SETTINGS`]. The enforced ones
//! are the ones a machine can decide from the text: whether the interface is
//! declared closed, how the names a document defines are spelled
//! ([`names`]), and whether every piece of evidence for the document's kind
//! names an anchor. The one guidance setting is a list of instructions to the
//! author that nothing checks, which a run under the profile says in as many
//! words. No setting is reported yet. A profile naming a setting this build
//! does not know is REFUSED whole ([`ProfileUnusable`]): a profile written
//! for a newer tool that was half applied would say "checked under this
//! profile" of a document it never held to it.
//!
//! # Part of the attempt key
//!
//! A design accepted under one profile is not the answer for another. The
//! acceptance record pins the file's sha256 as one of the things the design
//! was authored from ([`crate::acceptance_record::SourceRole::Profile`]), and
//! a run under a profile publishes the same digest in the manifest, so the
//! digest that judged a design and the digest an acceptance was taken under
//! can be compared.
//!
//! # What a profile may not configure
//!
//! The obligation to mark a guess, W3C SCXML semantics, the kind catalog and
//! `<sce:kind-basis>`, and acceptance by a person. Nothing here can turn
//! those off, and no setting is named so that it could.

mod messages;
pub mod names;

use std::path::Path;

use serde::Deserialize;

use crate::forge::error::{ForgeError, Located};
use crate::generator_witness::{hex_encode, sha256_bytes};
use crate::model::SCXMLModel;
use names::{Departure, EventProblem, NameClass, NameLimit, NamesRule, Style};

/// The value a profile's `record` field holds. A JSON file naming any other
/// kind is refused before its settings are read.
pub const RECORD_KIND: &str = "sce-authoring-profile";

/// The format version. Moves only when a setting changes meaning; a setting
/// is added without it.
pub const PROFILE_VERSION: u32 = 1;

/// Stability status of the profile wire surface. Pinned to the
/// `x-sce-schema-status` header of
/// `schemas/sce-authoring-profile.v1.schema.json` by
/// [`tests::schema_file_declares_status`].
pub const PROFILE_SCHEMA_STATUS: &str = "pre-release";

/// What the product does with a setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingClass {
    /// A draft that breaks it is refused.
    Enforced,
    /// Every departure is listed and the owner decides.
    Reported,
    /// Handed to the author; nothing checks it.
    Guidance,
}

/// Every setting the schema knows, with the class the schema fixes. One
/// table, read by the schema-drift guard, so a setting cannot be added to the
/// schema or to the reader alone.
pub const SETTINGS: &[(&str, SettingClass)] = &[
    ("interface", SettingClass::Enforced),
    ("names", SettingClass::Enforced),
    ("evidence", SettingClass::Enforced),
    ("traceability", SettingClass::Enforced),
    ("house_rules", SettingClass::Reported),
    ("guidance", SettingClass::Guidance),
];

/// What `interface` asks of a statechart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InterfaceRule {
    /// Every statechart declares `sce:interface="closed"`.
    Closed,
}

/// What `evidence` asks of a statechart's `<sce:kind-basis>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceRule {
    /// Every `<sce:evidence>` names where the specification states what it
    /// cites.
    Anchored,
}

/// What `traceability` asks of a statechart.
///
/// ⚠ Why a setting and not a line in every answer. Measured 2026-10-01, fifteen
/// drafts by a real client, told in the server's instructions to put each
/// requirement's id on the element that carries it: none did, and none named the
/// tool that lists them. The request asked for a kind, a draft, a check and
/// what is open, and the client did that. An owner who wants each sentence of
/// their specification found in the design is the one to say so, in the file
/// that says what a design is held to, and a draft that ignores it is then
/// refused rather than left to an instruction. Left out, nothing changes: a
/// statechart with no `sce:req` on it is as accepted as it was.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TraceabilityRule {
    /// Every state and every transition claims a requirement (`sce:req`), so
    /// each element of the design says which sentence of the specification it
    /// is there for.
    ///
    /// ⚠ States and transitions, and not what runs inside them. A transition's
    /// own actions do not inherit its `sce:req` (an `<onentry>`'s do), so the
    /// product's own table counts a `<cancel>` inside a claimed transition as
    /// unclaimed, and a rule over every node would refuse a design that claims
    /// every sentence (`requirements_report.rs`, "The unclaimed block
    /// dilutes"). A state and a transition are the units that do not dilute.
    Required,
}

/// A standing answer of the owner to a gap that recurs across specifications:
/// what a draft does where the specification is silent, decided once.
///
/// A draft that applies one cites it by its id (`sce:assumed="H1"`) on the
/// element it applies to, and every citation is listed in the run's `open`
/// ([`crate::open_matters::OpenKind::HouseRule`]), so a rule is never applied
/// silently. What the product cannot do is see a rule applied WITHOUT its
/// citation: a rule is prose, and nothing here reads a document's behaviour
/// against prose. The authoring core's `decisions` check is what licenses a
/// citation, the way it licenses a decision's.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HouseRule {
    /// What a draft writes in `sce:assumed` to cite the rule: one token.
    pub id: String,
    /// The rule, in the owner's words.
    pub rule: String,
}

/// What follows the `record` and `v` header. `deny_unknown_fields` is the
/// refusal of a setting this build does not know.
///
/// The header is read and checked first ([`AuthoringProfile::from_text`]),
/// so a profile from a newer tool is refused for its version and not for the
/// first setting it holds that this build has never heard of.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Settings {
    name: Option<String>,
    interface: Option<InterfaceRule>,
    names: Option<NamesRule>,
    evidence: Option<EvidenceRule>,
    traceability: Option<TraceabilityRule>,
    house_rules: Option<Vec<HouseRule>>,
    guidance: Option<Vec<String>>,
}

/// Why a profile cannot be used. The wire `kind` of `cli/profile-unusable`:
/// SCE determines it, unlike the parser's own sentence, so it can key a
/// record without a platform's spelling of an error in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileFault {
    /// The file could not be read.
    Unreadable,
    /// The text is not JSON.
    NotJson,
    /// JSON, and not the shape a profile has: a setting this build does not
    /// know, a value a setting does not take, or a missing field.
    InvalidShape,
    /// JSON of another kind of file.
    WrongRecord,
    /// A version this build does not read.
    UnsupportedVersion,
    /// A `name` with nothing in it.
    EmptyName,
}

impl ProfileFault {
    /// The spelling on the wire.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unreadable => "unreadable",
            Self::NotJson => "not-json",
            Self::InvalidShape => "invalid-shape",
            Self::WrongRecord => "wrong-record",
            Self::UnsupportedVersion => "unsupported-version",
            Self::EmptyName => "empty-name",
        }
    }
}

/// A profile file the product cannot use: unreadable, not JSON, another kind
/// of file, a version it does not read, or a setting it does not know.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileUnusable {
    /// Which refusal.
    pub kind: ProfileFault,
    /// What is wrong, in a sentence. Names no path: the caller knows which
    /// file it handed over.
    pub detail: String,
}

impl std::fmt::Display for ProfileUnusable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.detail)
    }
}

/// A finding the profile makes about a document: the document is valid, and
/// it is not what the profile asks for.
#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    /// The profile requires `sce:interface="closed"`, and the statechart does
    /// not declare it.
    #[error("{}", interface_not_closed_message(profile.as_deref(), imports))]
    InterfaceNotClosed {
        /// The profile's `name`, when it has one.
        profile: Option<String>,
        /// The statechart's model name (its file stem), so two statecharts
        /// that break one profile are two findings.
        machine: String,
        /// The event-schemas the statechart imports and so describes a
        /// boundary with, which nothing then holds it to.
        imports: Vec<String>,
    },
    /// A name is not spelled in the style the profile asks its class for.
    #[error("{}", messages::name_style(profile.as_deref(), *class, name, part, *style, respelled.as_deref()))]
    NameStyle {
        /// The profile's `name`, when it has one.
        profile: Option<String>,
        /// Which class of name.
        class: NameClass,
        /// The name as the document writes it.
        name: String,
        /// The token of the name that is not in the style: the name itself,
        /// unless the class is `event`, whose tokens are judged one by one.
        part: String,
        /// The style the class is asked for.
        style: Style,
        /// The name respelled in that style, when the style accepts a
        /// respelling of it.
        respelled: Option<String>,
    },
    /// A name breaks a limit the profile sets its class: a word it may not be
    /// made of, a length, or a prefix.
    #[error("{}", messages::name_limit(profile.as_deref(), *class, name, limit))]
    NameLimit {
        /// The profile's `name`, when it has one.
        profile: Option<String>,
        /// Which class of name.
        class: NameClass,
        /// The name as the document writes it.
        name: String,
        /// Which limit, and how the name breaks it.
        limit: NameLimit,
    },
    /// The structure of an event name is not what the profile asks for.
    #[error("{}", messages::event_structure(profile.as_deref(), name, problem))]
    EventStructure {
        /// The profile's `name`, when it has one.
        profile: Option<String>,
        /// The event name as the document writes it.
        name: String,
        /// What is wrong with its structure.
        problem: EventProblem,
    },
    /// An event name is a token prefix of another, so a descriptor on the
    /// shorter one also matches the longer (W3C SCXML 3.12.1).
    #[error("{}", messages::event_prefix(profile.as_deref(), prefix, longer))]
    EventPrefixOfAnother {
        /// The profile's `name`, when it has one.
        profile: Option<String>,
        /// The shorter name.
        prefix: String,
        /// The name it is a token prefix of.
        longer: String,
    },
    /// An `<sce:evidence>` of the document's kind basis names no anchor in the
    /// specification.
    #[error("{}", messages::evidence_unanchored(profile.as_deref(), evidence))]
    EvidenceUnanchored {
        /// The profile's `name`, when it has one.
        profile: Option<String>,
        /// What the evidence says, as the author wrote it.
        evidence: String,
    },
    /// A state or a transition claims no requirement (`sce:req`), so nothing
    /// says which sentence of the specification it is there for.
    #[error("{}", messages::element_untraced(profile.as_deref(), element))]
    ElementUntraced {
        /// The profile's `name`, when it has one.
        profile: Option<String>,
        /// The element, as the model names it: `state idle`, or the transition
        /// with its event and target.
        element: String,
    },
}

/// [`ProfileError::InterfaceNotClosed`]'s message.
fn interface_not_closed_message(profile: Option<&str>, imports: &[String]) -> String {
    let subject = messages::subject(profile);
    let described = if imports.is_empty() {
        String::new()
    } else {
        format!(
            "; it imports event-schema(s) ({}) that describe a boundary nothing holds it to",
            imports.join(", ")
        )
    };
    format!(
        "{subject} requires sce:interface=\"closed\" on a statechart, and this one does not \
         declare it{described} — declare it closed on the root <scxml> and give every event it \
         takes or sends an event-schema, or use a profile that does not require it: which \
         boundary a design is held to is the owner's decision"
    )
}

/// Why a list of house rules is not one the schema accepts: the rules `serde`
/// cannot state. The schema says the same with `minItems`, `pattern` and
/// `minLength`; what it cannot say is that an id names ONE rule.
fn house_rules_refusal(rules: &[HouseRule]) -> Option<String> {
    if rules.is_empty() {
        return Some("`house_rules` lists a rule or none".to_string());
    }
    for rule in rules {
        if rule.id.is_empty() || rule.id.chars().any(char::is_whitespace) {
            return Some(format!(
                "`house_rules` holds the id {:?}, and an id is one token: a draft writes it in \
                 sce:assumed, where a marker id holds no space",
                rule.id
            ));
        }
        if rule.rule.is_empty() {
            return Some(format!("the house rule {} says nothing", rule.id));
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    rules
        .iter()
        .find(|rule| !seen.insert(rule.id.as_str()))
        .map(|rule| {
            format!(
                "the house rule {} is listed twice; a draft citing it could not say which it \
                 applied",
                rule.id
            )
        })
}

/// The states and transitions of a statechart that claim no requirement, in
/// the order the document writes them, each named as an author reads it and
/// placed where it is written.
///
/// A state is named by its id and a transition by its owning state, its event
/// (`(eventless)` for none) and its target, which is what tells two transitions
/// of one state apart in a sentence an owner reads without the document open.
/// A transition inside a state that claims a requirement is still judged on its
/// own: `sce:req` on a state says the state is asked for, not that each way out
/// of it is.
fn untraced_elements(
    model: &SCXMLModel,
) -> Vec<(String, Option<crate::forge::error::SourceLocation>)> {
    let mut states: Vec<_> = model.states.values().collect();
    states.sort_by_key(|state| state.document_order);
    let mut found = Vec::new();
    for state in states {
        if state.req.is_empty() {
            found.push((format!("state {}", state.id), state.source_location.clone()));
        }
        for transition in &state.transitions {
            if !transition.req.is_empty() {
                continue;
            }
            let event = if transition.event.is_empty() {
                "(eventless)"
            } else {
                transition.event.as_str()
            };
            let target = if transition.target.is_empty() {
                "(no target)"
            } else {
                transition.target.as_str()
            };
            found.push((
                format!(
                    "the transition of state {} on {event} to {target}",
                    state.id
                ),
                transition.source_location.clone(),
            ));
        }
    }
    found
}

/// A profile, read and validated.
#[derive(Debug, Clone)]
pub struct AuthoringProfile {
    name: Option<String>,
    interface: Option<InterfaceRule>,
    names: Option<NamesRule>,
    evidence: Option<EvidenceRule>,
    traceability: Option<TraceabilityRule>,
    house_rules: Vec<HouseRule>,
    guidance: Vec<String>,
    sha256: String,
}

impl AuthoringProfile {
    /// Read a profile from its text. `text` is what the digest is taken over,
    /// so the same bytes are the same profile wherever they are read.
    pub fn from_text(text: &str) -> Result<Self, ProfileUnusable> {
        let unusable = |kind, detail: String| ProfileUnusable { kind, detail };
        let shape = |error: serde_json::Error| {
            let kind = match error.classify() {
                serde_json::error::Category::Data => ProfileFault::InvalidShape,
                _ => ProfileFault::NotJson,
            };
            unusable(kind, format!("not an authoring profile: {error}"))
        };
        let document: serde_json::Value = serde_json::from_str(text).map_err(shape)?;
        let serde_json::Value::Object(mut fields) = document else {
            return Err(unusable(
                ProfileFault::InvalidShape,
                "not an authoring profile: a profile is one JSON object".to_string(),
            ));
        };
        // The header first: `record` names the kind of file and `v` says
        // which settings it may hold, so both are judged before any setting
        // is read.
        let record = fields.remove("record");
        if record.as_ref().and_then(serde_json::Value::as_str) != Some(RECORD_KIND) {
            return Err(unusable(
                ProfileFault::WrongRecord,
                format!(
                    "the file's `record` is {}, and an authoring profile's is `{RECORD_KIND}`",
                    record.map_or_else(|| "missing".to_string(), |value| value.to_string())
                ),
            ));
        }
        let version = fields.remove("v");
        if version.as_ref().and_then(serde_json::Value::as_u64) != Some(u64::from(PROFILE_VERSION))
        {
            return Err(unusable(
                ProfileFault::UnsupportedVersion,
                format!(
                    "profile version {} — this build reads version {PROFILE_VERSION}, and half \
                     applying a profile it does not fully read would say a document was held to \
                     it when it was not",
                    version.map_or_else(|| "missing".to_string(), |value| value.to_string())
                ),
            ));
        }
        let settings: Settings =
            serde_json::from_value(serde_json::Value::Object(fields)).map_err(shape)?;
        if settings.name.as_deref() == Some("") {
            return Err(unusable(
                ProfileFault::EmptyName,
                "`name` is empty; leave it out or give it a label".to_string(),
            ));
        }
        // What the schema says with `minProperties`, `minItems` and `pattern`
        // and `serde` cannot: a setting that is present says something.
        if let Some(fault) = settings.names.as_ref().and_then(NamesRule::refusal) {
            return Err(unusable(
                ProfileFault::InvalidShape,
                format!("not an authoring profile: {fault}"),
            ));
        }
        if let Some(guidance) = &settings.guidance {
            if guidance.is_empty() || guidance.iter().any(|entry| entry.is_empty()) {
                return Err(unusable(
                    ProfileFault::InvalidShape,
                    "not an authoring profile: `guidance` lists an instruction or none".to_string(),
                ));
            }
        }
        if let Some(fault) = settings
            .house_rules
            .as_deref()
            .and_then(house_rules_refusal)
        {
            return Err(unusable(
                ProfileFault::InvalidShape,
                format!("not an authoring profile: {fault}"),
            ));
        }
        Ok(Self {
            name: settings.name,
            interface: settings.interface,
            names: settings.names,
            evidence: settings.evidence,
            traceability: settings.traceability,
            house_rules: settings.house_rules.unwrap_or_default(),
            guidance: settings.guidance.unwrap_or_default(),
            sha256: hex_encode(&sha256_bytes(text.as_bytes())),
        })
    }

    /// Read a profile from a file.
    pub fn load(path: &Path) -> Result<Self, ProfileUnusable> {
        let text = std::fs::read_to_string(path).map_err(|error| ProfileUnusable {
            kind: ProfileFault::Unreadable,
            detail: format!("cannot read it: {error}"),
        })?;
        Self::from_text(&text)
    }

    /// The label a report prints, when the profile has one.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The sha256 of the file's bytes: what an acceptance record pins and the
    /// manifest publishes.
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    /// Whether the profile asks anything of a statechart at all — false for a
    /// profile that holds only settings no statechart is judged by, or none.
    pub fn judges_statecharts(&self) -> bool {
        self.interface.is_some()
            || self.names.is_some()
            || self.evidence.is_some()
            || self.traceability.is_some()
    }

    /// The instructions the profile hands to whoever writes the document, in
    /// the order the owner wrote them. Nothing checks any of them.
    pub fn guidance(&self) -> &[String] {
        &self.guidance
    }

    /// The house rules, in the order the owner wrote them.
    pub fn house_rules(&self) -> &[HouseRule] {
        &self.house_rules
    }

    /// The ids of the house rules: what an `sce:assumed` writes to cite one.
    pub fn house_rule_ids(&self) -> Vec<&str> {
        self.house_rules.iter().map(|r| r.id.as_str()).collect()
    }

    /// Mark, in a run's marker records, the ones that cite one of this
    /// profile's house rules ([`crate::unresolved_check::cite_house_rules`]).
    pub fn cite_house_rules(&self, records: &mut [crate::unresolved_check::UnresolvedRecord]) {
        crate::unresolved_check::cite_house_rules(records, &self.house_rule_ids());
    }

    /// Every way a statechart departs from the profile's ENFORCED settings,
    /// each located where it is written.
    ///
    /// The model is the one the build compiles — the caller parsed the file
    /// through the production parser — so what is judged is what would be
    /// built, not a second reading of the text.
    ///
    /// In the order the settings are described: the interface, the names, the
    /// evidence.
    pub fn judge_statechart(
        &self,
        model: &SCXMLModel,
        diag_label: &str,
    ) -> Vec<Located<ForgeError>> {
        let profile = || self.name.clone();
        let mut findings = Vec::new();
        let mut place = |error: ProfileError, at: Option<&crate::forge::error::SourceLocation>| {
            findings.push(model.locate(ForgeError::Profile(Box::new(error)), at, diag_label));
        };
        if self.interface == Some(InterfaceRule::Closed) && !model.interface_closed {
            place(
                ProfileError::InterfaceNotClosed {
                    profile: profile(),
                    machine: model.name.clone(),
                    imports: crate::open_matters::interface_left_open(model),
                },
                model.source_location.as_ref(),
            );
        }
        if let Some(rule) = &self.names {
            for found in names::judge(rule, model) {
                // A name the walk found has its own row; one it could not
                // place (a raise inside a `<finalize>`) is put on the root,
                // which is where the document as a whole is.
                place(
                    self.error_for(found.departure),
                    found.at.as_ref().or(model.source_location.as_ref()),
                );
            }
        }
        if let Some(basis) = &model.kind_basis {
            for error in self.unanchored(basis) {
                place(error, model.source_location.as_ref());
            }
        }
        if self.traceability == Some(TraceabilityRule::Required) {
            for (element, at) in untraced_elements(model) {
                place(
                    ProfileError::ElementUntraced {
                        profile: profile(),
                        element,
                    },
                    at.as_ref().or(model.source_location.as_ref()),
                );
            }
        }
        findings
    }

    /// Whether some setting of the profile reaches a forge document of `kind`:
    /// the evidence rule reaches every kind that states a kind basis, and the
    /// names rule reaches an event-schema, whose event and fields are names the
    /// owner's document defines.
    ///
    /// The other kinds have names of their own — a transform's outputs, a
    /// codec's fields — that no setting judges yet, and a run that says
    /// `judged: 0` of them says so rather than passing them.
    pub fn judges_forge(&self, kind: crate::forge::model::ForgeKind) -> bool {
        self.evidence.is_some()
            || (kind == crate::forge::model::ForgeKind::EventSchema
                && self
                    .names
                    .as_ref()
                    .is_some_and(NamesRule::reaches_event_schemas))
    }

    /// Every way a forge document departs from the profile's ENFORCED
    /// settings. Empty for a kind [`Self::judges_forge`] says no setting
    /// reaches.
    ///
    /// A finding carries the document and no row: the forge model keeps none
    /// for the event a schema declares, its fields, or an evidence.
    pub fn judge_forge(
        &self,
        parsed: &crate::forge::model::ParsedForge,
        diag_label: &str,
    ) -> Vec<Located<ForgeError>> {
        let place = |error: ProfileError| {
            Located::new(ForgeError::Profile(Box::new(error)), diag_label, None, None)
        };
        let mut findings = Vec::new();
        if let (Some(rule), crate::forge::model::ForgeDocument::EventSchema(schema)) =
            (&self.names, &parsed.document)
        {
            let fields: Vec<String> = schema.fields.iter().map(|f| f.id.clone()).collect();
            for found in names::judge_event_schema(rule, &schema.event_name, &fields) {
                findings.push(place(self.error_for(found.departure)));
            }
        }
        if let Some(basis) = &parsed.kind_basis {
            findings.extend(self.unanchored(basis).into_iter().map(place));
        }
        findings
    }

    /// The errors for a kind basis's evidence that names no anchor, when the
    /// profile asks for one.
    fn unanchored(&self, basis: &crate::forge::kind_basis::KindBasis) -> Vec<ProfileError> {
        if self.evidence != Some(EvidenceRule::Anchored) {
            return Vec::new();
        }
        basis
            .evidence
            .iter()
            .filter(|evidence| evidence.provenance.is_none())
            .map(|evidence| ProfileError::EvidenceUnanchored {
                profile: self.name.clone(),
                evidence: evidence.text.clone(),
            })
            .collect()
    }

    /// The error a departure from the `names` setting is reported as.
    fn error_for(&self, departure: Departure) -> ProfileError {
        let profile = self.name.clone();
        match departure {
            Departure::Style {
                class,
                name,
                part,
                style,
                respelled,
            } => ProfileError::NameStyle {
                profile,
                class,
                name,
                part,
                style,
                respelled,
            },
            Departure::Limit { class, name, limit } => ProfileError::NameLimit {
                profile,
                class,
                name,
                limit,
            },
            Departure::Event { name, problem } => ProfileError::EventStructure {
                profile,
                name,
                problem,
            },
            Departure::PrefixOfAnother { prefix, longer } => ProfileError::EventPrefixOfAnother {
                profile,
                prefix,
                longer,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLOSED: &str =
        r#"{"record":"sce-authoring-profile","v":1,"name":"owner-review","interface":"closed"}"#;

    fn schema() -> serde_json::Value {
        serde_json::from_str(include_str!(
            "../../schemas/sce-authoring-profile.v1.schema.json"
        ))
        .expect("authoring profile schema is JSON")
    }

    fn violations(instance: &serde_json::Value) -> Vec<String> {
        let schema = schema();
        let validator = jsonschema::JSONSchema::options()
            .with_draft(jsonschema::Draft::Draft7)
            .compile(&schema)
            .expect("authoring profile schema compiles");
        let outcome = validator.validate(instance);
        match outcome {
            Ok(()) => Vec::new(),
            Err(errors) => errors.map(|e| e.to_string()).collect(),
        }
    }

    fn statechart(root_attrs: &str) -> SCXMLModel {
        crate::parser::SCXMLParser::new()
            .parse_string(
                &format!(
                    r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                             xmlns:sce="http://sce.dev/ext" version="1.0" name="gate"
                             initial="a" {root_attrs}>
                         <sce:import src="s.scxml" kind="event-schema" as="Sig"/>
                         <state id="a"/>
                       </scxml>"#
                ),
                "authoring_profile",
            )
            .expect("parses")
    }

    #[test]
    fn schema_file_declares_status() {
        assert_eq!(
            schema()["x-sce-schema-status"].as_str(),
            Some(PROFILE_SCHEMA_STATUS),
            "schemas/sce-authoring-profile.v1.schema.json x-sce-schema-status disagrees with \
             PROFILE_SCHEMA_STATUS; SCE_WIRE_CONTRACTS.md requires one commit to move both",
        );
    }

    #[test]
    fn schema_version_and_kind_match_the_reader() {
        let schema = schema();
        assert_eq!(
            schema["properties"]["v"]["const"].as_u64(),
            Some(u64::from(PROFILE_VERSION))
        );
        assert_eq!(
            schema["properties"]["record"]["const"].as_str(),
            Some(RECORD_KIND)
        );
    }

    /// The schema's settings and the reader's are one list. A setting added
    /// to the schema and not to [`SETTINGS`] — or the reverse — has a class
    /// nobody fixed, or a property nothing reads.
    #[test]
    fn the_schema_and_the_settings_table_name_the_same_settings() {
        let schema = schema();
        let properties = schema["properties"].as_object().expect("properties");
        let mut in_schema: Vec<&str> = properties
            .keys()
            .map(String::as_str)
            .filter(|key| !matches!(*key, "record" | "v" | "name"))
            .collect();
        in_schema.sort_unstable();
        let mut in_table: Vec<&str> = SETTINGS.iter().map(|(key, _)| *key).collect();
        in_table.sort_unstable();
        assert_eq!(in_schema, in_table);
    }

    #[test]
    fn every_profile_the_product_reads_validates_against_the_wire_schema() {
        for text in [
            CLOSED,
            r#"{"record":"sce-authoring-profile","v":1}"#,
            r#"{"record":"sce-authoring-profile","v":1,"interface":"closed"}"#,
        ] {
            let instance: serde_json::Value = serde_json::from_str(text).expect("JSON");
            assert!(violations(&instance).is_empty(), "{text}");
            AuthoringProfile::from_text(text).expect("the reader takes what the schema does");
        }
    }

    /// Starts from a profile the schema accepts and changes ONE thing, so the
    /// refusal is about that thing.
    #[test]
    fn the_profile_schema_rejects_a_setting_the_product_does_not_know() {
        let valid: serde_json::Value = serde_json::from_str(CLOSED).expect("JSON");
        assert!(violations(&valid).is_empty(), "the control has to be valid");
        let mut unknown = valid.clone();
        unknown["naming"] = serde_json::json!("camel");
        assert!(!violations(&unknown).is_empty());
        let mut wrong_value = valid;
        wrong_value["interface"] = serde_json::json!("open");
        assert!(!violations(&wrong_value).is_empty());
    }

    /// The reader refuses what the schema refuses, and says why — a profile
    /// half applied would say a document was held to it when it was not.
    #[test]
    fn the_reader_refuses_the_whole_profile_it_cannot_fully_read() {
        for (text, kind, needle) in [
            (
                r#"{"record":"sce-authoring-profile","v":1,"naming":"camel"}"#,
                ProfileFault::InvalidShape,
                "naming",
            ),
            (
                r#"{"record":"sce-authoring-profile","v":1,"interface":"open"}"#,
                ProfileFault::InvalidShape,
                "open",
            ),
            (
                r#"{"record":"sce-authoring-profile","v":2}"#,
                ProfileFault::UnsupportedVersion,
                "version 2",
            ),
            // A newer profile is refused for its version, not for the first
            // setting this build has never heard of.
            (
                r#"{"record":"sce-authoring-profile","v":2,"naming":"camel"}"#,
                ProfileFault::UnsupportedVersion,
                "version 2",
            ),
            (
                r#"{"record":"sce-authoring-profile"}"#,
                ProfileFault::UnsupportedVersion,
                "version missing",
            ),
            (
                r#"{"record":"sce-decision-record","v":1}"#,
                ProfileFault::WrongRecord,
                "sce-decision-record",
            ),
            (r#"{"v":1}"#, ProfileFault::WrongRecord, "missing"),
            (
                r#"{"record":"sce-authoring-profile","v":1,"name":""}"#,
                ProfileFault::EmptyName,
                "`name` is empty",
            ),
            ("[1]", ProfileFault::InvalidShape, "one JSON object"),
            (
                "not json",
                ProfileFault::NotJson,
                "not an authoring profile",
            ),
        ] {
            let refused = AuthoringProfile::from_text(text).expect_err(text);
            assert_eq!(refused.kind, kind, "{text}: {refused}");
            assert!(
                refused.detail.contains(needle),
                "{text}: expected the refusal to mention {needle:?}, got {refused}"
            );
        }
    }

    #[test]
    fn a_profile_file_that_is_not_there_is_unreadable() {
        let refused = AuthoringProfile::load(Path::new("/nonexistent/profile.json"))
            .expect_err("nothing there");
        assert_eq!(refused.kind, ProfileFault::Unreadable);
        assert_eq!(refused.kind.as_str(), "unreadable");
    }

    #[test]
    fn the_digest_is_the_files_own() {
        let a = AuthoringProfile::from_text(CLOSED).expect("reads");
        let renamed = CLOSED.replace("owner-review", "house-review");
        let b = AuthoringProfile::from_text(&renamed).expect("reads");
        assert_eq!(a.sha256().len(), 64);
        assert_ne!(
            a.sha256(),
            b.sha256(),
            "a renamed profile is a new profile to an acceptance record"
        );
        assert_eq!(
            a.sha256(),
            AuthoringProfile::from_text(CLOSED).expect("reads").sha256()
        );
        assert_eq!(a.name(), Some("owner-review"));
    }

    #[test]
    fn a_statechart_that_is_not_closed_departs_from_a_profile_that_requires_it() {
        let profile = AuthoringProfile::from_text(CLOSED).expect("reads");
        let open = profile.judge_statechart(&statechart(""), "gate.scxml");
        assert_eq!(open.len(), 1, "{open:?}");
        let said = open[0].error.to_string();
        assert!(
            said.contains("'owner-review'") && said.contains("(Sig)"),
            "{said}"
        );

        let closed =
            profile.judge_statechart(&statechart("sce:interface=\"closed\""), "gate.scxml");
        assert!(closed.is_empty(), "{closed:?}");
    }

    #[test]
    fn a_profile_that_constrains_nothing_finds_nothing() {
        let profile = AuthoringProfile::from_text(r#"{"record":"sce-authoring-profile","v":1}"#)
            .expect("reads");
        assert!(!profile.judges_statecharts());
        assert!(profile
            .judge_statechart(&statechart(""), "gate.scxml")
            .is_empty());
    }

    /// A profile that holds every kind of setting.
    const EVERYTHING: &str = r#"{
        "record":"sce-authoring-profile","v":1,"name":"owner-review",
        "interface":"closed","evidence":"anchored",
        "names":{
            "document":{"style":"snake","max_length":24},
            "state":{"style":"snake","forbidden_words":["state"],"required_prefix":"s"},
            "event":{"style":"snake","tokens":{"min":2,"max":3},"first_tokens":["door","lock"],"prefix_free":true},
            "data":{"style":"camel"}
        },
        "house_rules":[
            {"id":"H1","rule":"An event a state does not mention is ignored."},
            {"id":"H2","rule":"The initial state is the first condition the specification lists."}
        ],
        "guidance":["Ask before writing.","Write comments in the owner's language."]
    }"#;

    #[test]
    fn a_profile_that_holds_every_kind_of_setting_is_read_and_is_the_schemas() {
        let instance: serde_json::Value = serde_json::from_str(EVERYTHING).expect("JSON");
        assert!(
            violations(&instance).is_empty(),
            "{:?}",
            violations(&instance)
        );
        let profile = AuthoringProfile::from_text(EVERYTHING).expect("reads");
        assert!(profile.judges_statecharts());
        assert_eq!(
            profile.guidance(),
            [
                "Ask before writing.",
                "Write comments in the owner's language."
            ]
        );
        // Guidance alone asks nothing of a statechart: it is handed over, and
        // nothing checks it.
        let only_guidance = AuthoringProfile::from_text(
            r#"{"record":"sce-authoring-profile","v":1,"guidance":["Ask first."]}"#,
        )
        .expect("reads");
        assert!(!only_guidance.judges_statecharts());
        assert_eq!(only_guidance.guidance(), ["Ask first."]);
        assert!(only_guidance
            .judge_statechart(&statechart(""), "gate.scxml")
            .is_empty());
        // House rules are read in the owner's order, by id, and ask nothing of
        // a statechart either: they are cited, and the citation is listed.
        assert_eq!(profile.house_rule_ids(), ["H1", "H2"]);
        assert_eq!(
            profile.house_rules()[1].rule,
            "The initial state is the first condition the specification lists."
        );
        let only_rules = AuthoringProfile::from_text(
            r#"{"record":"sce-authoring-profile","v":1,"house_rules":[{"id":"H1","rule":"x"}]}"#,
        )
        .expect("reads");
        assert!(!only_rules.judges_statecharts());
    }

    /// The setting marks the records of a run that cite one of its rules, and
    /// only `sce:assumed` ones: a question is not a standing answer.
    #[test]
    fn a_profile_marks_the_records_that_cite_its_house_rules() {
        let profile = AuthoringProfile::from_text(EVERYTHING).expect("reads");
        let model = crate::parser::SCXMLParser::new()
            .parse_string(
                r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                          xmlns:sce="http://sce.dev/ext" version="1.0"
                          name="gate" initial="idle" datamodel="ecmascript">
                     <state id="idle" sce:assumed="H1" sce:assumed-reason="the spec lists no other events"/>
                     <state id="other" sce:assumed="retry-count" sce:assumed-reason="x"/>
                     <state id="asked" sce:unresolved="H2" sce:unresolved-reason="not answered"/>
                   </scxml>"#,
                "gate",
            )
            .expect("parses");
        let mut records = crate::unresolved_check::unresolved_records(&model);
        profile.cite_house_rules(&mut records);
        let cited: Vec<(&str, bool)> = records
            .iter()
            .map(|r| (r.id.as_str(), r.house_rule))
            .collect();
        assert_eq!(
            cited,
            [("H1", true), ("retry-count", false), ("H2", false)],
            "an assumed value that names a rule cites it; one that does not, and a question \
             that names a rule's id, do not"
        );
    }

    #[test]
    fn a_house_rule_is_one_rule_by_one_id() {
        for (rules, said) in [
            (
                r#"[{"id":"H1","rule":"a"},{"id":"H1","rule":"b"}]"#,
                "listed twice",
            ),
            (r#"[{"id":"H 1","rule":"a"}]"#, "one token"),
        ] {
            let text =
                format!(r#"{{"record":"sce-authoring-profile","v":1,"house_rules":{rules}}}"#);
            let refused = AuthoringProfile::from_text(&text).expect_err(&text);
            assert_eq!(refused.kind, ProfileFault::InvalidShape);
            assert!(refused.detail.contains(said), "{refused}");
        }
    }

    /// Settings the schema refuses, one thing changed in each, and the reader
    /// refuses every one of them too: the two are one contract. What only the
    /// reader can say — a range that runs backwards, a word listed twice in
    /// two cases — is not in this table.
    #[test]
    fn the_reader_refuses_what_the_schema_refuses() {
        let control: serde_json::Value = serde_json::from_str(EVERYTHING).expect("JSON");
        assert!(
            violations(&control).is_empty(),
            "the control has to be valid"
        );
        let changes: [(&str, serde_json::Value); 22] = [
            ("house_rules", serde_json::json!([])),
            (
                "house_rules",
                serde_json::json!([{"id": "H 1", "rule": "x"}]),
            ),
            ("house_rules", serde_json::json!([{"id": "", "rule": "x"}])),
            ("house_rules", serde_json::json!([{"id": "H1", "rule": ""}])),
            ("house_rules", serde_json::json!([{"id": "H1"}])),
            ("names", serde_json::json!({})),
            ("names", serde_json::json!({"state": {}})),
            ("names", serde_json::json!({"state": {"style": "title"}})),
            ("names", serde_json::json!({"state": {"colour": "red"}})),
            ("names", serde_json::json!({"state": {"max_length": 0}})),
            (
                "names",
                serde_json::json!({"state": {"forbidden_words": []}}),
            ),
            (
                "names",
                serde_json::json!({"state": {"forbidden_words": ["a b"]}}),
            ),
            (
                "names",
                serde_json::json!({"state": {"required_prefix": ""}}),
            ),
            ("names", serde_json::json!({"event": {"tokens": {}}})),
            (
                "names",
                serde_json::json!({"event": {"tokens": {"min": 0}}}),
            ),
            ("names", serde_json::json!({"event": {"first_tokens": []}})),
            (
                "names",
                serde_json::json!({"event": {"first_tokens": ["a", "a"]}}),
            ),
            (
                "names",
                serde_json::json!({"transition": {"style": "snake"}}),
            ),
            ("evidence", serde_json::json!("cited")),
            ("guidance", serde_json::json!([])),
            ("guidance", serde_json::json!([""])),
            ("guidance", serde_json::json!("ask first")),
        ];
        for (setting, value) in changes {
            let mut changed = control.clone();
            changed[setting] = value.clone();
            assert!(
                !violations(&changed).is_empty(),
                "the schema accepted {setting} = {value}"
            );
            let text = changed.to_string();
            let refused = AuthoringProfile::from_text(&text);
            assert!(
                refused.is_err(),
                "the reader accepted {setting} = {value}, which the schema refuses"
            );
        }
    }

    #[test]
    fn what_only_the_reader_can_say_is_refused_by_it() {
        for (setting, said) in [
            (r#"{"event":{"tokens":{"min":3,"max":2}}}"#, "at most 2"),
            (
                r#"{"state":{"forbidden_words":["Open","open"]}}"#,
                "lists a word twice",
            ),
        ] {
            let text = format!(r#"{{"record":"sce-authoring-profile","v":1,"names":{setting}}}"#);
            let refused = AuthoringProfile::from_text(&text).expect_err(&text);
            assert_eq!(refused.kind, ProfileFault::InvalidShape);
            assert!(refused.detail.contains(said), "{refused}");
        }
    }

    /// One statechart that breaks the interface, every kind of name rule and
    /// the evidence rule, so the order the findings come in is pinned: the
    /// interface, then the names by class, then the evidence.
    fn crooked() -> SCXMLModel {
        crate::parser::SCXMLParser::new()
            .parse_string(
                r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                          xmlns:sce="http://sce.dev/ext" version="1.0"
                          name="GateControllerForTheMainEntrance" initial="idle"
                          datamodel="ecmascript">
                     <sce:kind-basis>
                       <sce:evidence provenance="SPEC@2#4.1">the gate opens on request</sce:evidence>
                       <sce:evidence>the gate closes after a delay</sce:evidence>
                       <sce:rejected kind="timer">it is not periodic</sce:rejected>
                     </sce:kind-basis>
                     <datamodel><data id="open_count" expr="0"/></datamodel>
                     <state id="idle">
                       <transition event="gate.open" target="OpenState"/>
                     </state>
                     <state id="OpenState">
                       <transition event="gate.open.now" target="idle"/>
                     </state>
                   </scxml>"#,
                "gate",
            )
            .expect("parses")
    }

    fn codes_of(findings: &[Located<ForgeError>]) -> Vec<&'static str> {
        use crate::forge::diagnostic::ToDiagnostics;
        findings
            .iter()
            .map(|found| found.error.to_diagnostics()[0].code.as_str())
            .collect()
    }

    #[test]
    fn a_crooked_statechart_is_refused_in_the_order_the_settings_are_described() {
        let profile = AuthoringProfile::from_text(EVERYTHING).expect("reads");
        let found = profile.judge_statechart(&crooked(), "gate.scxml");
        assert_eq!(
            codes_of(&found),
            [
                // interface
                "profile/interface-not-closed",
                // document: the name is neither snake_case nor at most 24 long
                "profile/name-style",
                "profile/name-limit",
                // states: OpenState is not snake_case, is made of `state`,
                // and does not begin with `s`; idle does not begin with `s`
                "profile/name-limit",
                "profile/name-style",
                "profile/name-limit",
                "profile/name-limit",
                // events: gate.open is a prefix of gate.open.now, and the
                // first token gate is not one the profile lists
                "profile/event-structure",
                "profile/event-structure",
                "profile/event-prefix-of-another",
                // data: open_count is snake_case where camel was asked for
                "profile/name-style",
                // the evidence with no anchor
                "profile/evidence-unanchored",
            ],
            "{found:#?}"
        );
    }

    #[test]
    fn a_statechart_that_keeps_to_every_setting_is_not_refused() {
        let profile = AuthoringProfile::from_text(EVERYTHING).expect("reads");
        let kept = crate::parser::SCXMLParser::new()
            .parse_string(
                r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                          xmlns:sce="http://sce.dev/ext" version="1.0"
                          name="gate" initial="s_idle" datamodel="ecmascript"
                          sce:interface="closed">
                     <sce:kind-basis>
                       <sce:evidence provenance="SPEC@2#4.1">the gate opens on request</sce:evidence>
                       <sce:rejected kind="timer">it is not periodic</sce:rejected>
                     </sce:kind-basis>
                     <datamodel><data id="openCount" expr="0"/></datamodel>
                     <state id="s_idle">
                       <transition event="door.open" target="s_up"/>
                     </state>
                     <state id="s_up">
                       <transition event="door.shut.now" target="s_idle"/>
                     </state>
                   </scxml>"#,
                "gate",
            )
            .expect("parses");
        let found = profile.judge_statechart(&kept, "gate.scxml");
        assert!(found.is_empty(), "{found:#?}");
    }

    #[test]
    fn a_profile_without_a_setting_does_not_judge_what_it_asks_nothing_about() {
        let profile = AuthoringProfile::from_text(
            r#"{"record":"sce-authoring-profile","v":1,"names":{"data":{"style":"camel"}}}"#,
        )
        .expect("reads");
        let found = profile.judge_statechart(&crooked(), "gate.scxml");
        assert_eq!(codes_of(&found), ["profile/name-style"], "{found:#?}");
    }
}
