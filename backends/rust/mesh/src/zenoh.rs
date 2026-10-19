// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Zenoh binding's upstream client (SCE_MESH.md §mesh-20): the session of upstream's `zenoh`
//! crate, as the four operations the binding asks of a client.
//!
//! The binding itself (the key layout, the order a machine is set up in, the mapping from what
//! arrives to [`LinkEvent`](crate::link::LinkEvent)s) lives in [`crate::inbox`] and takes any
//! [`ZenohClient`]; this module is the implementation of that trait in this repository, plus the
//! names the binding has always had here ([`open`], [`ZenohBinding`], [`ZenohTransport`] and the
//! key helpers), so a host that wants upstream's client changes nothing.
//!
//! The router the session connects to carries the bytes. Which one it is does not matter to this
//! module: it speaks upstream's `zenoh` client, so it reaches any router of that protocol version
//! (§mesh-20.4).

use std::sync::{Mutex, PoisonError};

use ::zenoh::qos::CongestionControl;
use ::zenoh::sample::SampleKind;
use ::zenoh::{Config, Session, Wait};

use crate::inbox::{
    self, check_machine_name, open_with, ClientError, Held, PresenceKind, PutError, ZenohClient,
};

pub use crate::inbox::{
    inbox_filter, inbox_key, presence_filter, presence_key, ZenohError, ZenohEvents, KEY_PREFIX,
};

/// The core's side of the session when upstream's client carries it.
pub type ZenohTransport = inbox::ZenohTransport<UpstreamClient>;

/// An open binding on upstream's client: the transport to give the
/// [`Endpoint`](crate::endpoint::Endpoint), and the events to feed it.
pub type ZenohBinding = inbox::ZenohBinding<UpstreamClient>;

fn session_error(error: impl core::fmt::Display) -> ZenohError {
    ZenohError::Session(error.to_string())
}

/// A client configuration that connects to `endpoints` (`tcp/<host>:<port>`) and nothing else: no
/// multicast scouting, so the machine joins the network it was told to and not one it happened
/// upon.
pub fn client_config(endpoints: &[&str]) -> Result<Config, ZenohError> {
    let mut config = Config::default();
    let list = format!(
        "[{}]",
        endpoints
            .iter()
            .map(|endpoint| format!("\"{endpoint}\""))
            .collect::<Vec<_>>()
            .join(",")
    );
    for (key, value) in [
        ("mode", "\"client\"".to_string()),
        ("connect/endpoints", list),
        ("scouting/multicast/enabled", "false".to_string()),
    ] {
        config
            .insert_json5(key, &value)
            .map_err(|e| session_error(format!("{key}: {e}")))?;
    }
    Ok(config)
}

/// Upstream's `zenoh` crate as the binding's client: one session, from which the binding's
/// subscribers, token and puts are made.
pub struct UpstreamClient {
    session: Session,
}

impl UpstreamClient {
    /// Open a session from `config` (see [`client_config`]).
    pub fn connect(config: Config) -> Result<Self, ZenohError> {
        let session = ::zenoh::open(config).wait().map_err(session_error)?;
        Ok(UpstreamClient { session })
    }
}

impl ZenohClient for UpstreamClient {
    fn subscribe<F>(&mut self, filter: &str, on_sample: F) -> Result<Held, ClientError>
    where
        F: FnMut(&str, &[u8]) + Send + 'static,
    {
        // Upstream's callback is shared and called through `&self`; the trait's is called one at a
        // time, so the lock is what turns one into the other.
        let on_sample = Mutex::new(on_sample);
        let subscriber = self
            .session
            .declare_subscriber(filter.to_string())
            .callback(move |sample| {
                let payload = sample.payload().to_bytes();
                let mut guard = on_sample.lock().unwrap_or_else(PoisonError::into_inner);
                (*guard)(sample.key_expr().as_str(), &payload[..]);
            })
            .wait()
            .map_err(ClientError::new)?;
        Ok(Box::new(subscriber))
    }

    fn subscribe_presence<F>(
        &mut self,
        filter: &str,
        history: bool,
        on_change: F,
    ) -> Result<Held, ClientError>
    where
        F: FnMut(&str, PresenceKind) + Send + 'static,
    {
        let on_change = Mutex::new(on_change);
        let subscriber = self
            .session
            .liveliness()
            .declare_subscriber(filter.to_string())
            .history(history)
            .callback(move |sample| {
                let kind = match sample.kind() {
                    SampleKind::Put => PresenceKind::Put,
                    SampleKind::Delete => PresenceKind::Delete,
                };
                let mut guard = on_change.lock().unwrap_or_else(PoisonError::into_inner);
                (*guard)(sample.key_expr().as_str(), kind);
            })
            .wait()
            .map_err(ClientError::new)?;
        Ok(Box::new(subscriber))
    }

    fn declare_presence(&mut self, key: &str) -> Result<Held, ClientError> {
        let token = self
            .session
            .liveliness()
            .declare_token(key.to_string())
            .wait()
            .map_err(ClientError::new)?;
        Ok(Box::new(token))
    }

    fn put(&mut self, key: &str, payload: &[u8]) -> Result<(), PutError> {
        // Block rather than drop under congestion (§mesh-20.2): the trait requires it of every
        // client, and this is where upstream's client is asked for it.
        self.session
            .put(key.to_string(), payload.to_vec())
            .congestion_control(CongestionControl::Block)
            .wait()
            .map_err(PutError::retryable)
    }
}

/// Open the binding for `machine` on a session made from `config`, carried by upstream's client.
///
/// The name is checked before the session is opened, so a bad name never costs one; everything
/// else is [`open_with`].
pub fn open(config: Config, machine: &str) -> Result<ZenohBinding, ZenohError> {
    check_machine_name(machine)?;
    open_with(UpstreamClient::connect(config)?, machine)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_that_is_not_a_key_chunk_is_refused_before_a_session_is_opened() {
        for bad in ["", "a/b", "a*b", "a$b", "a b", "a?b", "a#b"] {
            match open(Config::default(), bad) {
                Err(ZenohError::NotAKeyChunk(name)) => assert_eq!(name, bad),
                Err(other) => panic!("{bad:?} gave {other}"),
                Ok(_) => panic!("{bad:?} opened a binding"),
            }
        }
    }

    #[test]
    fn a_client_config_names_its_endpoints_and_does_not_scout() {
        let config = client_config(&["tcp/127.0.0.1:7447", "tcp/10.0.0.2:7447"]).unwrap();
        let text = format!("{config:?}");
        assert!(text.contains("127.0.0.1:7447") && text.contains("10.0.0.2:7447"));
    }
}
