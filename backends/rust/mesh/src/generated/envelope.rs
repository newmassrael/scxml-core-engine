// SCE-GENERATED — DO NOT EDIT
// source-hash: f2f8682669670c7aaedbccb3102dd71d7b187eac29a32d3d20509a5bc070c12e
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

use super::causal_entry;
use super::pattern_kind;
use super::payload_codec;
use super::rpc_status;

/// Read one record of the event schema `causal_entry` (`record:CausalEntry`)
/// from `c`, as a list of this map holds it (docs/adr/0085, Landing order step
/// 3a), into the payload type the schema's own unit declares: one CBOR map of
/// every field, each under its id as text (docs/adr/0086, section 9.2). Its keys
/// in any order, a key the schema does not declare skipped whole, a key given
/// twice and a field missing refused, and a text or byte string past its bound
/// refused.
fn decode_record_causal_entry(
    c: &mut SceCursor<'_>,
) -> Result<causal_entry::CausalEntryPayload, CodecError> {
    let count = cbor::read_map_len(c)?;
    let mut held_hi: Option<u64> = None;
    let mut held_lo: Option<u64> = None;
    let mut held_seq: Option<u64> = None;
    for _ in 0..count {
        match cbor::read_text(c, None)? {
            "hi" => {
                // A key given twice is not one map entry.
                if held_hi.is_some() {
                    return Err(CodecError::CborMalformed);
                }
                held_hi = Some(cbor::read_uint(c)?);
            }
            "lo" => {
                // A key given twice is not one map entry.
                if held_lo.is_some() {
                    return Err(CodecError::CborMalformed);
                }
                held_lo = Some(cbor::read_uint(c)?);
            }
            "seq" => {
                // A key given twice is not one map entry.
                if held_seq.is_some() {
                    return Err(CodecError::CborMalformed);
                }
                held_seq = Some(cbor::read_uint(c)?);
            }
            _ => cbor::skip(c)?,
        }
    }
    Ok(causal_entry::CausalEntryPayload {
        hi: held_hi.ok_or(CodecError::CborRequiredKeyMissing)?,
        lo: held_lo.ok_or(CodecError::CborRequiredKeyMissing)?,
        seq: held_seq.ok_or(CodecError::CborRequiredKeyMissing)?,
    })
}

/// Write one record of `causal_entry` as one map of every field, the keys in
/// the order RFC 8949 §4.2.1 sorts their encodings: a shorter key first.
fn encode_record_causal_entry<S: SceSink + ?Sized>(
    w: &mut S,
    record: &causal_entry::CausalEntryPayload,
) -> Result<(), S::Error> {
    cbor::write_map_head(w, 3)?;
    cbor::write_text(w, "hi")?;
    cbor::write_uint(w, record.hi)?;
    cbor::write_text(w, "lo")?;
    cbor::write_uint(w, record.lo)?;
    cbor::write_text(w, "seq")?;
    cbor::write_uint(w, record.seq)?;
    Ok(())
}

/// One CBOR map (SCE_FORGE.md §4.6.1). Decode takes the keys in any order
/// and skips one this codec does not declare; encode writes the entries
/// present in ascending key order, every head in its shortest form.
// pub API: codecs are intended for cross-crate consumption (SCE_FORGE.md
// §6 codec), so an unused field is not dead code.
#[allow(dead_code)]
#[derive(Default, Debug, Clone, PartialEq)]
pub struct Envelope<'a> {
    /// Map key 0, required.
    pub id: ::sce_forge_runtime::codec::Bytes<'a, 16, 16>,
    /// Map key 1, required.
    pub source: ::sce_forge_runtime::codec::Text<'a, 262144>,
    /// Map key 2, required.
    pub event_type: ::sce_forge_runtime::codec::Text<'a, 262144>,
    /// Map key 3, required.
    pub pattern: pattern_kind::PatternKind,
    /// Map key 4, required.
    pub datacontenttype: payload_codec::PayloadCodec,
    /// Map key 5, required.
    pub data: ::sce_forge_runtime::codec::Bytes<'a, 16777216>,
    /// Map key 6.
    pub subject: Option<::sce_forge_runtime::codec::Text<'a, 262144>>,
    /// Map key 7.
    pub correlation_id: Option<::sce_forge_runtime::codec::Bytes<'a, 16, 16>>,
    /// Map key 8.
    pub reply_to: Option<::sce_forge_runtime::codec::Text<'a, 262144>>,
    /// Map key 9.
    pub invoke_id: Option<::sce_forge_runtime::codec::Bytes<'a, 16, 16>>,
    /// Map key 10.
    pub rpc_status: Option<rpc_status::RpcStatus>,
    /// Map key 11.
    pub rpc_error_message: Option<::sce_forge_runtime::codec::Text<'a, 262144>>,
    /// Map key 12.
    pub deadline_unix_ms: Option<u64>,
    /// Map key 14.
    pub sequence_no: Option<u64>,
    /// Map key 15.
    pub routing_id: Option<::sce_forge_runtime::codec::Bytes<'a, 16, 16>>,
    /// Map key 16.
    pub parallel_id: Option<::sce_forge_runtime::codec::Text<'a, 262144>>,
    /// Map key 17.
    pub region_id: Option<::sce_forge_runtime::codec::Text<'a, 262144>>,
    /// Map key 18.
    pub child_session_id: Option<::sce_forge_runtime::codec::Text<'a, 262144>>,
    /// Map key 19.
    pub causal_epoch: Option<u64>,
    /// Map key 20.
    pub causal_vector:
        Option<::sce_forge_runtime::codec::BoundedList<causal_entry::CausalEntryPayload, 64>>,
}

