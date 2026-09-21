// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! SCE Forge — semantic validation pass for the bytes max-size
//! contract (`sce:max-size` / `sce:response-max-size` /
//! `sce:returns-max-size` caps on bytes-typed slots).
//!
//! Runs after [`crate::forge::parser::parse_procedure`] has built a
//! [`ProcedureModel`]; catches inconsistencies between two declared
//! caps before any backend codegen runs. The runtime enforcement path
//! (`error.execution` raised on actual cap violations) is the safety
//! net for cases where the declarations are consistent but real data
//! exceeds the cap; this pass is the static counterpart that prevents
//! self-contradicting SCXML from compiling at all.

use crate::forge::error::ValidationError;
use crate::forge::limits::resolve_bytes_max;
use crate::forge::model::{ProcedureModel, ProcedureState, SceType};

/// Validate that every declared `sce:max-size` / `sce:response-max-size`
/// / `sce:returns-max-size` annotation in the procedure is consistent
/// with the destination slot it ultimately fills.
///
/// Rule (RFC §bytesguard-3 B1): for every `<assign location="X" expr="_event.data"/>`
/// inside a transition leaving a state whose `<onentry>` contains a
/// `<send>` with `sce:response-max-size=M`, and where `X` is a bytes-typed
/// slot with `sce:max-size=N`, the constraint `M ≤ N` must hold. The
/// resolved cap (annotation, else [`crate::forge::limits::BYTES_DEFAULT_MAX`])
/// is the comparison target.
///
/// Returns the first violation encountered, mirroring the existing
/// [`crate::forge::parser::parse_procedure`] convention of emitting one
/// validation error at a time. Multi-violation aggregation is
/// out-of-scope for this pass (consistent with how `parse_procedure`
/// itself short-circuits on the first failure).
/// Refuse a `sce:payload` that names a declared field which is not bytes.
///
/// A procedure's payload is a wire blob: every runtime in this crate types
/// it as raw bytes, and `ProcedureServiceTypes.h` says why in its own words
/// — the value comes from a codec's `encode_to_vec()`. Handing it a scalar
/// has no meaning without an endianness and a width, which is the decision a
/// codec exists to make.
///
/// ⚠ Before this, such a document generated with rc=0 and emitted
/// `req.payload = themeFileId_;` — assigning a `uint32_t` to an
/// `optional<vector<uint8_t>>`, which does not compile. The refusal arrived
/// in a consumer's build log instead of on the document that caused it
/// (measured 2026-09-17).
///
/// ⚠⚠ Only what can be PROVEN wrong is refused: a payload expression that
/// is exactly the identifier of a declared input or internal whose declared
/// type is not `bytes`. A call (`frame.encode_to_vec()`), a member access,
/// or anything this pass cannot resolve is left alone — a checker that
/// guessed at the rest would refuse working documents, which is the failure
/// this one is meant to prevent, pointed the other way.
pub fn validate_payload_is_bytes(
    model: &crate::forge::model::ProcedureModel,
) -> Result<(), Box<ValidationError>> {
    use crate::forge::model::SceType;

    for state in &model.states {
        for send in &state.on_entry_sends {
            let Some(payload) = send.payload.as_ref() else {
                continue;
            };
            let name = payload.trim();
            let declared = model
                .inputs
                .iter()
                .chain(model.internals.iter())
                .find(|f| f.id == name);
            let Some(field) = declared else {
                continue; // not a bare field name: nothing proven
            };
            if matches!(field.sce_type, SceType::Bytes) {
                continue;
            }
            return Err(Box::new(ValidationError::AttributeRuleViolated {
                element: format!("<send sce:service=\"{}\">", send.service),
                attr: "sce:payload".into(),
                value: format!("{name} (declared {})", field.sce_type.as_attr()),
                rule: "bytes — a codec's encode_to_vec(), or a bytes field. \
                           A scalar has no payload meaning without an endianness \
                           and a width, which is the decision a codec makes"
                    .into(),
            }));
        }
    }
    Ok(())
}

pub fn validate_bytes_max_size_consistency(
    model: &ProcedureModel,
) -> Result<(), Box<ValidationError>> {
    // Build a map: slot id -> resolved cap. Only bytes-typed slots
    // participate; everything else is irrelevant to this contract.
    let mut slot_caps: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
    for f in model.inputs.iter().chain(model.internals.iter()) {
        if matches!(f.sce_type, SceType::Bytes) {
            slot_caps.insert(f.id.as_str(), resolve_bytes_max(f.max_size));
        }
    }

    // For every state that has both an `<onentry><send>` with
    // response-max-size declared and an outgoing transition whose
    // assign reads `_event.data` into a bytes slot, compare caps.
    for state in model.states.iter().filter(|s| !s.is_final) {
        check_state_response_assigns(state, &slot_caps, &model.name)?;
    }

    Ok(())
}

