// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a screen says of a model server whose address a person typed: whether it is there, whether
//! it speaks the protocol this build speaks, and which models it lists to choose among.
//!
//! The only question asked is the one of the protocol that is not a model working (`GET /models`):
//! nothing is run, nothing of a work is sent, and no key is given, because this build has no place
//! to keep one. A server that wants one says so, in a state of its own, and a screen tells the
//! person what that comes to and does not offer to save the server as one that is ready.
//!
//! Where the server is, told apart from this computer, matters to the person who is about to send
//! it a specification: a screen shows it (`reach`), and whether the way there is encrypted (`tls`).
//! What the address says of it is all that is known. A tunnel to another computer is `127.0.0.1`,
//! which is why a connection also carries the name a person gave it.

use serde::Serialize;

use crate::connection::check_server_address;
use crate::error::StoreError;
use crate::http_client::Endpoint;
use crate::local::{list_models, ModelsError};

/// Where the server is, by its address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Reach {
    /// A loopback address: what is sent does not leave this computer, unless a tunnel carries it.
    ThisComputer,
    /// Any other: what is sent goes over the network.
    Network,
}

/// What came of asking the server for its models.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum ServerState {
    /// It listed its models, in its order. An empty list is a server with nothing loaded.
    Listed { models: Vec<String> },
    /// Nothing answered, or what answered broke or was too slow. `reason` is for the screen to
    /// show: it names no key and nothing of the person's.
    Unreachable { reason: String },
    /// Something answered over https and its certificate was refused. Nothing was sent to it.
    Certificate { reason: String },
    /// It answered that nobody is let in without a key, which this build has no place to keep.
    NeedsKey { reason: String },
    /// It answered, and what it answered is not a list of models: the address is probably not the
    /// OpenAI-compatible root of the server.
    NotAModelList { reason: String },
}

/// What a screen says of the server at an address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServerStatus {
    /// The address as it was given, which is what a connection would keep.
    pub address: String,
    pub reach: Reach,
    /// Whether the way to the server is encrypted and the server is known by its certificate.
    pub tls: bool,
    #[serde(flatten)]
    pub server: ServerState,
}

/// Ask the server at `address` for its models. An address that is not one a connection may keep is
/// refused as a connection would refuse it, and a server that does not answer is a state and not an
/// error: a person who typed the address wrong is told what is wrong with it, in the same screen.
pub fn read(address: &str) -> Result<ServerStatus, StoreError> {
    check_server_address(address)?;
    let endpoint = Endpoint::parse(address).map_err(|why| {
        StoreError::refused("bad-connection", why.to_string(), serde_json::Value::Null)
    })?;
    let server = match list_models(&endpoint, None) {
        Ok(models) => ServerState::Listed { models },
        Err(ModelsError::Unreachable(reason)) => ServerState::Unreachable { reason },
        Err(ModelsError::Certificate(reason)) => ServerState::Certificate { reason },
        Err(ModelsError::NeedsKey(reason)) => ServerState::NeedsKey { reason },
        Err(ModelsError::Refused(reason)) => ServerState::NotAModelList { reason },
    };
    Ok(ServerStatus {
        address: address.to_string(),
        reach: if endpoint.is_loopback() {
            Reach::ThisComputer
        } else {
            Reach::Network
        },
        tls: endpoint.is_tls(),
        server,
    })
}
