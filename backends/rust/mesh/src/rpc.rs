// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The requester's half of `<invoke type="sce:mesh-rpc">` (SCE_MESH.md
//! §mesh-9.5): the correlation table a request is kept in until its reply,
//! its deadline or its cancellation retires it, and the `_event.data` the
//! engine is handed when it ends in an error.
//!
//! The engine already carries the SCXML invocation — started once, cancelled
//! when its state exits, ended by exactly one `done.invoke.<id>` or
//! `error.invoke.<id>` (§mesh-19). What this module adds is what only the
//! router knows: which wire id answers which invocation, who may answer it
//! (§mesh-14.6), and when it stops waiting.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::generated::rpc_status::RpcStatus;
use crate::signal::Json;

/// The `<param>` a lowered Mesh request carries its event name in.
pub const MESH_EVENT_PARAM: &str = "_mesh_event";
/// The `<param>` a lowered Mesh request carries its own deadline in, when the
/// document gave one (§mesh-9.5 deadline precedence).
pub const MESH_DEADLINE_PARAM: &str = "_mesh_deadline_ms";

/// One request waiting for its answer.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Pending {
    /// The SCXML invoke id the answer ends (`done.invoke.<id>`).
    pub invoke_id: String,
    /// Which start of that invoke this is (the engine's token).
    pub token: u64,
    /// The peer it was sent to, whose link carries its reply.
    pub target: String,
    /// The machines whose reply may answer it (§mesh-14.6).
    pub responders: &'static [&'static str],
    /// The monotonic time it stops waiting, if it has a deadline.
    pub expires_ms: Option<i64>,
}

/// Every request this router has sent and not yet retired, keyed by the
/// wire `invoke_id` the requester minted for it (§mesh-9.5: not the SCXML
/// invoke id, which never crosses the wire).
#[derive(Debug, Default)]
pub(crate) struct Correlation {
    pending: BTreeMap<[u8; 16], Pending>,
}

/// How a reply that names a live request stands against it.
pub(crate) enum Answer {
    /// No request is waiting on that id: it was answered, cancelled or
    /// expired already, and what arrives for it now is dropped (§mesh-9.5).
    Unknown,
    /// The request is waiting and the binding the reply arrived on may
    /// answer it.
    Admitted,
    /// The request is waiting but that binding is not in its responder set
    /// (§mesh-16.7 row 14). The request stays answerable.
    Undeclared,
}

impl Correlation {
    pub fn register(&mut self, wire_id: [u8; 16], pending: Pending) {
        self.pending.insert(wire_id, pending);
    }

    /// Whether a reply for `wire_id` that arrived on `peer` may retire it.
    /// The binding a reply arrived on is what identifies its responder,
    /// never the envelope's `source`, which the sender writes (§mesh-9.5).
    pub fn check(&self, wire_id: &[u8; 16], peer: &str) -> Answer {
        match self.pending.get(wire_id) {
            None => Answer::Unknown,
            Some(pending) if pending.responders.contains(&peer) => Answer::Admitted,
            Some(_) => Answer::Undeclared,
        }
    }

    /// Whether a request is still waiting on `wire_id`.
    pub fn is_waiting(&self, wire_id: &[u8; 16]) -> bool {
        self.pending.contains_key(wire_id)
    }

    /// Retire the request `wire_id`, returning it if it was still waiting.
    pub fn retire(&mut self, wire_id: &[u8; 16]) -> Option<Pending> {
        self.pending.remove(wire_id)
    }

    /// Forget the request for the start `token` of `invoke_id`: its state
    /// exited, and nothing goes on the wire for it (§mesh-9.5, `<cancel>`).
    pub fn cancel(&mut self, invoke_id: &str, token: u64) {
        self.pending
            .retain(|_, p| !(p.invoke_id == invoke_id && p.token == token));
    }

