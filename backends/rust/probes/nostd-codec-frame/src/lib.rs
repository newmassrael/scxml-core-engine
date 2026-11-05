// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The MCU frame instrument of a codec's lent decode, and of a reply's encode.
//!
//! Each entry point below calls the lent `decode_into`, or the `encode`, of one
//! committed codec golden, so that code exists in an object for the target and
//! its frame can be read from its prologue (`scripts/codec_frame_probe.sh`,
//! through `tools/mcu_stack`). Three are the shapes a group's elements take
//! where its arm holds them (docs/adr/0065 §6, amendment of 2026-11-05): a
//! chain whose kinds decide the arm (`chain`), a bare entry's chain placed at
//! its first entry of the kind (`placed`), and two lists of one count
//! (`paired`). Two are a reply (`reply`): a response whose variant's arm holds a
//! body that holds a chain and a variant of its own, decoded lent into a value
//! the caller holds and encoded from one by reference, at a deployment's
//! profile, since the protocol states no bound of its chains (docs/adr/0077
//! §7, note of 2026-11-05 on a lent reply decode). The value lives behind the
//! caller's pointer in both, so a frame of either holds only what the code
//! itself keeps.
//!
//! What a frame is held to is computed here, not written in the script: each
//! `SCE_CODEC_FRAME_<SHAPE>_LIST` is a symbol as many bytes long as one inline
//! list of that codec, as its storage profile holds it on this target. A frame
//! that holds a whole list is at least that long, so the script refuses a frame
//! that is not shorter. `nm --print-size` reads the length without running
//! anything. A reply's arm holds a list as long as the response's own, so a
//! frame that holds the arm, or the value, is refused by the same length.
//!
//! The entry points are `#[no_mangle] extern "C"` so that neither the linker
//! nor LTO removes them and their names reach the disassembly unmangled.

#![no_std]
#![allow(dead_code)]

use core::ffi::c_void;
use core::mem::size_of;

use sce_forge_runtime::codec::{CodecError, HoldsList, Lent, SceCursor, SceSink, SliceSink};

// The committed goldens, staged here by `scripts/codec_frame_probe.sh`. Each
// names the codecs it holds as siblings (`use super::<codec>`), so all of them
// are modules of the crate root.
#[path = "codec_chain_has_envelope.rs"]
pub mod codec_chain_has_envelope;
#[path = "codec_chain_has_marker.rs"]
pub mod codec_chain_has_marker;
#[path = "codec_chain_has_marker_slice.rs"]
pub mod codec_chain_has_marker_slice;
#[path = "codec_chain_has_slice.rs"]
pub mod codec_chain_has_slice;
#[path = "codec_repeat_elem.rs"]
pub mod codec_repeat_elem;
#[path = "codec_repeat_unconditional_count.rs"]
pub mod codec_repeat_unconditional_count;
#[path = "codec_zenoh_del.rs"]
pub mod codec_zenoh_del;
#[path = "codec_zenoh_encoding.rs"]
pub mod codec_zenoh_encoding;
#[path = "codec_zenoh_err.rs"]
pub mod codec_zenoh_err;
#[path = "codec_zenoh_ext_entry.rs"]
pub mod codec_zenoh_ext_entry;
#[path = "codec_zenoh_ext_unit.rs"]
pub mod codec_zenoh_ext_unit;
#[path = "codec_zenoh_ext_zbuf.rs"]
pub mod codec_zenoh_ext_zbuf;
#[path = "codec_zenoh_ext_zint.rs"]
pub mod codec_zenoh_ext_zint;
#[path = "codec_zenoh_push_body.rs"]
pub mod codec_zenoh_push_body;
#[path = "codec_zenoh_put.rs"]
pub mod codec_zenoh_put;
#[path = "codec_zenoh_reply.rs"]
pub mod codec_zenoh_reply;
#[path = "codec_zenoh_response.rs"]
pub mod codec_zenoh_response;

