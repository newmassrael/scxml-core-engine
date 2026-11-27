// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! What a person accepted, pinned so that its authority can lapse.
//!
//! Requirement-closure RFC §8.3: an acceptance is of a `(manifest revision,
//! variant, document)` triple, and if any of the three moves, the authority
//! lapses and a person must accept again. Without the pin an acceptance is
//! a golden file — it goes on defending old behaviour after the
//! specification or the design has moved under it.
//!
//! # What is pinned, and why each part
//!
//! ```text
//!   manifest   doc_id + rev, and its bytes    a requirement added under an
//!                                             unchanged rev still moves it
//!   words      the bytes of the sidecar,      a manifest holds coordinates
//!              when the caller gave it        only, so two lists of one shape
//!                                             and different sentences share
//!                                             one manifest digest
//!   variant    the name the acceptance was    accepting base says nothing
//!              given for                      about base + tls (RFC §5.2d)
//!   inputs     every file the parse READ,     the entry document is not the
//!              each by sha256                 whole design: an <xi:include>d
//!                                             fragment is part of it
//! ```
//!
//! ⚠ The inputs are the parser's own answer,
//! [`SCXMLParser::preprocessor_deps`], and never a walk of the directory. A
//! walk decides the set a second time, and deciding a set twice is how a
//! member ends up outside one of the answers — `forge::drift`'s depfile
//! writer lost the shared macro family and then the import closure that
//! way. Re-checking parses again for the same reason: a change of
//! composition is a change of design even when every file that was pinned
//! still has its old bytes.
//!
//! # Bytes, not a model
//!
//! A pin over the parsed model would survive an edit that changes nothing a
//! person saw — a comment, a reflowed attribute — but only by trusting that
//! the model's serialisation is total over behaviour, which nothing here
//! checks. The byte pin lapses more often than the behaviour moves, and that
//! is the direction to be wrong in: a lapse costs a person a second look,
//! and a false hold costs the acceptance its meaning.
//!
//! # Paths are relative to a root the caller names
//!
//! A record is committed and read on other checkouts, so an absolute path
//! would lapse it everywhere but the machine that took it. Every pinned path
//! is stored relative to the root given when the record was taken, and an
//! input outside that root is refused rather than recorded as a path that
//! silently stops meaning anything elsewhere.
//!
//! # What the design was authored from
//!
//! ```text
//!   authored_from   the specification files and    an author asked again for
//!                   the owner's decision record,   the same specification is
//!                   each by sha256                 handed this design instead
//!                                                  of a new draft
//! ```
//!
//! Optional, and absent from a record taken without it, which then keeps
//! the bytes it always had. The model that wrote the draft runs in the
//! owner's client, where nothing makes two drafts equal, so the only way a
//! second request with the same inputs gets the same design is to be given
//! the one already accepted — and "the same inputs" is a question about
//! these files. A record pins them so that a revised specification lapses
//! the acceptance as surely as an edited document does, and so that
//! [`AcceptanceRecord::recheck_for`] can say whether a specification it is
//! asked about is the one this design was authored from.
//!
//! The authoring profile ([`crate::authoring_profile`]) is the third of them,
//! under [`crate::acceptance_record::SourceRole::Profile`], and is not an
//! input to the prose: it is what the finished design was judged against. It
//! is here because an acceptance is the owner's statement that THIS design is
//! what they accept, and a design accepted under one profile — say, one that
//! requires a closed interface — is not the answer for a profile that asks
//! something else. The same specification and the same decisions under another
//! profile are a different request, so they get a different design and a
//! different acceptance, and the digest is the whole of what makes them
//! different.
//!
//! The scenario set ([`crate::scenario_set`]) is the fourth, under
//! [`crate::acceptance_record::SourceRole::Examples`], and like the profile it
//! is held to and not authored from: the examples whose passing closed a
//! requirement (`scenario-passed`, [`crate::scenario_closure`]) are part of what
//! the owner accepted the design WITH. A set edited afterwards is another set,
//! and an acceptance that went on answering for it would be reading the old
//! examples' passes as the new ones'. The record pins the set's BYTES and says
//! nothing of how its scenarios came out: that is a run's, on an engine, and is
//! asked again by `requirements --scenarios --trace`.
//!
//! # Which rules of the profile the design applied
//!
//! ```text
//!   applied_rules   each house rule the design    "the profile changed" says
//!                   cited, as the profile worded  nothing a reader can act on;
//!                   it, and at how many places    "H2 changed, applied at 3
//!                                                 places" does
//! ```
//!
//! A design accepted under a profile cites house rules (`sce:assumed="H1"`) as
//! the owner's standing answers, and the profile is pinned by its bytes. When
//! those bytes move the lapse says the profile did, which is true of every rule
//! in it and useful for none. The record keeps the TEXT of each rule the design
//! applied, as it read when the design was accepted, so that a profile that
//! changes afterwards can be compared rule by rule ([`Lapse::Rule`]): this rule
//! now says something else, and this design relied on it at this many places; or
//! this rule is gone. It also puts the content a reader needs in the record
//! itself: a developer reading an acceptance need not ask for the profile to
//! learn what `H1` meant. Optional, and omitted when the design applied none, so
//! a record that never needed it keeps the bytes it always had.
//!
//! # What was still open when a person accepted
//!
//! ```text
//!   open_at_acceptance   the questions, assumed    a reader of the record
//!                        values, needed parent     sees what the owner
//!                        and host processor the    accepted WITH, in the
//!                        design left to a person   words the report used
//! ```
//!
//! The product accepts a design that leaves a question open — it is valid
//! SCXML, and a draft in progress is meant to — and an acceptance that did
//! not say so read, later, as though the design had been finished. Optional
//! and omitted when empty, so a record for a finished design keeps the bytes
//! it always had. It states and does not enforce: accepting with an open
//! question is the owner's decision, and this only keeps the record honest
//! about it.
//!
//! # Who stated the acceptance
//!
//! ```text
//!   channel   direct | relayed     which surface said the owner accepted
//! ```
//!
//! An acceptance is a person's act, and the product is never in the room: the
//! surface that calls `accept` states that the owner accepted, and nothing here
//! can see whether they did. Two surfaces exist, and they stand differently
//! behind the same record. The workbench application shows the owner the design
//! and takes the acceptance from a button the owner pressed, so it says `direct`.
//! An AI client reports that the owner accepted in conversation, so it says
//! `relayed`, which is a claim and reads as one. A record that states neither
//! (every record taken before this field, and any taken from a command line that
//! did not say) says nothing about it.
//!
//! ⚠ It is what the CALLER stated, recorded so a reader of the acceptance sees
//! whose word it rests on. The product does not verify it, and a record read as
//! proof that the owner pressed anything would claim what nothing here knows.
//! Not compared on a re-check: it says how the acceptance stood when it was taken,
//! and it cannot lapse. Optional and omitted when not stated, so a record that
//! never needed it keeps the bytes it always had.
//!
//! # What each requirement rested on
//!
//! ```text
//!   evidence    per requirement id, the digests   "bytes moved" says nothing about
//!               of the rows the report showed     WHICH requirement's evidence did
//!               the owner for it
//!   unclaimed   the digests of the rows that      the `(none)` block; an unannotated
//!               carry no requirement id           row a cited node depends on is here
//!               themselves                        too, and in the fragment that reaches it
//!   evidence_rule  the rule the digests above     two digests made differently are two
//!               were made under (`EVIDENCE_RULE`) numbers; `acceptance-delta` refuses
//!                                                 a record made under another rule
//!   succeeds    the record this one replaced,     a chain of acceptances an auditor
//!               by path and sha256                can walk
//! ```
//!
//! `evidence` is what a person was shown, not a second reading of the design:
//! the rows are the ones [`crate::acceptance_report`] puts in front of the owner
//! (a statechart's [`crate::acceptance_report::fragment`] of each cited
//! requirement, selected from [`crate::transition_table`]; a forge document's
//! [`crate::forge::review_table`] rows that claim it), so the report and the
//! record cannot disagree about what depends on what.
//!
//! A digest covers a row WITHOUT its `source` column (the claim is the key, not
//! the content) and WITH its position: transition order inside a state is
//! behaviour in SCXML, so a digest that left position out would call two
//! differently ordered machines equal. The price is the direction this module
//! always chooses: a transition inserted before others in the same state moves
//! the digests of the ones after it, and the owner is asked for a second look at
//! a requirement that did not need one.
//!
//! ⚠ Like `open_at_acceptance` it states and does not enforce, and it cannot
//! lapse: the design is pinned by its bytes, and an edit to it is a lapse
//! whatever the digests say. What the digests are for is the NEXT acceptance,
//! which can be offered as a difference. They are the product's closure of what
//! a requirement depends on, not a proof of its behaviour: a dependency the
//! closure does not follow is not in them. Optional and omitted when empty, so a
//! record taken before they existed keeps its bytes.
//!
//! # A design of several documents
//!
//! ```text
//!   documents      every document the caller     the design is a set: a claim made
//!                  named, `document` first,       in an imported algorithm or in a
//!                  when there is more than one    second statechart is the design's
//!   with_imports   true when the set also holds   the set a build of it generates,
//!                  every document they reach      re-walked on every re-check
//! ```
//!
//! A design is often more than one document ([`crate::design_set`]), and an
//! acceptance of it is taken once, over the same set `requirements` measures
//! and `acceptance-report` shows: the claims of every document are credited,
//! every file any of them reads is in `inputs`, and a re-check reads the SAME
//! set again, so an edit to any input of the set lapses the acceptance, and a
//! document the set reaches now and did not then is `added`.
//!
//! With several documents, each row's place in `evidence` and `unclaimed`
//! names its document, as a node path of `requirements` does
//! (`front.scxml#states.idle`): the digest of such a row is taken over the
//! document's path, relative to the root, and the row's own digest, since
//! `states.idle` is a place in every statechart and the same row in two
//! documents is two rows. The open matters are led by their document for the
//! same reason. A set that turns out to hold one document is that document
//! alone, row for row.
//!
//! ⚠ Both fields are optional and omitted for a design of one document read
//! alone, so a record of one document keeps the bytes it always had, and means
//! what it always meant: the imports of its document are pinned as inputs, and
//! a claim made in one of them is not credited, as it never was.
//!
//! # Deliberately not `Deserialize`
//!
//! [`AcceptanceRecord`] is obtained from [`AcceptanceRecord::take`] or
//! [`AcceptanceRecord::from_json`] and no other way, for the reason
//! [`crate::requirement_manifest`] gives for its own manifest: a derive on
//! the public type lets a caller load bytes the format check refuses.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::design_set::DesignSet;
use crate::generator_witness::{hex_encode, sha256_bytes};
use crate::parser::SCXMLParser;
use crate::requirement_manifest::{ManifestError, RequirementManifest};
use crate::requirement_sidecar::{RequirementSidecar, SidecarError};

