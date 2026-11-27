// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The formal judges' wire surfaces (docs/adr/0146, D5) held to their
// instances: this repository's models, proofs and traces manifests, the
// consumer fixture's, and the pinned toolchain, each against its schema, and
// each schema shown refusing what the judge refuses.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent dir")
        .to_path_buf()
}

fn compile(rel: &str) -> jsonschema::JSONSchema {
    let text = std::fs::read_to_string(repo_root().join(rel))
        .unwrap_or_else(|e| panic!("read {rel}: {e}"));
    let schema: serde_json::Value = serde_json::from_str(&text).expect("the schema is JSON");
    jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(&schema)
        .expect("the schema compiles as draft-07")
}

fn instance(rel: &str) -> serde_json::Value {
    let text = std::fs::read_to_string(repo_root().join(rel))
        .unwrap_or_else(|e| panic!("read {rel}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{rel} is not JSON: {e}"))
}

fn errors(validator: &jsonschema::JSONSchema, value: &serde_json::Value) -> Vec<String> {
    match validator.validate(value) {
        Ok(()) => Vec::new(),
        Err(errors) => errors
            .map(|e| format!("{e} at {}", e.instance_path))
            .collect(),
    }
}

const MODELS_SCHEMA: &str = "schemas/sce-formal-models.v1.schema.json";
const TOOLCHAIN_SCHEMA: &str = "schemas/sce-formal-toolchain.v1.schema.json";

/// Every models manifest of this tree: this repository's and the consumer
/// fixture's (docs/adr/0146, D6).
const MODELS_MANIFESTS: &[&str] = &[
    "docs/formal/models.json",
    "sce-build/tests/fixtures/formal_consumer/formal/models.json",
];

#[test]
fn this_repositorys_formal_manifest_validates_against_the_wire_schema() {
    let schema = compile(MODELS_SCHEMA);
    for rel in MODELS_MANIFESTS {
        let found = errors(&schema, &instance(rel));
        assert!(
            found.is_empty(),
            "{rel} is not a models manifest:\n{}",
            found.join("\n")
        );
    }
}

#[test]
fn the_models_schema_rejects_what_the_judge_refuses() {
    let schema = compile(MODELS_SCHEMA);
    let whole = instance("docs/formal/models.json");
    let refused = |what: &str, edit: &dyn Fn(&mut serde_json::Value)| {
        let mut value = whole.clone();
        edit(&mut value);
        assert!(
            !errors(&schema, &value).is_empty(),
            "the schema accepts a manifest with {what}"
        );
    };
    refused("a tool pinned in it", &|v| {
        v["tlc"] = serde_json::json!({"version": "1.7.4"})
    });
    refused("another version", &|v| v["v"] = serde_json::json!(2));
    refused("no version", &|v| {
        v.as_object_mut().expect("an object").remove("v");
    });
    refused("an unknown key of a model", &|v| {
        v["models"][0]["owner"] = serde_json::json!("x")
    });
    refused("an unknown key of a run", &|v| {
        v["models"][0]["runs"][0]["note"] = serde_json::json!("x")
    });
    refused("a run that expects neither", &|v| {
        v["models"][0]["runs"][0]["expect"] = serde_json::json!("passes")
    });
    refused("a count that is not a number", &|v| {
        v["models"][0]["runs"][0]["states"] = serde_json::json!("7")
    });
}

const PROOFS_SCHEMA: &str = "schemas/sce-formal-proofs.v1.schema.json";

#[test]
fn this_repositorys_proofs_manifest_validates_against_the_wire_schema() {
    let schema = compile(PROOFS_SCHEMA);
    let found = errors(&schema, &instance("docs/formal/proofs.json"));
    assert!(
        found.is_empty(),
        "docs/formal/proofs.json is not a proofs manifest:\n{}",
        found.join("\n")
    );
}

