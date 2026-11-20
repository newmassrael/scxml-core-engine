// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//! Three things follow from a type being standard:
//!
//! - **It is served without a declaration.** The host of every lane that
//!   serves the type serves it, so a build needs no `--host-invoker` for it
//!   ([`types_used_by`], read by
//!   [`crate::host_processor_analyzer::declare_host_surfaces`]).
//! - **Its interface is the library's** ([`validate`]): `sce:request` and
//!   `sce:result` are imported from `sce:std/host/...` by name, and an
//!   operation the table says takes no request takes no argument at all.
//! - **It is served on the lanes the table names** ([`refuse_unserved`],
//!   docs/adr/0107, D8): a build for a lane whose reference host does not
//!   serve the type refuses a machine that runs it, naming the lanes that do.
//!   A lane is a backend `-l` names or the Interpreter ([`HostLane`]), whose
//!   build is the document `sce-codegen lower` writes. That every lane serves
//!   every type stays the goal; it is not a precondition of adding one.

use std::path::Path;

use crate::forge::error::{ForgeError, GenerateError, Located, SourceLocation, ValidationError};
use crate::forge::model::{ForgeKind, SceType};
use crate::generator::Language;
use crate::host_processor_analyzer::HOST_INVOKE_DEADLINE_PARAM;
use crate::model::{Invoke, SCXMLModel, UnsupportedInvokeInfo};

/// A lane a standard host operation is served on: a row of the table of
/// reference hosts and drivers (docs/SCE_HOST_FEATURES.md, section 8). Each
/// generated backend is one, its machine written by `generate` for the
/// backend `-l` names; the Interpreter is one more, its machine the document
/// `sce-codegen lower` writes and its host the C++ reference host registered
/// on the `StateMachine`. A build for either refuses a machine that runs an
/// operation the lane's reference host does not serve (docs/adr/0107, D8).
///
/// The Interpreter's lane is its own and not the C++ one: the operations the
/// C++ host serves only once the embedding adds a feature to it (the COSE
/// operations, on libsodium) are served on the generated C++ machine, whose
/// driver adds it, and not on the Interpreter, whose driver does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostLane {
    /// The machine a backend generates.
    Generated(Language),
    /// The document lowered for the Interpreter
    /// ([`crate::forge::static_js::lower_file_set`]).
    Interpreter,
}

impl HostLane {
    /// Every lane, in the order an entry of the table names its lanes and a
    /// refusal reads them: the backends in the order of [`Language::ALL`],
    /// then the Interpreter.
    pub fn all() -> impl Iterator<Item = HostLane> {
        Language::ALL
            .iter()
            .map(|&language| HostLane::Generated(language))
            .chain(std::iter::once(HostLane::Interpreter))
    }

    /// The name a refusal gives the lane: a backend's wire name, as `-l`
    /// spells it, or `interpreter`.
    pub const fn wire_name(self) -> &'static str {
        match self {
            HostLane::Generated(language) => {
                crate::forge::codegen_matrix::language_wire_name(language)
            }
            HostLane::Interpreter => "interpreter",
        }
    }
}

/// One standard host operation: the `<invoke type>` that names it, the
/// library documents of its records, by their name under `sce:std/host/`
/// without the extension, and the lanes that serve it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StandardHostType {
    /// The `type` of the `<invoke>`.
    pub invoke_type: &'static str,
    /// The request record, or `None` for an operation that takes no argument.
    pub request: Option<&'static str>,
    /// The result record the completion carries.
    pub result: &'static str,
    /// The lanes whose reference host serves the operation
    /// (docs/SCE_HOST_FEATURES.md, section 8), in the order of
    /// [`HostLane::all`]. A build for a lane not listed refuses a machine that
    /// runs the operation ([`refuse_unserved`]). A lane joins the list in the
    /// change that lands its reference host's support for the operation and
    /// its driver's scenarios.
    pub served_by: &'static [HostLane],
}

