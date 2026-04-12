// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh codegen dispatcher — selects transport template and generates code.
//
// Supported transports:
//   local — same-process direct call (Phase 1)
//   shm   — shared memory cross-process via EventQueueBridge (Phase 2)

use crate::filters;
use crate::generator::{GeneratedOutput, Language};
use crate::mesh::error::CodegenError;
use crate::mesh::topology::ResolvedTarget;
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
    /// Transport type.
    transport: String,
    /// Transport-native extra config (passed to template).
    extra: std::collections::HashMap<String, serde_yaml_ng::Value>,
}

/// Generate mesh transport code for a machine's resolved targets.
///
/// Returns generated files to be written alongside the SM output.
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
    // Determine transport type — all targets must agree
    let transport = &targets[0].transport;
    for t in &targets[1..] {
        if t.transport != *transport {
            let mut seen: Vec<String> = targets.iter().map(|t| t.transport.clone()).collect();
            seen.sort();
            seen.dedup();
            return Err(CodegenError::MixedTransports {
                machine: machine_name.to_string(),
                transports: seen,
            });
        }
    }

    // Select template by transport type
    let template_name = match transport.as_str() {
        "local" => "mesh/cpp/local_transport.h.jinja2",
        "shm" => "mesh/cpp/shm_transport.h.jinja2",
        other => {
            return Err(CodegenError::UnsupportedTransport {
                transport: other.to_string(),
                target: targets[0].target.clone(),
            });
        }
    };

    let template_path = template_base.join(template_name);
    let template_content =
        std::fs::read_to_string(&template_path).map_err(|e| CodegenError::TemplateRead {
            path: template_path.display().to_string(),
            source: e,
        })?;

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

    let machine_pascal = filters::to_pascal_case(machine_name.to_string());

    let mut env = minijinja::Environment::new();
    let tpl_file_name = template_name.rsplit('/').next().unwrap_or(template_name);
    env.add_template(tpl_file_name, &template_content)
        .map_err(|e| CodegenError::TemplateRender(e.to_string()))?;

    let tmpl = env
        .get_template(tpl_file_name)
        .map_err(|e| CodegenError::TemplateRender(e.to_string()))?;

    let ctx = minijinja::context! {
        machine_name => machine_name,
        machine_pascal => machine_pascal,
        targets => target_contexts,
    };

    let code = tmpl
        .render(ctx)
        .map_err(|e| CodegenError::TemplateRender(e.to_string()))?;

    Ok(GeneratedOutput {
        files: vec![(format!("{machine_name}_transport.h"), code)],
    })
}
