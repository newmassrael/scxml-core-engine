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
//!   ([`manifest`]), against which a module written apart is compared;
//! - `S.ttcn`, the behaviour module, which imports the types module whole
//!   and holds the port types (E3), the templates (E2), the module parameters
//!   the suite declares and those of its features, each test component's type
//!   and behaviour (E4 to E13, [`behaviour`]), the system component type of
//!   each configuration a test case runs in, the test cases (E14) and the
//!   control part (E15, [`execution`]);
//! - `S.witness.json`, the correspondence witness of §9.4 ([`witness`]): the
//!   model both modules are written from, serialized, relating each construct
//!   of the documents and the rule that translated it to every place of the
//!   modules that rule wrote for it.
//!
//! Every name the export writes is a name of the documents, written as it is
//! written ([`names`]). Nothing is written before the whole export is
//! computed, so a refused suite leaves nothing on disk. The output is a
//! function of the documents and of this generator alone
//! (docs/SCE_CODEGEN_DETERMINISM.md).

pub mod behaviour;
pub mod execution;
mod expression;
mod macrostep;
pub mod manifest;
pub mod names;
pub mod port_types;
mod reach;
mod render;
pub mod templates;
pub mod types;
pub mod witness;

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::forge::error::{ForgeError, GenerateError, Located};
use crate::forge::model::{ForgeDocument, ForgeKind, ParsedForge, TestSuiteModel};
use names::{unit_name, Declared, Namespace, Origin};

/// What the export of a suite writes, and each definition of its behaviour
/// module, as text and as the model it is written from.
#[derive(Debug, Clone)]
pub struct Ttcn3Export {
    /// The suite's unit name, which names its modules.
    pub suite: String,
    /// The types module, `<suite>_Types.ttcn`.
    pub types_module: String,
    /// The structural manifest of the types module,
    /// `<suite>_Types.manifest.json`.
    pub manifest: String,
    /// The behaviour module, `<suite>.ttcn`: every definition below, in the
    /// order docs/SCE_TEST_PROFILE.md §9.5 gives.
    pub behaviour_module: String,
    /// The correspondence witness of both modules, `<suite>.witness.json`
    /// (docs/SCE_TEST_PROFILE.md §9.4).
    pub witness: String,
    /// Each port type the suite reaches (E3), as the behaviour module holds
    /// it, in the order of their names.
    pub port_types: Vec<Definition>,
    /// Each template the suite reaches (E2), as the behaviour module holds
    /// it, in the order of their names.
    pub templates: Vec<Definition>,
    /// Each module parameter the suite declares, as the behaviour module
    /// holds it, in the order the suite declares them.
    pub module_parameters: Vec<Definition>,
    /// The module parameter of each feature of the suite's feature model
    /// (E15), in the order of the features' names.
    pub feature_parameters: Vec<Definition>,
    /// Each test component the suite reaches as a component type (E4), in
    /// the order of their names.
    pub component_types: Vec<Definition>,
    /// The system component type of each configuration a test case runs in
    /// (E14), in the order of their names.
    pub system_types: Vec<Definition>,
    /// Each component's behaviour as a function (E5 to E13), in the order of
    /// their names.
    pub functions: Vec<Definition>,
    /// Each test case the suite executes (E14), in the suite's order.
    pub test_cases: Vec<Definition>,
    /// The control part (E15).
    pub control: String,
    /// The module parameters as the definitions above are written from, each
    /// with the construct it was translated from.
    pub parameter_views: Vec<behaviour::ModuleParameterView>,
    /// The component types and functions as the definitions above are
    /// written from, each definition and statement with the construct and
    /// the rule it was translated from: what the correspondence witness of
    /// docs/SCE_TEST_PROFILE.md §9.4 serializes.
    pub behaviour: behaviour::Behaviour,
    /// The module parameters of the features, the system component types,
    /// the test cases and the control part as they are written from, each
    /// with the construct and the rule it was translated from.
    pub execution: execution::Execution,
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
    /// it writes them: the types module, its manifest, the behaviour module
    /// and the witness.
    pub fn files(&self) -> Vec<(String, String)> {
        let module = self.types_module_name();
        vec![
            (format!("{module}.ttcn"), self.types_module.clone()),
            (format!("{module}.manifest.json"), self.manifest.clone()),
            (
                format!("{}.ttcn", self.suite),
                self.behaviour_module.clone(),
            ),
            (format!("{}.witness.json", self.suite), self.witness.clone()),
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

/// The behaviour module as its template reads it: its name, the types module
/// it imports, and the text of each definition, in the order of §9.5.
#[derive(Serialize)]
struct BehaviourModuleView<'a> {
    name: &'a str,
    types_module: String,
    port_types: Vec<&'a str>,
    templates: Vec<&'a str>,
    /// The module parameters the suite declares, then its features'.
    module_parameters: Vec<&'a str>,
    /// The component types of the test components, then the system
    /// component types.
    component_types: Vec<&'a str>,
    functions: Vec<&'a str>,
    test_cases: Vec<&'a str>,
    control: &'a str,
}

/// The texts of `definitions`, in their order.
fn texts(definitions: &[Definition]) -> impl Iterator<Item = &str> {
    definitions.iter().map(|d| d.text.as_str())
}

/// Each of `views` as the definition `template` writes of it, which reads it
/// as `key`, named by `name`.
fn definitions<V: Serialize>(
    env: &minijinja::Environment<'_>,
    template: &str,
    key: &str,
    views: &[V],
    name: impl Fn(&V) -> &str,
) -> Result<Vec<Definition>, ForgeError> {
    views
        .iter()
        .map(|view| -> Result<Definition, ForgeError> {
            Ok(Definition {
                name: name(view).to_string(),
                text: render::render(env, template, key, view)?,
            })
        })
        .collect()
}

/// A test suite read as the export reads it: parsed with its imports, held to
/// the rules `check` holds it to, and with every document it reaches read and
/// held to the rules of its kind (`reach`). What the export translates, and
/// what the check of a recorded input trace reads the suite's tables from
/// (`trace_check`), so both refuse the same suites. Private to the export and
/// the modules under it, as the tables it holds are.
struct ReadSuite {
    /// The suite's text as read, which places a refusal in it where its
    /// author wrote it, before its `<xi:include>` and `<sce:use>` expanded.
    loaded: crate::LoadedForgeSource,
    /// The path the suite was named by, which a refusal of its kind names.
    path: PathBuf,
    parsed: ParsedForge,
    /// The directory the suite's imports resolve against.
    dir: PathBuf,
    /// Where a name written in the suite is refused: at its root.
    at: Origin,
    reach: reach::Reach,
}

impl ReadSuite {
    /// The suite itself.
    fn suite(&self) -> Result<&TestSuiteModel, ExportError> {
        as_suite(&self.path, &self.parsed.document)
    }

    /// `e`, a refusal of the suite or of a document it reaches, placed where
    /// its author wrote it.
    fn refused(&self, e: Located<ForgeError>) -> ExportError {
        ExportError::Refused(self.loaded.positions.authored(e))
    }
}

/// `document`, read from `path`, as the test suite it must be.
fn as_suite<'d>(
    path: &Path,
    document: &'d ForgeDocument,
) -> Result<&'d TestSuiteModel, ExportError> {
    match document {
        ForgeDocument::TestSuite(suite) => Ok(suite),
        other => Err(ExportError::NotASuite(format!(
            "{} is a document of sce:kind=\"{}\"",
            path.display(),
            other.kind()
        ))),
    }
}

