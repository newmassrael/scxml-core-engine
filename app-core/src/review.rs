// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What SCE says of a model, for the person who reads it against the text.
//!
//! The figures show a model; this is what a person reads it by. Two runs of the
//! product's own generator, and nothing the workbench worked out itself:
//!
//! - `sce-codegen check --lint`, the product's verdict on the document: whether
//!   it accepts it, every record it wrote (a refusal names the construct and the
//!   line), and what an accepted model still leaves to a person (`open`: the
//!   questions the specification left unanswered, which the product counts and
//!   words itself);
//! - `sce-codegen pseudo`, the pseudocode page: the document written as sentences
//!   a person compares with the prose, in the vocabulary of the screen's language.
//!
//! ⚠ A model the product refuses is an ANSWER, not an error: the person is shown
//! the records and no page, because there is no page of a document SCE will not
//! read. `Err` is for the product not answering at all (absent, too slow, or
//! failing without a word of its own).
//!
//! ⚠ Accepted is the product's verdict on the document and not a claim that the
//! model agrees with the specification. Nothing here, and nothing on the screen,
//! says it does: the person compares the page with their own text.

use std::fs;

use serde::Serialize;
use serde_json::Value;

use crate::figures::{
    excerpt, refusal, run_bounded, stem, word, FigureRenderer, NoRenderer, RenderError, SceCodegen,
    Scratch,
};

/// What to read and how.
#[derive(Debug, Clone, Default)]
pub struct ReviewRequest<'a> {
    /// The model: an SCXML document, as text.
    pub model: &'a str,
    /// What the document is called; it is staged under this name, as for figures.
    pub name: Option<&'a str>,
    /// A vocabulary the product lists (`en`, `ko`). Its default when absent.
    pub lexicon: Option<&'a str>,
}

/// The product's verdict on a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Verdict {
    Accepted,
    Refused,
}

/// One thing the specification left unanswered, where the model marks it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Unresolved {
    /// The id the model gave the question (`sce:unresolved="..."`).
    pub id: String,
    /// Where in the document it sits, in the product's own path syntax.
    pub node_path: String,
    pub line: Option<u64>,
}

/// One record the product wrote about the document, in its words.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Record {
    /// The product's code for it (`validation/invalid-reference`).
    pub code: String,
    pub message: String,
    pub stage: Option<String>,
    pub line: Option<u64>,
    /// What the product suggests, passed on as it wrote it.
    pub fix: Option<Value>,
}

/// The product's check of a document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Check {
    pub verdict: Verdict,
    /// The kind the product read the document as (`statechart`, `lookup`, ...);
    /// absent for a document it refused.
    pub kind: Option<String>,
    /// What an accepted document still leaves to a person, one sentence each, as
    /// the product words them. Empty for a refused one.
    pub open: Vec<String>,
    /// The questions the document marks as not decided.
    pub unresolved: Vec<Unresolved>,
    /// Every record, in the order the product wrote them.
    pub records: Vec<Record>,
}

/// Why the product would not write the page of an accepted document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PageRefusal {
    pub code: String,
    pub message: String,
}

/// What SCE said of a model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Review {
    /// The generator's own version line, when it gave one.
    pub generator: Option<String>,
    pub check: Check,
    /// The pseudocode page, byte for byte as the product wrote it. `None` for a
    /// document it refused, or whose page it refused (`page_refusal`).
    pub page: Option<String>,
    pub page_refusal: Option<PageRefusal>,
}

