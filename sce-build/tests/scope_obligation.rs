// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What a run-time lowering caller would owe the scope, counted rather
// than argued.
//
// `docs/SCE_LUA_TRANSLATION_SEAM.md` prices a C-callable lowering
// surface. Four questions had to be answered before that price is a
// decision; one — per-call cost — is measured by
// `scripts/measure-lowering-per-call.sh`. This file answers the second,
// and it is deliberately NOT a stopwatch: the doubt is not how long a
// scope costs, it is whether a run-time caller has to maintain one at
// all, and that is a correctness count.
//
// # The shape of the question
//
// A build-time caller reads the whole document before it lowers
// anything, so it always holds `ScopeStage::Everything`. A caller that
// lowers while the document RUNS does not: at the moment the first
// `<transition cond>` is evaluated, a `<script>` further down has not
// executed and an `<assign>` further down has not written. So it holds
// some earlier stage, and every site that lowers differently at that
// stage is a site the caller would get wrong.
//
// The census below lowers every expression in every tracked document
// once per stage and counts the disagreements against `Everything`. The
// number at `Installed` is the population an FFI with no scope handle
// would be wrong about; the number at `WriteTargets` is the population it
// would be wrong about if it carried a handle but called `declare_chunk`
// at the wrong moment.
//
// # Why a control stands beside the census
//
// A zero here is a decision — it would say the FFI needs no scope handle
// at all — which makes a FALSE zero the expensive failure. Every way this
// measurement could go blind produces one: a staging argument that is
// ignored, a corpus glob that matches nothing, a parse that fails
// everywhere. So the stage boundaries are not merely printed, they are
// each held against a document written to cross them. If a boundary stops
// being observable, `every_stage_boundary_is_observable` fails and the
// census is never read as an answer.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sce_build::ecmascript::builtins::INSTALLED_GLOBALS;
use sce_build::ecmascript::{DocumentScope, ScopeStage};
use sce_build::ecmascript_acceptance::sites;
use sce_build::model::SCXMLModel;
use sce_build::parser::SCXMLParser;

/// The stages a run-time caller passes through, earliest first. The last
/// is what a build-time caller always has, so it is the reference every
/// other stage is compared against.
const STAGES: &[ScopeStage] = &[
    ScopeStage::Installed,
    ScopeStage::DataModel,
    ScopeStage::LoadTime,
    ScopeStage::WriteTargets,
    ScopeStage::Everything,
];

fn stage_name(stage: ScopeStage) -> &'static str {
    match stage {
        ScopeStage::Installed => "installed",
        ScopeStage::DataModel => "datamodel",
        ScopeStage::LoadTime => "load_time",
        ScopeStage::WriteTargets => "write_targets",
        ScopeStage::Everything => "everything",
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// Every `.scxml` this repository tracks — the same population
/// `ecmascript_acceptance_parity` sweeps, for the same reason: a corpus
/// narrowed to authored examples would exclude the W3C documents, which
/// are where `<script>` declarations and late `<assign>` targets actually
/// live.
fn corpus() -> Vec<PathBuf> {
    let out = std::process::Command::new("git")
        .args(["ls-files", "*.scxml"])
        .current_dir(repo_root())
        .output()
        .expect("git ls-files");
    assert!(out.status.success(), "git ls-files failed");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|line| repo_root().join(line))
        .filter(|p| p.is_file())
        .collect()
}

/// Parse one document, or `None` when it does not parse — this corpus
/// carries deliberate negative fixtures and the stage that judges them is
/// not this one.
fn parse(path: &Path) -> Option<SCXMLModel> {
    let mut parser = SCXMLParser::new();
    let mut model = parser.parse_file(path.to_str()?).ok()?;
    sce_build::analyzer::analyze(&mut model, path.to_str()?);
    Some(model)
}

/// What one site lowered to, reduced to something two stages can be
/// compared on.
///
/// A refusal is an outcome, not an absence: an expression that lowers
/// under `Everything` and is refused under `Installed` is precisely the
/// disagreement this file is counting, and folding both into one string
/// keeps that case from being dropped as "no result on either side".
fn outcome(
    site: &sce_build::ecmascript_acceptance::ExpressionSite,
    scope: &DocumentScope,
) -> String {
    match site.lower(scope) {
        Ok(lua) => format!("ok:{lua}"),
        Err(error) => format!("err:{error}"),
    }
}

