// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Where a deployment proves to a machine the person an instance acts for (docs/adr/0116,
//! D10): it binds the machine to at least one open machine, each binding of one proving a
//! person as D2 has it. One predicate ([`proof_of_person`]) decides two things. A document
//! that declares `<scxml sce:principal>` reads the person of an event an instance of an open
//! machine sent, and is bound nowhere a person is not proved ([`check_bound`]). And a machine
//! holds the person its events and runs are said for wherever one is proved to it, whether or
//! not its document reads one ([`proves_person`]), so that who did what is recorded by the
//! platform (D7), and holds none anywhere else, where no person can be told to it.
//!
//! * **Under the `key` authenticator.** A proof of a person is bound to the key the
//!   instance's exchanges are signed by (its challenge names the key's thumbprint); every
//!   other authenticator proves no key, and a proof made for it would be of use elsewhere.
//! * **With `sign:` and `verify:`.** The exchange that confirms an offer carries the proof
//!   and is answered by the server; an answer no key of the server's signs, or that the
//!   instance verifies under no trust set, is one a link between them could have written
//!   (docs/adr/0113, D4).
//! * **Under `receipt: durable`.** An event is told the person of the incarnation it was
//!   taken in under, and only an instance's completion records judge an envelope by its
//!   incarnation (docs/adr/0113, D5): an event that came under an incarnation that ended
//!   while it waited is fenced, and is never run as nobody's (D6).
//!
//! An https link names the instance that sends (SCE_MESH.md section 22.9), and the binding
//! of an open machine is held to https where the deployment is read
//! (`mesh::deploy::validate_instances`), so a transport that names no sender is refused
//! there. A machine generated for another backend than Rust is refused where its document
//! is lowered (`forge::static_lowering::refuse_principal`).

use crate::mesh::deploy::{BindingConfig, DeployConfig, ReceiptMode};
use crate::mesh::error::CodegenError;
use crate::mesh::target::TargetId;
use crate::model::SCXMLModel;

/// The authenticator a binding names when it names none (SCE_MESH.md section 22.5).
const DEFAULT_AUTHENTICATOR: &str = "bearer";

/// Refuse `machine`, whose document is `model`, as [`CodegenError::PrincipalUnproved`] when
/// it reads the person an instance acts for and `deploy` does not prove one to it
/// ([`proof_of_person`]), naming why. A machine whose document reads none is not judged: where
/// no person is proved to it, it holds none ([`proves_person`]).
pub fn check_bound(
    machine: &str,
    model: &SCXMLModel,
    deploy: &DeployConfig,
) -> Result<(), CodegenError> {
    if model.principal.is_none() {
        return Ok(());
    }
    proof_of_person(machine, deploy).map_err(|reason| CodegenError::PrincipalUnproved {
        machine: machine.to_string(),
        reason,
    })
}

/// Whether `deploy` proves to `machine` the person an instance acts for (docs/adr/0116,
/// D10): the predicate [`proof_of_person`] answers, without its reason. The Rust generation
/// holds the person in the machine's type where it does (`StatePolicy::Principal`), and
/// holds none where it does not.
pub fn proves_person(machine: &str, deploy: &DeployConfig) -> bool {
    proof_of_person(machine, deploy).is_ok()
}