/// Read the suite at `path` as the export reads it ([`ReadSuite`]).
/// `include_dirs` resolve the suite's `<xi:include>` and `<sce:use>`
/// fragments, as `generate` resolves them.
fn read_suite(path: &Path, include_dirs: &[PathBuf]) -> Result<ReadSuite, ExportError> {
    let loaded = crate::load_forge_source(path, include_dirs).map_err(ExportError::Refused)?;
    let path_text = path.to_string_lossy();
    let label = crate::DocumentLabel::for_input_path(&path_text);
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let refused = |e: Located<ForgeError>| ExportError::Refused(loaded.positions.authored(e));
    let parsed = crate::forge::parser::parse_forge_with_imports(loaded.text(), label)
        .map_err(refused)?
        .ok_or_else(|| ExportError::NotASuite(format!("{} is a statechart", path.display())))?;
    let suite = as_suite(path, &parsed.document)?;
    reach::check_document(&parsed, dir, label.diagnostic_label).map_err(refused)?;
    let reached = reach::reach(&parsed, dir, label.diagnostic_label).map_err(refused)?;
    let at = Origin {
        kind: ForgeKind::TestSuite,
        document: suite.name.clone(),
        label: label.diagnostic_label.to_string(),
        line: reach::root_line(&parsed.document),
    };
    Ok(ReadSuite {
        loaded,
        path: path.to_path_buf(),
        parsed,
        dir: dir.to_path_buf(),
        at,
        reach: reached,
    })
}

