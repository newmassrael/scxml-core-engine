// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The commands on the specs, as JSON in and JSON out.
//!
//! The `sce-spec` command and every application that links the specs (the workbench's desktop
//! and browser shells) reach the store through [`call`] or [`call_with`]. That is the point of
//! having it: entrances that each decode their own arguments and encode their own errors would be
//! as many chances to disagree about what a save is, and the screen is the one that cannot be
//! reasoned about from the command line. An application answers commands of its own in front of
//! these, and hands every other one here.
//!
//! A command takes one JSON object and answers one JSON object. Unknown fields
//! are refused, not ignored: an argument that nothing reads is a caller that
//! thinks it said something.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use std::collections::BTreeMap;

use crate::acceptance::{Acceptance, AcceptanceCorrupt, Basis, Snapshot};
use crate::acceptance_run::{CheckOutcome, RequirementsReport};
use crate::answers::{Answers, AnswersError};
use crate::bundle::{BundleCheck, CheckedBy};
use crate::clock::{utc_timestamp, Clock};
use crate::error::StoreError;
use crate::figures::{FigureRequest, RenderError};
use crate::journal::{Event, Journal};
use crate::model_set::{Document, ModelError, ModelFiles};
use crate::original::{MemberDraft, OriginalDraft, Role};
use crate::requests::{ConnectionRef, Failure, FailureKind, Inputs, Lease, Pin};
use crate::requirements::{Requirements, RequirementsError};
use crate::review::{Product, Review, ReviewRequest, Verdict};
use crate::revision::Revision;
use crate::store::{
    AdapterReport, AdapterStatus, AnswersText, CandidateWrite, HostStatus, ModelText, Published,
    Registration, RequestView, RequirementsText, SourceText, SpecId, SpecStore,
};

/// Every command on the specs, in the order a person would meet them.
pub const COMMANDS: &[&str] = &[
    "describe",
    "list_specs",
    "create_spec",
    "read_spec",
    "read_source",
    "save_source",
    "history",
    "save_model",
    "read_model",
    "model_history",
    "figures",
    "remove_spec",
    "review",
    "read_answers",
    "save_answers",
    "save_requirements",
    "read_requirements",
    "requirements_report",
    "accept",
    "read_acceptance",
    "read_acceptance_delta",
    "read_revision_report",
    "read_judgment",
    "read_spec_snapshot",
    "read_spec_heads",
    "request_generation",
    "read_request",
    "list_requests",
    "list_open_requests",
    "claim_request",
    "heartbeat_request",
    "save_request_candidate",
    "read_request_candidate",
    "complete_request",
    "fail_request",
    "cancel_request",
    "report_adapter",
    "read_adapter_status",
    "read_host_status",
    "read_queue",
    "record_request_event",
    "read_request_journal",
    "read_bundle",
    "bundle_history",
    "add_original",
    "read_original",
    "list_originals",
    "read_original_member",
    "save_extraction",
    "read_extraction",
    "list_extractions",
];

