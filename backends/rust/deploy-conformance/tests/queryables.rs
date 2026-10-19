// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The queries a machine serves (SCE Protocol-Synthesis RFC §synth-5-R,
//! Queryables a machine serves), as the host that links the generated module
//! sees them: which declared queryables a query reaches, the exact bytes of a
//! reply, and the size bound a host sizes its buffers from.

use sce_deploy_conformance::admin_node_queryables::{
    answering, config_len, status_connect_len, write_config, write_peer, write_peer_one,
    write_root, write_status_connect, ConfigDoc, ConfigDocConnect, Leg, LegSet, StatusConnectDoc,
    StatusConnectDocPermissions, CONFIG_MAX_BODY_BYTES, LEGS, LEG_COUNT, MAX_BODY_BYTES,
    STATUS_CONNECT_MAX_BODY_BYTES,
};
use sce_link_runtime::json::WriteError;

fn legs(query: &str) -> Vec<Leg> {
    answering(query).iter().collect()
}

#[test]
fn a_literal_query_reaches_the_queryable_that_declares_it() {
    assert_eq!(legs("config"), [Leg::Config]);
    assert_eq!(legs("status/connect"), [Leg::StatusConnect]);
    assert_eq!(legs(""), [Leg::Root]);
}

#[test]
fn a_literal_query_that_nothing_declares_reaches_nothing() {
    assert!(answering("status").is_empty());
    assert!(answering("status/connect/x").is_empty());
    assert!(answering("other").is_empty());
}

#[test]
fn a_declared_wildcard_answers_what_it_covers() {
    // `peer/**` covers `peer` itself and everything below it; `peer/*` is one
    // chunk below it.
    assert_eq!(legs("peer"), [Leg::Peer]);
    assert_eq!(legs("peer/x"), [Leg::Peer, Leg::PeerOne]);
    assert_eq!(legs("peer/x/y"), [Leg::Peer]);
}

#[test]
fn a_query_with_wildcards_reaches_every_queryable_it_overlaps() {
    assert_eq!(legs("**"), LEGS.to_vec());
    assert_eq!(legs("status/*"), [Leg::StatusConnect]);
    assert_eq!(legs("*"), [Leg::Config, Leg::Peer]);
    assert_eq!(legs("con$*"), [Leg::Config]);
    assert_eq!(legs("peer/**"), [Leg::Peer, Leg::PeerOne]);
    assert!(answering("nothing/**").is_empty());
}

#[test]
fn the_set_knows_what_it_holds() {
    let mut set = LegSet::EMPTY;
    assert!(set.is_empty());
    set.insert(Leg::Config);
    set.insert(Leg::Config);
    assert_eq!(set.len(), 1);
    assert!(set.contains(Leg::Config));
    assert!(!set.contains(Leg::Root));
    assert_eq!(LEG_COUNT, 5);
    assert_eq!(Leg::StatusConnect.key(), "status/connect");
    assert_eq!(Leg::Root.key(), "");
    assert_eq!(Leg::Config.encoding(), "application/json");
    assert_eq!(Leg::Peer.name(), "peer");
}

fn status_doc<'a>(
    endpoints: &'a [&'a str],
    last_write: Option<u64>,
    write: bool,
) -> StatusConnectDoc<'a> {
    StatusConnectDoc {
        endpoints,
        last_write,
        permissions: StatusConnectDocPermissions { write },
    }
}

fn written(f: impl FnOnce(&mut [u8]) -> Result<usize, WriteError>) -> String {
    let mut buf = [0u8; 512];
    let n = f(&mut buf).expect("fits");
    String::from_utf8(buf[..n].to_vec()).expect("utf-8")
}

#[test]
fn a_reply_is_the_declared_document_with_the_values_filled_in() {
    let endpoints = ["udp/10.0.0.2:7447", "tcp/10.0.0.3:7447"];
    let text = written(|out| write_status_connect(&status_doc(&endpoints, Some(42), true), out));
    assert_eq!(
        text,
        r#"{"endpoints":["udp/10.0.0.2:7447","tcp/10.0.0.3:7447"],"last_write":42,"permissions":{"write":true}}"#
    );
    let value: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
    assert_eq!(value["permissions"]["write"], true);
}

#[test]
fn a_nullable_value_that_is_absent_is_null() {
    let text = written(|out| write_status_connect(&status_doc(&[], None, false), out));
    assert_eq!(
        text,
        r#"{"endpoints":[],"last_write":null,"permissions":{"write":false}}"#
    );
}

