// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh §16.3/§16.4 — distributability analyzer (R1-R4) and
// cross-region transition auto-merge.
//
// Entry point: [`analyze_distributability`]. Consumes a parsed
// [`DeployConfig`] plus per-machine [`SCXMLModel`]s and produces a
// [`ResolvedPartitionPlan`] whose `resolved` field is the partition
// map the downstream pipeline (codegen, wire-21 routing, barrier
// timer install) consumes. In `permissive` mode the resolver merges
// regions that violate R1 or R2 into a single partition (§16.4
// fixed-point); in `strict` mode any R1/R2 violation is returned as
// a [`DeployError`] and the build halts before codegen.
//
// Algorithm shape (mirrors SCE_MESH.md §16.3 pseudocode):
//
//   for each <parallel> P in every machine M:
//     A = ancestor-scope(P)   // root + ancestor-state datamodels
//     for each child region R of P:
//       writes_R  = { assign.location } ∪ { names in <script> (R4) }
//       reads_R   = { names textually matched in region-local exprs }
//       targets_R = { transition targets resolving inside a sibling }
//     emit R1 constraint when >=2 regions write the same L in A
//     emit R3 notice     when |writers(L)|=1 and some sibling reads L
//     emit R2 constraint when R has a target in sibling R'
//
//   resolve(constraints, author_plan, mode)
//     strict     → any constraint crossing partition → DeployError
//     permissive → merge into lowest-name canonical partition, emit
//                  [`MergeNotice`]; repeat until fixed point.
//
// Design notes:
//
// - R4 ("script opacity") lands as a conservative text-identifier
//   match: a `<script>` body is treated as writing every
//   ancestor-scope data id that appears as an identifier token in
//   the script source. This matches SCE_MESH.md §16.3 R4 prose
//   ("writes every ancestor-scope data name observed in the
//   script's lexical context") without requiring a per-language
//   script parser. The `sce:script-safe="true"` opt-out from §16.3
//   has no parser surface today — until an author hits a false
//   positive and opens a consumer, the conservative default is the
//   safe direction (more merges, not fewer).
//
// - Reads are detected textually over expression strings because
//   sce-build does not hold a datamodel-language AST. This
//   over-approximates (substrings inside string literals or
//   comments would match) which is again the safe direction: R3 is
//   an informational notice, never a build error.
//
// - Merge selects the lowest sort-ordered partition name as the
//   canonical survivor (§16.4). BTreeMap iteration preserves
//   lexicographic order so the fixed-point reaches a deterministic
//   result regardless of constraint discovery order.

use crate::mesh::deploy::{
    DeployConfig, DistributabilityMode, PartitionContains, PartitionDecl, PartitionInvokeRef,
    PartitionMap, PartitionUnitRef,
};
use crate::mesh::error::DeployError;
use crate::model::{Action, SCXMLModel, State};
use std::collections::{BTreeMap, BTreeSet};

/// Output of the distributability analyzer.
///
/// `resolved` is the partition map the downstream pipeline (codegen
/// in `sce-build/src/lib.rs::compile_mesh_transport`, wire-21
/// routing, barrier timer install) consumes. It is either the
/// author's original [`PartitionMap`] (no violations found) or the
/// §16.4 minimum-merge result (permissive mode merged one or more
/// regions).
#[derive(Debug, Clone)]
pub struct ResolvedPartitionPlan {
    /// The merged partition map. Equal to the author's original when
    /// the analyzer found no R1/R2 violations.
    pub resolved: PartitionMap,
    /// §16.4 merge notices — one per merge that collapsed two or
    /// more partitions. Empty in strict mode (strict never merges).
    pub merge_notices: Vec<MergeNotice>,
    /// §16.3 R3 snapshot-read notices — informational guidance for
    /// the author ("entry-point sync required"). Never blocks the
    /// build and is emitted identically in strict and permissive
    /// modes.
    pub snapshot_notices: Vec<SnapshotNotice>,
}

/// A single merge event recorded while collapsing partitions.
#[derive(Debug, Clone)]
pub struct MergeNotice {
    /// Machine name whose regions triggered the merge.
    pub machine: String,
    /// `<parallel>` state id that hosts the merged regions.
    pub parallel_id: String,
    /// Rule that forced the merge (R1 shared-write or R2
    /// cross-region transition).
    pub rule: MergeRule,
    /// Data location that multiple regions wrote (R1 only). Empty
    /// string for R2.
    pub location: String,
    /// Region ids participating in the merge (sorted).
    pub regions: Vec<String>,
    /// Partition names that were absorbed into the canonical
    /// survivor. Listed in sorted order, excluding the survivor.
    pub absorbed: Vec<String>,
    /// Canonical partition name that the absorbed partitions merged
    /// into. Lowest sort-ordered name from the pre-merge set, per
    /// §16.4.
    pub canonical: String,
}

/// Source rule for a [`MergeNotice`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeRule {
    /// R1 — two or more regions wrote the same ancestor-scope data
    /// location.
    R1SharedWrite,
    /// R2 — a transition crossed a region boundary inside the same
    /// `<parallel>`.
    R2CrossRegionTransition,
}

