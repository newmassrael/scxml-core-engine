// SCE-GENERATED — DO NOT EDIT
// source-hash: ab119d19c373fb9e83e74bd30f74186a9e2f87ef70ba045b5c5ab8bb9e9d1849
#![doc = "SCE-MAP: envelope.scxml:22 :: _forge_body"]
// SCE-MAP: envelope.scxml:22 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec" sce:encoding="cbor")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::cbor;
use sce_forge_runtime::codec::{CodecError, SceCursor, SceSink};
// `VecSink` and the heap-backed `encode_to_vec` facade are gated on the
// runtime's `alloc` feature, as they are for a positional codec.
#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
#[cfg(feature = "alloc")]
use sce_forge_runtime::codec::VecSink;

use super::pattern_kind;
use super::payload_codec;
use super::rpc_status;

/// One CBOR map (SCE_FORGE.md §4.6.1). Decode takes the keys in any order
/// and skips one this codec does not declare; encode writes the entries
/// present in ascending key order, every head in its shortest form.
// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec), so an unused field is not dead code.
#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct Envelope<'a> {
    /// Map key 0, required.
    pub id: &'a [u8],
    /// Map key 1, required.
    pub source: &'a str,
    /// Map key 2, required.
    pub event_type: &'a str,
    /// Map key 3, required.
    pub pattern: pattern_kind::PatternKind,
    /// Map key 4, required.
    pub datacontenttype: payload_codec::PayloadCodec,
    /// Map key 5, required.
    pub data: &'a [u8],
    /// Map key 6.
    pub subject: Option<&'a str>,
    /// Map key 7.
    pub correlation_id: Option<&'a [u8]>,
    /// Map key 8.
    pub reply_to: Option<&'a str>,
    /// Map key 9.
    pub invoke_id: Option<&'a [u8]>,
    /// Map key 10.
    pub rpc_status: Option<rpc_status::RpcStatus>,
    /// Map key 11.
    pub rpc_error_message: Option<&'a str>,
    /// Map key 12.
    pub deadline_unix_ms: Option<u64>,
    /// Map key 14.
    pub sequence_no: Option<u64>,
    /// Map key 15.
    pub routing_id: Option<&'a [u8]>,
    /// Map key 16.
    pub parallel_id: Option<&'a str>,
    /// Map key 17.
    pub region_id: Option<&'a str>,
    /// Map key 18.
    pub child_session_id: Option<&'a str>,
}

#[allow(dead_code)]
impl<'a> Envelope<'a> {
    /// An instance with every entry at its type's [`Default`] and every
    /// optional entry absent — the infallible constructor every codec
    /// offers, as a positional one does.
    pub fn new() -> Self {
        Self::default()
    }

