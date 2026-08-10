// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Cross-surface stability registry guard.
//
// `SCE_WIRE_CONTRACTS.md` is the single registry declaring the
// stability status of every agent-facing wire surface. Per the
// doc-as-contract rule, the registry's claims must be machine-anchored
// so it cannot silently go stale:
//
//   1. Every wire surface declares a valid `x-sce-schema-status`
//      (`pre-release` or `stable`) in its schema-file header — JSON
//      surfaces in the `x-sce-schema-status` field, XSD surfaces in the
//      first `<xs:documentation>` annotation.
//   2. The registry lists every surface (keyed by its schema filename),
//      so dropping a surface from the registry fails this test.
//   3. Every schema file checked into `schemas/` and `apis/` appears in
//      the surface lists above. Without this direction the lists are
//      self-certifying: a schema can land on disk, be consumed by an
//      external tool, and never acquire a status header or a registry
//      row, because nothing walks from the tree back to the lists.
//
// Per-surface producer-const ↔ schema-header lockstep is guarded
// separately next to each producer (diagnostic.rs, ast_export.rs,
// sourcemap.rs); this test owns the cross-surface invariant only.

use std::path::{Path, PathBuf};

/// Repo root = the crate manifest dir's parent (`sce-build/..`).
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent dir")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    let p = repo_root().join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

const JSON_SURFACES: &[&str] = &[
    "schemas/sce-diagnostic.v1.schema.json",
    "apis/forge-ast.v1.schema.json",
    "schemas/sce-sourcemap.v1.schema.json",
    "schemas/sce-manifest.v1.schema.json",
    "schemas/sce-symbol-lookup.v1.schema.json",
];

const XSD_SURFACES: &[&str] = &["schemas/sce-forge.xsd", "schemas/sce-forge-ext.xsd"];

fn is_valid_status(s: &str) -> bool {
    matches!(s, "pre-release" | "stable")
}

#[test]
fn every_json_surface_declares_valid_status() {
    for rel in JSON_SURFACES {
        let parsed: serde_json::Value =
            serde_json::from_str(&read(rel)).unwrap_or_else(|e| panic!("{rel} invalid JSON: {e}"));
        let status = parsed
            .get("x-sce-schema-status")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| {
                panic!(
                    "{rel} must declare top-level x-sce-schema-status — see SCE_WIRE_CONTRACTS.md"
                )
            });
        assert!(
            is_valid_status(status),
            "{rel}: x-sce-schema-status must be 'pre-release' or 'stable'; got {status:?}",
        );
    }
}

#[test]
fn every_xsd_surface_declares_valid_status() {
    for rel in XSD_SURFACES {
        let body = read(rel);
        let marker = "x-sce-schema-status:";
        let idx = body.find(marker).unwrap_or_else(|| {
            panic!(
                "{rel} must carry an x-sce-schema-status: annotation — see SCE_WIRE_CONTRACTS.md"
            )
        });
        let after = body[idx + marker.len()..].trim_start();
        let status: String = after
            .chars()
            .take_while(|c| !c.is_whitespace() && *c != '<')
            .collect();
        assert!(
            is_valid_status(&status),
            "{rel}: x-sce-schema-status must be 'pre-release' or 'stable'; got {status:?}",
        );
    }
}

/// Directories that hold checked-in wire schemas. A schema landing in
/// either one is a wire surface by construction — it exists to be read
/// by something outside this crate.
const SCHEMA_DIRS: &[&str] = &["schemas", "apis"];

/// Lower bound on the surfaces the reverse walk must observe. A scan
/// that reads zero files passes every per-file assertion vacuously, so
/// the count is asserted alongside them: the guard must fail when the
/// walk stops finding the tree rather than quietly certifying nothing.
const MIN_DISCOVERED_SURFACES: usize = 7;

