// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// build.rs for the deploy-artifact conformance crate.
//
// The Rust files SCE writes from a deploy.yaml (a machine's LinkBus, its
// cooperative scheduler, its session runtime) were only ever compared as text. A
// template that stopped compiling would have passed every test that read them.
// Here the generator runs in-process, as `sce-forge-conformance`'s does, and the
// files it writes are compiled into this crate, one module per machine.
//
//   - `deploy.yaml` in this directory is parsed by `parse_deploy_str`, so what
//     is compiled is what the deploy parser accepts, and each tokio machine's
//     session runtime is rendered from its `scheduler:`;
//   - the LinkBus and scheduler of a cooperative machine are rendered from the
//     same numbers a deploy carries for `links:` and `scheduler:`.
//
// Every rendered file is wrapped in a `pub mod` in one generated file, because a
// generated file opens with inner attributes (`#![allow(...)]`) that `include!`
// does not accept at a crate's top level but a module body does.

use sce_build::forge::generator::{
    render_machine_concurrency_artifacts, render_machine_session_runtime_artifacts,
};
use sce_build::forge::queryables::render_machine_queryables_artifacts;
use sce_build::generator::Language;
use sce_build::mesh::deploy::parse_deploy_str;
use sce_build::mesh::queryable::resolve_queryables;
use std::fmt::Write as _;
use std::path::Path;

fn main() {
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set by cargo");
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set by cargo");
    let template_base = Path::new(&manifest_dir)
        .join("../../../tools/codegen/templates")
        .canonicalize()
        .expect("canonicalize tools/codegen/templates");
    let deploy_path = Path::new(&manifest_dir).join("deploy.yaml");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", deploy_path.display());
    println!(
        "cargo:rerun-if-changed={}",
        template_base.join("forge/rust").display()
    );

    let mut modules = String::new();

    // Session runtime, from the deploy file.
    let deploy_text = std::fs::read_to_string(&deploy_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", deploy_path.display()));
    let deploy = parse_deploy_str(&deploy_text).unwrap_or_else(|e| panic!("parse deploy: {e}"));
    for device in deploy.topology.values() {
        for (machine, config) in device.machines.iter() {
            let profile = config
                .scheduler
                .as_ref()
                .and_then(|s| s.session_runtime_profile());
            let files = render_machine_session_runtime_artifacts(
                &template_base,
                Language::Rust,
                machine,
                profile,
            )
            .unwrap_or_else(|e| panic!("render session runtime for {machine}: {e}"));
            for (name, body) in files {
                push_module(&mut modules, &name, &body);
            }
            // Queryables, from the same machine's deploy block.
            let legs = resolve_queryables(machine, &config.queryables)
                .unwrap_or_else(|e| panic!("resolve queryables for {machine}: {e}"));
            let files =
                render_machine_queryables_artifacts(&template_base, Language::Rust, machine, &legs)
                    .unwrap_or_else(|e| panic!("render queryables for {machine}: {e}"));
            for (name, body) in files {
                push_module(&mut modules, &name, &body);
            }
        }
    }

    // LinkBus and cooperative scheduler of one machine with two links.
    let links = ["udp_scout".to_string(), "udp_data".to_string()];
    let files = render_machine_concurrency_artifacts(
        &template_base,
        Language::Rust,
        "mcu_node",
        &links,
        Some(1000),
        Some(500),
    )
    .unwrap_or_else(|e| panic!("render concurrency artifacts: {e}"));
    for (name, body) in files {
        push_module(&mut modules, &name, &body);
    }

    std::fs::write(Path::new(&out_dir).join("deploy_artifacts.rs"), modules)
        .expect("write deploy_artifacts.rs");
}

/// Append `body` as `pub mod <file stem> { ... }`.
fn push_module(modules: &mut String, file_name: &str, body: &str) {
    let stem = file_name
        .strip_suffix(".rs")
        .unwrap_or_else(|| panic!("a Rust artifact name ends in .rs: {file_name}"));
    write!(modules, "pub mod {stem} {{\n{body}\n}}\n").expect("write to a String");
}
