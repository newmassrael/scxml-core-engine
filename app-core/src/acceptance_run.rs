// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Asking the product about a work's requirements and acceptance.
//!
//! Four things are the product's to say, and this module only runs them and passes on
//! what they write:
//!
//! - `requirements --manifest`: for each requirement the list names, whether the design
//!   carries it (`implemented`), does not (`missing`), or can only be tested
//!   (`needs-scenario`);
//! - `acceptance-report`: the page the owner reads before accepting, in which each
//!   requirement is shown with the words it quotes and the places that cite it;
//! - `accept`: the record that pins what the owner accepted;
//! - `acceptance-check`: whether a record still holds against the work as it is now,
//!   and what moved when it does not.
//!
//! ⚠ "Missing" and "open" are statements the product makes about the design, and the
//! application does not weigh them. Accepting a design that misses a requirement or
//! leaves a question open is the owner's decision, which the record states and does not
//! forbid; what the application owes is that the owner is told before they decide.

use std::fs;

use serde::Serialize;
use serde_json::Value;

use crate::acceptance::{Snapshot, VARIANT};
use crate::figures::{excerpt, refusal, run_bounded, NoRenderer, RenderError, SceCodegen, Scratch};
use crate::review::PageRefusal;

/// The record's code for a lapse, and the sentence the product opens its list with.
const LAPSED: &str = "cli/acceptance-lapsed";
const LAPSE_LEAD: &str = "the acceptance no longer holds: ";

/// One requirement and how the design stands to it, as the product classifies it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RequirementOutcome {
    pub id: String,
    /// The product's word: `implemented`, `missing`, `needs-scenario`, ...
    pub outcome: String,
    /// The sentence of the text the requirement is anchored in.
    pub section: Option<String>,
    /// Where in the design it is carried, in the product's path syntax.
    pub node_paths: Vec<String>,
}

/// What the product says of a design against its requirement list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RequirementsReport {
    pub generator: Option<String>,
    /// What the list is a denominator OF (`synthesized`: a reading of the text, not
    /// the text's own numbering), as the product states it.
    pub denominator: Option<String>,
    pub outcomes: Vec<RequirementOutcome>,
    /// The acceptance report page, byte for byte. `None` when the product would not
    /// write it (`page_refusal` says why).
    pub page: Option<String>,
    pub page_refusal: Option<PageRefusal>,
}

/// The record the product wrote, and what the design left open when it was taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Taken {
    pub generator: Option<String>,
    /// The product's record, byte for byte.
    pub record: String,
    /// The questions and values the design left to a person (the record's
    /// `open_at_acceptance`), in the product's words.
    pub open: Vec<String>,
}

/// Whether an acceptance still holds, and what the product says moved when it does not.
///
/// ⚠ The product writes a lapse as ONE sentence and not as a list: its lapses are joined
/// by `; ` and a single lapse can contain `; ` itself (a source that changed is reported
/// together with the record not saying which source it was authored from). Splitting the
/// sentence would invent a list the product does not state, so it is passed on whole.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "standing", rename_all = "kebab-case")]
pub enum CheckOutcome {
    Holds,
    Lapsed {
        /// What moved and where (`design/door.scxml: ...`), in the product's words.
        says: String,
    },
}

/// Something that can answer for a work's requirements and acceptance.
pub trait Acceptor: Send + Sync {
    fn report_requirements(&self, snapshot: &Snapshot) -> Result<RequirementsReport, RenderError>;

    fn take_acceptance(&self, snapshot: &Snapshot) -> Result<Taken, RenderError>;

    /// Whether `record` (an earlier acceptance's) holds for the work as `snapshot`
    /// puts it now.
    fn check_acceptance(
        &self,
        snapshot: &Snapshot,
        record: &str,
    ) -> Result<CheckOutcome, RenderError>;
}

impl Acceptor for NoRenderer {
    fn report_requirements(&self, _: &Snapshot) -> Result<RequirementsReport, RenderError> {
        Err(NoRenderer::unavailable())
    }

    fn take_acceptance(&self, _: &Snapshot) -> Result<Taken, RenderError> {
        Err(NoRenderer::unavailable())
    }