/// The lanes whose reference host serves every operation of the clock, the
/// key-value store, notifications, HTTP and generated text
/// (docs/SCE_HOST_FEATURES.md, sections 2 to 6): every lane today, each with
/// the reference host and driver section 8 lists, the Interpreter's included.
/// Written out rather than taken from [`HostLane::all`], so that a backend
/// added to [`Language`] is refused these operations until its reference host
/// serves them: a lane joins this list in the change that lands that host and
/// its scenarios, and in no other change. An operation added later names its
/// own lanes (docs/adr/0107, D8).
///
/// The Interpreter's host serves a model's generation too; the lowering
/// refuses that operation for a reason of its own, that the lowered document
/// carries no answer schema (docs/adr/0087, section 7.1).
pub const EVERY_REFERENCE_HOST: &[HostLane] = &[
    HostLane::Generated(Language::Rust),
    HostLane::Generated(Language::Cpp),
    HostLane::Generated(Language::Kotlin),
    HostLane::Generated(Language::Go),
    HostLane::Generated(Language::Python),
    HostLane::Generated(Language::C11),
    HostLane::Generated(Language::Swift),
    HostLane::Interpreter,
];

/// The lanes whose reference host serves signing and sealing, random bytes,
/// keyed digests, keys, credentials and the audit log (docs/SCE_HOST_FEATURES.md,
/// sections 10 to 15; docs/adr/0107, D7; docs/adr/0109, D0, D3, D5 to D7 and
/// D14): Rust on the RustCrypto crates and C++ on libsodium. Every other lane is a later step,
/// which adds the lane here in the change that lands its host and its driver's
/// replay of the scenario.
pub const COSE_REFERENCE_HOSTS: &[HostLane] = &[
    HostLane::Generated(Language::Rust),
    HostLane::Generated(Language::Cpp),
];

/// The directory of the standard library the records live in.
const LIBRARY_DIRECTORY: &str = "sce:std/host/";

