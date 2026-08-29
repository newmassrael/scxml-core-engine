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
    ScopeStage::WriteTargets,
    ScopeStage::Everything,
];

fn stage_name(stage: ScopeStage) -> &'static str {
    match stage {
        ScopeStage::Installed => "installed",
        ScopeStage::DataModel => "datamodel",
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
                if *stage == ScopeStage::WriteTargets {
                    census.residue.push(format!(
                        "{}: {} {:?}\n      holding write targets: {got}\n      holding everything:    {want}",
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
fn control(datamodel: &str, body: &str, cond: &str) -> SCXMLModel {
    let document = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="a">
  {datamodel}
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
        "counter &gt; 0",
    );
    // `<assign location>` — crossed between DataModel and WriteTargets.
    let by_write = control(
        "",
        r#"<onentry><assign location="written" expr="1"/></onentry>"#,
        "written &gt; 0",
    );
    // `<script>` top-level declaration — crossed between WriteTargets and
    // Everything.
    let by_script = control(
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
            "<assign location>",
            &by_write,
            ScopeStage::DataModel,
            ScopeStage::WriteTargets,
        ),
        (
            "<script> declaration",
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
         write_targets_diverging={} write_targets_documents={}",
        census.documents,
        census.sites,
        census.diverging["installed"],
        census.documents_touched["installed"],
        census.diverging["datamodel"],
        census.documents_touched["datamodel"],
        census.diverging["write_targets"],
        census.documents_touched["write_targets"],
    );

    // The residue is what a scope handle maintained through every write
    // still gets wrong, so it is the exact population `declare_chunk`
    // exists for. It is named, not just counted: a run-time caller has to
    // be judged against these, and three lines a reader can check are
    // worth more than a total nobody can open.
    println!(
        "ScopeObligation residue ({} site(s) reachable only by `declare_chunk`):",
        census.residue.len()
    );
    for entry in &census.residue {
        println!("  - {entry}");
    }
    assert_eq!(
        census.residue.len(),
        census.diverging["write_targets"],
        "the residue list and the count it is meant to enumerate disagree"
    );
}