/// Informational R3 notice: a region reads an ancestor-scope data
/// location that a sibling region writes. The read is snapshot-
/// captured at parallel entry per §16.3 R3. No build error — the
/// notice only advises the author that entry-point sync is required.
#[derive(Debug, Clone)]
pub struct SnapshotNotice {
    pub machine: String,
    pub parallel_id: String,
    pub reader_region: String,
    pub writer_region: String,
    pub location: String,
}

/// Internal representation of a "these regions must share a
/// partition" obligation emitted by R1/R2. Drives the §16.4 merge
/// fixed-point and the strict-mode error stream.
#[derive(Debug, Clone)]
struct Constraint {
    machine: String,
    parallel_id: String,
    rule: MergeRule,
    /// Empty for R2; holds the data location for R1.
    location: String,
    /// Region ids participating in the constraint (sorted, unique).
    regions: Vec<String>,
}

/// Run the §16.3/§16.4 analyzer against a parsed deploy config and
/// the machines it references.
///
/// Returns:
///
/// - `Ok(plan)` — analyzer accepted the author's plan (with zero or
///   more permissive-mode merges already applied).
/// - `Err(vec)` — strict mode rejected one or more constraints.
///   Every violation is reported; the caller decides how to render
///   them.
///
/// No-ops to `Ok(author_plan)` when `cfg.partitions` is absent
/// (documents that opt out of partitioning never run through the
/// analyzer — §14 partitions is the opt-in surface).
pub fn analyze_distributability(
    cfg: &DeployConfig,
    models: &BTreeMap<String, SCXMLModel>,
) -> Result<ResolvedPartitionPlan, Vec<DeployError>> {
    let mode = cfg.distributability.unwrap_or_default();

    let Some(author_plan) = cfg.partitions.clone() else {
        return Ok(ResolvedPartitionPlan {
            resolved: PartitionMap::default(),
            merge_notices: Vec::new(),
            snapshot_notices: Vec::new(),
        });
    };

    let mut constraints: Vec<Constraint> = Vec::new();
    let mut snapshot_notices: Vec<SnapshotNotice> = Vec::new();

    for (machine_name, model) in models {
        let child_map = build_child_map(model);
        for (parallel_id, regions) in &model.parallel_regions {
            analyze_parallel(
                machine_name,
                parallel_id,
                regions,
                model,
                &child_map,
                &mut constraints,
                &mut snapshot_notices,
            );
        }
    }

    if mode == DistributabilityMode::Strict && !constraints.is_empty() {
        let errors = constraints
            .into_iter()
            .map(constraint_to_error)
            .collect::<Vec<_>>();
        return Err(errors);
    }

    let (resolved, merge_notices) = apply_constraints(author_plan, &constraints);

    Ok(ResolvedPartitionPlan {
        resolved,
        merge_notices,
        snapshot_notices,
    })
}

/// §16.3 per-parallel analysis: produce R1/R2 constraints and R3
/// notices for a single `<parallel>` inside one machine.
fn analyze_parallel(
    machine_name: &str,
    parallel_id: &str,
    regions: &[String],
    model: &SCXMLModel,
    child_map: &BTreeMap<String, Vec<String>>,
    constraints: &mut Vec<Constraint>,
    snapshot_notices: &mut Vec<SnapshotNotice>,
) {
    let ancestor_data = ancestor_scope_data(model, parallel_id);
    if regions.len() < 2 {
        // A <parallel> with a single region cannot have shared-write
        // or cross-region issues — the whole spec is about
        // **sibling** regions. Skip to keep downstream state clean.
        return;
    }

    // Per-region descendant set, writes, reads, and transition
    // targets. Built once so R1/R2/R3 can consult the same data.
    let region_data: Vec<RegionAnalysis> = regions
        .iter()
        .map(|r| analyze_region(r, model, child_map, &ancestor_data))
        .collect();

    // R1: for each ancestor data L, if 2+ regions write L → share
    // partition constraint.
    for loc in &ancestor_data {
        let writers: Vec<&String> = region_data
            .iter()
            .filter(|rd| rd.writes.contains(loc))
            .map(|rd| &rd.region_id)
            .collect();
        if writers.len() >= 2 {
            let mut regions_sorted: Vec<String> = writers.iter().map(|s| s.to_string()).collect();
            regions_sorted.sort();
            regions_sorted.dedup();
            constraints.push(Constraint {
                machine: machine_name.to_string(),
                parallel_id: parallel_id.to_string(),
                rule: MergeRule::R1SharedWrite,
                location: loc.clone(),
                regions: regions_sorted,
            });
        } else if writers.len() == 1 {
            // R3 — snapshot read notice for every sibling that
            // textually reads L.
            let writer = writers[0];
            for rd in &region_data {
                if &rd.region_id == writer {
                    continue;
                }
                if rd.reads.contains(loc) {
                    snapshot_notices.push(SnapshotNotice {
                        machine: machine_name.to_string(),
                        parallel_id: parallel_id.to_string(),
                        reader_region: rd.region_id.clone(),
                        writer_region: writer.clone(),
                        location: loc.clone(),
                    });
                }
            }
        }
    }

    // R2: a region with any sibling target → share partition with
    // each sibling that owns a target.
    for rd in &region_data {
        if rd.sibling_targets.is_empty() {
            continue;
        }
        // Map each target state back to the sibling region that
        // contains it. `sibling_targets` already filtered to targets
        // inside some sibling.
        let mut group: BTreeSet<String> = BTreeSet::new();
        group.insert(rd.region_id.clone());
        for tgt in &rd.sibling_targets {
            for other in &region_data {
                if other.region_id == rd.region_id {
                    continue;
                }
                if other.descendants.contains(tgt) || &other.region_id == tgt {
                    group.insert(other.region_id.clone());
                }
            }
        }
        if group.len() < 2 {
            continue;
        }
        let regions_sorted: Vec<String> = group.into_iter().collect();
        constraints.push(Constraint {
            machine: machine_name.to_string(),
            parallel_id: parallel_id.to_string(),
            rule: MergeRule::R2CrossRegionTransition,
            location: String::new(),
            regions: regions_sorted,
        });
    }
}