/// The value a record's `record` field holds. A JSON file naming any other
/// kind is refused before its pins are read.
pub const RECORD_KIND: &str = "sce-acceptance-record";

/// The format version. Moves only when a pinned field changes meaning.
pub const RECORD_VERSION: u32 = 1;

/// The record's stability, as `SCE_WIRE_CONTRACTS.md` states it. The record
/// has no schema file, so the registry's row and this constant are the two
/// places the status lives, and `cli_acceptance_record_wire` holds them
/// together.
pub const ACCEPTANCE_RECORD_STATUS: &str = "pre-release";

/// What an [`authored_from`](AcceptanceRecord::authored_from) file is to the
/// design.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceRole {
    /// The prose the design was written from; there may be several files.
    Specification,
    /// The owner's answers to what the specification left open; one at most.
    Decisions,
    /// The authoring profile the design was held to
    /// ([`crate::authoring_profile`]); one at most.
    Profile,
    /// The scenario set whose examples the design was held to
    /// ([`crate::scenario_set`]); one at most.
    Examples,
}

impl SourceRole {
    /// Every role, in the order a record lists them.
    pub const ALL: [SourceRole; 4] = [
        SourceRole::Specification,
        SourceRole::Decisions,
        SourceRole::Profile,
        SourceRole::Examples,
    ];

    /// How the design stands to a file of this role, for a sentence that
    /// says it: a design is written FROM a specification and the owner's
    /// answers, and HELD TO a profile — the profile was not an input to the
    /// prose, it is what the finished design was judged against.
    fn relation(self) -> &'static str {
        match self {
            SourceRole::Specification | SourceRole::Decisions => "authored from",
            SourceRole::Profile | SourceRole::Examples => "held to",
        }
    }
}

impl fmt::Display for SourceRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            SourceRole::Specification => "specification",
            SourceRole::Decisions => "decision record",
            SourceRole::Profile => "authoring profile",
            SourceRole::Examples => "scenario set",
        })
    }
}

/// Which surface stated that the owner accepted. See the module docs: it is the
/// caller's statement, recorded and never verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    /// The surface the owner acts on took the acceptance from their own button.
    Direct,
    /// A client reported that the owner accepted, in a conversation the product
    /// did not see.
    Relayed,
}

impl Channel {
    /// The wire word, and what the CLI's `--channel` takes.
    pub fn as_str(self) -> &'static str {
        match self {
            Channel::Direct => "direct",
            Channel::Relayed => "relayed",
        }
    }
}

impl std::str::FromStr for Channel {
    type Err = String;

    fn from_str(word: &str) -> Result<Self, Self::Err> {
        match word {
            "direct" => Ok(Channel::Direct),
            "relayed" => Ok(Channel::Relayed),
            other => Err(format!(
                "`{other}` is not a channel; the channels are `direct` and `relayed`"
            )),
        }
    }
}

/// One house rule the design applied, as the profile worded it when the design
/// was accepted, and at how many places the design cites it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppliedRule {
    /// The id the design writes (`sce:assumed="H1"`).
    pub id: String,
    /// What the profile said the rule was, verbatim.
    pub rule: String,
    /// How many markers of the design cite it. Applying one rule twice is two
    /// places for the owner to confirm and the rule is one.
    pub places: usize,
    /// The owner's own words the rule was made from, as the profile held them
    /// when the design was accepted. Absent where the profile carried none, and
    /// then nothing here knows where the rule came from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quote: Option<String>,
    /// Who vouched for the rule when the design was accepted. Recorded so a
    /// reader of the acceptance sees on what authority the design applied it,
    /// and NOT compared afterwards: it says how the rule stood then, and the
    /// lapse of a rule is about what it SAYS.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirmation: Option<crate::authoring_profile::Confirmation>,
}

/// One file the design was authored from.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePin {
    pub role: SourceRole,
    /// Relative to the record's root.
    pub path: String,
    pub sha256: String,
}

/// The manifest an acceptance was measured against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestPin {
    /// Relative to the record's root.
    pub path: String,
    pub doc_id: String,
    pub rev: String,
    /// Over the manifest's bytes. `rev` alone is what the specification's
    /// owner says moved; the bytes are what did.
    pub sha256: String,
    /// Over the bytes of the sidecar, the sentences behind the manifest's ids.
    ///
    /// A manifest is coordinates only (an id, a section, a modality), so two
    /// lists of one shape and different sentences have one `sha256`, and a
    /// revision of one reads as a revision of the other. The sentences are
    /// never copied into the record, only their digest, so the record can be
    /// committed where the sidecar is not. Absent when the acceptance was
    /// taken without the sidecar: what the owner accepted is then a list the
    /// record cannot tell from another of its shape, and a revision of it
    /// cannot be judged against its words. Left out of the JSON when absent,
    /// so a record taken without it is the bytes it was before this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sidecar_sha256: Option<String>,
}

/// One file the design was read from.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputPin {
    /// Relative to the record's root.
    pub path: String,
    pub sha256: String,
}

/// The record an acceptance replaced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PredecessorPin {
    /// Relative to the record's root.
    pub path: String,
    /// Over the previous record's bytes.
    pub sha256: String,
}

/// What became of one requirement's evidence between a record and a design
/// ([`AcceptanceRecord::evidence_delta`]). Serialised for `acceptance-delta`, one
/// line a requirement, with `evidence` naming the variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "evidence", rename_all = "kebab-case")]
pub enum RequirementChange {
    /// The design shows the same rows for it as the record kept.
    Unchanged,
    /// Some row differs. `moved` is where, in the design now, the rows the record
    /// lacks are; `gone` is how many recorded rows no row of the design matches,
    /// which can be counted and not located, since the row that held them is not
    /// there to point at.
    Changed { moved: Vec<String>, gone: usize },
    /// Cited by a node now and with no evidence in the record. `at` is where.
    New { at: Vec<String> },
    /// Evidence in the record, and no node cites it now.
    Dropped { gone: usize },
}

/// What moved in a design since a record was taken: the evidence per requirement,
/// and the rows that claim nothing. See [`AcceptanceRecord::evidence_delta`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceDelta {
    /// Every requirement either side cites. One a manifest lists and neither side
    /// cites has no entry: `requirements` already calls that `missing`.
    pub requirements: BTreeMap<String, RequirementChange>,
    /// Where, in the design now, the rows that claim nothing and are not in the
    /// record are.
    pub unclaimed_added: Vec<String>,
    /// How many recorded rows that claimed nothing no row of the design matches.
    pub unclaimed_gone: usize,
}

/// An acceptance, pinned. See the module docs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptanceRecord {
    /// The name the acceptance was given for. Compared verbatim.
    pub variant: String,
    /// The entry document, relative to the root: the first the caller named.
    pub document: String,
    /// Every document the caller named, relative to the root, in the order
    /// named, `document` first. Empty when the caller named one document, which
    /// is then `document` alone. See "A design of several documents" in the
    /// module docs.
    pub documents: Vec<String>,
    /// Whether the design is the named documents and every document they reach
    /// ([`crate::design_set`]), read again so on every re-check.
    pub with_imports: bool,
    pub manifest: ManifestPin,
    /// Every file the parse of each document of the design read, every document
    /// itself included, sorted by path.
    pub inputs: Vec<InputPin>,
    /// The specification, the decision record and the authoring profile the
    /// design was authored from and held to, sorted by role and path. Empty when the record was taken
    /// without them — see the module docs.
    pub authored_from: Vec<SourcePin>,
    /// The house rules of the pinned authoring profile that the design cited,
    /// each as the profile worded it and with the number of places that cite
    /// it, sorted by id. Empty when the design cited none — and always empty
    /// for a record taken without a profile, which cannot tell a house rule
    /// from a value chosen without an answer. See the module docs.
    pub applied_rules: Vec<AppliedRule>,
    /// What the design left to a person when it was accepted: the open
    /// questions, the values chosen without the specification, a parent it
    /// needs, a processor the host must serve ([`crate::open_matters`]).
    /// Empty when it left nothing.
    ///
    /// ⚠ A statement of what a person accepted WITH, not a condition the
    /// record enforces. Accepting a design that leaves a question open is
    /// the owner's decision to make; what this refuses is a record that
    /// reads as though they had not. It cannot lapse: the design that was
    /// open is pinned by its bytes, and a design whose open matters moved
    /// has moved.
    pub open_at_acceptance: Vec<crate::open_matters::OpenMatter>,
    /// For each requirement the design cites, the sorted sha256 digests of the
    /// rows the report showed the owner for it ("What each requirement rested
    /// on" in the module docs). A requirement the design does not cite has no
    /// key: no evidence is not an empty fragment.
    ///
    /// ⚠ A statement of what a person was shown, not a condition the record
    /// enforces and not a proof of behaviour. It cannot lapse.
    pub evidence: BTreeMap<String, Vec<String>>,
    /// The sorted digests of the rows that claim no requirement.
    pub unclaimed: Vec<String>,
    /// The rule `evidence` and `unclaimed` were made under ([`EVIDENCE_RULE`]), when
    /// they state anything. `None` for a record taken before the rule was named, which
    /// is rule 1; and for one that states no evidence, which has no rule to name.
    ///
    /// ⚠ A digest is only comparable with a digest made the same way. The first rule
    /// left a transition's `type` out of its row, so a record of an `internal`
    /// transition holds the digest an `external` one has now: compared with the rule
    /// of today it would call a change from one to the other `unchanged`
    /// ([`AcceptanceRecord::evidence_delta`] refuses it instead).
    pub evidence_rule: Option<u32>,
    /// The record this one replaced, when the caller said.
    pub succeeds: Option<PredecessorPin>,
    /// Which surface stated the acceptance, when it said. See the module docs.
    pub channel: Option<Channel>,
}

