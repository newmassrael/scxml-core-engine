// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What every client that writes a draft shares: the task it is given, the form it answers in,
//! the folder it works in, and the way it is run and stopped.
//!
//! A client is a program the application starts to write the model of a work (Claude Code,
//! Codex). What it is told, and what it must answer, is the same for each: the work is read through
//! the SCE authoring server and the draft comes back as one JSON document in one form. What
//! differs is how each program is started, what it is refused, and how its own result is
//! wrapped, and that is each adapter's own. Keeping the shared part here is what keeps a change
//! to the task or to the form from being made for one client and not the other.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::claude_code::AuthorServer;
use crate::document_files::refuse_file_access;
use crate::model_set::{Document, ModelFiles};
use crate::requirements::Requirements;
use crate::revision::Revision;
use crate::runner::{Cancel, Draft, Job};
use crate::store::WorkId;

/// The most of what a client writes that is kept to show the owner when it fails.
const KEPT_BYTES: usize = 16 * 1024 * 1024;

/// What the SCE authoring server is called to a client, and so the prefix of its tools.
pub(crate) const SERVER: &str = "sce-author";

/// The authoring server's tools a client may use, by the names the server gives them. Reading a
/// work and checking a draft; not saving, not taking a request, not accepting. A client that
/// reaches the server by its own means offers these and no others; one that is told by name
/// (Claude Code, `claude_code::ALLOWED_TOOLS`) is told the same nine with the server's prefix.
pub const AUTHOR_TOOLS: [&str; 9] = [
    "works_read",
    "scxml_kinds",
    "validate_scxml",
    "validate_scxml_set",
    "scxml_unresolved",
    "decisions",
    "scxml_requirement_set",
    "scxml_requirements",
    "render_scxml_pseudocode",
];

/// The variable that tells the authoring server which work a run is for.
pub(crate) const WORK_SCOPE: &str = "SCE_AUTHOR_WORK";

/// The authoring server a run reaches: the one the host knows, told which work the run is for.
/// Told, the server advertises only [`AUTHOR_TOOLS`], refuses a read of another work, reads a
/// document only as text under a plain name, and holds the generator to its working folder. Not
/// told, it answers for every work of the folder, so a specification that names another work in a
/// call is read by the client. A work the host named for the server is replaced: the work is the
/// run's to say, and not a setting that was there before it.
pub(crate) fn scoped_to(author: &AuthorServer, work: &str) -> AuthorServer {
    let mut scoped = author.clone();
    scoped.env.retain(|(name, _)| name != WORK_SCOPE);
    scoped.env.push((WORK_SCOPE.to_string(), work.to_string()));
    scoped
}

/// The documents of the model and the requirement list, as the application keeps them.
pub(crate) fn draft_from(answer: &Value) -> Result<Draft, String> {
    let documents = answer["model"]["documents"]
        .as_array()
        .ok_or("the answer has no `model.documents` list: the model is missing")?;
    let documents: Vec<Document> = documents
        .iter()
        .map(|d| match (d["name"].as_str(), d["text"].as_str()) {
            (Some(name), Some(text)) => Ok(Document {
                name: name.to_string(),
                text: text.to_string(),
            }),
            _ => Err("every entry of `model.documents` has a `name` and a `text`".to_string()),
        })
        .collect::<Result<_, _>>()?;
    // The answer is the last thing a client writes and the first the application hands to the
    // checker, with no authoring server between: what the server refuses a document for when it
    // is handed over to be checked is refused here too, before the checker opens a file it names.
    for document in &documents {
        refuse_file_access(&document.name, &document.text)
            .map_err(|why| format!("the answer is not usable: {why}"))?;
    }
    let entry = answer["model"]["entry"].as_str();
    let model = match (documents.len(), entry) {
        (0, _) => return Err("`model.documents` is empty: the model has no documents".to_string()),
        // One document is the model as it always was, whatever name it was given.
        (1, None) => ModelFiles::single(documents[0].text.clone()),
        _ => ModelFiles::set(documents, entry).map_err(|e| e.to_string())?,
    };
    let manifest = answer["requirements"]["manifest_text"]
        .as_str()
        .ok_or("the answer has no `requirements.manifest_text`: the requirement list is missing")?
        .to_string();
    let sidecar = answer["requirements"]["sidecar_text"]
        .as_str()
        .map(str::to_string);
    let lineage = answer["requirements"]["lineage_text"]
        .as_str()
        .map(str::to_string);
    let requirements = Requirements::new(manifest, sidecar)
        .and_then(|list| list.with_lineage(lineage))
        .map_err(|e| {
            format!("the requirement list is not usable: its manifest, sidecar or lineage: {e}")
        })?;
    Ok(Draft {
        model,
        requirements,
    })
}