/// Per-region aggregation: the descendant set plus the location
/// sets the analyzer needs for R1/R2/R3 decisions.
struct RegionAnalysis {
    region_id: String,
    descendants: BTreeSet<String>,
    writes: BTreeSet<String>,
    reads: BTreeSet<String>,
    sibling_targets: BTreeSet<String>,
}

fn analyze_region(
    region_id: &str,
    model: &SCXMLModel,
    child_map: &BTreeMap<String, Vec<String>>,
    ancestor_data: &BTreeSet<String>,
) -> RegionAnalysis {
    let mut descendants: BTreeSet<String> = BTreeSet::new();
    collect_descendants(region_id, child_map, &mut descendants);

    let mut writes: BTreeSet<String> = BTreeSet::new();
    let mut reads: BTreeSet<String> = BTreeSet::new();
    let mut sibling_targets: BTreeSet<String> = BTreeSet::new();

    for state_id in &descendants {
        let Some(state) = model.states.get(state_id) else {
            continue;
        };
        collect_state_writes_reads(state, ancestor_data, &mut writes, &mut reads);
        for transition in &state.transitions {
            collect_transition_writes_reads(transition, ancestor_data, &mut writes, &mut reads);
            for target in split_targets(&transition.target) {
                // Target qualifies as "sibling" iff:
                //   1. it is not inside this region (not in descendants, not the region itself)
                //   2. it is a state id known to the model
                //   3. it is not an ancestor of the parallel (spec
                //      exception: targets that exit the parallel
                //      wholesale are not cross-region)
                if target == region_id || descendants.contains(&target) {
                    continue;
                }
                if !model.states.contains_key(&target) {
                    continue;
                }
                sibling_targets.insert(target);
            }
        }
    }

    RegionAnalysis {
        region_id: region_id.to_string(),
        descendants,
        writes,
        reads,
        sibling_targets,
    }
}

fn collect_state_writes_reads(
    state: &State,
    ancestor_data: &BTreeSet<String>,
    writes: &mut BTreeSet<String>,
    reads: &mut BTreeSet<String>,
) {
    for block in &state.on_entry_blocks {
        for action in block {
            collect_action_writes_reads(action, ancestor_data, writes, reads);
        }
    }
    for block in &state.on_exit_blocks {
        for action in block {
            collect_action_writes_reads(action, ancestor_data, writes, reads);
        }
    }
    // Region-local <datamodel> expressions — only reads (R1 note in
    // §16.3: a region-local <data expr="ancestor_name + 1"/> is
    // a **read** of `ancestor_name`, subject to R3 snapshot).
    for var in &state.datamodel {
        text_read_match(&var.expr, ancestor_data, reads);
        text_read_match(&var.content, ancestor_data, reads);
    }
}

fn collect_transition_writes_reads(
    transition: &crate::model::Transition,
    ancestor_data: &BTreeSet<String>,
    writes: &mut BTreeSet<String>,
    reads: &mut BTreeSet<String>,
) {
    text_read_match(&transition.cond, ancestor_data, reads);
    for action in &transition.actions {
        collect_action_writes_reads(action, ancestor_data, writes, reads);
    }
}

