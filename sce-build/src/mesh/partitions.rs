// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh §14 partition resolution — cross-reference validators that
// require SCXML AST in addition to deploy.yaml.
//
// The `partitions:` schema checks that run at `parse_deploy_str` time
// (rules 6-10) live in `deploy.rs`; this module adds the rules that
// compare the declared coverage against the machines' actual
// `<parallel>` / `<invoke>` inventory:
//
//   Rule 1  — every orthogonal unit in a partition-listed machine is
//             covered by exactly one partition's `contains:`, counting
//             a `<machine>_default:` partition as legal coverage.
//   Rule 2  — the specific case where uncovered units exist *and* no
//             `<machine>_default:` has been declared; the diagnostic
//             quotes spec L2793-2800 verbatim so the author's cheapest
//             repair (extend an existing partition *or* add `_default`)
//             is surfaced directly.
//   Rule 11 — inner-region units of a nested `<parallel>` follow the
//             same coverage rule independently. `SCXMLModel::parallel_regions`
//             already indexes every parallel state in the document
//             (including those that sit inside another parallel) keyed
//             by the parallel-state id, so iterating the map values
//             enumerates every region depth uniformly — no explicit
//             recursion is required.
//
// Rule 5 (the `__sce_synth_invoke__` infix reservation) is a pure
// string check on `topology.*.machines.*` keys and therefore runs
// unconditionally inside `deploy.rs::validate_synth_invoke_infix`; it
// must hold even when `partitions:` is absent so opting into
// partitions later cannot quietly re-label a valid id as a collision.
//
// The filesystem-aware entry point is [`validate_partitions_against_scxml`],
// which resolves each listed machine's `source:` field against the
// deploy.yaml's parent directory and feeds the parsed models into the
// pure [`validate_partitions_against_models`] helper. Tests exercise the
// pure form directly; production callers hit the filesystem wrapper.

use crate::mesh::deploy::DeployConfig;
use crate::mesh::error::{DeployError, MeshError};
use crate::model::{Invoke, SCXMLModel};
use crate::parser::SCXMLParser;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// The reserved `<machine>_default:` partition name suffix
/// (SCE_MESH.md §14 rule 2). A machine `brake` with partial coverage
/// is directed to a partition literally named `brake_default`; the
/// constant is shared with rule 1 so the two diagnostics agree on
/// which partition they are looking for.
const DEFAULT_PARTITION_SUFFIX: &str = "_default";

/// Canonical path for a parallel region unit (SCE_MESH.md §14 rule 8
/// key shape). Emitted in rule-1 / rule-2 diagnostics so authors can
/// locate the unit in their SCXML directly.
fn unit_path_region(machine: &str, region: &str) -> String {
    format!("parallel_region:{machine}/{region}")
}

/// Canonical path for an invoke unit.
fn unit_path_invoke(machine: &str, invoke_id: &str) -> String {
    format!("invoke:{machine}/{invoke_id}")
}

/// The `invoke_id` for each invoke kind sits one layer deeper than the
/// enum; pattern-matching here keeps every caller one expression long.
fn invoke_id_of(invoke: &Invoke) -> &str {
    match invoke {
        Invoke::Scxml(info) => info.common.base.invoke_id.as_str(),
        Invoke::Hybrid(info) => info.common.base.invoke_id.as_str(),
        Invoke::MeshRpc(info) => info.base.invoke_id.as_str(),
    }
}

/// Enumerate every orthogonal unit that SCE Mesh §14 rule 1 requires
/// the partition graph to cover for a given machine.
///
/// Returns the canonical unit paths as they appear in
/// [`DeployError::PartitionUncoveredUnit::units`] — so the caller can
/// diff them directly against the coverage set without re-deriving a
/// path format.
///
/// Rule 11 (nested `<parallel>`) falls out naturally: the parser
/// populates [`SCXMLModel::parallel_regions`] with **every** parallel
/// state's child list, including parallels that sit under another
/// parallel, so iterating the map values already enumerates nested
/// regions at every depth.
fn enumerate_units(machine: &str, model: &SCXMLModel) -> Vec<String> {
    let mut units: BTreeSet<String> = BTreeSet::new();
    for regions in model.parallel_regions.values() {
        for region in regions {
            units.insert(unit_path_region(machine, region));
        }
    }
    for invoke in &model.invokes {
        units.insert(unit_path_invoke(machine, invoke_id_of(invoke)));
    }
    units.into_iter().collect()
}

/// Build the set of units covered by any partition's `contains:`
/// block, keyed exactly like the output of [`enumerate_units`].
fn collect_covered_units(cfg: &DeployConfig) -> BTreeSet<String> {
    let mut covered: BTreeSet<String> = BTreeSet::new();
    let Some(partitions) = &cfg.partitions else {
        return covered;
    };
    for (_, decl) in partitions.iter() {
        for region in &decl.contains.parallel_regions {
            covered.insert(unit_path_region(&region.machine, &region.region));
        }
        for invoke in &decl.contains.invokes {
            covered.insert(unit_path_invoke(&invoke.machine, &invoke.invoke));
        }
    }
    covered
}