/// What the codec's wire is written through, as every codec has: a map's value
/// stores nothing and takes no profile, so it is this type (docs/adr/0077 §7).
#[allow(dead_code)]
pub type EnvelopeWire<'a> = Envelope<'a>;

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
        let mut id: Option<::sce_forge_runtime::codec::Bytes<'a, 16, 16>> = None;
        let mut source: Option<::sce_forge_runtime::codec::Text<'a, 262144>> = None;
        let mut event_type: Option<::sce_forge_runtime::codec::Text<'a, 262144>> = None;
        let mut pattern: Option<pattern_kind::PatternKind> = None;
        let mut datacontenttype: Option<payload_codec::PayloadCodec> = None;
        let mut data: Option<::sce_forge_runtime::codec::Bytes<'a, 16777216>> = None;
        let mut subject: Option<::sce_forge_runtime::codec::Text<'a, 262144>> = None;
        let mut correlation_id: Option<::sce_forge_runtime::codec::Bytes<'a, 16, 16>> = None;
        let mut reply_to: Option<::sce_forge_runtime::codec::Text<'a, 262144>> = None;
        let mut invoke_id: Option<::sce_forge_runtime::codec::Bytes<'a, 16, 16>> = None;
        let mut rpc_status: Option<rpc_status::RpcStatus> = None;
        let mut rpc_error_message: Option<::sce_forge_runtime::codec::Text<'a, 262144>> = None;
        let mut deadline_unix_ms: Option<u64> = None;
        let mut sequence_no: Option<u64> = None;
        let mut routing_id: Option<::sce_forge_runtime::codec::Bytes<'a, 16, 16>> = None;
        let mut parallel_id: Option<::sce_forge_runtime::codec::Text<'a, 262144>> = None;
        let mut region_id: Option<::sce_forge_runtime::codec::Text<'a, 262144>> = None;
        let mut child_session_id: Option<::sce_forge_runtime::codec::Text<'a, 262144>> = None;
        let mut causal_epoch: Option<u64> = None;
        let mut causal_vector: Option<
            ::sce_forge_runtime::codec::BoundedList<causal_entry::CausalEntryPayload, 64>,
        > = None;
        for _ in 0..count {
            match cbor::read_uint(&mut c)? {
                0 => {
                    // A key given twice is not one map entry.
                    if id.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    id = Some(
                        ::sce_forge_runtime::codec::Bounded::new(cbor::read_bytes_exact(
                            &mut c, 16,
                        )?)
                        .ok_or(CodecError::CborWrongLength)?,
                    );
                }
                1 => {
                    // A key given twice is not one map entry.
                    if source.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    source = Some(
                        ::sce_forge_runtime::codec::Bounded::new(cbor::read_text(
                            &mut c,
                            Some(262144),
                        )?)
                        .ok_or(CodecError::CborOutOfRange)?,
                    );
                }
                2 => {
                    // A key given twice is not one map entry.
                    if event_type.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    event_type = Some(
                        ::sce_forge_runtime::codec::Bounded::new(cbor::read_text(
                            &mut c,
                            Some(262144),
                        )?)
                        .ok_or(CodecError::CborOutOfRange)?,
                    );
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
                    data = Some(
                        ::sce_forge_runtime::codec::Bounded::new(cbor::read_bytes(
                            &mut c,
                            Some(16777216),
                        )?)
                        .ok_or(CodecError::CborOutOfRange)?,
                    );
                }
                6 => {
                    // A key given twice is not one map entry.
                    if subject.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    subject = Some(
                        ::sce_forge_runtime::codec::Bounded::new(cbor::read_text(
                            &mut c,
                            Some(262144),
                        )?)
                        .ok_or(CodecError::CborOutOfRange)?,
                    );
                }
                7 => {
                    // A key given twice is not one map entry.
                    if correlation_id.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    correlation_id = Some(
                        ::sce_forge_runtime::codec::Bounded::new(cbor::read_bytes_exact(
                            &mut c, 16,
                        )?)
                        .ok_or(CodecError::CborWrongLength)?,
                    );
                }
                8 => {
                    // A key given twice is not one map entry.
                    if reply_to.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    reply_to = Some(
                        ::sce_forge_runtime::codec::Bounded::new(cbor::read_text(
                            &mut c,
                            Some(262144),
                        )?)
                        .ok_or(CodecError::CborOutOfRange)?,
                    );
                }
                9 => {
                    // A key given twice is not one map entry.
                    if invoke_id.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    invoke_id = Some(
                        ::sce_forge_runtime::codec::Bounded::new(cbor::read_bytes_exact(
                            &mut c, 16,
                        )?)
                        .ok_or(CodecError::CborWrongLength)?,
                    );
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
                    rpc_error_message = Some(
                        ::sce_forge_runtime::codec::Bounded::new(cbor::read_text(
                            &mut c,
                            Some(262144),
                        )?)
                        .ok_or(CodecError::CborOutOfRange)?,
                    );
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
                    routing_id = Some(
                        ::sce_forge_runtime::codec::Bounded::new(cbor::read_bytes_exact(
                            &mut c, 16,
                        )?)
                        .ok_or(CodecError::CborWrongLength)?,
                    );
                }
                16 => {
                    // A key given twice is not one map entry.
                    if parallel_id.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    parallel_id = Some(
                        ::sce_forge_runtime::codec::Bounded::new(cbor::read_text(
                            &mut c,
                            Some(262144),
                        )?)
                        .ok_or(CodecError::CborOutOfRange)?,
                    );
                }
                17 => {
                    // A key given twice is not one map entry.
                    if region_id.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    region_id = Some(
                        ::sce_forge_runtime::codec::Bounded::new(cbor::read_text(
                            &mut c,
                            Some(262144),
                        )?)
                        .ok_or(CodecError::CborOutOfRange)?,
                    );
                }
                18 => {
                    // A key given twice is not one map entry.
                    if child_session_id.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    child_session_id = Some(
                        ::sce_forge_runtime::codec::Bounded::new(cbor::read_text(
                            &mut c,
                            Some(262144),
                        )?)
                        .ok_or(CodecError::CborOutOfRange)?,
                    );
                }
                19 => {
                    // A key given twice is not one map entry.
                    if causal_epoch.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    causal_epoch = Some(cbor::read_uint(&mut c)?);
                }
                20 => {
                    // A key given twice is not one map entry.
                    if causal_vector.is_some() {
                        return Err(CodecError::CborMalformed);
                    }
                    causal_vector = Some({
                        // A definite-length array within the list's counts,
                        // each item one record's map.
                        let len = cbor::read_array_len(&mut c)?;
                        if len > 64 {
                            return Err(CodecError::CborOutOfRange);
                        }
                        let mut list: ::sce_forge_runtime::codec::BoundedList<
                            causal_entry::CausalEntryPayload,
                            64,
                        > = ::sce_forge_runtime::codec::BoundedList::new();
                        for _ in 0..len {
                            list.push(decode_record_causal_entry(&mut c)?)?;
                        }
                        list
                    });
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
            causal_epoch,
            causal_vector,
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

    /// Encode this map into `w`. No entry has a bound a value can break
    /// (docs/adr/0065), so the only error is the sink's own.
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), S::Error> {
        Self::encode_source(self, w)
    }

    /// [`encode`](Self::encode) into a new `Vec`, which refuses no value.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        let Ok(()) = self.encode(&mut VecSink::new(&mut out));
        out
    }
}