/// The rule the digests of `evidence` and `unclaimed` are made under.
///
/// Moves whenever what a digest covers moves: a row gaining a column or a cell
/// gaining a value (rule 2: a transition's `type` is in its `to` cell when it is
/// not the default). A record made under another rule is not compared by
/// `acceptance-delta`, because two digests made differently are two numbers and
/// their equality says nothing; the acceptance itself still holds or lapses by
/// its bytes, as it always did.
pub const EVIDENCE_RULE: u32 = 2;

/// The record exactly as JSON spells it. Private — see the module docs.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordWire {
    record: String,
    v: u32,
    variant: String,
    document: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    documents: Vec<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    with_imports: bool,
    manifest: ManifestPin,
    inputs: Vec<InputPin>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    authored_from: Vec<SourcePin>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    applied_rules: Vec<AppliedRule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    open_at_acceptance: Vec<crate::open_matters::OpenMatter>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    evidence: BTreeMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    unclaimed: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    evidence_rule: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    succeeds: Option<PredecessorPin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    channel: Option<Channel>,
}

/// One way what was accepted is no longer what is there.
///
/// Serialised for `acceptance-impact`, which reports many records at once and
/// owes a reader the lapse as data (`kind`, and the fields below under their own
/// names) beside the sentence, so no consumer has to read a rule's id or a moved
/// file out of prose. `acceptance-check` keeps saying it as its sentences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Lapse {
    /// The record was taken for a different variant than the one asked
    /// about.
    Variant { recorded: String, asked: String },
    /// The manifest's bytes moved. `current_rev` is `None` when the
    /// manifest no longer loads, which is a lapse and not a failure to
    /// check: the record still says what was accepted.
    Manifest {
        recorded_rev: String,
        current_rev: Option<String>,
        recorded_sha256: String,
        current_sha256: String,
    },
    /// A pinned file is gone.
    Missing { path: String },
    /// A pinned file's bytes moved.
    Moved {
        path: String,
        recorded_sha256: String,
        current_sha256: String,
    },
    /// The parse now reads a file the acceptance never saw.
    Added { path: String },
    /// A document of the design no longer parses, or names a document that
    /// cannot be read, so which files make up the design cannot be said.
    Unparseable { path: String, detail: String },
    /// A file the design was authored from is gone (`current_sha256` is
    /// `None`) or its bytes moved: the specification or the owner's answers
    /// changed, and the design has not been looked at since.
    Source {
        role: SourceRole,
        path: String,
        recorded_sha256: String,
        current_sha256: Option<String>,
    },
    /// A house rule the design applied no longer reads as it did: the
    /// profile's bytes moved (`Source` says so) and, read now, the rule has
    /// other words (`current` is them) or is gone (`current` is `None`). Only
    /// said when the profile still loads; a profile that does not is `Source`'s
    /// to report, and a rule it cannot be read for is not said to be unchanged.
    Rule {
        id: String,
        places: usize,
        recorded: String,
        current: Option<String>,
    },
    /// The files a caller asked about are not the ones the design was
    /// authored from — compared by content, so the same specification under
    /// another name still matches. `pinned` is empty when the record does
    /// not say what it was authored from.
    NotAuthoredFrom {
        role: SourceRole,
        pinned: Vec<String>,
        asked: Vec<String>,
    },
}

impl fmt::Display for Lapse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Lapse::Variant { recorded, asked } => write!(
                f,
                "the acceptance was taken for variant `{recorded}`, not `{asked}`"
            ),
            Lapse::Manifest {
                recorded_rev,
                current_rev,
                ..
            } => match current_rev {
                Some(current) if current == recorded_rev => write!(
                    f,
                    "the manifest changed under an unchanged revision `{recorded_rev}`"
                ),
                Some(current) => write!(
                    f,
                    "the manifest moved from revision `{recorded_rev}` to `{current}`"
                ),
                None => write!(
                    f,
                    "the manifest accepted at revision `{recorded_rev}` no longer loads"
                ),
            },
            Lapse::Missing { path } => write!(f, "{path}: pinned, and no longer there"),
            Lapse::Moved { path, .. } => write!(f, "{path}: changed since it was accepted"),
            Lapse::Added { path } => {
                write!(f, "{path}: now part of the design, and never accepted")
            }
            Lapse::Unparseable { path, detail } => {
                write!(f, "{path}: no longer parses ({detail})")
            }
            Lapse::Source {
                role,
                path,
                current_sha256,
                ..
            } => match current_sha256 {
                None => write!(
                    f,
                    "{path}: the {role} this design was {} is no longer there",
                    role.relation()
                ),
                Some(_) => write!(
                    f,
                    "{path}: the {role} this design was {} changed since it was accepted",
                    role.relation()
                ),
            },
            Lapse::Rule {
                id,
                places,
                recorded,
                current,
            } => {
                let at = if *places == 1 {
                    "1 place".to_string()
                } else {
                    format!("{places} places")
                };
                match current {
                    Some(now) => write!(
                        f,
                        "house rule `{id}`, which the design applied at {at}, now says \
                         {now:?}; it said {recorded:?} when the design was accepted"
                    ),
                    None => write!(
                        f,
                        "house rule `{id}`, which the design applied at {at} and which \
                         said {recorded:?}, is no longer in the profile"
                    ),
                }
            }
            Lapse::NotAuthoredFrom {
                role,
                pinned,
                asked,
            } => {
                let asked = if asked.is_empty() {
                    "none was given".to_string()
                } else {
                    format!("the one asked about ({}) differs", asked.join(", "))
                };
                if pinned.is_empty() {
                    write!(
                        f,
                        "the record does not say which {role} the design was {}, so it \
                         cannot answer for this one: {asked}",
                        role.relation()
                    )
                } else {
                    write!(
                        f,
                        "the design was {} the {role} {}; {asked}",
                        role.relation(),
                        pinned.join(", ")
                    )
                }
            }
        }
    }
}

/// Why a record could not be taken or read.
#[derive(Debug)]
pub enum RecordError {
    Read {
        path: String,
        source: std::io::Error,
    },
    /// The document refused to parse when the record was being taken. A
    /// record of a design that cannot be read would pin nothing.
    Parse {
        path: String,
        detail: String,
    },
    Manifest(ManifestError),
    /// A file the design depends on lies outside the root, so no path
    /// relative to it can name the file.
    OutsideRoot {
        path: String,
        root: String,
    },
    /// A variant name that cannot be compared verbatim.
    Variant {
        value: String,
    },
    /// Bytes that are not a record this module wrote.
    Format {
        detail: String,
    },
    /// The files named as what the design was authored from cannot be
    /// pinned as given: a file named twice, or more than one decision
    /// record.
    Source {
        detail: String,
    },
    /// The sidecar given as the words behind the manifest is not the
    /// manifest's own, or cannot be read. The sidecar loader's refusal,
    /// unchanged, so the same sidecar is one diagnostic whichever door read it.
    Words(SidecarError),
    /// A draft of one document was offered for comparison with a record of a
    /// design of several, or of one read with what it imports: the draft is
    /// not the design the record names.
    OfSeveralDocuments {
        document: String,
    },
}

impl fmt::Display for RecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecordError::Read { path, source } => write!(f, "{path}: {source}"),
            RecordError::Parse { path, detail } => {
                write!(f, "{path}: the document does not parse: {detail}")
            }
            RecordError::Manifest(inner) => write!(f, "{inner}"),
            RecordError::OutsideRoot { path, root } => write!(
                f,
                "{path} lies outside {root}; an acceptance record names its files \
                 relative to one root, so take it from a root that contains the design"
            ),
            RecordError::Variant { value } => write!(
                f,
                "variant `{value}` is not a name: it must be non-empty and hold no \
                 whitespace, because it is compared verbatim"
            ),
            RecordError::Format { detail } => {
                write!(f, "not an acceptance record: {detail}")
            }
            RecordError::Source { detail } => write!(f, "{detail}"),
            RecordError::Words(inner) => write!(f, "{inner}"),
            RecordError::OfSeveralDocuments { document } => write!(
                f,
                "the record is of a design read as a set, `{document}` first (several \
                 documents, or one with what it imports), and a draft of one document read \
                 alone is not that design; compare the record with its own documents under \
                 the root"
            ),
        }
    }
}

impl std::error::Error for RecordError {}

impl RecordError {
    /// Which refusal this is, in words that do not carry a path — see
    /// [`crate::requirement_manifest::ManifestError::kind`] for why. A
    /// manifest that would not load keeps the manifest's own kind, so the
    /// same broken manifest is one diagnostic whichever door read it.
    pub fn kind(&self) -> &'static str {
        match self {
            RecordError::Read { .. } => "read",
            RecordError::Parse { .. } => "document-unparseable",
            RecordError::Manifest(inner) => inner.kind(),
            RecordError::OutsideRoot { .. } => "outside-root",
            RecordError::Variant { .. } => "variant",
            RecordError::Format { .. } => "format",
            RecordError::Source { .. } => "source",
            RecordError::Words(inner) => inner.kind(),
            RecordError::OfSeveralDocuments { .. } => "design-of-several-documents",
        }
    }
}

impl AcceptanceRecord {
    /// Pin the design at `document`, measured against `manifest`, as the
    /// acceptance of `variant`.
    ///
    /// Every path is recorded relative to `root`.
    pub fn take(
        root: &Path,
        document: &Path,
        manifest: &Path,
        variant: &str,
    ) -> Result<Self, RecordError> {
        Self::take_authored(root, document, manifest, variant, &[])
    }

    /// [`Self::take`], also pinning the files the design was authored from
    /// — see the module docs. At most one of them may be a decision record,
    /// and at most one an authoring profile.
    pub fn take_authored(
        root: &Path,
        document: &Path,
        manifest: &Path,
        variant: &str,
        sources: &[(SourceRole, &Path)],
    ) -> Result<Self, RecordError> {
        Self::take_design(
            root,
            &DesignSet::one(document.display().to_string()),
            manifest,
            variant,
            sources,
        )
    }

