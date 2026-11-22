// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The document a server is upgraded between in place (docs/adr/0109, D11), as a deployment
//! serves it: its two versions, generated (tests/server/documents/server_upgrade_1.scxml and
//! server_upgrade_2.scxml), each served by the library's adapter of a generated machine
//! (`sce_rust_mesh::served`, docs/adr/0112), on one reference host whose keys verify and open
//! the notes the client signed and sealed by name (docs/adr/0109, D9).
//!
//! What the client sends and is answered, by event:
//!
//! * `service.request.take` (`{"key", "tag"}`): the tag is recorded and answered with its
//!   position and the state that took it; a delayed send `reminded` names it three seconds later.
//! * `service.request.note` (`{"key", "cose"}`): a COSE_Sign1 of a note is kept and verified by
//!   name; `service.request.recheck` verifies the kept one again, under the version running.
//! * `service.request.seal` (`{"key", "cose"}`): a COSE_Encrypt0 of a note is kept and opened.
//! * `service.request.read` (`{"key"}`): what the session holds (`upgrade_read.scxml`).
//! * `service.request.close` and `service.request.end`: recording stops; the session ends.

use std::path::Path;
use std::sync::PoisonError;

use sce_rust_host_features::{serve, CoseKeys, TrustedKey};
use sce_rust_mesh::peers::Machine;
use sce_rust_mesh::served::SessionHost;
use sce_rust_runtime::json::{self, Value};
use sce_rust_runtime::Engine;

use crate::ledger::{self, shared_host, Binding, SharedHost};
use crate::machines::generated::server_upgrade_1_sm::ServerUpgrade1Policy;
use crate::machines::generated::server_upgrade_2_sm::ServerUpgrade2Policy;

/// deploy.yaml's name of the machine both builds serve: one name across the upgrade, since the
/// endpoint sections of a checkpoint are keyed by it and a migration does not rename them
/// (docs/SCE_SERVER_DEPLOY.md section 4.2).
pub const MACHINE_NAME: &str = "upgrade";

pub const TAKE: &str = "service.request.take";
pub const TAKEN: &str = "service.response.take";
pub const NOTE: &str = "service.request.note";
pub const NOTED: &str = "service.response.note";
pub const RECHECK: &str = "service.request.recheck";
pub const RECHECKED: &str = "service.response.recheck";
pub const SEAL: &str = "service.request.seal";
pub const SEALED: &str = "service.response.seal";
pub const READ: &str = "service.request.read";
pub const READ_BACK: &str = "service.response.read";
pub const CLOSE: &str = "service.request.close";
pub const CLOSED: &str = "service.response.close";
pub const END: &str = "service.request.end";
pub const ENDED: &str = "service.response.end";
/// The delayed send a take schedules.
pub const REMINDED: &str = "reminded";

/// The reply events deploy.yaml pairs with the client's requests.
pub const REPLIES: &[&str] = &[TAKEN, NOTED, RECHECKED, SEALED, READ_BACK, CLOSED, ENDED];

/// The trust set the documents verify a note under, and the key they open one with.
pub const TRUST_SET: &str = "devices";
pub const SEALING_KEY: &str = "vault";

/// The upgraded document as deploy.yaml declares it, the same for both builds.
pub fn machine(binding: Binding, idle_ms: u64) -> Machine {
    ledger::pooled(MACHINE_NAME, REPLIES, binding, idle_ms)
}

/// A note signed and one sealed by name under the first revision of its schema, and the keys
/// that verify and open them: tests/host_features/payload_record_cross_language.json, whose
/// objects the Rust and C++ lanes write byte for byte.
#[derive(Debug, Clone)]
pub struct Notes {
    /// The tagged COSE_Sign1 of the note.
    pub signed: Vec<u8>,
    /// The tagged COSE_Encrypt0 of the same note.
    pub sealed: Vec<u8>,
    /// What the note holds: its serial and its owner.
    pub serial: u64,
    pub owner: String,
    signer: String,
    public_key: [u8; 32],
    sealing_key: [u8; 32],
}