/// SCE_MESH.md §14 rule 12 scaffolding carve-out (L2844): does the
/// machine appear under any partition's `machines:` list?
///
/// The predicate reads only existence — partition names, unit
/// assignments, `transport_binding:`, and the still-rejected
/// `hosts_parallel_roots:` value are all invisible here. The flag
/// flows onto [`crate::model::SCXMLModel::partition_context_present`]
/// to steer C++ `<parallel>`-final emission toward
/// `mesh/cpp/parallel_final.jinja2`; its P0 body is a byte-identical
/// copy of the inline path, so true/false produce the same code for a
/// single-process machine. Widening the signal (reading partition
/// values) would re-introduce the "built but unconsumed" anti-pattern
/// the banner explicitly forbids until the §16.5 runtime and rule 12
/// validator land atomically.
pub fn is_machine_partition_listed(cfg: &DeployConfig, machine: &str) -> bool {
    let Some(partitions) = &cfg.partitions else {
        return false;
    };
    partitions
        .iter()
        .any(|(_, decl)| decl.machines.iter().any(|m| m == machine))
}

/// The set of machines that appear under any partition's `machines:`
/// list. Rule 1 only applies to these — a machine absent from every
/// partition runs monolithically per spec L2791 (no warning).
pub fn partition_listed_machines(cfg: &DeployConfig) -> BTreeSet<String> {
    let mut listed: BTreeSet<String> = BTreeSet::new();
    let Some(partitions) = &cfg.partitions else {
        return listed;
    };
    for (_, decl) in partitions.iter() {
        for m in &decl.machines {
            listed.insert(m.clone());
        }
    }
    listed
}