    /// [`Self::take_authored`] for a design of one document or several
    /// ([`crate::design_set`]): every document of `design` is read, a claim is
    /// credited wherever in it it is made, and every file any of them reads is
    /// pinned. See "A design of several documents" in the module docs.
    ///
    /// [`DesignSet::one`] is the design `take_authored` takes, and the record of
    /// it is the record `take_authored` writes, byte for byte.
    pub fn take_design(
        root: &Path,
        design: &DesignSet,
        manifest: &Path,
        variant: &str,
        sources: &[(SourceRole, &Path)],
    ) -> Result<Self, RecordError> {
        check_variant(variant)?;
        let root = canonical(root)?;
        let loaded = RequirementManifest::load(manifest).map_err(RecordError::Manifest)?;
        let manifest_pin = ManifestPin {
            path: relative(&root, &canonical(manifest)?)?,
            doc_id: loaded.doc_id,
            rev: loaded.rev,
            sha256: file_sha256(manifest)?,
            sidecar_sha256: None,
        };
        // Each named document once, in the order named: a document named twice
        // is one member of the design.
        let mut named: Vec<PathBuf> = Vec::with_capacity(design.named().len());
        for document in design.named() {
            let path = canonical(Path::new(document))?;
            if !named.contains(&path) {
                named.push(path);
            }
        }
        // A design accepted under a profile says what it left open the way a
        // run under that profile does: an `sce:assumed` that cites one of its
        // house rules is the owner's standing answer, not a value chosen
        // without one.
        let house_rules = house_rules_of(sources)?;
        let house_rule_ids: Vec<&str> = house_rules.iter().map(|rule| rule.id.as_str()).collect();
        let read = read_members(&root, &named, design.with_imports(), &house_rule_ids)
            .map_err(DesignFailure::into_take_error)?;
        let inputs = pin_inputs(&root, &read.files)?;
        let reading = read.reading;
        let applied_rules = applied_rules_of(&house_rules, &reading.cited);
        let open_at_acceptance = reading.open;
        let (evidence, unclaimed) = reading.evidence.digests();
        let evidence_rule =
            (!evidence.is_empty() || !unclaimed.is_empty()).then_some(EVIDENCE_RULE);
        let mut authored_from = Vec::with_capacity(sources.len());
        for (role, path) in sources {
            authored_from.push(SourcePin {
                role: *role,
                path: relative(&root, &canonical(path)?)?,
                sha256: file_sha256(path)?,
            });
        }
        authored_from.sort();
        check_sources(&authored_from)?;
        let mut documents = Vec::with_capacity(named.len());
        for path in &named {
            documents.push(relative(&root, path)?);
        }
        let document = documents
            .first()
            .cloned()
            .ok_or_else(|| RecordError::Format {
                detail: "a design is read from at least one document".to_string(),
            })?;
        if documents.len() == 1 {
            documents.clear();
        }
        Ok(AcceptanceRecord {
            variant: variant.to_string(),
            document,
            documents,
            with_imports: design.with_imports(),
            manifest: manifest_pin,
            inputs,
            authored_from,
            applied_rules,
            open_at_acceptance,
            evidence,
            unclaimed,
            evidence_rule,
            succeeds: None,
            channel: None,
        })
    }

    /// The documents the caller named, relative to the root, `document` first:
    /// `document` alone for a record of one.
    pub fn named_documents(&self) -> &[String] {
        if self.documents.is_empty() {
            std::slice::from_ref(&self.document)
        } else {
            &self.documents
        }
    }

    /// Whether the record is of a design other than one document read alone:
    /// several named, or one read with what it reaches.
    pub fn is_of_a_set(&self) -> bool {
        !self.documents.is_empty() || self.with_imports
    }

    /// The same record, also pinning the sentences behind the manifest's ids.
    ///
    /// `manifest` has to be the file this record pinned, and `sidecar` the
    /// manifest's own (the sidecar loader's refusals, unchanged: another
    /// document, another revision, no sentence at all). Only the sidecar's
    /// digest is kept; its sentences stay where they are.
    ///
    /// Without this a revision of the accepted design cannot be tied to the
    /// list the owner accepted: the manifest's digest names the shape of the
    /// list, and a list of another specification with the same shape has it
    /// too.
    pub fn with_words(mut self, manifest: &Path, sidecar: &Path) -> Result<Self, RecordError> {
        if file_sha256(manifest)? != self.manifest.sha256 {
            return Err(RecordError::Source {
                detail: format!(
                    "{} is not the manifest this record pins, so a sidecar cannot be tied \
                     to it",
                    manifest.display()
                ),
            });
        }
        let loaded = RequirementManifest::load(manifest).map_err(RecordError::Manifest)?;
        RequirementSidecar::load(sidecar, &loaded).map_err(RecordError::Words)?;
        self.manifest.sidecar_sha256 = Some(file_sha256(sidecar)?);
        Ok(self)
    }

    /// The same record, stating `previous` as the record it replaces.
    ///
    /// `previous` has to be a record this module reads, taken for the same
    /// specification (`doc_id`): an acceptance replaces an acceptance of the
    /// same thing, not of another. It is pinned by its bytes and named relative
    /// to `root`, like every other file. Like `channel` it says how the
    /// acceptance stood when it was taken and cannot lapse.
    pub fn succeeding(mut self, root: &Path, previous: &Path) -> Result<Self, RecordError> {
        let root = canonical(root)?;
        let previous = canonical(previous)?;
        let text = std::fs::read_to_string(&previous).map_err(|source| RecordError::Read {
            path: previous.display().to_string(),
            source,
        })?;
        let earlier = AcceptanceRecord::from_json(&text)?;
        if earlier.manifest.doc_id != self.manifest.doc_id {
            return Err(RecordError::Format {
                detail: format!(
                    "the record to replace is for specification `{}`, and this one is for `{}`",
                    earlier.manifest.doc_id, self.manifest.doc_id
                ),
            });
        }
        self.succeeds = Some(PredecessorPin {
            path: relative(&root, &previous)?,
            sha256: file_sha256(&previous)?,
        });
        Ok(self)
    }

    /// What moved in `document` since this record was taken, requirement by
    /// requirement: the record's digests against the rows of the design as it is
    /// now. A report, not a verdict: the acceptance lapses by its bytes whatever
    /// this says, and `unchanged` means the product's closure of what a requirement
    /// depends on reads the same, not that the requirement is still met.
    ///
    /// The comparison needs the design and not a second record because a digest
    /// has no place the owner can open, and a row of the design has one.
    /// `document` is whatever the caller names: the record's own document, which
    /// the caller resolves against the root it reads the record under, or another
    /// draft of it.
    ///
    /// ⚠ Refused for a record that states no evidence at all against a design that
    /// has rows: it was taken before records kept them, and reading it as "every
    /// requirement is new" would call the whole design changed.
    ///
    /// ⚠ Refused for a record of a set ([`Self::is_of_a_set`]): a draft of one
    /// document read alone is not the design that record names, and comparing
    /// the two would call every claim made elsewhere in the set dropped. Such a
    /// record is compared with its own documents by [`Self::evidence_delta_at`].
    pub fn evidence_delta(&self, document: &Path) -> Result<EvidenceDelta, RecordError> {
        if self.is_of_a_set() {
            return Err(RecordError::OfSeveralDocuments {
                document: self.document.clone(),
            });
        }
        let document = canonical(document)?;
        let (_, reading) = design_inputs(&document, &[]).map_err(DesignFailure::into_take_error)?;
        self.delta_against(reading.evidence)
    }

    /// [`Self::evidence_delta`] against the record's own design as it is now
    /// under `root`: its documents, and what they reach when it was taken with
    /// its imports, read as a re-check reads them. For a record of one document
    /// read alone it is `evidence_delta` of that document under `root`.
    pub fn evidence_delta_at(&self, root: &Path) -> Result<EvidenceDelta, RecordError> {
        let root = canonical(root)?;
        let mut named = Vec::with_capacity(self.named_documents().len());
        for document in self.named_documents() {
            named.push(canonical(&root.join(document))?);
        }
        let read = read_members(&root, &named, self.with_imports, &[])
            .map_err(DesignFailure::into_take_error)?;
        self.delta_against(read.reading.evidence)
    }

    /// The record's evidence against `now`, the evidence of the design as it is.
    fn delta_against(&self, now: Evidence) -> Result<EvidenceDelta, RecordError> {
        if self.evidence.is_empty()
            && self.unclaimed.is_empty()
            && (!now.by_requirement.is_empty() || !now.unclaimed.is_empty())
        {
            return Err(RecordError::Format {
                detail: "the record states no evidence (it was taken before records kept it), so \
                         there is nothing to compare the design with; take the acceptance again"
                    .to_string(),
            });
        }
        // A record that states evidence made it under some rule, and a digest is only
        // comparable with one made the same way: an absent rule is rule 1.
        let stated = !self.evidence.is_empty() || !self.unclaimed.is_empty();
        let made_under = self.evidence_rule.unwrap_or(1);
        if stated && made_under != EVIDENCE_RULE {
            return Err(RecordError::Format {
                detail: format!(
                    "the record's evidence was made under rule {made_under} and this build \
                     compares rule {EVIDENCE_RULE} (a transition's `type` is part of its row \
                     now), so equal digests would not mean an unchanged row; take the \
                     acceptance again. Whether the acceptance still holds is a different \
                     question and `acceptance-check` still answers it"
                ),
            });
        }
        let mut requirements = BTreeMap::new();
        let ids: std::collections::BTreeSet<&String> = self
            .evidence
            .keys()
            .chain(now.by_requirement.keys())
            .collect();
        for id in ids {
            let change = match (self.evidence.get(id), now.by_requirement.get(id)) {
                (Some(recorded), Some(rows)) => {
                    let (moved, gone) = compare_rows(recorded, rows);
                    if moved.is_empty() && gone == 0 {
                        RequirementChange::Unchanged
                    } else {
                        RequirementChange::Changed { moved, gone }
                    }
                }
                (Some(recorded), None) => RequirementChange::Dropped {
                    gone: recorded.len(),
                },
                (None, Some(rows)) => {
                    let mut at: Vec<String> = rows.iter().map(|row| row.at.clone()).collect();
                    at.sort();
                    RequirementChange::New { at }
                }
                (None, None) => continue,
            };
            requirements.insert(id.clone(), change);
        }
        let (unclaimed_added, unclaimed_gone) = compare_rows(&self.unclaimed, &now.unclaimed);
        Ok(EvidenceDelta {
            requirements,
            unclaimed_added,
            unclaimed_gone,
        })
    }

