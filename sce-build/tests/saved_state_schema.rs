// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The saved-state wire surface (`schemas/sce-saved-state.v1.schema.json`,
// SCE Accepted Subset §2.15) held to its instances.
//
// The shared fixtures under `tests/fixtures/static_datamodel/saved/` are the
// text every backend must save after the same run and must restore from —
// the Rust and Kotlin static suites each assert both. So they are the
// surface's instances, and each one must be what the schema says a saved
// state is. The directory is swept rather than listed, so a fixture added
// for a new backend or machine is held to the schema without an edit here.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent dir")
        .to_path_buf()
}

fn validator() -> jsonschema::JSONSchema {
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../../schemas/sce-saved-state.v1.schema.json"))
            .expect("the saved-state schema is JSON");
    jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(&schema)
        .expect("the saved-state schema compiles as draft-07")
}

fn errors(validator: &jsonschema::JSONSchema, instance: &serde_json::Value) -> Vec<String> {
    match validator.validate(instance) {
        Ok(()) => Vec::new(),
        Err(errors) => errors
            .map(|e| format!("{e} at {}", e.instance_path))
            .collect(),
    }
}

#[test]
fn every_shared_saved_state_fixture_is_a_saved_state() {
    let validator = validator();
    let dir = repo_root().join("sce-build/tests/fixtures/static_datamodel/saved");
    let mut fixtures: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .map(|entry| entry.expect("directory entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    fixtures.sort();
    // A sweep over nothing certifies nothing: a record and a list when this
    // floor was set.
    assert!(
        fixtures.len() >= 2,
        "swept only {} saved-state fixture(s) in {}",
        fixtures.len(),
        dir.display()
    );
    for path in fixtures {
        let text = std::fs::read_to_string(&path).expect("read fixture");
        let instance: serde_json::Value = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("{} is not JSON: {e}", path.display()));
        let found = errors(&validator, &instance);
        assert!(
            found.is_empty(),
            "{} is not a saved state:\n{}",
            path.display(),
            found.join("\n")
        );
    }
}

#[test]
fn the_schema_refuses_what_no_backend_writes() {
    // The control that keeps the sweep above from passing a schema that
    // admits anything: each instance breaks one rule a reader relies on.
    let validator = validator();
    let good = serde_json::json!({
        "format": 1,
        "shape": "0".repeat(64),
        "configuration": ["s"],
        "current": "s",
        "variables": {"count": 1},
        "external": [{
            "name": "tick", "data": "", "type": "external",
            "sendid": "", "origin": "", "origintype": "", "invokeid": ""
        }]
    });
    assert!(
        errors(&validator, &good).is_empty(),
        "{:?}",
        errors(&validator, &good)
    );

    let broken = [
        ("another format", "format", serde_json::json!(2)),
        (
            "a shape that is not a digest",
            "shape",
            serde_json::json!("abc"),
        ),
        (
            "an empty configuration",
            "configuration",
            serde_json::json!([]),
        ),
        (
            "a variable that is null",
            "variables",
            serde_json::json!({"count": null}),
        ),
        (
            "a queued event of no type",
            "external",
            serde_json::json!([{
                "name": "tick", "data": "", "type": "sideways",
                "sendid": "", "origin": "", "origintype": "", "invokeid": ""
            }]),
        ),
    ];
    for (what, key, value) in broken {
        let mut instance = good.clone();
        instance[key] = value;
        assert!(
            !errors(&validator, &instance).is_empty(),
            "the schema admitted {what}"
        );
    }
}