/// The version of this command set. It moves when a command's arguments or
/// answer change in a way a caller written against the last one would misread.
/// `describe` says it as `command_set_version` at every entrance, the specs' own
/// (`sce-spec`) and an application's that answers commands of its own in front of
/// these, which says its own beside it.
///
/// Up to 24 this was the workbench's one command set, and the entries for its
/// settings and the programs of a person's (14 to 18) are of commands that are no
/// longer in it (see 25).
///
/// 2: a spec also keeps a model (`save_model`, `read_model`, `model_history`) and
/// SCE draws it (`figures`). A screen written for 1 has no use for them, and a
/// screen written for 2 cannot run on a core of 1, so the two are told apart.
///
/// 3: a spec can be removed (`remove_spec`). A screen written for 3 offers it and
/// would be refused by a core of 2 with `unknown-command`, so the two are told
/// apart before the person presses the button.
///
/// 4: SCE's check and pseudocode page of a model can be read (`review`).
///
/// 5: the owner's answers to the questions a model leaves open are kept
/// (`read_answers`, `save_answers`).
///
/// 6: a model can be several documents that name each other. `save_model` takes
/// `documents` (and an `entry`) in place of `text`, and a model that was read says
/// its `entry` and lists its `documents`; a screen written for 5 shows one text and
/// would show a set's entry as if it were the whole.
///
/// 7: the requirements a text states are kept (`save_requirements`,
/// `read_requirements`), measured against the model (`requirements_report`), and the
/// owner can accept a design against them (`accept`) and ask whether the acceptance
/// still holds (`read_acceptance`).
///
/// 8: a spec can be read as one state (`read_spec_snapshot`): the text, the model, the
/// answers, the requirement list and the acceptance as they stood together, for a screen
/// that is told something changed and must not show a text of one moment beside a model
/// of another. A screen written for 7 has no use for it, and a screen written for 8
/// cannot run on a core of 7.
///
/// 9: where each chain of a spec stands can be read without reading the spec
/// (`read_spec_heads`), so that a screen can ask often whether the spec moved under it
/// (a save from another window, an authoring client's next model) and read it only
/// when it did. A screen written for 9 asks for it, and a core of 8 would refuse with
/// `unknown-command`.
///
/// 10: a spec can be asked for a model (`request_generation`), an executor takes the
/// request, keeps it and finishes it (`claim_request`, `heartbeat_request`,
/// `complete_request`, `fail_request`), the owner calls it off (`cancel_request`), and
/// requests are read (`read_request`, `list_requests`). An AI adapter says it is there
/// (`report_adapter`) and a screen asks which are (`read_adapter_status`). The heads of a
/// spec also say where its latest request stands. A screen written for 10 offers what a
/// request makes possible, and would be refused by a core of 9 with `unknown-command`.
///
/// 11: what a request makes becomes the spec's in one step. The executor writes a candidate
/// (`save_request_candidate`, read back by `read_request_candidate`) that is not yet the
/// spec's model, and `complete_request` publishes it: the core runs its own check of the
/// model, and the model and the requirement list become the spec's together as one bundle
/// (`read_bundle`, `bundle_history`). An executor finds the requests it may take across the
/// specs (`list_open_requests`), and a screen asks whether a shell hosts one and, if not, why
/// (`read_host_status`). A request says its `candidate` and, once done, its
/// `outcome`; the heads and the snapshot of a spec say their `bundle`. A spec that keeps
/// bundles refuses `save_model` and `save_requirements` with `bundled-spec`. A screen
/// written for 11 reads a spec's model by the bundle and would be refused by a core of 10.
///
/// 12: what SCE says of a spec can be asked of the revisions that were read
/// (`read_judgment`): whether the owner's acceptance holds for them, and SCE's measure of
/// the design against the list. The answer is of exactly the revisions named, whatever has
/// been saved since, so a screen shows no verdict beside a design it was not given of. A
/// screen written for 12 asks it after reading the spec, and would be refused by a core of
/// 11 with `unknown-command`.
///
/// 13: the measure `read_judgment` gives (`report.said`) is SCE's words alone: the revisions it
/// was made against, the source head and where the design and the list stand to the text
/// (`basis`, `source_head`, `model_standing`, `requirements_standing`) are no longer in it,
/// because a claim kept again for a later text would change an answer about the same bytes.
/// The snapshot says where they stand. A screen written for 12 reads those four out of the
/// measure and cannot read a core of 13; a screen written for 13 does not find them in a core
/// of 12. 12 was pushed with the measure in its older form, so the answer changed under the
/// number and the number moves: **a change to an answer moves the version even for a command
/// that is new in the version before it, once a build that has it has been pushed.**
///
/// 14: the ways a person reaches a model are settings the core keeps apart from the specs
/// (`list_connections`, `read_connection`, `save_connection`, `delete_connection`,
/// `set_default_connection`), and the table of sign-in routes the build uses can be read
/// (`read_auth_policy`). `describe` also says which entrance asked, whether it has a settings
/// folder and whether it may change what is in it. A screen written for 14 asks for them and
/// would be refused by a core of 13 with `unknown-command`.
///
/// 15: Codex is a client the application can ask about, like Claude Code: `read_codex_status`
/// says where it is, whether this build verified that version, and who is signed in by each of
/// the three sources a connection can take its credential from, and `find_clients` lists the
/// Codex programs it finds beside Claude Code's (`codex`), which a connection to Codex may name
/// as its program. A screen written for 15 asks for them, and would be refused by a core of 14
/// with `unknown-command` for the one and would not find the other in the answer; a screen
/// written for 14 reads a core of 15's `find_clients` all the same, and ignores what it does not
/// know.
///
/// 15 is also where what was pushed under 14 is told apart. 14 was first pushed without
/// `find_clients` and without the program's `path` in what `read_claude_status` says of an
/// installed client, and both were added under the same number, so a screen that requires the
/// `path` met a core of 14 that did not give it, and failed in the middle of a view instead of
/// being told the versions differ. A screen of 15 is refused by a core of 14 at once, by name.
///
/// 16: a model server of the person's can be asked what it is (`read_server_status`): whether it
/// is there, whether its certificate is accepted when it is reached over https, whether it wants a
/// key, and which models it lists. A connection to one (`adapter: "local"`) was always possible
/// to save and now can be run, so a screen written for 16 offers to register one and would be
/// refused by a core of 15 with `unknown-command`.
///
/// 17: the status of a client (`read_claude_status`, `read_codex_status`) can be asked of a
/// connection (`connection`): the program that answers is the one that connection names, and not
/// the one the default connection names. A screen that edits a connection that is not the default
/// (a program kept for a client nobody is signed in to does not make it the default) asks about the
/// connection it edits, and a core of 16 would refuse that argument as one it does not know.
///
/// 18: the same commands take the program the screen is about to keep (`executable`), which is not
/// kept yet: the status is of that program, or of the application's own choice when it is `null`,
/// and a program the application did not find is refused. A person who chose another program
/// is told who is signed in to that one, and not to the one that was kept; a core of 17 would
/// refuse the argument as one it does not know.
///
/// 19: a requirement list can keep the lineage it was built against (`lineage` of
/// `save_requirements` and of `save_request_candidate`; `lineage` in the list `read_requirements`
/// and the snapshot give back, present only for a list that has one), so that an id is issued once
/// across the revisions of a spec's text. A list saved without one onto a spec that holds one is
/// refused as `lineage-dropped`, and so is a candidate published that way. A screen written for
/// 18 would not know the argument or the refusal, and a core of 18 would refuse the argument as
/// one it does not know.
///
/// 20: what moved in a spec's design since the owner accepted it can be asked of the product
/// requirement by requirement (`read_acceptance_delta`): the product's own comparison of the rows
/// the owner was shown with the spec's design as it is now, read with the acceptance as one state
/// of the spec, and the manifest the acceptance pinned. A report and not a verdict: the acceptance
/// still lapses by its bytes (`read_acceptance`). A screen written for 19 does not ask for it, and
/// a core of 19 would refuse it as `unknown-command`.
///
/// 21: a requirement list's lineage is judged when the list is saved or published, and not only
/// for being lost: one that is not a lineage (`lineage-unusable`), that is not the lineage of the
/// list beside it (`lineage-of-another-list`: it names the manifest and the sidecar by their
/// digests, so a list with its sidecar left out is refused too) or that does not continue the
/// spec's (`lineage-not-continued`: an id taken back, a retired id lived again, a history
/// rewritten, an id numbered twice) is refused, so that an id means one requirement for ever
/// whatever built the list. A screen written for 20 does not know these refusals, and a core of
/// 20 would save such a list.
///
/// 22: what a revision of a spec did can be asked of the product requirement by requirement
/// (`read_revision_report`): the words of each requirement from the spec's own lineages beside
/// what the design's evidence did, whether the revision stayed within the reach of what changed,
/// and the page an owner reads, judged by the product and not by the client that revises. A
/// screen written for 21 does not ask for it, and a core of 21 would refuse it as
/// `unknown-command`. A list the product cannot say anything of the words of is refused as
/// `revision-not-judged`, in the sentence a person is told, and a report asked for after the
/// text was changed and before the model or the list was written again for it is refused as
/// `revision-not-current` (as is `read_acceptance_delta`), naming which is behind (a refusal kind a screen shows in its words
/// and does not branch on, so it is no new version).
///
/// 23: the owner can ask for every requirement of the list a request makes to be issued a new id
/// (`request_generation`'s `fresh_ids`, kept on the request and said in its reply as
/// `fresh_ids`), and a list that carries an id is then refused (`fresh-ids-not-issued`) before it
/// is published. A screen written for 22 neither sends it nor reads it, and a core of 22 would
/// refuse the argument as `bad-request`.
///
/// 24: a requirement list made before lineages is read with the lineage adopting it makes
/// (`read_requirements`' `lineage`, with `lineage_adopted` set to `true`: derived from the manifest
/// and the sidecar, stored nowhere), and the store holds the next list to it as to a kept one
/// (`lineage-dropped`, `lineage-not-continued`), so that the first lineage of a spec is judged like
/// every other. A client of 23 was handed no lineage for such a list and built its next list as a
/// first one, which a core of 24 refuses; a core of 23 would keep it.
///
/// 25: the commands are the specs' alone. The settings, the sign-in table and the status of a
/// person's clients and servers (`list_connections` to `set_default_connection`) are an
/// application's, which answers them in front of these and versions them itself; `describe` lists
/// only these, and says nothing of who asked (`entrance`, `settings`, `writes_settings`,
/// `starts_programs` are the application's too). A request may still name a connection, and the
/// entrance makes its pin ([`Pins`]). A caller of 24 that asked `sce-spec` for a setting was told
/// `no-settings`, and is now told `unknown-command`.
///
/// 26: what 25 called a work is a spec, in the name of every command (`list_works` is
/// `list_specs`, `read_work` is `read_spec`, and so on), of every argument and reply field
/// (`work` is `spec`, `works` is `specs`), and of the folder on disk (`works` is `specs`,
/// `work.json` is `spec.json`, format `sce-work` is `sce-spec`). A caller of 25 names commands this
/// core does not have and is told `unknown-command`; a folder of 25 is not read, and is moved once,
/// by hand, and not by a reader of two formats.
///
/// 27: a request can wait for a place of its connection, and the store says so
/// (`read_queue`): the queued requests that wait behind the connection's running ones, with how many
/// are running, how many places the connection has and how many requests will be taken first. An
/// executor that claims with the places of its connection (the application does, in its own
/// calls) is refused `lane-full` when none is free. A screen written for 26 does not ask for the
/// queue, and a core of 26 would refuse it as `unknown-command`.
///
/// 28: a request that failed says how: `fail_request` takes a `failure` (a `kind` the store holds,
/// and a `cause` word the executor chose) and every reply that is a request says it as `failure`
/// (`null` for a request that did not fail, and for one that failed before kinds were kept, which
/// reads as `unclassified`). What a run does is kept as a journal beside its request: an executor
/// says it one event at a time (`record_request_event`) and anybody reads what it adds up to and its
/// latest events (`read_request_journal`), for a request that ended as for one that runs. A screen
/// written for 28 shows whose a failure is to fix and what a run has done so far; a core of 27
/// would refuse the commands as `unknown-command` and the `failure` argument as `bad-request`.
///
/// 29: the model an executor wrote for a request (`read_request_candidate`) says the documents it is
/// made of and which is the entry, as `read_model` does (`documents`, `entry`; `null` for a text that
/// does not read as a model), so that a screen can show the last design a run that failed was
/// holding as the documents it is and not as the text it was kept in. A screen written for 28 reads
/// the `text` and shows a model of several documents as a blob; a core of 28 gives it no documents.
///
/// 30: a request that waits for a place says since when the place it waits for has been held
/// (`running_since`, the moment the attempt that has run longest of the connection's running
/// requests was taken), and `read_queue` says the store's `now`, so that a screen can say how long
/// the one ahead has run. A screen written for 29 does not say it; a core of 29 gives it neither.
///
/// 31: a spec keeps the originals it was brought in from, as bundles of members named by hash
/// (`add_original`, `read_original`, `list_originals`, `read_original_member`), and what an
/// extractor read out of a member (`save_extraction`, `read_extraction`, `list_extractions`). A
/// screen written for 31 imports a document and shows its blocks beside it; a core of 30 refuses
/// the seven commands as `unknown-command`, and a screen written for 30 has no use for them.
pub const COMMAND_SET_VERSION: u32 = 31;

/// A command that did not do what was asked, in a shape every shell can pass on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommandError {
    /// What a program branches on (see [`StoreError::kind`]); `bad-request` and
    /// `unknown-command` are this layer's own.
    pub kind: String,
    /// For a person.
    pub message: String,
    /// Facts a caller acts on, when there are any. For `conflict`, the base the
    /// caller wrote from and the revision that is current now.
    #[serde(skip_serializing_if = "Value::is_null")]
    pub detail: Value,
}

impl From<StoreError> for CommandError {
    fn from(error: StoreError) -> Self {
        let detail = match &error {
            StoreError::Conflict { base, current } => json!({ "base": base, "current": current }),
            StoreError::Refused { detail, .. } => detail.clone(),
            _ => Value::Null,
        };
        CommandError {
            kind: error.kind().to_string(),
            message: error.to_string(),
            detail,
        }
    }
}

impl From<RenderError> for CommandError {
    fn from(error: RenderError) -> Self {
        let detail = match &error {
            RenderError::Refused { code, .. } => json!({ "code": code }),
            RenderError::TimedOut { seconds } => json!({ "seconds": seconds }),
            _ => Value::Null,
        };
        CommandError {
            kind: error.kind().to_string(),
            message: error.to_string(),
            detail,
        }
    }
}

impl From<AnswersError> for CommandError {
    fn from(error: AnswersError) -> Self {
        CommandError {
            kind: match error {
                AnswersError::Invalid(_) => "invalid-answers",
                AnswersError::Corrupt(_) => "corrupt",
            }
            .to_string(),
            message: error.to_string(),
            detail: Value::Null,
        }
    }
}

impl From<ModelError> for CommandError {
    fn from(error: ModelError) -> Self {
        CommandError {
            kind: match error {
                ModelError::Invalid(_) => "invalid-model",
                ModelError::Corrupt(_) => "corrupt",
            }
            .to_string(),
            message: error.to_string(),
            detail: Value::Null,
        }
    }
}

impl From<RequirementsError> for CommandError {
    fn from(error: RequirementsError) -> Self {
        CommandError {
            kind: match error {
                RequirementsError::Invalid(_) => "invalid-requirements",
                RequirementsError::Corrupt(_) => "corrupt",
            }
            .to_string(),
            message: error.to_string(),
            detail: Value::Null,
        }
    }
}

impl From<AcceptanceCorrupt> for CommandError {
    fn from(error: AcceptanceCorrupt) -> Self {
        CommandError {
            kind: "corrupt".to_string(),
            message: error.to_string(),
            detail: Value::Null,
        }
    }
}