/// Every schema file on disk is a declared surface.
///
/// The forward direction (`registry_lists_every_surface`) walks the
/// hardcoded lists and cannot observe a file the lists never named.
/// This walks the tree instead, so a new schema is a build failure
/// until it is classified, given a status header, and registered.
#[test]
fn every_checked_in_schema_is_a_declared_surface() {
    let mut discovered = 0usize;
    for dir in SCHEMA_DIRS {
        let abs = repo_root().join(dir);
        let entries =
            std::fs::read_dir(&abs).unwrap_or_else(|e| panic!("read_dir {}: {e}", abs.display()));
        for entry in entries {
            let path = entry.expect("dir entry readable").path();
            let name = match path.file_name().and_then(|f| f.to_str()) {
                Some(n) => n,
                None => continue,
            };
            let is_json_schema = name.ends_with(".schema.json");
            let is_xsd = name.ends_with(".xsd");
            if !is_json_schema && !is_xsd {
                continue;
            }
            discovered += 1;
            let rel = format!("{dir}/{name}");
            let declared = if is_json_schema {
                JSON_SURFACES.contains(&rel.as_str())
            } else {
                XSD_SURFACES.contains(&rel.as_str())
            };
            assert!(
                declared,
                "{rel} is checked in but is not a declared wire surface. Add it to \
                 JSON_SURFACES / XSD_SURFACES, give it an x-sce-schema-status header, \
                 and register it in SCE_WIRE_CONTRACTS.md.",
            );
        }
    }
    assert!(
        discovered >= MIN_DISCOVERED_SURFACES,
        "reverse walk found only {discovered} schema files; expected at least \
         {MIN_DISCOVERED_SURFACES}. A walk that finds nothing certifies nothing.",
    );
}

/// Instance-validation coverage: `(surface, test fn, file declaring it)`.
///
/// A schema nothing is validated against is a document, not a
/// contract. Every other guard in this file compares a constant to a
/// header or a filename to a list — none of them puts a produced
/// artifact through the schema, so a producer could drift out of its
/// own schema with every status check still green.
///
/// This table names the test that does the validating, per surface.
/// The XSD surfaces are absent deliberately: `forge::xsd_validator`
/// validates every input document on the production path, which is
/// stronger than a test over a fixture set, and it is not a draft-07
/// validator, so it does not belong in this table's shape.
const INSTANCE_VALIDATION: &[(&str, &str, &str)] = &[
    (
        "schemas/sce-diagnostic.v1.schema.json",
        "every_golden_record_validates_against_the_wire_schema",
        "sce-build/src/forge/diagnostic.rs",
    ),
    (
        "schemas/sce-diagnostic.v1.schema.json",
        "every_cli_diagnostic_in_the_fixture_corpus_validates_against_the_schema",
        "sce-build/tests/diagnostic_corpus_schema.rs",
    ),
    (
        "apis/forge-ast.v1.schema.json",
        "round_trip_every_kind",
        "sce-build/tests/forge_ast_export.rs",
    ),
    (
        "schemas/sce-sourcemap.v1.schema.json",
        "committed_sourcemaps_validate_against_the_wire_schema",
        "sce-build/tests/committed_sourcemap_drift.rs",
    ),
    (
        "schemas/sce-manifest.v1.schema.json",
        "generate_manifest_instance_validates_against_schema",
        "sce-build/src/manifest.rs",
    ),
    (
        "schemas/sce-manifest.v1.schema.json",
        "check_manifest_validates_against_the_wire_schema",
        "sce-build/tests/cli_check.rs",
    ),
    (
        "schemas/sce-symbol-lookup.v1.schema.json",
        "both_lookup_directions_validate_against_the_wire_schema",
        "sce-build/tests/sourcemap_addr2sce.rs",
    ),
];

/// Negative-case coverage: `(surface, test fn, file declaring it)`.
///
/// `SCE_WIRE_CONTRACTS.md` policy item 5 says a positive sweep alone
/// proves only that everything is accepted — a validator that accepted
/// every input would pass one. The negative case is what shows the
/// schema refusing something.
///
/// That requirement went unenforced: the table above lists only
/// positives, and the symbol-lookup surface reached `pre-release` with
/// no negative case at all while its row here looked complete. Naming
/// the negatives in their own table makes "which surfaces can actually
/// reject" answerable instead of assumed.
///
/// Each named test must start from a record it first asserts is valid,
/// then change exactly one thing. Without that control the refusal may
/// be tripping some unrelated constraint, and the case proves nothing
/// about the field it is named after.
const NEGATIVE_VALIDATION: &[(&str, &str, &str)] = &[
    (
        "schemas/sce-diagnostic.v1.schema.json",
        "diagnostic_schema_rejects_a_missing_required_field",
        "sce-build/src/forge/diagnostic.rs",
    ),
    (
        "apis/forge-ast.v1.schema.json",
        "ast_schema_rejects_an_envelope_missing_a_required_field",
        "sce-build/tests/forge_ast_export.rs",
    ),
    (
        "schemas/sce-sourcemap.v1.schema.json",
        "sourcemap_schema_rejects_a_missing_required_field",
        "sce-build/tests/committed_sourcemap_drift.rs",
    ),
    (
        "schemas/sce-manifest.v1.schema.json",
        "schema_rejects_a_missing_required_field",
        "sce-build/src/manifest.rs",
    ),
    (
        "schemas/sce-symbol-lookup.v1.schema.json",
        "lookup_schema_rejects_a_record_without_the_generator_stamp",
        "sce-build/tests/sourcemap_addr2sce.rs",
    ),
];

