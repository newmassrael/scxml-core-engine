// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Envelopes as the suite's client writes and reads them.
//!
//! The client of the scenarios that serve a pool in a process of its own is the test itself: it
//! writes each request as the envelope a client machine's endpoint would send, so that it can send
//! the very same bytes again (a retry, SCE_MESH.md section 10.5), and it reads what the server
//! sends as the envelopes they are.

use std::sync::{Arc, Mutex, PoisonError};

use sce_forge_runtime::codec::{Bounded, SceCursor};
use sce_rust_mesh::endpoint::{Transport, TransportFailure};
use sce_rust_mesh::generated::envelope::Envelope;
use sce_rust_mesh::generated::pattern_kind::PatternKind;
use sce_rust_mesh::generated::payload_codec::PayloadCodec;
use sce_rust_mesh::generated::rpc_status::RpcStatus;

use crate::ledger::CLIENT;

/// `bytes` as lowercase hexadecimal digits, two to a byte.
pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}

/// The bytes `text` writes as [`hex`] writes them, or `None` for text that is not such digits.
pub fn unhex(text: &str) -> Option<Vec<u8>> {
    fn digit(c: u8) -> Option<u8> {
        match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            _ => None,
        }
    }
    text.as_bytes()
        .chunks(2)
        .map(|pair| match pair {
            [high, low] => Some((digit(*high)? << 4) | digit(*low)?),
            _ => None,
        })
        .collect()
}

/// Envelope ids for one client: a nonce of its own and a count, so that two clients of one test
/// (which share the peer name the server knows them by, and so one duplicate window) never send
/// two envelopes under one id.
#[derive(Debug, Clone)]
pub struct Ids {
    nonce: u64,
    next: u64,
}

impl Ids {
    pub fn new(nonce: u64) -> Ids {
        Ids { nonce, next: 0 }
    }

    /// The next id.
    pub fn draw(&mut self) -> [u8; 16] {
        self.next += 1;
        let mut id = [0u8; 16];
        id[..8].copy_from_slice(&self.nonce.to_be_bytes());
        id[8..].copy_from_slice(&self.next.to_be_bytes());
        id
    }
}

/// A request as the client sent it: its bytes, which a retry sends again unchanged, and the wire
/// `invoke_id` its reply carries back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub wire_id: [u8; 16],
    pub bytes: Vec<u8>,
}

/// The request `event` with `data` (a JSON object) from the client, as an `RpcRequest` envelope:
/// the shape a client machine's `<invoke type="sce:mesh-rpc">` puts on the wire.
pub fn request(ids: &mut Ids, event: &str, data: &str) -> Request {
    let envelope_id = ids.draw();
    let wire_id = ids.draw();
    let bytes = Envelope {
        id: (&envelope_id).into(),
        source: Bounded::new(CLIENT).expect("the client's name fits an envelope"),
        event_type: Bounded::new(event).expect("an event name fits an envelope"),
        pattern: PatternKind::RpcRequest,
        datacontenttype: PayloadCodec::Json,
        data: Bounded::new(data.as_bytes()).expect("a request's data fits an envelope"),
        invoke_id: Some((&wire_id).into()),
        ..Envelope::new()
    }
    .encode_to_vec();
    Request { wire_id, bytes }
}

/// An envelope the server sent, as much of it as the scenarios read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seen {
    /// The envelope's bytes as they were sent: two copies of one envelope are the same bytes.
    pub bytes: Vec<u8>,
    /// The envelope's id: what a receiver's duplicate window knows a copy by.
    pub id: [u8; 16],
    pub event: String,
    pub pattern: PatternKind,
    pub status: Option<RpcStatus>,
    /// The wire `invoke_id` of the request a reply answers.
    pub wire_id: Option<[u8; 16]>,
    /// The payload, as the text a JSON payload is.
    pub data: String,
}

impl Seen {
    /// Whether it is a reply that ends its request well: `Ok`, or no status, which a requester
    /// reads as `Ok`.
    pub fn is_ok_reply(&self) -> bool {
        self.pattern == PatternKind::RpcReply && matches!(self.status, None | Some(RpcStatus::Ok))
    }
}

/// The envelope `bytes` are, or `None` for bytes that are not one.
pub fn read(bytes: Vec<u8>) -> Option<Seen> {
    let (id, event, pattern, status, wire_id, data) = {
        let envelope = Envelope::decode(&mut SceCursor::new(&bytes)).ok()?;
        (
            <[u8; 16]>::from(envelope.id),
            envelope.event_type.to_string(),
            envelope.pattern,
            envelope.rpc_status,
            envelope.invoke_id.map(<[u8; 16]>::from),
            String::from_utf8_lossy(envelope.data.into_inner()).into_owned(),
        )
    };
    Some(Seen {
        bytes,
        id,
        event,
        pattern,
        status,
        wire_id,
        data,
    })
}

/// What a recording transport was handed, by peer, in order, shared by its clones.
pub type Handed = Arc<Mutex<Vec<(String, Vec<u8>)>>>;

/// A transport that records what it was handed, for a pool served in the test's own process; its
/// clones share the record.
#[derive(Clone, Default)]
pub struct Recording(Handed);

impl Transport for Recording {
    fn transmit(&mut self, peer: &str, bytes: &[u8]) -> Result<(), TransportFailure> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((peer.to_string(), bytes.to_vec()));
        Ok(())
    }
}

impl Recording {
    /// What the transport was handed since the last call, in order, each with the peer it was for.
    pub fn take(&self) -> Vec<(String, Seen)> {
        core::mem::take(&mut *self.0.lock().unwrap_or_else(PoisonError::into_inner))
            .into_iter()
            .map(|(peer, bytes)| (peer, read(bytes).expect("the pool sends envelopes")))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_is_read_back_as_the_bytes_it_writes() {
        let bytes = [0x00, 0x0f, 0xa5, 0xff];
        assert_eq!(hex(&bytes), "000fa5ff");
        assert_eq!(unhex("000fa5ff"), Some(bytes.to_vec()));
        assert_eq!(unhex(""), Some(Vec::new()));
        assert_eq!(unhex("abc"), None, "an odd count of digits");
        assert_eq!(unhex("0G"), None, "a digit that is no hexadecimal one");
        assert_eq!(unhex("AB"), None, "upper case is not what `hex` writes");
    }

    #[test]
    fn a_request_is_read_back_as_the_request_it_is() {
        let mut ids = Ids::new(3);
        let sent = request(&mut ids, "service.request.take", r#"{"key":"k"}"#);
        let seen = read(sent.bytes.clone()).expect("an envelope");
        assert_eq!(seen.pattern, PatternKind::RpcRequest);
        assert_eq!(seen.wire_id, Some(sent.wire_id));
        assert_eq!(seen.event, "service.request.take");
        assert_eq!(seen.data, r#"{"key":"k"}"#);
        assert_ne!(
            seen.id, sent.wire_id,
            "the envelope's id and its invoke id are two ids"
        );
        assert!(!seen.is_ok_reply());
    }
}
