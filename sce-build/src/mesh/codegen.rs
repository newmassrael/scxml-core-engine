// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh codegen dispatcher — per-target transport template rendering.
//
// Each target routes through its deploy.yaml-bound transport. Mixed transports
// are supported: e.g. #motor via local, #display via shm, #logger via someip.
// The unified template generates a single TransportRouter that dispatches
// per-target to the appropriate transport-specific send function.
//
// Adding a new transport: add conditionals in mesh_transport.h.jinja2 only.
// No Rust code changes needed — the template is the single source of truth
// for which transports are supported. Unknown transports produce a C++ #error.

use crate::filters;
use crate::generator::{GeneratedOutput, Language};
use crate::mesh::error::CodegenError;
use crate::mesh::topology::ResolvedTarget;
use std::collections::BTreeSet;
use std::path::Path;

/// Template context for a single resolved send target.
#[derive(Debug, Clone, serde::Serialize)]
struct TargetContext {
    /// Original target ID (e.g. "#motor").
    target: String,
    /// File stem for #include (e.g. "motor" from "#motor").
    target_stem: String,
    /// Snake_case identifier for function names (e.g. "motor").
    target_snake: String,
    /// PascalCase for namespace references (e.g. "Motor").
    target_pascal: String,
    /// Events sent to this target.
    events: Vec<String>,
    /// Transport type from deploy.yaml.
    transport: String,
    /// Transport-native extra config (passed to template).
    extra: std::collections::HashMap<String, serde_yaml_ng::Value>,
}

/// Generate mesh transport code for a machine's resolved targets.
pub fn generate_mesh(
    machine_name: &str,
    targets: &[ResolvedTarget],
    language: Language,
    template_base: &Path,
) -> Result<GeneratedOutput, CodegenError> {
    if targets.is_empty() {
        return Ok(GeneratedOutput { files: vec![] });
    }

    match language {
        Language::Cpp => generate_cpp_mesh(machine_name, targets, template_base),
        _ => Err(CodegenError::UnsupportedLanguage(format!("{:?}", language))),
    }
}

fn generate_cpp_mesh(
    machine_name: &str,
    targets: &[ResolvedTarget],
    template_base: &Path,
) -> Result<GeneratedOutput, CodegenError> {
    let target_contexts: Vec<TargetContext> = targets
        .iter()
        .map(|t| {
            let stripped = t.target.trim_start_matches('#');
            TargetContext {
                target: t.target.clone(),
                target_stem: stripped.to_string(),
                target_snake: filters::to_snake_case(stripped.to_string()),
                target_pascal: filters::to_pascal_case(stripped.to_string()),
                events: t.events.clone(),
                transport: t.transport.clone(),
                extra: t.extra.clone(),
            }
        })
        .collect();

    // Data-driven: template uses transport_types for conditional guards,
    // and filters targets by transport inline. No per-transport vectors.
    let transport_types: BTreeSet<&str> =
        target_contexts.iter().map(|t| t.transport.as_str()).collect();

    let machine_pascal = filters::to_pascal_case(machine_name.to_string());

    let template_name = "mesh/cpp/mesh_transport.h.jinja2";
    let template_path = template_base.join(template_name);
    let template_content =
        std::fs::read_to_string(&template_path).map_err(|e| CodegenError::TemplateRead {
            path: template_path.display().to_string(),
            source: e,
        })?;

    let mut env = minijinja::Environment::new();
    env.add_template("mesh_transport.h.jinja2", &template_content)
        .map_err(|e| CodegenError::TemplateRender(e.to_string()))?;

    let tmpl = env
        .get_template("mesh_transport.h.jinja2")
        .map_err(|e| CodegenError::TemplateRender(e.to_string()))?;

    let ctx = minijinja::context! {
        machine_name => machine_name,
        machine_pascal => machine_pascal,
        targets => target_contexts,
        transport_types => transport_types,
    };

    let code = tmpl
        .render(ctx)
        .map_err(|e| CodegenError::TemplateRender(e.to_string()))?;

    Ok(GeneratedOutput {
        files: vec![(format!("{machine_name}_transport.h"), code)],
    })
}
