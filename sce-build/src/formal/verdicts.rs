// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The judge of one machine's properties (docs/adr/0115, D15): every guarantee
//! of a properties document judged by TLC over the machine's model and the
//! exploration, and the verdict of each, which never says more than was
//! judged.
//!
//! | Verdict | Said when |
//! |---|---|
//! | *holds of the whole space* | every reachable state was explored and no bound was reached |
//! | *holds within* | explored, and these bounds were reached |
//! | *violated* | a run breaks it, under the assumptions |
//! | *vacuous* | it holds though a part of it is made stronger: it passes for nothing |
//! | *not judged* | the search did not answer, or a bound cut what it would decide |
//!
//! Every machine is also held to one property no document states: every
//! macrostep ends, so there is always a next observation. A machine whose
//! macrostep does not end is violated under every properties document.
//!
//! A judgement is of exactly one machine, one properties document with its
//! observers, one exploration and one judge, by their digests ([`Digests`]),
//! and is recorded with them (`schemas/sce-formal-verdicts.v1.schema.json`).
//! A recorded judgement of other digests is none: [`read_record`] refuses it,
//! naming each digest that moved.

use std::num::NonZeroUsize;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::document_model::{SEMANTICS, SEMANTICS_TEXT};
use super::models::Report;
use super::properties::{
    Check, Documents, Judged, PropertiesModule, Spec, MACROSTEPS_END, WITHIN_MOST,
};
use super::{tlc, toolchain, FormalError, WorkDir};

/// The surface's stability (`SCE_WIRE_CONTRACTS.md`), which the header of
/// `schemas/sce-formal-verdicts.v1.schema.json` repeats.
pub const VERDICTS_SCHEMA_STATUS: &str = "pre-release";

/// What a verdicts record names itself.
pub const RECORD: &str = "sce-formal-verdicts";

/// The version of the record's format this judge writes and reads.
pub const VERSION: u32 = 1;

/// The digests a judgement is of: the lowercase hex sha256 of the machine's
/// document, of the properties document with each observer's, of the
/// exploration, and of the judge (the pinned TLC and Java runtime, this
/// generator's sources and the semantics it writes models against).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Digests {
    pub machine: String,
    pub properties: String,
    pub exploration: String,
    pub tool: String,
}

impl Digests {
    /// The digests of `documents`, judged by this judge.
    pub fn of(documents: &Documents) -> Self {
        let observers: Vec<(&str, &str)> = documents
            .observers
            .iter()
            .map(|observer| (observer.id.as_str(), observer.document.text.as_str()))
            .collect();
        Self {
            machine: digest(&[documents.machine.text.as_bytes()]),
            properties: properties_digest(&documents.properties.text, &observers),
            exploration: digest(&[documents.exploration.text.as_bytes()]),
            tool: tool_digest(),
        }
    }
}

/// The sha256 of `parts`, each led by its length, so that two sequences of
/// parts with the same bytes in other places differ.
fn digest(parts: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part);
    }
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The digest of a properties document and its observers, each by the name a
/// condition reads it by: the observers are part of what the properties say.
pub fn properties_digest(text: &str, observers: &[(&str, &str)]) -> String {
    let mut parts: Vec<&[u8]> = vec![text.as_bytes()];
    for (id, observer) in observers {
        parts.push(id.as_bytes());
        parts.push(observer.as_bytes());
    }
    digest(&parts)
}

/// The digest of this judge: the pinned TLC and the Java runtime it runs on,
/// the sources of this generator, which writes the models, and the semantics
/// they extend.
pub fn tool_digest() -> String {
    let pins = toolchain::pinned();
    digest(&[
        b"tlc",
        pins.tlc.sha256.as_bytes(),
        b"java",
        pins.java.sha256.as_bytes(),
        b"sce-codegen",
        crate::GENERATOR_SOURCE_DIGEST.as_bytes(),
        SEMANTICS.as_bytes(),
        SEMANTICS_TEXT.as_bytes(),
    ])
}

/// What was said of one property, or of every macrostep ending.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "kebab-case")]
pub enum Verdict {
    /// Every reachable state was explored and no bound was reached.
    HoldsOfTheWholeSpace,
    /// Explored, and these bounds of the exploration were reached.
    HoldsWithin { bounds: Vec<String> },
    /// A run breaks it, under the assumptions: TLC's account of the run.
    Violated { run: String },
    /// It holds though `part` is replaced by `by`, which makes it stronger.
    Vacuous { part: String, by: bool },
    /// The search did not answer, or a bound cut what it would decide.
    NotJudged { why: String },
}