    fn check_acceptance(&self, _: &Snapshot, _: &str) -> Result<CheckOutcome, RenderError> {
        Err(NoRenderer::unavailable())
    }
}

impl Acceptor for SceCodegen {
    fn report_requirements(&self, snapshot: &Snapshot) -> Result<RequirementsReport, RenderError> {
        let scratch = stage_scratch()?;
        let staged = snapshot
            .stage(&scratch.path().join("work"))
            .map_err(staging_failed)?;

        let mut command = self.command();
        command
            .args(["--error-format", "json", "requirements"])
            .arg(&staged.entry)
            .arg("--manifest")
            .arg(&staged.manifest);
        let run = run_bounded(command, scratch.path(), self.timeout())?;
        if !run.success {
            return Err(refusal(&run.stderr, run.code));
        }
        let mut outcomes = Vec::new();
        let mut denominator = None;
        for line in run.stdout.lines().filter(|l| !l.trim().is_empty()) {
            let Ok(record) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            match record.get("kind").and_then(Value::as_str) {
                Some("extraction") => {
                    denominator = record
                        .get("denominator")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                }
                Some("requirement") => {
                    let (Some(id), Some(outcome)) = (
                        record.get("id").and_then(Value::as_str),
                        record.get("outcome").and_then(Value::as_str),
                    ) else {
                        continue;
                    };
                    outcomes.push(RequirementOutcome {
                        id: id.to_string(),
                        outcome: outcome.to_string(),
                        section: record
                            .get("section")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                        node_paths: record
                            .get("node_paths")
                            .and_then(Value::as_array)
                            .map(|paths| {
                                paths
                                    .iter()
                                    .filter_map(|p| p.as_str().map(str::to_string))
                                    .collect()
                            })
                            .unwrap_or_default(),
                    });
                }
                _ => {}
            }
        }
        if outcomes.is_empty() {
            return Err(RenderError::Failed {
                reason: format!(
                    "the product measured the design and named no requirement: {}",
                    excerpt(&run.stdout)
                ),
            });
        }

        // The page the owner reads before accepting. A document the product will not
        // write a page of is its own refusal beside the outcomes it did give.
        let mut command = self.command();
        command
            .args(["--error-format", "json", "acceptance-report"])
            .arg(&staged.entry)
            .arg("--manifest")
            .arg(&staged.manifest)
            .args(["--variant", VARIANT]);
        if let Some(sidecar) = &staged.sidecar {
            command.arg("--sidecar").arg(sidecar);
        }
        let run = run_bounded(command, scratch.path(), self.timeout())?;
        let (page, page_refusal) = if run.success {
            (Some(run.stdout), None)
        } else {
            match refusal(&run.stderr, run.code) {
                RenderError::Refused { code, message } => {
                    (None, Some(PageRefusal { code, message }))
                }
                other => return Err(other),
            }
        };
        Ok(RequirementsReport {
            generator: self.version().map(str::to_string),
            denominator,
            outcomes,
            page,
            page_refusal,
        })
    }

    fn take_acceptance(&self, snapshot: &Snapshot) -> Result<Taken, RenderError> {
        let scratch = stage_scratch()?;
        let root = scratch.path().join("work");
        let staged = snapshot.stage(&root).map_err(staging_failed)?;
        let out = scratch.path().join("acceptance.json");

        let mut command = self.command();
        command
            .args(["--error-format", "json", "accept"])
            .arg(&staged.entry)
            .arg("--manifest")
            .arg(&staged.manifest)
            .args(["--variant", VARIANT])
            .arg("--root")
            .arg(&staged.root)
            .arg("--out")
            .arg(&out)
            .arg("--source")
            .arg(&staged.source);
        if let Some(answers) = &staged.answers {
            command.arg("--decisions").arg(answers);
        }
        command.args(["--channel", crate::acceptance::CHANNEL]);
        let run = run_bounded(command, scratch.path(), self.timeout())?;
        if !run.success {
            return Err(refusal(&run.stderr, run.code));
        }
        let record = fs::read_to_string(&out).map_err(|e| RenderError::Failed {
            reason: format!("the product accepted the design and left no record: {e}"),
        })?;
        let open = serde_json::from_str::<Value>(&record)
            .ok()
            .and_then(|wire| {
                wire.get("open_at_acceptance")
                    .and_then(Value::as_array)
                    .map(|matters| {
                        matters
                            .iter()
                            .filter_map(|m| m.get("message").and_then(Value::as_str))
                            .map(str::to_string)
                            .collect::<Vec<_>>()
                    })
            })
            .unwrap_or_default();
        Ok(Taken {
            generator: self.version().map(str::to_string),
            record,
            open,
        })
    }

