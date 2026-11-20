// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-22 mesh-22.1 mesh-22.2 mesh-22.3 mesh-22.4 mesh-22.6 mesh-22.7 mesh-22.9

//! This crate's adapter of the HTTPS port, `HyperTransport`, run through the conformance kit every
//! adapter is held to (SCE_MESH.md section 22.7).

use sce_rust_mesh::https::adapter::{AdapterSettings, HyperTransport, ProxyPolicy};
use sce_rust_mesh::https::conformance::{self, Harness};

/// The reference adapter, believing what the kit tells it to beside the operating system's
/// authorities, and reaching the kit's servers through the kit's proxy or directly, never through
/// a proxy the machine running the test happens to configure.
struct Hyper;

impl Harness for Hyper {
    type Transport = HyperTransport;

    fn transport(&self, authorities: &[Vec<u8>], proxy: Option<&str>) -> HyperTransport {
        HyperTransport::new(AdapterSettings {
            extra_authorities: authorities.to_vec(),
            proxy: proxy.map_or(ProxyPolicy::Direct, |proxy| {
                ProxyPolicy::Through(proxy.to_string())
            }),
            client_identity: None,
        })
        .expect("the reference adapter")
    }
}

#[test]
fn an_envelope_crosses_and_its_reply_comes_back() {
    conformance::an_envelope_crosses_and_its_reply_comes_back(&Hyper);
}

#[test]
fn a_server_under_an_unknown_authority_is_refused() {
    conformance::a_server_under_an_unknown_authority_is_refused(&Hyper);
}

#[test]
fn through_an_explicit_proxy() {
    conformance::through_an_explicit_proxy(&Hyper);
}

#[test]
fn through_a_proxy_that_inspects_tls() {
    conformance::through_a_proxy_that_inspects_tls(&Hyper);
}

#[test]
fn a_response_lost_after_the_server_acted_is_carried_again() {
    conformance::a_response_lost_after_the_server_acted_is_carried_again(&Hyper);
}

#[test]
fn a_held_exchange_past_its_bound_is_a_timeout() {
    conformance::a_held_exchange_past_its_bound_is_a_timeout(&Hyper);
}

#[test]
fn an_altered_first_number_is_refused() {
    conformance::an_altered_first_number_is_refused(&Hyper);
}

#[test]
fn an_altered_body_is_refused() {
    conformance::an_altered_body_is_refused(&Hyper);
}

#[test]
fn a_replayed_response_is_refused() {
    conformance::a_replayed_response_is_refused(&Hyper);
}