// ── Owned projection (storage-parameterised native form) ─────────────
// `Envelope<'a>` above decodes a string or a byte string to a view of
// the buffer it was read from. A consumer that keeps a decoded map past that
// buffer, as a broadcast log does when it hands a read back, calls
// `try_into_owned_in` for this lifetime-free `EnvelopeOwned`, which copies
// each string and byte string into the storage profile's container for it.
//
// The profile is the one a positional codec's owned form takes:
// `EnvelopeOwned<Heap>` grows its containers and treats each declared
// `sce:max-size` as advisory, `EnvelopeOwned<Inline>` holds every entry
// inline at its declared capacity and never allocates, and
// `EnvelopeOwned` alone is the build's default profile. This form exists
// because every string and byte string of the map states a bound (`sce:max-size`,
// or the `sce:length` of an exact byte string): a container needs a capacity to be
// declared at, and an entry with none has no owned form.
//
// Each container is held inside a `Bounded` of its entry's bounds
// (docs/adr/0077 §3), so a value is within them on every profile.
// `try_into_owned_in` copies a view into the profile; `as_borrowed` re-borrows any profile back into
// the view; `transcode_in` moves a value between profiles as a checked
// projection; and `encode` writes the owned value through the source interface,
// without the view.
//
// Decoding picks the profile from the call (`try_into_owned_in::<Inline>()`).
// Hand-assembling a value names the profile on the value or its binding, because
// the fields reach it through its associated container types.
/// The field `id` of `Envelope`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct EnvelopeIdField;

impl ::sce_forge_runtime::codec::HoldsBytes<EnvelopeIdField>
    for ::sce_forge_runtime::codec::Inline
{
    type Bytes = ::sce_forge_runtime::codec::InlineBytes<16>;
}

/// The field `source` of `Envelope`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct EnvelopeSourceField;

impl ::sce_forge_runtime::codec::HoldsStr<EnvelopeSourceField>
    for ::sce_forge_runtime::codec::Inline
{
    type Str = ::sce_forge_runtime::codec::InlineStr<262144>;
}

/// The field `event_type` of `Envelope`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct EnvelopeEventTypeField;

impl ::sce_forge_runtime::codec::HoldsStr<EnvelopeEventTypeField>
    for ::sce_forge_runtime::codec::Inline
{
    type Str = ::sce_forge_runtime::codec::InlineStr<262144>;
}

/// The field `data` of `Envelope`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct EnvelopeDataField;

impl ::sce_forge_runtime::codec::HoldsBytes<EnvelopeDataField>
    for ::sce_forge_runtime::codec::Inline
{
    type Bytes = ::sce_forge_runtime::codec::InlineBytes<16777216>;
}

/// The field `subject` of `Envelope`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct EnvelopeSubjectField;

impl ::sce_forge_runtime::codec::HoldsStr<EnvelopeSubjectField>
    for ::sce_forge_runtime::codec::Inline
{
    type Str = ::sce_forge_runtime::codec::InlineStr<262144>;
}

/// The field `correlation_id` of `Envelope`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct EnvelopeCorrelationIdField;

impl ::sce_forge_runtime::codec::HoldsBytes<EnvelopeCorrelationIdField>
    for ::sce_forge_runtime::codec::Inline
{
    type Bytes = ::sce_forge_runtime::codec::InlineBytes<16>;
}

/// The field `reply_to` of `Envelope`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct EnvelopeReplyToField;

impl ::sce_forge_runtime::codec::HoldsStr<EnvelopeReplyToField>
    for ::sce_forge_runtime::codec::Inline
{
    type Str = ::sce_forge_runtime::codec::InlineStr<262144>;
}

/// The field `invoke_id` of `Envelope`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct EnvelopeInvokeIdField;

impl ::sce_forge_runtime::codec::HoldsBytes<EnvelopeInvokeIdField>
    for ::sce_forge_runtime::codec::Inline
{
    type Bytes = ::sce_forge_runtime::codec::InlineBytes<16>;
}

/// The field `rpc_error_message` of `Envelope`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct EnvelopeRpcErrorMessageField;

impl ::sce_forge_runtime::codec::HoldsStr<EnvelopeRpcErrorMessageField>
    for ::sce_forge_runtime::codec::Inline
{
    type Str = ::sce_forge_runtime::codec::InlineStr<262144>;
}

/// The field `routing_id` of `Envelope`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct EnvelopeRoutingIdField;

impl ::sce_forge_runtime::codec::HoldsBytes<EnvelopeRoutingIdField>
    for ::sce_forge_runtime::codec::Inline
{
    type Bytes = ::sce_forge_runtime::codec::InlineBytes<16>;
}

/// The field `parallel_id` of `Envelope`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct EnvelopeParallelIdField;

impl ::sce_forge_runtime::codec::HoldsStr<EnvelopeParallelIdField>
    for ::sce_forge_runtime::codec::Inline
{
    type Str = ::sce_forge_runtime::codec::InlineStr<262144>;
}

/// The field `region_id` of `Envelope`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct EnvelopeRegionIdField;

impl ::sce_forge_runtime::codec::HoldsStr<EnvelopeRegionIdField>
    for ::sce_forge_runtime::codec::Inline
{
    type Str = ::sce_forge_runtime::codec::InlineStr<262144>;
}

/// The field `child_session_id` of `Envelope`, which a storage profile
/// answers for by this type.
#[allow(dead_code)]
pub struct EnvelopeChildSessionIdField;

impl ::sce_forge_runtime::codec::HoldsStr<EnvelopeChildSessionIdField>
    for ::sce_forge_runtime::codec::Inline
{
    type Str = ::sce_forge_runtime::codec::InlineStr<262144>;
}