/// Something that can read a model: the product's check and its page.
pub trait ModelReviewer: Send + Sync {
    fn review(&self, request: &ReviewRequest<'_>) -> Result<Review, RenderError>;
}

/// The product as the command layer uses it: it draws a model and reads one.
/// Implemented by anything that does both, so a test double gives each its own
/// answer and the real generator gives both its own.
pub trait Product: FigureRenderer + ModelReviewer {}

impl<T: FigureRenderer + ModelReviewer + ?Sized> Product for T {}

impl ModelReviewer for SceCodegen {
    fn review(&self, request: &ReviewRequest<'_>) -> Result<Review, RenderError> {
        self.read_model(request)
    }
}

impl ModelReviewer for NoRenderer {
    fn review(&self, _: &ReviewRequest<'_>) -> Result<Review, RenderError> {
        Err(NoRenderer::unavailable())
    }
}

impl SceCodegen {
    /// Read a model with the product: check it, and write its page if it is
    /// accepted. See the module's note on what a refusal is.
    fn read_model(&self, request: &ReviewRequest<'_>) -> Result<Review, RenderError> {
        let scratch = Scratch::new().map_err(|e| RenderError::Failed {
            reason: format!("no place to stage the model: {e}"),
        })?;
        let document = scratch.path().join(format!("{}.scxml", stem(request.name)));
        fs::write(&document, request.model).map_err(|e| RenderError::Failed {
            reason: format!("the model could not be staged: {e}"),
        })?;

        // ⚠ `--lint`, always: the design-time lints are off in the product only
        // because its conformance corpus declares unreachable states on purpose,
        // and a person reading a model is an author with no such excuse.
        let mut command = self.command();
        command
            .args(["--error-format", "json", "check"])
            .arg(&document)
            .arg("--lint");
        let run = run_bounded(command, scratch.path(), self.timeout())?;
        let records = records_of(&run.stderr);
        if !run.success {
            if records.is_empty() {
                // A refusal with no record is the product's defect, said as it was.
                return Err(RenderError::Failed {
                    reason: format!("exit status {:?}: {}", run.code, excerpt(&run.stderr)),
                });
            }
            return Ok(Review {
                generator: self.version().map(str::to_string),
                check: Check {
                    verdict: Verdict::Refused,
                    kind: None,
                    open: Vec::new(),
                    unresolved: Vec::new(),
                    records,
                },
                page: None,
                page_refusal: None,
            });
        }
        let manifest = manifest_of(&run.stdout)?;
        let check = Check {
            verdict: Verdict::Accepted,
            kind: manifest
                .pointer("/document_kind/name")
                .and_then(Value::as_str)
                .map(str::to_string),
            open: list_of(&manifest, "open")
                .filter_map(|m| m.get("message").and_then(Value::as_str).map(str::to_string))
                .collect(),
            unresolved: list_of(&manifest, "unresolved")
                .filter_map(|u| {
                    Some(Unresolved {
                        id: u.get("id")?.as_str()?.to_string(),
                        node_path: u.get("node_path")?.as_str()?.to_string(),
                        line: u.pointer("/location/line").and_then(Value::as_u64),
                    })
                })
                .collect(),
            records,
        };

        let mut command = self.command();
        command
            .args(["--error-format", "json", "pseudo"])
            .arg(&document);
        if let Some(lexicon) = request.lexicon {
            command.arg("--lexicon").arg(word("--lexicon", lexicon)?);
        }
        let run = run_bounded(command, scratch.path(), self.timeout())?;
        let (page, page_refusal) = if run.success {
            // Byte for byte: the page's contract is that a value reaches it as its
            // author spelled it, and a reader that trims it is the first thing to
            // break that.
            (Some(run.stdout), None)
        } else {
            match refusal(&run.stderr, run.code) {
                RenderError::Refused { code, message } => {
                    (None, Some(PageRefusal { code, message }))
                }
                other => return Err(other),
            }
        };
        Ok(Review {
            generator: self.version().map(str::to_string),
            check,
            page,
            page_refusal,
        })
    }
}

/// The records on standard error: each line that is a JSON object with a `code`
/// and a `message`. A line that is anything else is not a record and is left out.
fn records_of(stderr: &str) -> Vec<Record> {
    stderr
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter_map(|record| {
            Some(Record {
                code: record.get("code")?.as_str()?.to_string(),
                message: record.get("message")?.as_str()?.to_string(),
                stage: record
                    .get("stage")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                line: record.pointer("/location/line").and_then(Value::as_u64),
                fix: record.get("fix").cloned(),
            })
        })
        .collect()
}

/// The manifest an accepted check wrote: the one JSON object on its standard output.
fn manifest_of(stdout: &str) -> Result<Value, RenderError> {
    stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .find_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(Value::is_object)
        .ok_or_else(|| RenderError::Failed {
            reason: format!(
                "the check accepted the model and wrote no manifest: {}",
                excerpt(stdout)
            ),
        })
}

/// The items of the manifest's list `key`; none when it is absent.
fn list_of<'a>(manifest: &'a Value, key: &str) -> impl Iterator<Item = &'a Value> {
    manifest
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_record_is_a_line_with_a_code_and_a_message_and_nothing_else_is() {
        let stderr = concat!(
            "{\"v\":1,\"code\":\"validation/invalid-reference\",\"stage\":\"validation\",",
            "\"message\":\"no such state\",\"location\":{\"file\":\"x\",\"line\":3,\"col\":22},",
            "\"fix\":{\"kind\":\"replace_one_of\",\"candidates\":[\"a\"]}}\n",
            "warning: something that is prose\n",
            "{\"v\":1,\"message\":\"no code\"}\n",
            "[1,2]\n"
        );
        let records = records_of(stderr);
        assert_eq!(records.len(), 1, "{records:?}");
        assert_eq!(records[0].code, "validation/invalid-reference");
        assert_eq!(records[0].line, Some(3));
        assert_eq!(records[0].stage.as_deref(), Some("validation"));
        assert!(records[0].fix.is_some());
    }

    #[test]
    fn a_manifest_is_the_object_on_standard_output() {
        let manifest = manifest_of("\n{\"v\":1,\"kind\":\"check\"}\n").unwrap();
        assert_eq!(manifest["kind"], "check");
        for bad in ["", "not json", "[1]"] {
            assert!(
                matches!(manifest_of(bad), Err(RenderError::Failed { .. })),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn a_missing_list_is_none_and_a_wrong_shape_is_skipped_not_guessed() {
        let manifest = serde_json::json!({
            "unresolved": [{"id": "a", "node_path": "p"}, {"id": 3}, "x"]
        });
        assert_eq!(list_of(&manifest, "open").count(), 0);
        assert_eq!(list_of(&manifest, "unresolved").count(), 3);
    }
}