/// SCE_MESH.md §14 rule 12 cross-reference validator. Every
/// distributed `<parallel>` (regions span two or more partitions) must
/// have exactly one partition claiming its root via
/// `partitions.<name>.hosts_parallel_roots:`, and every such claim
/// must satisfy the shape constraints in the spec (rule-9 machine
/// membership, co-hosting at least one region of the claimed parallel).
/// The five diagnostics at spec L2838-L2842 are spawned from this
/// function: `undesignated`, `ambiguous`, `not-in-machines`, `non-host`,
/// and `barrier-timeout-without-root`.
///
/// Single-partition `<parallel>`s (all regions live in one partition)
/// have that partition as implicit root — the `hosts_parallel_roots:`
/// entry may be omitted. Monolithic machines (absent from every
/// partition's `machines:` list) never enter this check; their
/// `<parallel>`s run single-process per spec L2791.
///
/// Called from [`validate_partitions_against_models`] after rule-1/2/11
/// coverage validation has passed — rule 12 assumes every unit has a
/// home and therefore every partition's `contains:` can be trusted as
/// the authoritative region assignment.
pub fn validate_parallel_root_designation(
    cfg: &DeployConfig,
    models: &BTreeMap<String, SCXMLModel>,
) -> Result<(), DeployError> {
    let Some(partitions) = &cfg.partitions else {
        return Ok(());
    };

    // Pass 1 — per-entry shape + non-host checks, and aggregate
    // (machine, parallel) → claimants for the ambiguity check.
    let mut claims_by_parallel: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();

    for (partition_name, decl) in partitions.iter() {
        // §14 rule 12 L2842 — `barrier_timeout_ms:` without any
        // `hosts_parallel_roots:` entry is a silent-broken knob (no
        // §16.5 tracker exists on the partition to gate). Treat the
        // `None` and `Some(vec![])` shapes identically — neither
        // declares a root.
        let declares_root = decl
            .hosts_parallel_roots
            .as_ref()
            .map(|v| !v.is_empty())
            .unwrap_or(false);

        if let Some(value) = decl.barrier_timeout_ms {
            if !declares_root {
                return Err(DeployError::PartitionBarrierTimeoutWithoutRoot {
                    partition: partition_name.clone(),
                    value,
                });
            }
        }

        let Some(roots) = decl.hosts_parallel_roots.as_ref() else {
            continue;
        };

        for root in roots {
            // §14 rule 12 L2840 — rule-9 shape: the claimed machine
            // must be one the partition already lists.
            if !decl.machines.iter().any(|m| m == &root.machine) {
                return Err(DeployError::PartitionParallelRootNotInMachines {
                    partition: partition_name.clone(),
                    claimed_machine: root.machine.clone(),
                    partition_machines: decl.machines.clone(),
                });
            }

            // §14 rule 12 L2841 — non-host: the partition must
            // co-host at least one region of the claimed parallel.
            // The claim is "a region of <parallel id=root.parallel>
            // in machine=root.machine sits in this partition's
            // `contains.parallel_regions:`".
            let parallel_regions = models
                .get(&root.machine)
                .and_then(|m| m.parallel_regions.get(&root.parallel));

            let co_hosts = match parallel_regions {
                None => false,
                Some(regions_of_parallel) => decl
                    .contains
                    .parallel_regions
                    .iter()
                    .any(|r| r.machine == root.machine && regions_of_parallel.contains(&r.region)),
            };

            if !co_hosts {
                return Err(DeployError::PartitionParallelRootNonHost {
                    partition: partition_name.clone(),
                    machine: root.machine.clone(),
                    parallel: root.parallel.clone(),
                });
            }

            claims_by_parallel
                .entry((root.machine.clone(), root.parallel.clone()))
                .or_default()
                .push(partition_name.clone());
        }
    }

    // Pass 2a — ambiguous claimants on the same (machine, parallel)
    // pair. Iteration is in `BTreeMap` key order, so the reported
    // partition list is deterministic across parses.
    for ((machine, parallel), claimants) in &claims_by_parallel {
        if claimants.len() > 1 {
            return Err(DeployError::PartitionParallelRootAmbiguous {
                machine: machine.clone(),
                parallel: parallel.clone(),
                claiming_partitions: claimants.clone(),
            });
        }
    }

    // Pass 2b — every distributed `<parallel>` must have exactly one
    // claimant. Walk the machines that appear under some partition's
    // `machines:` list (monolithic machines are exempt per spec
    // L2791). For each `<parallel>`, compute the set of partitions
    // hosting any of its regions; if that set has cardinality ≥ 2 the
    // parallel is distributed and needs a claim.
    let listed = partition_listed_machines(cfg);
    for machine in &listed {
        let Some(model) = models.get(machine) else {
            // Rule-1/2 would have fired earlier on a missing model;
            // rule 12 silently skips so it never overshadows the
            // earlier diagnostic.
            continue;
        };
        for (parallel_id, regions_of_parallel) in &model.parallel_regions {
            let hosting: BTreeSet<String> = partitions
                .iter()
                .filter(|(_, decl)| {
                    decl.contains.parallel_regions.iter().any(|r| {
                        r.machine == *machine && regions_of_parallel.contains(&r.region)
                    })
                })
                .map(|(name, _)| name.clone())
                .collect();

            if hosting.len() < 2 {
                // Single-partition (or unhosted, caught by rule 1)
                // — no root designation required.
                continue;
            }

            let claimed =
                claims_by_parallel.contains_key(&(machine.clone(), parallel_id.clone()));
            if !claimed {
                return Err(DeployError::PartitionParallelRootUndesignated {
                    machine: machine.clone(),
                    parallel: parallel_id.clone(),
                    hosting_partitions: hosting.into_iter().collect(),
                });
            }

            // §16.5 wire-21 transport implementation gap. The
            // `mesh_transport.h.jinja2` channel emitter materializes
            // `PartitionWire21Channel = SCE::Mesh::ShmChannel<>`
            // unconditionally. An explicit `transport_binding:
            // custom_tcp` on a partition that participates in this
            // distributed parallel's wire-21 route would compile to a
            // shm channel the runtime never opens — silent at compile
            // time, fail-loud at SM step time. Reject here so the
            // configuration gap surfaces at deploy validation. Pass 2b
            // already proved hosting.len() ≥ 2 (distributed) above, so
            // every partition in `hosting` carries wire-21 traffic.
            // Iteration is in `BTreeSet` order for deterministic
            // diagnostic output.
            for part_name in &hosting {
                let decl = partitions
                    .get(part_name)
                    .expect("hosting derived from partitions map");
                if decl.transport_binding.as_deref() == Some("custom_tcp") {
                    return Err(DeployError::PartitionWire21CustomTcpUnimplemented {
                        partition: part_name.clone(),
                        machine: machine.clone(),
                        parallel: parallel_id.clone(),
                    });
                }
            }
        }
    }

    Ok(())
}