fn collect_action_writes_reads(
    action: &Action,
    ancestor_data: &BTreeSet<String>,
    writes: &mut BTreeSet<String>,
    reads: &mut BTreeSet<String>,
) {
    match action.action_type.as_str() {
        "assign" => {
            // Assign location is the LHS; exact match to an ancestor
            // data id is a direct write. Locations for nested fields
            // (e.g., `obj.field`) anchor on the identifier before the
            // first dot — R1 treats the whole `<data>` as written
            // conservatively.
            if let Some(root) = location_root(&action.location) {
                if ancestor_data.contains(root) {
                    writes.insert(root.to_string());
                }
            }
            // RHS expression reads.
            text_read_match(&action.expr, ancestor_data, reads);
        }
        "script" => {
            // R4: conservative — every ancestor-scope identifier
            // that appears as a token in the script body is treated
            // as a write. Matches "observed in the script's lexical
            // context" prose from §16.3 R4.
            script_identifier_match(&action.content, ancestor_data, writes);
            script_identifier_match(&action.content_transformed, ancestor_data, writes);
            script_identifier_match(&action.content_kt, ancestor_data, writes);
        }
        "log" => {
            text_read_match(&action.expr, ancestor_data, reads);
        }
        "if" => {
            text_read_match(&action.cond, ancestor_data, reads);
            for then_a in &action.then_actions {
                collect_action_writes_reads(then_a, ancestor_data, writes, reads);
            }
            for branch in &action.elseif_branches {
                text_read_match(&branch.cond, ancestor_data, reads);
                for a in &branch.actions {
                    collect_action_writes_reads(a, ancestor_data, writes, reads);
                }
            }
            for else_a in &action.else_actions {
                collect_action_writes_reads(else_a, ancestor_data, writes, reads);
            }
        }
        "foreach" => {
            text_read_match(&action.array, ancestor_data, reads);
            if let Some(root) = location_root(&action.item) {
                if ancestor_data.contains(root) {
                    // foreach writes the item variable on each iteration
                    writes.insert(root.to_string());
                }
            }
            if let Some(root) = location_root(&action.index) {
                if ancestor_data.contains(root) {
                    writes.insert(root.to_string());
                }
            }
            for a in &action.actions {
                collect_action_writes_reads(a, ancestor_data, writes, reads);
            }
        }
        "send" => {
            text_read_match(&action.expr, ancestor_data, reads);
            text_read_match(&action.eventexpr, ancestor_data, reads);
            text_read_match(&action.targetexpr, ancestor_data, reads);
            text_read_match(&action.typeexpr, ancestor_data, reads);
            text_read_match(&action.delayexpr, ancestor_data, reads);
            text_read_match(&action.contentexpr, ancestor_data, reads);
            text_read_match(&action.sendidexpr, ancestor_data, reads);
            // namelist is a space-separated list of identifiers
            for name in action.namelist.split_whitespace() {
                if ancestor_data.contains(name) {
                    reads.insert(name.to_string());
                }
            }
            for param in &action.params {
                text_read_match(&param.expr, ancestor_data, reads);
                if let Some(root) = location_root(&param.location) {
                    if ancestor_data.contains(root) {
                        reads.insert(root.to_string());
                    }
                }
            }
        }
        "cancel" => {
            text_read_match(&action.sendidexpr, ancestor_data, reads);
        }
        _ => {
            // Fall-through covers raise / log-variants / unknown —
            // every action record carries `expr` for its expression
            // payload; a best-effort read match handles future
            // kinds without silent gaps.
            text_read_match(&action.expr, ancestor_data, reads);
        }
    }
}

/// Scan a string for ancestor-data identifiers appearing as
/// substrings. Matches are bounded on both sides by ASCII
/// non-identifier characters (so `foo` does not match `foobar`).
/// Used for read detection on expression strings — analyzer is
/// datamodel-language-agnostic, so textual matching is the only
/// portable option without embedding per-language parsers.
fn text_read_match(text: &str, ancestor_data: &BTreeSet<String>, reads: &mut BTreeSet<String>) {
    if text.is_empty() || ancestor_data.is_empty() {
        return;
    }
    for name in ancestor_data {
        if has_identifier_token(text, name) {
            reads.insert(name.clone());
        }
    }
}

/// R4 script opacity: for each ancestor-scope identifier appearing
/// as a token in the script body, mark it as written. Uses the same
/// word-boundary rule as [`text_read_match`] but targets the
/// `writes` set.
fn script_identifier_match(
    text: &str,
    ancestor_data: &BTreeSet<String>,
    writes: &mut BTreeSet<String>,
) {
    if text.is_empty() || ancestor_data.is_empty() {
        return;
    }
    for name in ancestor_data {
        if has_identifier_token(text, name) {
            writes.insert(name.clone());
        }
    }
}