use codec_chain_has_envelope::{CodecChainHasEnvelope, CodecChainHasEnvelopeExtensionsField};
use codec_chain_has_marker::{CodecChainHasMarker, CodecChainHasMarkerExtensionsField};
use codec_repeat_elem::CodecRepeatElem;
use codec_repeat_unconditional_count::{
    CodecRepeatUnconditionalCount, CodecRepeatUnconditionalCountLinksField,
};
use codec_zenoh_err::CodecZenohErrExtensionsField;
use codec_zenoh_ext_entry::CodecZenohExtEntryIn;
use codec_zenoh_reply::CodecZenohReplyExtensionsField;
use codec_zenoh_response::{CodecZenohResponseExtensionsField, CodecZenohResponseIn};

/// The profile every entry point decodes at: bytes borrowed from the frame,
/// lists inline at the bound each document states.
type Profile = Lent<'static>;

/// The list of extension entries each chain codec holds, as the profile holds it.
type ChainList = <Profile as HoldsList<CodecChainHasEnvelopeExtensionsField>>::List<
    CodecZenohExtEntryIn<Profile>,
>;
type PlacedList = <Profile as HoldsList<CodecChainHasMarkerExtensionsField>>::List<
    CodecZenohExtEntryIn<Profile>,
>;
/// The first of the two lists of one count, as the profile holds it.
type PairedList =
    <Profile as HoldsList<CodecRepeatUnconditionalCountLinksField>>::List<CodecRepeatElem>;

/// The entries a deployment holds of each extension chain of a reply. The
/// protocol states no depth for them, so `Inline` holds none, and a reply is
/// decoded at a deployment, as a bare-metal client decodes one (docs/adr/0077
/// §7). Sixteen is a capacity such a client may give, and makes a list long
/// enough that a frame holding one stands well apart from one holding none.
const KIT_EXTENSIONS: usize = 16;

/// A deployment of the probe's own, as a deployment's file declares one: the
/// extension chains of a response, a reply and an error, each held inline at
/// [`KIT_EXTENSIONS`] entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kit;

/// The list [`Kit`] holds each extension chain in.
type KitList<T> = sce_forge_runtime::heapless::Vec<T, KIT_EXTENSIONS>;

impl HoldsList<CodecZenohResponseExtensionsField> for Kit {
    type List<T>
        = KitList<T>
    where
        T: Default + core::fmt::Debug + Clone + PartialEq;
}

impl HoldsList<CodecZenohReplyExtensionsField> for Kit {
    type List<T>
        = KitList<T>
    where
        T: Default + core::fmt::Debug + Clone + PartialEq;
}

impl HoldsList<CodecZenohErrExtensionsField> for Kit {
    type List<T>
        = KitList<T>
    where
        T: Default + core::fmt::Debug + Clone + PartialEq;
}

/// The profile a reply is decoded at: bytes borrowed from the frame, lists as
/// [`Kit`] holds them.
type KitProfile = Lent<'static, Kit>;
/// A response, the value a lent reply decode fills and an encode reads.
type Response = CodecZenohResponseIn<KitProfile>;
/// The response's own extension chain, as the deployment holds it.
type ResponseList = <KitProfile as HoldsList<CodecZenohResponseExtensionsField>>::List<
    CodecZenohExtEntryIn<KitProfile>,
>;

/// One inline chain list's length in bytes, as the length of this symbol.
#[no_mangle]
#[used]
pub static SCE_CODEC_FRAME_CHAIN_LIST: [u8; size_of::<ChainList>()] = [0; size_of::<ChainList>()];

/// One inline list of the bare entry's chain, as the length of this symbol.
#[no_mangle]
#[used]
pub static SCE_CODEC_FRAME_PLACED_LIST: [u8; size_of::<PlacedList>()] =
    [0; size_of::<PlacedList>()];

/// One inline list of the two of one count, as the length of this symbol.
#[no_mangle]
#[used]
pub static SCE_CODEC_FRAME_PAIRED_LIST: [u8; size_of::<PairedList>()] =
    [0; size_of::<PairedList>()];

/// One inline extension chain of a reply, as the length of this symbol: the
/// ceiling of the reply's decode and of its encode alike.
#[no_mangle]
#[used]
pub static SCE_CODEC_FRAME_REPLY_LIST: [u8; size_of::<ResponseList>()] =
    [0; size_of::<ResponseList>()];

/// A staticlib for a bare target names what happens on panic. This crate is
/// compiled and disassembled, never run, and `panic = "abort"` keeps the
/// unwinder out of the frames read.
#[panic_handler]
fn on_panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

