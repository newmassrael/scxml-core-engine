// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The TTCN-3 export of a test suite (docs/adr/0091;
//! docs/SCE_TEST_PROFILE.md §9).
//!
//! The export is the translation `T` of §9.1: a function from one
//! `test-suite` document and every document it reaches to TTCN-3 modules,
//! each construct translated by one rule of §9.2. It is not a backend: it is
//! in no `Language`, it holds no W3C obligation, and it accepts the documents
//! of the test profile alone (ADR 0091 §8).
//!
//! For a suite whose unit name is `S` it writes:
//!
//! - `S_Types.ttcn`, the types module (rule E1; ADR 0091 C2): the types of
//!   the suite's messages and the codec functions of each, which it declares
//!   and does not implement, so a consumer can replace the module with one
//!   written apart from SCE;
//! - `S_Types.manifest.json`, the structural manifest of that module
//!   ([`manifest`]), against which a module written apart is compared.
//!
//! The port types (E3), the templates (E2), the module parameters the suite
//! declares, and each test component's type and behaviour (E4 to E13,
//! [`behaviour`]) are definitions of the behaviour module `S.ttcn`, which the
//! test cases and the control part (E14, E15) complete. The export computes
//! them on every run, so a suite whose definitions it cannot write is
//! refused now and not when that module is written, and it writes no part of
//! that module until every rule of it is written: no file the export writes
//! is a part of one.
//!
//! Every name the export writes is a name of the documents, written as it is
//! written ([`names`]). Nothing is written before the whole export is
//! computed, so a refused suite leaves nothing on disk. The output is a
//! function of the documents and of this generator alone
//! (docs/SCE_CODEGEN_DETERMINISM.md).

pub mod behaviour;
mod expression;
mod macrostep;
pub mod manifest;
pub mod names;
pub mod port_types;
mod reach;
mod render;
pub mod templates;
pub mod types;

use std::path::{Path, PathBuf};

use crate::forge::error::{ForgeError, GenerateError, Located};
use crate::forge::model::{ForgeDocument, ForgeKind};
use names::{unit_name, Declared, Namespace, Origin};

/// What the export of a suite writes, and what it computes of the behaviour
/// module that is not written yet.
#[derive(Debug, Clone)]
pub struct Ttcn3Export {
    /// The suite's unit name, which names its modules.
    pub suite: String,
    /// The types module, `<suite>_Types.ttcn`.
    pub types_module: String,
    /// The structural manifest of the types module,
    /// `<suite>_Types.manifest.json`.
    pub manifest: String,
    /// Each port type the suite reaches (E3), as the behaviour module holds
    /// it, in the order of their names.
    pub port_types: Vec<Definition>,
    /// Each template the suite reaches (E2), as the behaviour module holds
    /// it, in the order of their names.
    pub templates: Vec<Definition>,
    /// Each module parameter the suite declares, as the behaviour module
    /// holds it, in the order the suite declares them.
    pub module_parameters: Vec<Definition>,
    /// Each test component the suite reaches as a component type (E4), in
    /// the order of their names.
    pub component_types: Vec<Definition>,
    /// Each component's behaviour as a function (E5 to E13), in the order of
    /// their names.
    pub functions: Vec<Definition>,
    /// The module parameters as the definitions above are written from, each
    /// with the construct it was translated from.
    pub parameter_views: Vec<behaviour::ModuleParameterView>,
    /// The component types and functions as the definitions above are
    /// written from, each definition and statement with the construct and
    /// the rule it was translated from: what the correspondence witness of
    /// docs/SCE_TEST_PROFILE.md §9.4 serializes.
    pub behaviour: behaviour::Behaviour,
}

/// One definition of a module: its name and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Definition {
    pub name: String,
    pub text: String,
}

impl Ttcn3Export {
    /// The name of the types module.
    pub fn types_module_name(&self) -> String {
        format!("{}_Types", self.suite)
    }

    /// The files the export writes, each its name and its text, in the order
    /// it writes them: the types module and its manifest.
    pub fn files(&self) -> Vec<(String, String)> {
        let module = self.types_module_name();
        vec![
            (format!("{module}.ttcn"), self.types_module.clone()),
            (format!("{module}.manifest.json"), self.manifest.clone()),
        ]
    }
}

/// Why a suite is not exported.
#[derive(Debug)]
pub enum ExportError {
    /// The suite, or a document it reaches, is refused: by a rule of its
    /// kind, by the name contract (`test-profile/export-name`), or as a type
    /// or a template the export has no TTCN-3 form for
    /// (`generate/unsupported-feature`).
    Refused(Located<ForgeError>),
    /// The document named is no test suite; the text says what it is.
    NotASuite(String),
}