/// Every JSON surface can be shown refusing something, and every test
/// the negative table names still exists where it says it does.
#[test]
fn every_json_surface_has_a_negative_validation_test() {
    for rel in JSON_SURFACES {
        assert!(
            NEGATIVE_VALIDATION
                .iter()
                .any(|(surface, _, _)| surface == rel),
            "{rel} has no negative-validation test. A positive sweep \
             alone proves only that everything is accepted — add a case \
             that mutates one field of a known-valid record and asserts \
             the schema rejects it, then register it here and in \
             SCE_WIRE_CONTRACTS.md.",
        );
    }

    for (surface, test_fn, file) in NEGATIVE_VALIDATION {
        let body = read(file);
        assert!(
            body.contains(&format!("fn {test_fn}(")),
            "{file} no longer declares `fn {test_fn}` (registered as \
             the negative-validation test for {surface}). Update the \
             NEGATIVE_VALIDATION table and SCE_WIRE_CONTRACTS.md.",
        );
    }
}

/// Every JSON surface has at least one instance-validation test, and
/// every test the table names still exists where it says it does.
///
/// Without the first half a new surface can register, declare a
/// status, and never have one artifact checked against its schema.
/// Without the second the table decays into names that moved.
#[test]
fn every_json_surface_has_an_instance_validation_test() {
    for rel in JSON_SURFACES {
        assert!(
            INSTANCE_VALIDATION
                .iter()
                .any(|(surface, _, _)| surface == rel),
            "{rel} has no instance-validation test. A surface whose \
             schema no produced artifact is checked against is a \
             document, not a contract — add a test that runs emitted \
             records through it and register the test here and in \
             SCE_WIRE_CONTRACTS.md.",
        );
    }

    for (surface, test_fn, file) in INSTANCE_VALIDATION {
        let body = read(file);
        assert!(
            body.contains(&format!("fn {test_fn}(")),
            "{file} no longer declares `fn {test_fn}` (registered as \
             the instance-validation test for {surface}). Update the \
             INSTANCE_VALIDATION table and SCE_WIRE_CONTRACTS.md.",
        );
    }
}

/// The registry names the same tests this table does.
///
/// The doc-as-contract rule cuts both ways: a reader of
/// `SCE_WIRE_CONTRACTS.md` decides how much to trust a surface partly
/// from which tests cover it, so a name that drifts out of the code
/// has to fail here rather than mislead there.
#[test]
fn registry_names_every_instance_validation_test() {
    let registry = read("SCE_WIRE_CONTRACTS.md");
    for (surface, test_fn, _) in INSTANCE_VALIDATION {
        assert!(
            registry.contains(test_fn),
            "SCE_WIRE_CONTRACTS.md must name the instance-validation \
             test for {surface}; missing {test_fn:?}",
        );
    }
    for (surface, test_fn, _) in NEGATIVE_VALIDATION {
        assert!(
            registry.contains(test_fn),
            "SCE_WIRE_CONTRACTS.md must name the negative-validation \
             test for {surface}; missing {test_fn:?}",
        );
    }
}

#[test]
fn registry_lists_every_surface() {
    let registry = read("SCE_WIRE_CONTRACTS.md");
    for rel in JSON_SURFACES.iter().chain(XSD_SURFACES.iter()) {
        let filename = Path::new(rel)
            .file_name()
            .and_then(|f| f.to_str())
            .expect("surface path has a filename");
        assert!(
            registry.contains(filename),
            "SCE_WIRE_CONTRACTS.md must list every wire surface; missing {filename:?}",
        );
    }
}