/// Every standard host operation. The type names are one spelling in this
/// table, the document that specifies them and the reference hosts, and a
/// test holds this table to the library and the specification. Each names
/// the lanes that serve it: those of sections 2 to 6 every lane
/// ([`EVERY_REFERENCE_HOST`]), signing and sealing, random bytes, keyed
/// digests, keys, credentials and the audit log Rust and C++
/// ([`COSE_REFERENCE_HOSTS`]).
pub const TYPES: &[StandardHostType] = &[
    StandardHostType {
        invoke_type: "sce-host-clock-now",
        request: None,
        result: "clock_now_result",
        served_by: EVERY_REFERENCE_HOST,
    },
    StandardHostType {
        invoke_type: "sce-host-clock-zone",
        request: None,
        result: "clock_zone_result",
        served_by: EVERY_REFERENCE_HOST,
    },
    StandardHostType {
        invoke_type: "sce-host-kv-get",
        request: Some("kv_get_request"),
        result: "kv_get_result",
        served_by: EVERY_REFERENCE_HOST,
    },
    StandardHostType {
        invoke_type: "sce-host-kv-put",
        request: Some("kv_put_request"),
        result: "kv_put_result",
        served_by: EVERY_REFERENCE_HOST,
    },
    StandardHostType {
        invoke_type: "sce-host-kv-put-if",
        request: Some("kv_put_if_request"),
        result: "kv_put_if_result",
        served_by: EVERY_REFERENCE_HOST,
    },
    StandardHostType {
        invoke_type: "sce-host-kv-delete",
        request: Some("kv_delete_request"),
        result: "kv_delete_result",
        served_by: EVERY_REFERENCE_HOST,
    },
    StandardHostType {
        invoke_type: "sce-host-kv-next-key",
        request: Some("kv_next_key_request"),
        result: "kv_next_key_result",
        served_by: EVERY_REFERENCE_HOST,
    },
    StandardHostType {
        invoke_type: "sce-host-notify-permission",
        request: None,
        result: "notify_permission_result",
        served_by: EVERY_REFERENCE_HOST,
    },
    StandardHostType {
        invoke_type: "sce-host-notify-request-permission",
        request: None,
        result: "notify_permission_result",
        served_by: EVERY_REFERENCE_HOST,
    },
    StandardHostType {
        invoke_type: "sce-host-notify-post",
        request: Some("notify_post_request"),
        result: "notify_post_result",
        served_by: EVERY_REFERENCE_HOST,
    },
    StandardHostType {
        invoke_type: "sce-host-notify-cancel",
        request: Some("notify_cancel_request"),
        result: "notify_cancel_result",
        served_by: EVERY_REFERENCE_HOST,
    },
    StandardHostType {
        invoke_type: "sce-host-http-fetch",
        request: Some("http_fetch_request"),
        result: "http_fetch_result",
        served_by: EVERY_REFERENCE_HOST,
    },
    StandardHostType {
        invoke_type: crate::forge::model_answer::MODEL_GENERATE,
        request: Some("model_generate_request"),
        result: "model_generate_result",
        served_by: EVERY_REFERENCE_HOST,
    },
    StandardHostType {
        invoke_type: "sce-host-cose-sign1",
        request: Some("cose_sign1_request"),
        result: "cose_sign1_result",
        served_by: COSE_REFERENCE_HOSTS,
    },
    StandardHostType {
        invoke_type: "sce-host-cose-verify1",
        request: Some("cose_verify1_request"),
        result: "cose_verify1_result",
        served_by: COSE_REFERENCE_HOSTS,
    },
    StandardHostType {
        invoke_type: "sce-host-cose-encrypt0",
        request: Some("cose_encrypt0_request"),
        result: "cose_encrypt0_result",
        served_by: COSE_REFERENCE_HOSTS,
    },
    StandardHostType {
        invoke_type: "sce-host-cose-decrypt0",
        request: Some("cose_decrypt0_request"),
        result: "cose_decrypt0_result",
        served_by: COSE_REFERENCE_HOSTS,
    },
    StandardHostType {
        invoke_type: "sce-host-key-derive",
        request: Some("key_derive_request"),
        result: "key_derive_result",
        served_by: COSE_REFERENCE_HOSTS,
    },
    StandardHostType {
        invoke_type: "sce-host-random-bytes",
        request: Some("random_bytes_request"),
        result: "random_bytes_result",
        served_by: COSE_REFERENCE_HOSTS,
    },
    StandardHostType {
        invoke_type: "sce-host-mac",
        request: Some("mac_request"),
        result: "mac_result",
        served_by: COSE_REFERENCE_HOSTS,
    },
    StandardHostType {
        invoke_type: "sce-host-mac-verify",
        request: Some("mac_verify_request"),
        result: "mac_verify_result",
        served_by: COSE_REFERENCE_HOSTS,
    },
    StandardHostType {
        invoke_type: "sce-host-key-generate",
        request: Some("key_generate_request"),
        result: "key_generate_result",
        served_by: COSE_REFERENCE_HOSTS,
    },
    StandardHostType {
        invoke_type: "sce-host-key-public",
        request: Some("key_public_request"),
        result: "key_public_result",
        served_by: COSE_REFERENCE_HOSTS,
    },
    StandardHostType {
        invoke_type: "sce-host-key-retire",
        request: Some("key_retire_request"),
        result: "key_retire_result",
        served_by: COSE_REFERENCE_HOSTS,
    },
    StandardHostType {
        invoke_type: "sce-host-credential-issue",
        request: Some("credential_issue_request"),
        result: "credential_issue_result",
        served_by: COSE_REFERENCE_HOSTS,
    },
    StandardHostType {
        invoke_type: "sce-host-credential-revoke",
        request: Some("credential_revoke_request"),
        result: "credential_revoke_result",
        served_by: COSE_REFERENCE_HOSTS,
    },
    StandardHostType {
        invoke_type: crate::forge::audit_entry::AUDIT_APPEND,
        request: Some("audit_append_request"),
        result: "audit_append_result",
        served_by: COSE_REFERENCE_HOSTS,
    },
];

/// The standard operation `invoke_type` names, if it names one.
pub fn lookup(invoke_type: &str) -> Option<&'static StandardHostType> {
    TYPES.iter().find(|t| t.invoke_type == invoke_type)
}

