// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The commands, as JSON in and JSON out.
//!
//! The desktop shell, the browser shell and the `sce-work` command all reach the
//! store through [`call`]. That is the point of having it: three entrances that
//! each decode their own arguments and encode their own errors would be three
//! chances to disagree about what a save is, and the screen is the one that
//! cannot be reasoned about from the command line.
//!
//! A command takes one JSON object and answers one JSON object. Unknown fields
//! are refused, not ignored: an argument that nothing reads is a caller that
//! thinks it said something.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::acceptance::{Acceptance, AcceptanceCorrupt, Basis, Snapshot};
use crate::acceptance_run::{CheckOutcome, RequirementsReport};
use crate::answers::{Answers, AnswersError};
use crate::auth_policy::{Policy, Route};
use crate::bundle::{BundleCheck, CheckedBy};
use crate::claude_code::{candidates, Search};
use crate::claude_status;
use crate::clock::{utc_timestamp, Clock};
use crate::codex;
use crate::codex_status;
use crate::codex_support::Support;
use crate::connection::{AdapterKind, Connection, ConnectionId};
use crate::error::StoreError;
use crate::figures::{FigureRequest, RenderError};
use crate::model_set::{Document, ModelError, ModelFiles};
use crate::requests::{ConnectionRef, Inputs, Lease, Pin};
use crate::requirements::{Requirements, RequirementsError};
use crate::review::{Product, Review, ReviewRequest, Verdict};
use crate::revision::Revision;
use crate::server_status;
use crate::store::{
    AdapterReport, AdapterStatus, AnswersText, CandidateWrite, ConnectionStore, HostStatus,
    ModelText, Published, Registration, RequestView, RequirementsText, SourceText, WorkId,
    WorkStore,
};

/// Every command, in the order a person would meet them.
pub const COMMANDS: &[&str] = &[
    "describe",
    "list_works",
    "create_work",
    "read_work",
    "read_source",
    "save_source",
    "history",
    "save_model",
    "read_model",
    "model_history",
    "figures",
    "remove_work",
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
    "read_work_snapshot",
    "read_work_heads",
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
    "read_bundle",
    "bundle_history",
    "list_connections",
    "read_connection",
    "read_auth_policy",
    "read_claude_status",
    "read_codex_status",
    "find_clients",
    "read_server_status",
    "save_connection",
    "delete_connection",
    "set_default_connection",
];

/// The commands that read the settings a person keeps apart from the works.
const SETTINGS_READ: &[&str] = &["list_connections", "read_connection", "read_auth_policy"];

/// The commands that start a program of the person's, or reach a server they named, to ask it
/// something. A command that does is a way to make the application run it or call it, for whoever
/// can send the command (an address is anywhere the machine can reach), so only the entrance that
/// is the person at the keyboard may: the same reason the settings are theirs.
const STARTS_PROGRAMS: &[&str] = &[
    "read_claude_status",
    "read_codex_status",
    "find_clients",
    "read_server_status",
];

/// The commands that change them. A connection names a program the application runs, so a
/// command that writes one is a way to make the application run something: only the entrance
/// that is the person at the keyboard may.
const SETTINGS_WRITE: &[&str] = &[
    "save_connection",
    "delete_connection",
    "set_default_connection",
];