    /// The same record, stating `channel` as the surface that took it. Taking the
    /// record does not know which surface asked, so the caller says it here.
    #[must_use]
    pub fn stated_by(mut self, channel: Option<Channel>) -> Self {
        self.channel = channel;
        self
    }

    /// [`Self::recheck`], and also whether the files a caller asks about are
    /// the ones the design was authored from.
    ///
    /// Compared by content, role by role: the specification files asked
    /// about must be exactly the pinned ones' bytes, as a set, and so must the
    /// decision record and the authoring profile — including none when none
    /// was pinned. This is the
    /// question "is this accepted design the answer for these inputs", which
    /// a caller asks before writing a new draft of the same specification.
    pub fn recheck_for(
        &self,
        root: &Path,
        variant: &str,
        asked: &[(SourceRole, &Path)],
    ) -> Result<Vec<Lapse>, RecordError> {
        let mut lapses = self.recheck(root, variant)?;
        for role in SourceRole::ALL {
            let pinned: Vec<&SourcePin> = self
                .authored_from
                .iter()
                .filter(|p| p.role == role)
                .collect();
            let mut asked_paths = Vec::new();
            let mut asked_digests = std::collections::BTreeSet::new();
            for (asked_role, path) in asked {
                if *asked_role == role {
                    asked_paths.push(path.display().to_string());
                    asked_digests.insert(file_sha256(path)?);
                }
            }
            let pinned_digests: std::collections::BTreeSet<String> =
                pinned.iter().map(|p| p.sha256.clone()).collect();
            // A caller that names nothing is asking about the design alone,
            // which `recheck` has answered. One that names anything is asking
            // about a set of inputs, and a role it left out is part of the
            // answer: a decision record pinned and not given, or a
            // specification not given, is a different set.
            if !asked.is_empty() && asked_digests != pinned_digests {
                lapses.push(Lapse::NotAuthoredFrom {
                    role,
                    pinned: pinned.iter().map(|p| p.path.clone()).collect(),
                    asked: asked_paths,
                });
            }
        }
        Ok(lapses)
    }

    /// Everything that moved since the record was taken, asked about
    /// `variant` under `root`. Empty means the acceptance still holds.
    ///
    /// Every component is examined even after one has lapsed, so a reader
    /// learns the whole of what moved at once rather than one part a run.
    pub fn recheck(&self, root: &Path, variant: &str) -> Result<Vec<Lapse>, RecordError> {
        let root = canonical(root)?;
        let mut lapses = Vec::new();

        if variant != self.variant {
            lapses.push(Lapse::Variant {
                recorded: self.variant.clone(),
                asked: variant.to_string(),
            });
        }

        let manifest_path = root.join(&self.manifest.path);
        match std::fs::read(&manifest_path) {
            Err(_) => lapses.push(Lapse::Missing {
                path: self.manifest.path.clone(),
            }),
            Ok(bytes) => {
                let current_sha256 = hex_encode(&sha256_bytes(&bytes));
                if current_sha256 != self.manifest.sha256 {
                    lapses.push(Lapse::Manifest {
                        recorded_rev: self.manifest.rev.clone(),
                        current_rev: RequirementManifest::load(&manifest_path)
                            .ok()
                            .map(|m| m.rev),
                        recorded_sha256: self.manifest.sha256.clone(),
                        current_sha256,
                    });
                }
            }
        }

        for pin in &self.authored_from {
            let current_sha256 = std::fs::read(root.join(&pin.path))
                .ok()
                .map(|bytes| hex_encode(&sha256_bytes(&bytes)));
            let moved = current_sha256.as_deref() != Some(pin.sha256.as_str());
            if moved {
                lapses.push(Lapse::Source {
                    role: pin.role,
                    path: pin.path.clone(),
                    recorded_sha256: pin.sha256.clone(),
                    current_sha256: current_sha256.clone(),
                });
            }
            // The profile's bytes moved: say which of the rules the design
            // applied moved with them. Only from a profile that loads now.
            if moved && pin.role == SourceRole::Profile && current_sha256.is_some() {
                self.rule_lapses(&root.join(&pin.path), &mut lapses);
            }
        }

        // Every document the caller named is read again. One that is gone leaves
        // no design to read, and is said with the rest of what moved.
        let mut named = Vec::with_capacity(self.named_documents().len());
        let mut gone = false;
        for document in self.named_documents() {
            let path = root.join(document);
            if path.is_file() {
                named.push(canonical(&path)?);
            } else {
                lapses.push(Lapse::Missing {
                    path: document.clone(),
                });
                gone = true;
            }
        }
        if gone {
            return Ok(lapses);
        }
        // What was open is a fact about the moment of acceptance and not a
        // thing to re-judge: a design that changed is a lapse by its bytes.
        let current = match read_members(&root, &named, self.with_imports, &[]) {
            Ok(read) => pin_inputs(&root, &read.files)?,
            Err(DesignFailure::Parse { path, detail }) => {
                // A document the record names is said as the record names it.
                let path = named
                    .iter()
                    .zip(self.named_documents())
                    .find(|(read, _)| **read == path)
                    .map_or_else(
                        || relative(&root, &path).unwrap_or_else(|_| path.display().to_string()),
                        |(_, recorded)| recorded.clone(),
                    );
                lapses.push(Lapse::Unparseable { path, detail });
                return Ok(lapses);
            }
            Err(DesignFailure::Record(error)) => return Err(error),
        };

        let now: BTreeMap<&str, &str> = current
            .iter()
            .map(|pin| (pin.path.as_str(), pin.sha256.as_str()))
            .collect();
        let then: BTreeMap<&str, &str> = self
            .inputs
            .iter()
            .map(|pin| (pin.path.as_str(), pin.sha256.as_str()))
            .collect();
        for (path, recorded) in &then {
            match now.get(path) {
                // Gone from the design either way: deleted, or still on
                // disk and no longer read because the composition moved
                // away from it. Both mean the design that was accepted is
                // not the one the parse assembles now.
                None => lapses.push(Lapse::Missing {
                    path: (*path).to_string(),
                }),
                Some(current) if current != recorded => lapses.push(Lapse::Moved {
                    path: (*path).to_string(),
                    recorded_sha256: (*recorded).to_string(),
                    current_sha256: (*current).to_string(),
                }),
                Some(_) => {}
            }
        }
        for path in now.keys() {
            if !then.contains_key(path) {
                lapses.push(Lapse::Added {
                    path: (*path).to_string(),
                });
            }
        }
        Ok(lapses)
    }

    /// Which of the rules the design applied no longer read as they did, read
    /// from the profile now at `profile`. A profile that does not load says
    /// nothing here: its bytes moved, `Source` has said so, and a rule it
    /// cannot be read for is not reported as unchanged.
    fn rule_lapses(&self, profile: &Path, lapses: &mut Vec<Lapse>) {
        let Ok(now) = crate::authoring_profile::AuthoringProfile::load(profile) else {
            return;
        };
        for applied in &self.applied_rules {
            let current = now
                .house_rules()
                .iter()
                .find(|rule| rule.id == applied.id)
                .map(|rule| rule.rule.clone());
            if current.as_deref() != Some(applied.rule.as_str()) {
                lapses.push(Lapse::Rule {
                    id: applied.id.clone(),
                    places: applied.places,
                    recorded: applied.rule.clone(),
                    current,
                });
            }
        }
    }

    /// The record as committed: pretty JSON with a trailing newline, so a
    /// re-take that pins the same design writes the same bytes.
    pub fn to_json(&self) -> String {
        let wire = RecordWire {
            record: RECORD_KIND.to_string(),
            v: RECORD_VERSION,
            variant: self.variant.clone(),
            document: self.document.clone(),
            documents: self.documents.clone(),
            with_imports: self.with_imports,
            manifest: self.manifest.clone(),
            inputs: self.inputs.clone(),
            authored_from: self.authored_from.clone(),
            applied_rules: self.applied_rules.clone(),
            open_at_acceptance: self.open_at_acceptance.clone(),
            evidence: self.evidence.clone(),
            unclaimed: self.unclaimed.clone(),
            evidence_rule: self.evidence_rule,
            succeeds: self.succeeds.clone(),
            channel: self.channel,
        };
        let mut text = serde_json::to_string_pretty(&wire)
            .expect("a record of strings and one integer always serialises");
        text.push('\n');
        text
    }