    fn check_acceptance(
        &self,
        snapshot: &Snapshot,
        record: &str,
    ) -> Result<CheckOutcome, RenderError> {
        let scratch = stage_scratch()?;
        let staged = snapshot
            .stage(&scratch.path().join("work"))
            .map_err(staging_failed)?;
        let record_file = scratch.path().join("acceptance.json");
        fs::write(&record_file, record).map_err(staging_failed)?;

        let mut command = self.command();
        command
            .args(["--error-format", "json", "acceptance-check"])
            .arg(&record_file)
            .args(["--variant", VARIANT])
            .arg("--root")
            .arg(&staged.root)
            .arg("--source")
            .arg(&staged.source);
        if let Some(answers) = &staged.answers {
            command.arg("--decisions").arg(answers);
        }
        let run = run_bounded(command, scratch.path(), self.timeout())?;
        if run.success {
            return Ok(CheckOutcome::Holds);
        }
        // A lapse is an ANSWER, written as a record whose code says so; anything else
        // that stops the run is the product failing to answer.
        for line in run.stderr.lines() {
            let Ok(record) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            if record.get("code").and_then(Value::as_str) == Some(LAPSED) {
                return Ok(CheckOutcome::Lapsed {
                    says: lapse_of(&record),
                });
            }
        }
        Err(refusal(&run.stderr, run.code))
    }
}

/// The sentence of a lapse record. The product states it twice: in `actual`, on its own,
/// and in `message`, behind `<record path>: the acceptance no longer holds: ` where the
/// path is a scratch folder's, which a person has no use for. `actual` is the one the
/// product keys the record on, so it is read first.
fn lapse_of(record: &Value) -> String {
    let text = |key: &str| {
        record
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|t| !t.is_empty())
    };
    if let Some(actual) = text("actual") {
        return actual.to_string();
    }
    if let Some(message) = text("message") {
        return message
            .split_once(LAPSE_LEAD)
            .map_or(message, |(_, lapse)| lapse.trim())
            .to_string();
    }
    "the product says the acceptance no longer holds, and not what moved".to_string()
}

fn stage_scratch() -> Result<Scratch, RenderError> {
    Scratch::new().map_err(|e| RenderError::Failed {
        reason: format!("no place to stage the work: {e}"),
    })
}

fn staging_failed(error: std::io::Error) -> RenderError {
    RenderError::Failed {
        reason: format!("the work could not be staged: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lapse_is_the_products_one_sentence_and_is_not_cut_where_it_says_a_semicolon() {
        let sentence = "spec/source.txt: the specification this design was authored from changed \
                        since it was accepted; the design was authored from the specification \
                        spec/source.txt; the one asked about (spec/source.txt) differs";
        let record = serde_json::json!({
            "code": LAPSED,
            "message": format!(
                "/tmp/sce-figures-1/acceptance.json: the acceptance no longer holds: {sentence}"
            ),
            "actual": sentence,
        });
        assert_eq!(lapse_of(&record), sentence);

        // Without `actual`, the message less the scratch path that prefixes it says the same.
        let only_message = serde_json::json!({
            "code": LAPSED,
            "message": format!("/tmp/x/acceptance.json: the acceptance no longer holds: {sentence}"),
        });
        assert_eq!(lapse_of(&only_message), sentence);
    }

    #[test]
    fn a_lapse_that_says_nothing_is_still_a_lapse() {
        assert!(lapse_of(&serde_json::json!({"code": LAPSED})).contains("not what moved"));
        let bare = serde_json::json!({"code": LAPSED, "message": "elsewhere: it moved"});
        assert_eq!(lapse_of(&bare), "elsewhere: it moved");
    }
}