/// The version of this command set. It moves when a command's arguments or
/// answer change in a way a caller written against the last one would misread.
///
/// 2: a work also keeps a model (`save_model`, `read_model`, `model_history`) and
/// SCE draws it (`figures`). A screen written for 1 has no use for them, and a
/// screen written for 2 cannot run on a core of 1, so the two are told apart.
///
/// 3: a work can be removed (`remove_work`). A screen written for 3 offers it and
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
/// 8: a work can be read as one state (`read_work_snapshot`): the text, the model, the
/// answers, the requirement list and the acceptance as they stood together, for a screen
/// that is told something changed and must not show a text of one moment beside a model
/// of another. A screen written for 7 has no use for it, and a screen written for 8
/// cannot run on a core of 7.
///
/// 9: where each chain of a work stands can be read without reading the work
/// (`read_work_heads`), so that a screen can ask often whether the work moved under it
/// (a save from another window, an authoring client's next model) and read it only
/// when it did. A screen written for 9 asks for it, and a core of 8 would refuse with
/// `unknown-command`.
///
/// 10: a work can be asked for a model (`request_generation`), an executor takes the
/// request, keeps it and finishes it (`claim_request`, `heartbeat_request`,
/// `complete_request`, `fail_request`), the owner calls it off (`cancel_request`), and
/// requests are read (`read_request`, `list_requests`). An AI adapter says it is there
/// (`report_adapter`) and a screen asks which are (`read_adapter_status`). The heads of a
/// work also say where its latest request stands. A screen written for 10 offers what a
/// request makes possible, and would be refused by a core of 9 with `unknown-command`.
///
/// 11: what a request makes becomes the work's in one step. The executor writes a candidate
/// (`save_request_candidate`, read back by `read_request_candidate`) that is not yet the
/// work's model, and `complete_request` publishes it: the core runs its own check of the
/// model, and the model and the requirement list become the work's together as one bundle
/// (`read_bundle`, `bundle_history`). An executor finds the requests it may take across the
/// works (`list_open_requests`), and a screen asks whether a shell hosts one and, if not, why
/// (`read_host_status`). A request says its `candidate` and, once done, its
/// `outcome`; the heads and the snapshot of a work say their `bundle`. A work that keeps
/// bundles refuses `save_model` and `save_requirements` with `bundled-work`. A screen
/// written for 11 reads a work's model by the bundle and would be refused by a core of 10.
///
/// 12: what SCE says of a work can be asked of the revisions that were read
/// (`read_judgment`): whether the owner's acceptance holds for them, and SCE's measure of
/// the design against the list. The answer is of exactly the revisions named, whatever has
/// been saved since, so a screen shows no verdict beside a design it was not given of. A
/// screen written for 12 asks it after reading the work, and would be refused by a core of
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
/// 14: the ways a person reaches a model are settings the core keeps apart from the works
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
/// across the revisions of a work's text. A list saved without one onto a work that holds one is
/// refused as `lineage-dropped`, and so is a candidate published that way. A screen written for
/// 18 would not know the argument or the refusal, and a core of 18 would refuse the argument as
/// one it does not know.
///
/// 20: what moved in a work's design since the owner accepted it can be asked of the product
/// requirement by requirement (`read_acceptance_delta`): the product's own comparison of the rows
/// the owner was shown with the work's design as it is now, read with the acceptance as one state
/// of the work, and the manifest the acceptance pinned. A report and not a verdict: the acceptance
/// still lapses by its bytes (`read_acceptance`). A screen written for 19 does not ask for it, and
/// a core of 19 would refuse it as `unknown-command`.
///
/// 21: a requirement list's lineage is judged when the list is saved or published, and not only
/// for being lost: one that is not a lineage (`lineage-unusable`), that is not the lineage of the
/// list beside it (`lineage-of-another-list`: it names the manifest and the sidecar by their
/// digests, so a list with its sidecar left out is refused too) or that does not continue the
/// work's (`lineage-not-continued`: an id taken back, a retired id lived again, a history
/// rewritten, an id numbered twice) is refused, so that an id means one requirement for ever
/// whatever built the list. A screen written for 20 does not know these refusals, and a core of
/// 20 would save such a list.
///
/// 22: what a revision of a work did can be asked of the product requirement by requirement
/// (`read_revision_report`): the words of each requirement from the work's own lineages beside
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
/// (`lineage-dropped`, `lineage-not-continued`), so that the first lineage of a work is judged like
/// every other. A client of 23 was handed no lineage for such a list and built its next list as a
/// first one, which a core of 24 refuses; a core of 23 would keep it.
pub const COMMAND_SET_VERSION: u32 = 24;

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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateWork {
    title: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OneWork {
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
struct SaveSource {
    id: String,
    text: String,
    /// Absent or `null` means "this is the work's first text".
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
    /// Absent or `null` means "this is the work's first model".
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
    /// Absent or `null` means "this is the work's first set of answers".
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
    /// it (a work that holds one refuses a list without: `lineage-dropped`).
    #[serde(default)]
    lineage: Option<String>,
    /// Absent or `null` means "this is the work's first requirement list".
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
    /// Replace the open request of the work, if there is one.
    #[serde(default)]
    supersede: bool,
    /// The connection it is made for, at the revision the screen read. The core copies what a
    /// run needs of it from the settings; the screen does not say what that is.
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
    /// The bundle; the work's current one when absent.
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
fn request_json(work: &WorkId, view: &RequestView) -> Value {
    let request = &view.request;
    json!({
        "id": request.id,
        "work": work.as_str(),
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

fn arguments<T: for<'de> Deserialize<'de>>(args: Value) -> Result<T, CommandError> {
    // A caller that sends nothing means an empty object.
    let args = if args.is_null() { json!({}) } else { args };
    serde_json::from_value(args).map_err(|e| CommandError::bad_request(e.to_string()))
}

fn answer<T: Serialize>(value: &T) -> Result<Value, CommandError> {
    serde_json::to_value(value).map_err(|e| CommandError::bad_request(e.to_string()))
}

fn work_id(text: &str) -> Result<WorkId, CommandError> {
    WorkId::parse(text).map_err(CommandError::from)
}

fn none_saved(what: &str, id: &WorkId) -> CommandError {
    CommandError::from(StoreError::NotFound {
        what: format!("{what} of work `{}` (none was saved)", id.as_str()),
    })
}

/// What a work's four chains hold now, as the product is asked about them: the
/// text, the model, the requirement list and the owner's answers, each at the
/// revision it was read at (`basis`), and the source claims the model's and the
/// list's saves made.
struct WorkNow {
    snapshot: Snapshot,
    basis: Basis,
    model_written_for: Option<Revision>,
    requirements_written_for: Option<Revision>,
}

/// Read the work as it is now. A chain with nothing in it is `not-found`, naming
/// which: the requirement list and the model are not things the application makes up.
fn work_now<C: Clock>(store: &WorkStore<C>, id: &WorkId) -> Result<WorkNow, CommandError> {
    // One state of the work, not four reads a save or a publication can land between: the
    // model and the list of a bundle are one generation, and a design is accepted against
    // them together.
    let state = store.read_work_snapshot(id)?;
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
/// the design. Where it stands is a fact of one read of the work (`read_work_snapshot`, which
/// says it beside the revisions), and a judgment is a function of the revisions alone.
struct Revisions {
    snapshot: Snapshot,
    basis: Basis,
}

/// The content of the revisions `basis` names, wherever the work is now. A revision's file is
/// never rewritten, so what is read is what was read when the basis was taken, whatever has
/// been saved since; one the work does not keep is `not-found`.
fn work_at<C: Clock>(
    store: &WorkStore<C>,
    id: &WorkId,
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

/// What the product is asked about, from the four parts of a work and the revisions they are.
fn now_of(
    source: SourceText,
    model: ModelText,
    requirements: RequirementsText,
    answers: Option<AnswersText>,
) -> Result<WorkNow, CommandError> {
    let basis = Basis {
        source: source.revision.clone(),
        model: model.revision.clone(),
        requirements: requirements.revision.clone(),
        answers: answers.as_ref().map(|a| a.revision.clone()),
    };
    Ok(WorkNow {
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
/// (`read_acceptance_delta`) are about the model and the list written for the text the work has
/// now. When the text was changed and one of them was not written again, the list the work
/// holds is the list the owner accepted (or an earlier one) and comparing the two would call it
/// "all carried over" or "unchanged" for a revision nobody made. Only a part known to be written for
/// an earlier text is held back: one that cannot say what it was written for is judged as it
/// always was (an acceptance is never taken of one, so it is one saved after).
fn refuse_what_is_behind_the_text(now: &WorkNow) -> Result<(), CommandError> {
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

/// What the product measured of the work as `now` puts it, with the revisions it was measured
/// against and where the model and the list stand to the text they were measured beside. The
/// source head is the text `now` holds, and not a later read of it.
fn report_json(now: &WorkNow, report: &RequirementsReport) -> Value {
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
/// answering, because the caller still has the work it read when the product does not answer: a
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

/// What SCE says of `files`, the model of work `id`: its check and its pseudocode page.
fn review_model(
    renderer: &dyn Product,
    id: &WorkId,
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
/// when the check accepts, the model and the requirement list become the work's together.
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
    store: &WorkStore<C>,
    renderer: &dyn Product,
    id: &WorkId,
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
    store: &WorkStore<C>,
    id: &WorkId,
    revision: Option<&Revision>,
) -> Result<(ModelText, ModelFiles), CommandError> {
    let Some(model) = store.read_model(id, revision)? else {
        return Err(CommandError::from(StoreError::NotFound {
            what: format!("a model of work `{}` (none was saved)", id.as_str()),
        }));
    };
    let files = ModelFiles::parse(&model.text)?;
    Ok((model, files))
}

/// Which door a command came in by. Who is asking decides what may be changed, so it is said
/// where the command is run and not left to each shell to check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Entrance {
    /// The application's window: the person at the keyboard.
    Desktop,
    /// The development shell a token reaches over a network.
    Browser,
    /// A program that runs commands (`sce-work`, which the authoring MCP uses). An AI is on the
    /// other end of it, and an AI must not be able to change which AI it runs on.
    Tool,
}

impl Entrance {
    /// Whether this entrance may change the person's settings.
    pub fn writes_settings(self) -> bool {
        self == Entrance::Desktop
    }

    /// Whether this entrance may start a program of the person's (to ask it who is signed in).
    pub fn starts_programs(self) -> bool {
        self == Entrance::Desktop
    }
}

/// Everything a command may need, and who is asking.
///
/// Made with [`Context::new`] and what the entrance has added to it, so that a new thing a
/// command needs is one more `with_` and not an edit to every place a context is made.
pub struct Context<'a, C: Clock> {
    pub works: &'a WorkStore<C>,
    /// The product: it draws a model for `figures` and reads one for `review`.
    pub product: &'a dyn Product,
    /// The settings folder, when the entrance has one.
    pub connections: Option<&'a ConnectionStore>,
    /// Which ways of signing in this build uses.
    pub policy: &'a Policy,
    pub entrance: Entrance,
    /// The program the shell was told is Claude Code (`SCE_CLAUDE`), when it was told one; none
    /// is the first one the application finds.
    pub claude: Option<&'a Path>,
    /// The program the shell was told is Codex (`SCE_CODEX`), when it was told one.
    pub codex: Option<&'a Path>,
    /// Where to look for the programs a connection may name; this process's own places when
    /// absent.
    pub search: Option<&'a Search>,
}

impl<'a, C: Clock> Context<'a, C> {
    /// A context with a works folder and nothing the person keeps of their own.
    pub fn new(
        works: &'a WorkStore<C>,
        product: &'a dyn Product,
        policy: &'a Policy,
        entrance: Entrance,
    ) -> Self {
        Context {
            works,
            product,
            connections: None,
            policy,
            entrance,
            claude: None,
            codex: None,
            search: None,
        }
    }

    /// With the person's settings folder.
    pub fn with_connections(mut self, connections: Option<&'a ConnectionStore>) -> Self {
        self.connections = connections;
        self
    }

    /// With the program the shell was told is Claude Code.
    pub fn with_claude(mut self, claude: Option<&'a Path>) -> Self {
        self.claude = claude;
        self
    }

    /// With the program the shell was told is Codex.
    pub fn with_codex(mut self, codex: Option<&'a Path>) -> Self {
        self.codex = codex;
        self
    }

    /// With the places to look for the programs a connection may name.
    pub fn with_search(mut self, search: Option<&'a Search>) -> Self {
        self.search = search;
        self
    }

    /// Where to look for the programs a connection may name.
    fn places(&self) -> Search {
        self.search
            .cloned()
            .unwrap_or_else(Search::from_environment)
    }
}

/// Run the command `name` with `args` against `store`; `renderer` is the
/// product: it draws a model for `figures` and reads one for `review`.
///
/// This is the entrance of a program that has a works folder and no settings: the settings
/// commands answer `no-settings`, and `describe` says it is a tool. A shell that has a person's
/// settings folder uses [`call_in`].
pub fn call<C: Clock>(
    store: &WorkStore<C>,
    renderer: &dyn Product,
    name: &str,
    args: Value,
) -> Result<Value, CommandError> {
    let policy = Policy::shipped();
    call_in(
        &Context::new(store, renderer, &policy, Entrance::Tool),
        name,
        args,
    )
}

/// Run the command `name` with `args` in `context`.
pub fn call_in<C: Clock>(
    context: &Context<'_, C>,
    name: &str,
    args: Value,
) -> Result<Value, CommandError> {
    if name == "describe" {
        let mut described = call_works(context.works, context.product, name, args)?;
        if let Some(fields) = described.as_object_mut() {
            fields.insert("entrance".to_string(), json!(context.entrance));
            fields.insert("settings".to_string(), json!(context.connections.is_some()));
            fields.insert(
                "writes_settings".to_string(),
                json!(context.connections.is_some() && context.entrance.writes_settings()),
            );
            fields.insert(
                "starts_programs".to_string(),
                json!(context.entrance.starts_programs()),
            );
        }
        return Ok(described);
    }
    if STARTS_PROGRAMS.contains(&name) {
        return call_program(context, name, args);
    }
    if SETTINGS_READ.contains(&name) || SETTINGS_WRITE.contains(&name) {
        return call_settings(context, name, args);
    }
    if name == "request_generation" {
        return request_generation(context, args);
    }
    call_works(context.works, context.product, name, args)
}

/// Ask for a model, made for the connection the screen names, when it names one.
///
/// This is a command of the works that has to read the settings: the core copies what a run
/// needs of the connection (see [`Pin`]) and the screen does not say what that is, so a caller
/// cannot make a request that carries more than the person's own settings hold.
fn request_generation<C: Clock>(
    context: &Context<'_, C>,
    args: Value,
) -> Result<Value, CommandError> {
    let RequestGeneration {
        id,
        key,
        origin,
        expect,
        supersede,
        connection,
        fresh_ids,
    } = arguments(args)?;
    let id = work_id(&id)?;
    // A press sent again is the request it made, whatever has become of the connection since:
    // the connection is not an input of the key, so it is not looked at when the key is known.
    let repeated = context
        .works
        .list_requests(&id)?
        .iter()
        .any(|view| view.request.key == key);
    let pin = match (&connection, repeated) {
        (Some(wanted), false) => Some(pin_for(context.connections, wanted)?),
        _ => None,
    };
    let made = context.works.register_request_with(
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

/// What the core copies of the connection a request is made for, from the settings and not from
/// the caller. Refused as `moved` when the connection is no longer at the revision the caller
/// read: a person who pressed the button on one state of it is not answered under another.
fn pin_for(
    connections: Option<&ConnectionStore>,
    wanted: &ConnectionRef,
) -> Result<Pin, CommandError> {
    let Some(connections) = connections else {
        return Err(CommandError {
            kind: "no-settings".to_string(),
            message:
                "a request for a connection needs a settings folder, and this entrance has none"
                    .to_string(),
            detail: Value::Null,
        });
    };
    let Some(current) = connections.read(&wanted.id, None)? else {
        return Err(CommandError::from(StoreError::NotFound {
            what: format!("connection `{}`", wanted.id),
        }));
    };
    if current.revision != wanted.revision {
        return Err(CommandError::from(StoreError::refused(
            "moved",
            "the connection was changed after it was read, so nothing was registered; read it again",
            json!({
                "moved": ["connection"],
                "current": { "connection": { "id": wanted.id, "revision": current.revision } },
            }),
        )));
    }
    let connection = current.connection;
    Ok(Pin {
        connection: connection.id,
        revision: current.revision,
        adapter: connection.adapter,
        model: connection.model,
        limits: connection.limits,
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadConnection {
    id: String,
    #[serde(default)]
    revision: Option<Revision>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadClientStatus {
    /// The connection the screen is about, kept or not yet: the program it names is the one asked.
    /// None asks about the default connection, as a screen written for 16 does.
    #[serde(default)]
    connection: Option<String>,
    /// The program the screen is about to keep for it, which is not kept yet: that program is the
    /// one asked, whatever the connection names. `null` is the application's own choice, and
    /// absent leaves it to the connection. A person who chose another program has not been told
    /// who is signed in to it until this is asked of it.
    #[serde(default, deserialize_with = "present")]
    executable: Option<Option<String>>,
}

/// A field that is there, `null` or not, apart from one that is not there.
fn present<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadServerStatus {
    server_url: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveConnection {
    connection: Connection,
    #[serde(default)]
    base: Option<Revision>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeleteConnection {
    id: String,
    base: Revision,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SetDefaultConnection {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    expect: Option<String>,
}

fn connection_id(text: &str) -> Result<ConnectionId, CommandError> {
    ConnectionId::parse(text).map_err(CommandError::from)
}

/// Whether `executable` is a program the application finds now for the kind of connection that
/// names it. Found again here and not remembered from when the screen asked: a file that was
/// replaced in between, or one that never was a candidate, is not one to run.
fn check_executable<C: Clock>(
    context: &Context<'_, C>,
    adapter: AdapterKind,
    executable: &str,
) -> Result<(), CommandError> {
    let refuse = |why: &str| {
        CommandError::from(StoreError::refused(
            "bad-connection",
            why.to_string(),
            Value::Null,
        ))
    };
    let places = context.places();
    let found = match adapter {
        AdapterKind::ClaudeCode => candidates(&places),
        AdapterKind::Codex => codex::candidates(&places),
        AdapterKind::Local => {
            return Err(refuse(
                "the application finds no program for this kind of connection, so none can be \
                 named",
            ));
        }
    };
    if found.iter().any(|c| c.path == Path::new(executable)) {
        return Ok(());
    }
    Err(refuse(
        "that is not one of the programs the application found and asked: ask for them \
         (`find_clients`) and choose among those",
    ))
}

/// The program a status is asked of: the one the screen is about to keep (`executable`), which is
/// one the application found (anything else is refused, because a command that started any path
/// it was given would be a way to run any file) or its own choice, else the one the connection
/// names.
fn program_asked<C: Clock>(
    context: &Context<'_, C>,
    adapter: AdapterKind,
    connection: Option<&str>,
    executable: Option<Option<String>>,
) -> Result<Option<PathBuf>, CommandError> {
    match executable {
        Some(Some(path)) => {
            check_executable(context, adapter, &path)?;
            Ok(Some(PathBuf::from(path)))
        }
        Some(None) => Ok(None),
        None => connection_program(context, adapter, connection),
    }
}

/// The program a generation would run for a connection to `adapter`, when a connection names one:
/// the connection the screen is about (`wanted`) when that one is kept, else the default connection
/// when it is a connection to `adapter`.
///
/// The connection a screen edits is not always the default (a program kept for a client nobody is
/// signed in to is not made the default, and the default can be a connection to another client), so
/// the program that answers is the edited connection's and not the default's. A connection that is
/// kept and names no program names none: the default's is not borrowed for it. One that is not kept
/// yet is answered as nothing was named, by the default's program, so that a screen can ask before
/// it has saved anything and gets what it did before.
fn connection_program<C: Clock>(
    context: &Context<'_, C>,
    adapter: AdapterKind,
    wanted: Option<&str>,
) -> Result<Option<PathBuf>, CommandError> {
    let Some(settings) = context.connections else {
        return Ok(None);
    };
    if let Some(wanted) = wanted {
        let id = connection_id(wanted)?;
        if let Some(stored) = settings.read(&id, None).ok().flatten() {
            if stored.connection.adapter != adapter {
                return Err(CommandError::from(StoreError::refused(
                    "bad-connection",
                    format!("`{wanted}` is not a connection to `{}`", adapter.word()),
                    Value::Null,
                )));
            }
            return Ok(stored.connection.executable.map(PathBuf::from));
        }
    }
    let Some(id) = settings.default_connection().ok().flatten() else {
        return Ok(None);
    };
    let Some(stored) = settings.read(&id, None).ok().flatten() else {
        return Ok(None);
    };
    Ok(if stored.connection.adapter == adapter {
        stored.connection.executable.map(PathBuf::from)
    } else {
        None
    })
}

/// The commands that start a program of the person's to ask it something.
fn call_program<C: Clock>(
    context: &Context<'_, C>,
    name: &str,
    args: Value,
) -> Result<Value, CommandError> {
    if !context.entrance.starts_programs() {
        return Err(CommandError {
            kind: "not-allowed-here".to_string(),
            message: format!(
                "`{name}` starts a program of the person's, and only the desktop application \
                 does that"
            ),
            detail: json!({ "entrance": context.entrance }),
        });
    }
    match name {
        "read_claude_status" => {
            let ReadClientStatus {
                connection,
                executable,
            } = arguments(args)?;
            // The program a generation would run: the one the screen is about to keep, or the one
            // the connection the screen is about names (the default connection when it names
            // none), or else the one the environment named, or else the first the application
            // finds.
            let named = program_asked(
                context,
                AdapterKind::ClaudeCode,
                connection.as_deref(),
                executable,
            )?;
            let status = claude_status::read(
                named.as_deref().or(context.claude),
                context.policy,
                &context.places(),
            );
            Ok(json!({ "claude": status }))
        }
        "read_codex_status" => {
            let ReadClientStatus {
                connection,
                executable,
            } = arguments(args)?;
            // The application's own home for Codex is in the settings folder, so without one
            // there is no stored login to ask about.
            let Some(settings) = context.connections else {
                return Err(CommandError {
                    kind: "no-settings".to_string(),
                    message: format!(
                        "`{name}` needs a settings folder, and this entrance has none"
                    ),
                    detail: Value::Null,
                });
            };
            let named = program_asked(
                context,
                AdapterKind::Codex,
                connection.as_deref(),
                executable,
            )?;
            let environment: Vec<(String, String)> = std::env::vars().collect();
            let status = codex_status::read(
                named.as_deref().or(context.codex),
                context.policy,
                &Support::shipped(),
                &settings.root().join(codex::HOME_DIR),
                &environment,
                &context.places(),
            );
            Ok(json!({ "codex": status }))
        }
        "find_clients" => {
            arguments::<Empty>(args)?;
            let places = context.places();
            Ok(json!({
                "claude": candidates(&places),
                "codex": codex::candidates(&places),
            }))
        }
        "read_server_status" => {
            let ReadServerStatus { server_url } = arguments(args)?;
            Ok(json!({ "server": server_status::read(&server_url)? }))
        }
        other => Err(CommandError {
            kind: "unknown-command".to_string(),
            message: format!("`{other}` does not start a program"),
            detail: Value::Null,
        }),
    }
}

fn call_settings<C: Clock>(
    context: &Context<'_, C>,
    name: &str,
    args: Value,
) -> Result<Value, CommandError> {
    if SETTINGS_WRITE.contains(&name) && !context.entrance.writes_settings() {
        return Err(CommandError {
            kind: "not-allowed-here".to_string(),
            message: format!(
                "`{name}` changes the person's settings, and only the desktop application does that"
            ),
            detail: json!({ "entrance": context.entrance }),
        });
    }
    let Some(connections) = context.connections else {
        return Err(CommandError {
            kind: "no-settings".to_string(),
            message: format!("`{name}` needs a settings folder, and this entrance has none"),
            detail: Value::Null,
        });
    };
    match name {
        "list_connections" => {
            arguments::<Empty>(args)?;
            let listing = connections.list()?;
            Ok(json!({
                "connections": listing.connections,
                "unreadable": listing.unreadable,
                "default": connections.default_connection()?,
            }))
        }
        "read_connection" => {
            let ReadConnection { id, revision } = arguments(args)?;
            let id = connection_id(&id)?;
            Ok(json!({ "connection": connections.read(&id, revision.as_ref())? }))
        }
        "read_auth_policy" => {
            arguments::<Empty>(args)?;
            let routes: Vec<Value> = Route::ALL
                .iter()
                .map(|route| {
                    json!({
                        "route": route,
                        "status": route.status(),
                        "decision": context.policy.decide(*route),
                    })
                })
                .collect();
            Ok(json!({ "routes": routes, "switched_off": context.policy.switched_off() }))
        }
        "save_connection" => {
            let SaveConnection { connection, base } = arguments(args)?;
            // A command that took any path would be a way to make the application run a file, for
            // anyone who can send the command. A program is chosen among the ones the application
            // found and asked, and only those are kept.
            if let Some(executable) = connection.executable.as_deref() {
                check_executable(context, connection.adapter, executable)?;
            }
            answer(&connections.save(&connection, base.as_ref())?)
        }
        "delete_connection" => {
            let DeleteConnection { id, base } = arguments(args)?;
            let id = connection_id(&id)?;
            connections.delete(&id, &base)?;
            Ok(json!({ "deleted": id }))
        }
        "set_default_connection" => {
            let SetDefaultConnection { id, expect } = arguments(args)?;
            let id = id.as_deref().map(connection_id).transpose()?;
            let expect = expect.as_deref().map(connection_id).transpose()?;
            connections.set_default(id.as_ref(), expect.as_ref())?;
            Ok(json!({ "default": id }))
        }
        other => unreachable!("`{other}` is a settings command that has no handler"),
    }
}

/// The commands on the works folder.
fn call_works<C: Clock>(
    store: &WorkStore<C>,
    renderer: &dyn Product,
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
        "list_works" => {
            arguments::<Empty>(args)?;
            answer(&store.list_works()?)
        }
        "create_work" => {
            let CreateWork { title } = arguments(args)?;
            answer(&store.create_work(&title)?)
        }
        "read_work" => {
            let OneWork { id } = arguments(args)?;
            let id = work_id(&id)?;
            let work = store.read_work(&id)?;
            let head = store.head(&id)?;
            Ok(json!({ "work": work, "head": head }))
        }
        "read_source" => {
            let ReadSource { id, revision } = arguments(args)?;
            let source = store.read_source(&work_id(&id)?, revision.as_ref())?;
            Ok(json!({ "source": source }))
        }
        "save_source" => {
            let SaveSource { id, text, base } = arguments(args)?;
            answer(&store.save_source(&work_id(&id)?, &text, base.as_ref())?)
        }
        "history" => {
            let OneWork { id } = arguments(args)?;
            Ok(json!({ "entries": store.history(&work_id(&id)?)? }))
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
                &work_id(&id)?,
                &files.stored_text(),
                base.as_ref(),
                written_for.as_ref(),
            )?)
        }
        "read_model" => {
            let ReadSource { id, revision } = arguments(args)?;
            let id = work_id(&id)?;
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
            let OneWork { id } = arguments(args)?;
            Ok(json!({ "entries": store.model_history(&work_id(&id)?)? }))
        }
        "figures" => {
            let Figures {
                id,
                revision,
                page,
                lexicon,
                min_pt,
            } = arguments(args)?;
            let id = work_id(&id)?;
            let (model, files) = read_model_files(store, &id, revision.as_ref())?;
            let others: Vec<Document> = files.others().into_iter().cloned().collect();
            let source_head = store.head(&id)?;
            let drawn = renderer.render(&FigureRequest {
                model: files.entry_text(),
                // The product titles its figures by the document's name, which it
                // takes from the file's: the work's own name, not `model`. A model of
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
            let id = work_id(&id)?;
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
            let id = work_id(&id)?;
            let read = store.read_answers(&id, revision.as_ref())?;
            let answers = match read {
                None => Value::Null,
                Some(saved) => answers_json(&saved)?,
            };
            Ok(json!({ "answers": answers }))
        }
        "save_answers" => {
            let SaveAnswers { id, answers, base } = arguments(args)?;
            let id = work_id(&id)?;
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
                &work_id(&id)?,
                &list.stored_text(),
                base.as_ref(),
                written_for.as_ref(),
            )?)
        }
        "read_requirements" => {
            let ReadSource { id, revision } = arguments(args)?;
            let id = work_id(&id)?;
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
            let OneWork { id } = arguments(args)?;
            let id = work_id(&id)?;
            let now = work_now(store, &id)?;
            let report = renderer.report_requirements(&now.snapshot)?;
            Ok(report_json(&now, &report))
        }
        "read_judgment" => {
            let ReadJudgment {
                id,
                basis,
                acceptance,
            } = arguments(args)?;
            let id = work_id(&id)?;
            // SCE is asked about the revisions named and about nothing read after them, so the
            // answer is of the state the caller read and of no other: a save landing now moves
            // the work, and the caller learns that from the heads, not from a verdict that is
            // of a design it was not shown.
            let named = work_at(store, &id, &basis)?;
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
            let id = work_id(&id)?;
            let now = work_now(store, &id)?;
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
            let OneWork { id } = arguments(args)?;
            let id = work_id(&id)?;
            let Some(saved) = store.read_acceptance(&id, None)? else {
                return Ok(json!({
                    "acceptance": null,
                    "standing": "none",
                    "lapse": null,
                }));
            };
            let acceptance = Acceptance::parse(&saved.text)?;
            // Whether it still holds is the product's to say, about the work as it is
            // now laid out the way it was when the record was taken.
            let now = work_now(store, &id)?;
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
            let OneWork { id } = arguments(args)?;
            let id = work_id(&id)?;
            // ONE state of the work: the acceptance and the design and list it is compared with
            // are read together, so a save landing between them cannot make the comparison one
            // of an acceptance and a design that were never both the work's.
            let state = store.read_work_snapshot(&id)?;
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
            let id = work_id(&id)?;
            // ONE state of the work, as `read_acceptance_delta` reads it: the acceptance, and the
            // design and list it is compared with.
            let state = store.read_work_snapshot(&id)?;
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
            // The list the owner accepted is the work's own, read at the revision the
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
                "work {id}, list {} to {}",
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
        "read_work_snapshot" => {
            let OneWork { id } = arguments(args)?;
            let snapshot = store.read_work_snapshot(&work_id(&id)?)?;
            // The parts are the answers of the commands that read one chain, in their
            // words, so the screen holds one definition of each; what differs is that
            // they were read as one state of the work and not as six.
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
                "work": snapshot.work,
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
        "read_work_heads" => {
            let OneWork { id } = arguments(args)?;
            answer(&store.read_work_heads(&work_id(&id)?)?)
        }
        "read_request" => {
            let OneRequest { id, request } = arguments(args)?;
            let id = work_id(&id)?;
            let view = store.read_request(&id, &request)?;
            Ok(json!({ "request": request_json(&id, &view) }))
        }
        "list_requests" => {
            let OneWork { id } = arguments(args)?;
            let id = work_id(&id)?;
            let views = store.list_requests(&id)?;
            let requests: Vec<Value> = views.iter().map(|v| request_json(&id, v)).collect();
            Ok(json!({ "requests": requests }))
        }
        "list_open_requests" => {
            arguments::<Empty>(args)?;
            let requests: Vec<Value> = store
                .open_requests()?
                .iter()
                .map(|(work, view)| request_json(work, view))
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
            let id = work_id(&id)?;
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
            let id = work_id(&id)?;
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
            let id = work_id(&id)?;
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
            let id = work_id(&id)?;
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
            let id = work_id(&id)?;
            let texts = store.read_candidate(&id, &request)?;
            Ok(json!({
                "request": request,
                "model": texts.model.map(|(revision, text)| json!({ "revision": revision, "text": text })),
                "requirements": texts
                    .requirements
                    .map(|(revision, text)| json!({ "revision": revision, "text": text })),
            }))
        }
        "read_bundle" => {
            let OneBundle { id, revision } = arguments(args)?;
            let id = work_id(&id)?;
            let read = store.read_bundle(&id, revision.as_ref())?;
            Ok(json!({
                "bundle": read.map(|r| json!({ "revision": r.revision, "bundle": r.bundle })),
            }))
        }
        "bundle_history" => {
            let OneWork { id } = arguments(args)?;
            Ok(json!({ "entries": store.bundle_history(&work_id(&id)?)? }))
        }
        "fail_request" => {
            let FailRequest {
                id,
                request,
                holder,
                attempt,
                reason,
            } = arguments(args)?;
            let id = work_id(&id)?;
            let view = store.fail_request(&id, &request, &holder, attempt, &reason)?;
            Ok(json!({ "request": request_json(&id, &view) }))
        }
        "cancel_request" => {
            let OneRequest { id, request } = arguments(args)?;
            let id = work_id(&id)?;
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
        "remove_work" => {
            let OneWork { id } = arguments(args)?;
            Ok(json!({ "removed": store.remove_work(&work_id(&id)?)? }))
        }
        other => Err(CommandError {
            kind: "unknown-command".to_string(),
            message: format!(
                "`{other}` is not a command; the commands are {}",
                COMMANDS.join(", ")
            ),
            detail: Value::Null,
        }),
    }
}
