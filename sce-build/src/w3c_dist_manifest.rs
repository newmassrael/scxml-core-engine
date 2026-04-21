// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §16.8.2 W3C IRP distributed conformance manifest parser.
//
// `tests/w3c_distributed_manifest.yaml` classifies every W3C IRP test
// by its distribution axis (`yes` / `merged_single_partition` / `no` /
// `forbidden`) and — for `yes` entries — lists the partition plan the
// harness drives. The CI acid-test bucket is the set of `yes` entries.
// §16.8.2 closes the label enum at four values so the report cannot
// silently conflate an analyzer-merged test with a genuinely
// distributed one.
//
// This module is intentionally minimal for the §16.8 seed landing:
// `distributable` + `partitions` (with per-partition `parallel_regions`).
// The §16.8.2 spec lists additional author-visible fields
// (`transport_override`, `inferred_constraints`, `effective_partitions`,
// `reason`, `notes`) that the seed has no consumer for; adding them now
// would be dead shape per `feedback_built_but_unconsumed` /
// `feedback_verify_before_ship`. They land in the same commit as their
// consumer (driver flag, CI report renderer, analyzer cross-check)
// when S5+ work introduces that consumer.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// §16.8.2 distributability classification. The enum is closed at
/// four values so the CI report cannot conflate an analyzer-merged
/// test with a genuinely distributed one.
///
/// YAML literal forms follow the spec table exactly: lowercase
/// `yes` / `merged_single_partition` / `no` / `forbidden`.
/// `serde_yaml_ng` parses YAML 1.2 core schema, so `yes` and `no`
/// remain strings (not booleans as in YAML 1.1) and reach this
/// enum without a string/bool conversion layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Distributable {
    /// Author partition plan produces 2+ effective partitions. Runs
    /// N ≥ 2 OS processes. Counts toward the §16.8 acid-test bucket.
    Yes,
    /// Analyzer ran (applied R1/R2) but the result is 1 effective
    /// partition. Single process; distributed mode adds no new
    /// signal. Reported separately as "analyzer-exercised".
    MergedSinglePartition,
    /// No `<parallel>` and no `<invoke>` — no orthogonal split axis
    /// exists. Single-process only.
    No,
    /// Author partition plan violates R1/R2 under strict mode. Does
    /// not compile with distribution. CI fails if a `yes` test
    /// regresses to `forbidden`.
    Forbidden,
}

/// Per-partition `contains:` block. Mirrors the
/// [`crate::mesh::deploy::PartitionContains`] shape but carries only
/// `parallel_regions:` — the seed has no `<invoke>` tests, so
/// `invokes:` is absent; the field returns when a §9.6 remote-invoke
/// test enters the manifest.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartitionContains {
    #[serde(default)]
    pub parallel_regions: Vec<ParallelRegion>,
}

/// Region identity inside a partition. Manifest-level form omits
/// `machine:` because the IRP harness drives exactly one machine per
/// test (the test's own top-level `<scxml>`); the deploy.yaml that
/// sce-codegen consumes still carries the redundant `machine:` field
/// because the `PartitionMap` type in `mesh::deploy` is shared with
/// multi-machine deploys.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParallelRegion {
    pub region: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartitionEntry {
    pub contains: PartitionContains,
}

/// One entry in the `tests:` map. Represents a single W3C IRP test's
/// §16.8.2 classification.
///
/// `partitions:` is required when `distributable: yes` and omitted
/// otherwise — validated in [`ManifestFile::validate`] rather than at
/// the serde layer because the cross-field rule needs both
/// sides visible.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestEntry {
    pub distributable: Distributable,
    #[serde(default)]
    pub partitions: Option<BTreeMap<String, PartitionEntry>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestFile {
    pub tests: BTreeMap<String, ManifestEntry>,
}

impl ManifestFile {
    /// Parse the manifest yaml from its on-disk path. Returns a
    /// `Result<_, String>` so call sites (cargo unit tests, the
    /// eventual CMake integration) can propagate a human-readable
    /// error without depending on serde types.
    pub fn parse_file(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
        Self::parse_str(&text)
    }

    pub fn parse_str(text: &str) -> Result<Self, String> {
        let parsed: Self = serde_yaml_ng::from_str(text)
            .map_err(|e| format!("manifest parse error: {}", e))?;
        parsed.validate()?;
        Ok(parsed)
    }