/// The counts one corpus walk produces.
#[derive(Default)]
struct Census {
    documents: usize,
    sites: usize,
    /// Sites whose lowering at this stage disagrees with `Everything`.
    diverging: BTreeMap<&'static str, usize>,
    /// Documents holding at least one such site.
    documents_touched: BTreeMap<&'static str, usize>,
    /// The sites still disagreeing at the LAST stage before
    /// `Everything` — the ones a caller gets wrong even holding a scope
    /// it maintains through every write. Named rather than counted,
    /// because the count is small enough that a reader can check it and
    /// a residue nobody can enumerate is not a residue anybody has
    /// classified.
    residue: Vec<String>,
}

fn walk() -> Census {
    let mut census = Census::default();
    for stage in STAGES {
        if *stage == ScopeStage::Everything {
            continue;
        }
        census.diverging.insert(stage_name(*stage), 0);
        census.documents_touched.insert(stage_name(*stage), 0);
    }

    for path in corpus() {
        let Some(model) = parse(&path) else { continue };
        let sites = sites(&model);
        if sites.is_empty() {
            continue;
        }
        census.documents += 1;
        census.sites += sites.len();

        let reference = DocumentScope::from_model_upto(&model, ScopeStage::Everything);
        let referenced: Vec<String> = sites.iter().map(|s| outcome(s, &reference)).collect();

        for stage in STAGES {
            if *stage == ScopeStage::Everything {
                continue;
            }
            let scope = DocumentScope::from_model_upto(&model, *stage);
            let mut here = 0usize;
            for (site, want) in sites.iter().zip(referenced.iter()) {
                let got = outcome(site, &scope);
                if got == *want {
                    continue;
                }
                here += 1;
                // Collected at DataModel, which is where the residue now
                // lives: a caller holding every `<data id>` and nothing
                // else. That set is exactly what the second entry point
                // — `declare_chunk` over the document-level `<script>`s
                // — buys, so enumerating it here names the ANSWER rather
                // than a leftover. It used to be collected at
                // WriteTargets, when a load-time name was reached only
                // by running the document; with the stages in their real
                // order that list is empty, and an empty list would have
                // read as "measured nothing".
                if *stage == ScopeStage::DataModel {
                    census.residue.push(format!(
                        "{}: {} {:?}\n      holding <data id> only: {got}\n      holding everything:     {want}",
                        path.strip_prefix(repo_root()).unwrap_or(&path).display(),
                        site.site,
                        site.source,
                    ));
                }
            }
            if here > 0 {
                *census
                    .documents_touched
                    .get_mut(stage_name(*stage))
                    .expect("stage seeded above") += 1;
                *census
                    .diverging
                    .get_mut(stage_name(*stage))
                    .expect("stage seeded above") += here;
            }
        }
    }
    census
}

/// A document written so that exactly one stage boundary decides whether
/// its `cond` lowers.
fn control(datamodel: &str, toplevel: &str, body: &str, cond: &str) -> SCXMLModel {
    let document = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="a">
  {datamodel}
  {toplevel}
  <state id="a">
    {body}
    <transition cond="{cond}" target="b"/>
  </state>
  <state id="b"/>
</scxml>
"#
    );
    let mut parser = SCXMLParser::new();
    let mut model = parser
        .parse_string(&document, "control")
        .expect("the control document parses");
    sce_build::analyzer::analyze(&mut model, "control.scxml");
    model
}