/// The form a client must answer in.
pub(crate) fn schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["model", "requirements"],
        "properties": {
            "model": {
                "type": "object",
                "additionalProperties": false,
                "required": ["documents", "entry"],
                "properties": {
                    "documents": {
                        "type": "array",
                        "minItems": 1,
                        "items": {
                            "type": "object",
                            "additionalProperties": false,
                            "required": ["name", "text"],
                            "properties": {
                                "name": {"type": "string"},
                                "text": {"type": "string"},
                            },
                        },
                    },
                    "entry": {"type": ["string", "null"]},
                },
            },
            "requirements": {
                "type": "object",
                "additionalProperties": false,
                "required": ["manifest_text", "sidecar_text", "lineage_text"],
                "properties": {
                    "manifest_text": {"type": "string"},
                    "sidecar_text": {"type": ["string", "null"]},
                    "lineage_text": {"type": ["string", "null"]},
                },
            },
        },
    })
}

/// Said to a client beside what the product already tells it (the authoring server's own
/// instructions). What is different here is that nobody is on the other end.
pub(crate) const SYSTEM_PROMPT: &str = "You are the model-writing step of the SCE specification \
workbench. No person is on the other end of this conversation, so never ask a question: a \
decision the specification does not make is marked in the model as the authoring flow says \
(sce:unresolved), never guessed. You can read the work and check what you write, and you \
cannot save anything: the application saves what you answer, in the form it asks for. Your \
last message is that answer and nothing else.";

/// A job with nothing of a work in it: the task's wording, for naming a version of it.
pub(crate) fn blank_job() -> Job {
    Job {
        work: WorkId::parse("work").expect("a valid id"),
        title: String::new(),
        request: String::new(),
        attempt: 0,
        source: String::new(),
        source_revision: Revision::of(b""),
        answers: Default::default(),
        previous: None,
        refusal: None,
    }
}

/// What a client is asked to do for `job`.
pub(crate) fn prompt(job: &Job) -> String {
    let mut text = format!(
        "Write the model for the work `{work}` (\"{title}\"), for request {request}, attempt \
         {attempt}.\n\n\
         1. Call works_read with work = \"{work}\". It gives the specification (`source.text`), \
            the owner's answers to the questions an earlier model left open (`answers`) and the \
            decision record they make (`decisions_text`), and the model the work has now, if it \
            has one.\n\
         2. Follow the authoring flow in your instructions: choose the document kind from what \
            the specification states (scxml_kinds), write the SCXML, check it with validate_scxml \
            (validate_scxml_set for several documents that import each other), and hold it to \
            the owner's answers with the decisions tool and `decisions_text`. Apply each answer \
            and cite it as sce:assumed=\"<id>\"; never leave an answered question \
            sce:unresolved.\n\
         3. Build the requirement list from `source.text` with scxml_requirement_set, and check \
            the design against it with scxml_requirements. When works_read gave \
            `requirements.lineage_text`, the work already holds a list: build this one against \
            it, giving that as `lineage_text` and `requirements.sidecar_text` as \
            `previous_sidecar_text`, so that every requirement keeps the id it had and a retired \
            id is not issued again. Hand documents over as text. For a tool that takes \
            document_text, also provide companions_text for every imported SCXML document, under \
            the exact file name used by its import. Call decisions only when works_read provided \
            a decisions_text record.\n\
         4. Do not call any tool that saves, takes a request or accepts: you have none. Your \
            last message is the draft in the form asked for: the model's documents exactly as \
            you checked them, and `manifest_text`, `sidecar_text` and `lineage_text` exactly as \
            scxml_requirement_set returned them.\n",
        work = job.work.as_str(),
        title = job.title,
        request = job.request,
        attempt = job.attempt,
    );
    if let Some(refusal) = &job.refusal {
        text.push_str(&format!(
            "\nYour last draft was refused. What was said of it:\n{refusal}\n\
             Write the draft again so that each finding is answered, and check it again before \
             you answer.\n"
        ));
    }
    text
}