/// What `EnvelopeIn` and the codecs it holds ask of a storage profile.
#[allow(dead_code)]
pub trait EnvelopeStorage:
    ::core::fmt::Debug
    + Clone
    + PartialEq
    + ::sce_forge_runtime::codec::HoldsBytes<EnvelopeIdField>
    + ::sce_forge_runtime::codec::HoldsStr<EnvelopeSourceField>
    + ::sce_forge_runtime::codec::HoldsStr<EnvelopeEventTypeField>
    + ::sce_forge_runtime::codec::HoldsBytes<EnvelopeDataField>
    + ::sce_forge_runtime::codec::HoldsStr<EnvelopeSubjectField>
    + ::sce_forge_runtime::codec::HoldsBytes<EnvelopeCorrelationIdField>
    + ::sce_forge_runtime::codec::HoldsStr<EnvelopeReplyToField>
    + ::sce_forge_runtime::codec::HoldsBytes<EnvelopeInvokeIdField>
    + ::sce_forge_runtime::codec::HoldsStr<EnvelopeRpcErrorMessageField>
    + ::sce_forge_runtime::codec::HoldsBytes<EnvelopeRoutingIdField>
    + ::sce_forge_runtime::codec::HoldsStr<EnvelopeParallelIdField>
    + ::sce_forge_runtime::codec::HoldsStr<EnvelopeRegionIdField>
    + ::sce_forge_runtime::codec::HoldsStr<EnvelopeChildSessionIdField>
{
}

impl<
        S: ::core::fmt::Debug
            + Clone
            + PartialEq
            + ::sce_forge_runtime::codec::HoldsBytes<EnvelopeIdField>
            + ::sce_forge_runtime::codec::HoldsStr<EnvelopeSourceField>
            + ::sce_forge_runtime::codec::HoldsStr<EnvelopeEventTypeField>
            + ::sce_forge_runtime::codec::HoldsBytes<EnvelopeDataField>
            + ::sce_forge_runtime::codec::HoldsStr<EnvelopeSubjectField>
            + ::sce_forge_runtime::codec::HoldsBytes<EnvelopeCorrelationIdField>
            + ::sce_forge_runtime::codec::HoldsStr<EnvelopeReplyToField>
            + ::sce_forge_runtime::codec::HoldsBytes<EnvelopeInvokeIdField>
            + ::sce_forge_runtime::codec::HoldsStr<EnvelopeRpcErrorMessageField>
            + ::sce_forge_runtime::codec::HoldsBytes<EnvelopeRoutingIdField>
            + ::sce_forge_runtime::codec::HoldsStr<EnvelopeParallelIdField>
            + ::sce_forge_runtime::codec::HoldsStr<EnvelopeRegionIdField>
            + ::sce_forge_runtime::codec::HoldsStr<EnvelopeChildSessionIdField>,
    > EnvelopeStorage for S
{
}

/// What `EnvelopeIn<Lent<'a, D>>` asks of the profile `D`: how it holds
/// the lists of `Envelope` and of the codecs it holds, since the lent
/// form borrows every byte string and text from the frame (docs/adr/0077 §7).
#[allow(dead_code)]
pub trait EnvelopeLentStorage: ::core::fmt::Debug + Clone + PartialEq {}

impl<D: ::core::fmt::Debug + Clone + PartialEq> EnvelopeLentStorage for D {}

/// What a decode of a frame of the lifetime `'f` asks of a storage profile
/// beside EnvelopeStorage: how it stores each field of that frame.
#[allow(dead_code)]
pub trait EnvelopeFrom<'f>:
    EnvelopeStorage
    + ::sce_forge_runtime::codec::StoresBytes<'f, EnvelopeIdField>
    + ::sce_forge_runtime::codec::StoresStr<'f, EnvelopeSourceField>
    + ::sce_forge_runtime::codec::StoresStr<'f, EnvelopeEventTypeField>
    + ::sce_forge_runtime::codec::StoresBytes<'f, EnvelopeDataField>
    + ::sce_forge_runtime::codec::StoresStr<'f, EnvelopeSubjectField>
    + ::sce_forge_runtime::codec::StoresBytes<'f, EnvelopeCorrelationIdField>
    + ::sce_forge_runtime::codec::StoresStr<'f, EnvelopeReplyToField>
    + ::sce_forge_runtime::codec::StoresBytes<'f, EnvelopeInvokeIdField>
    + ::sce_forge_runtime::codec::StoresStr<'f, EnvelopeRpcErrorMessageField>
    + ::sce_forge_runtime::codec::StoresBytes<'f, EnvelopeRoutingIdField>
    + ::sce_forge_runtime::codec::StoresStr<'f, EnvelopeParallelIdField>
    + ::sce_forge_runtime::codec::StoresStr<'f, EnvelopeRegionIdField>
    + ::sce_forge_runtime::codec::StoresStr<'f, EnvelopeChildSessionIdField>
{
}

impl<
        'f,
        S: EnvelopeStorage
            + ::sce_forge_runtime::codec::StoresBytes<'f, EnvelopeIdField>
            + ::sce_forge_runtime::codec::StoresStr<'f, EnvelopeSourceField>
            + ::sce_forge_runtime::codec::StoresStr<'f, EnvelopeEventTypeField>
            + ::sce_forge_runtime::codec::StoresBytes<'f, EnvelopeDataField>
            + ::sce_forge_runtime::codec::StoresStr<'f, EnvelopeSubjectField>
            + ::sce_forge_runtime::codec::StoresBytes<'f, EnvelopeCorrelationIdField>
            + ::sce_forge_runtime::codec::StoresStr<'f, EnvelopeReplyToField>
            + ::sce_forge_runtime::codec::StoresBytes<'f, EnvelopeInvokeIdField>
            + ::sce_forge_runtime::codec::StoresStr<'f, EnvelopeRpcErrorMessageField>
            + ::sce_forge_runtime::codec::StoresBytes<'f, EnvelopeRoutingIdField>
            + ::sce_forge_runtime::codec::StoresStr<'f, EnvelopeParallelIdField>
            + ::sce_forge_runtime::codec::StoresStr<'f, EnvelopeRegionIdField>
            + ::sce_forge_runtime::codec::StoresStr<'f, EnvelopeChildSessionIdField>,
    > EnvelopeFrom<'f> for S
{
}

