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

use crate::forge::error::{ForgeError, Located, ValidationError};
use crate::forge::expression_site::ExpressionSite;
use crate::forge::limits::resolve_bytes_max;
use crate::forge::model::{ProcedureModel, ProcedureState, SceType};

/// What a `<send>`'s `sce:payload` may be, as a refusal names it
/// (`SCE_FORGE.md` §4.5).
pub const PAYLOAD_RULE: &str = "bytes — a codec's encode_to_vec(), or a bytes field; \
     a scalar has no payload meaning without an endianness and a width, which is \
     the decision a codec makes";

/// What a `<send>`'s `sce:addr` may be, as a refusal names it
/// (`SCE_FORGE.md` §4.5).
pub const ADDRESS_RULE: &str = "an integer, sent in decimal, or a string, sent as written \
     — the text every backend spells alike";

/// How a `<send>`'s address reaches the service request, whose address is
/// text in every runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressForm {
    /// An unsigned integer, sent in decimal.
    Unsigned,
    /// A signed integer, sent in decimal with its sign.
    Signed,
    /// A string, sent as written.
    Text,
}

impl AddressForm {
    /// The name a template branches on.
    pub fn as_str(self) -> &'static str {
        match self {
            AddressForm::Unsigned => "unsigned",
            AddressForm::Signed => "signed",
            AddressForm::Text => "text",
        }
    }
}