impl Verdict {
    /// Whether it says the property holds, within its bounds or not.
    pub fn holds(&self) -> bool {
        matches!(
            self,
            Verdict::HoldsOfTheWholeSpace | Verdict::HoldsWithin { .. }
        )
    }
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Verdict::HoldsOfTheWholeSpace => f.write_str("holds of the whole space"),
            Verdict::HoldsWithin { bounds } => {
                write!(f, "holds within: {} reached", bounds.join("; "))
            }
            Verdict::Violated { .. } => f.write_str("violated"),
            Verdict::Vacuous { part, by } => write!(
                f,
                "vacuous: it holds though `{part}` is replaced by {by}, so it passes for \
                 nothing in that part"
            ),
            Verdict::NotJudged { why } => write!(f, "not judged: {why}"),
        }
    }
}

/// The verdict of one guarantee.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyVerdict {
    /// The `sce:req` token the guarantee states.
    pub req: String,
    /// The form it is written in.
    pub form: String,
    #[serde(flatten)]
    pub verdict: Verdict,
}

impl std::fmt::Display for PropertyVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (<sce:{}>): {}", self.req, self.form, self.verdict)
    }
}

/// A judgement of one machine's properties, as it is recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Judgement {
    /// [`RECORD`].
    pub record: String,
    /// [`VERSION`].
    pub v: u32,
    pub digests: Digests,
    /// Whether every macrostep ends.
    pub macrosteps: Verdict,
    /// The assumptions, by the `sce:req` token each states, that no
    /// machine of SCE discharges in this judgement.
    pub not_discharged: Vec<String>,
    /// Each guarantee's verdict, in document order.
    pub verdicts: Vec<PropertyVerdict>,
}

impl Judgement {
    /// The record's text: the judgement as pretty JSON, one newline at the end.
    pub fn to_text(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).expect("a judgement serialises");
        text.push('\n');
        text
    }

    /// Why the judgement is not that every property holds, as the diagnostic
    /// the judge leaves through, or `None` where each holds: a violation or a
    /// property not judged is `formal/verdict`, and, where there is none, a
    /// vacuous property is `formal/property-refused`. `document` names the
    /// properties document.
    pub fn failure(&self, document: &str) -> Option<FormalError> {
        let mut failures = Vec::new();
        if !self.macrosteps.holds() {
            failures.push(format!("every macrostep ends: {}", self.macrosteps));
        }
        for verdict in &self.verdicts {
            if matches!(
                verdict.verdict,
                Verdict::Violated { .. } | Verdict::NotJudged { .. }
            ) {
                failures.push(verdict.to_string());
            }
        }
        if !failures.is_empty() {
            return Some(FormalError::Verdict {
                manifest: document.to_string(),
                runs: self.verdicts.len() + 1,
                failures,
            });
        }
        let vacuous: Vec<String> = self
            .verdicts
            .iter()
            .filter(|verdict| matches!(verdict.verdict, Verdict::Vacuous { .. }))
            .map(|verdict| {
                format!(
                    "`{}` (<sce:{}>): {}",
                    verdict.req, verdict.form, verdict.verdict
                )
            })
            .collect();
        (!vacuous.is_empty()).then(|| FormalError::PropertyRefused {
            document: document.to_string(),
            refusals: vacuous,
        })
    }
}

/// The judgement recorded as `text`, when it is of the documents and the judge
/// `current` names; else every reason it is not read: a record of other
/// digests is no judgement of these.
pub fn read_record(text: &str, current: &Digests) -> Result<Judgement, Vec<String>> {
    let judgement: Judgement =
        serde_json::from_str(text).map_err(|e| vec![format!("is no record of verdicts: {e}")])?;
    let mut problems = Vec::new();
    if judgement.record != RECORD {
        problems.push(format!(
            "names itself {:?}, and a record of verdicts is {RECORD:?}",
            judgement.record
        ));
    }
    if judgement.v != VERSION {
        problems.push(format!(
            "is of version {}, and this judge reads {VERSION}",
            judgement.v
        ));
    }
    let recorded = &judgement.digests;
    for (what, was, is) in [
        ("machine", &recorded.machine, &current.machine),
        ("properties", &recorded.properties, &current.properties),
        ("exploration", &recorded.exploration, &current.exploration),
        ("judge", &recorded.tool, &current.tool),
    ] {
        if was != is {
            problems.push(format!(
                "judged the {what} of digest {was}, and the {what} is now of digest {is}: a \
                 verdict of another digest is none"
            ));
        }
    }
    if problems.is_empty() {
        Ok(judgement)
    } else {
        Err(problems)
    }
}

