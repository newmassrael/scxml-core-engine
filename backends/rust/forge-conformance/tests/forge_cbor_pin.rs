// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Rust driver of the CBOR pin (`tests/forge/cbor_pin/pin.py`, docs/adr/0101
//! step 0).
//!
//! Built only for the `cbor-pin` feature, which `scripts/capture_cbor_pin.sh`
//! asks for: build.rs generates the pinned codecs and every document they import
//! into `OUT_DIR/cbor_pin` (its Step 7d), included below as siblings, since each
//! codec names its imports through `super::`. Each line of the file
//! `SCE_CBOR_PIN_INPUTS` names, `<codec>/<case> <hex>`, is decoded by that codec
//! and the value it read encoded again; the file `SCE_CBOR_PIN_OUTPUT` names gets
//! `<id> ok <hex>` with what the encode wrote, or `<id> refused <words>` naming
//! the `CodecError` the decode or the encode answered. With either variable unset
//! the run fails: there is nothing to capture and nowhere to write it.
// The generated modules declare more than a decode and an encode, which is all a
// capture reads of them.
#![allow(dead_code)]

use std::fmt::Write as _;

use sce_forge_runtime::codec::{CodecError, SceCursor};

mod causal_entry {
    include!(concat!(env!("OUT_DIR"), "/cbor_pin/causal_entry.rs"));
}
mod pattern_kind {
    include!(concat!(env!("OUT_DIR"), "/cbor_pin/pattern_kind.rs"));
}
mod payload_codec {
    include!(concat!(env!("OUT_DIR"), "/cbor_pin/payload_codec.rs"));
}
mod rpc_status {
    include!(concat!(env!("OUT_DIR"), "/cbor_pin/rpc_status.rs"));
}
mod envelope {
    include!(concat!(env!("OUT_DIR"), "/cbor_pin/envelope.rs"));
}
mod codec_cbor_map {
    include!(concat!(env!("OUT_DIR"), "/cbor_pin/codec_cbor_map.rs"));
}
mod enum_uds_nrc {
    include!(concat!(env!("OUT_DIR"), "/cbor_pin/enum_uds_nrc.rs"));
}
mod codec_cbor_open_enum {
    include!(concat!(
        env!("OUT_DIR"),
        "/cbor_pin/codec_cbor_open_enum.rs"
    ));
}
mod broadcast_log_conformance_text_event {
    include!(concat!(
        env!("OUT_DIR"),
        "/cbor_pin/broadcast_log_conformance_text_event.rs"
    ));
}
mod cbor_signed_status {
    include!(concat!(env!("OUT_DIR"), "/cbor_pin/cbor_signed_status.rs"));
}
mod codec_cbor_signed_enum {
    include!(concat!(
        env!("OUT_DIR"),
        "/cbor_pin/codec_cbor_signed_enum.rs"
    ));
}
mod cbor_causal_entry {
    include!(concat!(env!("OUT_DIR"), "/cbor_pin/cbor_causal_entry.rs"));
}
mod cbor_digest_entry {
    include!(concat!(env!("OUT_DIR"), "/cbor_pin/cbor_digest_entry.rs"));
}
mod codec_cbor_record_list {
    include!(concat!(
        env!("OUT_DIR"),
        "/cbor_pin/codec_cbor_record_list.rs"
    ));
}
mod codec_cbor_record_list_bytes {
    include!(concat!(
        env!("OUT_DIR"),
        "/cbor_pin/codec_cbor_record_list_bytes.rs"
    ));
}
mod codec_cbor_record_list_least {
    include!(concat!(
        env!("OUT_DIR"),
        "/cbor_pin/codec_cbor_record_list_least.rs"
    ));
}

/// What an encode returned, as the capture reads it: a codec whose encode can
/// refuse a value returns a `Result`, a total one its bytes (docs/adr/0065).
trait Encoded {
    fn encoded(self) -> Result<Vec<u8>, String>;
}

impl Encoded for Vec<u8> {
    fn encoded(self) -> Result<Vec<u8>, String> {
        Ok(self)
    }
}

impl Encoded for Result<Vec<u8>, CodecError> {
    fn encoded(self) -> Result<Vec<u8>, String> {
        self.map_err(|e| format!("encode {e:?}"))
    }
}

/// One decode of `input` and one encode of what it read.
type RoundTrip = fn(&[u8]) -> Result<Vec<u8>, String>;

macro_rules! round_trip {
    ($codec:ty) => {
        |input: &[u8]| -> Result<Vec<u8>, String> {
            let mut cursor = SceCursor::new(input);
            let value = <$codec>::decode(&mut cursor).map_err(|e| format!("decode {e:?}"))?;
            value.encode_to_vec().encoded()
        }
    };
}

/// Each pinned codec by its document's name.
fn codecs() -> [(&'static str, RoundTrip); 8] {
    [
        ("codec_cbor_map", round_trip!(codec_cbor_map::CodecCborMap)),
        (
            "broadcast_log_conformance_text_event",
            round_trip!(broadcast_log_conformance_text_event::BroadcastLogConformanceTextEvent),
        ),
        (
            "codec_cbor_open_enum",
            round_trip!(codec_cbor_open_enum::CodecCborOpenEnum),
        ),
        (
            "codec_cbor_signed_enum",
            round_trip!(codec_cbor_signed_enum::CodecCborSignedEnum),
        ),
        (
            "codec_cbor_record_list",
            round_trip!(codec_cbor_record_list::CodecCborRecordList),
        ),
        (
            "codec_cbor_record_list_bytes",
            round_trip!(codec_cbor_record_list_bytes::CodecCborRecordListBytes),
        ),
        (
            "codec_cbor_record_list_least",
            round_trip!(codec_cbor_record_list_least::CodecCborRecordListLeast),
        ),
        ("envelope", round_trip!(envelope::Envelope)),
    ]
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0 {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect()
}

#[test]
fn each_pinned_case_is_decoded_and_encoded_again() {
    let inputs = std::env::var_os("SCE_CBOR_PIN_INPUTS")
        .expect("SCE_CBOR_PIN_INPUTS names the cases to capture (scripts/capture_cbor_pin.sh)");
    let output = std::env::var_os("SCE_CBOR_PIN_OUTPUT")
        .expect("SCE_CBOR_PIN_OUTPUT names where the capture is written");
    let text = std::fs::read_to_string(&inputs)
        .unwrap_or_else(|e| panic!("read {}: {e}", inputs.to_string_lossy()));
    let codecs = codecs();
    let mut written = String::new();
    for line in text.lines().filter(|line| !line.is_empty()) {
        let (id, hex) = line
            .split_once(' ')
            .unwrap_or_else(|| panic!("not a case line: {line:.80}"));
        let codec = id.split_once('/').map_or(id, |(codec, _)| codec);
        let run = codecs
            .iter()
            .find(|(name, _)| *name == codec)
            .map(|(_, run)| *run)
            .unwrap_or_else(|| panic!("{id} names no pinned codec"));
        let input = unhex(hex).unwrap_or_else(|| panic!("{id}: the input is not hexadecimal"));
        match run(&input) {
            Ok(bytes) => {
                written.reserve(id.len() + 5 + bytes.len() * 2);
                written.push_str(id);
                written.push_str(" ok ");
                for b in &bytes {
                    write!(written, "{b:02x}").expect("a String takes every write");
                }
                written.push('\n');
            }
            Err(refusal) => {
                writeln!(written, "{id} refused {refusal}").expect("a String takes every write");
            }
        }
    }
    std::fs::write(&output, written)
        .unwrap_or_else(|e| panic!("write {}: {e}", output.to_string_lossy()));
}