/// The form `send`'s address reaches the service request in, or `None`
/// when the send has no address. Decided once, from the expression's type,
/// so every backend sends the same text.
///
/// ⚠ Each backend used to decide for itself: Python sent a boolean as
/// `True` where Rust, Kotlin and Go sent `true` and C++ sent `1`, a float
/// took as many digits as each language's own formatting chose, C++ could
/// not compile a string address at all (`std::to_string` of a string), and
/// C sent every address as `""` — read from the six templates' conversions
/// on 2026-09-22, each a fact of its language's library. An integer and a
/// string are the two types every backend spells alike, so an address of
/// any other type — or of one this document does not establish — is
/// refused as `validation/send-operand-type`, on the attribute's row.
pub fn address_form(
    send: &crate::forge::model::ProcedureSendAction,
    type_ctx: &crate::forge::types::TypeCtx<'_>,
) -> Result<Option<AddressForm>, ForgeError> {
    use crate::forge::types::InferredType;

    let Some(addr) = send.addr.as_deref() else {
        return Ok(None);
    };
    let site = ExpressionSite::new(addr, send.addr_spelling.as_ref());
    // The names first. An undeclared one infers as no type at all, and
    // judging that as the address's type would report the type while the
    // name is what is wrong — which is what this did until 2026-09-23.
    let ast = crate::forge::expr::resolve(addr, type_ctx).map_err(|refusal| site.place(refusal))?;
    let found = match ast.ty {
        InferredType::Int { signed: false, .. } => return Ok(Some(AddressForm::Unsigned)),
        InferredType::Int { signed: true, .. } | InferredType::UntypedInt => {
            return Ok(Some(AddressForm::Signed))
        }
        InferredType::Str => return Ok(Some(AddressForm::Text)),
        InferredType::Bool => "bool".to_string(),
        InferredType::Float { bits } => format!("float{bits}"),
        InferredType::UntypedFloat => "a floating-point number".to_string(),
        InferredType::Bytes => "bytes".to_string(),
        InferredType::Null => "null".to_string(),
        InferredType::Quantity { .. } => "a physical quantity".to_string(),
        _ => "of no type this document establishes".to_string(),
    };
    let expr = addr.trim();
    let at = site.locate(Some(0..expr.len()));
    let refusal: ForgeError = ValidationError::SendOperandType {
        service: send.service.clone(),
        attr: "sce:addr",
        observed: at.observed().unwrap_or_else(|| expr.to_string()),
        found,
        rule: ADDRESS_RULE,
    }
    .into();
    Err(at.place(refusal))
}

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
///
/// The refusal names the field as the attribute spells it, on the row it
/// sits on. It carried no line until 2026-09-22 and put the sentence
/// `"<name> (declared <type>)"` where its `actual` belongs, which no row
/// of any document holds.
pub fn validate_payload_is_bytes(
    model: &ProcedureModel,
    doc_name: &str,
) -> Result<(), Located<ForgeError>> {
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
            let at = ExpressionSite::new(payload, send.payload_spelling.as_ref())
                .locate(Some(0..name.len()));
            return Err(Located::new(
                ValidationError::SendOperandType {
                    service: send.service.clone(),
                    attr: "sce:payload",
                    observed: at.observed().unwrap_or_else(|| name.to_string()),
                    found: format!("declared {}", field.sce_type.as_attr()),
                    rule: PAYLOAD_RULE,
                }
                .into(),
                doc_name,
                at.line,
                at.col,
            ));
        }
    }
    Ok(())
}

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
                expr_spelling: None,
                expr_splices: None,
                quantity: None,
                max_size: seed_cap,
                default_covers: Vec::new(),
                retain: None,
                initial: None,
                initial_spelling: None,
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
                        cond_spelling: None,
                        event: Some("ok".to_string()),
                        assigns: vec![ProcedureAssign {
                            location: "seed".to_string(),
                            location_spelling: None,
                            expr: "_event.data".to_string(),
                            expr_spelling: None,
                        }],
                        line: None,
                    }],
                    on_entry_sends: vec![ProcedureSendAction {
                        service: "SecurityAccess".to_string(),
                        subfunc: Some("0x01".to_string()),
                        addr: None,
                        addr_spelling: None,
                        payload: None,
                        payload_spelling: None,
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

    /// A scalar payload is refused on the row its attribute sits on,
    /// naming the field as written — the attribute here is on a row of its
    /// own, below the `<send` it belongs to.
    #[test]
    fn a_scalar_payload_is_refused_on_its_own_row() {
        use crate::forge::diagnostic::ToDiagnostics;
        let document = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="procedure" name="probe" initial="a" version="1.0">
  <datamodel><data id="fileId" sce:type="uint32" sce:direction="in"/></datamodel>
  <state id="a">
    <onentry>
      <send sce:service="svc"
            sce:payload="fileId"/>
    </onentry>
    <transition event="ok" target="d"/>
  </state>
  <final id="d"/>
</scxml>"#;
        let label = crate::DocumentLabel {
            identifier: "probe",
            diagnostic_label: "probe.scxml",
        };
        let refusal = crate::forge::parser::parse_forge_with_imports(document, label)
            .expect_err("a uint32 payload is refused");
        let record = &refusal.error.to_diagnostics()[0];
        assert_eq!(
            serde_json::to_string(&record.code).unwrap(),
            "\"validation/send-operand-type\""
        );
        assert_eq!(record.actual.as_deref(), Some("fileId"));
        assert_eq!(
            (refusal.location.line, refusal.location.col),
            (Some(7), Some(26)),
            "{refusal:?}"
        );
    }

    /// An address of a type no two backends spell alike is refused, on the
    /// row its attribute sits on and naming it as written; the three forms
    /// every backend spells alike are told apart by the expression's type.
    #[test]
    fn an_address_is_an_integer_or_a_string() {
        use crate::forge::diagnostic::ToDiagnostics;
        let document = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="procedure" name="probe" initial="a" version="1.0">
  <datamodel>
    <data id="armed" sce:type="bool" sce:direction="in"/>
    <data id="ecu" sce:type="uint32" sce:direction="in"/>
    <data id="offset" sce:type="int16" sce:direction="in"/>
    <data id="gateway" sce:type="string" sce:direction="in"/>
  </datamodel>
  <state id="a">
    <onentry>
      <send sce:service="svc"
            sce:addr="armed"/>
      <send sce:service="svc" sce:addr="ecu"/>
      <send sce:service="svc" sce:addr="offset"/>
      <send sce:service="svc" sce:addr="gateway"/>
    </onentry>
    <transition event="ok" target="d"/>
  </state>
  <final id="d"/>
</scxml>"#;
        let label = crate::DocumentLabel {
            identifier: "probe",
            diagnostic_label: "probe.scxml",
        };
        let parsed = crate::forge::parser::parse_forge_with_imports(document, label)
            .expect("the document parses")
            .expect("a forge document");
        let crate::forge::model::ForgeDocument::Procedure(model) = &parsed.document else {
            panic!("a procedure: {:?}", parsed.document);
        };
        let ctx = crate::forge::type_ctx::procedure(model, &[]);
        let sends = &model.states[0].on_entry_sends;

        let refusal = Located::in_file(
            address_form(&sends[0], &ctx).expect_err("a bool address is refused"),
            "probe.scxml",
        );
        let record = &refusal.error.to_diagnostics()[0];
        assert_eq!(
            serde_json::to_string(&record.code).unwrap(),
            "\"validation/send-operand-type\""
        );
        assert_eq!(record.actual.as_deref(), Some("armed"));
        assert_eq!(refusal.location.line, Some(12), "{refusal:?}");

        let forms: Vec<_> = sends[1..]
            .iter()
            .map(|send| address_form(send, &ctx).expect("an admitted address"))
            .collect();
        assert_eq!(
            forms,
            [
                Some(AddressForm::Unsigned),
                Some(AddressForm::Signed),
                Some(AddressForm::Text)
            ]
        );
    }
}