/// A folder for one run, private to the owner and removed with it.
pub(crate) struct Scratch(PathBuf);

static SCRATCH_COUNTER: AtomicU64 = AtomicU64::new(0);

impl Scratch {
    /// A new folder whose name starts `prefix`, in the system's temporary folder.
    pub(crate) fn new(prefix: &str) -> io::Result<Scratch> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        for _ in 0..8 {
            let path = std::env::temp_dir().join(format!(
                "{prefix}-{}-{}-{nanos}",
                std::process::id(),
                SCRATCH_COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        fs::set_permissions(&path, fs::Permissions::from_mode(0o700))?;
                    }
                    return Ok(Scratch(path));
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "eight generated folder names were all taken",
        ))
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// How a run of a client ended.
pub(crate) enum Ended {
    Exited {
        status: ExitStatus,
        stdout: String,
        stderr: String,
    },
    Cancelled,
    TimedOut,
}

/// Feed the client its input, read everything it says as it says it, and wait for it to end,
/// to be told to stop, or to run out of time.
///
/// Reading is not left until the client has exited: more than a pipe holds, written to a pipe
/// nobody reads, holds the client at its write for ever and it never exits.
pub(crate) fn supervise(
    mut child: Child,
    input: Option<Vec<u8>>,
    cancel: &Cancel,
    timeout: Duration,
) -> io::Result<Ended> {
    let writer = child.stdin.take().map(|mut stdin| {
        thread::spawn(move || {
            if let Some(input) = input {
                // A client that stops before it has read everything closes the pipe: not an
                // error of ours, and its exit says what happened.
                let _ = stdin.write_all(&input);
            }
        })
    });
    let out = child.stdout.take().map(reader);
    let err = child.stderr.take().map(reader);

    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if cancel.is_cancelled() {
            stop(&mut child);
            break None;
        }
        if started.elapsed() > timeout {
            stop(&mut child);
            let _ = join(writer, out, err);
            return Ok(Ended::TimedOut);
        }
        thread::sleep(Duration::from_millis(20));
    };
    let (stdout, stderr) = join(writer, out, err);
    Ok(match status {
        Some(status) => Ended::Exited {
            status,
            stdout,
            stderr,
        },
        None => Ended::Cancelled,
    })
}

fn stop(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// Read a pipe to its end on a thread of its own; what is kept is the first part of it.
fn reader<R: Read + Send + 'static>(mut pipe: R) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let mut kept = Vec::new();
        let mut chunk = [0u8; 8192];
        loop {
            match pipe.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    // Past the limit it is still read, so that the client is never held at a
                    // write, and not kept.
                    if kept.len() < KEPT_BYTES {
                        let room = KEPT_BYTES - kept.len();
                        kept.extend_from_slice(&chunk[..n.min(room)]);
                    }
                }
            }
        }
        String::from_utf8_lossy(&kept).into_owned()
    })
}

fn join(
    writer: Option<thread::JoinHandle<()>>,
    out: Option<thread::JoinHandle<String>>,
    err: Option<thread::JoinHandle<String>>,
) -> (String, String) {
    if let Some(writer) = writer {
        let _ = writer.join();
    }
    let take = |handle: Option<thread::JoinHandle<String>>| {
        handle
            .map(|h| h.join().unwrap_or_default())
            .unwrap_or_default()
    };
    (take(out), take(err))
}

/// The last `limit` characters of `text`, for a message that says what the client said.
pub(crate) fn tail(text: &str, limit: usize) -> String {
    let count = text.chars().count();
    if count <= limit {
        return text.to_string();
    }
    let kept: String = text.chars().skip(count - limit).collect();
    format!("...{kept}")
}