    /// Read a record, refusing anything this module would not have written.
    pub fn from_json(text: &str) -> Result<Self, RecordError> {
        let wire: RecordWire =
            crate::strict_json::from_str(text).map_err(|e| RecordError::Format {
                detail: e.to_string(),
            })?;
        if wire.record != RECORD_KIND {
            return Err(RecordError::Format {
                detail: format!("`record` is `{}`, not `{RECORD_KIND}`", wire.record),
            });
        }
        if wire.v != RECORD_VERSION {
            return Err(RecordError::Format {
                detail: format!("version {} is not {RECORD_VERSION}", wire.v),
            });
        }
        check_variant(&wire.variant)?;
        for path in std::iter::once(&wire.document)
            .chain(wire.documents.iter())
            .chain(std::iter::once(&wire.manifest.path))
            .chain(wire.inputs.iter().map(|pin| &pin.path))
            .chain(wire.authored_from.iter().map(|pin| &pin.path))
            .chain(wire.succeeds.iter().map(|pin| &pin.path))
        {
            check_relative(path)?;
        }
        for digest in std::iter::once(&wire.manifest.sha256)
            .chain(wire.manifest.sidecar_sha256.iter())
            .chain(wire.inputs.iter().map(|pin| &pin.sha256))
            .chain(wire.authored_from.iter().map(|pin| &pin.sha256))
            .chain(wire.succeeds.iter().map(|pin| &pin.sha256))
        {
            check_sha256(digest)?;
        }
        check_evidence(&wire.evidence, &wire.unclaimed)?;
        check_evidence_rule(
            wire.evidence_rule,
            !wire.evidence.is_empty() || !wire.unclaimed.is_empty(),
        )?;
        let mut authored_from = wire.authored_from;
        authored_from.sort();
        check_sources(&authored_from).map_err(|e| RecordError::Format {
            detail: e.to_string(),
        })?;
        // The entry document is the one input no composition can drop, so a
        // record whose inputs omit it pins a design it never read.
        if !wire.inputs.iter().any(|pin| pin.path == wire.document) {
            return Err(RecordError::Format {
                detail: format!("the inputs do not pin the document `{}`", wire.document),
            });
        }
        check_documents(&wire.document, &wire.documents, &wire.inputs)?;
        let mut inputs = wire.inputs;
        let before = inputs.len();
        inputs.sort();
        inputs.dedup_by(|a, b| a.path == b.path);
        if inputs.len() != before {
            return Err(RecordError::Format {
                detail: "an input path is pinned more than once".to_string(),
            });
        }
        // A rule is applied from a profile, so a record that names rules and
        // pins no profile cites something it cannot be checked against, and a
        // rule named twice leaves open which wording the design relied on.
        let mut applied_rules = wire.applied_rules;
        if !applied_rules.is_empty() && !authored_from.iter().any(|p| p.role == SourceRole::Profile)
        {
            return Err(RecordError::Format {
                detail: "applied rules are named, and no authoring profile is pinned for them \
                         to have come from"
                    .to_string(),
            });
        }
        let before = applied_rules.len();
        applied_rules.sort();
        applied_rules.dedup_by(|a, b| a.id == b.id);
        if applied_rules.len() != before {
            return Err(RecordError::Format {
                detail: "a house rule is named more than once among the applied rules".to_string(),
            });
        }
        if let Some(blank) = applied_rules
            .iter()
            .find(|r| r.id.is_empty() || r.places == 0)
        {
            return Err(RecordError::Format {
                detail: format!(
                    "applied rule {:?} has no id or no place that cites it",
                    blank.id
                ),
            });
        }
        Ok(AcceptanceRecord {
            variant: wire.variant,
            document: wire.document,
            documents: wire.documents,
            with_imports: wire.with_imports,
            manifest: wire.manifest,
            inputs,
            authored_from,
            applied_rules,
            open_at_acceptance: wire.open_at_acceptance,
            evidence: wire.evidence,
            unclaimed: wire.unclaimed,
            evidence_rule: wire.evidence_rule,
            succeeds: wire.succeeds,
            channel: wire.channel,
        })
    }
}

/// The documents a record names, as read back: none beyond `document` for a
/// design of one, and otherwise at least two, `document` first, none twice,
/// each pinned among the inputs. Refuses what `take_design` would not write.
fn check_documents(
    document: &str,
    documents: &[String],
    inputs: &[InputPin],
) -> Result<(), RecordError> {
    if documents.is_empty() {
        return Ok(());
    }
    if documents.len() == 1 {
        return Err(RecordError::Format {
            detail: "`documents` names one document; a design of one names it as `document` \
                     alone"
                .to_string(),
        });
    }
    if documents[0] != document {
        return Err(RecordError::Format {
            detail: format!(
                "`documents` begins with `{}`, and the record's `document` is `{document}`",
                documents[0]
            ),
        });
    }
    let mut seen = std::collections::BTreeSet::new();
    for named in documents {
        if !seen.insert(named.as_str()) {
            return Err(RecordError::Format {
                detail: format!("`{named}` is named twice among the documents"),
            });
        }
        if !inputs.iter().any(|pin| pin.path == *named) {
            return Err(RecordError::Format {
                detail: format!("the inputs do not pin the document `{named}`"),
            });
        }
    }
    Ok(())
}

/// The rules an `authored_from` list keeps, sorted: no path twice, and one
/// decision record and one profile at most — the owner's answers are one
/// record, and what a design is held to is one profile, so two of either
/// would leave open which of them a design followed.
fn check_sources(sorted: &[SourcePin]) -> Result<(), RecordError> {
    let mut paths = std::collections::BTreeSet::new();
    for pin in sorted {
        if !paths.insert(pin.path.as_str()) {
            return Err(RecordError::Source {
                detail: format!(
                    "{} is named twice as a file the design was authored from",
                    pin.path
                ),
            });
        }
    }
    for (role, what, followed) in [
        (
            SourceRole::Decisions,
            "decision records",
            "one record of the owner's answers",
        ),
        (
            SourceRole::Profile,
            "authoring profiles",
            "one authoring profile",
        ),
        (SourceRole::Examples, "scenario sets", "one set of examples"),
    ] {
        let count = sorted.iter().filter(|p| p.role == role).count();
        if count > 1 {
            return Err(RecordError::Source {
                detail: format!("{count} {what} given; a design follows {followed}"),
            });
        }
    }
    Ok(())
}

/// Why the design's files could not be read.
enum DesignFailure {
    /// A document of the design does not parse, or names a document that
    /// cannot be read: `path` is the document. A lapse when re-checking, a
    /// refusal when taking.
    Parse {
        path: PathBuf,
        detail: String,
    },
    Record(RecordError),
}

impl DesignFailure {
    fn into_take_error(self) -> RecordError {
        match self {
            DesignFailure::Parse { path, detail } => RecordError::Parse {
                path: path.display().to_string(),
                detail,
            },
            DesignFailure::Record(error) => error,
        }
    }
}

impl From<RecordError> for DesignFailure {
    fn from(error: RecordError) -> Self {
        DesignFailure::Record(error)
    }
}

/// The house rules of the authoring profile among `sources` as `(id, rule)`, or
/// none when the record is taken under no profile. The profile is read through
/// its own reader; one that cannot be read is refused here in its words, though
/// the command line has already refused it before it got this far.
fn house_rules_of(
    sources: &[(SourceRole, &Path)],
) -> Result<Vec<crate::authoring_profile::HouseRule>, RecordError> {
    let Some((_, path)) = sources
        .iter()
        .find(|(role, _)| *role == SourceRole::Profile)
    else {
        return Ok(Vec::new());
    };
    let profile = crate::authoring_profile::AuthoringProfile::load(path).map_err(|unusable| {
        RecordError::Source {
            detail: format!("{}: {unusable}", path.display()),
        }
    })?;
    Ok(profile.house_rules().to_vec())
}

/// The rules the design cited, as the profile worded them: `cited` is the
/// `(id, places)` the design's markers name, and an id the profile does not hold
/// is not a house rule and is not here.
fn applied_rules_of(
    house_rules: &[crate::authoring_profile::HouseRule],
    cited: &[(String, usize)],
) -> Vec<AppliedRule> {
    let mut applied: Vec<AppliedRule> = cited
        .iter()
        .filter_map(|(id, places)| {
            house_rules
                .iter()
                .find(|known| known.id == *id)
                .map(|rule| AppliedRule {
                    id: id.clone(),
                    rule: rule.rule.clone(),
                    places: *places,
                    quote: rule.quote.clone(),
                    confirmation: rule.confirmation,
                })
        })
        .collect();
    applied.sort();
    applied
}

/// What reading a design says besides the files it read: what it leaves to a
/// person ([`crate::open_matters`]) and how many places cite each house rule of
/// the profile it is read under.
#[derive(Default)]
struct DesignReading {
    open: Vec<crate::open_matters::OpenMatter>,
    cited: Vec<(String, usize)>,
    evidence: Evidence,
}

impl DesignReading {
    /// What reading one document of a design said, pooled into what reading
    /// the design says. `document` is the document's path relative to the root
    /// when the design is of several documents, and `None` when it is of this
    /// one alone, whose reading is then taken as it is.
    fn absorb(&mut self, one: DesignReading, document: Option<&str>) {
        match document {
            Some(document) => self
                .open
                .extend(crate::open_matters::in_document(document, one.open)),
            None => self.open.extend(one.open),
        }
        for (id, places) in one.cited {
            match self.cited.iter_mut().find(|(known, _)| *known == id) {
                Some((_, total)) => *total += places,
                None => self.cited.push((id, places)),
            }
        }
        self.evidence.absorb(one.evidence, document);
    }
}

/// One row of the review table as a record sees it: its digest, and where it
/// is in the design that was read (a statechart's `node_path`, a forge node's
/// path). The record keeps the digest; the place is for [`evidence_delta`],
/// which has the design in hand and the record's digests only.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct EvidenceRow {
    digest: String,
    at: String,
}

/// What the report shows an owner for each requirement: the module docs' "What
/// each requirement rested on". Rows are sorted, so a list of digests read off
/// them is in order.
#[derive(Debug, Default)]
struct Evidence {
    by_requirement: BTreeMap<String, Vec<EvidenceRow>>,
    unclaimed: Vec<EvidenceRow>,
}

impl Evidence {
    /// One document's evidence pooled into the design's, each row placed in
    /// its document when `document` is given ([`placed_in`]).
    fn absorb(&mut self, one: Evidence, document: Option<&str>) {
        for (id, rows) in one.by_requirement {
            let pooled = self.by_requirement.entry(id).or_default();
            pooled.extend(rows.into_iter().map(|row| placed_in(row, document)));
            pooled.sort();
        }
        self.unclaimed.extend(
            one.unclaimed
                .into_iter()
                .map(|row| placed_in(row, document)),
        );
        self.unclaimed.sort();
    }

    /// The record's form: digests only, in order.
    fn digests(&self) -> (BTreeMap<String, Vec<String>>, Vec<String>) {
        let digests = |rows: &[EvidenceRow]| rows.iter().map(|row| row.digest.clone()).collect();
        (
            self.by_requirement
                .iter()
                .map(|(id, rows)| (id.clone(), digests(rows)))
                .collect(),
            digests(&self.unclaimed),
        )
    }
}

/// The digest of one row of the review table, `source` column left out and
/// position left in. A JSON array of fixed shape, so no cell can run into the
/// next, and a tag so a statechart's row and a forge row cannot collide.
fn row_digest(cells: &[&str]) -> String {
    let text = serde_json::to_string(cells).expect("a list of strings always serialises");
    hex_encode(&sha256_bytes(text.as_bytes()))
}

/// A row of one document of a design of several, placed in its document: its
/// place led by the document (`front.scxml#states.idle`, the spelling a node
/// path of `requirements` takes), and its digest taken over the document's
/// path and the row's own digest, so the same row in two documents is two rows.
/// A row of a design of one document (`None`) is left as it is.
fn placed_in(row: EvidenceRow, document: Option<&str>) -> EvidenceRow {
    let Some(document) = document else {
        return row;
    };
    EvidenceRow {
        digest: row_digest(&["document", document, &row.digest]),
        at: format!("{document}#{}", row.at),
    }
}