/// Pure cross-reference validator: given a parsed
/// [`DeployConfig`] and a model-per-machine map, apply SCE_MESH.md §14
/// rules 1, 2, 11, and 12.
///
/// The caller is responsible for having parsed every machine named in
/// any partition's `machines:` list; a missing entry is itself treated
/// as a rule-1 failure (every listed unit is uncovered because there
/// is nothing to enumerate).
pub fn validate_partitions_against_models(
    cfg: &DeployConfig,
    models: &BTreeMap<String, SCXMLModel>,
) -> Result<(), DeployError> {
    if cfg.partitions.is_none() {
        return Ok(());
    }
    let listed = partition_listed_machines(cfg);
    if listed.is_empty() {
        return Ok(());
    }
    let covered = collect_covered_units(cfg);
    let partitions = cfg.partitions.as_ref().expect("guarded by is_none above");

    for machine in &listed {
        let model = match models.get(machine) {
            Some(m) => m,
            None => {
                // No SCXML content available ⇒ every potential unit
                // is uncovered by definition. The missing-source case
                // surfaces as rule 2 because the spec prose guides
                // the author toward declaring a default before it
                // can decide which units belong where.
                return Err(DeployError::PartitionPartialCoverageRequiresDefault {
                    machine: machine.clone(),
                    missing: vec![format!("<{machine} SCXML source missing>")],
                });
            }
        };

        let units = enumerate_units(machine, model);
        let uncovered: Vec<String> = units
            .into_iter()
            .filter(|u| !covered.contains(u))
            .collect();
        if uncovered.is_empty() {
            continue;
        }

        let default_name = format!("{machine}{DEFAULT_PARTITION_SUFFIX}");
        if partitions.get(&default_name).is_some() {
            return Err(DeployError::PartitionUncoveredUnit {
                machine: machine.clone(),
                units: uncovered,
            });
        }
        return Err(DeployError::PartitionPartialCoverageRequiresDefault {
            machine: machine.clone(),
            missing: uncovered,
        });
    }

    // §14 rule 12 — parallel root designation. Runs after rules
    // 1/2/11 so a coverage failure surfaces the repair for the cheaper
    // cause before the author is asked about distributed-parallel
    // designation.
    validate_parallel_root_designation(cfg, models)?;

    Ok(())
}

/// Filesystem-aware entry point: resolve each partition-listed
/// machine's `source:` field against the deploy.yaml parent directory,
/// parse the SCXML, and delegate to
/// [`validate_partitions_against_models`] for the actual rule checks.
///
/// Called from the mesh compile pipeline after `parse_deploy` but
/// before any topology work so a coverage violation aborts the build
/// at the same pipeline stage (§14 rules 1/2/11 are mesh-deploy-stage
/// diagnostics even though they inspect SCXML — the failure is still
/// that deploy.yaml's partition graph disagrees with the machines it
/// names).
///
/// A no-op when `partitions:` is absent, so every non-partitioned
/// deploy pays effectively zero cost (the early-exit fires in
/// [`validate_partitions_against_models`]).
pub fn validate_partitions_against_scxml(
    deploy_dir: &Path,
    cfg: &DeployConfig,
) -> Result<(), MeshError> {
    if cfg.partitions.is_none() {
        return Ok(());
    }
    let listed = partition_listed_machines(cfg);
    if listed.is_empty() {
        return Ok(());
    }

    let models = load_partition_machine_models(deploy_dir, cfg, &listed)?;
    validate_partitions_against_models(cfg, &models)?;
    Ok(())
}

/// Shared helper: parse every SCXML source referenced by a partition-
/// listed machine in `cfg.topology`. Used by
/// [`validate_partitions_against_scxml`] and the §16.3 distributability
/// analyzer — both need the same model set to reason about partition
/// coverage and region-level write/transition graphs.
///
/// Returns an empty map (Ok) when `cfg.partitions` is absent or no
/// partition-listed machines resolve — matching the "zero cost for
/// monolithic deployments" invariant.
pub fn load_partition_machine_models(
    deploy_dir: &Path,
    cfg: &DeployConfig,
    listed: &BTreeSet<String>,
) -> Result<BTreeMap<String, SCXMLModel>, MeshError> {
    // Resolve each listed machine's SCXML path via the deploy.yaml
    // machine table. Partition machines that are not declared in
    // `topology:` would already have been rejected by Phase A rule 9
    // (PartitionMachineNotListed) if referenced by any `contains:`
    // entry, but a bare `machines:` entry that names an unknown
    // machine has no earlier gate — fall back to the rule-2 path
    // (missing SCXML ⇒ no-default partial coverage).
    let mut machine_to_source: BTreeMap<String, String> = BTreeMap::new();
    for device in cfg.topology.values() {
        for (name, mc) in &device.machines {
            if listed.contains(name) {
                machine_to_source.insert(name.clone(), mc.source.clone());
            }
        }
    }

    let mut models: BTreeMap<String, SCXMLModel> = BTreeMap::new();
    for (machine, source) in &machine_to_source {
        let path = deploy_dir.join(source);
        let mut parser = SCXMLParser::new();
        let model = parser.parse_file(&path.display().to_string()).map_err(|e| {
            MeshError::Io {
                path: path.clone(),
                source: std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "failed to parse SCXML for machine '{machine}': {}",
                        e.error
                    ),
                ),
            }
        })?;
        models.insert(machine.clone(), model);
    }

    Ok(models)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::deploy::parse_deploy_str;

    fn parse_scxml(name: &str, content: &str) -> SCXMLModel {
        SCXMLParser::new()
            .parse_string(content, name)
            .expect("fixture SCXML must parse")
    }

    /// Minimal SCXML with a single `<parallel>` whose children are
    /// the two regions `left` and `right`.
    const TWO_REGION_PARALLEL: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" name="brake" initial="root" version="1.0">
  <parallel id="root">
    <state id="left"/>
    <state id="right"/>
  </parallel>