/// The census's instrument, proved able to see a difference at every
/// boundary it reports on.
///
/// Without this, a `from_model_upto` that ignored its `stage` argument
/// would report ZERO divergence at every stage — and zero is the reading
/// that would retire the scope handle from the C surface. A blind
/// instrument and a real absence produce the same number, so the
/// boundaries are held open by documents that cross them.
#[test]
fn every_stage_boundary_is_observable() {
    // `<data id>` — crossed between Installed and DataModel.
    let by_data = control(
        r#"<datamodel><data id="counter" expr="0"/></datamodel>"#,
        "",
        "",
        "counter &gt; 0",
    );
    // A DOCUMENT-LEVEL `<script>` — crossed between DataModel and
    // LoadTime. This is the boundary the census could not see at all
    // until the stage existed, and it is the one the residue of three
    // sits on: every one of them is a name a top-level `<script>`
    // introduces, which W3C SCXML 5.8 puts in the datamodel before the
    // first macrostep.
    let by_toplevel_script = control(
        "",
        r#"<script>var loaded = 1;</script>"#,
        "",
        "loaded &gt; 0",
    );
    // `<assign location>` — crossed between LoadTime and WriteTargets.
    let by_write = control(
        "",
        "",
        r#"<onentry><assign location="written" expr="1"/></onentry>"#,
        "written &gt; 0",
    );
    // A `<script>` INSIDE A STATE — crossed between WriteTargets and
    // Everything. Not the same boundary as the document-level one above,
    // and keeping both is what stops the two collapsing into one.
    let by_script = control(
        "",
        "",
        r#"<onentry><script>var declared = 1;</script></onentry>"#,
        "declared &gt; 0",
    );

    for (label, model, earlier, later) in [
        (
            "<data id>",
            &by_data,
            ScopeStage::Installed,
            ScopeStage::DataModel,
        ),
        (
            "document-level <script>",
            &by_toplevel_script,
            ScopeStage::DataModel,
            ScopeStage::LoadTime,
        ),
        (
            "<assign location>",
            &by_write,
            ScopeStage::LoadTime,
            ScopeStage::WriteTargets,
        ),
        (
            "in-state <script> declaration",
            &by_script,
            ScopeStage::WriteTargets,
            ScopeStage::Everything,
        ),
    ] {
        let sites = sites(model);
        assert!(
            !sites.is_empty(),
            "{label}: the control carries no expression site, so it proves nothing"
        );
        let before = DocumentScope::from_model_upto(model, earlier);
        let after = DocumentScope::from_model_upto(model, later);
        let crossed = sites
            .iter()
            .filter(|site| outcome(site, &before) != outcome(site, &after))
            .count();
        assert!(
            crossed > 0,
            "{label}: no site lowers differently between {} and {} — the census \
             cannot see this boundary, so a zero it reports there is not evidence \
             of anything. Outcomes: {:?}",
            stage_name(earlier),
            stage_name(later),
            sites
                .iter()
                .map(|s| (s.source.clone(), outcome(s, &before), outcome(s, &after)))
                .collect::<Vec<_>>()
        );
    }
}

/// The stages nest, so the divergence they report cannot grow as the
/// stage advances.
///
/// This is the invariant that makes a difference between two stages
/// attributable: if `WriteTargets` could disagree with `Everything` about
/// a site that `DataModel` agreed on, the extra names a stage adds would
/// be silencing declarations rather than adding them, and no number in
/// the census could be assigned to a cause.
#[test]
fn the_stages_nest() {
    let census = walk();
    let installed = census.diverging["installed"];
    let datamodel = census.diverging["datamodel"];
    let writes = census.diverging["write_targets"];
    assert!(
        installed >= datamodel && datamodel >= writes,
        "the stages do not nest: installed={installed} datamodel={datamodel} \
         write_targets={writes} — each stage admits everything the one before \
         it does, so its divergence against `Everything` cannot rise"
    );
}

/// The census itself. Printed rather than bounded: the corpus grows, and
/// a hard-coded total would turn every new document into a failure of
/// this file. What is asserted is that the walk actually swept something
/// — a glob that matched nothing, or a parser that refused everything,
/// would otherwise report a clean zero and read as "no obligation".
///
/// Re-derive the document's numbers with:
///
/// ```sh
/// scripts/measure-scope-obligation.sh
/// ```
#[test]
fn scope_obligation_census() {
    let census = walk();

    // 728 tracked `.scxml` when this bound was set, of which 225 carry
    // at least one expression the frontend is asked to lower. The bound
    // is on the second number, well under it, because a corpus that
    // shrinks is a legitimate change and a walk that finds nothing is
    // not.
    assert!(
        census.documents >= 200,
        "swept only {} document(s) carrying an expression — the corpus walk \
         found nothing to measure, which is not the same as finding no \
         obligation",
        census.documents
    );
    assert!(
        census.sites >= 1000,
        "swept only {} expression site(s)",
        census.sites
    );

    println!(
        "ScopeObligation census: documents={} sites={} \
         installed_diverging={} installed_documents={} \
         datamodel_diverging={} datamodel_documents={} \
         load_time_diverging={} load_time_documents={} \
         write_targets_diverging={} write_targets_documents={}",
        census.documents,
        census.sites,
        census.diverging["installed"],
        census.documents_touched["installed"],
        census.diverging["datamodel"],
        census.documents_touched["datamodel"],
        census.diverging["load_time"],
        census.documents_touched["load_time"],
        census.diverging["write_targets"],
        census.documents_touched["write_targets"],
    );

    // THE ANSWER, and the reason this stage was added. Everything a
    // run-time caller needs it can have BEFORE the first macrostep:
    // `<data id>` by early binding (W3C SCXML 5.3) and a document-level
    // `<script>` by load-time evaluation (W3C SCXML 5.8). If that is
    // true of the whole corpus, the obligation needs a `declare` and a
    // `declare_chunk` and NO execution-time scope tracking at all — and
    // this is the assertion that says so rather than a paragraph
    // claiming it.
    //
    // ⚠ A zero here is an ANSWER, so a blind instrument would forge one.
    // `every_stage_boundary_is_observable` is what stops that: it holds
    // the DataModel -> LoadTime boundary open with a document that
    // crosses it, so a `from_model_upto` that ignored the new stage
    // fails there before this reads zero.
    assert_eq!(
        census.diverging["load_time"], 0,
        "{} site(s) still diverge once the caller has read every `<data id>` \
         and every document-level `<script>`. Both are available before the \
         first macrostep, so a residue here would mean a run-time caller \
         needs scope maintained THROUGH execution — which is a different \
         C surface from the one this ledger prices. The sites: {:?}",
        census.diverging["load_time"], census.residue,
    );

    // What the SECOND entry point buys, named rather than counted: the
    // sites a caller holding every `<data id>` still gets wrong, and
    // which one `declare_chunk` over the document-level `<script>`s
    // answers. A reader can open all of them.
    println!(
        "ScopeObligation residue ({} site(s) `declare` alone cannot reach, \
         all answered by `declare_chunk`):",
        census.residue.len()
    );
    for entry in &census.residue {
        println!("  - {entry}");
    }
    assert_eq!(
        census.residue.len(),
        census.diverging["datamodel"],
        "the residue list and the count it is meant to enumerate disagree"
    );

    // The stages nest, so this is implied by the LoadTime zero above.
    // It is asserted anyway because it is the sentence a decision reads:
    // NOTHING in this corpus needs a scope that tracks execution.
    assert_eq!(
        census.diverging["write_targets"], 0,
        "a site diverges at write_targets that did not diverge at load_time, \
         which the nesting makes impossible — the ladder's order is wrong"
    );
}