impl CommandError {
    fn bad_request(message: impl Into<String>) -> Self {
        CommandError {
            kind: "bad-request".to_string(),
            message: message.into(),
            detail: Value::Null,
        }
    }
}

/// The arguments of a command that takes none.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Empty {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateSpec {
    title: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OneSpec {
    id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RevisionReport {
    id: String,
    /// Print the specification's sentences on the page, which then says it is a local artefact.
    #[serde(default)]
    sentences: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadSource {
    id: String,
    #[serde(default)]
    revision: Option<Revision>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OriginalMemberArgs {
    path: String,
    role: Role,
    /// The member's bytes, as standard base64 with padding.
    data: String,
    /// For a rendition: the path of the member it was made from.
    #[serde(default)]
    of: Option<String>,
    /// For a rendition: what drew it.
    #[serde(default)]
    renderer: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AddOriginal {
    id: String,
    #[serde(default)]
    label: Option<String>,
    members: Vec<OriginalMemberArgs>,
    /// Paths the original refers to and was not brought with.
    #[serde(default)]
    missing: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OneOriginal {
    id: String,
    original: Revision,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OriginalMember {
    id: String,
    original: Revision,
    path: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveExtraction {
    id: String,
    original: Revision,
    /// The path of the member of the original the extraction was read from.
    member: String,
    /// The extraction, kept exactly as given.
    text: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OneExtraction {
    id: String,
    digest: Revision,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListExtractions {
    id: String,
    #[serde(default)]
    original: Option<Revision>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveSource {
    id: String,
    text: String,
    /// Absent or `null` means "this is the spec's first text".
    #[serde(default)]
    base: Option<Revision>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveModel {
    id: String,
    /// The model as one document. Exactly one of `text` and `documents` is given.
    #[serde(default)]
    text: Option<String>,
    /// The model as several documents that name each other, each under the file name
    /// its imports know it by.
    #[serde(default)]
    documents: Option<Vec<Document>>,
    /// Which of `documents` SCE is asked about: the first when absent.
    #[serde(default)]
    entry: Option<String>,
    /// Absent or `null` means "this is the spec's first model".
    #[serde(default)]
    base: Option<Revision>,
    /// The source revision the writer read; absent or `null` when it cannot say.
    #[serde(default)]
    written_for: Option<Revision>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Figures {
    id: String,
    #[serde(default)]
    revision: Option<Revision>,
    #[serde(default)]
    page: Option<String>,
    #[serde(default)]
    lexicon: Option<String>,
    #[serde(default)]
    min_pt: Option<f64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveAnswers {
    id: String,
    /// The answers as the owner now has them: question id to words. A question
    /// that is not here is not answered.
    answers: BTreeMap<String, String>,
    /// Absent or `null` means "this is the spec's first set of answers".
    #[serde(default)]
    base: Option<Revision>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveRequirements {
    id: String,
    /// The manifest, as the authoring package wrote it, byte for byte.
    manifest: String,
    /// The sidecar of quoted sentences, when the list came with one.
    #[serde(default)]
    sidecar: Option<String>,
    /// The requirement lineage the list was built against and extends, as the authoring
    /// package returned it, byte for byte. A list that follows a revision of its text gives
    /// it (a spec that holds one refuses a list without: `lineage-dropped`).
    #[serde(default)]
    lineage: Option<String>,
    /// Absent or `null` means "this is the spec's first requirement list".
    #[serde(default)]
    base: Option<Revision>,
    /// The source revision the list was read from; absent or `null` when it cannot say.
    #[serde(default)]
    written_for: Option<Revision>,
}

/// The revisions the owner was shown when they pressed accept.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Expect {
    source: Revision,
    model: Revision,
    requirements: Revision,
    /// Absent or `null` when the owner had answered nothing.
    #[serde(default)]
    answers: Option<Revision>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Accept {
    id: String,
    expect: Expect,
}

/// What SCE is asked about: the revisions that were read, and the acceptance whose standing
/// is wanted when there is one.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadJudgment {
    id: String,
    basis: Basis,
    /// The acceptance whose standing is wanted; absent or `null` when nothing is accepted.
    #[serde(default)]
    acceptance: Option<Revision>,
}

/// The revisions a request is asked about: the text, and the owner's answers when they
/// had given some.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectInputs {
    source: Revision,
    /// Absent or `null` when the owner had answered nothing.
    #[serde(default)]
    answers: Option<Revision>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestGeneration {
    id: String,
    /// What makes the same call sent again the request it already made.
    key: String,
    /// Where it is asked from: `gui`, or the name of a client.
    origin: String,
    expect: ExpectInputs,
    /// Replace the open request of the spec, if there is one.
    #[serde(default)]
    supersede: bool,
    /// The connection it is made for, at the revision the screen read. The entrance makes the pin
    /// of it ([`Pins`]) from what it keeps; the caller does not say what a run needs.
    #[serde(default)]
    connection: Option<ConnectionRef>,
    /// The owner asks for every requirement of the list the request makes to be issued a new id
    /// (ADR 0012). Absent is not asked.
    #[serde(default)]
    fresh_ids: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OneRequest {
    id: String,
    request: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ClaimRequest {
    id: String,
    request: String,
    holder: String,
    #[serde(default)]
    ttl_seconds: Option<u64>,
    /// Take a request an executor let go of, as its next attempt.
    #[serde(default)]
    resume: bool,
    /// The connection the executor runs for. A request is taken by the executor of the
    /// connection it was made for, and one that was made for none by an executor that offers none.
    #[serde(default)]
    connection: Option<ConnectionRef>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HeartbeatRequest {
    id: String,
    request: String,
    holder: String,
    attempt: u32,
    #[serde(default)]
    ttl_seconds: Option<u64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveCandidate {
    id: String,
    request: String,
    holder: String,
    attempt: u32,
    /// The model as one document. At most one of `text` and `documents` is given; neither
    /// means this call writes the requirement list alone.
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    documents: Option<Vec<Document>>,
    #[serde(default)]
    entry: Option<String>,
    /// The requirement list, as `save_requirements` takes it. Absent when this call writes
    /// the model alone.
    #[serde(default)]
    manifest: Option<String>,
    #[serde(default)]
    sidecar: Option<String>,
    /// The lineage of the list, as `save_requirements` takes it. Belongs to a `manifest`.
    #[serde(default)]
    lineage: Option<String>,
    /// The version of the working instructions the executor was given, in its own words
    /// (`claude-code/0123456789ab`); a bundle records it.
    #[serde(default)]
    instructions: Option<String>,
}

/// A check only the executor's side can run, as it reports it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ClientCheck {
    name: String,
    /// `accepted` or `refused`.
    verdict: String,
    #[serde(default)]
    generator: Option<String>,
    #[serde(default)]
    digest: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FinishRequest {
    id: String,
    request: String,
    holder: String,
    attempt: u32,
    /// What the executor checked that the core cannot: it is kept as reported, beside the
    /// check the core runs of the model itself.
    #[serde(default)]
    checks: Vec<ClientCheck>,
    #[serde(default)]
    lexicon: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OneBundle {
    id: String,
    /// The bundle; the spec's current one when absent.
    #[serde(default)]
    revision: Option<Revision>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FailRequest {
    id: String,
    request: String,
    holder: String,
    attempt: u32,
    reason: String,
    /// How it failed. An executor that does not say is `unclassified`, as is every request that
    /// failed before kinds existed; a kind that is none of the store's is refused.
    #[serde(default)]
    failure: Option<FailureWords>,
}

/// How a failure is said on the wire. It is not [`Failure`] itself: that is also what is kept, and
/// a record read must not refuse a field a newer writer added, while a caller's word is held to
/// what is read, as every other argument is.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FailureWords {
    kind: FailureKind,
    #[serde(default)]
    cause: Option<String>,
}

impl From<FailureWords> for Failure {
    fn from(words: FailureWords) -> Self {
        Failure {
            kind: words.kind,
            cause: words.cause,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordEvent {
    id: String,
    request: String,
    holder: String,
    attempt: u32,
    event: Event,
}

/// A journal as the screens read it: what it adds up to, with the moments as text, and the latest
/// events. `now` is the store's moment, so that a screen measures a run that is going on by the
/// clock that wrote its events and not by its own.
fn journal_json(journal: &Journal, now: String) -> Value {
    let progress = &journal.progress;
    json!({
        "progress": {
            "entries": progress.entries,
            "tries": progress.tries,
            "turns": progress.turns,
            "checks": progress.checks,
            "refused": progress.refused,
            "drafts_refused": progress.drafts_refused,
            "not_drafts": progress.not_drafts,
            "handovers": progress.handovers,
            "last_refusal": progress.last_refusal,
            "started_at": progress.started_at.map(utc_timestamp),
            "last_at": progress.last_at.map(utc_timestamp),
        },
        "tail": journal.tail.iter().map(|entry| json!({
            "at": utc_timestamp(entry.at),
            "event": entry.event,
        })).collect::<Vec<_>>(),
        "full": journal.full,
        "now": now,
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReportAdapter {
    name: String,
    kind: String,
    capabilities: Vec<String>,
    #[serde(default)]
    version: Option<String>,
}

/// An executor's claim as the screens read it: who, which attempt, and when it ran or runs out.
fn lease_json(lease: &Lease) -> Value {
    json!({
        "holder": lease.holder,
        "attempt": lease.attempt,
        "granted_at": utc_timestamp(lease.granted_at),
        "expires_at": utc_timestamp(lease.expires_at),
    })
}

/// A request as the screens read it. `state` is where it stands as the clock reads it now;
/// `stored_state` is what was last written. They differ for a lease that ran out: nobody
/// wrote that down, and the request is interrupted all the same.
fn request_json(spec: &SpecId, view: &RequestView) -> Value {
    let request = &view.request;
    json!({
        "id": request.id,
        "spec": spec.as_str(),
        "seq": request.seq,
        "key": request.key,
        "origin": request.origin,
        "state": view.state,
        "stored_state": request.state,
        "attempt": request.attempt,
        "inputs": {
            "source": request.inputs.source,
            "answers": request.inputs.answers,
        },
        "created_at": request.created_at,
        "lease": request.lease.as_ref().map(lease_json),
        "candidate": request.candidate,
        "outcome": request.outcome,
        "pin": request.pin,
        "fresh_ids": request.fresh_ids,
        "ended_at": request.ended_at,
        "note": request.note,
        "failure": request.failure,
    })
}

/// A shell's word about its executor as the screens read it, and whether the shell is still
/// saying it.
fn host_json(status: &HostStatus) -> Value {
    let host = &status.host;
    json!({
        "name": host.name,
        "hosting": host.hosting,
        "reason": host.reason,
        "client_version": host.client_version,
        "waiting": host.waiting,
        "seen_at": host.seen_at,
        "live": status.live,
    })
}

/// An adapter as the screens read it, and whether it is there now.
fn adapter_json(status: &AdapterStatus) -> Value {
    let adapter = &status.adapter;
    json!({
        "name": adapter.name,
        "kind": adapter.kind,
        "capabilities": adapter.capabilities,
        "version": adapter.version,
        "seen_at": adapter.seen_at,
        "live": status.live,
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewModel {
    id: String,
    #[serde(default)]
    revision: Option<Revision>,
    #[serde(default)]
    lexicon: Option<String>,
}

/// How a model stands to the text now, as one word every shell uses the same
/// way: `current` when it was written for the text as it is, `behind` when it
/// was written for an earlier one, `unstated` when its writer did not say.
fn standing(written_for: Option<&Revision>, source_head: Option<&Revision>) -> &'static str {
    match (written_for, source_head) {
        (Some(written), Some(head)) if written == head => "current",
        (Some(_), _) => "behind",
        (None, _) => "unstated",
    }
}

/// The arguments of a command, or `bad-request` in the words the decoder said. Public so that an
/// application's own commands read theirs as these do.
pub fn arguments<T: for<'de> Deserialize<'de>>(args: Value) -> Result<T, CommandError> {
    // A caller that sends nothing means an empty object.
    let args = if args.is_null() { json!({}) } else { args };
    serde_json::from_value(args).map_err(|e| CommandError::bad_request(e.to_string()))
}

/// What a command answers, as JSON.
pub fn answer<T: Serialize>(value: &T) -> Result<Value, CommandError> {
    serde_json::to_value(value).map_err(|e| CommandError::bad_request(e.to_string()))
}

fn spec_id(text: &str) -> Result<SpecId, CommandError> {
    SpecId::parse(text).map_err(CommandError::from)
}

fn none_saved(what: &str, id: &SpecId) -> CommandError {
    CommandError::from(StoreError::NotFound {
        what: format!("{what} of spec `{}` (none was saved)", id.as_str()),
    })
}

/// What a spec's four chains hold now, as the product is asked about them: the
/// text, the model, the requirement list and the owner's answers, each at the
/// revision it was read at (`basis`), and the source claims the model's and the
/// list's saves made.
struct SpecNow {
    snapshot: Snapshot,
    basis: Basis,
    model_written_for: Option<Revision>,
    requirements_written_for: Option<Revision>,
}

/// Read the spec as it is now. A chain with nothing in it is `not-found`, naming
/// which: the requirement list and the model are not things the application makes up.
fn spec_now<C: Clock>(store: &SpecStore<C>, id: &SpecId) -> Result<SpecNow, CommandError> {
    // One state of the spec, not four reads a save or a publication can land between: the
    // model and the list of a bundle are one generation, and a design is accepted against
    // them together.
    let state = store.read_spec_snapshot(id)?;
    let source = state.source.ok_or_else(|| none_saved("a text", id))?;
    let model = state.model.ok_or_else(|| none_saved("a model", id))?;
    let requirements = state
        .requirements
        .ok_or_else(|| none_saved("a requirement list", id))?;
    now_of(source, model, requirements, state.answers)
}

/// What the product is asked about and nothing more: the content of the revisions a basis names.
///
/// It carries no claim of which text the model and the list were written for. The same bytes can
/// be kept again for a text that came later, and that is a claim made after the revisions were
/// read: the store answers a revision with the latest claim made of it, so a judgment that
/// reported where the design stands to the text would change with a save that did not change
/// the design. Where it stands is a fact of one read of the spec (`read_spec_snapshot`, which
/// says it beside the revisions), and a judgment is a function of the revisions alone.
struct Revisions {
    snapshot: Snapshot,
    basis: Basis,
}

/// The content of the revisions `basis` names, wherever the spec is now. A revision's file is
/// never rewritten, so what is read is what was read when the basis was taken, whatever has
/// been saved since; one the spec does not keep is `not-found`.
fn spec_at<C: Clock>(
    store: &SpecStore<C>,
    id: &SpecId,
    basis: &Basis,
) -> Result<Revisions, CommandError> {
    let source = store.read_source(id, Some(&basis.source))?;
    let model = store.read_model(id, Some(&basis.model))?;
    let requirements = store.read_requirements(id, Some(&basis.requirements))?;
    let answers = match &basis.answers {
        None => None,
        Some(revision) => store.read_answers(id, Some(revision))?,
    };
    let now = now_of(
        source.ok_or_else(|| none_saved("a text", id))?,
        model.ok_or_else(|| none_saved("a model", id))?,
        requirements.ok_or_else(|| none_saved("a requirement list", id))?,
        answers,
    )?;
    Ok(Revisions {
        snapshot: now.snapshot,
        basis: now.basis,
    })
}

/// What the product is asked about, from the four parts of a spec and the revisions they are.
fn now_of(
    source: SourceText,
    model: ModelText,
    requirements: RequirementsText,
    answers: Option<AnswersText>,
) -> Result<SpecNow, CommandError> {
    let basis = Basis {
        source: source.revision.clone(),
        model: model.revision.clone(),
        requirements: requirements.revision.clone(),
        answers: answers.as_ref().map(|a| a.revision.clone()),
    };
    Ok(SpecNow {
        snapshot: Snapshot {
            model: ModelFiles::parse(&model.text)?,
            requirements: Requirements::parse(&requirements.text)?,
            source: source.text,
            answers: answers.map(|a| a.text),
        },
        basis,
        model_written_for: model.written_for,
        requirements_written_for: requirements.written_for,
    })
}

/// What a revision did (`read_revision_report`) and what moved since the owner accepted
/// (`read_acceptance_delta`) are about the model and the list written for the text the spec has
/// now. When the text was changed and one of them was not written again, the list the spec
/// holds is the list the owner accepted (or an earlier one) and comparing the two would call it
/// "all carried over" or "unchanged" for a revision nobody made. Only a part known to be written for
/// an earlier text is held back: one that cannot say what it was written for is judged as it
/// always was (an acceptance is never taken of one, so it is one saved after).
fn refuse_what_is_behind_the_text(now: &SpecNow) -> Result<(), CommandError> {
    let head = &now.basis.source;
    let requirements = standing(now.requirements_written_for.as_ref(), Some(head));
    let model = standing(now.model_written_for.as_ref(), Some(head));
    let behind: Vec<&str> = [("requirement list", requirements), ("model", model)]
        .into_iter()
        .filter(|(_, standing)| *standing == "behind")
        .map(|(name, _)| name)
        .collect();
    if behind.is_empty() {
        return Ok(());
    }
    let (names, verb) = (
        behind.join(" and the "),
        if behind.len() == 1 { "was" } else { "were" },
    );
    Err(CommandError::from(StoreError::refused(
        "revision-not-current",
        format!(
            "the specification was changed after the {names} {verb} written for it, so a report \
             would compare the list the owner accepted with one that does not answer the text \
             as it is now, and call that a revision. Ask for the report again after you \
             generate the {names} for the text as it is now"
        ),
        json!({ "requirements": requirements, "model": model, "source_head": head }),
    )))
}

/// A saved model as the screens read it: its revision and what it was written for,
/// the entry's text (what a screen written for one document shows), and every
/// document of the model, the entry first. A model of one document is listed under
/// the name `model.scxml`.
fn model_json(model: &ModelText, files: &ModelFiles) -> Value {
    json!({
        "revision": model.revision,
        "written_for": model.written_for,
        "text": files.entry_text(),
        "entry": files.entry_file(),
        "documents": files.documents(),
    })
}

/// The model an executor wrote for a request, as the screens read it: the text as it was kept, and
/// the documents it is made of with the one that is the entry, as `read_model` says them. A text
/// that does not read as a model (a client may keep what it likes in a candidate, which nothing has
/// checked) has no documents: `entry` and `documents` are `null`, and the text is what there is.
fn candidate_model_json(revision: &Revision, text: &str) -> Value {
    let files = ModelFiles::parse(text).ok();
    json!({
        "revision": revision,
        "text": text,
        "entry": files.as_ref().map(ModelFiles::entry_file),
        "documents": files.as_ref().map(ModelFiles::documents),
    })
}

/// The owner's answers as the screens read them: the revision, and each answer with the
/// time its words last changed.
fn answers_json(saved: &AnswersText) -> Result<Value, CommandError> {
    Ok(json!({
        "revision": saved.revision,
        "entries": Answers::parse(&saved.text)?.entries(),
    }))
}

/// A saved requirement list as the screens read it: its revision, the source it was
/// written for, the manifest and the sidecar of quoted sentences, and the lineage it was built
/// against when it has one. `lineage` is absent, and not `null`, for a list without one that
/// cannot be adopted (it keeps no sidecar).
///
/// A list made before lineages is given the lineage adopting it makes, with `lineage_adopted`
/// set to `true` (command set 24): it is derived from the manifest and the sidecar and stored
/// nowhere, and it is what the next list is built against and what the store holds it to
/// (ADR 0012). A client is not left to adopt it, which none was told to.
fn requirements_json(saved: &RequirementsText) -> Result<Value, CommandError> {
    let list = Requirements::parse(&saved.text)?;
    let mut reply = json!({
        "revision": saved.revision,
        "written_for": saved.written_for,
        "manifest": list.manifest,
        "sidecar": list.sidecar,
    });
    if let Some(lineage) = &list.lineage {
        reply["lineage"] = Value::String(lineage.clone());
    } else if let Some(adopted) = list.adopted_lineage() {
        reply["lineage"] = Value::String(format!("{adopted:#}\n"));
        reply["lineage_adopted"] = Value::Bool(true);
    }
    Ok(reply)
}

/// An acceptance as the screens read it, without the product's word on whether it still
/// holds: when it was made and on which channel, what it was taken from, and what the
/// design left open.
fn acceptance_json(revision: &Revision, acceptance: &Acceptance) -> Value {
    json!({
        "revision": revision,
        "accepted_at": acceptance.accepted_at,
        "channel": acceptance.channel,
        "basis": acceptance.basis,
        "open": acceptance.open,
    })
}

/// What the product measured of a design against a list, in its words: who measured, what the
/// list is a denominator of, each requirement's outcome, and the page the owner reads.
fn measured_json(report: &RequirementsReport) -> Value {
    json!({
        "generator": report.generator,
        "denominator": report.denominator,
        "outcomes": report.outcomes,
        "page": report.page,
        "page_refusal": report.page_refusal,
    })
}

/// What the product measured of the spec as `now` puts it, with the revisions it was measured
/// against and where the model and the list stand to the text they were measured beside. The
/// source head is the text `now` holds, and not a later read of it.
fn report_json(now: &SpecNow, report: &RequirementsReport) -> Value {
    let source_head = &now.basis.source;
    let mut json = measured_json(report);
    json["basis"] = json!(now.basis);
    json["source_head"] = json!(source_head);
    json["model_standing"] = json!(standing(now.model_written_for.as_ref(), Some(source_head)));
    json["requirements_standing"] = json!(standing(
        now.requirements_written_for.as_ref(),
        Some(source_head)
    ));
    json
}

/// The product's word on whether an acceptance holds, and its one sentence of what moved.
fn standing_of(outcome: CheckOutcome) -> (&'static str, Value) {
    match outcome {
        CheckOutcome::Holds => ("holds", Value::Null),
        CheckOutcome::Lapsed { says } => ("lapsed", Value::String(says)),
    }
}

/// The product not answering, as data: the kind a program branches on, its words, and the
/// code it refused with. It is what a caller shows beside what it did read.
fn refusal_json(error: RenderError) -> Value {
    let error = CommandError::from(error);
    json!({
        "kind": error.kind,
        "message": error.message,
        "code": error.detail.get("code").cloned().unwrap_or(Value::Null),
    })
}

/// What the product says of the revisions `of` names: whether the owner's acceptance (`record`,
/// when there is one; `null` when there is none) holds for them, and its measure of the design
/// against the list. Each is either `{"said": ...}` or `{"refused": ...}` for the product not
/// answering, because the caller still has the spec it read when the product does not answer: a
/// model SCE cannot draw is still a model. The measure is the product's words and nothing of
/// where the design stands to the text (`Revisions` says why).
fn judgment_json(renderer: &dyn Product, of: &Revisions, record: Option<&Acceptance>) -> Value {
    let acceptance = match record {
        None => Value::Null,
        Some(accepted) => match renderer.check_acceptance(&of.snapshot, &accepted.record) {
            Ok(outcome) => {
                let (standing, lapse) = standing_of(outcome);
                json!({ "said": { "standing": standing, "lapse": lapse } })
            }
            Err(error) => json!({ "refused": refusal_json(error) }),
        },
    };
    let report = match renderer.report_requirements(&of.snapshot) {
        Ok(report) => json!({ "said": measured_json(&report) }),
        Err(error) => json!({ "refused": refusal_json(error) }),
    };
    json!({ "basis": of.basis, "acceptance": acceptance, "report": report })
}

/// A model as its caller gave it: `text` (one document), or `documents` (several that name
/// each other) with the `entry` the product is asked about. Both, or neither, is refused.
fn model_files_of(
    text: Option<String>,
    documents: Option<Vec<Document>>,
    entry: Option<String>,
) -> Result<ModelFiles, CommandError> {
    match (text, documents) {
        (Some(text), None) if entry.is_none() => Ok(ModelFiles::single(text)),
        (Some(_), None) => Err(CommandError::bad_request(
            "`entry` names one of several `documents`; a model given as `text` has one",
        )),
        (None, Some(documents)) => Ok(ModelFiles::set(documents, entry.as_deref())?),
        _ => Err(CommandError::bad_request(
            "give the model as `text` (one document) or as `documents` (several), \
             not both and not neither",
        )),
    }
}

/// What SCE says of `files`, the model of spec `id`: its check and its pseudocode page.
fn review_model(
    renderer: &dyn Product,
    id: &SpecId,
    files: &ModelFiles,
    lexicon: Option<&str>,
) -> Result<Review, RenderError> {
    let others: Vec<Document> = files.others().into_iter().cloned().collect();
    renderer.review(&ReviewRequest {
        model: files.entry_text(),
        name: Some(id.slug()),
        entry_file: files.entry_name(),
        siblings: &others,
        lexicon,
    })
}

/// The executor of `request` says it is done: the core checks the model it wrote itself, and
/// when the check accepts, the model and the requirement list become the spec's together.
///
/// What an executor reports of a check only it can run is `reported`, kept as it says. The
/// model is checked by the product the core asks and not by what the executor says of its own
/// model, so a bundle always carries a verdict the core reached. A request already completed is
/// said again without asking the product a second time.
///
/// Shared by the `complete_request` command and by the runner that hosts an executor, so that
/// the two cannot come to different words about what a completion is.
#[allow(clippy::too_many_arguments)]
pub fn complete_generation<C: Clock>(
    store: &SpecStore<C>,
    renderer: &dyn Product,
    id: &SpecId,
    request: &str,
    holder: &str,
    attempt: u32,
    reported: Vec<BundleCheck>,
    lexicon: Option<&str>,
) -> Result<Published, CommandError> {
    let mut checks = Vec::new();
    // What the product said when it refused, to go with the store's refusal: the store knows
    // that a check refused and not what the product wrote.
    let mut records = Value::Null;
    if store.read_request(id, request)?.state.is_open() {
        if let Some((revision, text)) = store.read_candidate(id, request)?.model {
            let files = ModelFiles::parse(&text)?;
            let review = review_model(renderer, id, &files, lexicon)?;
            if review.check.verdict == Verdict::Refused {
                records = json!(review.check.records);
            }
            checks.push(core_check_of(&review, revision));
        }
    }
    checks.extend(reported);
    match store.publish_candidate(id, request, holder, attempt, checks) {
        Err(StoreError::Refused {
            kind: "check-refused",
            message,
            mut detail,
        }) if !records.is_null() => {
            detail["records"] = records;
            Err(CommandError::from(StoreError::Refused {
                kind: "check-refused",
                message,
                detail,
            }))
        }
        done => Ok(done?),
    }
}

/// A check the executor reports, as a bundle keeps it: as reported, and said to be so.
fn client_check_of(check: ClientCheck) -> Result<BundleCheck, CommandError> {
    if check.verdict != "accepted" && check.verdict != "refused" {
        return Err(CommandError::bad_request(format!(
            "the verdict of check `{}` is `accepted` or `refused`, not `{}`",
            check.name, check.verdict
        )));
    }
    Ok(BundleCheck {
        by: CheckedBy::Client,
        name: check.name,
        verdict: check.verdict,
        generator: check.generator,
        digest: check.digest,
        subject: None,
    })
}

/// The check the core ran of a candidate model, as a bundle keeps it: who ran it, what it
/// said, with which generator, the digest of the record it wrote so that what was checked is
/// pinned and not only what it came to, and the revision it ran on.
fn core_check_of(review: &Review, model: Revision) -> BundleCheck {
    BundleCheck {
        by: CheckedBy::Core,
        name: "model".to_string(),
        verdict: match review.check.verdict {
            Verdict::Accepted => "accepted",
            Verdict::Refused => "refused",
        }
        .to_string(),
        generator: review.generator.clone(),
        digest: Some(
            Revision::of(&serde_json::to_vec(&review.check).unwrap_or_default()).to_string(),
        ),
        subject: Some(model),
    }
}

/// The model of `id` as the product is asked about it: parsed into its documents,
/// the staging that goes with it, and the save it came from.
fn read_model_files<C: Clock>(
    store: &SpecStore<C>,
    id: &SpecId,
    revision: Option<&Revision>,
) -> Result<(ModelText, ModelFiles), CommandError> {
    let Some(model) = store.read_model(id, revision)? else {
        return Err(CommandError::from(StoreError::NotFound {
            what: format!("a model of spec `{}` (none was saved)", id.as_str()),
        }));
    };
    let files = ModelFiles::parse(&model.text)?;
    Ok((model, files))
}

/// Where a request made for a connection gets its pin.
///
/// The specs hold a request to the connection it names (only that connection's executor takes it)
/// and know nothing else of a connection: which ones there are, and what a run of one needs, is
/// the application's that keeps them. So `request_generation` asks the entrance it came in by.
pub trait Pins {
    /// The pin of a request made for `wanted`, or why none is made.
    fn pin_for(&self, wanted: &ConnectionRef) -> Result<Pin, CommandError>;
}

/// An entrance that keeps no connections (`sce-spec`): a request made through it names none, and
/// one that does is refused as `no-settings`.
pub struct NoPins;

impl Pins for NoPins {
    fn pin_for(&self, _wanted: &ConnectionRef) -> Result<Pin, CommandError> {
        Err(CommandError {
            kind: "no-settings".to_string(),
            message:
                "a request for a connection needs a settings folder, and this entrance has none"
                    .to_string(),
            detail: Value::Null,
        })
    }
}

/// Run the command `name` with `args` against `store`; `renderer` is the product: it draws a
/// model for `figures` and reads one for `review`. A request that names a connection is refused
/// (see [`NoPins`]); an entrance that keeps connections uses [`call_with`].
pub fn call<C: Clock>(
    store: &SpecStore<C>,
    renderer: &dyn Product,
    name: &str,
    args: Value,
) -> Result<Value, CommandError> {
    call_with(store, renderer, &NoPins, name, args)
}

/// Run the command `name` with `args` against `store`, with `pins` making the pin of a request
/// made for a connection.
pub fn call_with<C: Clock>(
    store: &SpecStore<C>,
    renderer: &dyn Product,
    pins: &dyn Pins,
    name: &str,
    args: Value,
) -> Result<Value, CommandError> {
    match name {
        "describe" => {
            arguments::<Empty>(args)?;
            Ok(json!({
                "command_set_version": COMMAND_SET_VERSION,
                "commands": COMMANDS,
                "root": store.root().display().to_string(),
            }))
        }
        "list_specs" => {
            arguments::<Empty>(args)?;
            answer(&store.list_specs()?)
        }
        "create_spec" => {
            let CreateSpec { title } = arguments(args)?;
            answer(&store.create_spec(&title)?)
        }
        "read_spec" => {
            let OneSpec { id } = arguments(args)?;
            let id = spec_id(&id)?;
            let spec = store.read_spec(&id)?;
            let head = store.head(&id)?;
            Ok(json!({ "spec": spec, "head": head }))
        }
        "read_source" => {
            let ReadSource { id, revision } = arguments(args)?;
            let source = store.read_source(&spec_id(&id)?, revision.as_ref())?;
            Ok(json!({ "source": source }))
        }
        "save_source" => {
            let SaveSource { id, text, base } = arguments(args)?;
            answer(&store.save_source(&spec_id(&id)?, &text, base.as_ref())?)
        }
        "history" => {
            let OneSpec { id } = arguments(args)?;
            Ok(json!({ "entries": store.history(&spec_id(&id)?)? }))
        }
        "save_model" => {
            let SaveModel {
                id,
                text,
                documents,
                entry,
                base,
                written_for,
            } = arguments(args)?;
            let files = model_files_of(text, documents, entry)?;
            answer(&store.save_model(
                &spec_id(&id)?,
                &files.stored_text(),
                base.as_ref(),
                written_for.as_ref(),
            )?)
        }
        "read_model" => {
            let ReadSource { id, revision } = arguments(args)?;
            let id = spec_id(&id)?;
            let saved = store.read_model(&id, revision.as_ref())?;
            let source_head = store.head(&id)?;
            let (model, standing) = match saved {
                None => (None, None),
                Some(model) => {
                    let files = ModelFiles::parse(&model.text)?;
                    let standing = standing(model.written_for.as_ref(), source_head.as_ref());
                    (Some(model_json(&model, &files)), Some(standing))
                }
            };
            Ok(json!({ "model": model, "source_head": source_head, "standing": standing }))
        }
        "model_history" => {
            let OneSpec { id } = arguments(args)?;
            Ok(json!({ "entries": store.model_history(&spec_id(&id)?)? }))
        }
        "figures" => {
            let Figures {
                id,
                revision,
                page,
                lexicon,
                min_pt,
            } = arguments(args)?;
            let id = spec_id(&id)?;
            let (model, files) = read_model_files(store, &id, revision.as_ref())?;
            let others: Vec<Document> = files.others().into_iter().cloned().collect();
            let source_head = store.head(&id)?;
            let drawn = renderer.render(&FigureRequest {
                model: files.entry_text(),
                // The product titles its figures by the document's name, which it
                // takes from the file's: the spec's own name, not `model`. A model of
                // several documents keeps the names its imports know them by.
                name: Some(id.slug()),
                entry_file: files.entry_name(),
                siblings: &others,
                page: page.as_deref(),
                lexicon: lexicon.as_deref(),
                min_pt,
            })?;
            Ok(json!({
                "model": { "revision": model.revision, "written_for": model.written_for },
                "source_head": source_head,
                "standing": standing(model.written_for.as_ref(), source_head.as_ref()),
                "generator": drawn.generator,
                "sheets": drawn.sheets,
            }))
        }
        "review" => {
            let ReviewModel {
                id,
                revision,
                lexicon,
            } = arguments(args)?;
            let id = spec_id(&id)?;
            let (model, files) = read_model_files(store, &id, revision.as_ref())?;
            let source_head = store.head(&id)?;
            let read = review_model(renderer, &id, &files, lexicon.as_deref())?;
            Ok(json!({
                "model": { "revision": model.revision, "written_for": model.written_for },
                "source_head": source_head,
                "standing": standing(model.written_for.as_ref(), source_head.as_ref()),
                "generator": read.generator,
                "check": read.check,
                "page": read.page,
                "page_refusal": read.page_refusal,
            }))
        }
        "read_answers" => {
            let ReadSource { id, revision } = arguments(args)?;
            let id = spec_id(&id)?;
            let read = store.read_answers(&id, revision.as_ref())?;
            let answers = match read {
                None => Value::Null,
                Some(saved) => answers_json(&saved)?,
            };
            Ok(json!({ "answers": answers }))
        }
        "save_answers" => {
            let SaveAnswers { id, answers, base } = arguments(args)?;
            let id = spec_id(&id)?;
            // What is held at `base` is what the new words are compared with, to
            // keep the stamp of an answer that did not change. A `base` that is not
            // current is refused by the save below, so the comparison is never made
            // against a revision somebody has since replaced.
            let held = match base.as_ref() {
                None => Answers::default(),
                Some(base) => match store.read_answers(&id, Some(base))? {
                    Some(saved) => Answers::parse(&saved.text)?,
                    None => Answers::default(),
                },
            };
            let next = held.amended(&answers, &store.now())?;
            answer(&store.save_answers(&id, &next.text(), base.as_ref())?)
        }
        "save_requirements" => {
            let SaveRequirements {
                id,
                manifest,
                sidecar,
                lineage,
                base,
                written_for,
            } = arguments(args)?;
            let list = Requirements::new(manifest, sidecar)?.with_lineage(lineage)?;
            answer(&store.save_requirements(
                &spec_id(&id)?,
                &list.stored_text(),
                base.as_ref(),
                written_for.as_ref(),
            )?)
        }
        "read_requirements" => {
            let ReadSource { id, revision } = arguments(args)?;
            let id = spec_id(&id)?;
            let saved = store.read_requirements(&id, revision.as_ref())?;
            let source_head = store.head(&id)?;
            let (requirements, standing) = match saved {
                None => (None, None),
                Some(saved) => {
                    let standing = standing(saved.written_for.as_ref(), source_head.as_ref());
                    (Some(requirements_json(&saved)?), Some(standing))
                }
            };
            Ok(json!({
                "requirements": requirements,
                "source_head": source_head,
                "standing": standing,
            }))
        }
        "requirements_report" => {
            let OneSpec { id } = arguments(args)?;
            let id = spec_id(&id)?;
            let now = spec_now(store, &id)?;
            let report = renderer.report_requirements(&now.snapshot)?;
            Ok(report_json(&now, &report))
        }
        "read_judgment" => {
            let ReadJudgment {
                id,
                basis,
                acceptance,
            } = arguments(args)?;
            let id = spec_id(&id)?;
            // SCE is asked about the revisions named and about nothing read after them, so the
            // answer is of the state the caller read and of no other: a save landing now moves
            // the spec, and the caller learns that from the heads, not from a verdict that is
            // of a design it was not shown.
            let named = spec_at(store, &id, &basis)?;
            let record = match &acceptance {
                None => None,
                Some(revision) => {
                    let saved = store
                        .read_acceptance(&id, Some(revision))?
                        .ok_or_else(|| none_saved("an acceptance", &id))?;
                    Some(Acceptance::parse(&saved.text)?)
                }
            };
            Ok(judgment_json(renderer, &named, record.as_ref()))
        }
        "accept" => {
            let Accept { id, expect } = arguments(args)?;
            let id = spec_id(&id)?;
            let now = spec_now(store, &id)?;
            let source_head = store.head(&id)?;

            // The owner accepts what they were SHOWN. If any of it moved since (the
            // text saved from another window, the client's next model), nothing is
            // accepted and they are shown what is there now.
            let shown = [
                ("source", &expect.source, Some(&now.basis.source)),
                ("model", &expect.model, Some(&now.basis.model)),
                (
                    "requirements",
                    &expect.requirements,
                    Some(&now.basis.requirements),
                ),
            ];
            let mut moved: Vec<&str> = shown
                .iter()
                .filter(|(_, seen, current)| Some(*seen) != *current)
                .map(|(name, _, _)| *name)
                .collect();
            if expect.answers != now.basis.answers {
                moved.push("answers");
            }
            if !moved.is_empty() {
                return Err(CommandError {
                    kind: "moved".to_string(),
                    message: format!(
                        "{} changed after you were shown it, so nothing was accepted; read it again",
                        moved.join(", ")
                    ),
                    detail: json!({ "moved": moved, "current": now.basis }),
                });
            }

            // A design written for an earlier text is not an answer to this one.
            let behind: Vec<&str> = [
                ("model", now.model_written_for.as_ref()),
                ("requirements", now.requirements_written_for.as_ref()),
            ]
            .iter()
            .filter(|(_, written_for)| *written_for != source_head.as_ref())
            .map(|(name, _)| *name)
            .collect();
            if !behind.is_empty() {
                return Err(CommandError {
                    kind: "not-current".to_string(),
                    message: format!(
                        "the {} was not written for the text as it is now, so it cannot be accepted as \
                         an answer to it",
                        behind.join(" and the ")
                    ),
                    detail: json!({ "behind": behind, "source_head": source_head }),
                });
            }

            let taken = renderer.take_acceptance(&now.snapshot)?;
            let acceptance = Acceptance::new(store.now(), now.basis, taken.record, taken.open);
            let base = store.read_acceptance(&id, None)?.map(|a| a.revision);
            answer(&store.save_acceptance(&id, &acceptance.stored_text(), base.as_ref())?)
        }
        "read_acceptance" => {
            let OneSpec { id } = arguments(args)?;
            let id = spec_id(&id)?;
            let Some(saved) = store.read_acceptance(&id, None)? else {
                return Ok(json!({
                    "acceptance": null,
                    "standing": "none",
                    "lapse": null,
                }));
            };
            let acceptance = Acceptance::parse(&saved.text)?;
            // Whether it still holds is the product's to say, about the spec as it is
            // now laid out the way it was when the record was taken.
            let now = spec_now(store, &id)?;
            let (standing, lapse) =
                standing_of(renderer.check_acceptance(&now.snapshot, &acceptance.record)?);
            Ok(json!({
                "acceptance": acceptance_json(&saved.revision, &acceptance),
                "standing": standing,
                "lapse": lapse,
                "now": now.basis,
            }))
        }
        "read_acceptance_delta" => {
            let OneSpec { id } = arguments(args)?;
            let id = spec_id(&id)?;
            // ONE state of the spec: the acceptance and the design and list it is compared with
            // are read together, so a save landing between them cannot make the comparison one
            // of an acceptance and a design that were never both the spec's.
            let state = store.read_spec_snapshot(&id)?;
            let Some(saved) = state.acceptance else {
                return Ok(json!({
                    "acceptance": null,
                    "manifest": null,
                    "lines": null,
                    "now": null,
                }));
            };
            let acceptance = Acceptance::parse(&saved.text)?;
            let source = state.source.ok_or_else(|| none_saved("a text", &id))?;
            let model = state.model.ok_or_else(|| none_saved("a model", &id))?;
            let requirements = state
                .requirements
                .ok_or_else(|| none_saved("a requirement list", &id))?;
            let now = now_of(source, model, requirements, state.answers)?;
            refuse_what_is_behind_the_text(&now)?;
            let lines = renderer.delta_acceptance(&now.snapshot, &acceptance.record)?;
            // The manifest the record pinned (its document, revision and digest), so a caller
            // that holds a words-side comparison can say it is about THIS list and not another
            // of the same name and number. The product's record is not otherwise passed on.
            let manifest = serde_json::from_str::<Value>(&acceptance.record)
                .ok()
                .and_then(|record| record.get("manifest").cloned())
                .unwrap_or(Value::Null);
            Ok(json!({
                "acceptance": acceptance_json(&saved.revision, &acceptance),
                "manifest": manifest,
                "lines": lines,
                "now": now.basis,
            }))
        }
        "read_revision_report" => {
            let RevisionReport { id, sentences } = arguments(args)?;
            let id = spec_id(&id)?;
            // ONE state of the spec, as `read_acceptance_delta` reads it: the acceptance, and the
            // design and list it is compared with.
            let state = store.read_spec_snapshot(&id)?;
            let Some(saved) = state.acceptance else {
                return Ok(json!({ "acceptance": null, "report": null }));
            };
            let acceptance = Acceptance::parse(&saved.text)?;
            let source = state.source.ok_or_else(|| none_saved("a text", &id))?;
            let model = state.model.ok_or_else(|| none_saved("a model", &id))?;
            let requirements = state
                .requirements
                .ok_or_else(|| none_saved("a requirement list", &id))?;
            let now = now_of(source, model, requirements, state.answers)?;
            refuse_what_is_behind_the_text(&now)?;
            let lines = renderer.delta_acceptance(&now.snapshot, &acceptance.record)?;
            // The list the owner accepted is the spec's own, read at the revision the
            // acceptance names: the words side is derived from two states of one chain.
            let accepted = store
                .read_requirements(&id, Some(&acceptance.basis.requirements))?
                .ok_or_else(|| none_saved("the requirement list that was accepted", &id))?;
            let accepted = Requirements::parse(&accepted.text)?;
            let pin = serde_json::from_str::<Value>(&acceptance.record)
                .ok()
                .and_then(|record| record.get("manifest").cloned())
                .unwrap_or(Value::Null);
            let title = format!(
                "spec {id}, list {} to {}",
                acceptance.basis.requirements.short(),
                now.basis.requirements.short()
            );
            let report = crate::revision_report::judge(
                &accepted,
                &now.snapshot.requirements,
                &pin,
                &json!(lines),
                json!({ "accepted": acceptance.basis, "now": now.basis }),
                sentences,
                &title,
            )
            .map_err(|not_judged| {
                CommandError::from(StoreError::refused(
                    "revision-not-judged",
                    not_judged.0,
                    json!({}),
                ))
            })?;
            Ok(json!({
                "acceptance": acceptance_json(&saved.revision, &acceptance),
                "report": report,
            }))
        }
        "read_spec_snapshot" => {
            let OneSpec { id } = arguments(args)?;
            let snapshot = store.read_spec_snapshot(&spec_id(&id)?)?;
            // The parts are the answers of the commands that read one chain, in their
            // words, so the screen holds one definition of each; what differs is that
            // they were read as one state of the spec and not as six.
            let source_head = snapshot.source.as_ref().map(|s| &s.revision);
            let (model, model_standing) = match &snapshot.model {
                None => (None, None),
                Some(model) => {
                    let files = ModelFiles::parse(&model.text)?;
                    (
                        Some(model_json(model, &files)),
                        Some(standing(model.written_for.as_ref(), source_head)),
                    )
                }
            };
            let (requirements, requirements_standing) = match &snapshot.requirements {
                None => (None, None),
                Some(saved) => (
                    Some(requirements_json(saved)?),
                    Some(standing(saved.written_for.as_ref(), source_head)),
                ),
            };
            let answers = snapshot.answers.as_ref().map(answers_json).transpose()?;
            let acceptance = match &snapshot.acceptance {
                None => None,
                Some(saved) => Some(acceptance_json(
                    &saved.revision,
                    &Acceptance::parse(&saved.text)?,
                )),
            };
            Ok(json!({
                "spec": snapshot.spec,
                "source": snapshot.source,
                "model": model,
                "model_standing": model_standing,
                "answers": answers,
                "requirements": requirements,
                "requirements_standing": requirements_standing,
                "acceptance": acceptance,
                "bundle": snapshot.bundle,
            }))
        }
        "read_spec_heads" => {
            let OneSpec { id } = arguments(args)?;
            answer(&store.read_spec_heads(&spec_id(&id)?)?)
        }
        "request_generation" => {
            let RequestGeneration {
                id,
                key,
                origin,
                expect,
                supersede,
                connection,
                fresh_ids,
            } = arguments(args)?;
            let id = spec_id(&id)?;
            // A press sent again is the request it made, whatever has become of the connection
            // since: the connection is not an input of the key, so it is not looked at when the
            // key is known.
            let repeated = store
                .list_requests(&id)?
                .iter()
                .any(|view| view.request.key == key);
            let pin = match (&connection, repeated) {
                (Some(wanted), false) => Some(pins.pin_for(wanted)?),
                _ => None,
            };
            let made = store.register_request_with(
                &id,
                Registration {
                    key: &key,
                    origin: &origin,
                    expect: Inputs {
                        source: expect.source,
                        answers: expect.answers,
                    },
                    supersede,
                },
                pin,
                fresh_ids,
            )?;
            let view = RequestView {
                request: made.request,
                state: made.state,
            };
            Ok(json!({ "request": request_json(&id, &view), "created": made.created }))
        }
        "read_request" => {
            let OneRequest { id, request } = arguments(args)?;
            let id = spec_id(&id)?;
            let view = store.read_request(&id, &request)?;
            Ok(json!({ "request": request_json(&id, &view) }))
        }
        "list_requests" => {
            let OneSpec { id } = arguments(args)?;
            let id = spec_id(&id)?;
            let views = store.list_requests(&id)?;
            let requests: Vec<Value> = views.iter().map(|v| request_json(&id, v)).collect();
            Ok(json!({ "requests": requests }))
        }
        "list_open_requests" => {
            arguments::<Empty>(args)?;
            let requests: Vec<Value> = store
                .open_requests()?
                .iter()
                .map(|(spec, view)| request_json(spec, view))
                .collect();
            Ok(json!({ "requests": requests }))
        }
        "claim_request" => {
            let ClaimRequest {
                id,
                request,
                holder,
                ttl_seconds,
                resume,
                connection,
            } = arguments(args)?;
            let id = spec_id(&id)?;
            let view = store.claim_request_for(
                &id,
                &request,
                &holder,
                ttl_seconds,
                resume,
                connection.as_ref(),
            )?;
            Ok(json!({ "request": request_json(&id, &view) }))
        }
        "heartbeat_request" => {
            let HeartbeatRequest {
                id,
                request,
                holder,
                attempt,
                ttl_seconds,
            } = arguments(args)?;
            let id = spec_id(&id)?;
            let view = store.heartbeat_request(&id, &request, &holder, attempt, ttl_seconds)?;
            Ok(json!({ "request": request_json(&id, &view) }))
        }
        "complete_request" => {
            let FinishRequest {
                id,
                request,
                holder,
                attempt,
                checks,
                lexicon,
            } = arguments(args)?;
            let id = spec_id(&id)?;
            // What the caller reports is read first: a report that is not one costs nothing
            // to refuse, and the product is not asked about a call that will be refused.
            let reported = checks
                .into_iter()
                .map(client_check_of)
                .collect::<Result<Vec<_>, _>>()?;
            let published = complete_generation(
                store,
                renderer,
                &id,
                &request,
                &holder,
                attempt,
                reported,
                lexicon.as_deref(),
            )?;
            Ok(json!({
                "request": request_json(&id, &published.request),
                "bundle": published.bundle,
            }))
        }
        "save_request_candidate" => {
            let SaveCandidate {
                id,
                request,
                holder,
                attempt,
                text,
                documents,
                entry,
                manifest,
                sidecar,
                lineage,
                instructions,
            } = arguments(args)?;
            let model = match (&text, &documents, &entry) {
                (None, None, None) => None,
                _ => Some(model_files_of(text, documents, entry)?.stored_text()),
            };
            let requirements =
                match (manifest, sidecar, lineage) {
                    (None, None, None) => None,
                    (Some(manifest), sidecar, lineage) => Some(
                        Requirements::new(manifest, sidecar)?
                            .with_lineage(lineage)?
                            .stored_text(),
                    ),
                    (None, _, _) => return Err(CommandError::bad_request(
                        "a `sidecar` and a `lineage` belong to a `manifest`: give them with one, \
                         or none of them",
                    )),
                };
            let id = spec_id(&id)?;
            let view = store.save_candidate(
                &id,
                &request,
                &holder,
                attempt,
                CandidateWrite {
                    model,
                    requirements,
                    instructions,
                },
            )?;
            Ok(json!({ "request": request_json(&id, &view) }))
        }
        "read_request_candidate" => {
            let OneRequest { id, request } = arguments(args)?;
            let id = spec_id(&id)?;
            let texts = store.read_candidate(&id, &request)?;
            Ok(json!({
                "request": request,
                "model": texts.model.map(|(revision, text)| candidate_model_json(&revision, &text)),
                "requirements": texts
                    .requirements
                    .map(|(revision, text)| json!({ "revision": revision, "text": text })),
            }))
        }
        "read_bundle" => {
            let OneBundle { id, revision } = arguments(args)?;
            let id = spec_id(&id)?;
            let read = store.read_bundle(&id, revision.as_ref())?;
            Ok(json!({
                "bundle": read.map(|r| json!({ "revision": r.revision, "bundle": r.bundle })),
            }))
        }
        "bundle_history" => {
            let OneSpec { id } = arguments(args)?;
            Ok(json!({ "entries": store.bundle_history(&spec_id(&id)?)? }))
        }
        "add_original" => {
            let AddOriginal {
                id,
                label,
                members,
                missing,
            } = arguments(args)?;
            let mut drafts = Vec::with_capacity(members.len());
            for member in members {
                let bytes = crate::base64::decode(&member.data).map_err(|why| {
                    CommandError::bad_request(format!(
                        "the data of the member `{}` is not base64: {why}",
                        member.path
                    ))
                })?;
                drafts.push(MemberDraft {
                    path: member.path,
                    role: member.role,
                    bytes,
                    of: member.of,
                    renderer: member.renderer,
                });
            }
            answer(&store.add_original(
                &spec_id(&id)?,
                OriginalDraft {
                    label: label.unwrap_or_default(),
                    members: drafts,
                    missing,
                },
            )?)
        }
        "read_original" => {
            let OneOriginal { id, original } = arguments(args)?;
            let manifest = store.read_original(&spec_id(&id)?, &original)?;
            Ok(json!({
                "original": original,
                "label": manifest.label,
                "members": manifest.members,
                "missing": manifest.missing,
                "bytes": manifest.bytes(),
            }))
        }
        "list_originals" => {
            let OneSpec { id } = arguments(args)?;
            Ok(json!({ "originals": store.list_originals(&spec_id(&id)?)? }))
        }
        "read_original_member" => {
            let OriginalMember { id, original, path } = arguments(args)?;
            let bytes = store.read_original_member(&spec_id(&id)?, &original, &path)?;
            Ok(json!({
                "path": path,
                "size": bytes.len(),
                "data": crate::base64::encode(&bytes),
            }))
        }
        "save_extraction" => {
            let SaveExtraction {
                id,
                original,
                member,
                text,
            } = arguments(args)?;
            answer(&store.save_extraction(&spec_id(&id)?, &original, &member, &text)?)
        }
        "read_extraction" => {
            let OneExtraction { id, digest } = arguments(args)?;
            let text = store.read_extraction(&spec_id(&id)?, &digest)?;
            Ok(json!({ "digest": digest, "text": text }))
        }
        "list_extractions" => {
            let ListExtractions { id, original } = arguments(args)?;
            Ok(json!({
                "extractions": store.list_extractions(&spec_id(&id)?, original.as_ref())?,
            }))
        }
        "fail_request" => {
            let FailRequest {
                id,
                request,
                holder,
                attempt,
                reason,
                failure,
            } = arguments(args)?;
            let id = spec_id(&id)?;
            let failure = failure.map_or_else(Failure::unclassified, Failure::from);
            let view = store.fail_request(&id, &request, &holder, attempt, &reason, failure)?;
            Ok(json!({ "request": request_json(&id, &view) }))
        }
        "record_request_event" => {
            let RecordEvent {
                id,
                request,
                holder,
                attempt,
                event,
            } = arguments(args)?;
            let id = spec_id(&id)?;
            let entries = store.record_event(&id, &request, &holder, attempt, event)?;
            Ok(json!({ "entries": entries }))
        }
        "read_request_journal" => {
            let OneRequest { id, request } = arguments(args)?;
            let id = spec_id(&id)?;
            let journal = store.read_journal(&id, &request)?;
            Ok(journal_json(&journal, store.now()))
        }
        "cancel_request" => {
            let OneRequest { id, request } = arguments(args)?;
            let id = spec_id(&id)?;
            let view = store.cancel_request(&id, &request)?;
            Ok(json!({ "request": request_json(&id, &view) }))
        }
        "report_adapter" => {
            let ReportAdapter {
                name,
                kind,
                capabilities,
                version,
            } = arguments(args)?;
            let status = store.report_adapter(AdapterReport {
                name: &name,
                kind: &kind,
                capabilities,
                version: version.as_deref(),
            })?;
            Ok(json!({ "adapter": adapter_json(&status) }))
        }
        "read_adapter_status" => {
            arguments::<Empty>(args)?;
            let listing = store.adapter_status()?;
            let adapters: Vec<Value> = listing.adapters.iter().map(adapter_json).collect();
            Ok(json!({ "adapters": adapters, "unreadable": listing.unreadable }))
        }
        "read_host_status" => {
            arguments::<Empty>(args)?;
            let listing = store.host_status()?;
            let hosts: Vec<Value> = listing.hosts.iter().map(host_json).collect();
            Ok(json!({ "hosts": hosts, "unreadable": listing.unreadable }))
        }
        "read_queue" => {
            arguments::<Empty>(args)?;
            let waiting: Vec<Value> = store
                .lane_waits()?
                .iter()
                .map(|wait| {
                    json!({
                        "spec": wait.spec.as_str(),
                        "request": wait.request,
                        "connection": wait.connection,
                        "running": wait.running,
                        "places": wait.places,
                        "ahead": wait.ahead,
                        "running_since": wait.running_since.map(utc_timestamp),
                    })
                })
                .collect();
            // `now` is the store's moment, as the journal's is, so that a screen measures how long
            // a place has been held by the clock that wrote when it was taken.
            Ok(json!({ "waiting": waiting, "now": store.now() }))
        }
        "remove_spec" => {
            let OneSpec { id } = arguments(args)?;
            Ok(json!({ "removed": store.remove_spec(&spec_id(&id)?)? }))
        }
        other => Err(unknown_command(other, COMMANDS)),
    }
}

/// The refusal of a command that is not one of `commands`, naming them all. Public so that an
/// application that answers commands of its own in front of these says so of all of them.
pub fn unknown_command(name: &str, commands: &[&str]) -> CommandError {
    CommandError {
        kind: "unknown-command".to_string(),
        message: format!(
            "`{name}` is not a command; the commands are {}",
            commands.join(", ")
        ),
        detail: Value::Null,
    }
}