</scxml>
"#;

    /// Parallel with an author-named invoke on one region — rule 1
    /// must surface both the region and the invoke units.
    const REGION_WITH_INVOKE: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" name="brake" initial="root" version="1.0">
  <parallel id="root">
    <state id="monitoring"/>
    <state id="control">
      <invoke id="compute_force_inv" type="scxml" src="force.scxml"/>
    </state>
  </parallel>
</scxml>
"#;

    /// Nested parallel: outer region `outer_r` contains another
    /// `<parallel>` whose child `inner_r` must also be covered
    /// (SCE_MESH.md §14 rule 11).
    const NESTED_PARALLEL: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" name="brake" initial="outer" version="1.0">
  <parallel id="outer">
    <state id="outer_r">
      <parallel id="inner">
        <state id="inner_r"/>
        <state id="inner_r2"/>
      </parallel>
    </state>
    <state id="outer_r2"/>
  </parallel>
</scxml>
"#;

    #[test]
    fn happy_path_full_coverage_passes() {
        // Two regions split across two partitions. With rule 12 now
        // enforcing root designation, a distributed parallel needs a
        // claimant — brake_left volunteers.
        let deploy = r#"
topology:
  ecu:
    machines:
      brake: { source: brake.scxml }
partitions:
  brake_left:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: left }
    hosts_parallel_roots:
      - { machine: brake, parallel: root }
  brake_right:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: right }
"#;
        let cfg = parse_deploy_str(deploy).expect("deploy must parse");
        let mut models = BTreeMap::new();
        models.insert("brake".to_string(), parse_scxml("brake", TWO_REGION_PARALLEL));
        validate_partitions_against_models(&cfg, &models).expect("full coverage must pass");
    }

    #[test]
    fn machine_not_listed_runs_monolithic() {
        // Spec L2791: a machine absent from every partition: entry
        // runs single-process with no warning. brake has parallel
        // regions but is not in partitions — must not error.
        let deploy = r#"
topology:
  ecu:
    machines:
      brake: { source: brake.scxml }
      motor: { source: motor.scxml }
partitions:
  motor_only:
    machines: [motor]
    contains:
      parallel_regions:
        - { machine: motor, region: solo }
"#;
        let cfg = parse_deploy_str(deploy).expect("deploy must parse");
        let motor_scxml = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" name="motor" initial="root" version="1.0">
  <parallel id="root">
    <state id="solo"/>
  </parallel>
</scxml>
"#;
        let mut models = BTreeMap::new();
        models.insert("brake".to_string(), parse_scxml("brake", TWO_REGION_PARALLEL));
        models.insert("motor".to_string(), parse_scxml("motor", motor_scxml));
        validate_partitions_against_models(&cfg, &models)
            .expect("unmentioned machine must run monolithic without error");
    }

    #[test]
    fn rule2_partial_coverage_without_default() {
        // brake is listed but its `right` region is uncovered, and no
        // brake_default partition exists — must surface rule 2.
        let deploy = r#"
topology:
  ecu:
    machines:
      brake: { source: brake.scxml }
partitions:
  brake_left:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: left }
"#;
        let cfg = parse_deploy_str(deploy).expect("deploy must parse");
        let mut models = BTreeMap::new();
        models.insert("brake".to_string(), parse_scxml("brake", TWO_REGION_PARALLEL));
        let err = validate_partitions_against_models(&cfg, &models)
            .expect_err("partial coverage must fail");
        match err {
            DeployError::PartitionPartialCoverageRequiresDefault { machine, missing } => {
                assert_eq!(machine, "brake");
                assert_eq!(missing, vec!["parallel_region:brake/right".to_string()]);
            }
            other => panic!("expected PartitionPartialCoverageRequiresDefault, got {other:?}"),
        }
    }

    #[test]
    fn rule1_incomplete_default_partition() {
        // brake has a brake_default partition, but `right` is STILL
        // uncovered (default is incomplete). Rule 1 fires with the
        // "extend the existing default" wording.
        let deploy = r#"
topology:
  ecu:
    machines:
      brake: { source: brake.scxml }
partitions:
  brake_left:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: left }
  brake_default:
    machines: [brake]
    contains:
      invokes:
        - { machine: brake, invoke: some_other_invoke }
