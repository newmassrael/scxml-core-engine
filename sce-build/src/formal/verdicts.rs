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
//! A violation TLC finds is a run, and the judgement holds it as a scenario
//! every lane replays (D8's converse; [`super::behaviour`]): the run is read
//! from the behaviour TLC reports, and a violation whose behaviour cannot be
//! read as one is not judged, since no lane could show it is the code's. A
//! `<sce:possible>` that holds has a witness, the run TLC finds to a state
//! where its condition holds, held the same way.
//!
//! Asked for its coverage, the judge also alters the machine in each way of a
//! closed list ([`super::coverage`]) and judges every guarantee that held
//! again over each altered machine: a guarantee no alteration breaks says
//! nothing of what the machine does, and is refused by name; an alteration no
//! guarantee notices is listed.
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
use super::{behaviour, coverage, tlc, toolchain, FormalError, WorkDir};
use crate::run_trace::{Fired, RunTrace};

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
    /// A run breaks it, under the assumptions: TLC's account of the run, and
    /// the run as the scenario every lane replays. A violation with no run (a
    /// `<sce:possible>` lost from a reachable state) has no scenario.
    Violated {
        run: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        scenario: Option<Box<RunTrace>>,
    },
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
    /// For a `<sce:possible>` that holds, a run to a state where its
    /// condition holds, as the scenario every lane replays.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub witness: Option<Box<RunTrace>>,
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
    /// The digest of the sources of the generator that judged
    /// ([`crate::GENERATOR_SOURCE_DIGEST`]), or `unavailable` on a build
    /// that could not read them: a record read apart from the run that wrote
    /// it names its producer (`SCE_WIRE_CONTRACTS.md`, policy 1).
    pub generator: String,
    pub digests: Digests,
    /// Whether every macrostep ends.
    pub macrosteps: Verdict,
    /// The assumptions, by the `sce:req` token each states, that no
    /// machine of SCE discharges in this judgement.
    pub not_discharged: Vec<String>,
    /// Each guarantee's verdict, in document order.
    pub verdicts: Vec<PropertyVerdict>,
    /// The machine altered in each way of the closed list, and the guarantees
    /// that notice each alteration, where the judgement was asked for them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage: Option<Coverage>,
}

/// What altering the machine showed of its guarantees (docs/adr/0115, D15).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coverage {
    /// Each alteration, in the order of the closed list.
    pub alterations: Vec<AlterationVerdict>,
}

/// One alteration of the machine and what noticed it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlterationVerdict {
    /// The alteration, as [`coverage::Alteration::describe`] names it.
    pub alteration: String,
    /// The guarantees, by their `sce:req` tokens, that a run of the altered
    /// machine breaks.
    pub noticed_by: Vec<String>,
    /// Why the altered machine was not judged: it has no model, or a search
    /// did not answer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_judged: Option<String>,
}