/// Export the suite at `path` (docs/SCE_TEST_PROFILE.md §9.5).
/// `include_dirs` resolve the suite's `<xi:include>` and `<sce:use>`
/// fragments, as `generate` resolves them.
pub fn export_suite(path: &Path, include_dirs: &[PathBuf]) -> Result<Ttcn3Export, ExportError> {
    let loaded = crate::load_forge_source(path, include_dirs).map_err(ExportError::Refused)?;
    let path_text = path.to_string_lossy();
    let label = crate::DocumentLabel::for_input_path(&path_text);
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let refused = |e: Located<ForgeError>| ExportError::Refused(loaded.positions.authored(e));
    let parsed = crate::forge::parser::parse_forge_with_imports(loaded.text(), label)
        .map_err(refused)?
        .ok_or_else(|| ExportError::NotASuite(format!("{} is a statechart", path.display())))?;
    let ForgeDocument::TestSuite(suite) = &parsed.document else {
        return Err(ExportError::NotASuite(format!(
            "{} is a document of sce:kind=\"{}\"",
            path.display(),
            parsed.document.kind()
        )));
    };
    reach::check_document(&parsed, dir, label.diagnostic_label).map_err(refused)?;
    let reached = reach::reach(&parsed, dir, label.diagnostic_label).map_err(refused)?;

    let name = unit_name(&suite.name);
    let at = Origin {
        kind: ForgeKind::TestSuite,
        document: suite.name.clone(),
        label: label.diagnostic_label.to_string(),
        line: reach::root_line(&parsed.document),
    };
    let mut names = Namespace::default();
    names
        .definition(&name, Declared::read("the suite's module", at.clone()))
        .map_err(refused)?;
    names
        .definition(
            &format!("{name}_Types"),
            Declared::made("the types module", at.clone()),
        )
        .map_err(refused)?;
    // The module parameters are definitions of the module, and a template's
    // formal parameter or a component's name that is one of them is refused
    // where it is written, after them.
    let suite_scope = crate::forge::import_source::type_scope(&parsed.imports, dir);
    let parameter_views =
        behaviour::module_parameters(suite, &suite_scope, &at, &mut names).map_err(refused)?;
    let module = types::types_module(&name, &reached, &mut names, &at).map_err(refused)?;
    let port_type_views = port_types::port_types(&reached, &mut names).map_err(refused)?;
    let template_views = templates::templates(&reached, &mut names).map_err(refused)?;
    let behaviour = behaviour::behaviour(&reached, &mut names).map_err(refused)?;

    let in_suite =
        |e: ForgeError| ExportError::Refused(Located::in_file(e, label.diagnostic_label));
    let env = render::environment().map_err(in_suite)?;
    let types_module =
        render::render(&env, render::TYPES_MODULE, "module", &module).map_err(in_suite)?;
    let manifest = manifest::manifest_text(&module).map_err(|e| {
        in_suite(GenerateError::TemplateRender(format!("the manifest of {name}_Types: {e}")).into())
    })?;
    let mut port_type_definitions = Vec::new();
    for view in &port_type_views {
        port_type_definitions.push(Definition {
            name: view.name.clone(),
            text: render::render(&env, render::PORT_TYPE, "port_type", view).map_err(in_suite)?,
        });
    }
    let mut template_definitions = Vec::new();
    for view in &template_views {
        template_definitions.push(Definition {
            name: view.name.clone(),
            text: render::render(&env, render::TEMPLATE, "template", view).map_err(in_suite)?,
        });
    }
    let mut module_parameters = Vec::new();
    for view in &parameter_views {
        module_parameters.push(Definition {
            name: view.name.clone(),
            text: render::render(&env, render::MODULE_PARAMETER, "parameter", view)
                .map_err(in_suite)?,
        });
    }
    let mut component_types = Vec::new();
    for view in &behaviour.component_types {
        component_types.push(Definition {
            name: view.name.clone(),
            text: render::render(&env, render::COMPONENT_TYPE, "component", view)
                .map_err(in_suite)?,
        });
    }
    let mut functions = Vec::new();
    for view in &behaviour.functions {
        functions.push(Definition {
            name: view.name.clone(),
            text: render::render(&env, render::FUNCTION, "function", view).map_err(in_suite)?,
        });
    }
    Ok(Ttcn3Export {
        suite: name,
        types_module,
        manifest,
        port_types: port_type_definitions,
        templates: template_definitions,
        module_parameters,
        component_types,
        functions,
        parameter_views,
        behaviour,
    })
}
