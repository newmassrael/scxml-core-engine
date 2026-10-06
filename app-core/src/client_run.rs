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
    let requirements = Requirements::new(manifest, sidecar)
        .map_err(|e| format!("the requirement list is not usable: its manifest or sidecar: {e}"))?;
    Ok(Draft {
        model,
        requirements,
    })
}

/// The form a client must answer in.
pub(crate) fn schema() -> Value {
    json!({
        "type": "object",
        "required": ["model", "requirements"],
        "properties": {
            "model": {
                "type": "object",
                "required": ["documents"],
                "properties": {
                    "documents": {
                        "type": "array",
                        "minItems": 1,
                        "items": {
                            "type": "object",
                            "required": ["name", "text"],
                            "properties": {
                                "name": {"type": "string"},
                                "text": {"type": "string"},
                            },
                        },
                    },
                    "entry": {"type": "string"},
                },
            },
            "requirements": {
                "type": "object",
                "required": ["manifest_text"],
                "properties": {
                    "manifest_text": {"type": "string"},
                    "sidecar_text": {"type": "string"},
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
            the design against it with scxml_requirements.\n\
         4. Do not call any tool that saves, takes a request or accepts: you have none. Your \
            last message is the draft in the form asked for: the model's documents exactly as \
            you checked them, and `manifest_text` and `sidecar_text` exactly as \
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
