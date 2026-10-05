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

use crate::acceptance::{Acceptance, AcceptanceCorrupt, Basis, Snapshot};
use crate::acceptance_run::CheckOutcome;
use crate::answers::{Answers, AnswersError};
use crate::bundle::{BundleCheck, CheckedBy};
use crate::clock::{utc_timestamp, Clock};
use crate::error::StoreError;
use crate::figures::{FigureRequest, RenderError};
use crate::model_set::{Document, ModelError, ModelFiles};
use crate::requests::{Inputs, Lease};
use crate::requirements::{Requirements, RequirementsError};
use crate::review::{Product, Review, ReviewRequest, Verdict};
use crate::revision::Revision;
use crate::store::{
    AdapterReport, AdapterStatus, AnswersText, CandidateWrite, ModelText, Published, Registration,
    RequestView, RequirementsText, WorkId, WorkStore,
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
    "read_bundle",
    "bundle_history",
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
/// works (`list_open_requests`). A request says its `candidate` and, once done, its
/// `outcome`; the heads and the snapshot of a work say their `bundle`. A work that keeps
/// bundles refuses `save_model` and `save_requirements` with `bundled-work`. A screen
/// written for 11 reads a work's model by the bundle and would be refused by a core of 10.
pub const COMMAND_SET_VERSION: u32 = 11;

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
        "ended_at": request.ended_at,
        "note": request.note,
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
    let answers = state.answers;
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
/// written for, the manifest and the sidecar of quoted sentences.
fn requirements_json(saved: &RequirementsText) -> Result<Value, CommandError> {
    let list = Requirements::parse(&saved.text)?;
    Ok(json!({
        "revision": saved.revision,
        "written_for": saved.written_for,
        "manifest": list.manifest,
        "sidecar": list.sidecar,
    }))
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

/// Run the command `name` with `args` against `store`; `renderer` is the
/// product: it draws a model for `figures` and reads one for `review`.
pub fn call<C: Clock>(
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
                base,
                written_for,
            } = arguments(args)?;
            let list = Requirements::new(manifest, sidecar)?;
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
            let source_head = store.head(&id)?;
            let report = renderer.report_requirements(&now.snapshot)?;
            Ok(json!({
                "basis": now.basis,
                "source_head": source_head,
                "model_standing": standing(now.model_written_for.as_ref(), source_head.as_ref()),
                "requirements_standing":
                    standing(now.requirements_written_for.as_ref(), source_head.as_ref()),
                "generator": report.generator,
                "denominator": report.denominator,
                "outcomes": report.outcomes,
                "page": report.page,
                "page_refusal": report.page_refusal,
            }))
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
                match renderer.check_acceptance(&now.snapshot, &acceptance.record)? {
                    CheckOutcome::Holds => ("holds", Value::Null),
                    CheckOutcome::Lapsed { says } => ("lapsed", Value::String(says)),
                };
            Ok(json!({
                "acceptance": acceptance_json(&saved.revision, &acceptance),
                "standing": standing,
                "lapse": lapse,
                "now": now.basis,
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
        "request_generation" => {
            let RequestGeneration {
                id,
                key,
                origin,
                expect,
                supersede,
            } = arguments(args)?;
            let id = work_id(&id)?;
            let made = store.register_request(
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
            )?;
            let view = RequestView {
                request: made.request,
                state: made.state,
            };
            Ok(json!({ "request": request_json(&id, &view), "created": made.created }))
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
            } = arguments(args)?;
            let id = work_id(&id)?;
            let view = store.claim_request(&id, &request, &holder, ttl_seconds, resume)?;
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
            } = arguments(args)?;
            let model = match (&text, &documents, &entry) {
                (None, None, None) => None,
                _ => Some(model_files_of(text, documents, entry)?.stored_text()),
            };
            let requirements = match (manifest, sidecar) {
                (None, None) => None,
                (Some(manifest), sidecar) => {
                    Some(Requirements::new(manifest, sidecar)?.stored_text())
                }
                (None, Some(_)) => {
                    return Err(CommandError::bad_request(
                        "a `sidecar` belongs to a `manifest`: give both, or neither",
                    ))
                }
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