/// The fields of `invoke_type`'s request that the build writes and a document
/// does not: the answer schema of a model's generation, written from
/// `sce:answer` (docs/adr/0087, section 4.1, item 2), and the marks of an
/// audit entry's schema, written from `sce:entry` (docs/adr/0109, D14). Empty
/// for every other type, standard or not.
pub fn written_by_build(invoke_type: &str) -> &'static [&'static str] {
    if invoke_type == crate::forge::model_answer::MODEL_GENERATE {
        &[crate::forge::model_answer::ANSWER_SCHEMA_FIELD]
    } else if invoke_type == crate::forge::audit_entry::AUDIT_APPEND {
        &[crate::forge::audit_entry::SENSITIVITY_FIELD]
    } else {
        &[]
    }
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

/// The entry of `table` that names `invoke_type` and does not list `lane`
/// among the lanes that serve it, if there is one: the decision
/// [`refuse_unserved`] takes for each `<invoke>`, against the table it is
/// given. A type `table` does not name is no standard type, and is `None`.
pub fn unserved<'t>(
    invoke_type: &str,
    lane: HostLane,
    table: &'t [StandardHostType],
) -> Option<&'t StandardHostType> {
    table
        .iter()
        .find(|standard| standard.invoke_type == invoke_type)
        .filter(|standard| !standard.served_by.contains(&lane))
}

/// [`refuse_unserved`] against `table`: the first `<invoke>` of `model`, in
/// document order, of a type `table` says `lane` does not serve is refused
/// as `codegen/standard-host-type-not-served`, naming the invoke, its type,
/// the lane and the lanes that serve it.
pub fn refuse_unserved_in(
    model: &SCXMLModel,
    lane: HostLane,
    table: &[StandardHostType],
) -> Result<(), GenerateError> {
    for info in host_invokes(model) {
        if let Some(standard) = unserved(&info.invoke_type, lane, table) {
            return Err(GenerateError::CodegenStandardHostTypeNotServed {
                invoke_id: info.base.invoke_id.clone(),
                invoke_type: standard.invoke_type,
                lane,
                served_by: standard.served_by,
                at: info.base.source_location.clone(),
            });
        }
    }
    Ok(())
}

/// Refuse `model` for `lane` when it runs an `<invoke>` of a standard type
/// whose entry in [`TYPES`] does not list that lane (docs/adr/0107, D8). Asked
/// where a machine is written for a lane: for a backend where its artifacts
/// are emitted ([`crate::emit_model_artifacts`]), for the Interpreter where
/// each document of a set is lowered
/// ([`crate::forge::static_js::lower_file_set`]); the parse that runs
/// [`validate`] does not know the lane.
pub fn refuse_unserved(model: &SCXMLModel, lane: HostLane) -> Result<(), GenerateError> {
    refuse_unserved_in(model, lane, TYPES)
}

/// The lanes whose build reads an enum field of a typed host-run request
/// (SCE Accepted Subset §2.12; docs/adr/0109, D0 and step 4 of its order of
/// work): Rust and C++, whose request records hold the field in the machine's
/// own enum and whose adapters read it back by the name it crosses as. Every
/// other lane refuses such a request by name ([`refuse_enum_request_fields`]);
/// a lane joins this list in the change that lands that reading on it.
pub const ENUM_REQUEST_FIELD_LANES: &[HostLane] = &[
    HostLane::Generated(Language::Rust),
    HostLane::Generated(Language::Cpp),
];