/// Export the suite at `path` (docs/SCE_TEST_PROFILE.md §9.5).
/// `include_dirs` resolve the suite's `<xi:include>` and `<sce:use>`
/// fragments, as `generate` resolves them.
pub fn export_suite(path: &Path, include_dirs: &[PathBuf]) -> Result<Ttcn3Export, ExportError> {
    let read = read_suite(path, include_dirs)?;
    let suite = read.suite()?;
    let parsed = &read.parsed;
    let dir = read.dir.as_path();
    let label = read.at.label.as_str();
    let refused = |e: Located<ForgeError>| read.refused(e);
    let reached = &read.reach;
    // The feature model, which E15 alone reads: the walk does not follow it.
    let features = suite
        .features
        .as_deref()
        .and_then(|alias| {
            parsed
                .imports
                .iter()
                .find(|i| i.alias == alias && i.kind == ForgeKind::FeatureModel)
        })
        .map(|import| reach::feature_model(import, dir, label))
        .transpose()
        .map_err(refused)?;

    let name = unit_name(&suite.name);
    let at = &read.at;
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
        behaviour::module_parameters(suite, &suite_scope, at, &mut names).map_err(refused)?;
    let module = types::types_module(&name, reached, &mut names, at).map_err(refused)?;
    let port_type_views = port_types::port_types(reached, &mut names).map_err(refused)?;
    let template_views = templates::templates(reached, &mut names).map_err(refused)?;
    let behaviour = behaviour::behaviour(reached, &mut names).map_err(refused)?;
    let execution = execution::execution(
        suite,
        &parsed.imports,
        dir,
        at,
        features.as_ref(),
        reached,
        &mut names,
    )
    .map_err(refused)?;

    let in_suite = |e: ForgeError| ExportError::Refused(Located::in_file(e, label));
    let env = render::environment().map_err(in_suite)?;
    let types_module =
        render::render(&env, render::TYPES_MODULE, "module", &module).map_err(in_suite)?;
    let manifest = manifest::manifest_text(&module).map_err(|e| {
        in_suite(GenerateError::TemplateRender(format!("the manifest of {name}_Types: {e}")).into())
    })?;
    let port_types = definitions(
        &env,
        render::PORT_TYPE,
        "port_type",
        &port_type_views,
        |v| v.name.as_str(),
    )
    .map_err(in_suite)?;
    let templates = definitions(&env, render::TEMPLATE, "template", &template_views, |v| {
        v.name.as_str()
    })
    .map_err(in_suite)?;
    let module_parameters = definitions(
        &env,
        render::MODULE_PARAMETER,
        "parameter",
        &parameter_views,
        |v| v.name.as_str(),
    )
    .map_err(in_suite)?;
    let feature_parameters = definitions(
        &env,
        render::MODULE_PARAMETER,
        "parameter",
        &execution.features,
        |v| v.name.as_str(),
    )
    .map_err(in_suite)?;
    let component_types = definitions(
        &env,
        render::COMPONENT_TYPE,
        "component",
        &behaviour.component_types,
        |v| v.name.as_str(),
    )
    .map_err(in_suite)?;
    let system_types = definitions(
        &env,
        render::COMPONENT_TYPE,
        "component",
        &execution.system_types,
        |v| v.name.as_str(),
    )
    .map_err(in_suite)?;
    let functions = definitions(
        &env,
        render::FUNCTION,
        "function",
        &behaviour.functions,
        |v| v.name.as_str(),
    )
    .map_err(in_suite)?;
    let test_cases = definitions(
        &env,
        render::TEST_CASE,
        "test_case",
        &execution.test_cases,
        |v| v.name.as_str(),
    )
    .map_err(in_suite)?;
    let control =
        render::render(&env, render::CONTROL, "control", &execution.control).map_err(in_suite)?;
    let witness = witness::witness_text(&witness::witness(&witness::Parts {
        suite: &name,
        base: dir,
        types: &module,
        port_types: &port_type_views,
        templates: &template_views,
        module_parameters: &parameter_views,
        behaviour: &behaviour,
        execution: &execution,
    }))
    .map_err(|e| {
        in_suite(GenerateError::TemplateRender(format!("the witness of {name}: {e}")).into())
    })?;
    let behaviour_module = render::render(
        &env,
        render::BEHAVIOUR_MODULE,
        "module",
        &BehaviourModuleView {
            name: &name,
            types_module: format!("{name}_Types"),
            port_types: texts(&port_types).collect(),
            templates: texts(&templates).collect(),
            module_parameters: texts(&module_parameters)
                .chain(texts(&feature_parameters))
                .collect(),
            component_types: texts(&component_types)
                .chain(texts(&system_types))
                .collect(),
            functions: texts(&functions).collect(),
            test_cases: texts(&test_cases).collect(),
            control: &control,
        },
    )
    .map_err(in_suite)?;
    Ok(Ttcn3Export {
        suite: name,
        types_module,
        manifest,
        behaviour_module,
        witness,
        port_types,
        templates,
        module_parameters,
        feature_parameters,
        component_types,
        system_types,
        functions,
        test_cases,
        control,
        parameter_views,
        behaviour,
        execution,
    })
}
