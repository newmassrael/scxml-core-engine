// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A link carries the events it declares (SCE Protocol-Synthesis RFC
//! link section, "Codegen contract", docs/adr/0106 #142): the receive path
//! `driver.poll() -> bytes -> framer.decode() -> event inject` and the
//! transmit path `event extract -> encode -> pool slot -> driver.send()`, on
//! the two backends that generate a link (Rust and C11), with the codecs read
//! through the link's own `<sce:import>`.

use std::fs;
use std::path::Path;
use tempfile::tempdir;

use sce_build::compile_forge_with_imports;
use sce_build::forge::error::ForgeError;
use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

/// A fixed-layout codec of three bytes: what every link here frames by.
const FRAMER: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="little" name="scout_frame_codec">
  <datamodel>
    <data id="msg_id" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <data id="payload_len" sce:type="uint16" sce:byte="1" sce:bit-size="16"/>
  </datamodel>
</scxml>"##;

/// A transmit pool whose slot holds the framer's three bytes.
const TX_POOL: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="buffer-pool" name="scout_tx_pool" version="1.0">
  <sce:slot-count>4</sce:slot-count>
  <sce:slot-size>64</sce:slot-size>
  <sce:section>sram1</sce:section>
  <sce:alignment>32</sce:alignment>
  <sce:cache-policy>none</sce:cache-policy>
</scxml>"##;

/// A link named `udp_scout` with `body` after its framer import, its class,
/// its framer and its policy.
fn link(body: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="link" name="udp_scout" version="1.0">
  <sce:import as="scout_frame_codec" src="scout_frame_codec.scxml" kind="codec"/>
  <sce:link-class>udp</sce:link-class>
  <sce:framer ref="scout_frame_codec"/>
  <sce:backpressure>drop</sce:backpressure>
{body}
</scxml>"##
    )
}

const INBOUND: &str = r##"  <sce:events>
    <sce:inbound event="scout.hello.received" when="decoded.msg_id === 0x02"/>
    <sce:inbound event="scout.any.received"/>
  </sce:events>"##;

const OUTBOUND: &str = r##"  <sce:events>
    <sce:outbound event="scout.query.send" encode="scout_frame_codec"/>
  </sce:events>"##;

const OUTBOUND_THROUGH_POOL: &str = r##"  <sce:import as="scout_tx_pool" src="scout_tx_pool.scxml" kind="buffer-pool"/>
  <sce:tx-pool ref="scout_tx_pool"/>
  <sce:events>
    <sce:outbound event="scout.query.send" encode="scout_frame_codec"/>
  </sce:events>"##;

fn generate(body: &str, lang: Language) -> Result<String, ForgeError> {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("scout_frame_codec.scxml"), FRAMER).expect("write framer");
    fs::write(dir.path().join("scout_tx_pool.scxml"), TX_POOL).expect("write pool");
    compile(&link(body), lang, dir.path())
}

fn compile(scxml: &str, lang: Language, base_dir: &Path) -> Result<String, ForgeError> {
    let out = compile_forge_with_imports(
        scxml,
        DocumentLabel::symmetric("udp_scout"),
        lang,
        base_dir,
        &ForgeCompileOptions::default(),
    )
    .map_err(|e| e.error)?;
    assert_eq!(out.files.len(), 1, "a link emits one file");
    Ok(out.files[0].1.clone())
}

/// The line after the `when` row's comment opens with `opening` and reads the
/// decoded frame's field.
fn assert_guarded_by_the_decoded_frame(body: &str, opening: &str) {
    let comment = "`<sce:inbound event=\"scout.hello.received\">` when it holds.";
    let at = body
        .find(comment)
        .unwrap_or_else(|| panic!("the `when` row is commented; full source:\n{body}"));
    let guard = body[at..].lines().nth(1).unwrap_or_default();
    assert!(
        guard.starts_with(opening) && guard.contains("decoded.msg_id"),
        "the `when` row is an `if` over the decoded frame, got `{guard}`; full source:\n{body}"
    );
}

fn assert_has(body: &str, needle: &str) {
    assert!(
        body.contains(needle),
        "expected `{needle}` in the generated link; full source:\n{body}"
    );
}

// ─── Rust ───────────────────────────────────────────────────────────

#[test]
fn rust_receive_path_decodes_with_the_framer_and_injects_every_row_that_holds() {
    let body = generate(INBOUND, Language::Rust).expect("rust link with inbound rows");
    assert_has(&body, "use super::scout_frame_codec::ScoutFrameCodec;");
    assert_has(&body, "pub enum UdpScoutInbound {");
    assert_has(
        &body,
        "Self::ScoutHelloReceived => \"scout.hello.received\",",
    );
    assert_has(&body, "pub fn pump<I, R>(");
    assert_has(&body, "I: FnMut(UdpScoutInbound, &ScoutFrameCodec),");
    assert_has(&body, "self.driver.poll(deadline_us);");
    assert_has(&body, "ScoutFrameCodec::decode(&mut cursor)");
    assert_has(&body, "refused(error);");
    // The row with `when` is guarded by the predicate over the decoded frame;
    // the row without it injects every frame.
    assert_guarded_by_the_decoded_frame(&body, "            if ");
    assert_has(
        &body,
        "            inject(UdpScoutInbound::ScoutAnyReceived, &decoded);\n            injected += 1;",
    );
}