fn check_state_response_assigns(
    state: &ProcedureState,
    slot_caps: &std::collections::HashMap<&str, u32>,
    procedure_name: &str,
) -> Result<(), Box<ValidationError>> {
    // Resolve the response cap for this state: maximum across every
    // `<onentry><send>`, with a missing annotation falling through to
    // the default cap. Falling through is load-bearing — a state
    // dispatching with no annotation still bounds responses at the
    // default, and the destination slot must accommodate that. If the
    // state has no sends at all, there is no response to bound and
    // the check does not apply.
    let response_cap = state
        .on_entry_sends
        .iter()
        .map(|s| resolve_bytes_max(s.response_max_size))
        .max();
    let Some(response_cap) = response_cap else {
        return Ok(());
    };
    let representative_service = state
        .on_entry_sends
        .first()
        .map_or("?", |s| s.service.as_str());

    for transition in &state.transitions {
        for assign in &transition.assigns {
            if assign.expr.trim() != "_event.data" {
                continue;
            }
            let Some(&slot_cap) = slot_caps.get(assign.location.as_str()) else {
                continue;
            };
            if response_cap > slot_cap {
                return Err(Box::new(ValidationError::BytesMaxSizeViolation {
                    procedure: procedure_name.to_string(),
                    detail: format!(
                        "<send sce:service=\"{representative_service}\"> sce:response-max-size={response_cap} \
                         exceeds destination slot '{}' sce:max-size={slot_cap}",
                        assign.location
                    ),
                }));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::model::{
        Direction, ForgeField, ProcedureAssign, ProcedureHelper, ProcedureSendAction,
        ProcedureState, ProcedureTransition,
    };

    /// Build a minimal procedure mimicking security_access's
    /// `requestSeed` → `<assign>seed = _event.data` chain. All caps are
    /// supplied by the test so each case can assert pass/fail
    /// deterministically without a shared default fall-through.
    fn build_model(seed_cap: Option<u32>, response_cap: Option<u32>) -> ProcedureModel {
        ProcedureModel {
            name: "security_access".to_string(),
            inputs: Vec::new(),
            internals: vec![ForgeField {
                id: "seed".to_string(),
                sce_type: SceType::Bytes,
                direction: Direction::Internal,
                expr: None,
                expr_line: None,
                quantity: None,
                max_size: seed_cap,
                default_covers: Vec::new(),
                retain: None,
            }],
            helpers: vec![ProcedureHelper {
                name: "computeKey".to_string(),
                args: vec![SceType::Bytes],
                returns: SceType::Bytes,
                returns_max_size: None,
            }],
            initial: "requestSeed".to_string(),
            states: vec![
                ProcedureState {
                    id: "requestSeed".to_string(),
                    is_final: false,
                    transitions: vec![ProcedureTransition {
                        target: "done".to_string(),
                        cond: None,
                        event: Some("ok".to_string()),
                        assigns: vec![ProcedureAssign {
                            location: "seed".to_string(),
                            expr: "_event.data".to_string(),
                        }],
                        line: None,
                    }],
                    on_entry_sends: vec![ProcedureSendAction {
                        service: "SecurityAccess".to_string(),
                        subfunc: Some("0x01".to_string()),
                        addr: None,
                        payload: None,
                        response_max_size: response_cap,
                    }],
                    done_params: Vec::new(),
                    line: None,
                },
                ProcedureState {
                    id: "done".to_string(),
                    is_final: true,
                    transitions: Vec::new(),
                    on_entry_sends: Vec::new(),
                    done_params: Vec::new(),
                    line: None,
                },
            ],
            source_location: None,
        }
    }

    #[test]
    fn consistent_caps_pass() {
        // Both 64; security_access's actual annotation shape.
        let model = build_model(Some(64), Some(64));
        assert!(validate_bytes_max_size_consistency(&model).is_ok());
    }

    #[test]
    fn equal_caps_pass() {
        // Boundary case: response cap equals slot cap → allowed.
        let model = build_model(Some(128), Some(128));
        assert!(validate_bytes_max_size_consistency(&model).is_ok());
    }

    #[test]
    fn response_exceeds_slot_fails() {
        let model = build_model(Some(64), Some(128));
        let err = validate_bytes_max_size_consistency(&model).unwrap_err();
        match *err {
            ValidationError::BytesMaxSizeViolation { procedure, detail } => {
                assert_eq!(procedure, "security_access");
                assert!(detail.contains("response-max-size=128"));
                assert!(detail.contains("sce:max-size=64"));
                assert!(detail.contains("seed"));
            }
            other => panic!("expected BytesMaxSizeViolation, got {other:?}"),
        }
    }

    #[test]
    fn missing_response_cap_uses_default() {
        // No annotation on send → falls through to BYTES_DEFAULT_MAX
        // = 256. Slot declares 64 → 256 > 64 → violation.
        let model = build_model(Some(64), None);
        assert!(validate_bytes_max_size_consistency(&model).is_err());
    }

    #[test]
    fn missing_slot_cap_uses_default() {
        // No annotation on slot → 256. Send declares 64 → 64 ≤ 256 → ok.
        let model = build_model(None, Some(64));
        assert!(validate_bytes_max_size_consistency(&model).is_ok());
    }

    #[test]
    fn no_caps_use_default_both_sides() {
        // Both default to 256 → equal → ok. Confirms the default
        // resolver gates the check symmetrically.
        let model = build_model(None, None);
        assert!(validate_bytes_max_size_consistency(&model).is_ok());
    }
}