fn statechart_row_digest(row: &crate::transition_table::TransitionRow) -> String {
    row_digest(&[
        "statechart-row",
        &row.from,
        &row.event,
        &row.guard,
        &row.after,
        &row.to,
        &row.action,
        &row.node_path,
    ])
}

fn forge_row_digest(row: &crate::forge::review_table::ReviewRow) -> String {
    row_digest(&["forge-row", &row.node, row.node_type, &row.detail])
}

/// The evidence of a statechart: for each requirement id a node cites, the rows
/// of its [`crate::acceptance_report::fragment`] (the dependency closure the
/// report prints, never the nodes carrying the id alone), and the rows that
/// claim nothing.
fn statechart_evidence(model: &crate::model::SCXMLModel) -> Evidence {
    let rows = crate::transition_table::transition_table(model);
    let by_path: BTreeMap<&str, &crate::transition_table::TransitionRow> = rows
        .iter()
        .map(|row| (row.node_path.as_str(), row))
        .collect();
    let cited: std::collections::BTreeSet<String> = crate::requirements_report::walk_nodes(model)
        .iter()
        .flat_map(|node| {
            node.record
                .requirement_ids
                .iter()
                .map(|id| (*id).to_string())
        })
        .collect();
    let evidence_row = |row: &crate::transition_table::TransitionRow| EvidenceRow {
        digest: statechart_row_digest(row),
        at: row.node_path.clone(),
    };
    let mut by_requirement = BTreeMap::new();
    for id in cited {
        // A node can be in a fragment for two reasons; it is one row.
        let paths: std::collections::BTreeSet<String> =
            crate::acceptance_report::fragment(model, &id)
                .into_iter()
                .map(|dependency| dependency.node_path)
                .collect();
        let mut found: Vec<EvidenceRow> = paths
            .iter()
            .filter_map(|path| by_path.get(path.as_str()))
            .map(|row| evidence_row(row))
            .collect();
        found.sort();
        by_requirement.insert(id, found);
    }
    let mut unclaimed: Vec<EvidenceRow> = rows
        .iter()
        .filter(|row| row.is_unclaimed())
        .map(evidence_row)
        .collect();
    unclaimed.sort();
    Evidence {
        by_requirement,
        unclaimed,
    }
}

/// The evidence of a forge document: the rows of its review table that claim
/// each id, and the rows that claim nothing, its root's among them.
fn forge_evidence(parsed: &crate::forge::model::ParsedForge) -> Evidence {
    let rows = crate::forge::review_table::review_table(parsed);
    let mut by_requirement: BTreeMap<String, Vec<EvidenceRow>> = BTreeMap::new();
    let mut unclaimed = Vec::new();
    for row in &rows {
        let found = EvidenceRow {
            digest: forge_row_digest(row),
            at: row.node.clone(),
        };
        if row.is_unclaimed() {
            unclaimed.push(found);
            continue;
        }
        for id in row.source.split_whitespace() {
            by_requirement
                .entry(id.to_string())
                .or_default()
                .push(found.clone());
        }
    }
    for found in by_requirement.values_mut() {
        found.sort();
    }
    unclaimed.sort();
    Evidence {
        by_requirement,
        unclaimed,
    }
}

/// A record's digests against the rows of a design now, as multisets: the places
/// (sorted) of the rows the record lacks, and how many recorded digests no row
/// matches. Two equal digests are two rows, so a digest matched once is spent.
fn compare_rows(recorded: &[String], now: &[EvidenceRow]) -> (Vec<String>, usize) {
    let mut left: BTreeMap<&str, usize> = BTreeMap::new();
    for digest in recorded {
        *left.entry(digest.as_str()).or_default() += 1;
    }
    let mut moved = Vec::new();
    for row in now {
        match left.get_mut(row.digest.as_str()) {
            Some(count) if *count > 0 => *count -= 1,
            _ => moved.push(row.at.clone()),
        }
    }
    moved.sort();
    (moved, left.values().sum())
}

/// The rule a record names its evidence under, as read back: a positive number,
/// and only where the record states evidence, as `take` writes it. Refuses what
/// `take` would not write.
fn check_evidence_rule(rule: Option<u32>, stated: bool) -> Result<(), RecordError> {
    match rule {
        Some(0) => Err(RecordError::Format {
            detail: "the evidence rule is 0; rules are numbered from 1".to_string(),
        }),
        Some(_) if !stated => Err(RecordError::Format {
            detail: "the record names an evidence rule and states no evidence".to_string(),
        }),
        _ => Ok(()),
    }
}

/// A record's evidence as read back: every key a name, every digest a sha256,
/// every list in order. Refuses what `take` would not write.
fn check_evidence(
    evidence: &BTreeMap<String, Vec<String>>,
    unclaimed: &[String],
) -> Result<(), RecordError> {
    let sorted = |digests: &[String]| digests.windows(2).all(|pair| pair[0] <= pair[1]);
    for (id, digests) in evidence {
        if id.is_empty() {
            return Err(RecordError::Format {
                detail: "the evidence names a requirement with an empty id".to_string(),
            });
        }
        for digest in digests {
            check_sha256(digest)?;
        }
        if !sorted(digests) {
            return Err(RecordError::Format {
                detail: format!("the evidence of `{id}` is not in order"),
            });
        }
    }
    for digest in unclaimed {
        check_sha256(digest)?;
    }
    if !sorted(unclaimed) {
        return Err(RecordError::Format {
            detail: "the unclaimed digests are not in order".to_string(),
        });
    }
    Ok(())
}

/// What reading the documents of a design gave: every file they read, each
/// document itself included, canonical and in the order read, and what reading
/// them said, pooled ([`DesignReading::absorb`]).
struct SetReading {
    files: Vec<PathBuf>,
    reading: DesignReading,
}

/// Read every document of the design `named` (canonical paths, never empty)
/// makes, with what they reach when `with_imports` ([`DesignSet::members`]).
///
/// One reader for taking a record, re-checking it and comparing it with the
/// design now, so the three cannot read one record as two designs. A walk that
/// is refused is a document that cannot be read: a lapse when re-checking
/// (`path` the first named document, since the reference is somewhere in the
/// design it is the entry of), a refusal when taking.
fn read_members(
    root: &Path,
    named: &[PathBuf],
    with_imports: bool,
    house_rule_ids: &[&str],
) -> Result<SetReading, DesignFailure> {
    let entry = named.first().cloned().unwrap_or_default();
    let design = DesignSet::new(
        named
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        with_imports,
    )
    .ok_or_else(|| RecordError::Format {
        detail: "a design is read from at least one document".to_string(),
    })?;
    let members = design.members().map_err(|refusal| {
        let refused = refusal.into_located();
        DesignFailure::Parse {
            path: entry,
            detail: format!("{}: {}", refused.location.file, refused.error),
        }
    })?;
    // One document is the design alone, read as it always was; several are
    // told apart by their paths relative to the root.
    let several = members.len() > 1;
    let mut files: Vec<PathBuf> = Vec::new();
    let mut reading = DesignReading::default();
    for member in &members {
        let document = canonical(Path::new(&member.document))?;
        let (read, one) = design_inputs(&document, house_rule_ids)?;
        files.push(document.clone());
        files.extend(read);
        let place = if several {
            Some(relative(root, &document)?)
        } else {
            None
        };
        reading.absorb(one, place.as_deref());
    }
    Ok(SetReading { files, reading })
}

/// Every file of `files` pinned relative to `root`, each once, sorted by path.
fn pin_inputs(root: &Path, files: &[PathBuf]) -> Result<Vec<InputPin>, RecordError> {
    let mut pins: BTreeMap<String, String> = BTreeMap::new();
    for path in files {
        let path = canonical(path)?;
        pins.insert(relative(root, &path)?, file_sha256(&path)?);
    }
    Ok(pins
        .into_iter()
        .map(|(path, sha256)| InputPin { path, sha256 })
        .collect())
}

/// The files other than `document` that its parse reads, for either
/// pipeline — what an acceptance rests on besides the document itself.
///
/// - A statechart: the fragments its preprocessors spliced in, and the
///   forge documents its `<sce:import>` declarations name (the event
///   schemas and enums its guards and sends are typed by). ⚠ The second
///   half was missing until the forge route below was written: the parser
///   reads those siblings through a path that does not add to
///   `preprocessor_deps`, so a statechart's record did not notice its
///   schema moving.
/// - A forge document: the fragments its expansion read, and the
///   transitive `<sce:import>` closure — the set a forge compile reports
///   as its dependencies.
fn design_inputs(
    document: &Path,
    house_rule_ids: &[&str],
) -> Result<(Vec<PathBuf>, DesignReading), DesignFailure> {
    let text = std::fs::read_to_string(document).map_err(|source| {
        DesignFailure::Record(RecordError::Read {
            path: document.display().to_string(),
            source,
        })
    })?;
    let unparseable = |detail: String| DesignFailure::Parse {
        path: document.to_path_buf(),
        detail,
    };
    let base_dir = document.parent().unwrap_or_else(|| Path::new("."));
    match crate::classify_document(&text) {
        crate::Pipeline::Scxml => {
            let mut parser = SCXMLParser::new();
            let model = parser
                .parse_file(&document.display().to_string())
                .map_err(|located| unparseable(located.error.to_string()))?;
            let mut inputs = parser.preprocessor_deps().to_vec();
            inputs.extend(
                model
                    .forge_imports
                    .iter()
                    .map(|import| Path::new(&import.src))
                    .filter(|src| !crate::forge::stdlib::names_standard(src))
                    .map(|src| base_dir.join(src)),
            );
            // The same reading the acceptance report makes, so what a
            // person was shown as open and what the record keeps as open
            // are one list.
            let open = crate::open_matters::of_statechart_under(&model, house_rule_ids);
            // And the places that cite each house rule, from the same
            // markers `of_statechart_under` reads.
            let mut markers = crate::unresolved_check::unresolved_records(&model);
            crate::unresolved_check::cite_house_rules(&mut markers, house_rule_ids);
            Ok((
                inputs,
                DesignReading {
                    open,
                    cited: crate::unresolved_check::applied_house_rules(&markers),
                    evidence: statechart_evidence(&model),
                },
            ))
        }
        crate::Pipeline::Forge => {
            let label_text = document.display().to_string();
            let label = crate::DocumentLabel::for_input_path(&label_text);
            let loaded = crate::load_forge_source(document, &[])
                .map_err(|located| unparseable(located.error.to_string()))?;
            let parsed = crate::forge::parser::parse_forge_with_imports(loaded.text(), label)
                .map_err(|located| unparseable(located.error.to_string()))?
                .ok_or_else(|| unparseable("the document is not a forge kind".to_string()))?;
            let imports =
                crate::forge::cross_kind_check::check(&parsed, base_dir, label.diagnostic_label)
                    .map_err(|located| unparseable(located.error.to_string()))?;
            // A forge kind sends to no parent and no processor, so what it
            // leaves open is its markers.
            let mut markers = crate::unresolved_check::unresolved_records_forge(&loaded.positions)
                .map_err(|located| unparseable(located.error.to_string()))?;
            crate::unresolved_check::cite_house_rules(&mut markers, house_rule_ids);
            let open = crate::open_matters::of(&markers, &[], &[], &[], &[]);
            let cited = crate::unresolved_check::applied_house_rules(&markers);
            // A `sce:std/...` document is read from the library compiled
            // into the generator; it has no file to pin, and moves only
            // with the generator itself.
            Ok((
                loaded
                    .deps
                    .into_iter()
                    .chain(imports)
                    .filter(|path| !crate::forge::stdlib::names_standard(path))
                    .collect(),
                DesignReading {
                    open,
                    cited,
                    evidence: forge_evidence(&parsed),
                },
            ))
        }
    }
}

