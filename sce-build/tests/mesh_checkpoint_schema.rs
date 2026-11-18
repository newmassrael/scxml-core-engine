// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The Mesh checkpoint wire surface (`schemas/sce-mesh-checkpoint.v1.schema.json`,
// SCE_MESH.md section 10.11) held to its instances.
//
// The shared instances under `tests/fixtures/mesh_checkpoint/` are texts the
// Rust host core reads and writes back byte for byte
// (`the_shared_instance_is_what_this_core_writes` in
// `backends/rust/mesh/src/checkpoint.rs`), so what this test accepts is what the
// producer writes. Each is held to the checkpoint schema, and the saved tree it
// holds to the schema of format 4, the saved state every backend writes. The directory is swept, so an
// instance added later is held without an edit here.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent dir")
        .to_path_buf()
}

fn compile(text: &str) -> jsonschema::JSONSchema {
    let schema: serde_json::Value = serde_json::from_str(text).expect("the schema is JSON");
    jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(&schema)
        .expect("the schema compiles as draft-07")
}

fn checkpoint_schema() -> jsonschema::JSONSchema {
    compile(include_str!(
        "../../schemas/sce-mesh-checkpoint.v1.schema.json"
    ))
}

fn saved_state_schema() -> jsonschema::JSONSchema {
    compile(include_str!("../../schemas/sce-saved-state.v4.schema.json"))
}

fn errors(validator: &jsonschema::JSONSchema, instance: &serde_json::Value) -> Vec<String> {
    match validator.validate(instance) {
        Ok(()) => Vec::new(),
        Err(errors) => errors
            .map(|e| format!("{e} at {}", e.instance_path))
            .collect(),
    }
}

/// The shared instances, sorted.
fn instances() -> Vec<(PathBuf, serde_json::Value)> {
    let dir = repo_root().join("sce-build/tests/fixtures/mesh_checkpoint");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .map(|entry| entry.expect("directory entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "{} holds no instance", dir.display());
    paths
        .into_iter()
        .map(|path| {
            let text = std::fs::read_to_string(&path).expect("read instance");
            let value = serde_json::from_str(&text)
                .unwrap_or_else(|e| panic!("{} is not JSON: {e}", path.display()));
            (path, value)
        })
        .collect()
}

#[test]
fn every_shared_checkpoint_is_a_checkpoint() {
    let checkpoint = checkpoint_schema();
    let saved = saved_state_schema();
    for (path, instance) in instances() {
        let found = errors(&checkpoint, &instance);
        assert!(
            found.is_empty(),
            "{} is not a checkpoint:\n{}",
            path.display(),
            found.join("\n")
        );
        let found = errors(&saved, &instance["state"]);
        assert!(
            found.is_empty(),
            "{} holds a state that is not a format-4 saved state:\n{}",
            path.display(),
            found.join("\n")
        );
    }
}

/// One way to break an instance: what it breaks, and the change that breaks it.
type Breakage = (&'static str, fn(&mut serde_json::Value));

/// Each change below breaks one rule of the surface, and the schema refuses
/// it: a positive sweep alone would only show that everything is accepted.
#[test]
fn the_checkpoint_schema_rejects_what_the_core_does_not_write() {
    let checkpoint = checkpoint_schema();
    let (_, valid) = instances().remove(0);
    let broken: [Breakage; 7] = [
        ("another format", |v| v["format"] = serde_json::json!(2)),
        ("a number as a JSON number", |v| {
            v["checkpoint"] = serde_json::json!(7)
        }),
        ("an endpoint without its machine", |v| {
            v["endpoints"][0]
                .as_object_mut()
                .expect("an endpoint is an object")
                .remove("machine");
        }),
        ("an envelope in uppercase hexadecimal", |v| {
            v["endpoints"][0]["unreleased"][0]["envelope"] = serde_json::json!("AB")
        }),
        ("an id that is not 16 bytes", |v| {
            v["endpoints"][0]["senders"][0]["window"][0] = serde_json::json!("0101")
        }),
        ("a call of no kind", |v| {
            v["endpoints"][0]["calls"][0]["call"] = serde_json::json!("raise")
        }),
        ("a member no reader knows", |v| {
            v["endpoints"][0]["peers"][0]["routing"] = serde_json::json!("x")
        }),
    ];
    for (what, breaks) in broken {
        let mut instance = valid.clone();
        breaks(&mut instance);
        assert!(
            !errors(&checkpoint, &instance).is_empty(),
            "the schema accepts {what}"
        );
    }
}