/// Write what `judgement` says, one line for every macrostep ending, one for
/// the assumptions not discharged, and one per guarantee.
pub fn report(judgement: &Judgement, report: &mut Report<'_>) {
    (report.out)(&format!("every macrostep ends: {}", judgement.macrosteps));
    if !judgement.not_discharged.is_empty() {
        (report.out)(&format!(
            "assumed and not discharged: {}",
            judgement.not_discharged.join(", ")
        ));
    }
    for verdict in &judgement.verdicts {
        (report.out)(&verdict.to_string());
        if let Verdict::Violated { run } = &verdict.verdict {
            (report.detail)(run);
        }
    }
}

/// Judge every guarantee of `documents` with the pinned TLC, `workers` the
/// processors each run is given, and write what each is said to be.
pub fn judge(
    documents: &Documents,
    workers: NonZeroUsize,
    report_to: &mut Report<'_>,
) -> Result<Judgement, FormalError> {
    let written = super::properties::write(documents)?;
    let pins = toolchain::pinned();
    let jar = toolchain::admitted_tlc(&pins.tlc)?;
    let java_home = toolchain::admitted_java(&pins.java)?;
    let java = toolchain::java_program(&java_home);
    let tools = tlc::Tools {
        java: &java,
        jar: &jar,
        library: None,
    };
    let stage = WorkDir::new("properties")?;
    stage_modules(stage.path(), &written)?;
    let runner = Runner {
        tools,
        sources: stage.path(),
        written: &written,
        workers,
    };

    let bounds = if written.most > 0 {
        let answer = runner.run(Spec::Safe, Check::Invariant(WITHIN_MOST))?;
        match answer.status {
            tlc::HOLDS => Ok(Vec::new()),
            tlc::SAFETY => Ok(vec![format!(
                "the most events from outside ({})",
                written.most
            )]),
            status => Err(not_answered(
                "whether the most events were reached",
                status,
                &answer.output,
            )),
        }
    } else {
        Ok(Vec::new())
    };
    let holding = match &bounds {
        Ok(reached) if reached.is_empty() => Verdict::HoldsOfTheWholeSpace,
        Ok(reached) => Verdict::HoldsWithin {
            bounds: reached.clone(),
        },
        Err(why) => Verdict::NotJudged { why: why.clone() },
    };

    let macrosteps = classify(
        &runner.run(Spec::Live, Check::Property(MACROSTEPS_END))?,
        tlc::LIVENESS,
        &holding,
    );
    (report_to.out)(&format!("every macrostep ends: {macrosteps}"));
    if !written.assumptions.is_empty() {
        (report_to.out)(&format!(
            "assumed and not discharged: {}",
            written.assumptions.join(", ")
        ));
    }

    let mut verdicts = Vec::with_capacity(written.guarantees.len());
    for guarantee in &written.guarantees {
        let mut verdict = runner.verdict(guarantee.judged, &guarantee.operator, &holding)?;
        if verdict.holds() {
            for stronger in &guarantee.strengthened {
                if runner
                    .verdict(guarantee.judged, &stronger.operator, &holding)?
                    .holds()
                {
                    verdict = Verdict::Vacuous {
                        part: stronger.part.clone(),
                        by: stronger.by,
                    };
                    break;
                }
            }
        }
        let judged = PropertyVerdict {
            req: guarantee.req.clone(),
            form: guarantee.form.to_string(),
            verdict,
        };
        (report_to.out)(&judged.to_string());
        if let Verdict::Violated { run } = &judged.verdict {
            (report_to.detail)(run);
        }
        verdicts.push(judged);
    }

    Ok(Judgement {
        record: RECORD.to_string(),
        v: VERSION,
        digests: Digests::of(documents),
        macrosteps,
        not_discharged: written.assumptions.clone(),
        verdicts,
    })
}

/// Write the semantics, the machine's model and the properties module into
/// `dir`, where every run reads them.
fn stage_modules(dir: &Path, written: &PropertiesModule) -> Result<(), FormalError> {
    for (module, text) in [
        (SEMANTICS, SEMANTICS_TEXT),
        (
            written.machine.module.as_str(),
            written.machine.text.as_str(),
        ),
        (written.module.as_str(), written.text.as_str()),
    ] {
        let path = dir.join(format!("{module}.tla"));
        std::fs::write(&path, text).map_err(|e| FormalError::ToolUnavailable {
            tool: "a working directory".to_string(),
            detail: format!("cannot write {}: {e}", path.display()),
        })?;
    }
    Ok(())
}