    /// Decode one map from `cursor`. On success the cursor advances past it;
    /// on any refusal the cursor is left untouched.
    pub fn decode(cursor: &mut SceCursor<'a>) -> Result<Self, CodecError> {
        let mut c = *cursor;
        let count = cbor::read_map_len(&mut c)?;
        let mut id: Option<&'a [u8]> = None;
        let mut source: Option<&'a str> = None;
        let mut event_type: Option<&'a str> = None;
        let mut pattern: Option<pattern_kind::PatternKind> = None;
        let mut datacontenttype: Option<payload_codec::PayloadCodec> = None;
        let mut data: Option<&'a [u8]> = None;
        let mut subject: Option<&'a str> = None;
        let mut correlation_id: Option<&'a [u8]> = None;
        let mut reply_to: Option<&'a str> = None;
        let mut invoke_id: Option<&'a [u8]> = None;
        let mut rpc_status: Option<rpc_status::RpcStatus> = None;
        let mut rpc_error_message: Option<&'a str> = None;
        let mut deadline_unix_ms: Option<u64> = None;
        let mut sequence_no: Option<u64> = None;
        let mut routing_id: Option<&'a [u8]> = None;
        let mut parallel_id: Option<&'a str> = None;
        let mut region_id: Option<&'a str> = None;
        let mut child_session_id: Option<&'a str> = None;
        for _ in 0..count {
            match cbor::read_uint(&mut c)? {
                0 => {
                    // A key given twice is not one map entry.
                    if id.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    id = Some(cbor::read_bytes_exact(&mut c, 16)?);
                }
                1 => {
                    // A key given twice is not one map entry.
                    if source.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    source = Some(cbor::read_text(&mut c, Some(262144))?);
                }
                2 => {
                    // A key given twice is not one map entry.
                    if event_type.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    event_type = Some(cbor::read_text(&mut c, Some(262144))?);
                }
                3 => {
                    // A key given twice is not one map entry.
                    if pattern.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    pattern = Some(
                        pattern_kind::PatternKind::from_underlying(
                            cbor::read_uint(&mut c)?
                                .try_into()
                                .map_err(|_| CodecError::CborOutOfRange)?,
                        )
                        .ok_or(CodecError::UndeclaredEnumValue)?,
                    );
                }
                4 => {
                    // A key given twice is not one map entry.
                    if datacontenttype.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    datacontenttype = Some(
                        payload_codec::PayloadCodec::from_underlying(
                            cbor::read_uint(&mut c)?
                                .try_into()
                                .map_err(|_| CodecError::CborOutOfRange)?,
                        )
                        .ok_or(CodecError::UndeclaredEnumValue)?,
                    );
                }
                5 => {
                    // A key given twice is not one map entry.
                    if data.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    data = Some(cbor::read_bytes(&mut c, Some(16777216))?);
                }
                6 => {
                    // A key given twice is not one map entry.
                    if subject.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    subject = Some(cbor::read_text(&mut c, Some(262144))?);
                }
                7 => {
                    // A key given twice is not one map entry.
                    if correlation_id.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    correlation_id = Some(cbor::read_bytes_exact(&mut c, 16)?);
                }
                8 => {
                    // A key given twice is not one map entry.
                    if reply_to.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    reply_to = Some(cbor::read_text(&mut c, Some(262144))?);
                }
                9 => {
                    // A key given twice is not one map entry.
                    if invoke_id.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    invoke_id = Some(cbor::read_bytes_exact(&mut c, 16)?);
                }
                10 => {
                    // A key given twice is not one map entry.
                    if rpc_status.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    rpc_status = Some(
                        rpc_status::RpcStatus::from_underlying(
                            cbor::read_uint(&mut c)?
                                .try_into()
                                .map_err(|_| CodecError::CborOutOfRange)?,
                        )
                        .ok_or(CodecError::UndeclaredEnumValue)?,
                    );
                }
                11 => {
                    // A key given twice is not one map entry.
                    if rpc_error_message.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    rpc_error_message = Some(cbor::read_text(&mut c, Some(262144))?);
                }
                12 => {
                    // A key given twice is not one map entry.
                    if deadline_unix_ms.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    deadline_unix_ms = Some(cbor::read_uint(&mut c)?);
                }
                14 => {
                    // A key given twice is not one map entry.
                    if sequence_no.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    sequence_no = Some(cbor::read_uint(&mut c)?);
                }
                15 => {
                    // A key given twice is not one map entry.
                    if routing_id.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    routing_id = Some(cbor::read_bytes_exact(&mut c, 16)?);
                }
                16 => {
                    // A key given twice is not one map entry.
                    if parallel_id.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    parallel_id = Some(cbor::read_text(&mut c, Some(262144))?);
                }
                17 => {
                    // A key given twice is not one map entry.
                    if region_id.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    region_id = Some(cbor::read_text(&mut c, Some(262144))?);
                }
                18 => {
                    // A key given twice is not one map entry.
                    if child_session_id.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    child_session_id = Some(cbor::read_text(&mut c, Some(262144))?);
                }
                _ => cbor::skip(&mut c)?,
            }
        }
        let value = Self {
            id: id.ok_or(CodecError::CborRequiredKeyMissing)?,
            source: source.ok_or(CodecError::CborRequiredKeyMissing)?,
            event_type: event_type.ok_or(CodecError::CborRequiredKeyMissing)?,
            pattern: pattern.ok_or(CodecError::CborRequiredKeyMissing)?,
            datacontenttype: datacontenttype.ok_or(CodecError::CborRequiredKeyMissing)?,
            data: data.ok_or(CodecError::CborRequiredKeyMissing)?,
            subject,
            correlation_id,
            reply_to,
            invoke_id,
            rpc_status,
            rpc_error_message,
            deadline_unix_ms,
            sequence_no,
            routing_id,
            parallel_id,
            region_id,
            child_session_id,
        };
        // Only a map that decoded whole moves the cursor.
        *cursor = c;
        Ok(value)
    }