/// Refuse `model` for `lane` when the lane is not one of
/// [`ENUM_REQUEST_FIELD_LANES`] and a host-run `<invoke>` of `model` names a
/// request record with an enum field: the first such `<invoke>`, in document
/// order, as `generate/unsupported-feature`, naming the invoke, the field, the
/// lane and the lanes that read one (docs/adr/0109, D0). Asked where
/// [`refuse_unserved`] is, so a standard type a lane does not serve is refused
/// as that first; this refuses a request of any other type, a document's own.
pub fn refuse_enum_request_fields(model: &SCXMLModel, lane: HostLane) -> Result<(), GenerateError> {
    if ENUM_REQUEST_FIELD_LANES.contains(&lane) {
        return Ok(());
    }
    let records = model.entry_records();
    for info in host_invokes(model) {
        let Some(request) = records.get(&info.request_schema) else {
            continue;
        };
        let Some(field) = request
            .fields
            .iter()
            .find(|f| matches!(f.sce_type.present_type(), SceType::Enum(_)))
        else {
            continue;
        };
        let readers: Vec<&str> = ENUM_REQUEST_FIELD_LANES
            .iter()
            .map(|reader| reader.wire_name())
            .collect();
        return Err(GenerateError::unsupported_at(
            format!(
                "<invoke id=\"{}\"> names the request record '{}', whose field '{}' is {}: the {} \
                 lane does not read an enum field of a typed host-run request; it is read on {}",
                info.base.invoke_id,
                info.request_schema,
                field.id,
                field.sce_type.as_attr(),
                lane.wire_name(),
                readers.join(", ")
            ),
            info.request_schema_at
                .clone()
                .or_else(|| info.base.source_location.clone()),
        ));
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SCXMLParser;

    /// A table in which the one operation is served by two lanes, as an
    /// operation added after the first features is (docs/adr/0107, D8).
    const PARTLY_SERVED: &[StandardHostType] = &[StandardHostType {
        invoke_type: "x-probe-op",
        request: None,
        result: "probe_result",
        served_by: &[
            HostLane::Generated(Language::Rust),
            HostLane::Generated(Language::Cpp),
        ],
    }];

    /// The lanes of [`PARTLY_SERVED`]'s operation and of signing and sealing.
    const RUST_AND_CPP: [HostLane; 2] = [
        HostLane::Generated(Language::Rust),
        HostLane::Generated(Language::Cpp),
    ];

    /// A machine that runs a type the table does not name, then the table's
    /// operation, on rows 3 and 4.
    fn machine() -> SCXMLModel {
        SCXMLParser::new()
            .parse_string(
                r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
  <state id="s">
    <invoke type="x-other" id="other"/>
    <invoke type="x-probe-op" id="op"/>
  </state>
</scxml>"#,
                "probe",
            )
            .expect("the probe parses")
    }

    #[test]
    fn a_lane_the_table_does_not_list_is_unserved() {
        let go = HostLane::Generated(Language::Go);
        let entry = unserved("x-probe-op", go, PARTLY_SERVED)
            .expect("go is not among the lanes that serve the operation");
        assert_eq!(entry.invoke_type, "x-probe-op");
        assert!(unserved("x-probe-op", HostLane::Interpreter, PARTLY_SERVED).is_some());
        for lane in RUST_AND_CPP {
            assert!(unserved("x-probe-op", lane, PARTLY_SERVED).is_none());
        }
        assert!(
            unserved("x-other", go, PARTLY_SERVED).is_none(),
            "a type the table does not name is no standard type, and is never refused here"
        );
    }

    #[test]
    fn a_machine_that_runs_an_unserved_type_is_refused_naming_the_lanes_that_serve_it() {
        let refused =
            refuse_unserved_in(&machine(), HostLane::Generated(Language::Go), PARTLY_SERVED)
                .expect_err("go does not serve x-probe-op, and the machine was not refused");
        assert_eq!(
            refused.to_string(),
            "<invoke id=\"op\"> runs the standard host type 'x-probe-op', which the go lane \
             does not serve: it is served on rust, cpp"
        );
        let GenerateError::CodegenStandardHostTypeNotServed {
            invoke_id,
            invoke_type,
            lane,
            served_by,
            at,
        } = refused
        else {
            panic!("the machine was refused as another error");
        };
        assert_eq!(invoke_id, "op");
        assert_eq!(invoke_type, "x-probe-op");
        assert_eq!(lane, HostLane::Generated(Language::Go));
        assert_eq!(served_by, &RUST_AND_CPP[..]);
        assert_eq!(
            at.and_then(|at| at.line),
            Some(4),
            "the refusal is placed on the <invoke> that runs the type"
        );
    }

    /// The Interpreter is a lane of the table as a backend is, and a lowering
    /// for it is refused by the name `interpreter`.
    #[test]
    fn a_machine_lowered_for_the_interpreter_is_refused_a_type_it_does_not_serve() {
        let refused = refuse_unserved_in(&machine(), HostLane::Interpreter, PARTLY_SERVED)
            .expect_err("the Interpreter does not serve x-probe-op, and was not refused");
        assert_eq!(
            refused.to_string(),
            "<invoke id=\"op\"> runs the standard host type 'x-probe-op', which the \
             interpreter lane does not serve: it is served on rust, cpp"
        );
    }

    #[test]
    fn every_lane_is_named_once_backends_first_then_the_interpreter() {
        let names: Vec<&str> = HostLane::all().map(HostLane::wire_name).collect();
        assert_eq!(
            names,
            [
                "rust",
                "cpp",
                "kotlin",
                "go",
                "python",
                "c11",
                "swift",
                "interpreter"
            ]
        );
    }

    #[test]
    fn a_machine_is_generated_for_a_lane_that_serves_what_it_runs() {
        for lane in RUST_AND_CPP {
            assert!(
                refuse_unserved_in(&machine(), lane, PARTLY_SERVED).is_ok(),
                "{lane:?} serves x-probe-op and was refused"
            );
        }
    }

    /// Every operation of [`EVERY_REFERENCE_HOST`] is served on every lane, so
    /// the refusal refuses none of them. A backend added to [`Language`] reds
    /// this until it is decided: [`EVERY_REFERENCE_HOST`] gains it in the
    /// change that lands its reference host, and not before.
    #[test]
    fn the_standard_table_refuses_no_operation_of_every_reference_host_on_any_lane() {
        let mut judged = 0;
        for standard in TYPES
            .iter()
            .filter(|standard| standard.served_by == EVERY_REFERENCE_HOST)
        {
            for lane in HostLane::all() {
                assert!(
                    unserved(standard.invoke_type, lane, TYPES).is_none(),
                    "{} is not served on {lane:?}",
                    standard.invoke_type
                );
                judged += 1;
            }
        }
        assert!(
            judged >= 12 * HostLane::all().count(),
            "judged {judged} pairs of an operation and a lane: a test that read none would pass"
        );
    }

    /// Signing and sealing, random bytes, keyed digests and keys are served by
    /// the Rust and C++ reference hosts and refused on every other lane, naming
    /// those two (docs/adr/0107, D7, D8; docs/adr/0109, D0).
    #[test]
    fn signing_and_sealing_are_served_on_rust_and_cpp_and_refused_elsewhere() {
        let cose: Vec<&StandardHostType> = TYPES
            .iter()
            .filter(|standard| standard.served_by == COSE_REFERENCE_HOSTS)
            .collect();
        let names: Vec<&str> = cose.iter().map(|standard| standard.invoke_type).collect();
        assert_eq!(
            names,
            [
                "sce-host-cose-sign1",
                "sce-host-cose-verify1",
                "sce-host-cose-encrypt0",
                "sce-host-cose-decrypt0",
                "sce-host-key-derive",
                "sce-host-random-bytes",
                "sce-host-mac",
                "sce-host-mac-verify",
                "sce-host-key-generate",
                "sce-host-key-public",
                "sce-host-key-retire",
                "sce-host-credential-issue",
                "sce-host-credential-revoke",
                "sce-host-audit-append",
            ]
        );
        for standard in cose {
            for lane in HostLane::all() {
                let refused = unserved(standard.invoke_type, lane, TYPES);
                if RUST_AND_CPP.contains(&lane) {
                    assert!(
                        refused.is_none(),
                        "{} is refused on {lane:?}",
                        standard.invoke_type
                    );
                } else {
                    assert_eq!(
                        refused.map(|entry| entry.served_by),
                        Some(&RUST_AND_CPP[..]),
                        "{} is served on {lane:?}",
                        standard.invoke_type
                    );
                }
            }
        }
    }

    /// Each entry lists at least one lane, each once, in the order of
    /// [`HostLane::all`], which is the order a refusal names them in.
    #[test]
    fn each_operation_names_its_lanes_once_in_the_order_of_every_lane() {
        for standard in TYPES.iter().chain(PARTLY_SERVED) {
            assert!(
                !standard.served_by.is_empty(),
                "{} is served on no lane",
                standard.invoke_type
            );
            let positions: Vec<usize> = standard
                .served_by
                .iter()
                .map(|lane| {
                    HostLane::all()
                        .position(|every| every == *lane)
                        .expect("a lane of HostLane::all")
                })
                .collect();
            assert!(
                positions.windows(2).all(|pair| pair[0] < pair[1]),
                "{} names its lanes out of order or twice: {:?}",
                standard.invoke_type,
                standard.served_by
            );
        }
    }
}