fn outcome(decoded: Result<(), CodecError>) -> i32 {
    match decoded {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

/// Decode `len` bytes at `bytes` into the `codec_chain_has_envelope` value at
/// `out`, through its lent `decode_into`.
///
/// # Safety
///
/// `bytes` points at `len` readable bytes that outlive every read of the value
/// at `out`, and `out` at a value of `CodecChainHasEnvelope`.
#[no_mangle]
pub unsafe extern "C" fn sce_codec_frame_chain_decode_into(
    bytes: *const u8,
    len: usize,
    out: *mut c_void,
) -> i32 {
    let frame: &'static [u8] = core::slice::from_raw_parts(bytes, len);
    let out = &mut *out.cast::<CodecChainHasEnvelope<'static>>();
    outcome(CodecChainHasEnvelope::decode_into(&mut SceCursor::new(frame), out))
}

/// Decode into a `codec_chain_has_marker` value, as
/// [`sce_codec_frame_chain_decode_into`] does into its codec.
///
/// # Safety
///
/// As [`sce_codec_frame_chain_decode_into`], `out` pointing at a value of
/// `CodecChainHasMarker`.
#[no_mangle]
pub unsafe extern "C" fn sce_codec_frame_placed_decode_into(
    bytes: *const u8,
    len: usize,
    out: *mut c_void,
) -> i32 {
    let frame: &'static [u8] = core::slice::from_raw_parts(bytes, len);
    let out = &mut *out.cast::<CodecChainHasMarker<'static>>();
    outcome(CodecChainHasMarker::decode_into(&mut SceCursor::new(frame), out))
}

/// Decode into a `codec_repeat_unconditional_count` value, as
/// [`sce_codec_frame_chain_decode_into`] does into its codec.
///
/// # Safety
///
/// As [`sce_codec_frame_chain_decode_into`], `out` pointing at a value of
/// `CodecRepeatUnconditionalCount`.
#[no_mangle]
pub unsafe extern "C" fn sce_codec_frame_paired_decode_into(
    bytes: *const u8,
    len: usize,
    out: *mut c_void,
) -> i32 {
    let frame: &'static [u8] = core::slice::from_raw_parts(bytes, len);
    let out = &mut *out.cast::<CodecRepeatUnconditionalCount<'static>>();
    outcome(CodecRepeatUnconditionalCount::decode_into(
        &mut SceCursor::new(frame),
        out,
    ))
}

/// Decode into a `codec_zenoh_response` value at the deployment [`Kit`], as
/// [`sce_codec_frame_chain_decode_into`] does into its codec: a lent reply
/// decode, whose variant installs the arm the frame names and decodes the
/// reply's chain and push body where the arm holds them.
///
/// # Safety
///
/// As [`sce_codec_frame_chain_decode_into`], `out` pointing at a value of
/// `CodecZenohResponseIn<Lent<'static, Kit>>`.
#[no_mangle]
pub unsafe extern "C" fn sce_codec_frame_reply_decode_into(
    bytes: *const u8,
    len: usize,
    out: *mut c_void,
) -> i32 {
    let frame: &'static [u8] = core::slice::from_raw_parts(bytes, len);
    let out = &mut *out.cast::<Response>();
    outcome(Response::decode_into(&mut SceCursor::new(frame), out))
}

/// Encode the `codec_zenoh_response` value at `value` into the `len` bytes at
/// `window`, through its `encode` over a [`SliceSink`]: the write a bare-metal
/// client makes of a value it holds, read by reference. Answers the bytes
/// written, or -1 where the window is too short.
///
/// # Safety
///
/// `value` points at a value of `CodecZenohResponseIn<Lent<'static, Kit>>`
/// whose borrowed bytes are readable, and `window` at `len` writable bytes that
/// alias nothing the value borrows.
#[no_mangle]
pub unsafe extern "C" fn sce_codec_frame_reply_encode(
    value: *const c_void,
    window: *mut u8,
    len: usize,
) -> isize {
    let value = &*value.cast::<Response>();
    let mut sink = SliceSink::new(core::slice::from_raw_parts_mut(window, len));
    match value.encode(&mut sink) {
        Ok(()) => sink.position() as isize,
        Err(_) => -1,
    }
}