/// A name no tracked document declares, so declaring it grows a scope
/// without answering anything that was refused for want of it.
const PROBE_NAME: &str = "__a_name_no_tracked_document_declares";

/// One scope on the growth ladder.
///
/// `redeclares` is carried rather than read back off the label, because
/// the floor below counts the rungs an AUTHOR'S growth produces and a
/// count taken from a name is a count a rename silently zeroes.
struct Rung {
    label: &'static str,
    scope: DocumentScope,
    /// This rung offers the scope a name it may already hold — an
    /// installed global, or a `<data id>` the stages already declared.
    redeclares: bool,
}

/// Scopes for one document, each holding every name the one before it
/// held.
///
/// The stages are the growth a RUN produces. The two after them are the
/// growth an AUTHOR produces and the stages cannot express: a `<data id>`
/// whose name the frontend already installed — `Math`, `In`, `_event` —
/// and a name nothing in the corpus uses. Both matter because
/// [`DocumentScope`] holds one set of names with no room to say where a
/// name came from, so re-declaring an installed global is the closest
/// this frontend has to shadowing, and it is the shape a caller would
/// have to worry about if lowering ever read the scope for anything but
/// a refusal.
fn ladder(model: &SCXMLModel) -> Vec<Rung> {
    let mut out: Vec<Rung> = STAGES
        .iter()
        .map(|stage| Rung {
            label: stage_name(*stage),
            scope: DocumentScope::from_model_upto(model, *stage),
            redeclares: false,
        })
        .collect();

    let mut shadowed = DocumentScope::from_model_upto(model, ScopeStage::Everything);
    for name in INSTALLED_GLOBALS {
        shadowed.declare(name);
    }
    for var in &model.variables {
        shadowed.declare(&var.id);
    }
    out.push(Rung {
        label: "everything+redeclared",
        scope: shadowed,
        redeclares: true,
    });

    let mut probed = DocumentScope::from_model_upto(model, ScopeStage::Everything);
    for name in INSTALLED_GLOBALS {
        probed.declare(name);
    }
    for var in &model.variables {
        probed.declare(&var.id);
    }
    probed.declare(PROBE_NAME);
    out.push(Rung {
        label: "everything+redeclared+probe",
        scope: probed,
        redeclares: true,
    });

    out
}

/// A document whose one expression lowers under the EMPTY scope, so the
/// whole ladder above answers it and every growth step is a comparison
/// rather than a skip.
///
/// `Math.round` names an installed global on purpose: the growth steps
/// this control exists for are the two that re-declare one, and an
/// expression naming nothing installed would walk past them unchanged
/// whatever they did.
fn closed_control() -> SCXMLModel {
    control("", "", "", "Math.round(2.5) &gt; 0")
}

