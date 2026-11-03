// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// build.rs for the feature-conformance crate.
//
// A statechart written against a feature model is generated once for every
// build of its product line, each part the variants share written once and
// each part some write under a `#[cfg]` (docs/adr/0057 §9). The generator
// checks that output, build by build, against the machine each variant
// generates alone, by resolving the `#[cfg]` on its syntax tree. That is a
// proof about the text; it is not a compiler. Here the generator runs
// in-process, as `sce-deploy-conformance`'s does, and writes:
//
//   - `lifted`: the machine for every build, its `#[cfg]` over this crate's own
//     features, so that rustc resolves it for whichever feature set the crate
//     is built with;
//   - `variant_none`, `variant_unicast` and `variant_accept`: the machine of
//     each feature set generated alone, which carry no `#[cfg]`.
//
// The tests build the crate under each feature set and ask that `lifted` and
// the variant of that set are one machine.
//
// Every file is wrapped in a `pub mod` in one generated file, because a
// generated file opens with inner attributes (`#![allow(...)]`) that `include!`
// does not accept at a crate's top level but a module body does.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::parser::SCXMLParser;
use sce_build::variant_lift::Variants;
use sce_build::{analyzer, CompileError, ModelEmitOptions};

/// The feature sets the crate is built under, each with the module its variant
/// is generated into.
const FEATURE_SETS: &[(&str, &[&str])] = &[
    ("variant_none", &[]),
    ("variant_unicast", &["session-unicast"]),
    (
        "variant_accept",
        &["session-unicast", "session-unicast-accept"],
    ),
];

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR not set by cargo"));
    let manifest_dir = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set by cargo"),
    );
    let templates = manifest_dir
        .join("../../../tools/codegen/templates")
        .canonicalize()
        .expect("canonicalize tools/codegen/templates");
    let template_dir = templates.join(Language::Rust.template_subdir());
    let document = manifest_dir.join("product-line/session.scxml");
    let document = document.to_str().expect("a path of text");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=product-line");
    println!("cargo:rerun-if-changed={}", templates.display());
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir.join("../../../sce-build/src").display()
    );

    check_manifest(&manifest_dir.join("Cargo.toml"), document);

    let mut machines = String::new();
    push_module(
        &mut machines,
        "lifted",
        &generate(&template_dir, document, None),
    );
    for (module, features) in FEATURE_SETS {
        let features: Vec<String> = features.iter().map(|f| f.to_string()).collect();
        push_module(
            &mut machines,
            module,
            &generate(&template_dir, document, Some(features)),
        );
    }
    std::fs::write(out_dir.join("machines.rs"), machines).expect("write machines.rs");
}

/// The machine of `document`: for every build when `features` is `None`, else
/// the variant those features select, generated as any document is.
fn generate(template_dir: &Path, document: &str, features: Option<Vec<String>>) -> String {
    let parse = |features: Option<&[String]>| {
        let parser = SCXMLParser::new();
        let mut parser = match features {
            Some(features) => parser.with_features(features.to_vec()),
            None => parser,
        };
        parser
            .parse_file(document)
            .unwrap_or_else(|e| panic!("parse {document}: {e:?}"))
    };
    let mut model = parse(features.as_deref());
    analyzer::analyze(&mut model, document);
    let variants = match features {
        Some(_) => None,
        None => Variants::derive(
            &model,
            document,
            &SCXMLParser::new,
            &mut |variant| -> Result<(), CompileError> {
                analyzer::analyze(variant, document);
                Ok(())
            },
        )
        .unwrap_or_else(|e| panic!("derive the variants of {document}: {e:?}")),
    };
    let output = sce_build::emit_model_artifacts(
        &model,
        template_dir,
        "session",
        document,
        Language::Rust,
        &ModelEmitOptions {
            variants: variants.as_ref(),
            ..Default::default()
        },
    )
    .unwrap_or_else(|e| panic!("generate {document}: {e:?}"));
    let [(_, machine)] = output.files.as_slice() else {
        panic!(
            "a statechart writes one Rust file, this one wrote {}",
            output.files.len()
        );
    };
    machine.clone()
}

/// Refuse a manifest whose `[features]` table is not the one the feature model
/// of `document` states (docs/adr/0057 §1): the model is the one source of the
/// product line's features and of what each requires, and the manifest is
/// where Cargo reads them. The table's entries are compared line for line,
/// comments and blank lines aside, and a refusal prints the table to write.
fn check_manifest(manifest: &Path, document: &str) {
    let model = SCXMLParser::new()
        .parse_file(document)
        .unwrap_or_else(|e| panic!("parse {document}: {e:?}"));
    let line = model
        .product_line
        .as_ref()
        .unwrap_or_else(|| panic!("{document} is written against no feature model"));
    let stated = line.model().cargo_features();
    let text = std::fs::read_to_string(manifest)
        .unwrap_or_else(|e| panic!("read {}: {e}", manifest.display()));
    let written: Vec<&str> = text
        .lines()
        .map(str::trim)
        .skip_while(|row| *row != "[features]")
        .skip(1)
        .take_while(|row| !row.starts_with('['))
        .filter(|row| !row.is_empty() && !row.starts_with('#'))
        .collect();
    assert!(
        written == stated,
        "the [features] of {} are not those the feature model `{}` states; write:\n\n[features]\n{}\n",
        manifest.display(),
        line.model_name(),
        stated.join("\n")
    );
}

fn push_module(out: &mut String, name: &str, body: &str) {
    out.push_str(&format!("pub mod {name} {{\n{body}\n}}\n"));
}
