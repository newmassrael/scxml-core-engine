// SCE-GENERATED — DO NOT EDIT
// source-hash: f3593d0da7c47c6e95770dd740ec6b7f10d9172b65629c75948b0c0e547691b7
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
    pub fn encode<S: SceSink>(&self, w: &mut S) -> Result<(), CodecError>
    where
        CodecError: From<S::Error>,
    {
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
// `try_into_owned_in` enforces each declared bound on the inline profile
// (`CodecError::TooManyElements`); `as_borrowed` re-borrows any profile back into
// the view; `transcode_in` moves a value between profiles as a checked
// projection; and `encode` writes the owned value through the source interface,
// without the view.
//
// Decoding picks the profile from the call (`try_into_owned_in::<Inline>()`).
// Hand-assembling a value names the profile on the value or its binding, because
// the fields reach it through its associated container types.
// Same pub-API policy as the view above: the owned form and its projections are
// cross-crate surface, and which of them a given in-repo fixture calls says
// nothing about their value.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct EnvelopeOwned<
    S: ::sce_forge_runtime::codec::CodecStorage = ::sce_forge_runtime::codec::DefaultStorage,
> {
    /// Map key 0, required.
    pub id: S::Bytes<16>,
    /// Map key 1, required.
    pub source: S::Str<262144>,
    /// Map key 2, required.
    pub event_type: S::Str<262144>,
    /// Map key 3, required.
    pub pattern: pattern_kind::PatternKind,
    /// Map key 4, required.
    pub datacontenttype: payload_codec::PayloadCodec,
    /// Map key 5, required.
    pub data: S::Bytes<16777216>,
    /// Map key 6.
    pub subject: Option<S::Str<262144>>,
    /// Map key 7.
    pub correlation_id: Option<S::Bytes<16>>,
    /// Map key 8.
    pub reply_to: Option<S::Str<262144>>,
    /// Map key 9.
    pub invoke_id: Option<S::Bytes<16>>,
    /// Map key 10.
    pub rpc_status: Option<rpc_status::RpcStatus>,
    /// Map key 11.
    pub rpc_error_message: Option<S::Str<262144>>,
    /// Map key 12.
    pub deadline_unix_ms: Option<u64>,
    /// Map key 14.
    pub sequence_no: Option<u64>,
    /// Map key 15.
    pub routing_id: Option<S::Bytes<16>>,
    /// Map key 16.
    pub parallel_id: Option<S::Str<262144>>,
    /// Map key 17.
    pub region_id: Option<S::Str<262144>>,
    /// Map key 18.
    pub child_session_id: Option<S::Str<262144>>,
}

#[allow(dead_code)]
impl<'a> Envelope<'a> {
    /// Deep-copy this view into an owned, lifetime-free [`EnvelopeOwned`]
    /// held in the given storage profile. Call at a decode boundary when the
    /// decoded value must outlive the input buffer.
    ///
    /// Fallible for profile uniformity: on the inline profile a string or byte
    /// string longer than its declared capacity raises
    /// `CodecError::TooManyElements`, on the growable profile the copy cannot
    /// fail. Decode has already held each entry to its bound, so a value that
    /// was decoded fits.
    pub fn try_into_owned_in<S: ::sce_forge_runtime::codec::CodecStorage>(
        self,
    ) -> Result<EnvelopeOwned<S>, CodecError> {
        Ok(EnvelopeOwned {
            id: <S::Bytes<16> as ::sce_forge_runtime::codec::SceByteBuf>::from_slice(self.id)?,
            source: <S::Str<262144> as ::sce_forge_runtime::codec::SceStr>::from_view(self.source)?,
            event_type: <S::Str<262144> as ::sce_forge_runtime::codec::SceStr>::from_view(
                self.event_type,
            )?,
            pattern: self.pattern,
            datacontenttype: self.datacontenttype,
            data: <S::Bytes<16777216> as ::sce_forge_runtime::codec::SceByteBuf>::from_slice(
                self.data,
            )?,
            subject: self
                .subject
                .map(<S::Str<262144> as ::sce_forge_runtime::codec::SceStr>::from_view)
                .transpose()?,
            correlation_id: self
                .correlation_id
                .map(<S::Bytes<16> as ::sce_forge_runtime::codec::SceByteBuf>::from_slice)
                .transpose()?,
            reply_to: self
                .reply_to
                .map(<S::Str<262144> as ::sce_forge_runtime::codec::SceStr>::from_view)
                .transpose()?,
            invoke_id: self
                .invoke_id
                .map(<S::Bytes<16> as ::sce_forge_runtime::codec::SceByteBuf>::from_slice)
                .transpose()?,
            rpc_status: self.rpc_status,
            rpc_error_message: self
                .rpc_error_message
                .map(<S::Str<262144> as ::sce_forge_runtime::codec::SceStr>::from_view)
                .transpose()?,
            deadline_unix_ms: self.deadline_unix_ms,
            sequence_no: self.sequence_no,
            routing_id: self
                .routing_id
                .map(<S::Bytes<16> as ::sce_forge_runtime::codec::SceByteBuf>::from_slice)
                .transpose()?,
            parallel_id: self
                .parallel_id
                .map(<S::Str<262144> as ::sce_forge_runtime::codec::SceStr>::from_view)
                .transpose()?,
            region_id: self
                .region_id
                .map(<S::Str<262144> as ::sce_forge_runtime::codec::SceStr>::from_view)
                .transpose()?,
            child_session_id: self
                .child_session_id
                .map(<S::Str<262144> as ::sce_forge_runtime::codec::SceStr>::from_view)
                .transpose()?,
        })
    }

    /// The same projection at the build's default storage profile: growable
    /// where an allocator exists, inline on the heap-free tier.
    pub fn try_into_owned(self) -> Result<EnvelopeOwned, CodecError> {
        self.try_into_owned_in()
    }
}

#[allow(dead_code)]
impl<S: ::sce_forge_runtime::codec::CodecStorage> EnvelopeOwned<S> {
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
        }
    }

    /// Move this value to a different storage profile: growable to inline when
    /// handing it to a path that must not allocate, or inline to growable when it
    /// is leaving that path.
    ///
    /// A checked projection through the view, not a re-decode: the bytes are
    /// copied once and every destination capacity is enforced, so a value that
    /// cannot fit the target profile is rejected here rather than truncated.
    pub fn transcode_in<D: ::sce_forge_runtime::codec::CodecStorage>(
        &self,
    ) -> Result<EnvelopeOwned<D>, CodecError> {
        self.as_borrowed().try_into_owned_in::<D>()
    }

    // Every string and byte string of an owned form has a declared bound, so the
    // encode can refuse a value (`CodecError::CborOutOfRange`, or
    // `CodecError::CborWrongLength` for an exact byte string): the growable
    // profile holds a value past a bound, which the encode then refuses.

    /// Encode `self` into the caller-owned sink, straight from the owned fields:
    /// the bytes [`Envelope::encode`] writes for the value
    /// [`Self::as_borrowed`] gives, without the projection. An entry past its
    /// declared bound is refused, not written.
    pub fn encode<K: SceSink>(&self, w: &mut K) -> Result<(), CodecError>
    where
        CodecError: From<K::Error>,
    {
        Envelope::encode_source(self, w)
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

impl<S: ::sce_forge_runtime::codec::CodecStorage> EnvelopeSource for EnvelopeOwned<S> {
    fn id(&self) -> &[u8] {
        <S::Bytes<16> as ::sce_forge_runtime::codec::SceByteBuf>::as_slice(&self.id)
    }
    fn source(&self) -> &str {
        <S::Str<262144> as ::sce_forge_runtime::codec::SceStr>::as_str(&self.source)
    }
    fn event_type(&self) -> &str {
        <S::Str<262144> as ::sce_forge_runtime::codec::SceStr>::as_str(&self.event_type)
    }
    fn pattern(&self) -> pattern_kind::PatternKind {
        self.pattern
    }
    fn datacontenttype(&self) -> payload_codec::PayloadCodec {
        self.datacontenttype
    }
    fn data(&self) -> &[u8] {
        <S::Bytes<16777216> as ::sce_forge_runtime::codec::SceByteBuf>::as_slice(&self.data)
    }
    fn subject(&self) -> Option<&str> {
        self.subject
            .as_ref()
            .map(<S::Str<262144> as ::sce_forge_runtime::codec::SceStr>::as_str)
    }
    fn correlation_id(&self) -> Option<&[u8]> {
        self.correlation_id
            .as_ref()
            .map(<S::Bytes<16> as ::sce_forge_runtime::codec::SceByteBuf>::as_slice)
    }
    fn reply_to(&self) -> Option<&str> {
        self.reply_to
            .as_ref()
            .map(<S::Str<262144> as ::sce_forge_runtime::codec::SceStr>::as_str)
    }
    fn invoke_id(&self) -> Option<&[u8]> {
        self.invoke_id
            .as_ref()
            .map(<S::Bytes<16> as ::sce_forge_runtime::codec::SceByteBuf>::as_slice)
    }
    fn rpc_status(&self) -> Option<rpc_status::RpcStatus> {
        self.rpc_status
    }
    fn rpc_error_message(&self) -> Option<&str> {
        self.rpc_error_message
            .as_ref()
            .map(<S::Str<262144> as ::sce_forge_runtime::codec::SceStr>::as_str)
    }
    fn deadline_unix_ms(&self) -> Option<u64> {
        self.deadline_unix_ms
    }
    fn sequence_no(&self) -> Option<u64> {
        self.sequence_no
    }
    fn routing_id(&self) -> Option<&[u8]> {
        self.routing_id
            .as_ref()
            .map(<S::Bytes<16> as ::sce_forge_runtime::codec::SceByteBuf>::as_slice)
    }
    fn parallel_id(&self) -> Option<&str> {
        self.parallel_id
            .as_ref()
            .map(<S::Str<262144> as ::sce_forge_runtime::codec::SceStr>::as_str)
    }
    fn region_id(&self) -> Option<&str> {
        self.region_id
            .as_ref()
            .map(<S::Str<262144> as ::sce_forge_runtime::codec::SceStr>::as_str)
    }
    fn child_session_id(&self) -> Option<&str> {
        self.child_session_id
            .as_ref()
            .map(<S::Str<262144> as ::sce_forge_runtime::codec::SceStr>::as_str)
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
    ) -> Result<(), CodecError>
    where
        CodecError: From<K::Error>,
    {
        Self::count_source(src)?;
        Self::write_source(src, w)
    }

    /// [`Self::encode_source`] without the pre-pass a parent has already made.
    #[doc(hidden)]
    pub fn write_source<S: EnvelopeSource + ?Sized, K: SceSink>(
        src: &S,
        w: &mut K,
    ) -> Result<(), CodecError>
    where
        CodecError: From<K::Error>,
    {
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

    /// The bytes [`Self::write_source`] writes for `src`, counted with
    /// nothing written.
    #[doc(hidden)]
    pub fn count_source<S: EnvelopeSource + ?Sized>(src: &S) -> Result<usize, CodecError> {
        let mut _c = ::sce_forge_runtime::codec::CountSink::new();
        Self::write_source(src, &mut _c)?;
        Ok(_c.len())
    }
}

impl<'a, S: EnvelopeSource + ?Sized> ::sce_forge_runtime::codec::SourceWrite<S> for Envelope<'a> {
    type Error<E> = CodecError;

    fn write<K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError>
    where
        CodecError: From<K::Error>,
    {
        Self::write_source(src, w)
    }
}

#[allow(dead_code)]
impl<'a> Envelope<'a> {
    /// The size of the frame [`Self::encode_source`] writes for `src`, found
    /// with nothing written, or the refusal that encode would give. The result
    /// writes the frame into a window of exactly that size and cannot fail.
    pub fn measure<S: EnvelopeSource + ?Sized>(
        src: &S,
    ) -> Result<::sce_forge_runtime::codec::Measured<'_, Self, S>, CodecError> {
        ::sce_forge_runtime::codec::measure(src)
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
