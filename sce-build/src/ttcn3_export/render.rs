// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The environment the export's templates are rendered in: the templates
//! under `ttcn3/` of the tree this generator embeds, each through the one
//! door every template passes ([`crate::generator::register_template_in`]),
//! read as TTCN-3 ([`Syntax::Ttcn3`]). A value written into a comment is
//! encoded for it, and a text value is written by the filter
//! `ttcn3_charstring`, so a module holds no character a value could use to
//! leave the place it is written in.

use minijinja::Environment;
use serde::Serialize;

use crate::forge::error::{ForgeError, GenerateError};
use crate::template_lexing::Syntax;

/// The directory of the template tree the export's templates are in.
const DIRECTORY: &str = "ttcn3/";

/// The template of the types module; its value is `module`.
pub(super) const TYPES_MODULE: &str = "ttcn3/types_module.ttcn.jinja2";

/// The template of one port type; its value is `port_type`.
pub(super) const PORT_TYPE: &str = "ttcn3/port_type.ttcn.jinja2";

/// The template of one template; its value is `template`.
pub(super) const TEMPLATE: &str = "ttcn3/template.ttcn.jinja2";

/// The template of one module parameter; its value is `parameter`.
pub(super) const MODULE_PARAMETER: &str = "ttcn3/module_parameter.ttcn.jinja2";

/// The template of one component type; its value is `component`.
pub(super) const COMPONENT_TYPE: &str = "ttcn3/component_type.ttcn.jinja2";

/// The template of one function; its value is `function`.
pub(super) const FUNCTION: &str = "ttcn3/function.ttcn.jinja2";

/// The template of one test case; its value is `test_case`.
pub(super) const TEST_CASE: &str = "ttcn3/test_case.ttcn.jinja2";

/// The template of the control part; its value is `control`.
pub(super) const CONTROL: &str = "ttcn3/control.ttcn.jinja2";

/// The template of the behaviour module; its value is `module`.
pub(super) const BEHAVIOUR_MODULE: &str = "ttcn3/behaviour_module.ttcn.jinja2";

/// The environment, every template of the export registered.
pub(super) fn environment() -> Result<Environment<'static>, ForgeError> {
    let mut env = crate::generator::new_env();
    env.add_filter("ttcn3_charstring", crate::filters::ttcn3_charstring);
    for &(name, content) in crate::template_registry::EMBEDDED_TEMPLATES {
        if !name.starts_with(DIRECTORY) {
            continue;
        }
        crate::generator::register_template_in(&mut env, name.to_string(), content, Syntax::Ttcn3)
            .map_err(|e| GenerateError::TemplateLoad(format!("{name}: {e:#}")))?;
    }
    Ok(env)
}

/// The text `template` writes of `value`, which it reads as `key`.
pub(super) fn render(
    env: &Environment<'_>,
    template: &str,
    key: &str,
    value: &impl Serialize,
) -> Result<String, ForgeError> {
    let context = minijinja::Value::from_iter([(key, minijinja::Value::from_serialize(value))]);
    env.get_template(template)
        .and_then(|t| t.render(context))
        .map_err(|e| GenerateError::TemplateRender(format!("{template}: {e:#}")).into())
}