#[test]
fn rust_transmit_path_encodes_into_a_buffer_of_the_largest_encoding() {
    let body = generate(OUTBOUND, Language::Rust).expect("rust link with an outbound row");
    assert_has(&body, "pub enum UdpScoutOutbound {");
    assert_has(&body, "ScoutQuerySend(ScoutFrameCodec),");
    assert_has(
        &body,
        "pub fn send(&mut self, event: &UdpScoutOutbound) -> Result<(), UdpScoutSendError> {",
    );
    assert_has(&body, "let mut buffer = [0u8; 3];");
    assert_has(
        &body,
        "UdpScoutOutbound::ScoutQuerySend(value) => value.encode(&mut sink),",
    );
    assert_has(&body, ".map_err(UdpScoutSendError::Link)");
    assert!(
        !body.contains("PoolExhausted"),
        "a link with no transmit pool has no pool to exhaust; full source:\n{body}"
    );
    assert!(
        !body.contains("pub fn pump"),
        "a link with no inbound row has no receive path; full source:\n{body}"
    );
}

#[test]
fn rust_transmit_path_encodes_into_a_slot_of_the_transmit_pool() {
    let body =
        generate(OUTBOUND_THROUGH_POOL, Language::Rust).expect("rust link with a transmit pool");
    assert_has(
        &body,
        "pub fn send(&mut self, tx_pool: &mut super::scout_tx_pool::ScoutTxPool, event: &UdpScoutOutbound)",
    );
    assert_has(&body, ".pool_acquire_for_encode()");
    assert_has(&body, ".ok_or(UdpScoutSendError::PoolExhausted)?;");
    assert_has(&body, "SliceSink::new(&mut slot.write(tx_pool)[..])");
    assert_has(&body, "slot.pool_return(tx_pool);");
}

#[test]
fn a_link_without_events_stays_a_byte_endpoint() {
    let body = generate("", Language::Rust).expect("rust link without events");
    assert_has(
        &body,
        "pub fn tx(&mut self, bytes: &[u8]) -> Result<(), LinkError>",
    );
    assert!(
        !body.contains("CodecError")
            && !body.contains("pub fn pump")
            && !body.contains("pub fn send"),
        "a link that declares no event decodes and encodes nothing; full source:\n{body}"
    );
}

#[test]
fn events_through_a_framer_the_link_does_not_import_are_refused() {
    let dir = tempdir().expect("tempdir");
    let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="link" name="udp_scout" version="1.0">
  <sce:link-class>udp</sce:link-class>
  <sce:framer ref="scout_frame_codec"/>
  <sce:backpressure>drop</sce:backpressure>
  <sce:events>
    <sce:inbound event="scout.hello.received"/>
  </sce:events>
</scxml>"##;
    for lang in [Language::Rust, Language::C11] {
        let err = compile(scxml, lang, dir.path()).expect_err("an unread framer is refused");
        let text = err.to_string();
        assert!(
            text.contains("link 'udp_scout'")
                && text.contains(
                    "the codec 'scout_frame_codec' its <sce:framer ref> names is not an import"
                ),
            "{lang:?}: the refusal names the link, the codec and the import to add; got {text}"
        );
    }
}

#[test]
fn two_events_spelled_alike_in_generated_code_are_refused() {
    let body = r##"  <sce:events>
    <sce:inbound event="scout.hello"/>
    <sce:inbound event="scout-hello"/>
  </sce:events>"##;
    let err = generate(body, Language::Rust).expect_err("a variant two names share is refused");
    let text = err.to_string();
    assert!(
        text.contains("its events 'scout.hello' and 'scout-hello' are spelled alike"),
        "the refusal names both events; got {text}"
    );
}

// ─── C11 ────────────────────────────────────────────────────────────

#[test]
fn c11_receive_path_decodes_with_the_framer_and_injects_every_row_that_holds() {
    let body = generate(INBOUND, Language::C11).expect("c11 link with inbound rows");
    assert_has(&body, "#include \"sce/forge/codec.h\"");
    assert_has(&body, "#include \"scout_frame_codec.h\"");
    assert_has(&body, "UDP_SCOUT_INBOUND_SCOUT_HELLO_RECEIVED = 0,");
    assert_has(&body, "} udp_scout_inbound_t;");
    assert_has(&body, "static inline size_t udp_scout_link_pump(");
    assert_has(
        &body,
        "self->driver.ops->poll(self->driver.self, deadline_us);",
    );
    assert_has(
        &body,
        "sce_forge_codec_status_t status = scout_frame_codec_decode(&cursor, &decoded);",
    );
    assert_has(&body, "refused(ctx, status);");
    assert_guarded_by_the_decoded_frame(&body, "        if (");
    assert_has(
        &body,
        "        inject(ctx, UDP_SCOUT_INBOUND_SCOUT_ANY_RECEIVED, &decoded);\n        ++injected;",
    );
}

#[test]
fn c11_transmit_path_encodes_into_a_buffer_of_the_largest_encoding() {
    let body = generate(OUTBOUND, Language::C11).expect("c11 link with an outbound row");
    assert_has(
        &body,
        "static inline udp_scout_send_result_t udp_scout_link_send_scout_query_send(",
    );
    assert_has(&body, "uint8_t buffer[3];");
    assert_has(
        &body,
        "result.codec_status = scout_frame_codec_encode_to_buf(value, buffer, sizeof buffer, &length);",
    );
    assert_has(
        &body,
        "result.link_status = self->driver.ops->tx(self->driver.self, frame);",
    );
}

#[test]
fn c11_transmit_path_encodes_into_a_slot_of_the_transmit_pool() {
    let body =
        generate(OUTBOUND_THROUGH_POOL, Language::C11).expect("c11 link with a transmit pool");
    assert_has(&body, "#include \"scout_tx_pool.h\"");
    assert_has(&body, "slot = scout_tx_pool_pool_acquire_for_encode();");
    assert_has(&body, "result.pool_exhausted = true;");
    assert_has(&body, "SCOUT_TX_POOL_SLOT_SIZE, &length);");
    assert_has(&body, "(void)scout_tx_pool_pool_return(&slot);");
}
