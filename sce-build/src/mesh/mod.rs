// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh: Distributed SCXML transport codegen pipeline.
//
// Reads deploy.yaml topology configuration and generates transport-native
// routing code alongside the existing SM codegen output. See SCE_MESH.md.
//
// Pipeline stages:
//   Deploy   → stage 1 (deploy.yaml parsing in deploy.rs)
//   Topology → stage 2 (SCXML <send> target collection + binding resolution)
//   Codegen  → stage 3 (transport template selection + rendering)

pub mod codegen;
pub mod deploy;
pub mod distributability;
pub mod error;
pub mod external;
pub mod partitions;
pub mod pattern;
pub mod someip_format;
pub mod target;
pub mod topology;
pub mod transport;
pub mod vsomeip_config;

use std::path::Path;

/// Resolved view of a parsed `deploy.yaml` after the SCE_MESH.md
/// §16.3 distributability analyzer has either accepted the author's
/// `partitions:` plan unchanged or produced a §16.4 auto-merge.
///
/// Downstream consumers (`inject_partition_context_for`,
/// `compile_mesh_transport`) read `resolved_partitions()` rather than
/// `original.partitions` so that a permissive-mode merge surfaces at
/// every codegen site that cares about partition membership.
/// Strict-mode violations never reach this struct — the analyzer
/// returns `Err(MeshError::Deploy(_))` before the resolver completes.
///
/// The lifetime binds to the parsed [`deploy::DeployConfig`] so
/// callers that already hold the config do not pay a clone — the
/// wrapper adds only the post-merge partition map and the notice
/// vectors.
pub struct ResolvedDeployConfig<'a> {
    /// The author-parsed config, preserved byte-for-byte. Emit-site
    /// diagnostics that want to quote the author's `partitions:`
    /// text (instead of the post-merge version) reach for this.
    pub original: &'a deploy::DeployConfig,
    /// Analyzer output — [`distributability::ResolvedPartitionPlan`]
    /// carries the merged partition map and R2/R3 notice streams.
    /// Empty vectors mean "analyzer accepted unchanged".
    pub plan: distributability::ResolvedPartitionPlan,
}

impl<'a> ResolvedDeployConfig<'a> {
    /// The partition map that downstream codegen should consume —
    /// the merged plan if `original.partitions` is set, else `None`.
    /// Mirrors `deploy::DeployConfig::partitions` so call-sites swap
    /// with minimal change.
    pub fn partitions(&self) -> Option<&deploy::PartitionMap> {
        if self.original.partitions.is_none() {
            None
        } else {
            Some(&self.plan.resolved)
        }
    }

    /// §16.4 merge notices recorded while collapsing partitions.
    /// Consumers that want to surface the notice stream (CLI
    /// `--verbose` mode, IDE tooling) iterate this slice.
    pub fn merge_notices(&self) -> &[distributability::MergeNotice] {
        &self.plan.merge_notices
    }

    /// §16.3 R3 snapshot-read notices. Advisory only — the analyzer
    /// never rejects a build on R3 grounds; this surface is for
    /// author-facing warning channels.
    pub fn snapshot_notices(&self) -> &[distributability::SnapshotNotice] {
        &self.plan.snapshot_notices
    }
}

/// Run SCE_MESH.md §14 coverage rules (rules 1/2/11) *and* §16.3
/// distributability (R1-R4) against a parsed deploy config, producing
/// a [`ResolvedDeployConfig`]. Strict-mode R1/R2 violations surface
/// here as the first [`error::DeployError`] the analyzer emits;
/// permissive mode merges silently and records
/// [`distributability::MergeNotice`] entries on the returned plan.
///
/// Called from both [`crate::inject_partition_context_for`] and
/// [`crate::compile_mesh_transport`] so every codegen site sees the
/// same post-merge partition map.
pub fn resolve_deploy_config<'a>(
    deploy_dir: &Path,
    cfg: &'a deploy::DeployConfig,
) -> Result<ResolvedDeployConfig<'a>, error::MeshError> {
    // Coverage rules first — a shape failure there makes R1/R2
    // analysis moot (the partition graph the analyzer would check is
    // already incoherent).
    partitions::validate_partitions_against_scxml(deploy_dir, cfg)?;

    // §16.3 analyzer needs all partition-listed machines' models.
    // For non-partitioned deploys this is an empty set and the
    // analyzer short-circuits to `Ok(unchanged)` — zero cost.
    let listed = partitions::partition_listed_machines(cfg);
    let models = if listed.is_empty() || cfg.partitions.is_none() {
        std::collections::BTreeMap::new()
    } else {
        partitions::load_partition_machine_models(deploy_dir, cfg, &listed)?
    };

    let plan = distributability::analyze_distributability(cfg, &models).map_err(|errs| {
        // Strict-mode: surface the first diagnostic. The others are
        // unreachable through today's `MeshError` channel (which
        // carries one error at a time); multi-error rendering is a
        // CLI concern handled upstream via the explicit
        // `to_diagnostics` path when it lands.
        error::MeshError::Deploy(errs.into_iter().next().expect("non-empty strict-mode err"))
    })?;

    Ok(ResolvedDeployConfig {
        original: cfg,
        plan,
    })
}