    /// Encode this map into `w`. An entry whose value breaks its declared
    /// bound — an exact length, a maximum size — is refused, not written.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), CodecError> {
        let mut count: u64 = 6;
        if self.subject.is_some() {
            count += 1;
        }
        if self.correlation_id.is_some() {
            count += 1;
        }
        if self.reply_to.is_some() {
            count += 1;
        }
        if self.invoke_id.is_some() {
            count += 1;
        }
        if self.rpc_status.is_some() {
            count += 1;
        }
        if self.rpc_error_message.is_some() {
            count += 1;
        }
        if self.deadline_unix_ms.is_some() {
            count += 1;
        }
        if self.sequence_no.is_some() {
            count += 1;
        }
        if self.routing_id.is_some() {
            count += 1;
        }
        if self.parallel_id.is_some() {
            count += 1;
        }
        if self.region_id.is_some() {
            count += 1;
        }
        if self.child_session_id.is_some() {
            count += 1;
        }
        cbor::write_map_head(w, count)?;
        let v = self.id;
        cbor::write_uint(w, 0)?;
        if v.len() != 16 {
            return Err(CodecError::CborWrongLength);
        }
        cbor::write_bytes(w, v)?;
        let v = self.source;
        cbor::write_uint(w, 1)?;
        if v.len() > 262144 {
            return Err(CodecError::CborOutOfRange);
        }
        cbor::write_text(w, v)?;
        let v = self.event_type;
        cbor::write_uint(w, 2)?;
        if v.len() > 262144 {
            return Err(CodecError::CborOutOfRange);
        }
        cbor::write_text(w, v)?;
        let v = self.pattern;
        cbor::write_uint(w, 3)?;
        cbor::write_uint(w, u64::from(v.to_underlying()))?;
        let v = self.datacontenttype;
        cbor::write_uint(w, 4)?;
        cbor::write_uint(w, u64::from(v.to_underlying()))?;
        let v = self.data;
        cbor::write_uint(w, 5)?;
        if v.len() > 16777216 {
            return Err(CodecError::CborOutOfRange);
        }
        cbor::write_bytes(w, v)?;
        if let Some(v) = self.subject {
            cbor::write_uint(w, 6)?;
            if v.len() > 262144 {
                return Err(CodecError::CborOutOfRange);
            }
            cbor::write_text(w, v)?;
        }
        if let Some(v) = self.correlation_id {
            cbor::write_uint(w, 7)?;
            if v.len() != 16 {
                return Err(CodecError::CborWrongLength);
            }
            cbor::write_bytes(w, v)?;
        }
        if let Some(v) = self.reply_to {
            cbor::write_uint(w, 8)?;
            if v.len() > 262144 {
                return Err(CodecError::CborOutOfRange);
            }
            cbor::write_text(w, v)?;
        }
        if let Some(v) = self.invoke_id {
            cbor::write_uint(w, 9)?;
            if v.len() != 16 {
                return Err(CodecError::CborWrongLength);
            }
            cbor::write_bytes(w, v)?;
        }
        if let Some(v) = self.rpc_status {
            cbor::write_uint(w, 10)?;
            cbor::write_uint(w, u64::from(v.to_underlying()))?;
        }
        if let Some(v) = self.rpc_error_message {
            cbor::write_uint(w, 11)?;
            if v.len() > 262144 {
                return Err(CodecError::CborOutOfRange);
            }
            cbor::write_text(w, v)?;
        }
        if let Some(v) = self.deadline_unix_ms {
            cbor::write_uint(w, 12)?;
            cbor::write_uint(w, v)?;
        }
        if let Some(v) = self.sequence_no {
            cbor::write_uint(w, 14)?;
            cbor::write_uint(w, v)?;
        }
        if let Some(v) = self.routing_id {
            cbor::write_uint(w, 15)?;
            if v.len() != 16 {
                return Err(CodecError::CborWrongLength);
            }
            cbor::write_bytes(w, v)?;
        }
        if let Some(v) = self.parallel_id {
            cbor::write_uint(w, 16)?;
            if v.len() > 262144 {
                return Err(CodecError::CborOutOfRange);
            }
            cbor::write_text(w, v)?;
        }
        if let Some(v) = self.region_id {
            cbor::write_uint(w, 17)?;
            if v.len() > 262144 {
                return Err(CodecError::CborOutOfRange);
            }
            cbor::write_text(w, v)?;
        }
        if let Some(v) = self.child_session_id {
            cbor::write_uint(w, 18)?;
            if v.len() > 262144 {
                return Err(CodecError::CborOutOfRange);
            }
            cbor::write_text(w, v)?;
        }
        Ok(())
    }

    /// [`encode`](Self::encode) into a new `Vec`.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Result<Vec<u8>, CodecError> {
        let mut out: Vec<u8> = Vec::new();
        self.encode(&mut VecSink::new(&mut out))?;
        Ok(out)
    }
}
