// SCE-GENERATED — DO NOT EDIT
// source-hash: 3590e0da727b5c60e46e46571a637da3e7beb956941ee2491fcd012174a020af
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

    /// [`Self::decode`] into a destination the caller owns, the one surface
    /// every codec offers so a codec that embeds this one decodes it where it
    /// will live. A map holds no bounded list to build in place, so this
    /// decodes and stores.
    pub fn decode_into(cursor: &mut SceCursor<'a>, _out: &mut Self) -> Result<(), CodecError> {
        *_out = Self::decode(cursor)?;
        Ok(())
    }

    /// Encode this map into `w`. An entry whose value breaks its declared
    /// bound — an exact length, a maximum size — is refused, not written.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), CodecError> {
        Self::encode_source(self, w)
    }

    /// [`encode`](Self::encode) into a new `Vec`.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Result<Vec<u8>, CodecError> {
        let mut out: Vec<u8> = Vec::new();
        self.encode(&mut VecSink::new(&mut out))?;
        Ok(out)
    }
}
// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `Envelope` says: its entries. The count the map head holds
/// is not among them; `Envelope` counts the entries present
/// ([`Envelope::encode_source`]).
pub trait EnvelopeSource {
    /// The entry with key 0, required.
    fn id(&self) -> &[u8];
    /// The entry with key 1, required.
    fn source(&self) -> &str;
    /// The entry with key 2, required.
    fn event_type(&self) -> &str;
    /// The entry with key 3, required.
    fn pattern(&self) -> pattern_kind::PatternKind;
    /// The entry with key 4, required.
    fn datacontenttype(&self) -> payload_codec::PayloadCodec;
    /// The entry with key 5, required.
    fn data(&self) -> &[u8];
    /// The entry with key 6.
    fn subject(&self) -> Option<&str>;
    /// The entry with key 7.
    fn correlation_id(&self) -> Option<&[u8]>;
    /// The entry with key 8.
    fn reply_to(&self) -> Option<&str>;
    /// The entry with key 9.
    fn invoke_id(&self) -> Option<&[u8]>;
    /// The entry with key 10.
    fn rpc_status(&self) -> Option<rpc_status::RpcStatus>;
    /// The entry with key 11.
    fn rpc_error_message(&self) -> Option<&str>;
    /// The entry with key 12.
    fn deadline_unix_ms(&self) -> Option<u64>;
    /// The entry with key 14.
    fn sequence_no(&self) -> Option<u64>;
    /// The entry with key 15.
    fn routing_id(&self) -> Option<&[u8]>;
    /// The entry with key 16.
    fn parallel_id(&self) -> Option<&str>;
    /// The entry with key 17.
    fn region_id(&self) -> Option<&str>;
    /// The entry with key 18.
    fn child_session_id(&self) -> Option<&str>;
}

impl<T: EnvelopeSource + ?Sized> EnvelopeSource for &T {
    fn id(&self) -> &[u8] {
        (**self).id()
    }
    fn source(&self) -> &str {
        (**self).source()
    }
    fn event_type(&self) -> &str {
        (**self).event_type()
    }
    fn pattern(&self) -> pattern_kind::PatternKind {
        (**self).pattern()
    }
    fn datacontenttype(&self) -> payload_codec::PayloadCodec {
        (**self).datacontenttype()
    }
    fn data(&self) -> &[u8] {
        (**self).data()
    }
    fn subject(&self) -> Option<&str> {
        (**self).subject()
    }
    fn correlation_id(&self) -> Option<&[u8]> {
        (**self).correlation_id()
    }
    fn reply_to(&self) -> Option<&str> {
        (**self).reply_to()
    }
    fn invoke_id(&self) -> Option<&[u8]> {
        (**self).invoke_id()
    }
    fn rpc_status(&self) -> Option<rpc_status::RpcStatus> {
        (**self).rpc_status()
    }
    fn rpc_error_message(&self) -> Option<&str> {
        (**self).rpc_error_message()
    }
    fn deadline_unix_ms(&self) -> Option<u64> {
        (**self).deadline_unix_ms()
    }
    fn sequence_no(&self) -> Option<u64> {
        (**self).sequence_no()
    }
    fn routing_id(&self) -> Option<&[u8]> {
        (**self).routing_id()
    }
    fn parallel_id(&self) -> Option<&str> {
        (**self).parallel_id()
    }
    fn region_id(&self) -> Option<&str> {
        (**self).region_id()
    }
    fn child_session_id(&self) -> Option<&str> {
        (**self).child_session_id()
    }
}

impl<'a> EnvelopeSource for Envelope<'a> {
    fn id(&self) -> &[u8] {
        self.id
    }
    fn source(&self) -> &str {
        self.source
    }
    fn event_type(&self) -> &str {
        self.event_type
    }
    fn pattern(&self) -> pattern_kind::PatternKind {
        self.pattern
    }
    fn datacontenttype(&self) -> payload_codec::PayloadCodec {
        self.datacontenttype
    }
    fn data(&self) -> &[u8] {
        self.data
    }
    fn subject(&self) -> Option<&str> {
        self.subject
    }
    fn correlation_id(&self) -> Option<&[u8]> {
        self.correlation_id
    }
    fn reply_to(&self) -> Option<&str> {
        self.reply_to
    }
    fn invoke_id(&self) -> Option<&[u8]> {
        self.invoke_id
    }
    fn rpc_status(&self) -> Option<rpc_status::RpcStatus> {
        self.rpc_status
    }
    fn rpc_error_message(&self) -> Option<&str> {
        self.rpc_error_message
    }
    fn deadline_unix_ms(&self) -> Option<u64> {
        self.deadline_unix_ms
    }
    fn sequence_no(&self) -> Option<u64> {
        self.sequence_no
    }
    fn routing_id(&self) -> Option<&[u8]> {
        self.routing_id
    }
    fn parallel_id(&self) -> Option<&str> {
        self.parallel_id
    }
    fn region_id(&self) -> Option<&str> {
        self.region_id
    }
    fn child_session_id(&self) -> Option<&str> {
        self.child_session_id
    }
}