/// `Ok` when `deploy` proves to `machine` the person an instance acts for (docs/adr/0116,
/// D10), and why not otherwise: it binds the machine to no open machine, or binds it to one
/// on a binding that is not under the `key` authenticator, that names no `sign:` (or whose
/// instances' binding names no `verify:`), or that does not receive durably. Bindings are
/// judged in target order, so the reason names the first.
pub fn proof_of_person(machine: &str, deploy: &DeployConfig) -> Result<(), String> {
    let entry = deploy
        .topology
        .values()
        .find_map(|device| device.machines.get(machine));
    let mut bindings: Vec<(&TargetId, &BindingConfig)> = entry
        .map(|entry| {
            entry
                .bindings
                .iter()
                .filter(|(target, _)| deploy.is_open_machine(target.name()))
                .collect()
        })
        .unwrap_or_default();
    bindings.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));
    if bindings.is_empty() {
        return Err(
            "it is bound to no instance of an open machine (`instances: open`), whose events \
             alone are said for a person"
                .to_string(),
        );
    }
    for (target, binding) in bindings {
        let at = format!("its binding '{}'", target.as_str());
        let authenticator = binding
            .extra
            .get("authenticator")
            .and_then(|a| a.as_str())
            .unwrap_or(DEFAULT_AUTHENTICATOR);
        if authenticator != "key" {
            return Err(format!(
                "{at} is under the `{authenticator}` authenticator, which proves no key a proof \
                 of a person is bound to"
            ));
        }
        if !binding.extra.contains_key("sign") {
            return Err(format!(
                "{at} names no `sign:`, so no key of the server's signs the answer to the \
                 exchange that carries a proof"
            ));
        }
        let verified = deploy
            .topology
            .values()
            .find_map(|device| device.machines.get(target.name()))
            .and_then(|open| {
                open.bindings
                    .iter()
                    .find(|(back, _)| back.name() == machine)
            })
            .is_some_and(|(_, back)| back.extra.contains_key("verify"));
        if !verified {
            return Err(format!(
                "the binding of '{}' to it names no `verify:`, so an instance verifies no answer \
                 of the server's",
                target.name()
            ));
        }
        if binding.receipt != ReceiptMode::Durable {
            return Err(format!(
                "{at} does not receive under `receipt: durable`, and an event is told the person \
                 of the incarnation it was taken in under, which only an instance's completion \
                 records judge (docs/adr/0113, D5)"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::deploy::parse_deploy_str;

    /// A server `rooms` joined to the open machine `phone`, `rooms_binding` and
    /// `phone_binding` after each binding's keys.
    fn joined(rooms_binding: &str, phone_binding: &str) -> String {
        format!(
            r##"
version: "1.0"
topology:
  cloud:
    machines:
      rooms:
        source: rooms.scxml
        persistence: checkpoint
        bindings:
          "#phone": {{ transport: https, listen: "127.0.0.1:9443"{rooms_binding} }}
  handset:
    machines:
      phone:
        source: phone.scxml
        instances: open
        members: {{ lease_ms: 60000, provisional_ms: 1000, max_instances: 4 }}
        persistence: checkpoint
        bindings:
          "#rooms": {{ transport: https, url: "https://rooms.example"{phone_binding} }}
"##
        )
    }

    const PROVED: &str = ", authenticator: key, fresh_ms: 60000, sign: rooms-key, receipt: durable";
    const VERIFIED: &str = ", authenticator: key, verify: rooms-trust";

    fn reading_a_person() -> SCXMLModel {
        SCXMLModel {
            principal: Some("Person".to_string()),
            ..SCXMLModel::default()
        }
    }

    fn refusal(rooms_binding: &str, phone_binding: &str) -> String {
        let deploy = parse_deploy_str(&joined(rooms_binding, phone_binding))
            .expect("a deployment the parser takes");
        match check_bound("rooms", &reading_a_person(), &deploy) {
            Err(CodegenError::PrincipalUnproved { machine, reason }) => {
                assert_eq!(machine, "rooms");
                reason
            }
            other => panic!("expected PrincipalUnproved, got {other:?}"),
        }
    }

    /// docs/adr/0116, D10: a machine that reads a person is bound to an open machine under
    /// the `key` authenticator, with `sign:` and `verify:`, receiving durably; each one
    /// missing is refused by name and proves no person, and a machine that reads none is not
    /// judged. Red if `proof_of_person` takes any of the four.
    #[test]
    fn a_machine_that_reads_a_person_is_bound_where_one_is_proved() {
        let deploy = parse_deploy_str(&joined(PROVED, VERIFIED)).expect("a deployment");
        assert!(check_bound("rooms", &reading_a_person(), &deploy).is_ok());
        assert!(check_bound("phone", &reading_a_person(), &deploy)
            .is_err_and(|e| e.to_string().contains("no instance of an open machine")));
        assert!(check_bound("phone", &SCXMLModel::default(), &deploy).is_ok());
        for (rooms_binding, phone_binding, needle) in [
            (
                ", authenticator: install, sign: rooms-key, receipt: durable",
                ", authenticator: install, verify: rooms-trust",
                "`install` authenticator",
            ),
            (
                ", authenticator: key, fresh_ms: 60000, receipt: durable",
                VERIFIED,
                "no `sign:`",
            ),
            (PROVED, ", authenticator: key", "no `verify:`"),
            (
                ", authenticator: key, fresh_ms: 60000, sign: rooms-key",
                VERIFIED,
                "`receipt: durable`",
            ),
        ] {
            let reason = refusal(rooms_binding, phone_binding);
            assert!(reason.contains(needle), "{needle}: {reason}");
            let deploy = parse_deploy_str(&joined(rooms_binding, phone_binding))
                .expect("a deployment the parser takes");
            assert!(!proves_person("rooms", &deploy), "{needle}");
        }
    }

    /// docs/adr/0116, D7 and D10: one predicate decides both where a document that reads a
    /// person may be bound and where a machine holds the person its events are said for, so
    /// the two never part; it asks nothing of the document, and the refusal is asked of a
    /// document that reads a person alone. Red if `proves_person` judges other bindings than
    /// `check_bound` refuses, or if `check_bound` refuses a document that reads none.
    #[test]
    fn one_predicate_decides_the_refusal_and_where_a_person_is_held() {
        let proved = parse_deploy_str(&joined(PROVED, VERIFIED)).expect("a deployment");
        assert!(proves_person("rooms", &proved));
        assert!(
            !proves_person("phone", &proved),
            "the open machine is bound to no open machine"
        );
        let unproved = parse_deploy_str(&joined(
            ", authenticator: key, fresh_ms: 60000, sign: rooms-key",
            VERIFIED,
        ))
        .expect("a deployment");
        assert!(!proves_person("rooms", &unproved));
        assert!(
            check_bound("rooms", &SCXMLModel::default(), &unproved).is_ok(),
            "a document that reads no person is not refused where none is proved"
        );
        match check_bound("rooms", &reading_a_person(), &unproved) {
            Err(CodegenError::PrincipalUnproved { reason, .. }) => {
                assert_eq!(Err(reason), proof_of_person("rooms", &unproved));
            }
            other => panic!("expected PrincipalUnproved, got {other:?}"),
        }
    }
}