/// Word-boundary identifier match. Returns `true` iff `needle`
/// appears in `haystack` surrounded on both sides by either
/// start/end of string or a non-identifier byte (anything other
/// than ASCII alphanumeric or underscore).
fn has_identifier_token(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    let bytes = haystack.as_bytes();
    let n = needle.as_bytes();
    if n.len() > bytes.len() {
        return false;
    }
    let mut i = 0;
    while i + n.len() <= bytes.len() {
        if &bytes[i..i + n.len()] == n {
            let before_ok = i == 0 || !is_ident_byte(bytes[i - 1]);
            let after_idx = i + n.len();
            let after_ok = after_idx == bytes.len() || !is_ident_byte(bytes[after_idx]);
            if before_ok && after_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Extract the root identifier of an `<assign location>` /
/// `<param location>` expression. A location like `obj.field[0]` is
/// conservatively anchored on `obj`: R1 treats the whole data as
/// touched.
fn location_root(loc: &str) -> Option<&str> {
    let trimmed = loc.trim();
    if trimmed.is_empty() {
        return None;
    }
    let end = trimmed
        .bytes()
        .position(|b| !is_ident_byte(b))
        .unwrap_or(trimmed.len());
    if end == 0 {
        None
    } else {
        Some(&trimmed[..end])
    }
}

/// Split a W3C SCXML transition `target` attribute on whitespace
/// per §3.13.
fn split_targets(target: &str) -> Vec<String> {
    target.split_whitespace().map(str::to_string).collect()
}

/// Ancestor-scope data locations for a `<parallel>`: the union of
/// root `<datamodel>` ids plus every ancestor state's datamodel.
/// The parallel's own datamodel (if any) and its children's
/// datamodels are **not** in this set — they are region-local or
/// parallel-local and not relevant to R1/R3.
fn ancestor_scope_data(model: &SCXMLModel, parallel_id: &str) -> BTreeSet<String> {
    let mut out: BTreeSet<String> = BTreeSet::new();
    for var in &model.variables {
        if !var.id.is_empty() {
            out.insert(var.id.clone());
        }
    }
    // Walk parent chain starting from parallel's parent. Stop at
    // root (no parent).
    let mut cursor = model.states.get(parallel_id).and_then(|s| s.parent.clone());
    while let Some(pid) = cursor {
        if let Some(parent_state) = model.states.get(&pid) {
            for var in &parent_state.datamodel {
                if !var.id.is_empty() {
                    out.insert(var.id.clone());
                }
            }
            cursor = parent_state.parent.clone();
        } else {
            break;
        }
    }
    out
}

/// Build a parent→children lookup once per machine. The parser
/// populates `State.parent` on every state; inverting that map lets
/// region-subtree walking be a simple BFS without re-scanning all
/// states at every step.
fn build_child_map(model: &SCXMLModel) -> BTreeMap<String, Vec<String>> {
    let mut children: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (id, state) in &model.states {
        if let Some(parent) = &state.parent {
            children.entry(parent.clone()).or_default().push(id.clone());
        }
    }
    children
}

fn collect_descendants(
    root: &str,
    child_map: &BTreeMap<String, Vec<String>>,
    out: &mut BTreeSet<String>,
) {
    out.insert(root.to_string());
    if let Some(kids) = child_map.get(root) {
        for k in kids {
            collect_descendants(k, child_map, out);
        }
    }
}

/// Map a constraint to the equivalent strict-mode [`DeployError`]
/// so the caller can bubble a single typed error stream.
fn constraint_to_error(c: Constraint) -> DeployError {
    match c.rule {
        MergeRule::R1SharedWrite => DeployError::DistributabilityR1SharedWrite {
            machine: c.machine,
            parallel: c.parallel_id,
            location: c.location,
            regions: c.regions,
        },
        MergeRule::R2CrossRegionTransition => {
            DeployError::DistributabilityR2CrossRegionTransition {
                machine: c.machine,
                parallel: c.parallel_id,
                regions: c.regions,
            }
        }
    }
}

/// §16.4 auto-merge fixed-point. Repeatedly collapses partitions
/// that share a constraint-group until no more changes are needed,
/// selecting the lowest sort-ordered partition name as canonical
/// survivor for each merge event.
fn apply_constraints(
    author_plan: PartitionMap,
    constraints: &[Constraint],
) -> (PartitionMap, Vec<MergeNotice>) {
    if constraints.is_empty() {
        return (author_plan, Vec::new());
    }

    // Build region-id → partition-name index. A region id can only
    // appear in a single partition per §14 rule 8 (enforced at
    // deploy-parse time), so the map is well-defined.
    let mut region_to_partition: BTreeMap<(String, String), String> = BTreeMap::new();
    for (part_name, decl) in author_plan.iter() {
        for ur in &decl.contains.parallel_regions {
            region_to_partition.insert((ur.machine.clone(), ur.region.clone()), part_name.clone());
        }
    }

    // Working map of partition contents. Cloned from author_plan so
    // the author-visible original is preserved at the call site.
    let mut working: BTreeMap<String, PartitionDecl> = BTreeMap::new();
    for (name, decl) in author_plan.iter() {
        working.insert(name.clone(), decl.clone());
    }

    let mut notices: Vec<MergeNotice> = Vec::new();

    // Fixed-point: re-iterate constraints until a full pass produces
    // no merges. Typical convergence is 1-2 iterations because most
    // constraints are already satisfied after the first pass.
    loop {
        let mut changed = false;
        for c in constraints {
            let mut owning_partitions: BTreeSet<String> = BTreeSet::new();
            for region in &c.regions {
                if let Some(p) = region_to_partition.get(&(c.machine.clone(), region.clone())) {
                    owning_partitions.insert(p.clone());
                }
            }
            if owning_partitions.len() < 2 {
                continue;
            }
            let mut sorted: Vec<String> = owning_partitions.into_iter().collect();
            sorted.sort();
            let canonical = sorted.remove(0);
            let absorbed = sorted;

            // Move every unit from absorbed partitions into the
            // canonical partition.
            for part_name in &absorbed {
                let Some(decl) = working.remove(part_name) else {
                    continue;
                };
                let canon_decl = working
                    .get_mut(&canonical)
                    .expect("canonical partition must exist");
                merge_into(canon_decl, decl);
                // Re-key every region formerly in `part_name` to
                // canonical.
                for key in region_to_partition
                    .iter()
                    .filter_map(|(k, v)| {
                        if v == part_name {
                            Some(k.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                {
                    region_to_partition.insert(key, canonical.clone());
                }
            }

            notices.push(MergeNotice {
                machine: c.machine.clone(),
                parallel_id: c.parallel_id.clone(),
                rule: c.rule,
                location: c.location.clone(),
                regions: c.regions.clone(),
                absorbed,
                canonical: canonical.clone(),
            });
            changed = true;
        }
        if !changed {
            break;
        }
    }

    (PartitionMap::from_map(working), notices)
}

/// Absorb one partition's contents into another. Used by the §16.4
/// merge loop when a violating constraint forces two partitions to
/// collapse.
fn merge_into(dest: &mut PartitionDecl, src: PartitionDecl) {
    dest.machines = merge_string_vec(std::mem::take(&mut dest.machines), src.machines);

    let PartitionContains {
        parallel_regions,
        invokes,
    } = src.contains;
    dest.contains.parallel_regions = merge_region_refs(
        std::mem::take(&mut dest.contains.parallel_regions),
        parallel_regions,
    );
    dest.contains.invokes = merge_invoke_refs(std::mem::take(&mut dest.contains.invokes), invokes);

    if dest.transport_binding.is_none() {
        dest.transport_binding = src.transport_binding;
    }
    if dest.barrier_timeout_ms.is_none() {
        dest.barrier_timeout_ms = src.barrier_timeout_ms;
    }
    if let Some(src_roots) = src.hosts_parallel_roots {
        let dest_roots = dest.hosts_parallel_roots.get_or_insert_with(Vec::new);
        for r in src_roots {
            if !dest_roots
                .iter()
                .any(|existing| existing.machine == r.machine && existing.parallel == r.parallel)
            {
                dest_roots.push(r);
            }
        }
    }
}

fn merge_string_vec(mut a: Vec<String>, b: Vec<String>) -> Vec<String> {
    for v in b {
        if !a.contains(&v) {
            a.push(v);
        }
    }
    a.sort();
    a.dedup();
    a
}

fn merge_region_refs(
    mut a: Vec<PartitionUnitRef>,
    b: Vec<PartitionUnitRef>,
) -> Vec<PartitionUnitRef> {
    for v in b {
        if !a
            .iter()
            .any(|existing| existing.machine == v.machine && existing.region == v.region)
        {
            a.push(v);
        }
    }
    a.sort_by(|x, y| {
        x.machine
            .cmp(&y.machine)
            .then_with(|| x.region.cmp(&y.region))
    });
    a
}

fn merge_invoke_refs(
    mut a: Vec<PartitionInvokeRef>,
    b: Vec<PartitionInvokeRef>,
) -> Vec<PartitionInvokeRef> {
    for v in b {
        if !a
            .iter()
            .any(|existing| existing.machine == v.machine && existing.invoke == v.invoke)
        {
            a.push(v);
        }
    }
    a.sort_by(|x, y| {
        x.machine
            .cmp(&y.machine)
            .then_with(|| x.invoke.cmp(&y.invoke))
    });
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::deploy::parse_deploy_str;
    use crate::parser::SCXMLParser;

    fn parse_model(name: &str, xml: &str) -> SCXMLModel {
        let mut parser = SCXMLParser::new();
        parser
            .parse_string(xml, name)
            .unwrap_or_else(|e| panic!("parse '{name}' failed: {}", e.error))
    }

    fn models(entries: &[(&str, &str)]) -> BTreeMap<String, SCXMLModel> {
        let mut out = BTreeMap::new();
        for (name, xml) in entries {
            out.insert((*name).to_string(), parse_model(name, xml));
        }
        out
    }

    // Two-region machine with a root-level <data> that both regions
    // write. Used to exercise R1 shared-write detection.
    const MOTOR_R1_VIOLATION: &str = r#"<?xml version="1.0"?>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" name="motor"
               datamodel="lua" initial="root">
          <datamodel>
            <data id="shared" expr="0"/>
          </datamodel>
          <parallel id="root">
            <state id="left">
              <onentry><assign location="shared" expr="1"/></onentry>
            </state>
            <state id="right">
              <onentry><assign location="shared" expr="2"/></onentry>
            </state>
          </parallel>
        </scxml>"#;

    // Two-region machine with a cross-region transition (R2 violation).
    const MOTOR_R2_VIOLATION: &str = r#"<?xml version="1.0"?>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" name="motor"
               datamodel="lua" initial="root">
          <parallel id="root">
            <state id="left" initial="l1">
              <state id="l1">
                <transition event="go" target="r1"/>
              </state>
            </state>
            <state id="right" initial="r1">
              <state id="r1"/>
            </state>
          </parallel>
        </scxml>"#;

    // Two-region machine with one writer + one sibling reader of the
    // same ancestor-scope data (R3 notice, not a violation).
    const MOTOR_R3_NOTICE: &str = r#"<?xml version="1.0"?>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" name="motor"
               datamodel="lua" initial="root">
          <datamodel>
            <data id="shared" expr="0"/>
          </datamodel>
          <parallel id="root">
            <state id="left">
              <onentry><assign location="shared" expr="7"/></onentry>
            </state>
            <state id="right">
              <onentry><log expr="shared"/></onentry>
            </state>
          </parallel>
        </scxml>"#;

    // Two-region machine with no cross-region hazards — fully
    // region-local datamodels and transitions. Happy path.
    const MOTOR_HAPPY: &str = r#"<?xml version="1.0"?>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" name="motor"
               datamodel="lua" initial="root">
          <parallel id="root">
            <state id="left">
              <datamodel><data id="local_l" expr="0"/></datamodel>
              <onentry><assign location="local_l" expr="1"/></onentry>
            </state>
            <state id="right">
              <datamodel><data id="local_r" expr="0"/></datamodel>
              <onentry><assign location="local_r" expr="1"/></onentry>
            </state>
          </parallel>
        </scxml>"#;

    // Two-region machine where a <script> inside one region
    // textually references an ancestor-scope data name — R4
    // conservative treatment classifies it as a write, so combining
    // with a sibling assign forces R1.
    const MOTOR_R4_SCRIPT: &str = r#"<?xml version="1.0"?>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" name="motor"
               datamodel="lua" initial="root">
          <datamodel>
            <data id="shared" expr="0"/>
          </datamodel>
          <parallel id="root">
            <state id="left">
              <onentry><assign location="shared" expr="1"/></onentry>
            </state>
            <state id="right">
              <onentry><script>shared = 42</script></onentry>
            </state>
          </parallel>
        </scxml>"#;

    fn deploy_split_regions() -> &'static str {
        // Two partitions on one device, each hosting a single region
        // of the motor machine. Distributability analysis fires when
        // the regions are split like this.
        r#"version: '1.0'
topology:
  dev1:
    machines:
      motor:
        source: motor.scxml
partitions:
  motor_left:
    device: dev1
    machines: [motor]
    contains:
      parallel_regions:
        - machine: motor
          region: left
  motor_right:
    device: dev1
    machines: [motor]
    contains:
      parallel_regions:
        - machine: motor
          region: right
"#
    }

    #[test]
    fn happy_path_no_constraints() {
        let cfg = parse_deploy_str(deploy_split_regions()).expect("deploy parse");
        let ms = models(&[("motor", MOTOR_HAPPY)]);
        let plan = analyze_distributability(&cfg, &ms).expect("happy path Ok");
        assert!(plan.merge_notices.is_empty(), "no merges expected");
        assert!(plan.snapshot_notices.is_empty(), "no R3 notices expected");
        // Partition map unchanged — both named partitions survive.
        assert_eq!(plan.resolved.len(), 2);
        assert!(plan.resolved.get("motor_left").is_some());
        assert!(plan.resolved.get("motor_right").is_some());
    }

    #[test]
    fn r1_strict_rejects_build() {
        let yaml = format!("{}distributability: strict\n", deploy_split_regions());
        let cfg = parse_deploy_str(&yaml).expect("deploy parse");
        let ms = models(&[("motor", MOTOR_R1_VIOLATION)]);
        let err = analyze_distributability(&cfg, &ms).expect_err("strict must fail");
        assert_eq!(err.len(), 1);
        match &err[0] {
            DeployError::DistributabilityR1SharedWrite {
                machine,
                parallel,
                location,
                regions,
            } => {
                assert_eq!(machine, "motor");
                assert_eq!(parallel, "root");
                assert_eq!(location, "shared");
                assert_eq!(regions, &vec!["left".to_string(), "right".to_string()]);
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn r1_permissive_auto_merges() {
        let cfg = parse_deploy_str(deploy_split_regions()).expect("deploy parse");
        let ms = models(&[("motor", MOTOR_R1_VIOLATION)]);
        let plan = analyze_distributability(&cfg, &ms).expect("permissive Ok");
        assert_eq!(plan.merge_notices.len(), 1);
        let notice = &plan.merge_notices[0];
        assert_eq!(notice.rule, MergeRule::R1SharedWrite);
        assert_eq!(notice.canonical, "motor_left"); // lowest sort-order
        assert_eq!(notice.absorbed, vec!["motor_right".to_string()]);
        // After merge the resolved plan has a single partition.
        assert_eq!(plan.resolved.len(), 1);
        let merged = plan.resolved.get("motor_left").expect("canonical survives");
        assert_eq!(merged.contains.parallel_regions.len(), 2);
    }

    #[test]
    fn r2_strict_rejects_build() {
        let yaml = format!("{}distributability: strict\n", deploy_split_regions());
        let cfg = parse_deploy_str(&yaml).expect("deploy parse");
        let ms = models(&[("motor", MOTOR_R2_VIOLATION)]);
        let err = analyze_distributability(&cfg, &ms).expect_err("strict must fail");
        assert_eq!(err.len(), 1);
        match &err[0] {
            DeployError::DistributabilityR2CrossRegionTransition {
                machine,
                parallel,
                regions,
            } => {
                assert_eq!(machine, "motor");
                assert_eq!(parallel, "root");
                assert_eq!(regions, &vec!["left".to_string(), "right".to_string()]);
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn r2_permissive_auto_merges() {
        let cfg = parse_deploy_str(deploy_split_regions()).expect("deploy parse");
        let ms = models(&[("motor", MOTOR_R2_VIOLATION)]);
        let plan = analyze_distributability(&cfg, &ms).expect("permissive Ok");
        assert_eq!(plan.merge_notices.len(), 1);
        assert_eq!(
            plan.merge_notices[0].rule,
            MergeRule::R2CrossRegionTransition
        );
        assert_eq!(plan.resolved.len(), 1);
    }

    #[test]
    fn r3_notice_emitted_without_merge() {
        let cfg = parse_deploy_str(deploy_split_regions()).expect("deploy parse");
        let ms = models(&[("motor", MOTOR_R3_NOTICE)]);
        let plan = analyze_distributability(&cfg, &ms).expect("R3 is informational");
        assert!(plan.merge_notices.is_empty(), "R3 must not trigger a merge");
        assert_eq!(plan.snapshot_notices.len(), 1);
        let notice = &plan.snapshot_notices[0];
        assert_eq!(notice.reader_region, "right");
        assert_eq!(notice.writer_region, "left");
        assert_eq!(notice.location, "shared");
        assert_eq!(plan.resolved.len(), 2);
    }

    #[test]
    fn r4_script_textual_match_triggers_r1() {
        // Sibling region assigns `shared`; region with `<script>`
        // body textually references `shared` (R4 conservative). R1
        // sees both as writers → merge.
        let cfg = parse_deploy_str(deploy_split_regions()).expect("deploy parse");
        let ms = models(&[("motor", MOTOR_R4_SCRIPT)]);
        let plan = analyze_distributability(&cfg, &ms).expect("permissive Ok");
        assert_eq!(plan.merge_notices.len(), 1);
        assert_eq!(plan.merge_notices[0].rule, MergeRule::R1SharedWrite);
    }

    #[test]
    fn unpartitioned_deploy_is_noop() {
        // No partitions: block — analyzer short-circuits without
        // touching any model.
        let yaml = r#"version: '1.0'
topology:
  dev1:
    machines:
      motor:
        source: motor.scxml
"#;
        let cfg = parse_deploy_str(yaml).expect("deploy parse");
        let ms = models(&[("motor", MOTOR_R1_VIOLATION)]);
        let plan = analyze_distributability(&cfg, &ms).expect("noop Ok");
        assert!(plan.merge_notices.is_empty());
        assert!(plan.snapshot_notices.is_empty());
        assert!(plan.resolved.is_empty());
    }

    #[test]
    fn distributability_mode_default_is_permissive() {
        // Explicitly cover the default-value contract: absent
        // `distributability:` behaves like `permissive` so the
        // build does not fail on R1/R2.
        assert_eq!(
            DistributabilityMode::default(),
            DistributabilityMode::Permissive
        );
    }

    #[test]
    fn identifier_token_matching_is_word_bounded() {
        // `foo` inside `foobar` must not match — R4 conservative
        // write detection depends on word-boundary tokenisation.
        let mut set: BTreeSet<String> = BTreeSet::new();
        set.insert("foo".to_string());
        let mut reads: BTreeSet<String> = BTreeSet::new();
        text_read_match("foobar + 1", &set, &mut reads);
        assert!(
            reads.is_empty(),
            "'foo' must not match inside 'foobar': {reads:?}"
        );

        let mut reads2: BTreeSet<String> = BTreeSet::new();
        text_read_match("foo + 1", &set, &mut reads2);
        assert_eq!(reads2.len(), 1);
        assert!(reads2.contains("foo"));
    }

    #[test]
    fn fixed_point_merge_converges_on_chained_constraints() {
        // Three partitions, chained R1 violations forcing all three
        // into one canonical partition. Covers the fixed-point loop
        // (one pass merges A+B, second pass merges AB+C).
        let yaml = r#"version: '1.0'
topology:
  dev1:
    machines:
      motor:
        source: motor.scxml
partitions:
  motor_a:
    device: dev1
    machines: [motor]
    contains:
      parallel_regions:
        - machine: motor
          region: left
  motor_b:
    device: dev1
    machines: [motor]
    contains:
      parallel_regions:
        - machine: motor
          region: middle
  motor_c:
    device: dev1
    machines: [motor]
    contains:
      parallel_regions:
        - machine: motor
          region: right
"#;
        let cfg = parse_deploy_str(yaml).expect("deploy parse");
        let xml = r#"<?xml version="1.0"?>
            <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" name="motor"
                   datamodel="lua" initial="root">
              <datamodel>
                <data id="shared_lr" expr="0"/>
                <data id="shared_mr" expr="0"/>
              </datamodel>
              <parallel id="root">
                <state id="left">
                  <onentry><assign location="shared_lr" expr="1"/></onentry>
                </state>
                <state id="middle">
                  <onentry><assign location="shared_mr" expr="1"/></onentry>
                </state>
                <state id="right">
                  <onentry>
                    <assign location="shared_lr" expr="2"/>
                    <assign location="shared_mr" expr="2"/>
                  </onentry>
                </state>
              </parallel>
            </scxml>"#;
        let ms = models(&[("motor", xml)]);
        let plan = analyze_distributability(&cfg, &ms).expect("permissive Ok");
        // Two constraints (L-R and M-R) collapse all three
        // partitions into motor_a (alphabetically first).
        assert_eq!(plan.resolved.len(), 1);
        assert!(plan.resolved.get("motor_a").is_some());
        let merged = plan.resolved.get("motor_a").unwrap();
        assert_eq!(merged.contains.parallel_regions.len(), 3);
    }
}
