// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The standard host features' invoke types (docs/SCE_HOST_FEATURES.md).
//!
//! A standard host feature is a closed set of host-run `<invoke>` types, each
//! with the request and result records the standard library defines for it
//! (`sce:std/host/...`). This table is where a type and its records are tied
//! together, so a document cannot name the type and bring a record of its own
//! to it: the host that serves the type reads the request the library
//! describes, and a machine that read some other result would be reading what
//! no standard host answers.
//!
//! Two things follow from a type being standard:
//!
//! - **It is served without a declaration.** Every host serves the standard
//!   types, so a build needs no `--host-invoker` for them
//!   ([`types_used_by`], read by
//!   [`crate::host_processor_analyzer::declare_host_surfaces`]).
//! - **Its interface is the library's** ([`validate`]): `sce:request` and
//!   `sce:result` are imported from `sce:std/host/...` by name, and an
//!   operation the table says takes no request takes no argument at all.

use std::path::Path;

use crate::forge::error::{ForgeError, Located, SourceLocation, ValidationError};
use crate::forge::model::ForgeKind;
use crate::host_processor_analyzer::HOST_INVOKE_DEADLINE_PARAM;
use crate::model::{Invoke, SCXMLModel, UnsupportedInvokeInfo};

/// One standard host operation: the `<invoke type>` that names it and the
/// library documents of its records, by their name under `sce:std/host/`
/// without the extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StandardHostType {
    /// The `type` of the `<invoke>`.
    pub invoke_type: &'static str,
    /// The request record, or `None` for an operation that takes no argument.
    pub request: Option<&'static str>,
    /// The result record the completion carries.
    pub result: &'static str,
}

/// The directory of the standard library the records live in.
const LIBRARY_DIRECTORY: &str = "sce:std/host/";

/// Every standard host operation. The type names are one spelling in this
/// table, the document that specifies them and the reference hosts, and a
/// test holds this table to the library and the specification.
pub const TYPES: &[StandardHostType] = &[
    StandardHostType {
        invoke_type: "sce-host-clock-now",
        request: None,
        result: "clock_now_result",
    },
    StandardHostType {
        invoke_type: "sce-host-clock-zone",
        request: None,
        result: "clock_zone_result",
    },
    StandardHostType {
        invoke_type: "sce-host-kv-get",
        request: Some("kv_get_request"),
        result: "kv_get_result",
    },
    StandardHostType {
        invoke_type: "sce-host-kv-put",
        request: Some("kv_put_request"),
        result: "kv_put_result",
    },
    StandardHostType {
        invoke_type: "sce-host-kv-delete",
        request: Some("kv_delete_request"),
        result: "kv_delete_result",
    },
    StandardHostType {
        invoke_type: "sce-host-kv-next-key",
        request: Some("kv_next_key_request"),
        result: "kv_next_key_result",
    },
    StandardHostType {
        invoke_type: "sce-host-notify-permission",
        request: None,
        result: "notify_permission_result",
    },
    StandardHostType {
        invoke_type: "sce-host-notify-request-permission",
        request: None,
        result: "notify_permission_result",
    },
    StandardHostType {
        invoke_type: "sce-host-notify-post",
        request: Some("notify_post_request"),
        result: "notify_post_result",
    },
    StandardHostType {
        invoke_type: "sce-host-notify-cancel",
        request: Some("notify_cancel_request"),
        result: "notify_cancel_result",
    },
    StandardHostType {
        invoke_type: "sce-host-http-fetch",
        request: Some("http_fetch_request"),
        result: "http_fetch_result",
    },
];

/// The standard operation `invoke_type` names, if it names one.
pub fn lookup(invoke_type: &str) -> Option<&'static StandardHostType> {
    TYPES.iter().find(|t| t.invoke_type == invoke_type)
}

/// The path of the library document `name` — `sce:std/host/<name>.scxml`.
pub fn document(name: &str) -> String {
    format!("{LIBRARY_DIRECTORY}{name}.scxml")
}

/// The host-run `<invoke>`s of `model`, in document order.
fn host_invokes(model: &SCXMLModel) -> impl Iterator<Item = &UnsupportedInvokeInfo> {
    let mut states: Vec<_> = model.states.values().collect();
    states.sort_by_key(|s| s.document_order);
    states
        .into_iter()
        .flat_map(|s| s.invokes.iter())
        .filter_map(|invoke| match invoke {
            Invoke::Unsupported(info) => Some(info),
            _ => None,
        })
}