// Same pub-API policy as the view above: the owned form and its projections are
// cross-crate surface, and which of them a given in-repo fixture calls says
// nothing about their value.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct EnvelopeOwned<S: EnvelopeStorage = ::sce_forge_runtime::codec::DefaultStorage> {
    /// Map key 0, required.
    pub id: ::sce_forge_runtime::codec::Bounded<
        <S as ::sce_forge_runtime::codec::HoldsBytes<EnvelopeIdField>>::Bytes,
        16,
        16,
    >,
    /// Map key 1, required.
    pub source: ::sce_forge_runtime::codec::Bounded<
        <S as ::sce_forge_runtime::codec::HoldsStr<EnvelopeSourceField>>::Str,
        262144,
    >,
    /// Map key 2, required.
    pub event_type: ::sce_forge_runtime::codec::Bounded<
        <S as ::sce_forge_runtime::codec::HoldsStr<EnvelopeEventTypeField>>::Str,
        262144,
    >,
    /// Map key 3, required.
    pub pattern: pattern_kind::PatternKind,
    /// Map key 4, required.
    pub datacontenttype: payload_codec::PayloadCodec,
    /// Map key 5, required.
    pub data: ::sce_forge_runtime::codec::Bounded<
        <S as ::sce_forge_runtime::codec::HoldsBytes<EnvelopeDataField>>::Bytes,
        16777216,
    >,
    /// Map key 6.
    pub subject: Option<
        ::sce_forge_runtime::codec::Bounded<
            <S as ::sce_forge_runtime::codec::HoldsStr<EnvelopeSubjectField>>::Str,
            262144,
        >,
    >,
    /// Map key 7.
    pub correlation_id: Option<
        ::sce_forge_runtime::codec::Bounded<
            <S as ::sce_forge_runtime::codec::HoldsBytes<EnvelopeCorrelationIdField>>::Bytes,
            16,
            16,
        >,
    >,
    /// Map key 8.
    pub reply_to: Option<
        ::sce_forge_runtime::codec::Bounded<
            <S as ::sce_forge_runtime::codec::HoldsStr<EnvelopeReplyToField>>::Str,
            262144,
        >,
    >,
    /// Map key 9.
    pub invoke_id: Option<
        ::sce_forge_runtime::codec::Bounded<
            <S as ::sce_forge_runtime::codec::HoldsBytes<EnvelopeInvokeIdField>>::Bytes,
            16,
            16,
        >,
    >,
    /// Map key 10.
    pub rpc_status: Option<rpc_status::RpcStatus>,
    /// Map key 11.
    pub rpc_error_message: Option<
        ::sce_forge_runtime::codec::Bounded<
            <S as ::sce_forge_runtime::codec::HoldsStr<EnvelopeRpcErrorMessageField>>::Str,
            262144,
        >,
    >,
    /// Map key 12.
    pub deadline_unix_ms: Option<u64>,
    /// Map key 14.
    pub sequence_no: Option<u64>,
    /// Map key 15.
    pub routing_id: Option<
        ::sce_forge_runtime::codec::Bounded<
            <S as ::sce_forge_runtime::codec::HoldsBytes<EnvelopeRoutingIdField>>::Bytes,
            16,
            16,
        >,
    >,
    /// Map key 16.
    pub parallel_id: Option<
        ::sce_forge_runtime::codec::Bounded<
            <S as ::sce_forge_runtime::codec::HoldsStr<EnvelopeParallelIdField>>::Str,
            262144,
        >,
    >,
    /// Map key 17.
    pub region_id: Option<
        ::sce_forge_runtime::codec::Bounded<
            <S as ::sce_forge_runtime::codec::HoldsStr<EnvelopeRegionIdField>>::Str,
            262144,
        >,
    >,
    /// Map key 18.
    pub child_session_id: Option<
        ::sce_forge_runtime::codec::Bounded<
            <S as ::sce_forge_runtime::codec::HoldsStr<EnvelopeChildSessionIdField>>::Str,
            262144,
        >,
    >,
    /// Map key 19.
    pub causal_epoch: Option<u64>,
    /// Map key 20.
    pub causal_vector:
        Option<::sce_forge_runtime::codec::BoundedList<causal_entry::CausalEntryPayload, 64>>,
}