/// A time limit as a person reads it: `30 minutes`, `90 seconds`, `1 second`. Whole minutes are
/// said in minutes and the rest as it is, so that a limit of ninety seconds is not reported as
/// two minutes, and one of a single second is not reported as one minute.
pub(crate) fn span_words(span: Duration) -> String {
    let count = |n: u128, unit: &str| {
        if n == 1 {
            format!("{n} {unit}")
        } else {
            format!("{n} {unit}s")
        }
    };
    if span.subsec_nanos() != 0 {
        return count(span.as_millis(), "millisecond");
    }
    let seconds = span.as_secs();
    let minutes = seconds / 60;
    if minutes >= 1 && Duration::from_secs(minutes * 60) == span {
        count(u128::from(minutes), "minute")
    } else {
        count(u128::from(seconds), "second")
    }
}

/// What a program said when it was run to completion.
pub(crate) struct Captured {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

/// Run `command` to its end, reading what it says as it says it, and give up on it after
/// `timeout`. For the short questions asked of a program (`--version`, `--check`).
pub(crate) fn capture(mut command: Command, timeout: Duration) -> Result<Captured, String> {
    let child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    match supervise(child, None, &Cancel::new(), timeout).map_err(|e| e.to_string())? {
        Ended::Exited {
            status,
            stdout,
            stderr,
        } => Ok(Captured {
            status,
            stdout,
            stderr,
        }),
        Ended::TimedOut => Err(format!(
            "it did not answer within {} seconds",
            timeout.as_secs()
        )),
        Ended::Cancelled => Err("it was stopped".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCXML: &str = r#"xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext""#;

    /// An answer whose model is `documents` (name and text) with `entry` as its entry.
    fn answer_of(documents: &[(&str, String)], entry: Option<&str>) -> Value {
        json!({
            "model": {
                "documents": documents
                    .iter()
                    .map(|(name, text)| json!({"name": name, "text": text}))
                    .collect::<Vec<_>>(),
                "entry": entry,
            },
            "requirements": {"manifest_text": "{}\n", "sidecar_text": null},
        })
    }

    /// The form a client answers in asks for a lineage and the task tells it to build the list
    /// against the one the work holds. That form and the task beside it are what a Codex version
    /// is verified against, so asking for it is a new execution contract, to be verified again
    /// with the real client before a Codex version is listed for it. The draft reads a lineage when an answer has one, and a draft
    /// without one is a list without one, as every list was before lineages.
    #[test]
    fn a_lineage_in_the_answer_is_the_drafts_and_its_absence_is_a_list_without_one() {
        let lineage = "{\"lineage\":\"sce-requirement-lineage\",\"v\":1,\"doc_id\":\"d\",\
                       \"next\":2,\"revisions\":[],\"requirements\":[]}\n";
        let doc = || {
            vec![(
                "m.scxml",
                format!(r#"<scxml {SCXML} initial="a"><state id="a"/></scxml>"#),
            )]
        };
        let mut said = answer_of(&doc(), None);
        said["requirements"]["lineage_text"] = json!(lineage);
        assert_eq!(
            draft_from(&said).unwrap().requirements.lineage.as_deref(),
            Some(lineage)
        );
        // An answer that has none, as every answer of the form a client is given has none, and
        // one that says `null`, are a list without one.
        assert_eq!(
            draft_from(&answer_of(&doc(), None))
                .unwrap()
                .requirements
                .lineage,
            None
        );
        let mut nulled = answer_of(&doc(), None);
        nulled["requirements"]["lineage_text"] = json!(null);
        assert_eq!(draft_from(&nulled).unwrap().requirements.lineage, None);
        // What is not a lineage is not a draft.
        let mut wrong = answer_of(&doc(), None);
        wrong["requirements"]["lineage_text"] = json!("not a lineage");
        let said = draft_from(&wrong).unwrap_err();
        assert!(said.contains("manifest, sidecar or lineage"), "{said}");
    }

    #[test]
    fn an_answer_whose_document_names_a_file_is_not_a_draft() {
        // The answer is handed to the checker as it is, and what the checker finds in a file the
        // document names comes back in its diagnostics, to the AI, when a draft is asked for again.
        let text = format!(
            r#"<scxml {SCXML} initial="a"><sce:import as="L" src="/private/secret.scxml" kind="enum"/></scxml>"#
        );

        let said = draft_from(&answer_of(&[("main.scxml", text)], None)).unwrap_err();

        assert!(said.starts_with("the answer is not usable"), "{said}");
        assert!(said.contains("main.scxml"), "{said}");
    }

    #[test]
    fn what_a_companion_names_is_checked_as_what_the_entry_names_is() {
        let entry = format!(
            r#"<scxml {SCXML} initial="a"><sce:import as="L" src="levels.scxml" kind="enum"/></scxml>"#
        );
        let companion = format!(
            r#"<scxml {SCXML} initial="a"><sce:import as="M" src="../secret.scxml" kind="enum"/></scxml>"#
        );

        let said = draft_from(&answer_of(
            &[("main.scxml", entry), ("levels.scxml", companion)],
            Some("main.scxml"),
        ))
        .unwrap_err();

        assert!(said.contains("levels.scxml"), "{said}");
    }

    #[test]
    fn documents_that_name_each_other_by_plain_names_are_a_draft() {
        let entry = format!(
            r#"<scxml {SCXML} initial="a"><sce:import as="L" src="levels.scxml" kind="enum"/></scxml>"#
        );
        let companion = format!(r#"<scxml {SCXML} initial="a"><state id="a"/></scxml>"#);

        let draft = draft_from(&answer_of(
            &[("main.scxml", entry), ("levels.scxml", companion)],
            Some("main.scxml"),
        ));

        assert!(draft.is_ok(), "{:?}", draft.err());
    }

    #[test]
    fn the_answer_schema_satisfies_strict_structured_output_rules() {
        fn check(value: &Value) {
            if value["type"] == "object" {
                assert_eq!(value["additionalProperties"], false);
                let properties = value["properties"].as_object().unwrap();
                let required = value["required"].as_array().unwrap();
                assert_eq!(required.len(), properties.len());
                for (key, child) in properties {
                    assert!(required.contains(&json!(key)));
                    check(child);
                }
            }
            if let Some(items) = value.get("items") {
                check(items);
            }
        }
        let answer = schema();
        check(&answer);
        assert_eq!(
            answer["properties"]["model"]["properties"]["entry"]["type"],
            json!(["string", "null"])
        );
        assert_eq!(
            answer["properties"]["requirements"]["properties"]["sidecar_text"]["type"],
            json!(["string", "null"])
        );
        // A list may have no lineage (a first list of a client that was not given one), and a
        // strict form lists every property as required, so the lineage is one that may be null.
        assert_eq!(
            answer["properties"]["requirements"]["properties"]["lineage_text"]["type"],
            json!(["string", "null"])
        );
    }

    /// What the task tells a client about a lineage is the names the authoring server gives the
    /// pieces: it reads `requirements.lineage_text` from works_read and gives it back to
    /// scxml_requirement_set as `lineage_text` beside `previous_sidecar_text`. A wording that
    /// drifts from either name sends the client to an argument the tool does not have.
    #[test]
    fn the_task_names_the_lineage_by_the_names_the_server_uses() {
        let said = prompt(&blank_job());
        for name in [
            "requirements.lineage_text",
            "`lineage_text`",
            "requirements.sidecar_text",
            "`previous_sidecar_text`",
        ] {
            assert!(said.contains(name), "the task does not name {name}: {said}");
        }
    }

    #[test]
    fn a_limit_is_said_in_the_unit_it_was_given_in() {
        for (span, said) in [
            (Duration::from_secs(30 * 60), "30 minutes"),
            (Duration::from_secs(60), "1 minute"),
            (Duration::from_secs(120), "2 minutes"),
            // Not a whole number of minutes: not rounded up to one.
            (Duration::from_secs(90), "90 seconds"),
            (Duration::from_secs(45), "45 seconds"),
            (Duration::from_secs(1), "1 second"),
            (Duration::from_millis(1500), "1500 milliseconds"),
            (Duration::from_millis(400), "400 milliseconds"),
            (Duration::from_millis(1), "1 millisecond"),
        ] {
            assert_eq!(span_words(span), said, "{span:?}");
        }
    }
}