fn canonical(path: &Path) -> Result<PathBuf, RecordError> {
    std::fs::canonicalize(path).map_err(|source| RecordError::Read {
        path: path.display().to_string(),
        source,
    })
}

fn file_sha256(path: &Path) -> Result<String, RecordError> {
    let bytes = std::fs::read(path).map_err(|source| RecordError::Read {
        path: path.display().to_string(),
        source,
    })?;
    Ok(hex_encode(&sha256_bytes(&bytes)))
}

/// `path` relative to `root`, spelled with `/` on every platform so a
/// record reads the same wherever it was taken.
fn relative(root: &Path, path: &Path) -> Result<String, RecordError> {
    let rel = path
        .strip_prefix(root)
        .map_err(|_| RecordError::OutsideRoot {
            path: path.display().to_string(),
            root: root.display().to_string(),
        })?;
    Ok(rel
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/"))
}

fn check_variant(value: &str) -> Result<(), RecordError> {
    if value.is_empty() || value.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(RecordError::Variant {
            value: value.to_string(),
        });
    }
    Ok(())
}

/// A pinned path must stay under the root it is read against.
fn check_relative(path: &str) -> Result<(), RecordError> {
    let parsed = Path::new(path);
    let escapes = parsed
        .components()
        .any(|part| !matches!(part, Component::Normal(_)));
    if path.is_empty() || escapes || path.contains('\\') {
        return Err(RecordError::Format {
            detail: format!("`{path}` is not a path relative to the record's root"),
        });
    }
    Ok(())
}

fn check_sha256(digest: &str) -> Result<(), RecordError> {
    let well_formed = digest.len() == 64
        && digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if !well_formed {
        return Err(RecordError::Format {
            detail: format!("`{digest}` is not a lowercase sha256"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "0000000000000000000000000000000000000000000000000000000000000000";

    fn wire(edit: impl FnOnce(&mut serde_json::Value)) -> String {
        let mut value = serde_json::json!({
            "record": RECORD_KIND,
            "v": RECORD_VERSION,
            "variant": "base",
            "document": "machine.scxml",
            "manifest": {"path": "spec.manifest.json", "doc_id": "SPEC", "rev": "D3", "sha256": SHA},
            "inputs": [{"path": "machine.scxml", "sha256": SHA}],
        });
        edit(&mut value);
        value.to_string()
    }

    /// The control for every refusal below: the same bytes, unedited, load.
    #[test]
    fn a_record_this_module_would_write_loads() {
        let record = AcceptanceRecord::from_json(&wire(|_| {})).expect("loads");
        assert_eq!(record.variant, "base");
        assert_eq!(
            AcceptanceRecord::from_json(&record.to_json()).expect("round-trips"),
            record
        );
    }

    #[test]
    fn the_rule_the_evidence_was_made_under_is_kept_and_a_record_without_one_is_rule_one() {
        let with_rule = AcceptanceRecord::from_json(&wire(|v| {
            v["evidence"] = serde_json::json!({"R1": [SHA]});
            v["evidence_rule"] = EVIDENCE_RULE.into();
        }))
        .expect("loads");
        assert_eq!(with_rule.evidence_rule, Some(EVIDENCE_RULE));
        assert_eq!(
            AcceptanceRecord::from_json(&with_rule.to_json()).expect("round-trips"),
            with_rule
        );
        // A record made before the rule was named has evidence and no rule, and writes
        // back as it was: nothing is added to it by reading it.
        let before = AcceptanceRecord::from_json(&wire(|v| {
            v["evidence"] = serde_json::json!({"R1": [SHA]});
        }))
        .expect("loads");
        assert_eq!(before.evidence_rule, None);
        assert!(
            !before.to_json().contains("evidence_rule"),
            "{}",
            before.to_json()
        );
    }

    /// A record of a set names its documents and its reach, and writes them back
    /// as it read them; a record of one document read alone writes neither.
    #[test]
    fn a_record_of_a_set_keeps_its_documents_and_one_of_a_document_names_none() {
        let set = AcceptanceRecord::from_json(&wire(|v| {
            v["inputs"] = serde_json::json!([
                {"path": "machine.scxml", "sha256": SHA},
                {"path": "other.scxml", "sha256": SHA},
            ]);
            v["documents"] = serde_json::json!(["machine.scxml", "other.scxml"]);
            v["with_imports"] = true.into();
        }))
        .expect("loads");
        assert_eq!(set.named_documents(), ["machine.scxml", "other.scxml"]);
        assert!(set.with_imports && set.is_of_a_set());
        assert_eq!(
            AcceptanceRecord::from_json(&set.to_json()).expect("round-trips"),
            set
        );

        let one = AcceptanceRecord::from_json(&wire(|_| {})).expect("loads");
        assert_eq!(one.named_documents(), ["machine.scxml"]);
        assert!(!one.is_of_a_set());
        let written = one.to_json();
        assert!(
            !written.contains("documents") && !written.contains("with_imports"),
            "{written}"
        );
    }

    #[test]
    fn bytes_this_module_would_not_write_are_refused() {
        let cases: Vec<(&str, String)> = vec![
            ("an unknown field", wire(|v| v["note"] = "x".into())),
            (
                "another kind",
                wire(|v| v["record"] = "sce-manifest".into()),
            ),
            ("another version", wire(|v| v["v"] = 2.into())),
            (
                "a variant with a space",
                wire(|v| v["variant"] = "base tls".into()),
            ),
            ("an empty variant", wire(|v| v["variant"] = "".into())),
            (
                "an absolute path",
                wire(|v| v["inputs"][0]["path"] = "/etc/passwd".into()),
            ),
            (
                "a path that climbs",
                wire(|v| v["manifest"]["path"] = "../spec.json".into()),
            ),
            (
                "an upper-case digest",
                wire(|v| v["manifest"]["sha256"] = SHA.to_uppercase().replace('0', "A").into()),
            ),
            (
                "a short digest",
                wire(|v| v["inputs"][0]["sha256"] = "abc".into()),
            ),
            (
                "inputs that omit the document",
                wire(|v| v["inputs"][0]["path"] = "other.scxml".into()),
            ),
            (
                "an input pinned twice",
                wire(|v| {
                    v["inputs"] = serde_json::json!([
                        {"path": "machine.scxml", "sha256": SHA},
                        {"path": "machine.scxml", "sha256": SHA},
                    ])
                }),
            ),
            (
                "an evidence rule of 0",
                wire(|v| {
                    v["evidence"] = serde_json::json!({"R1": [SHA]});
                    v["evidence_rule"] = 0.into();
                }),
            ),
            (
                "an evidence rule where no evidence is stated",
                wire(|v| v["evidence_rule"] = EVIDENCE_RULE.into()),
            ),
            (
                "documents that name one document",
                wire(|v| v["documents"] = serde_json::json!(["machine.scxml"])),
            ),
            (
                "documents that do not begin with the document",
                wire(|v| {
                    v["inputs"] = serde_json::json!([
                        {"path": "machine.scxml", "sha256": SHA},
                        {"path": "other.scxml", "sha256": SHA},
                    ]);
                    v["documents"] = serde_json::json!(["other.scxml", "machine.scxml"]);
                }),
            ),
            (
                "a document named twice",
                wire(|v| {
                    v["inputs"] = serde_json::json!([
                        {"path": "machine.scxml", "sha256": SHA},
                        {"path": "other.scxml", "sha256": SHA},
                    ]);
                    v["documents"] =
                        serde_json::json!(["machine.scxml", "other.scxml", "other.scxml"]);
                }),
            ),
            (
                "a named document the inputs do not pin",
                wire(|v| v["documents"] = serde_json::json!(["machine.scxml", "other.scxml"])),
            ),
            (
                "a path among the documents that climbs",
                wire(|v| {
                    v["inputs"] = serde_json::json!([
                        {"path": "machine.scxml", "sha256": SHA},
                        {"path": "../other.scxml", "sha256": SHA},
                    ]);
                    v["documents"] = serde_json::json!(["machine.scxml", "../other.scxml"]);
                }),
            ),
            (
                "with_imports that is not a boolean",
                wire(|v| v["with_imports"] = "yes".into()),
            ),
            (
                "an evidence rule that is not a number",
                wire(|v| {
                    v["evidence"] = serde_json::json!({"R1": [SHA]});
                    v["evidence_rule"] = "2".into();
                }),
            ),
        ];
        for (what, bytes) in cases {
            assert!(
                AcceptanceRecord::from_json(&bytes).is_err(),
                "{what} was loaded as a record: {bytes}"
            );
        }
    }
}