#[allow(dead_code)]
impl<'a> Envelope<'a> {
    /// Encode what `src` says as one CBOR map into `w`: the entries present in
    /// ascending key order, every head in its shortest form. An entry past its
    /// declared bound is refused before the first byte is written.
    pub fn encode_source<S: EnvelopeSource + ?Sized, K: SceSink>(
        src: &S,
        w: &mut K,
    ) -> Result<(), CodecError> {
        Self::write_source(src, &mut ::sce_forge_runtime::codec::CountSink::new())?;
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the pre-pass a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: EnvelopeSource + ?Sized, K: SceSink>(
        src: &S,
        w: &mut K,
    ) -> Result<(), CodecError> {
        let mut count: u64 = 6;
        if src.subject().is_some() {
            count += 1;
        }
        if src.correlation_id().is_some() {
            count += 1;
        }
        if src.reply_to().is_some() {
            count += 1;
        }
        if src.invoke_id().is_some() {
            count += 1;
        }
        if src.rpc_status().is_some() {
            count += 1;
        }
        if src.rpc_error_message().is_some() {
            count += 1;
        }
        if src.deadline_unix_ms().is_some() {
            count += 1;
        }
        if src.sequence_no().is_some() {
            count += 1;
        }
        if src.routing_id().is_some() {
            count += 1;
        }
        if src.parallel_id().is_some() {
            count += 1;
        }
        if src.region_id().is_some() {
            count += 1;
        }
        if src.child_session_id().is_some() {
            count += 1;
        }
        cbor::write_map_head(w, count)?;
        let v = src.id();
        cbor::write_uint(w, 0)?;
        if v.len() != 16 {
            return Err(CodecError::CborWrongLength);
        }
        cbor::write_bytes(w, v)?;
        let v = src.source();
        cbor::write_uint(w, 1)?;
        if v.len() > 262144 {
            return Err(CodecError::CborOutOfRange);
        }
        cbor::write_text(w, v)?;
        let v = src.event_type();
        cbor::write_uint(w, 2)?;
        if v.len() > 262144 {
            return Err(CodecError::CborOutOfRange);
        }
        cbor::write_text(w, v)?;
        let v = src.pattern();
        cbor::write_uint(w, 3)?;
        cbor::write_uint(w, u64::from(v.to_underlying()))?;
        let v = src.datacontenttype();
        cbor::write_uint(w, 4)?;
        cbor::write_uint(w, u64::from(v.to_underlying()))?;
        let v = src.data();
        cbor::write_uint(w, 5)?;
        if v.len() > 16777216 {
            return Err(CodecError::CborOutOfRange);
        }
        cbor::write_bytes(w, v)?;
        if let Some(v) = src.subject() {
            cbor::write_uint(w, 6)?;
            if v.len() > 262144 {
                return Err(CodecError::CborOutOfRange);
            }
            cbor::write_text(w, v)?;
        }
        if let Some(v) = src.correlation_id() {
            cbor::write_uint(w, 7)?;
            if v.len() != 16 {
                return Err(CodecError::CborWrongLength);
            }
            cbor::write_bytes(w, v)?;
        }
        if let Some(v) = src.reply_to() {
            cbor::write_uint(w, 8)?;
            if v.len() > 262144 {
                return Err(CodecError::CborOutOfRange);
            }
            cbor::write_text(w, v)?;
        }
        if let Some(v) = src.invoke_id() {
            cbor::write_uint(w, 9)?;
            if v.len() != 16 {
                return Err(CodecError::CborWrongLength);
            }
            cbor::write_bytes(w, v)?;
        }
        if let Some(v) = src.rpc_status() {
            cbor::write_uint(w, 10)?;
            cbor::write_uint(w, u64::from(v.to_underlying()))?;
        }
        if let Some(v) = src.rpc_error_message() {
            cbor::write_uint(w, 11)?;
            if v.len() > 262144 {
                return Err(CodecError::CborOutOfRange);
            }
            cbor::write_text(w, v)?;
        }
        if let Some(v) = src.deadline_unix_ms() {
            cbor::write_uint(w, 12)?;
            cbor::write_uint(w, v)?;
        }
        if let Some(v) = src.sequence_no() {
            cbor::write_uint(w, 14)?;
            cbor::write_uint(w, v)?;
        }
        if let Some(v) = src.routing_id() {
            cbor::write_uint(w, 15)?;
            if v.len() != 16 {
                return Err(CodecError::CborWrongLength);
            }
            cbor::write_bytes(w, v)?;
        }
        if let Some(v) = src.parallel_id() {
            cbor::write_uint(w, 16)?;
            if v.len() > 262144 {
                return Err(CodecError::CborOutOfRange);
            }
            cbor::write_text(w, v)?;
        }
        if let Some(v) = src.region_id() {
            cbor::write_uint(w, 17)?;
            if v.len() > 262144 {
                return Err(CodecError::CborOutOfRange);
            }
            cbor::write_text(w, v)?;
        }
        if let Some(v) = src.child_session_id() {
            cbor::write_uint(w, 18)?;
            if v.len() > 262144 {
                return Err(CodecError::CborOutOfRange);
            }
            cbor::write_text(w, v)?;
        }
        Ok(())
    }
}