#[test]
fn the_proofs_schema_rejects_what_the_judge_refuses() {
    let schema = compile(PROOFS_SCHEMA);
    let whole = instance("docs/formal/proofs.json");
    let refused = |what: &str, edit: &dyn Fn(&mut serde_json::Value)| {
        let mut value = whole.clone();
        edit(&mut value);
        assert!(
            !errors(&schema, &value).is_empty(),
            "the schema accepts a manifest with {what}"
        );
    };
    refused("Verus pinned in it", &|v| {
        v["verus"] = serde_json::json!({"version": "0"})
    });
    refused("another version", &|v| v["v"] = serde_json::json!(2));
    refused("a proof with no mutant", &|v| {
        v["proofs"][0]["mutants"] = serde_json::json!([])
    });
    refused("a mutant of no text", &|v| {
        v["proofs"][0]["mutants"][0]["from"] = serde_json::json!("")
    });
    refused("a proof that names no module compiling it", &|v| {
        v["proofs"][0]
            .as_object_mut()
            .expect("an object")
            .remove("compiled_by");
    });
}

const TRACES_SCHEMA: &str = "schemas/sce-formal-traces.v1.schema.json";

#[test]
fn this_repositorys_traces_manifest_validates_against_the_wire_schema() {
    let schema = compile(TRACES_SCHEMA);
    let found = errors(&schema, &instance("docs/formal/traces.json"));
    assert!(
        found.is_empty(),
        "docs/formal/traces.json is not a traces manifest:\n{}",
        found.join("\n")
    );
}

#[test]
fn the_traces_schema_rejects_what_the_judge_refuses() {
    let schema = compile(TRACES_SCHEMA);
    let whole = instance("docs/formal/traces.json");
    assert!(
        errors(&schema, &whole).is_empty(),
        "the control: the manifest as it is validates"
    );
    let refused = |what: &str, edit: &dyn Fn(&mut serde_json::Value)| {
        let mut value = whole.clone();
        edit(&mut value);
        assert!(
            !errors(&schema, &value).is_empty(),
            "the schema accepts a manifest with {what}"
        );
    };
    refused("TLC pinned in it", &|v| {
        v["tlc"] = serde_json::json!({"version": "1.7.4"})
    });
    refused("another version", &|v| v["v"] = serde_json::json!(2));
    refused("no run", &|v| v["traces"] = serde_json::json!([]));
    refused("a run with no altered run", &|v| {
        v["traces"][0]["altered"] = serde_json::json!([])
    });
    refused("a run that names no configuration", &|v| {
        v["traces"][0]
            .as_object_mut()
            .expect("an object")
            .remove("config");
    });
    refused("an unknown key of a run", &|v| {
        v["traces"][0]["note"] = serde_json::json!("x")
    });
    refused("an alteration of a kind the format has not", &|v| {
        v["traces"][0]["altered"][0]["kind"] = serde_json::json!("pause")
    });
    refused("an alteration of a field the format has not", &|v| {
        v["traces"][0]["altered"][0]["field"] = serde_json::json!("datamodel")
    });
    refused("an alteration at no place", &|v| {
        v["traces"][0]["altered"][0]["step"] = serde_json::json!(-1)
    });
}

#[test]
fn the_pinned_toolchain_validates_against_the_wire_schema() {
    let schema = compile(TOOLCHAIN_SCHEMA);
    let found = errors(&schema, &instance("sce-build/src/formal/toolchain.json"));
    assert!(
        found.is_empty(),
        "the toolchain is not of its schema:\n{}",
        found.join("\n")
    );
}

#[test]
fn the_toolchain_schema_rejects_what_the_judge_refuses() {
    let schema = compile(TOOLCHAIN_SCHEMA);
    let whole = instance("sce-build/src/formal/toolchain.json");
    let refused = |what: &str, edit: &dyn Fn(&mut serde_json::Value)| {
        let mut value = whole.clone();
        edit(&mut value);
        assert!(
            !errors(&schema, &value).is_empty(),
            "the schema accepts a toolchain with {what}"
        );
    };
    refused("a digest in capitals", &|v| {
        v["tlc"]["sha256"] = serde_json::json!("A".repeat(64))
    });
    refused("a fetch over http", &|v| {
        v["tlc"]["url"] = serde_json::json!("http://example.invalid/t.jar")
    });
    refused("Verus without its Rust release", &|v| {
        v["verus"]
            .as_object_mut()
            .expect("an object")
            .remove("toolchain");
    });
    refused("a tool the judge does not run", &|v| {
        v["apalache"] = serde_json::json!({})
    });
}