    /// Forget every request sent to `peer`, whose link is gone, in wire-id
    /// order: no reply can come back on it (§mesh-16.7 row 5).
    pub fn forget_sent_to(&mut self, peer: &str) -> Vec<[u8; 16]> {
        let lost: Vec<[u8; 16]> = self
            .pending
            .iter()
            .filter(|(_, p)| p.target == peer)
            .map(|(id, _)| *id)
            .collect();
        for id in &lost {
            self.pending.remove(id);
        }
        lost
    }

    /// Retire every request whose deadline is at or before `now_ms`, in
    /// wire-id order.
    pub fn expire(&mut self, now_ms: i64) -> Vec<([u8; 16], Pending)> {
        let expired: Vec<[u8; 16]> = self
            .pending
            .iter()
            .filter(|(_, p)| p.expires_ms.is_some_and(|at| at <= now_ms))
            .map(|(id, _)| *id)
            .collect();
        expired
            .into_iter()
            .filter_map(|id| self.pending.remove(&id).map(|p| (id, p)))
            .collect()
    }
}

/// The `_event.data` of an `error.invoke.<id>` (§mesh-10.7.1,
/// `errorName: "invoke"`): `status` by the name `rpc_status.scxml` declares
/// for it, the reply's message as `detail`, the machine that answered as
/// `source` — absent for a deadline the requester synthesised — and the wire
/// `invoke_id`.
pub fn invoke_error_data(
    status: RpcStatus,
    detail: Option<&str>,
    source: Option<&str>,
    wire_id: &[u8; 16],
) -> String {
    let mut json = Json::new("invoke", status.declared_name());
    if let Some(detail) = detail {
        json.string("detail", detail);
    }
    if let Some(source) = source {
        json.string("source", source);
    }
    json.string("invoke_id", &uuid_text(wire_id));
    json.finish()
}

/// The `_event.data` of the `error.execution` a request that cannot reach
/// the wire raises instead of starting (§mesh-9.5's pre-envelope tier):
/// reason `INVOKE_SRC_NOT_FOUND`, with what the target was.
pub fn src_not_found_data(detail: &str) -> String {
    let mut json = Json::new("execution", "INVOKE_SRC_NOT_FOUND");
    json.string("detail", detail);
    json.finish()
}

/// Where RFC 4122 §3's canonical text puts its dashes.
const UUID_DASHES: [usize; 4] = [8, 13, 18, 23];

/// A 16-byte id — an envelope's, or a request's wire `invoke_id` — as
/// RFC 4122 §3's canonical text (`xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`,
/// lowercase): the form §mesh-10.7 gives `_event.invokeid` and §mesh-10.7.1
/// an `invoke_id` field, and the one the C++ core writes
/// (`SCE::uuid::to_string`), so a document cannot tell which core it runs
/// beside.
pub fn uuid_text(id: &[u8; 16]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(36);
    for (i, byte) in id.iter().enumerate() {
        if matches!(i, 4 | 6 | 8 | 10) {
            out.push('-');
        }
        out.push(DIGITS[usize::from(byte >> 4)] as char);
        out.push(DIGITS[usize::from(byte & 0x0f)] as char);
    }
    out
}