    /// Cross-field validation: a `yes` entry must declare a
    /// `partitions:` map with at least two partitions (the
    /// N ≥ 2 OS processes premise in §16.8.2); a non-`yes` entry
    /// must not declare `partitions:` because the harness never
    /// reads it for those classifications.
    fn validate(&self) -> Result<(), String> {
        for (test_id, entry) in &self.tests {
            match entry.distributable {
                Distributable::Yes => {
                    let partitions = entry.partitions.as_ref().ok_or_else(|| {
                        format!(
                            "test {}: distributable=yes requires a partitions: map (§16.8.2)",
                            test_id
                        )
                    })?;
                    if partitions.len() < 2 {
                        return Err(format!(
                            "test {}: distributable=yes requires ≥2 partitions, got {} (§16.8.2)",
                            test_id,
                            partitions.len()
                        ));
                    }
                }
                Distributable::MergedSinglePartition
                | Distributable::No
                | Distributable::Forbidden => {
                    if entry.partitions.is_some() {
                        return Err(format!(
                            "test {}: distributable={:?} must not declare partitions: — the harness would never read it (§16.8.2)",
                            test_id, entry.distributable
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// Partition names for a `yes` test in sort order (BTreeMap
    /// iteration). Used by the CMake macro's eventual manifest
    /// integration and by the seed harness assertion that the
    /// manifest's `partitions:` keys match the seed's deploy.yaml
    /// partition names.
    pub fn partition_names(&self, test_id: &str) -> Option<Vec<String>> {
        let entry = self.tests.get(test_id)?;
        let partitions = entry.partitions.as_ref()?;
        Some(partitions.keys().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        // CARGO_MANIFEST_DIR is sce-build/; the workspace root (where
        // tests/w3c_distributed_manifest.yaml lives) is its parent.
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("sce-build parent")
            .to_path_buf()
    }

    #[test]
    fn checked_in_manifest_parses() {
        let path = repo_root().join("tests/w3c_distributed_manifest.yaml");
        let manifest = ManifestFile::parse_file(&path)
            .expect("tests/w3c_distributed_manifest.yaml must parse");
        assert!(
            manifest.tests.contains_key("test417"),
            "§16.8 seed test417 must be present in the manifest"
        );
    }

    /// Load-bearing consumer check per `feedback_built_but_unconsumed`:
    /// the parser's output must agree with the seed's deploy.yaml on
    /// partition names. A drift in either file surfaces here rather
    /// than as a runtime mismatch at partition_runner dispatch time.
    #[test]
    fn seed_manifest_partitions_match_deploy_yaml() {
        let manifest_path = repo_root().join("tests/w3c_distributed_manifest.yaml");
        let manifest = ManifestFile::parse_file(&manifest_path).unwrap();
        let from_manifest = manifest
            .partition_names("test417")
            .expect("test417 is a yes entry");
        assert_eq!(
            from_manifest,
            vec!["test417_main".to_string(), "test417_worker".to_string()],
            "manifest partition names must match the seed deploy.yaml"
        );

        let deploy_path = repo_root().join("tests/w3c/dist/test417/deploy.yaml");
        let deploy_text = std::fs::read_to_string(&deploy_path).unwrap();
        for name in &from_manifest {
            assert!(
                deploy_text.contains(&format!("{}:", name)),
                "deploy.yaml must declare partition `{}`",
                name
            );
        }

        let entry = manifest.tests.get("test417").unwrap();
        let partitions = entry.partitions.as_ref().unwrap();
        let main_regions: Vec<_> = partitions["test417_main"]
            .contains
            .parallel_regions
            .iter()
            .map(|r| r.region.as_str())
            .collect();
        let worker_regions: Vec<_> = partitions["test417_worker"]
            .contains
            .parallel_regions
            .iter()
            .map(|r| r.region.as_str())
            .collect();
        assert_eq!(main_regions, vec!["s1p11"]);
        assert_eq!(worker_regions, vec!["s1p12"]);
    }

    #[test]
    fn distributable_yes_requires_partitions() {
        let yaml = r#"
tests:
  testX:
    distributable: yes
"#;
        let err = ManifestFile::parse_str(yaml).unwrap_err();
        assert!(err.contains("requires a partitions: map"), "got: {err}");
    }

    #[test]
    fn distributable_yes_requires_at_least_two_partitions() {
        let yaml = r#"
tests:
  testX:
    distributable: yes
    partitions:
      only_one:
        contains:
          parallel_regions:
            - { region: r1 }
"#;
        let err = ManifestFile::parse_str(yaml).unwrap_err();
        assert!(err.contains("≥2 partitions"), "got: {err}");
    }

    #[test]
    fn non_yes_rejects_partitions_field() {
        let yaml = r#"
tests:
  testX:
    distributable: no
    partitions:
      p:
        contains:
          parallel_regions: []
"#;
        let err = ManifestFile::parse_str(yaml).unwrap_err();
        assert!(err.contains("must not declare partitions"), "got: {err}");
    }

    #[test]
    fn all_four_labels_round_trip() {
        let yaml = r#"
tests:
  a:
    distributable: yes
    partitions:
      p1: { contains: { parallel_regions: [{ region: r1 }] } }
      p2: { contains: { parallel_regions: [{ region: r2 }] } }
  b:
    distributable: merged_single_partition
  c:
    distributable: no
  d:
    distributable: forbidden
"#;
        let manifest = ManifestFile::parse_str(yaml).unwrap();
        assert_eq!(manifest.tests["a"].distributable, Distributable::Yes);
        assert_eq!(
            manifest.tests["b"].distributable,
            Distributable::MergedSinglePartition
        );
        assert_eq!(manifest.tests["c"].distributable, Distributable::No);
        assert_eq!(
            manifest.tests["d"].distributable,
            Distributable::Forbidden
        );
    }

    #[test]
    fn unknown_top_level_field_rejected() {
        let yaml = r#"
tests:
  testX:
    distributable: no
    notes: this should be rejected
"#;
        let err = ManifestFile::parse_str(yaml).unwrap_err();
        assert!(err.contains("unknown field"), "got: {err}");
    }
}