#[test]
fn nested_objects_signed_integers_and_strings_are_written() {
    let endpoints = ["a"];
    let doc = ConfigDoc {
        connect: ConfigDocConnect {
            endpoints: &endpoints,
        },
        mode: "peer",
        ttl: -5,
    };
    let text = written(|out| write_config(&doc, out));
    assert_eq!(
        text,
        r#"{"connect":{"endpoints":["a"]},"mode":"peer","ttl":-5}"#
    );
}

#[test]
fn a_string_is_escaped_so_the_reply_stays_valid_json() {
    let endpoints = ["a\"b\\c\n\u{1}"];
    let text = written(|out| write_status_connect(&status_doc(&endpoints, None, false), out));
    let value: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
    assert_eq!(value["endpoints"][0], "a\"b\\c\n\u{1}");
}

#[test]
fn a_static_reply_is_its_body() {
    assert_eq!(written(write_root), r#"{"ok":true}"#);
    assert_eq!(written(write_peer), "peer");
    assert_eq!(written(write_peer_one), "one");
}

#[test]
fn a_buffer_too_small_is_an_error_that_writes_nothing() {
    let endpoints = ["udp/10.0.0.2:7447"];
    let doc = status_doc(&endpoints, Some(1), true);
    let needed = status_connect_len(&doc).expect("measures");
    let mut buf = vec![0xAAu8; needed - 1];
    assert_eq!(
        write_status_connect(&doc, &mut buf),
        Err(WriteError::BufferTooSmall { needed })
    );
    assert!(
        buf.iter().all(|b| *b == 0xAA),
        "a short buffer is left as it was"
    );
    let mut exact = vec![0u8; needed];
    assert_eq!(write_status_connect(&doc, &mut exact), Ok(needed));
}

#[test]
fn a_value_outside_a_declared_bound_is_refused_and_not_written() {
    let too_long = "x".repeat(25);
    let endpoints = [too_long.as_str()];
    let mut buf = [0u8; 512];
    assert_eq!(
        write_status_connect(&status_doc(&endpoints, None, false), &mut buf),
        Err(WriteError::StringTooLong {
            field: "endpoints[]",
            max: 24
        })
    );
    let three = ["a", "b", "c"];
    assert_eq!(
        write_status_connect(&status_doc(&three, None, false), &mut buf),
        Err(WriteError::TooManyItems {
            field: "endpoints",
            max: 2
        })
    );
    assert!(buf.iter().all(|b| *b == 0), "nothing was written");
}

#[test]
fn the_worst_case_reply_fits_the_size_the_deploy_derives_exactly() {
    // Two strings of 24 control characters (each `\u00XX`, 6 bytes), the
    // widest u64, and the longer of the two booleans.
    let control = "\u{1}".repeat(24);
    let endpoints = [control.as_str(), control.as_str()];
    let doc = status_doc(&endpoints, Some(u64::MAX), false);
    let len = status_connect_len(&doc).expect("measures");
    assert_eq!(len, STATUS_CONNECT_MAX_BODY_BYTES);
    // By hand: braces 2, `"endpoints":` 12 + array (2 + 2 * (2 + 24 * 6) + 1),
    // `"last_write":` 13 + 20, `"permissions":` 14 + `{"write":false}` 15,
    // and two commas.
    assert_eq!(
        STATUS_CONNECT_MAX_BODY_BYTES,
        2 + 12 + 295 + 13 + 20 + 14 + 15 + 2
    );
    let mut buf = vec![0u8; STATUS_CONNECT_MAX_BODY_BYTES];
    assert_eq!(write_status_connect(&doc, &mut buf), Ok(len));
}

#[test]
fn the_machine_bound_is_the_largest_reply() {
    assert_eq!(
        MAX_BODY_BYTES,
        STATUS_CONNECT_MAX_BODY_BYTES.max(CONFIG_MAX_BODY_BYTES)
    );
    for leg in LEGS {
        assert!(leg.max_body_bytes() <= MAX_BODY_BYTES);
    }
    let doc = ConfigDoc {
        connect: ConfigDocConnect { endpoints: &[] },
        mode: "",
        ttl: 0,
    };
    assert!(config_len(&doc).expect("measures") <= CONFIG_MAX_BODY_BYTES);
}