#[allow(dead_code)]
impl<'a> Envelope<'a> {
    /// Deep-copy this view into an owned, lifetime-free [`EnvelopeOwned`]
    /// held in the given storage profile. Call at a decode boundary when the
    /// decoded value must outlive the input buffer.
    ///
    /// Fallible for profile uniformity. Every string and byte string of this
    /// view is within its bounds, which its type holds (docs/adr/0077 §3), and
    /// the inline and growable profiles hold each at its bound, so the copy
    /// does not fail on them; a profile whose capacity is below a bound raises
    /// `CodecError::CapacityExceeded`.
    pub fn try_into_owned_in<S: EnvelopeFrom<'a>>(self) -> Result<EnvelopeOwned<S>, CodecError> {
        Ok(EnvelopeOwned {
            id: self.id.store::<S, EnvelopeIdField>()?,
            source: self.source.store::<S, EnvelopeSourceField>()?,
            event_type: self.event_type.store::<S, EnvelopeEventTypeField>()?,
            pattern: self.pattern,
            datacontenttype: self.datacontenttype,
            data: self.data.store::<S, EnvelopeDataField>()?,
            subject: self
                .subject
                .map(|held| held.store::<S, EnvelopeSubjectField>())
                .transpose()?,
            correlation_id: self
                .correlation_id
                .map(|held| held.store::<S, EnvelopeCorrelationIdField>())
                .transpose()?,
            reply_to: self
                .reply_to
                .map(|held| held.store::<S, EnvelopeReplyToField>())
                .transpose()?,
            invoke_id: self
                .invoke_id
                .map(|held| held.store::<S, EnvelopeInvokeIdField>())
                .transpose()?,
            rpc_status: self.rpc_status,
            rpc_error_message: self
                .rpc_error_message
                .map(|held| held.store::<S, EnvelopeRpcErrorMessageField>())
                .transpose()?,
            deadline_unix_ms: self.deadline_unix_ms,
            sequence_no: self.sequence_no,
            routing_id: self
                .routing_id
                .map(|held| held.store::<S, EnvelopeRoutingIdField>())
                .transpose()?,
            parallel_id: self
                .parallel_id
                .map(|held| held.store::<S, EnvelopeParallelIdField>())
                .transpose()?,
            region_id: self
                .region_id
                .map(|held| held.store::<S, EnvelopeRegionIdField>())
                .transpose()?,
            child_session_id: self
                .child_session_id
                .map(|held| held.store::<S, EnvelopeChildSessionIdField>())
                .transpose()?,
            causal_epoch: self.causal_epoch,
            causal_vector: self.causal_vector,
        })
    }

    /// The same projection at the build's default storage profile: growable
    /// where an allocator exists, inline on the heap-free tier.
    pub fn try_into_owned(self) -> Result<EnvelopeOwned, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: EnvelopeStorage> EnvelopeOwned<S> {
    /// Re-borrow this owned value back into the view: the inverse of
    /// `try_into_owned_in`. Each entry is read through
    /// [`EnvelopeSource`], so a string or a byte string is a re-borrow of
    /// the container that holds it and not a copy.
    pub fn as_borrowed(&self) -> Envelope<'_> {
        Envelope {
            id: <Self as EnvelopeSource>::id(self),
            source: <Self as EnvelopeSource>::source(self),
            event_type: <Self as EnvelopeSource>::event_type(self),
            pattern: <Self as EnvelopeSource>::pattern(self),
            datacontenttype: <Self as EnvelopeSource>::datacontenttype(self),
            data: <Self as EnvelopeSource>::data(self),
            subject: <Self as EnvelopeSource>::subject(self),
            correlation_id: <Self as EnvelopeSource>::correlation_id(self),
            reply_to: <Self as EnvelopeSource>::reply_to(self),
            invoke_id: <Self as EnvelopeSource>::invoke_id(self),
            rpc_status: <Self as EnvelopeSource>::rpc_status(self),
            rpc_error_message: <Self as EnvelopeSource>::rpc_error_message(self),
            deadline_unix_ms: <Self as EnvelopeSource>::deadline_unix_ms(self),
            sequence_no: <Self as EnvelopeSource>::sequence_no(self),
            routing_id: <Self as EnvelopeSource>::routing_id(self),
            parallel_id: <Self as EnvelopeSource>::parallel_id(self),
            region_id: <Self as EnvelopeSource>::region_id(self),
            child_session_id: <Self as EnvelopeSource>::child_session_id(self),
            causal_epoch: <Self as EnvelopeSource>::causal_epoch(self),
            // A list of records is held by value in both forms, so the view
            // takes a copy of it.
            causal_vector: self.causal_vector.clone(),
        }
    }

    /// Move this value to a different storage profile: growable to inline when
    /// handing it to a path that must not allocate, or inline to growable when it
    /// is leaving that path.
    ///
    /// A checked projection through the view, not a re-decode: the bytes are
    /// copied once and every destination capacity is enforced, so a value that
    /// cannot fit the target profile is rejected here rather than truncated.
    pub fn transcode_in<D: for<'x> EnvelopeFrom<'x>>(
        &self,
    ) -> Result<EnvelopeOwned<D>, CodecError> {
        self.as_borrowed().try_into_owned_in::<D>()
    }

    // Every string and byte string of an owned form is held within its bounds
    // (docs/adr/0077 §3), so the owned encode refuses what the view's does and
    // no more.

    /// Encode `self` into the caller-owned sink, straight from the owned fields:
    /// the bytes [`Envelope::encode`] writes for the value
    /// [`Self::as_borrowed`] gives, without the projection. The only error is
    /// the sink's own (docs/adr/0065).
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), K::Error> {
        Envelope::encode_source(self, w)
    }

    /// [`encode`](Self::encode) into a new `Vec`, which refuses no value.
    #[cfg(feature = "alloc")]
    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        let Ok(()) = self.encode(&mut VecSink::new(&mut out));
        out
    }
}