impl Coverage {
    /// The guarantees among `held` no alteration breaks, in their order.
    pub fn unbroken<'a>(&self, held: impl IntoIterator<Item = &'a str>) -> Vec<&'a str> {
        held.into_iter()
            .filter(|req| {
                !self
                    .alterations
                    .iter()
                    .any(|altered| altered.noticed_by.iter().any(|by| by == req))
            })
            .collect()
    }

    /// The alterations no guarantee notices and that were judged.
    pub fn unnoticed(&self) -> impl Iterator<Item = &AlterationVerdict> {
        self.alterations
            .iter()
            .filter(|altered| altered.noticed_by.is_empty() && altered.not_judged.is_none())
    }
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
        let mut refused: Vec<String> = self
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
        if let Some(coverage) = &self.coverage {
            let held = self
                .verdicts
                .iter()
                .filter(|verdict| verdict.verdict.holds())
                .map(|verdict| verdict.req.as_str());
            for req in coverage.unbroken(held) {
                let form = self
                    .verdicts
                    .iter()
                    .find(|verdict| verdict.req == req)
                    .map_or("", |verdict| verdict.form.as_str());
                refused.push(format!("`{req}` (<sce:{form}>): {UNBROKEN}"));
            }
        }
        (!refused.is_empty()).then(|| FormalError::PropertyRefused {
            document: document.to_string(),
            refusals: refused,
        })
    }

    /// Each scenario the judgement holds, by the name of the file a lane
    /// reads it from: `macrosteps.scenario.json` for a macrostep that does
    /// not end, and `<req>.scenario.json` for a guarantee's violation or
    /// witness.
    pub fn scenarios(&self) -> Vec<(String, &RunTrace)> {
        let mut found = Vec::new();
        if let Verdict::Violated {
            scenario: Some(run),
            ..
        } = &self.macrosteps
        {
            found.push(("macrosteps.scenario.json".to_string(), &**run));
        }
        for verdict in &self.verdicts {
            let run = match (&verdict.verdict, &verdict.witness) {
                (
                    Verdict::Violated {
                        scenario: Some(run),
                        ..
                    },
                    _,
                ) => run,
                (_, Some(run)) => run,
                _ => continue,
            };
            found.push((format!("{}.scenario.json", verdict.req), &**run));
        }
        found
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
        if let Verdict::Violated { run, .. } = &verdict.verdict {
            (report.detail)(run);
        }
    }
    if let Some(coverage) = &judgement.coverage {
        report_coverage(coverage, report);
    }
}

/// Write what altering the machine showed: one line per alteration no
/// guarantee notices, and one per alteration not judged.
fn report_coverage(coverage: &Coverage, report: &mut Report<'_>) {
    (report.out)(&format!(
        "the machine altered in {} ways",
        coverage.alterations.len()
    ));
    for altered in coverage.unnoticed() {
        (report.out)(&format!("no guarantee notices: {}", altered.alteration));
    }
    for altered in &coverage.alterations {
        if let Some(why) = &altered.not_judged {
            (report.out)(&format!("not judged: {}: {why}", altered.alteration));
        }
    }
}

/// What a guarantee no alteration of the machine breaks is refused as.
pub const UNBROKEN: &str =
    "no alteration of the machine breaks it, so it says nothing of what the machine does";

/// What a judgement is asked for beside the verdicts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Asked {
    /// Alter the machine in each way of the closed list and judge the
    /// guarantees that held over each (docs/adr/0115, D15).
    pub coverage: bool,
}

/// Judge every guarantee of `documents` with the pinned TLC, `workers` the
/// processors each run is given, and write what each is said to be.
pub fn judge(
    documents: &Documents,
    workers: NonZeroUsize,
    asked: Asked,
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

    let holding = runner.holding()?;

    let ended = runner.run(Spec::Live, Check::Property(MACROSTEPS_END))?;
    let macrosteps = runner.read_scenario(classify(&ended, tlc::LIVENESS, &holding), &ended.output);
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
                    .judged(guarantee.judged, &stronger.operator, &holding)?
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
        let mut witness = None;
        if let (true, Some(unseen)) = (verdict.holds(), &guarantee.witness) {
            match runner.witness(unseen)? {
                Ok(run) => witness = Some(Box::new(run)),
                Err(why) => verdict = Verdict::NotJudged { why },
            }
        }
        let judged = PropertyVerdict {
            req: guarantee.req.clone(),
            form: guarantee.form.to_string(),
            verdict,
            witness,
        };
        (report_to.out)(&judged.to_string());
        if let Verdict::Violated { run, .. } = &judged.verdict {
            (report_to.detail)(run);
        }
        verdicts.push(judged);
    }

    let coverage = if asked.coverage {
        let coverage = judge_coverage(documents, &written, &verdicts, tools, workers)?;
        report_coverage(&coverage, report_to);
        Some(coverage)
    } else {
        None
    };

    Ok(Judgement {
        record: RECORD.to_string(),
        v: VERSION,
        generator: crate::GENERATOR_SOURCE_DIGEST.to_string(),
        digests: Digests::of(documents),
        macrosteps,
        not_discharged: written.assumptions.clone(),
        verdicts,
        coverage,
    })
}

