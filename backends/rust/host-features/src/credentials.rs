// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Credentials (docs/SCE_HOST_FEATURES.md, section 14; docs/adr/0109, D3): a
//! statechart decides who gets a bearer token and whose credentials are
//! revoked, and the host keeps them.
//!
//! - `sce-host-credential-issue` answers a token for `name` that expires
//!   `ttl_s` seconds after the host's clock reads: `sce1.` and the base64url of
//!   the HMAC-SHA-256, under the key ring's issuing key, of the invocation's
//!   identity and the name (`sce_rust_crypto::credential`). The credential
//!   store keeps its SHA-256 and never the token, so a repeat under one
//!   identity answers the same token and keeps no second row.
//! - `sce-host-credential-revoke` revokes every token of `name` and removes its
//!   installation pin, in one transaction of the store, and answers how many it
//!   took away.
//!
//! The store is the one the embedding gives
//! ([`ReferenceHost::set_credentials`]), in memory until it gives another; a
//! server gives the store its `https` authenticator reads, so a token the
//! statechart issued is the one the server takes.

use sce_rust_crypto::credential::{issue_message, token_digest, token_text};
use sce_rust_crypto::keyring::KeyError;
use sce_rust_session_store::{CredentialStore, TokenRow, TokenStore};
use serde_json::json;

use crate::cose::UNAVAILABLE;
use crate::INVALID_ARGUMENT;
use crate::{failure, record, Invocation, Outcome, Params, ReferenceHost, Transport};

/// The operation that issues a bearer token.
pub const CREDENTIAL_ISSUE: &str = "sce-host-credential-issue";
/// The operation that revokes a name's credentials.
pub const CREDENTIAL_REVOKE: &str = "sce-host-credential-revoke";

/// The two operations, in the order the specification lists them.
pub const OPERATIONS: [&str; 2] = [CREDENTIAL_ISSUE, CREDENTIAL_REVOKE];

/// The bound of a request's `name`, in bytes.
pub const NAME_MAX: usize = 256;

/// The failure of an issue whose issuing key the operator retired.
const KEY_RETIRED: &str = "key_retired";

/// A request the host accepts.
enum Request<'p> {
    Issue { name: &'p str, ttl_s: u32 },
    Revoke { name: &'p str },
}

/// The request `params` makes of `operation`, or `None` when the host refuses
/// it: an empty name or one past its bound, or a `ttl_s` that is not a count of
/// seconds from 1 to the largest a uint32 holds.
fn request<'p>(operation: &str, params: &'p Params) -> Option<Request<'p>> {
    let name = params.get("name").map(String::as_str).unwrap_or("");
    if name.is_empty() || name.len() > NAME_MAX {
        return None;
    }
    Some(match operation {
        CREDENTIAL_ISSUE => {
            let ttl_s: u32 = params.get("ttl_s")?.parse().ok()?;
            if ttl_s == 0 {
                return None;
            }
            Request::Issue { name, ttl_s }
        }
        CREDENTIAL_REVOKE => Request::Revoke { name },
        _ => return None,
    })
}

/// Whether the host refuses `operation` with `params`.
pub(crate) fn refuses(operation: &str, params: &Params) -> bool {
    request(operation, params).is_none()
}

/// The failure code of an issue the key ring could not tag for: `key_retired`
/// when the operator retired the issuing key, `unavailable` otherwise.
fn issue_failure(error: &KeyError) -> &'static str {
    match error {
        KeyError::Retired => KEY_RETIRED,
        KeyError::Unknown | KeyError::Unavailable(_) => UNAVAILABLE,
    }
}

/// Perform `operation` of `invocation`, a request [`refuses`] accepted.
pub(crate) fn perform<T: Transport>(
    host: &mut ReferenceHost<T>,
    operation: &str,
    invocation: &Invocation,
    params: &Params,
) -> Outcome {
    let Some(request) = request(operation, params) else {
        return Outcome::Refuse(INVALID_ARGUMENT);
    };
    match request {
        Request::Issue { name, ttl_s } => {
            // A token whose expiry the host cannot place is not issued.
            let Some(now_ms) = crate::clock::utc_now(host).and_then(|ms| u64::try_from(ms).ok())
            else {
                sce_rust_runtime::sce_log_info!(
                    "{operation} {}: the clock gave no time to issue from",
                    invocation.id
                );
                return failure(operation, UNAVAILABLE);
            };
            let message = issue_message(
                &invocation.session_id,
                &invocation.id,
                invocation.start,
                name,
            );
            let ReferenceHost {
                keyring,
                random,
                credentials,
                ..
            } = host;
            let tag = match keyring.issuing_tag(&message, now_ms, random) {
                Ok(tag) => tag,
                Err(error) => {
                    sce_rust_runtime::sce_log_info!(
                        "{operation} {}: no tag under the issuing key, {error}",
                        invocation.id
                    );
                    return failure(operation, issue_failure(&error));
                }
            };
            let token = token_text(&tag);
            let row = TokenRow {
                digest: token_digest(&token),
                name: name.to_string(),
                issued_ms: now_ms,
                expires_ms: now_ms.saturating_add(u64::from(ttl_s).saturating_mul(1_000)),
                revoked: false,
            };
            match TokenStore::issue(&**credentials, &row) {
                Ok(_) => record(json!({ "token": token })),
                Err(error) => {
                    sce_rust_runtime::sce_log_info!(
                        "{operation} {}: the credential store did not keep the token, {error}",
                        invocation.id
                    );
                    failure(operation, UNAVAILABLE)
                }
            }
        }
        Request::Revoke { name } => match CredentialStore::revoke(&*host.credentials, name) {
            Ok(revoked) => record(json!({ "revoked": revoked.count() })),
            Err(error) => {
                sce_rust_runtime::sce_log_info!(
                    "{operation} {}: the credential store did not revoke, {error}",
                    invocation.id
                );
                failure(operation, UNAVAILABLE)
            }
        },
    }
}