"#;
        let cfg = parse_deploy_str(deploy).expect("deploy must parse");
        let mut models = BTreeMap::new();
        models.insert("brake".to_string(), parse_scxml("brake", TWO_REGION_PARALLEL));
        let err = validate_partitions_against_models(&cfg, &models)
            .expect_err("incomplete default partition must fail");
        match err {
            DeployError::PartitionUncoveredUnit { machine, units } => {
                assert_eq!(machine, "brake");
                assert_eq!(units, vec!["parallel_region:brake/right".to_string()]);
            }
            other => panic!("expected PartitionUncoveredUnit, got {other:?}"),
        }
    }

    #[test]
    fn rule11_nested_parallel_inner_region_required() {
        // outer region `outer_r` is covered, but the nested parallel
        // `inner` has `inner_r` and `inner_r2` children that the
        // walker must also surface. Rule 11 is implicit: rule 1 already
        // sees the nested regions via `parallel_regions` indexing.
        let deploy = r#"
topology:
  ecu:
    machines:
      brake: { source: brake.scxml }
partitions:
  brake_outer:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: outer_r }
        - { machine: brake, region: outer_r2 }
        - { machine: brake, region: inner_r }
"#;
        let cfg = parse_deploy_str(deploy).expect("deploy must parse");
        let mut models = BTreeMap::new();
        models.insert("brake".to_string(), parse_scxml("brake", NESTED_PARALLEL));
        let err = validate_partitions_against_models(&cfg, &models)
            .expect_err("nested uncovered region must fail");
        match err {
            DeployError::PartitionPartialCoverageRequiresDefault { machine, missing } => {
                assert_eq!(machine, "brake");
                assert_eq!(missing, vec!["parallel_region:brake/inner_r2".to_string()]);
            }
            other => panic!("expected PartitionPartialCoverageRequiresDefault, got {other:?}"),
        }
    }

    #[test]
    fn invokes_are_enumerated_as_units() {
        // Author-named invoke must be surfaced when uncovered. Covers
        // the "every <invoke>" half of rule 1 that the region-only
        // fixtures cannot exercise.
        let deploy = r#"
topology:
  ecu:
    machines:
      brake: { source: brake.scxml }
partitions:
  brake_regions:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: monitoring }
        - { machine: brake, region: control }
"#;
        let cfg = parse_deploy_str(deploy).expect("deploy must parse");
        let mut models = BTreeMap::new();
        models.insert("brake".to_string(), parse_scxml("brake", REGION_WITH_INVOKE));
        let err = validate_partitions_against_models(&cfg, &models)
            .expect_err("uncovered invoke must fail");
        match err {
            DeployError::PartitionPartialCoverageRequiresDefault { machine, missing } => {
                assert_eq!(machine, "brake");
                assert_eq!(missing, vec!["invoke:brake/compute_force_inv".to_string()]);
            }
            other => panic!("expected PartitionPartialCoverageRequiresDefault, got {other:?}"),
        }
    }

    #[test]
    fn no_partitions_block_is_noop() {
        let deploy = r#"
topology:
  ecu:
    machines:
      brake: { source: brake.scxml }
"#;
        let cfg = parse_deploy_str(deploy).expect("deploy must parse");
        let mut models = BTreeMap::new();
        models.insert("brake".to_string(), parse_scxml("brake", TWO_REGION_PARALLEL));
        validate_partitions_against_models(&cfg, &models)
            .expect("absent partitions block must short-circuit to Ok");
    }

    // ── §14 rule 12 — parallel root designation ─────────────────

    #[test]
    fn rule12_distributed_parallel_without_root_fires_undesignated() {
        // brake has two regions split across two partitions and no
        // partition claims root — must surface undesignated.
        let deploy = r#"
topology:
  ecu:
    machines:
      brake: { source: brake.scxml }
partitions:
  brake_left:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: left }
  brake_right:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: right }
"#;
        let cfg = parse_deploy_str(deploy).expect("deploy must parse");
        let mut models = BTreeMap::new();
        models.insert("brake".to_string(), parse_scxml("brake", TWO_REGION_PARALLEL));
        let err = validate_partitions_against_models(&cfg, &models)
            .expect_err("distributed parallel with no claimant must fail");
        match err {
            DeployError::PartitionParallelRootUndesignated {
                machine,
                parallel,
                hosting_partitions,
            } => {
                assert_eq!(machine, "brake");
                assert_eq!(parallel, "root");
                assert_eq!(hosting_partitions, vec!["brake_left", "brake_right"]);
            }
            other => panic!("expected PartitionParallelRootUndesignated, got {other:?}"),
        }
    }

    #[test]
    fn rule12_single_partition_parallel_has_implicit_root() {
        // All regions live in one partition — no claim needed, no error.
        let deploy = r#"
topology:
  ecu:
    machines:
      brake: { source: brake.scxml }
partitions:
  brake_all:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: left }
        - { machine: brake, region: right }