/// Judge every guarantee in `verdicts` that held over each alteration of
/// the machine of `documents`, whose properties are `written`.
fn judge_coverage(
    documents: &Documents,
    written: &PropertiesModule,
    verdicts: &[PropertyVerdict],
    tools: tlc::Tools<'_>,
    workers: NonZeroUsize,
) -> Result<Coverage, FormalError> {
    let held: Vec<&super::properties::Guarantee> = written
        .guarantees
        .iter()
        .filter(|guarantee| {
            verdicts
                .iter()
                .any(|verdict| verdict.req == guarantee.req && verdict.verdict.holds())
        })
        .collect();
    let mut alterations = Vec::new();
    for alteration in coverage::alterations(&documents.machine.model) {
        let described = alteration.describe();
        let altered_documents = Documents {
            machine: super::properties::Read {
                path: documents.machine.path.clone(),
                text: documents.machine.text.clone(),
                model: alteration.apply(&documents.machine.model),
            },
            properties: documents.properties.clone(),
            observers: documents.observers.clone(),
            exploration: documents.exploration.clone(),
        };
        let altered = match super::properties::write(&altered_documents) {
            Ok(altered) => altered,
            Err(refused) => {
                alterations.push(AlterationVerdict {
                    alteration: described,
                    noticed_by: Vec::new(),
                    not_judged: Some(format!("the altered machine has no model: {refused}")),
                });
                continue;
            }
        };
        let stage = WorkDir::new("coverage")?;
        stage_modules(stage.path(), &altered)?;
        let runner = Runner {
            tools,
            sources: stage.path(),
            written: &altered,
            workers,
        };
        let holding = runner.holding()?;
        let mut noticed_by = Vec::new();
        let mut not_judged = None;
        for guarantee in &held {
            match runner.judged(guarantee.judged, &guarantee.operator, &holding)? {
                Verdict::Violated { .. } => noticed_by.push(guarantee.req.clone()),
                Verdict::NotJudged { why } => {
                    if not_judged.is_none() {
                        not_judged = Some(format!("`{}`: {why}", guarantee.req));
                    }
                }
                _ => {}
            }
        }
        alterations.push(AlterationVerdict {
            alteration: described,
            noticed_by,
            not_judged,
        });
    }
    Ok(Coverage { alterations })
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
    /// What a run that holds is said to show: the whole space, or within the
    /// bounds the search reached, or nothing where that could not be read.
    fn holding(&self) -> Result<Verdict, FormalError> {
        if self.written.most == 0 {
            return Ok(Verdict::HoldsOfTheWholeSpace);
        }
        let answer = self.run(Spec::Safe, Check::Invariant(WITHIN_MOST))?;
        Ok(match answer.status {
            tlc::HOLDS => Verdict::HoldsOfTheWholeSpace,
            tlc::SAFETY => Verdict::HoldsWithin {
                bounds: vec![format!(
                    "the most events from outside ({})",
                    self.written.most
                )],
            },
            status => Verdict::NotJudged {
                why: not_answered(
                    "whether the most events were reached",
                    status,
                    &answer.output,
                ),
            },
        })
    }

    /// `verdict` with the scenario of the run TLC gave in `output`
    /// ([`with_scenario`]).
    fn read_scenario(&self, verdict: Verdict, output: &str) -> Verdict {
        with_scenario(verdict, output, &self.written.rows)
    }

    /// The witness of a `<sce:possible>` that holds: the run TLC finds to
    /// break `unseen`, the invariant that its condition never holds; or why
    /// there is none.
    fn witness(&self, unseen: &str) -> Result<Result<RunTrace, String>, FormalError> {
        let answer = self.run(Spec::Safe, Check::Invariant(unseen))?;
        Ok(match answer.status {
            tlc::SAFETY => scenario_of(&answer.output, &self.written.rows).map_err(|why| {
                format!(
                    "the run TLC found to a state where it holds is not read as a scenario \
                     every lane replays: {why}"
                )
            }),
            tlc::HOLDS => Err(
                "it is possible from every reachable state, and no run from the start reaches \
                 a state where it holds, which cannot both be"
                    .to_string(),
            ),
            status => Err(not_answered(
                "a run to a state where it holds",
                status,
                &answer.output,
            )),
        })
    }

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

    /// What the run that judges `operator` as `judged` says of it, with the
    /// scenario of a violation; `holding` is what a run that holds is said
    /// to show.
    fn verdict(
        &self,
        judged: Judged,
        operator: &str,
        holding: &Verdict,
    ) -> Result<Verdict, FormalError> {
        self.judged_as(judged, operator, holding, true)
    }

    /// What the run that judges `operator` as `judged` says of it, a
    /// violation with TLC's account alone.
    fn judged(
        &self,
        judged: Judged,
        operator: &str,
        holding: &Verdict,
    ) -> Result<Verdict, FormalError> {
        self.judged_as(judged, operator, holding, false)
    }

    /// What the run that judges `operator` as `judged` says of it, with
    /// the scenario of a violation where `scenario` is asked.
    fn judged_as(
        &self,
        judged: Judged,
        operator: &str,
        holding: &Verdict,
        scenario: bool,
    ) -> Result<Verdict, FormalError> {
        Ok(match judged {
            Judged::Invariant | Judged::Temporal => {
                let (spec, check, violated) = if judged == Judged::Invariant {
                    (Spec::Safe, Check::Invariant(operator), tlc::SAFETY)
                } else {
                    (Spec::Live, Check::Property(operator), tlc::LIVENESS)
                };
                let answer = self.run(spec, check)?;
                let verdict = classify(&answer, violated, holding);
                if scenario {
                    self.read_scenario(verdict, &answer.output)
                } else {
                    verdict
                }
            }
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

/// `verdict` with the scenario of the run TLC gave in `output`, where it is
/// a violation: one whose behaviour cannot be read as a scenario is not
/// judged, since no lane could replay it.
fn with_scenario(verdict: Verdict, output: &str, rows: &[Option<Fired>]) -> Verdict {
    match verdict {
        Verdict::Violated {
            run,
            scenario: None,
        } => match scenario_of(output, rows) {
            Ok(scenario) => Verdict::Violated {
                run,
                scenario: Some(Box::new(scenario)),
            },
            Err(why) => Verdict::NotJudged {
                why: format!(
                    "TLC found a run that breaks it, which is not read as a scenario every \
                     lane replays: {why}"
                ),
            },
        },
        other => other,
    }
}

/// The scenario of the behaviour TLC reports in `output`, the machine's rows
/// numbered as `rows` names them.
fn scenario_of(output: &str, rows: &[Option<Fired>]) -> Result<RunTrace, String> {
    behaviour::read(output).and_then(|read| behaviour::scenario(&read, rows))
}

/// What a run of TLC says of the property it checked: `holding` where it
/// holds, a violation where it ended with `violated` (TLC's status for the
/// kind of property checked), and nothing judged otherwise.
fn classify(answer: &tlc::Answer, violated: i32, holding: &Verdict) -> Verdict {
    match answer.status {
        tlc::HOLDS => holding.clone(),
        status if status == violated => Verdict::Violated {
            run: account(&answer.output),
            scenario: None,
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
                scenario: None,
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
            generator: crate::GENERATOR_SOURCE_DIGEST.to_string(),
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
                    witness: None,
                })
                .collect(),
            coverage: None,
        }
    }

    /// A scenario of two steps: the start, and `go` from outside taken.
    fn scenario() -> Box<RunTrace> {
        Box::new(
            RunTrace::from_json(
                r#"{"version": 1, "lane": "model", "steps": [
                    {"kind": "start", "entered": ["a"], "configuration": ["a"]},
                    {"kind": "external", "event": "go", "fired": [{"source": "a", "index": 0}],
                     "exited": ["a"], "entered": ["b"], "configuration": ["b"]}],
                   "inputs": [{"kind": "arrive", "after": 1, "event": "go"}]}"#,
            )
            .expect("a scenario"),
        )
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
                    scenario: Some(scenario()),
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

    /// Turns red if the schema accepts a record the judge never writes: each
    /// case changes one field of a record first shown valid, a stamp that
    /// names no producer among them.
    #[test]
    fn the_verdicts_schema_rejects_what_the_judge_does_not_write() {
        let schema = schema();
        let validator = jsonschema::JSONSchema::options()
            .with_draft(jsonschema::Draft::Draft7)
            .compile(&schema)
            .expect("the verdicts schema compiles");
        let valid: serde_json::Value =
            serde_json::from_str(&every_verdict().to_text()).expect("the record is JSON");
        assert!(
            validator.validate(&valid).is_ok(),
            "the control record must be valid, or a refusal below shows nothing"
        );
        let cases: [(&str, fn(&mut serde_json::Value)); 6] = [
            ("a record without the generator stamp", |record| {
                record
                    .as_object_mut()
                    .expect("a record is an object")
                    .remove("generator");
            }),
            ("a stamp that is a crate version", |record| {
                record["generator"] = serde_json::json!("0.1.0");
            }),
            ("a digest that is no sha256", |record| {
                record["digests"]["machine"] = serde_json::json!("abc");
            }),
            ("a verdict the judge does not say", |record| {
                record["verdicts"][0]["verdict"] = serde_json::json!("proved");
            }),
            ("a form no property is written in", |record| {
                record["verdicts"][0]["form"] = serde_json::json!("eventually");
            }),
            ("a member the record does not have", |record| {
                record["note"] = serde_json::json!("x");
            }),
        ];
        for (case, change) in cases {
            let mut changed = valid.clone();
            change(&mut changed);
            assert!(
                validator.validate(&changed).is_err(),
                "{case} must be refused"
            );
        }
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
                scenario: None,
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
                run: "Error: Invariant G is violated.\nState 1".to_string(),
                scenario: None,
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

    /// What the wire schema finds wrong with the record `text`.
    fn schema_errors(text: &str) -> Vec<String> {
        let schema = schema();
        let validator = jsonschema::JSONSchema::options()
            .with_draft(jsonschema::Draft::Draft7)
            .compile(&schema)
            .expect("the verdicts schema compiles");
        let instance: serde_json::Value = serde_json::from_str(text).expect("the record is JSON");
        let found: Vec<String> = match validator.validate(&instance) {
            Ok(()) => Vec::new(),
            Err(errors) => errors.map(|e| e.to_string()).collect(),
        };
        found
    }

    /// Turns red if a violation's scenario or a possibility's witness is
    /// dropped from the record, read back as another run, or named by another
    /// file than a lane reads it from.
    #[test]
    fn a_scenario_is_recorded_with_its_verdict() {
        let mut recorded = every_verdict();
        recorded.verdicts[0].witness = Some(scenario());
        recorded.macrosteps = Verdict::Violated {
            run: "Error: Temporal properties were violated.".to_string(),
            scenario: Some(scenario()),
        };
        let text = recorded.to_text();
        assert!(text.contains("\"inputs\""), "{text}");
        assert_eq!(read_record(&text, &digests()), Ok(recorded.clone()));
        assert_eq!(schema_errors(&text), Vec::<String>::new());
        let named: Vec<String> = recorded
            .scenarios()
            .into_iter()
            .map(|(name, run)| {
                assert_eq!(run, &*scenario());
                name
            })
            .collect();
        assert_eq!(
            named,
            [
                "macrosteps.scenario.json",
                "R1.scenario.json",
                "R3.scenario.json"
            ]
        );
    }

    /// Turns red if a violation whose behaviour cannot be read as a scenario
    /// is told as violated, or one whose behaviour can is told without it.
    #[test]
    fn a_violation_no_lane_could_replay_is_not_judged() {
        let violated = Verdict::Violated {
            run: "Error: Invariant G is violated.".to_string(),
            scenario: None,
        };
        assert!(matches!(
            with_scenario(violated.clone(), "Error: Invariant G is violated.\n", &[]),
            Verdict::NotJudged { why }
                if why.starts_with("TLC found a run that breaks it, which is not read as a scenario")
        ));
        let output = "Error: Invariant G is violated.\n\
                      Error: The behavior up to this point is:\n\
                      State 1: <Initial predicate>\n\
                      /\\ configuration = {}\n\
                      /\\ told = [exited |-> <<>>, fired |-> <<>>, entered |-> <<>>]\n\
                      \n\
                      State 2: <PStart line 1, col 1 to line 1, col 9 of module M>\n\
                      /\\ configuration = {\"a\"}\n\
                      /\\ told = [exited |-> <<>>, fired |-> <<1>>, entered |-> <<\"a\">>]\n\
                      \n";
        match with_scenario(violated, output, &[]) {
            Verdict::Violated {
                scenario: Some(run),
                ..
            } => assert_eq!(run.steps.len(), 1),
            other => panic!("expected a violation with its scenario, got {other:?}"),
        }
        assert_eq!(
            with_scenario(Verdict::HoldsOfTheWholeSpace, "", &[]),
            Verdict::HoldsOfTheWholeSpace
        );
    }

    /// Turns red if a guarantee no alteration of the machine breaks passes,
    /// one an alteration breaks is refused, or an alteration no guarantee
    /// notices goes unlisted.
    #[test]
    fn a_guarantee_no_alteration_breaks_is_refused_by_name() {
        let held = judgement(vec![
            ("R1", "always", Verdict::HoldsOfTheWholeSpace),
            ("R2", "leads-to", Verdict::HoldsOfTheWholeSpace),
        ]);
        let altered = |noticed_by: &[&str]| Coverage {
            alterations: vec![
                AlterationVerdict {
                    alteration: "transition 0 of state `a` taken out".to_string(),
                    noticed_by: noticed_by.iter().map(|req| req.to_string()).collect(),
                    not_judged: None,
                },
                AlterationVerdict {
                    alteration: "action 0 of the <onentry> 0 of state `b` taken out".to_string(),
                    noticed_by: Vec::new(),
                    not_judged: None,
                },
            ],
        };
        let partly = Judgement {
            coverage: Some(altered(&["R2"])),
            ..held.clone()
        };
        match partly.failure("p.scxml") {
            Some(FormalError::PropertyRefused { refusals, .. }) => {
                assert_eq!(refusals, [format!("`R1` (<sce:always>): {UNBROKEN}")]);
            }
            other => panic!("expected formal/property-refused, got {other:?}"),
        }
        let whole = Judgement {
            coverage: Some(altered(&["R1", "R2"])),
            ..held
        };
        assert!(whole.failure("p.scxml").is_none());
        let unnoticed: Vec<&str> = whole
            .coverage
            .as_ref()
            .expect("a coverage")
            .unnoticed()
            .map(|altered| altered.alteration.as_str())
            .collect();
        assert_eq!(
            unnoticed,
            ["action 0 of the <onentry> 0 of state `b` taken out"]
        );
        let text = whole.to_text();
        assert_eq!(read_record(&text, &digests()), Ok(whole.clone()));
        assert_eq!(schema_errors(&text), Vec::<String>::new());
    }
}
