// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Random bytes, keyed digests and keys (docs/SCE_HOST_FEATURES.md, sections
//! 11 to 13; docs/adr/0109, D5 to D7).
//!
//! - `sce-host-random-bytes` draws from the host's random source, the operating
//!   system's unless the embedding gives another
//!   ([`ReferenceHost::set_random_source`]).
//! - `sce-host-mac` and `sce-host-mac-verify` are HMAC-SHA-256 under a MAC key
//!   the host holds, the tag compared in constant time
//!   (`sce_rust_crypto::equal::same_secret`): a statechart compares no secret.
//! - `sce-host-key-generate`, `sce-host-key-public` and `sce-host-key-retire`
//!   are the life of a key of the host's key ring
//!   (`sce_rust_crypto::keyring`): one use per key, its material wrapped under
//!   a key-encryption key, its public key exported as a `COSE_Key`.

use sce_rust_crypto::keyring::{generated_use, is_key_ring_id, KeyUse, Purpose};
use sce_rust_crypto::mac::{hmac_sha256, verify_hmac_sha256, TAG_BYTES};
use sce_rust_crypto::random::RandomSource;
use serde_json::json;

use crate::cose::{
    cose_key_ed25519, mac_key, public_key_of, signing_seed, text_of, Arguments, UNAVAILABLE,
    UNKNOWN_KEY,
};
use crate::{failure, record, Invocation, Outcome, Params, ReferenceHost, Transport};
use crate::{NamedBounds, INVALID_ARGUMENT};

/// The operation that draws random bytes.
pub const RANDOM_BYTES: &str = "sce-host-random-bytes";
/// The operation that tags a message.
pub const MAC: &str = "sce-host-mac";
/// The operation that verifies a tag.
pub const MAC_VERIFY: &str = "sce-host-mac-verify";
/// The operation that generates a key.
pub const KEY_GENERATE: &str = "sce-host-key-generate";
/// The operation that exports a signing key's public key.
pub const KEY_PUBLIC: &str = "sce-host-key-public";
/// The operation that retires a key.
pub const KEY_RETIRE: &str = "sce-host-key-retire";

/// The six operations, in the order the specification lists them.
pub const OPERATIONS: [&str; 6] = [
    RANDOM_BYTES,
    MAC,
    MAC_VERIFY,
    KEY_GENERATE,
    KEY_PUBLIC,
    KEY_RETIRE,
];

/// The bound of a generation's `label`, in bytes.
const LABEL_MAX: usize = 64;

/// A request the host accepts.
enum Request<'p> {
    Random {
        length: usize,
    },
    Mac {
        key_id: &'p str,
        message: Vec<u8>,
    },
    MacVerify {
        key_id: &'p str,
        message: Vec<u8>,
        tag: Vec<u8>,
    },
    Generate {
        key_use: KeyUse,
        label: &'p str,
    },
    Public {
        key_id: &'p str,
    },
    Retire {
        key_id: &'p str,
    },
}

/// The request `params` makes of `operation`, or `None` when the host refuses
/// it: a length of 0 or past the bound, an empty key id, a use that is not one
/// of the four a key is generated for, or a value its record's field does not
/// hold at the bounds `bounds` gives the named ones.
fn request<'p>(operation: &str, params: &'p Params, bounds: &NamedBounds) -> Option<Request<'p>> {
    let arguments = Arguments(params);
    Some(match operation {
        RANDOM_BYTES => {
            let length: usize = arguments.text("length").parse().ok()?;
            if !(1..=bounds.random_bytes).contains(&length) {
                return None;
            }
            Request::Random { length }
        }
        MAC => Request::Mac {
            key_id: arguments.id("key_id")?,
            message: arguments.bytes("message", bounds.mac_message)?,
        },
        MAC_VERIFY => Request::MacVerify {
            key_id: arguments.id("key_id")?,
            message: arguments.bytes("message", bounds.mac_message)?,
            tag: arguments.bytes("tag", TAG_BYTES)?,
        },
        KEY_GENERATE => {
            let label = arguments.text("label");
            if label.len() > LABEL_MAX {
                return None;
            }
            Request::Generate {
                key_use: generated_use(arguments.text("key_use"))?,
                label,
            }
        }
        KEY_PUBLIC => Request::Public {
            key_id: arguments.id("key_id")?,
        },
        KEY_RETIRE => Request::Retire {
            key_id: arguments.id("key_id")?,
        },
        _ => return None,
    })
}

/// Whether the host refuses `operation` with `params` at `bounds`.
pub(crate) fn refuses(operation: &str, params: &Params, bounds: &NamedBounds) -> bool {
    request(operation, params, bounds).is_none()
}

/// The identity a generation is kept under: the invocation's, which a restore
/// starts again (docs/SCE_HOST_FEATURES.md section 1, "Identity").
fn generation_of(invocation: &Invocation) -> String {
    json!([invocation.session_id, invocation.id, invocation.start]).to_string()
}

/// Perform `operation` of `invocation`, a request [`refuses`] accepted.
pub(crate) fn perform<T: Transport>(
    host: &mut ReferenceHost<T>,
    operation: &str,
    invocation: &Invocation,
    params: &Params,
) -> Outcome {
    let bounds = host.bounds;
    let Some(request) = request(operation, params, &bounds) else {
        return Outcome::Refuse(INVALID_ARGUMENT);
    };
    match request {
        Request::Random { length } => {
            let mut drawn = vec![0u8; length];
            if host.random.fill(&mut drawn) {
                record(json!({ "bytes": text_of(&drawn) }))
            } else {
                failure(operation, UNAVAILABLE)
            }
        }
        Request::Mac { key_id, message } => match mac_key(host, key_id, Purpose::Tag) {
            Ok(key) => record(json!({ "tag": text_of(&hmac_sha256(&key, &message)) })),
            Err(error) => failure(operation, error.code()),
        },
        Request::MacVerify {
            key_id,
            message,
            tag,
        } => match mac_key(host, key_id, Purpose::Verify) {
            Ok(key) => record(json!({ "valid": verify_hmac_sha256(&key, &message, &tag) })),
            Err(error) => failure(operation, error.code()),
        },
        Request::Generate { key_use, label } => {
            let now_ms = crate::clock::utc_now(host)
                .and_then(|ms| u64::try_from(ms).ok())
                .unwrap_or(0);
            let generation = generation_of(invocation);
            let ReferenceHost {
                keyring, random, ..
            } = host;
            match keyring.generate(key_use, label, &generation, now_ms, random) {
                Ok(key_id) => record(json!({ "key_id": key_id })),
                Err(error) => failure(operation, error.code()),
            }
        }
        Request::Public { key_id } => match signing_seed(host, key_id, Purpose::Export) {
            Ok(seed) => record(json!({
                "cose_key": text_of(&cose_key_ed25519(key_id.as_bytes(), &public_key_of(&seed))),
            })),
            Err(error) => failure(operation, error.code()),
        },
        Request::Retire { key_id } => {
            // A configured key is the embedding's, not the key ring's to retire.
            if !is_key_ring_id(key_id) {
                return failure(operation, UNKNOWN_KEY);
            }
            match host.keyring.retire(key_id) {
                Ok(retired) => record(json!({ "retired": retired })),
                Err(error) => failure(operation, error.code()),
            }
        }
    }
}