/// The runs of one judgement, over the modules staged once.
struct Runner<'a> {
    tools: tlc::Tools<'a>,
    sources: &'a Path,
    written: &'a PropertiesModule,
    workers: NonZeroUsize,
}

impl Runner<'_> {
    /// One run of `spec` checking `check`.
    fn run(&self, spec: Spec, check: Check<'_>) -> Result<tlc::Answer, FormalError> {
        let config_name = format!("{}.cfg", self.written.module);
        tlc::run(&tlc::Run {
            tools: self.tools,
            sources: self.sources,
            module: &self.written.module,
            config_name: &config_name,
            config_text: &self.written.config(spec, check),
            extra: &[],
            workers: self.workers,
            write_graph: false,
        })
    }

    /// What the run that judges `operator` as `judged` says of it;
    /// `holding` is what a run that holds is said to show.
    fn verdict(
        &self,
        judged: Judged,
        operator: &str,
        holding: &Verdict,
    ) -> Result<Verdict, FormalError> {
        Ok(match judged {
            Judged::Invariant => classify(
                &self.run(Spec::Safe, Check::Invariant(operator))?,
                tlc::SAFETY,
                holding,
            ),
            Judged::Temporal => classify(
                &self.run(Spec::Live, Check::Property(operator))?,
                tlc::LIVENESS,
                holding,
            ),
            Judged::Possible => {
                let (name, text) = self.written.possible_module(operator);
                let possible = super::models::possible_over(
                    self.tools,
                    self.sources,
                    (name.as_str(), text.as_str()),
                    &self.written.config(Spec::Possible, Check::Nothing),
                    self.workers,
                )?;
                possible_verdict(possible, holding)
            }
        })
    }
}

/// What a run of TLC says of the property it checked: `holding` where it
/// holds, a violation where it ended with `violated` (TLC's status for the
/// kind of property checked), and nothing judged otherwise.
fn classify(answer: &tlc::Answer, violated: i32, holding: &Verdict) -> Verdict {
    match answer.status {
        tlc::HOLDS => holding.clone(),
        status if status == violated => Verdict::Violated {
            run: account(&answer.output),
        },
        status => Verdict::NotJudged {
            why: not_answered("the property", status, &answer.output),
        },
    }
}

/// What `AG EF` decided says of a `<sce:possible>`. Where it fails and a bound
/// of the exploration was reached, the bound may be what cut the way to a
/// state where the condition holds, so it is not judged: a violation is said
/// only of a space explored whole.
fn possible_verdict(possible: Result<bool, String>, holding: &Verdict) -> Verdict {
    match possible {
        Ok(true) => holding.clone(),
        Ok(false) => match holding {
            Verdict::HoldsOfTheWholeSpace => Verdict::Violated {
                run: "from a reachable state, no state where it holds can be reached".to_string(),
            },
            Verdict::HoldsWithin { bounds } => Verdict::NotJudged {
                why: format!(
                    "from a reachable state no state where it holds can be reached within {}, \
                     which may be what cut the way",
                    bounds.join("; ")
                ),
            },
            other => other.clone(),
        },
        Err(why) => Verdict::NotJudged { why },
    }
}

/// TLC's account of a violation: its report from the first error on, the
/// behaviour that shows it, cut to its last 8000 characters.
fn account(output: &str) -> String {
    let from = output.find("Error:").unwrap_or(0);
    let shown = &output[from..];
    let cut = shown.len().saturating_sub(8000);
    let start = (cut..=shown.len())
        .find(|at| shown.is_char_boundary(*at))
        .unwrap_or(shown.len());
    shown[start..].to_string()
}