impl Notes {
    /// The notes of the cross-language file.
    pub fn read() -> Notes {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../host_features/payload_record_cross_language.json");
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{} is not read: {error}", path.display()));
        let file = json::parse(&text).unwrap_or_else(|_| panic!("{} is no JSON", path.display()));
        let text_of = |value: Option<&Value>, what: &str| match value {
            Some(Value::Text(text)) => text.clone(),
            other => panic!("{what} is {other:?}"),
        };
        let signing = file.member("signing_key").expect("a signing key");
        let sealing = file.member("sealing_key").expect("a sealing key");
        let record = file.member("record").expect("a record");
        Notes {
            signed: hex(&text_of(file.member("signed"), "signed")),
            sealed: hex(&text_of(file.member("sealed"), "sealed")),
            serial: match record.member("serial") {
                Some(Value::Number(digits)) => digits.parse().expect("a serial"),
                other => panic!("the serial is {other:?}"),
            },
            owner: text_of(record.member("owner"), "owner"),
            signer: text_of(signing.member("id"), "the signer's id"),
            public_key: key(&text_of(signing.member("public_key"), "the public key")),
            sealing_key: key(&text_of(sealing.member("key"), "the sealing key")),
        }
    }

    /// The keys a host verifies and opens the notes with, and no key that signs or seals one.
    fn keys(&self) -> CoseKeys {
        let mut keys = CoseKeys::default();
        keys.trust.insert(
            TRUST_SET.to_string(),
            vec![TrustedKey {
                id: self.signer.clone(),
                public_key: self.public_key,
            }],
        );
        keys.symmetric
            .insert(SEALING_KEY.to_string(), self.sealing_key);
        keys
    }
}

/// The bytes `text` spells in hexadecimal.
fn hex(text: &str) -> Vec<u8> {
    assert_eq!(text.len() % 2, 0, "{text} is not hex");
    (0..text.len() / 2)
        .map(|i| u8::from_str_radix(&text[2 * i..2 * i + 2], 16).expect("hex"))
        .collect()
}

fn key(text: &str) -> [u8; 32] {
    hex(text).try_into().expect("32 bytes")
}

/// The text a byte string crosses as in a payload: each byte the character of its code point
/// (SCE Accepted Subset §2.12).
pub fn latin1(bytes: &[u8]) -> String {
    bytes.iter().copied().map(char::from).collect()
}

/// The data of a `take` of `tag` for `key`.
pub fn take_data(key: &str, tag: &str) -> String {
    format!(
        r#"{{"key":{},"tag":{}}}"#,
        json::quote(key),
        json::quote(tag)
    )
}

/// The data of a `note` or a `seal` of `object` for `key`.
pub fn object_data(key: &str, object: &[u8]) -> String {
    format!(
        r#"{{"key":{},"cose":{}}}"#,
        json::quote(key),
        json::quote(&latin1(object))
    )
}

/// The data of a request that names only its key.
pub fn key_data(key: &str) -> String {
    format!(r#"{{"key":{}}}"#, json::quote(key))
}

/// What every session of a server of the upgraded document is given: the policy of its
/// version, and one reference host that holds the keys of [`Notes`].
#[derive(Clone)]
pub struct UpgradeHost {
    host: SharedHost,
}

impl UpgradeHost {
    /// A host that verifies and opens `notes`.
    pub fn new(notes: &Notes) -> UpgradeHost {
        let host = shared_host();
        host.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .set_cose_keys(notes.keys())
            .expect("the notes' keys take no reserved id");
        UpgradeHost { host }
    }
}

impl SessionHost<ServerUpgrade1Policy> for UpgradeHost {
    fn policy(&self, _key: &str) -> Result<ServerUpgrade1Policy, String> {
        Ok(ServerUpgrade1Policy::new())
    }

    fn register(&self, _key: &str, engine: &mut Engine<ServerUpgrade1Policy>) {
        serve(engine, &self.host);
    }
}

impl SessionHost<ServerUpgrade2Policy> for UpgradeHost {
    fn policy(&self, _key: &str) -> Result<ServerUpgrade2Policy, String> {
        Ok(ServerUpgrade2Policy::new())
    }

    fn register(&self, _key: &str, engine: &mut Engine<ServerUpgrade2Policy>) {
        serve(engine, &self.host);
    }
}