"#;
        let cfg = parse_deploy_str(deploy).expect("deploy must parse");
        let mut models = BTreeMap::new();
        models.insert("brake".to_string(), parse_scxml("brake", TWO_REGION_PARALLEL));
        validate_partitions_against_models(&cfg, &models)
            .expect("single-partition parallel has implicit root — no error");
    }

    #[test]
    fn rule12_valid_root_claim_passes() {
        // Distributed parallel with exactly one claimant that co-hosts
        // one of the regions — rule 12 accepts.
        let deploy = r#"
topology:
  ecu:
    machines:
      brake: { source: brake.scxml }
partitions:
  brake_left:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: left }
    hosts_parallel_roots:
      - { machine: brake, parallel: root }
  brake_right:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: right }
"#;
        let cfg = parse_deploy_str(deploy).expect("deploy must parse");
        let mut models = BTreeMap::new();
        models.insert("brake".to_string(), parse_scxml("brake", TWO_REGION_PARALLEL));
        validate_partitions_against_models(&cfg, &models)
            .expect("well-formed distributed parallel with one root must pass");
    }

    #[test]
    fn rule12_two_claimants_fire_ambiguous() {
        let deploy = r#"
topology:
  ecu:
    machines:
      brake: { source: brake.scxml }
partitions:
  brake_left:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: left }
    hosts_parallel_roots:
      - { machine: brake, parallel: root }
  brake_right:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: right }
    hosts_parallel_roots:
      - { machine: brake, parallel: root }
"#;
        let cfg = parse_deploy_str(deploy).expect("deploy must parse");
        let mut models = BTreeMap::new();
        models.insert("brake".to_string(), parse_scxml("brake", TWO_REGION_PARALLEL));
        let err = validate_partitions_against_models(&cfg, &models)
            .expect_err("two claimants must fail");
        match err {
            DeployError::PartitionParallelRootAmbiguous {
                machine,
                parallel,
                claiming_partitions,
            } => {
                assert_eq!(machine, "brake");
                assert_eq!(parallel, "root");
                assert_eq!(claiming_partitions, vec!["brake_left", "brake_right"]);
            }
            other => panic!("expected PartitionParallelRootAmbiguous, got {other:?}"),
        }
    }

    #[test]
    fn rule12_claim_references_unlisted_machine_fires_not_in_machines() {
        // Partition claims a parallel in a machine not in its
        // `machines:` list — rule-9 shape applied to root entries.
        let deploy = r#"
topology:
  ecu:
    machines:
      brake: { source: brake.scxml }
      motor: { source: motor.scxml }
partitions:
  brake_left:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: left }
    hosts_parallel_roots:
      - { machine: motor, parallel: root }
  brake_right:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: right }
"#;
        let cfg = parse_deploy_str(deploy).expect("deploy must parse");
        let mut models = BTreeMap::new();
        models.insert("brake".to_string(), parse_scxml("brake", TWO_REGION_PARALLEL));
        // motor is untouched so the claim cannot resolve against any model.
        models.insert(
            "motor".to_string(),
            parse_scxml(
                "motor",
                r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" name="motor" initial="idle" version="1.0">
  <state id="idle"/>
</scxml>
"#,
            ),
        );
        let err = validate_partitions_against_models(&cfg, &models)
            .expect_err("claim against non-listed machine must fail");
        match err {
            DeployError::PartitionParallelRootNotInMachines {
                partition,
                claimed_machine,
                partition_machines,
            } => {
                assert_eq!(partition, "brake_left");
                assert_eq!(claimed_machine, "motor");
                assert_eq!(partition_machines, vec!["brake"]);
            }
            other => panic!("expected PartitionParallelRootNotInMachines, got {other:?}"),
        }
    }

    #[test]
    fn rule12_non_host_claimant_fires_non_host() {
        // Author adds a brake_default partition with an invoke (so
        // coverage passes) and tries to claim the parallel's root from
        // there — but brake_default hosts no region of the parallel.
        let deploy = r#"
topology:
  ecu:
    machines:
      brake: { source: brake.scxml }
partitions:
  brake_left:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: monitoring }
        - { machine: brake, region: control }
  brake_default:
    machines: [brake]
    contains:
      invokes:
        - { machine: brake, invoke: compute_force_inv }
    hosts_parallel_roots:
      - { machine: brake, parallel: root }
"#;
        let cfg = parse_deploy_str(deploy).expect("deploy must parse");
        let mut models = BTreeMap::new();
        models.insert("brake".to_string(), parse_scxml("brake", REGION_WITH_INVOKE));
        let err = validate_partitions_against_models(&cfg, &models)
            .expect_err("non-host claimant must fail");
        match err {
            DeployError::PartitionParallelRootNonHost {
                partition,
                machine,
                parallel,
            } => {
                assert_eq!(partition, "brake_default");
                assert_eq!(machine, "brake");
                assert_eq!(parallel, "root");
            }
            other => panic!("expected PartitionParallelRootNonHost, got {other:?}"),
        }
    }

    #[test]
    fn rule12_barrier_timeout_without_root_fires_without_root() {
        // Partition has barrier_timeout_ms but no hosts_parallel_roots.
        let deploy = r#"
topology:
  ecu:
    machines:
      brake: { source: brake.scxml }
partitions:
  brake_left:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: left }
    barrier_timeout_ms: 5000
  brake_right:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: right }
    hosts_parallel_roots:
      - { machine: brake, parallel: root }