/// The standard types `model` runs an `<invoke>` of, each once, in the order
/// the document first names it. These are served with no declaration.
pub fn types_used_by(model: &SCXMLModel) -> Vec<String> {
    let mut used: Vec<String> = Vec::new();
    for info in host_invokes(model) {
        if lookup(&info.invoke_type).is_some() && !used.contains(&info.invoke_type) {
            used.push(info.invoke_type.clone());
        }
    }
    used
}

/// Hold every `<invoke>` of a standard type to the interface the library gives
/// it. Run where a typed host-run invoke's aliases are resolved, after
/// [`crate::forge::typed_invoke::validate`], so an alias read here names an
/// import.
pub fn validate(model: &SCXMLModel, diag_label: &str) -> Result<(), Located<ForgeError>> {
    for info in host_invokes(model) {
        let Some(standard) = lookup(&info.invoke_type) else {
            continue;
        };
        let schema = |attr: &str,
                      alias: &str,
                      at: &Option<SourceLocation>,
                      expected: Option<&str>|
         -> Result<(), Located<ForgeError>> {
            let refuse = |detail: String| {
                located(
                    at.as_ref().or(info.base.source_location.as_ref()),
                    diag_label,
                    ValidationError::TypedInvokeSchema {
                        invoke_id: info.base.invoke_id.clone(),
                        attr: attr.to_string(),
                        alias: alias.to_string(),
                        detail,
                    },
                )
            };
            let Some(expected) = expected else {
                return if alias.is_empty() {
                    Ok(())
                } else {
                    Err(refuse(format!(
                        "{} takes no request, so it names no record",
                        standard.invoke_type
                    )))
                };
            };
            let path = document(expected);
            if alias.is_empty() {
                return Err(refuse(format!(
                    "{} is a standard host type, whose {attr} is the record {path}: import it \
                     with <sce:import kind=\"event-schema\" src=\"{path}\" as=\"…\"/> and name \
                     its alias",
                    standard.invoke_type
                )));
            }
            let from_library = model.forge_imports.iter().any(|import| {
                import.alias == alias
                    && matches!(import.kind, ForgeKind::EventSchema)
                    && crate::forge::stdlib::identity(Path::new(&import.src)) == path
            });
            if from_library {
                Ok(())
            } else {
                Err(refuse(format!(
                    "{} is a standard host type, whose {attr} is the record {path}, and this \
                     alias names another document",
                    standard.invoke_type
                )))
            }
        };
        schema(
            "result",
            &info.result_schema,
            &info.result_schema_at,
            Some(standard.result),
        )?;
        schema(
            "request",
            &info.request_schema,
            &info.request_schema_at,
            standard.request,
        )?;
        if standard.request.is_none() {
            let arguments = info
                .base
                .params
                .iter()
                .find(|p| p.name != HOST_INVOKE_DEADLINE_PARAM);
            let refuse = |at: &Option<SourceLocation>, detail: String, observed: &str| {
                located(
                    at.as_ref().or(info.base.source_location.as_ref()),
                    diag_label,
                    ValidationError::TypedInvokeRequest {
                        invoke_id: info.base.invoke_id.clone(),
                        alias: String::new(),
                        detail,
                        observed: observed.to_string(),
                    },
                )
            };
            if let Some(param) = arguments {
                return Err(refuse(
                    &param.source_location,
                    format!(
                        "{} takes no argument, and <param name=\"{}\"> would be one",
                        standard.invoke_type, param.name
                    ),
                    &param.name,
                ));
            }
            for (present, what) in [
                (!info.namelist.is_empty(), "namelist"),
                (
                    !info.content.is_empty() || !info.contentexpr.is_empty(),
                    "<content>",
                ),
            ] {
                if present {
                    return Err(refuse(
                        &None,
                        format!(
                            "{} takes no argument, so {what} cannot be given",
                            standard.invoke_type
                        ),
                        what,
                    ));
                }
            }
        }
    }
    Ok(())
}

fn located(
    at: Option<&SourceLocation>,
    diag_label: &str,
    error: ValidationError,
) -> Located<ForgeError> {
    Located::new(
        error.into(),
        diag_label,
        at.and_then(|l| l.line),
        at.and_then(|l| l.col),
    )
}