/// A lowering that SUCCEEDED is not changed by the scope growing.
///
/// This is the premise `LuaEngine`'s two per-session caches rest on.
/// They are keyed on the AUTHOR'S text and hold only lowerings that
/// succeeded — a refusal returns before either map is written — so
/// serving a cached entry after the session declared something more is
/// correct exactly while this holds. `sce/src/scripting/LuaEngine.cpp`
/// cites this test by name where it says so.
///
/// It holds today for one reason, and the reason is narrow enough to be
/// worth watching: [`DocumentScope`] reaches lowering at exactly one
/// place — `ecmascript::resolve`'s `read`, which asks `declares` and
/// either continues or refuses — and the set it asks only ever grows. So
/// the scope decides WHETHER text lowers and never WHAT it lowers to.
/// The day an emitter reads the scope, or a name can leave it, this test
/// fails and those caches need their scope guard back. That is what its
/// failure message says, because the next reader will meet it there and
/// not here.
///
/// ⚠ A vacuous pass is the failure mode to design against: if nothing
/// lowered at a small scope, every comparison would be skipped and the
/// test would report a clean pass having measured nothing. Three things
/// stop that — a control document written to lower under the installed
/// scope alone, a floor on the comparisons the corpus contributes, and a
/// floor on how many of them cross a step that re-declares an installed
/// global.
#[test]
fn a_lowering_that_succeeded_is_unchanged_by_a_larger_scope() {
    let mut compared = 0usize;
    let mut across_redeclaration = 0usize;
    let mut disagreements: Vec<String> = Vec::new();

    let control_model = closed_control();
    let mut documents: Vec<(String, SCXMLModel)> = vec![("<control>".to_string(), control_model)];
    for path in corpus() {
        let Some(model) = parse(&path) else { continue };
        documents.push((
            path.strip_prefix(repo_root())
                .unwrap_or(&path)
                .display()
                .to_string(),
            model,
        ));
    }

    for (name, model) in &documents {
        let sites = sites(model);
        if sites.is_empty() {
            continue;
        }
        let ladder = ladder(model);
        for site in &sites {
            // The first scope on the ladder that lowers this site pins
            // the answer every later one has to repeat.
            let mut pinned: Option<(&'static str, String)> = None;
            for rung in &ladder {
                let Rung {
                    label,
                    scope,
                    redeclares,
                } = rung;
                let here = site.lower(scope);
                let Some((first_label, first)) = &pinned else {
                    if let Ok(lua) = here {
                        pinned = Some((label, lua));
                    }
                    continue;
                };
                compared += 1;
                if *redeclares {
                    across_redeclaration += 1;
                }
                match here {
                    Ok(lua) if lua == *first => {}
                    Ok(lua) => disagreements.push(format!(
                        "{name}: {} {:?}\n      at {first_label}: {first}\n      at {label}: {lua}",
                        site.site, site.source
                    )),
                    Err(error) => disagreements.push(format!(
                        "{name}: {} {:?}\n      at {first_label}: {first}\n      at {label}: REFUSED — {error}",
                        site.site, site.source
                    )),
                }
            }
        }
    }

    assert!(
        disagreements.is_empty(),
        "{} lowering(s) changed when the scope grew, so a lowering cached \
         against an earlier scope is no longer the answer. `LuaEngine`'s \
         `exprExecCache` and `scriptExecCache` are keyed on the author's text \
         and drop their scope guard on the strength of this test: restore \
         `scopeGeneration` on both maps, and restore the three cases \
         `sce-build/tests/mutations/a_session_scope_is_what_the_frontend_\
         answers_with.cases` retired, before this is made green again.\n  {}",
        disagreements.len(),
        disagreements.join("\n  ")
    );

    // 6362 comparisons when this bound was set, of which 2222 cross a
    // step that re-declares an installed global. Both bounds are well
    // under the measurement, because a corpus that shrinks is a
    // legitimate change and a walk that compares nothing is not.
    assert!(
        compared >= 4000,
        "only {compared} lowering(s) were compared across a growth step, so \
         this test did not measure the premise it reports on"
    );
    assert!(
        across_redeclaration >= 1400,
        "only {across_redeclaration} comparison(s) crossed a step that \
         re-declares an installed global — the growth an AUTHOR produces is \
         the half a stage ladder cannot express, and without it this test \
         measures only the half a run produces"
    );

    println!("ScopeGrowth: compared={compared} across_redeclaration={across_redeclaration}");
}