/// The id a request's `_event.invokeid` names: [`uuid_text`] read back,
/// either case, as the C++ core's `SCE::uuid::from_string` reads it. `None`
/// for text that is not one — an event raised by something other than a
/// Mesh request.
pub fn parse_uuid_text(text: &str) -> Option<[u8; 16]> {
    let chars = text.as_bytes();
    if chars.len() != 36 || UUID_DASHES.iter().any(|&i| chars[i] != b'-') {
        return None;
    }
    let mut digits = chars
        .iter()
        .enumerate()
        .filter(|(i, _)| !UUID_DASHES.contains(i))
        .map(|(_, &d)| char::from(d).to_digit(16).map(|v| v as u8));
    let mut out = [0u8; 16];
    for byte in &mut out {
        *byte = digits.next()?? << 4 | digits.next()??;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIRE: [u8; 16] = [
        0x01, 0x92, 0x00, 0x00, 0x00, 0x00, 0x70, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0xab,
    ];

    // Each expectation is the literal the Kotlin core's test pins too, so the
    // two cores cannot drift apart without one of them failing.

    #[test]
    fn a_failed_reply_names_its_status_by_the_declared_name() {
        assert_eq!(
            invoke_error_data(RpcStatus::Unavailable, Some("busy"), Some("cloud"), &WIRE),
            r#"{"errorName":"invoke","reason":"unavailable","detail":"busy","source":"cloud","invoke_id":"01920000-0000-7000-8000-0000000000ab"}"#
        );
    }

    #[test]
    fn a_synthesised_deadline_has_no_source() {
        assert_eq!(
            invoke_error_data(RpcStatus::DeadlineExceeded, None, None, &WIRE),
            r#"{"errorName":"invoke","reason":"deadlineExceeded","invoke_id":"01920000-0000-7000-8000-0000000000ab"}"#
        );
    }

    #[test]
    fn a_request_with_no_route_is_src_not_found() {
        assert_eq!(
            src_not_found_data("no binding for '#cloud'"),
            r#"{"errorName":"execution","reason":"INVOKE_SRC_NOT_FOUND","detail":"no binding for '#cloud'"}"#
        );
    }

    fn pending(invoke_id: &str, token: u64, expires_ms: Option<i64>) -> Pending {
        Pending {
            invoke_id: invoke_id.into(),
            token,
            target: "cloud".into(),
            responders: &["cloud"],
            expires_ms,
        }
    }

    /// §mesh-16.7 row 5: a lost link forgets what was sent on it, and only
    /// that.
    #[test]
    fn a_lost_link_forgets_only_the_requests_sent_on_it() {
        let mut table = Correlation::default();
        let mut other = WIRE;
        other[15] = 0xac;
        table.register(WIRE, pending("ask", 1, Some(100)));
        table.register(
            other,
            Pending {
                target: "hmi".into(),
                ..pending("look", 2, None)
            },
        );
        assert_eq!(table.forget_sent_to("cloud"), alloc::vec![WIRE]);
        assert!(!table.is_waiting(&WIRE));
        assert!(table.is_waiting(&other));
        assert!(
            table.expire(1000).is_empty(),
            "a forgotten request has no deadline"
        );
    }

    /// §mesh-14.6: only a declared responder retires a request, and a
    /// rejected reply leaves it answerable.
    #[test]
    fn only_a_declared_responder_is_admitted() {
        let mut table = Correlation::default();
        table.register(WIRE, pending("ask", 1, None));
        assert!(matches!(table.check(&WIRE, "mallory"), Answer::Undeclared));
        assert!(matches!(table.check(&WIRE, "cloud"), Answer::Admitted));
        assert!(table.retire(&WIRE).is_some());
        assert!(matches!(table.check(&WIRE, "cloud"), Answer::Unknown));
    }

    /// A cancel forgets only the start it names: a state entered again
    /// starts the same invoke id under a new token.
    #[test]
    fn a_cancel_forgets_only_its_own_start() {
        let mut table = Correlation::default();
        let mut second = WIRE;
        second[15] = 0xac;
        table.register(WIRE, pending("ask", 1, None));
        table.register(second, pending("ask", 2, None));
        table.cancel("ask", 1);
        assert!(matches!(table.check(&WIRE, "cloud"), Answer::Unknown));
        assert!(matches!(table.check(&second, "cloud"), Answer::Admitted));
    }

    #[test]
    fn a_deadline_expires_at_its_instant_and_not_before() {
        let mut table = Correlation::default();
        table.register(WIRE, pending("ask", 1, Some(100)));
        assert!(table.expire(99).is_empty());
        let expired = table.expire(100);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].1.invoke_id, "ask");
        assert!(table.expire(1000).is_empty());
    }
}