// ── Source (ADR 0058) ────────────────────────────────────────────────
/// What a value of `Envelope` says: its entries. The count the map head holds
/// is not among them; `Envelope` counts the entries present
/// ([`Envelope::encode_source`]).
pub trait EnvelopeSource {
    /// The entry with key 0, required.
    fn id(&self) -> ::sce_forge_runtime::codec::Bytes<'_, 16, 16>;
    /// The entry with key 1, required.
    fn source(&self) -> ::sce_forge_runtime::codec::Text<'_, 262144>;
    /// The entry with key 2, required.
    fn event_type(&self) -> ::sce_forge_runtime::codec::Text<'_, 262144>;
    /// The entry with key 3, required.
    fn pattern(&self) -> pattern_kind::PatternKind;
    /// The entry with key 4, required.
    fn datacontenttype(&self) -> payload_codec::PayloadCodec;
    /// The entry with key 5, required.
    fn data(&self) -> ::sce_forge_runtime::codec::Bytes<'_, 16777216>;
    /// The entry with key 6.
    fn subject(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 262144>>;
    /// The entry with key 7.
    fn correlation_id(&self) -> Option<::sce_forge_runtime::codec::Bytes<'_, 16, 16>>;
    /// The entry with key 8.
    fn reply_to(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 262144>>;
    /// The entry with key 9.
    fn invoke_id(&self) -> Option<::sce_forge_runtime::codec::Bytes<'_, 16, 16>>;
    /// The entry with key 10.
    fn rpc_status(&self) -> Option<rpc_status::RpcStatus>;
    /// The entry with key 11.
    fn rpc_error_message(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 262144>>;
    /// The entry with key 12.
    fn deadline_unix_ms(&self) -> Option<u64>;
    /// The entry with key 14.
    fn sequence_no(&self) -> Option<u64>;
    /// The entry with key 15.
    fn routing_id(&self) -> Option<::sce_forge_runtime::codec::Bytes<'_, 16, 16>>;
    /// The entry with key 16.
    fn parallel_id(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 262144>>;
    /// The entry with key 17.
    fn region_id(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 262144>>;
    /// The entry with key 18.
    fn child_session_id(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 262144>>;
    /// The entry with key 19.
    fn causal_epoch(&self) -> Option<u64>;
    /// The entry with key 20.
    fn causal_vector(
        &self,
    ) -> Option<&::sce_forge_runtime::codec::BoundedList<causal_entry::CausalEntryPayload, 64>>;
}

impl<T: EnvelopeSource + ?Sized> EnvelopeSource for &T {
    fn id(&self) -> ::sce_forge_runtime::codec::Bytes<'_, 16, 16> {
        (**self).id()
    }
    fn source(&self) -> ::sce_forge_runtime::codec::Text<'_, 262144> {
        (**self).source()
    }
    fn event_type(&self) -> ::sce_forge_runtime::codec::Text<'_, 262144> {
        (**self).event_type()
    }
    fn pattern(&self) -> pattern_kind::PatternKind {
        (**self).pattern()
    }
    fn datacontenttype(&self) -> payload_codec::PayloadCodec {
        (**self).datacontenttype()
    }
    fn data(&self) -> ::sce_forge_runtime::codec::Bytes<'_, 16777216> {
        (**self).data()
    }
    fn subject(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 262144>> {
        (**self).subject()
    }
    fn correlation_id(&self) -> Option<::sce_forge_runtime::codec::Bytes<'_, 16, 16>> {
        (**self).correlation_id()
    }
    fn reply_to(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 262144>> {
        (**self).reply_to()
    }
    fn invoke_id(&self) -> Option<::sce_forge_runtime::codec::Bytes<'_, 16, 16>> {
        (**self).invoke_id()
    }
    fn rpc_status(&self) -> Option<rpc_status::RpcStatus> {
        (**self).rpc_status()
    }
    fn rpc_error_message(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 262144>> {
        (**self).rpc_error_message()
    }
    fn deadline_unix_ms(&self) -> Option<u64> {
        (**self).deadline_unix_ms()
    }
    fn sequence_no(&self) -> Option<u64> {
        (**self).sequence_no()
    }
    fn routing_id(&self) -> Option<::sce_forge_runtime::codec::Bytes<'_, 16, 16>> {
        (**self).routing_id()
    }
    fn parallel_id(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 262144>> {
        (**self).parallel_id()
    }
    fn region_id(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 262144>> {
        (**self).region_id()
    }
    fn child_session_id(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 262144>> {
        (**self).child_session_id()
    }
    fn causal_epoch(&self) -> Option<u64> {
        (**self).causal_epoch()
    }
    fn causal_vector(
        &self,
    ) -> Option<&::sce_forge_runtime::codec::BoundedList<causal_entry::CausalEntryPayload, 64>>
    {
        (**self).causal_vector()
    }
}

impl<'a> EnvelopeSource for Envelope<'a> {
    fn id(&self) -> ::sce_forge_runtime::codec::Bytes<'_, 16, 16> {
        self.id
    }
    fn source(&self) -> ::sce_forge_runtime::codec::Text<'_, 262144> {
        self.source
    }
    fn event_type(&self) -> ::sce_forge_runtime::codec::Text<'_, 262144> {
        self.event_type
    }
    fn pattern(&self) -> pattern_kind::PatternKind {
        self.pattern
    }
    fn datacontenttype(&self) -> payload_codec::PayloadCodec {
        self.datacontenttype
    }
    fn data(&self) -> ::sce_forge_runtime::codec::Bytes<'_, 16777216> {
        self.data
    }
    fn subject(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 262144>> {
        self.subject
    }
    fn correlation_id(&self) -> Option<::sce_forge_runtime::codec::Bytes<'_, 16, 16>> {
        self.correlation_id
    }
    fn reply_to(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 262144>> {
        self.reply_to
    }
    fn invoke_id(&self) -> Option<::sce_forge_runtime::codec::Bytes<'_, 16, 16>> {
        self.invoke_id
    }
    fn rpc_status(&self) -> Option<rpc_status::RpcStatus> {
        self.rpc_status
    }
    fn rpc_error_message(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 262144>> {
        self.rpc_error_message
    }
    fn deadline_unix_ms(&self) -> Option<u64> {
        self.deadline_unix_ms
    }
    fn sequence_no(&self) -> Option<u64> {
        self.sequence_no
    }
    fn routing_id(&self) -> Option<::sce_forge_runtime::codec::Bytes<'_, 16, 16>> {
        self.routing_id
    }
    fn parallel_id(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 262144>> {
        self.parallel_id
    }
    fn region_id(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 262144>> {
        self.region_id
    }
    fn child_session_id(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 262144>> {
        self.child_session_id
    }
    fn causal_epoch(&self) -> Option<u64> {
        self.causal_epoch
    }
    fn causal_vector(
        &self,
    ) -> Option<&::sce_forge_runtime::codec::BoundedList<causal_entry::CausalEntryPayload, 64>>
    {
        self.causal_vector.as_ref()
    }
}

impl<S: EnvelopeStorage> EnvelopeSource for EnvelopeOwned<S> {
    fn id(&self) -> ::sce_forge_runtime::codec::Bytes<'_, 16, 16> {
        ::sce_forge_runtime::codec::Bounded::as_bytes(&self.id)
    }
    fn source(&self) -> ::sce_forge_runtime::codec::Text<'_, 262144> {
        ::sce_forge_runtime::codec::Bounded::as_text(&self.source)
    }
    fn event_type(&self) -> ::sce_forge_runtime::codec::Text<'_, 262144> {
        ::sce_forge_runtime::codec::Bounded::as_text(&self.event_type)
    }
    fn pattern(&self) -> pattern_kind::PatternKind {
        self.pattern
    }
    fn datacontenttype(&self) -> payload_codec::PayloadCodec {
        self.datacontenttype
    }
    fn data(&self) -> ::sce_forge_runtime::codec::Bytes<'_, 16777216> {
        ::sce_forge_runtime::codec::Bounded::as_bytes(&self.data)
    }
    fn subject(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 262144>> {
        self.subject
            .as_ref()
            .map(::sce_forge_runtime::codec::Bounded::as_text)
    }
    fn correlation_id(&self) -> Option<::sce_forge_runtime::codec::Bytes<'_, 16, 16>> {
        self.correlation_id
            .as_ref()
            .map(::sce_forge_runtime::codec::Bounded::as_bytes)
    }
    fn reply_to(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 262144>> {
        self.reply_to
            .as_ref()
            .map(::sce_forge_runtime::codec::Bounded::as_text)
    }
    fn invoke_id(&self) -> Option<::sce_forge_runtime::codec::Bytes<'_, 16, 16>> {
        self.invoke_id
            .as_ref()
            .map(::sce_forge_runtime::codec::Bounded::as_bytes)
    }
    fn rpc_status(&self) -> Option<rpc_status::RpcStatus> {
        self.rpc_status
    }
    fn rpc_error_message(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 262144>> {
        self.rpc_error_message
            .as_ref()
            .map(::sce_forge_runtime::codec::Bounded::as_text)
    }
    fn deadline_unix_ms(&self) -> Option<u64> {
        self.deadline_unix_ms
    }
    fn sequence_no(&self) -> Option<u64> {
        self.sequence_no
    }
    fn routing_id(&self) -> Option<::sce_forge_runtime::codec::Bytes<'_, 16, 16>> {
        self.routing_id
            .as_ref()
            .map(::sce_forge_runtime::codec::Bounded::as_bytes)
    }
    fn parallel_id(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 262144>> {
        self.parallel_id
            .as_ref()
            .map(::sce_forge_runtime::codec::Bounded::as_text)
    }
    fn region_id(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 262144>> {
        self.region_id
            .as_ref()
            .map(::sce_forge_runtime::codec::Bounded::as_text)
    }
    fn child_session_id(&self) -> Option<::sce_forge_runtime::codec::Text<'_, 262144>> {
        self.child_session_id
            .as_ref()
            .map(::sce_forge_runtime::codec::Bounded::as_text)
    }
    fn causal_epoch(&self) -> Option<u64> {
        self.causal_epoch
    }
    fn causal_vector(
        &self,
    ) -> Option<&::sce_forge_runtime::codec::BoundedList<causal_entry::CausalEntryPayload, 64>>
    {
        self.causal_vector.as_ref()
    }
}

#[allow(dead_code)]
impl<'a> Envelope<'a> {
    /// Encode what `src` says as one CBOR map into `w`: the entries present in
    /// ascending key order, every head in its shortest form. An entry past its
    /// declared bound is not one a value of it can break: the only error is
    /// the sink's (docs/adr/0065).
    pub fn encode_source<S: EnvelopeSource + ?Sized, K: SceSink>(
        src: &S,
        w: &mut K,
    ) -> Result<(), K::Error> {
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the pre-pass a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: EnvelopeSource + ?Sized, K: SceSink>(
        src: &S,
        w: &mut K,
    ) -> Result<(), K::Error> {
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
        if src.causal_epoch().is_some() {
            count += 1;
        }
        if src.causal_vector().is_some() {
            count += 1;
        }
        cbor::write_map_head(w, count)?;
        let v = src.id();
        cbor::write_uint(w, 0)?;
        cbor::write_bytes(w, v.into_inner())?;
        let v = src.source();
        cbor::write_uint(w, 1)?;
        cbor::write_text(w, v.into_inner())?;
        let v = src.event_type();
        cbor::write_uint(w, 2)?;
        cbor::write_text(w, v.into_inner())?;
        let v = src.pattern();
        cbor::write_uint(w, 3)?;
        cbor::write_uint(w, u64::from(v.to_underlying()))?;
        let v = src.datacontenttype();
        cbor::write_uint(w, 4)?;
        cbor::write_uint(w, u64::from(v.to_underlying()))?;
        let v = src.data();
        cbor::write_uint(w, 5)?;
        cbor::write_bytes(w, v.into_inner())?;
        if let Some(v) = src.subject() {
            cbor::write_uint(w, 6)?;
            cbor::write_text(w, v.into_inner())?;
        }
        if let Some(v) = src.correlation_id() {
            cbor::write_uint(w, 7)?;
            cbor::write_bytes(w, v.into_inner())?;
        }
        if let Some(v) = src.reply_to() {
            cbor::write_uint(w, 8)?;
            cbor::write_text(w, v.into_inner())?;
        }
        if let Some(v) = src.invoke_id() {
            cbor::write_uint(w, 9)?;
            cbor::write_bytes(w, v.into_inner())?;
        }
        if let Some(v) = src.rpc_status() {
            cbor::write_uint(w, 10)?;
            cbor::write_uint(w, u64::from(v.to_underlying()))?;
        }
        if let Some(v) = src.rpc_error_message() {
            cbor::write_uint(w, 11)?;
            cbor::write_text(w, v.into_inner())?;
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
            cbor::write_bytes(w, v.into_inner())?;
        }
        if let Some(v) = src.parallel_id() {
            cbor::write_uint(w, 16)?;
            cbor::write_text(w, v.into_inner())?;
        }
        if let Some(v) = src.region_id() {
            cbor::write_uint(w, 17)?;
            cbor::write_text(w, v.into_inner())?;
        }
        if let Some(v) = src.child_session_id() {
            cbor::write_uint(w, 18)?;
            cbor::write_text(w, v.into_inner())?;
        }
        if let Some(v) = src.causal_epoch() {
            cbor::write_uint(w, 19)?;
            cbor::write_uint(w, v)?;
        }
        if let Some(v) = src.causal_vector() {
            cbor::write_uint(w, 20)?;
            let len = v.as_slice().len();
            cbor::write_array_head(w, ::sce_forge_runtime::codec::widen_usize(len))?;
            for record in v.as_slice() {
                encode_record_causal_entry(w, record)?;
            }
        }
        Ok(())
    }

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: EnvelopeSource + ?Sized>(src: &S) -> usize {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        let Ok(()) = Self::write_source(src, &mut _c);
        _c.len()
    }
}

impl<'a, S: EnvelopeSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for Envelope<'a> {
    type Error<E> = E;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), K::Error>
    where
        CodecError: From<K::Error>,
    {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl<'a> Envelope<'a> {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written: the encode refuses no value, so neither does
    /// this. The result writes the frame into a window of exactly that size
    /// and cannot fail.
    pub fn measure<S: EnvelopeSource + ?Sized>(
        src: &S,
    ) -> ::sce_forge_runtime::codec::Measured<'_, Self, S> {
        let Ok(measured) = ::sce_forge_runtime::codec::measure(src);
        measured
    }

    /// Write `src` into `window` in one pass and return the bytes it is, for a
    /// window as large as the caller can spare. On an error the window holds a
    /// prefix to discard; see [`::sce_forge_runtime::codec::write_window`].
    pub fn write_window<S: EnvelopeSource + ?Sized>(
        src: &S,
        window: &mut [u8],
    ) -> Result<usize, CodecError> {
        let mut sink = ::sce_forge_runtime::codec::SliceSink::new(window);
        Self::write_source(src, &mut sink)?;
        Ok(sink.position())
    }
}