/// Why a run judged nothing: what it was to decide, TLC's status, and the end
/// of what it printed.
fn not_answered(what: &str, status: i32, output: &str) -> String {
    let lines: Vec<&str> = output.lines().collect();
    let tail = lines[lines.len().saturating_sub(12)..].join("\n");
    format!("TLC answered {status} where it was to decide {what}:\n{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema() -> serde_json::Value {
        serde_json::from_str(include_str!(
            "../../../schemas/sce-formal-verdicts.v1.schema.json"
        ))
        .expect("the verdicts schema is JSON")
    }

    fn digests() -> Digests {
        Digests {
            machine: "a".repeat(64),
            properties: "b".repeat(64),
            exploration: "c".repeat(64),
            tool: "d".repeat(64),
        }
    }

    fn judgement(verdicts: Vec<(&str, &str, Verdict)>) -> Judgement {
        Judgement {
            record: RECORD.to_string(),
            v: VERSION,
            digests: digests(),
            macrosteps: Verdict::HoldsWithin {
                bounds: vec!["the most events from outside (4)".to_string()],
            },
            not_discharged: vec!["ENV-1".to_string()],
            verdicts: verdicts
                .into_iter()
                .map(|(req, form, verdict)| PropertyVerdict {
                    req: req.to_string(),
                    form: form.to_string(),
                    verdict,
                })
                .collect(),
        }
    }

    fn every_verdict() -> Judgement {
        judgement(vec![
            ("R1", "always", Verdict::HoldsOfTheWholeSpace),
            (
                "R2",
                "leads-to",
                Verdict::HoldsWithin {
                    bounds: vec!["the most events from outside (4)".to_string()],
                },
            ),
            (
                "R3",
                "step",
                Verdict::Violated {
                    run: "Error: Invariant Guarantee3 is violated.".to_string(),
                },
            ),
            (
                "R4",
                "recurs",
                Verdict::Vacuous {
                    part: "In('a')".to_string(),
                    by: false,
                },
            ),
            (
                "R5",
                "possible",
                Verdict::NotJudged {
                    why: "TLC answered 75".to_string(),
                },
            ),
        ])
    }

    #[test]
    fn schema_file_declares_status() {
        assert_eq!(
            schema()["x-sce-schema-status"].as_str(),
            Some(VERDICTS_SCHEMA_STATUS),
            "schemas/sce-formal-verdicts.v1.schema.json x-sce-schema-status disagrees with \
             VERDICTS_SCHEMA_STATUS; SCE_WIRE_CONTRACTS.md requires one commit to move both",
        );
    }

    /// Turns red if the record leaves its schema: a field renamed, a verdict
    /// spelled otherwise, or the version moved on one side alone.
    #[test]
    fn a_record_validates_against_the_wire_schema() {
        let schema = schema();
        let validator = jsonschema::JSONSchema::options()
            .with_draft(jsonschema::Draft::Draft7)
            .compile(&schema)
            .expect("the verdicts schema compiles");
        let instance: serde_json::Value =
            serde_json::from_str(&every_verdict().to_text()).expect("the record is JSON");
        let found: Vec<String> = match validator.validate(&instance) {
            Ok(()) => Vec::new(),
            Err(errors) => errors.map(|e| e.to_string()).collect(),
        };
        assert!(found.is_empty(), "{found:?}");
        let mut other = instance.clone();
        other["verdicts"][0]["verdict"] = serde_json::json!("proved");
        assert!(
            validator.validate(&other).is_err(),
            "a verdict the judge does not say must be refused"
        );
        assert_eq!(
            schema["properties"]["v"]["const"].as_u64(),
            Some(u64::from(VERSION))
        );
    }

    /// Turns red if a recorded judgement is read for documents or a judge it
    /// was not made of: a verdict of another digest is none.
    #[test]
    fn a_verdict_of_another_digest_is_not_read() {
        let recorded = every_verdict();
        let text = recorded.to_text();
        assert_eq!(read_record(&text, &digests()), Ok(recorded));
        for (moved, word) in [
            (
                Digests {
                    machine: "e".repeat(64),
                    ..digests()
                },
                "the machine",
            ),
            (
                Digests {
                    properties: "e".repeat(64),
                    ..digests()
                },
                "the properties",
            ),
            (
                Digests {
                    exploration: "e".repeat(64),
                    ..digests()
                },
                "the exploration",
            ),
            (
                Digests {
                    tool: "e".repeat(64),
                    ..digests()
                },
                "the judge",
            ),
        ] {
            let refused = read_record(&text, &moved).expect_err("a record of other digests");
            assert_eq!(refused.len(), 1, "{refused:?}");
            assert!(
                refused[0].starts_with(&format!("judged {word} of digest "))
                    && refused[0].ends_with("a verdict of another digest is none"),
                "{refused:?}"
            );
        }
        assert!(read_record("{}", &digests()).is_err());
    }

    /// Turns red if the observers stop being part of what a properties
    /// document is judged as, or two documents' parts may be shifted into one
    /// digest.
    #[test]
    fn the_properties_digest_covers_each_observer_by_its_name() {
        let alone = properties_digest("p", &[]);
        let observed = properties_digest("p", &[("since", "s")]);
        let renamed = properties_digest("p", &[("after", "s")]);
        let edited = properties_digest("p", &[("since", "t")]);
        let shifted = properties_digest("ps", &[("ince", "s")]);
        let all = [&alone, &observed, &renamed, &edited, &shifted];
        for (at, one) in all.iter().enumerate() {
            assert_eq!(one.len(), 64);
            for other in &all[at + 1..] {
                assert_ne!(one, other);
            }
        }
    }

    /// Turns red if a violation or a property not judged is let through, a
    /// vacuous property is told as holding, or one kind of failure hides the
    /// other's names.
    #[test]
    fn a_failure_names_each_violation_and_each_vacuous_property() {
        match every_verdict().failure("p.scxml") {
            Some(FormalError::Verdict {
                manifest,
                runs,
                failures,
            }) => {
                assert_eq!(manifest, "p.scxml");
                assert_eq!(runs, 6);
                assert_eq!(
                    failures,
                    [
                        "R3 (<sce:step>): violated",
                        "R5 (<sce:possible>): not judged: TLC answered 75"
                    ]
                );
            }
            other => panic!("expected formal/verdict, got {other:?}"),
        }
        let vacuous = judgement(vec![
            ("R1", "always", Verdict::HoldsOfTheWholeSpace),
            (
                "R4",
                "recurs",
                Verdict::Vacuous {
                    part: "In('a')".to_string(),
                    by: false,
                },
            ),
        ]);
        match vacuous.failure("p.scxml") {
            Some(FormalError::PropertyRefused { refusals, .. }) => assert_eq!(
                refusals,
                ["`R4` (<sce:recurs>): vacuous: it holds though `In('a')` is replaced by false, so it passes for nothing in that part"]
            ),
            other => panic!("expected formal/property-refused, got {other:?}"),
        }
        let holding = judgement(vec![("R1", "always", Verdict::HoldsOfTheWholeSpace)]);
        assert!(holding.failure("p.scxml").is_none());
        let endless = Judgement {
            macrosteps: Verdict::Violated {
                run: "Error: Temporal properties were violated.".to_string(),
            },
            ..holding
        };
        assert!(matches!(
            endless.failure("p.scxml"),
            Some(FormalError::Verdict { failures, .. })
                if failures == ["every macrostep ends: violated"]
        ));
    }

    /// Turns red if a run's status is read as another verdict than TLC's own:
    /// a violation of the kind checked, a hold, or no answer.
    #[test]
    fn a_run_is_read_by_its_status() {
        let answer = |status: i32, output: &str| tlc::Answer {
            status,
            output: output.to_string(),
            graph: None,
        };
        let holding = Verdict::HoldsWithin {
            bounds: vec!["the most events from outside (2)".to_string()],
        };
        assert_eq!(
            classify(&answer(tlc::HOLDS, ""), tlc::SAFETY, &holding),
            holding
        );
        assert_eq!(
            classify(
                &answer(
                    tlc::SAFETY,
                    "Model checking\nError: Invariant G is violated.\nState 1"
                ),
                tlc::SAFETY,
                &holding
            ),
            Verdict::Violated {
                run: "Error: Invariant G is violated.\nState 1".to_string()
            }
        );
        assert!(matches!(
            classify(&answer(tlc::SAFETY, "Error"), tlc::LIVENESS, &holding),
            Verdict::NotJudged { why } if why.starts_with("TLC answered 12 where it was to decide")
        ));
    }

    /// Turns red if a possibility that fails where a bound was reached is told
    /// as violated, or one that fails over a space explored whole is not.
    #[test]
    fn a_possibility_is_violated_only_over_a_space_explored_whole() {
        let within = Verdict::HoldsWithin {
            bounds: vec!["the most events from outside (2)".to_string()],
        };
        assert_eq!(possible_verdict(Ok(true), &within), within);
        assert!(matches!(
            possible_verdict(Ok(false), &within),
            Verdict::NotJudged { .. }
        ));
        assert!(matches!(
            possible_verdict(Ok(false), &Verdict::HoldsOfTheWholeSpace),
            Verdict::Violated { .. }
        ));
        assert_eq!(
            possible_verdict(Err("no graph".to_string()), &within),
            Verdict::NotJudged {
                why: "no graph".to_string()
            }
        );
    }
}