"#;
        let cfg = parse_deploy_str(deploy).expect("deploy must parse");
        let mut models = BTreeMap::new();
        models.insert("brake".to_string(), parse_scxml("brake", TWO_REGION_PARALLEL));
        let err = validate_partitions_against_models(&cfg, &models)
            .expect_err("barrier_timeout without hosts_parallel_roots must fail");
        match err {
            DeployError::PartitionBarrierTimeoutWithoutRoot { partition, value } => {
                assert_eq!(partition, "brake_left");
                assert_eq!(value, 5000);
            }
            other => panic!("expected PartitionBarrierTimeoutWithoutRoot, got {other:?}"),
        }
    }

    #[test]
    fn rule12_wire21_custom_tcp_on_root_fires_unimplemented() {
        // Distributed parallel where the Root partition declares
        // `transport_binding: custom_tcp`. The §16.5 wire-21 emitter
        // is shm-only — accepting custom_tcp here would compile to
        // a shm channel the runtime never opens. Validator must
        // reject at deploy time so the gap surfaces here instead of
        // as a delayed missing-callback throw at SM step time.
        let deploy = r#"
topology:
  ecu:
    machines:
      brake: { source: brake.scxml }
partitions:
  brake_left:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: left }
    transport_binding: custom_tcp
    hosts_parallel_roots:
      - { machine: brake, parallel: root }
  brake_right:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: right }
"#;
        let cfg = parse_deploy_str(deploy).expect("deploy must parse");
        let mut models = BTreeMap::new();
        models.insert("brake".to_string(), parse_scxml("brake", TWO_REGION_PARALLEL));
        let err = validate_partitions_against_models(&cfg, &models)
            .expect_err("custom_tcp on a wire-21 route must fail");
        match err {
            DeployError::PartitionWire21CustomTcpUnimplemented {
                partition,
                machine,
                parallel,
            } => {
                assert_eq!(partition, "brake_left");
                assert_eq!(machine, "brake");
                assert_eq!(parallel, "root");
            }
            other => panic!("expected PartitionWire21CustomTcpUnimplemented, got {other:?}"),
        }
    }

    #[test]
    fn rule12_wire21_custom_tcp_on_nonroot_fires_unimplemented() {
        // Same gap on the NonRoot side. `BTreeSet` iteration over
        // `hosting` is alphabetic, so `brake_left` (NonRoot, custom_tcp)
        // is reported before `brake_right` (Root, default shm) —
        // deterministic across parses regardless of yaml key order.
        let deploy = r#"
topology:
  ecu:
    machines:
      brake: { source: brake.scxml }
partitions:
  brake_left:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: left }
    transport_binding: custom_tcp
  brake_right:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: right }
    hosts_parallel_roots:
      - { machine: brake, parallel: root }
"#;
        let cfg = parse_deploy_str(deploy).expect("deploy must parse");
        let mut models = BTreeMap::new();
        models.insert("brake".to_string(), parse_scxml("brake", TWO_REGION_PARALLEL));
        let err = validate_partitions_against_models(&cfg, &models)
            .expect_err("custom_tcp on a NonRoot wire-21 route must fail");
        match err {
            DeployError::PartitionWire21CustomTcpUnimplemented {
                partition,
                machine,
                parallel,
            } => {
                assert_eq!(partition, "brake_left");
                assert_eq!(machine, "brake");
                assert_eq!(parallel, "root");
            }
            other => panic!("expected PartitionWire21CustomTcpUnimplemented, got {other:?}"),
        }
    }

    #[test]
    fn rule12_wire21_custom_tcp_on_single_partition_passes() {
        // SinglePartition parallel (all regions in one partition) has
        // no wire-21 traffic — a `transport_binding: custom_tcp`
        // setting on it must NOT trip the rejection. Validates the
        // hosting.len() < 2 short-circuit.
        let deploy = r#"
topology:
  ecu:
    machines:
      brake: { source: brake.scxml }
partitions:
  brake_all:
    machines: [brake]
    contains:
      parallel_regions:
        - { machine: brake, region: left }
        - { machine: brake, region: right }
    transport_binding: custom_tcp
"#;
        let cfg = parse_deploy_str(deploy).expect("deploy must parse");
        let mut models = BTreeMap::new();
        models.insert("brake".to_string(), parse_scxml("brake", TWO_REGION_PARALLEL));
        validate_partitions_against_models(&cfg, &models)
            .expect("single-partition parallel with custom_tcp must pass — no wire-21 traffic");
    }
}
