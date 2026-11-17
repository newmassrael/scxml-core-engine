// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Which codecs the Python backend prints from their encode plan
//! (docs/adr/0081 step 4, §6), and what a printed one is.
//!
//! The Python printer lowers the plan's forms one landing at a time. A codec
//! whose every form it lowers is printed: its module names a `Protocol` of the
//! elements a source presents, an encode over any source that derives every
//! other element of the frame (docs/adr/0058 §1), and a value that holds no
//! element the encode derives and holds each byte string in the class of its
//! bound. Every other codec keeps the template's encode whole, and the printer
//! names the first form it does not lower.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{
    compile_forge_file, python_codec_printed_file, python_codec_unlowered_file, ForgeCompileOptions,
};

/// The directory of the resources.
fn resources() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources")
}

/// The Python module the document `path` generates.
fn module_of(path: &Path) -> Result<String, String> {
    let stem = path
        .file_stem()
        .expect("a document has a stem")
        .to_string_lossy()
        .into_owned();
    let output = compile_forge_file(path, Language::Python, &[], &ForgeCompileOptions::default())
        .map_err(|e| format!("{:?}", e.error))?;
    output
        .files
        .into_iter()
        .find(|(file, _)| *file == format!("{stem}.py"))
        .map(|(_, text)| text)
        .ok_or_else(|| format!("{stem} generates no `{stem}.py`"))
}

/// The Python module of the resource `name`, which generates.
fn resource_module(name: &str) -> String {
    module_of(&resources().join(format!("{name}.scxml")))
        .unwrap_or_else(|e| panic!("{name} generates on Python: {e}"))
}

/// The forms of the resource `name` the Python printer does not lower.
fn unlowered(name: &str) -> Vec<String> {
    python_codec_unlowered_file(
        &resources().join(format!("{name}.scxml")),
        &[],
        &ForgeCompileOptions::default(),
    )
    .unwrap_or_else(|e| panic!("{name} reads: {e:?}"))
    .expect("the resource is a codec")
}

/// Whether `text` holds each of `needles`, with `text` in the failure.
fn holds(name: &str, text: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            text.contains(needle),
            "{name}: the module holds `{needle}`:\n{text}"
        );
    }
}

/// Whether `text` holds none of `needles`, with `text` in the failure.
fn holds_none(name: &str, text: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            !text.contains(needle),
            "{name}: the module holds no `{needle}`:\n{text}"
        );
    }
}

/// A length of a byte string is derived on encode from the bytes the source
/// presents, and is no field of the value; the byte string is presented in the
/// class of its bound, which the value's constructor holds it to.
///
/// Turns red where `python_source::encode_source` reads the length off the
/// source rather than measuring the bytes, where the template's dataclass loop
/// stops skipping a derived field, or where `render` names no bound class.
#[test]
fn a_length_is_derived_from_the_bytes_it_measures_and_is_no_field_of_the_value() {
    assert!(unlowered("codec_length_ref").is_empty());
    let text = resource_module("codec_length_ref");
    holds(
        "codec_length_ref",
        &text,
        &[
            "from sce_forge_runtime.bounded import BoundedBytes, BoundedInt\n",
            "from typing import Optional, Protocol\n",
            "class CodecLengthRefMsgIdInt(BoundedInt):\n",
            "    BITS = 8\n    SIGNED = False\n",
            "class CodecLengthRefPayloadBytes(BoundedBytes):\n",
            "    LEAST = 0\n    MOST = 32\n",
            "class CodecLengthRefSource(Protocol):\n",
            "    @property\n    def msg_id(self) -> CodecLengthRefMsgIdInt: ...\n",
            "    @property\n    def payload(self) -> CodecLengthRefPayloadBytes: ...\n",
            "    msg_id: CodecLengthRefMsgIdInt = 0\n",
            "    payload: CodecLengthRefPayloadBytes = CodecLengthRefPayloadBytes()\n",
            "    def __setattr__(self, name: str, value: object) -> None:\n",
            "        if name == \"msg_id\":\n            value = CodecLengthRefMsgIdInt(value)\n",
            "        elif name == \"payload\":\n            value = CodecLengthRefPayloadBytes(value)\n",
            "        object.__setattr__(self, name, value)\n",
            "        encode_source(self, w)\n",
            "\ndef encode_source(src: CodecLengthRefSource, w: SceSink) -> None:\n",
            "    _d_len = len(src.payload)\n",
            "    w.write_u8(src.msg_id & 0xFF)\n",
            "    w.write_u8(_d_len & 0xFF)\n",
            "    w.write_bytes(src.payload)\n",
        ],
    );
    // The value holds no length, is made without one, and the encode reads
    // none off the value; it refuses no bound, which the class holds.
    holds_none(
        "codec_length_ref",
        &text,
        &[
            "\n    len: ",
            "            len=",
            "src.len",
            "self.len",
            "if len(self.payload) > 32:",
        ],
    );
}

/// A length its `sce:length-arith` takes off is derived less that, and the
/// class of the bytes holds at least what it takes off, so a value made by
/// default is within its bound.
///
/// Turns red where `python_source::encode_source` drops the `Offset::Less`
/// arithmetic, or where `render` defaults a byte string of a positive least to
/// no bytes.
#[test]
fn a_length_less_its_arithmetic_is_derived_less_it() {
    assert!(unlowered("codec_scout_zid_body").is_empty());
    let text = resource_module("codec_scout_zid_body");
    holds(
        "codec_scout_zid_body",
        &text,
        &[
            "class CodecScoutZidBodyZidBytes(BoundedBytes):\n",
            "    LEAST = 1\n    MOST = 16\n",
            "    zid: CodecScoutZidBodyZidBytes = CodecScoutZidBodyZidBytes(b\"\\x00\" * 1)\n",
            "    _d_zid_len_m1 = len(src.zid) - 1\n",
            "    w.write_u8(_d_zid_len_m1 & 0xFF)\n",
            "    w.write_bytes(src.zid)\n",
        ],
    );
    holds_none(
        "codec_scout_zid_body",
        &text,
        &["\n    zid_len_m1: ", "src.zid_len_m1"],
    );
}

/// A codec of integers holds each in the class of its field's width and
/// signedness, a subclass of the runtime's `BoundedInt` that the value's
/// `__setattr__` makes wherever the element is set, so a number past the width
/// is refused where it is made and never written as the bits the field holds.
///
/// Turns red where `python_source::held_as` holds an integer as a bare `int`
/// again, where `int_width` stops reading the field's width, or where
/// `render` declares an integer as `int`.
#[test]
fn a_codec_of_integers_holds_each_in_the_class_of_its_width() {
    assert!(unlowered("codec_simple_frame").is_empty());
    let text = resource_module("codec_simple_frame");
    holds(
        "codec_simple_frame",
        &text,
        &[
            "from sce_forge_runtime.bounded import BoundedInt\n",
            "class CodecSimpleFrameMsgIdInt(BoundedInt):\n",
            "    BITS = 8\n    SIGNED = False\n",
            "class CodecSimpleFramePayloadInt(BoundedInt):\n",
            "    BITS = 16\n    SIGNED = False\n",
            "    @property\n    def payload(self) -> CodecSimpleFramePayloadInt: ...\n",
            "    payload: CodecSimpleFramePayloadInt = 0\n",
            "        if name == \"msg_id\":\n            value = CodecSimpleFrameMsgIdInt(value)\n",
            "        elif name == \"payload\":\n            value = CodecSimpleFramePayloadInt(value)\n",
            "class CodecSimpleFrameSource(Protocol):\n",
            "\ndef encode_source(src: CodecSimpleFrameSource, w: SceSink) -> None:\n",
            "        encode_source(self, w)\n",
        ],
    );
    holds_none(
        "codec_simple_frame",
        &text,
        &["BoundedBytes", "_d_", "def payload(self) -> int"],
    );
}

/// A signed integer narrower than a Python `int` is held to its two's
/// complement span, and the decode reads the bits of its field as that span
/// does, so the frame of a negative value reads back as the value.
///
/// Turns red where `python_source::decoded_int` stops reading a signed
/// field's bits by `from_wire`, or where `int_class` writes a signed field as
/// unsigned.
#[test]
fn a_signed_integer_is_read_back_as_the_twos_complement_of_its_bits() {
    assert!(unlowered("codec_quantity_temperature").is_empty());
    let text = resource_module("codec_quantity_temperature");
    holds(
        "codec_quantity_temperature",
        &text,
        &[
            "class CodecQuantityTemperatureRawTempInt(BoundedInt):\n",
            "    BITS = 8\n    SIGNED = True\n",
            "raw_temp=CodecQuantityTemperatureRawTempInt.from_wire(",
            "            value = CodecQuantityTemperatureRawTempInt(value)\n",
        ],
    );
}

/// An element a gate decides is `None` where it is absent, written only where
/// it is there, and the flag that gates it is derived from it and folded into
/// its carrier in place of the value's bit.
///
/// Turns red where `python_source::encode_source` reads the presence flag off
/// the value rather than folding it from the gated element, or writes the
/// gated element whether or not it is there.
#[test]
fn a_gated_element_derives_its_flag_and_is_written_where_it_is_there() {
    assert!(unlowered("codec_present_if_basic").is_empty());
    let text = resource_module("codec_present_if_basic");
    holds(
        "codec_present_if_basic",
        &text,
        &[
            "    @property\n    def seq(self) -> Optional[CodecPresentIfBasicSeqInt]: ...\n",
            "value = None if value is None else CodecPresentIfBasicSeqInt(value)\n",
            "    _c_flags = src.flags\n",
            "    _c_flags = (_c_flags & ~0x1) | (0x1 if src.seq is not None else 0)\n",
            "    w.write_u8(_c_flags & 0xFF)\n",
            "    if src.seq is not None:\n        w.write_u8((src.seq >> 8) & 0xFF)\n        \
             w.write_u8(src.seq & 0xFF)\n",
        ],
    );
}

/// A body of a printed codec is any source of that codec, written by that
/// codec's module's `encode_source`, which the printed parent imports whole;
/// the value makes it by default by its codec's class.
///
/// Turns red where `python_source::positional_forms` stops printing a body of
/// a printed codec, or `encode_source` writes it other than through its
/// codec's module.
#[test]
fn a_body_is_written_by_its_codecs_own_encode_over_a_source() {
    assert!(unlowered("codec_embed_basic").is_empty());
    let text = resource_module("codec_embed_basic");
    holds(
        "codec_embed_basic",
        &text,
        &[
            "from . import codec_zenoh_locator as sce_codec_zenoh_locator\n",
            "from dataclasses import dataclass, field\n",
            "    def locator(self) -> sce_codec_zenoh_locator.CodecZenohLocatorSource: ...\n",
            "    locator: CodecZenohLocator = field(default_factory=CodecZenohLocator)\n",
            "    w.write_u8(src.tag & 0xFF)\n",
            "    sce_codec_zenoh_locator.encode_source(src.locator, w)\n",
        ],
    );
}

/// The encoded length of a body is the length of its frame, encoded once into
/// a buffer that is then written where the body is.
///
/// Turns red where `python_source::encode_source` reads the length off the
/// value, or writes the body by encoding it a second time.
#[test]
fn a_bodys_encoded_length_is_measured_over_the_frame_it_writes() {
    assert!(unlowered("codec_zenoh_timestamp_ext").is_empty());
    let text = resource_module("codec_zenoh_timestamp_ext");
    holds(
        "codec_zenoh_timestamp_ext",
        &text,
        &[
            "    _b_ts = bytearray()\n    \
             sce_codec_zenoh_timestamp.encode_source(src.ts, BytearraySink(_b_ts))\n    \
             _d_ext_size = len(_b_ts)\n",
            "    w.write_vle_u64(_d_ext_size)\n    w.write_bytes(_b_ts)\n",
        ],
    );
    holds_none(
        "codec_zenoh_timestamp_ext",
        &text,
        &["\n    ext_size: ", "src.ext_size"],
    );
}

/// A list of a printed codec is held to its bound in a tuple class, its count
/// derived from how many entries it holds, and each entry written by its
/// codec's own encode over the source the list holds it as.
///
/// Turns red where `python_source::encode_source` reads the count off the value
/// or stops walking the list through its entries' module.
#[test]
fn a_list_derives_its_count_and_writes_each_entry_by_its_codec() {
    assert!(unlowered("codec_repeat_basic").is_empty());
    let text = resource_module("codec_repeat_basic");
    holds(
        "codec_repeat_basic",
        &text,
        &[
            "from sce_forge_runtime.bounded import BoundedTuple\n",
            "class CodecRepeatBasicFragsList(BoundedTuple):\n",
            "    LEAST = 0\n    MOST = 32\n",
            "    frags: CodecRepeatBasicFragsList = CodecRepeatBasicFragsList()\n",
            "    _d_num_frags = len(src.frags)\n",
            "    w.write_u8(_d_num_frags & 0xFF)\n",
            "    for _entry in src.frags:\n        \
             sce_codec_repeat_elem.encode_source(_entry, w)\n",
        ],
    );
    holds_none(
        "codec_repeat_basic",
        &text,
        &["\n    num_frags: ", "src.num_frags"],
    );
}

/// A gate the encode cannot derive the presence from is a check: the value is
/// held to it before the first byte, tested on the carrier as the encode writes
/// it, and refused with the runtime's `PresentIfMismatch`.
///
/// Turns red where `python_source::encode_source` drops the loop over
/// `plan.gate_checks`, tests the gate on the value's carrier rather than the
/// one the encode writes, or `positional_forms` names a checked gate unlowered.
#[test]
fn a_gate_the_encode_cannot_derive_is_held_to_the_field_it_gates() {
    assert!(unlowered("codec_present_if_checked").is_empty());
    let text = resource_module("codec_present_if_checked");
    holds(
        "codec_present_if_checked",
        &text,
        &[
            "PresentIfMismatch, SceCursor",
            "    _c_flags = (_c_flags & ~0x1) | (0x1 if src.seq is not None else 0)\n",
            "    if (((_c_flags >> 0) & 1) != 0 or ((_c_flags >> 1) & 1) != 0) != \
             (src.trailer is not None):\n        raise PresentIfMismatch(\"trailer\")\n",
        ],
    );
}

/// Two byte strings that share one length each measure it, and a value whose
/// two differ is refused as an inconsistent source before the first byte.
///
/// Turns red where `python_source::derive_number` stops comparing every
/// measure always there with the first, or takes the number from the second.
#[test]
fn a_length_two_byte_strings_share_refuses_two_that_differ() {
    assert!(unlowered("codec_length_shared").is_empty());
    let text = resource_module("codec_length_shared");
    holds(
        "codec_length_shared",
        &text,
        &[
            "InconsistentSource, ",
            "    if len(src.beta) != len(src.alpha):\n        \
             raise InconsistentSource(\"span_len\")\n",
            "    _d_span_len = len(src.alpha)\n",
        ],
    );
}

/// The encoded length of a body that can refuse is refused where it does not
/// fit its field, and a frame that holds a codec that can refuse is written
/// whole first and handed to the caller's sink only once it is, so a refusal
/// leaves that sink as it was.
///
/// Turns red where `python_source::derive_number` drops the `Narrow::Checked`
/// test, or `encode_source` writes straight into the caller's sink for a plan
/// with a `RefusalKind::Child` refusal.
#[test]
fn a_frame_that_holds_a_codec_that_can_refuse_is_written_whole_first() {
    assert!(unlowered("codec_encoded_length_checked").is_empty());
    let text = resource_module("codec_encoded_length_checked");
    holds(
        "codec_encoded_length_checked",
        &text,
        &[
            "    _b_inner = bytearray()\n    \
             sce_codec_length_shared.encode_source(src.inner, BytearraySink(_b_inner))\n",
            "    if len(_b_inner) > 0xFFFF:\n        raise LengthOutOfRange(\"inner_size\")\n",
            "    _w = w\n    _frame = bytearray()\n    w = BytearraySink(_frame)\n",
            "    _w.write_bytes(_frame)\n",
        ],
    );
}

/// A length the value states while the byte strings it measures are absent is
/// `None` while any of them is there, derived then from them, each one there
/// saying the same, and what the value states where none is; a value that
/// states it beside content that says otherwise is refused, and the decode
/// gives it back `None` where it read any of the content.
///
/// Turns red where `python_source::derive_number` reads the stated number
/// where content is there, drops the check of a stated number against the
/// content, or `render` decodes the number where the content was read.
#[test]
fn a_length_the_value_states_is_read_only_where_its_content_is_absent() {
    assert!(unlowered("codec_length_stated").is_empty());
    let text = resource_module("codec_length_stated");
    holds(
        "codec_length_stated",
        &text,
        &[
            "    @property\n    def span_len(self) -> Optional[CodecLengthStatedSpanLenInt]: ...\n",
            "    span_len: Optional[CodecLengthStatedSpanLenInt] = None\n",
            "        value = None if value is None else CodecLengthStatedSpanLenInt(value)\n",
            "    _f_span_len = None\n    if src.alpha is not None:\n        \
             _f_span_len = len(src.alpha)\n    elif src.beta is not None:\n        \
             _f_span_len = len(src.beta)\n",
            "    if src.beta is not None and len(src.beta) != _f_span_len:\n        \
             raise InconsistentSource(\"span_len\")\n",
            "    _d_span_len = (_f_span_len) if _f_span_len is not None else \
             (0 if src.span_len is None else src.span_len)\n",
            "    if _f_span_len is not None and src.span_len is not None and \
             src.span_len != _d_span_len:\n        raise InconsistentSource(\"span_len\")\n",
            "            span_len=f_span_len if f_alpha is None and f_beta is None else None,\n",
        ],
    );
}

/// A length held in a flag of a carrier is folded into the carrier from the
/// bytes it measures, in place of the bits the value presents, and the value's
/// other flags of that carrier are written as it presents them.
///
/// Turns red where `python_source::derive_number` stops folding a
/// `NumberInto::Flag` number into its carrier, or `number_form` names it
/// unlowered.
#[test]
fn a_length_held_in_a_flag_is_folded_into_its_carrier() {
    assert!(unlowered("codec_length_ref_dotted_basic").is_empty());
    let text = resource_module("codec_length_ref_dotted_basic");
    holds(
        "codec_length_ref_dotted_basic",
        &text,
        &[
            "    _c_carrier = src.carrier\n",
            "    _c_carrier = (_c_carrier & ~0xF0) | ((len(src.payload)) << 4)\n",
            "    w.write_bytes(src.payload)\n",
        ],
    );
}

/// A flag input the value does not say is a parameter of the encode after the
/// sink, which a gate tests as it tests a flag, and which the value's `encode`
/// hands on.
///
/// Turns red where `python_source::encode_source` drops `plan.given` from its
/// parameters or `gate_test` stops spelling a `Clause::Input`.
#[test]
fn a_flag_input_the_value_does_not_say_is_handed_to_the_encode() {
    assert!(unlowered("codec_chain_handed_entry").is_empty());
    let text = resource_module("codec_chain_handed_entry");
    holds(
        "codec_chain_handed_entry",
        &text,
        &[
            "\ndef encode_source(src: CodecChainHandedEntrySource, w: SceSink, \
             after_marker: int, *, override_header_wide: Optional[int] = None, \
             override_header_z: Optional[int] = None) -> None:\n",
            "    if (after_marker != 0 or ((_c_header >> 6) & 1) != 0) != \
             (src.value is not None):\n        raise PresentIfMismatch(\"value\")\n",
            "        encode_source(self, w, after_marker)\n",
        ],
    );
}

/// A flag input the value says by the field gated on it alone is no parameter
/// of the encode or of the value's `encode`, and a function of the module,
/// `input_<name>`, reads it off any source; the decode still takes it. A
/// parent the template writes hands the printed codec's encode no such input,
/// and its decode every input.
///
/// Turns red where `python_source::positional_forms` names an input a gate
/// says unlowered, `input_functions` stops printing `input_<name>` or
/// `said_input` reads the gate's field other than by its presence, where
/// `render_codec` hands the value's `encode` an input its value says, or where
/// `embed_flag_bind_thread_args_supplying` hands a printed Python codec's
/// encode an input it says.
#[test]
fn a_flag_input_the_value_says_is_read_off_the_value_and_not_handed() {
    assert!(unlowered("codec_zenoh_undecl_token").is_empty());
    let text = resource_module("codec_zenoh_undecl_token");
    holds(
        "codec_zenoh_undecl_token",
        &text,
        &[
            "\ndef encode_source(src: CodecZenohUndeclTokenSource, w: SceSink) -> None:\n",
            "\ndef input_z(src: CodecZenohUndeclTokenSource) -> int:\n",
            "    return (1 if src.ext_keyexpr is not None else 0)\n",
            "    def decode(cls, cursor: SceCursor, z: int) -> \
             Optional[CodecZenohUndeclToken]:\n",
            "    def encode(self, w: SceSink) -> None:\n",
            "        encode_source(self, w)\n",
            "    def encode_to_bytes(self) -> bytes:\n",
        ],
    );
    // The parent hands the arm's encode no input the arm says, whether the
    // template writes the parent or the printer does, which writes the arm the
    // value holds by its codec's own encode.
    let parent = resource_module("codec_zenoh_declaration");
    let arm_write = if unlowered("codec_zenoh_declaration").is_empty() {
        "sce_codec_zenoh_undecl_token.encode_source(_v_body, w)\n"
    } else {
        "self.body.codec_zenoh_undecl_token.encode(w)\n"
    };
    holds(
        "codec_zenoh_declaration",
        &parent,
        &[
            "CodecZenohUndeclToken.decode(cursor, ((f_header >> 7) & 0x1))\n",
            arm_write,
        ],
    );
}

/// A group of a gated byte string and the length stated on the side without it
/// (docs/adr/0065 §6 R4b) is one field of the value, the `Union` of a dataclass
/// for each side; the encode binds each member off the arm the value holds,
/// derives the length from the bytes where the arm holds them and reads it off
/// the other arm otherwise, and the decode makes the arm the flag it read
/// selects. The members are no fields of the value of their own.
///
/// Turns red where `python_source::group_part` stops printing an arm's class or
/// the `Union`, `encode_source` reads a member off the source rather than the
/// arm, `derive_number` reads an `Absent::Group` number other than off the arm,
/// or `derived_field` declares a member as a field of the value.
#[test]
fn a_group_is_one_field_whose_arms_hold_its_members() {
    assert!(unlowered("codec_present_if_length_ref").is_empty());
    let text = resource_module("codec_present_if_length_ref");
    holds(
        "codec_present_if_length_ref",
        &text,
        &[
            "from typing import Optional, Protocol, Union\n",
            "@dataclass\nclass CodecPresentIfLengthRefFlagsHasPayloadHasPayload:\n",
            "    payload: CodecPresentIfLengthRefPayloadBytes = CodecPresentIfLengthRefPayloadBytes()\n",
            "@dataclass\nclass CodecPresentIfLengthRefFlagsHasPayloadNotHasPayload:\n",
            "    payload_size: CodecPresentIfLengthRefPayloadSizeInt = 0\n",
            "CodecPresentIfLengthRefFlagsHasPayload = Union[\
             CodecPresentIfLengthRefFlagsHasPayloadHasPayload, \
             CodecPresentIfLengthRefFlagsHasPayloadNotHasPayload]\n",
            "    def flags_has_payload(self) -> CodecPresentIfLengthRefFlagsHasPayload: ...\n",
            "    flags_has_payload: CodecPresentIfLengthRefFlagsHasPayload = \
             field(default_factory=CodecPresentIfLengthRefFlagsHasPayloadHasPayload)\n",
            "    _g_flags_has_payload = src.flags_has_payload\n",
            "    _e_payload = _g_flags_has_payload.payload if isinstance(_g_flags_has_payload, \
             CodecPresentIfLengthRefFlagsHasPayloadHasPayload) else None\n",
            "    _d_payload_size = (len(_e_payload)) if _e_payload is not None else \
             (0 if _e_payload_size is None else _e_payload_size)\n",
            "    _c_flags = (_c_flags & ~0x1) | (0x1 if _e_payload is not None else 0)\n",
            "    if _e_payload is not None:\n        w.write_bytes(_e_payload)\n",
            "            flags_has_payload=CodecPresentIfLengthRefFlagsHasPayloadHasPayload(\
             payload=f_payload) if (((f_flags >> 0) & 1) != 0) else \
             CodecPresentIfLengthRefFlagsHasPayloadNotHasPayload(payload_size=f_payload_size),\n",
        ],
    );
    holds_none(
        "codec_present_if_length_ref",
        &text,
        &[
            "\n    payload: Optional",
            "src.payload",
            "            payload=",
        ],
    );
}

/// A disjunction of a codec's own flags decides a group of an arm for each
/// assignment that holds it (docs/adr/0065 §6 R4c); each flag is derived from
/// the arm the value holds, and the value may hold none.
///
/// Turns red where `python_source::encode_source` stops folding a
/// `Derive::ArmFlag` from the arm the value holds, or `group_part` declares a
/// group the gates may leave out as other than `Optional`.
#[test]
fn a_disjunctions_flags_are_derived_from_the_arm_the_value_holds() {
    assert!(unlowered("codec_present_if_disjunction").is_empty());
    let text = resource_module("codec_present_if_disjunction");
    holds(
        "codec_present_if_disjunction",
        &text,
        &[
            "    flags_wants_a_or_wants_b: Optional[CodecPresentIfDisjunctionFlagsWantsAOrWantsB] \
             = None\n",
            "    _g_flags_wants_a_or_wants_b = src.flags_wants_a_or_wants_b\n",
            "    _c_flags = (_c_flags & ~0x1) | (0x1 if isinstance(_g_flags_wants_a_or_wants_b, (",
            "    _c_flags = (_c_flags & ~0x2) | (0x2 if isinstance(_g_flags_wants_a_or_wants_b, (",
            " else None,\n",
        ],
    );
}

/// An input a group gated on it says is what the arm the value holds gives it,
/// read off any source by `input_<name>`.
///
/// Turns red where `python_source::said_input` stops reading a group's input
/// off the arm the value holds, or `positional_forms` names it unlowered.
#[test]
fn an_input_a_group_says_is_read_off_the_arm_the_value_holds() {
    assert!(unlowered("codec_zenoh_init_body").is_empty());
    let text = resource_module("codec_zenoh_init_body");
    holds(
        "codec_zenoh_init_body",
        &text,
        &[
            "\ndef input_s(src: CodecZenohInitBodySource) -> int:\n",
            "    return (1 if isinstance(src.s, CodecZenohInitBodyS) else 0)\n",
            "    s: Optional[CodecZenohInitBodyS] = None\n",
            "    _e_sn_res = None if _g_s is None else _g_s.sn_res\n",
        ],
    );
}

/// Two lists of one count are one member of the arm that holds both, an
/// `Aligned` of the two, each held in the class of its bound (docs/adr/0065
/// §6 R5): the encode reads each list off the pair where the arm holds it and
/// off the arm where it holds the first alone, and the decode pairs the two
/// lists it read.
///
/// Turns red where `python_source::positional_forms` names two lists of one
/// count unlowered again, `group_part` declares the second list as a member
/// of its own, or `encode_source` binds `_e_links` off the arm alone.
#[test]
fn two_lists_of_one_count_are_one_aligned_member_of_their_arm() {
    let name = "codec_repeat_unconditional_count";
    assert!(unlowered(name).is_empty(), "{:?}", unlowered(name));
    let text = resource_module(name);
    holds(
        name,
        &text,
        &[
            "from sce_forge_runtime.bounded import Aligned, BoundedTuple\n",
            "    links_weights: Aligned[CodecRepeatUnconditionalCountLinksList, \
             CodecRepeatUnconditionalCountWeightsList] = \
             Aligned(CodecRepeatUnconditionalCountLinksList(), \
             CodecRepeatUnconditionalCountWeightsList())\n",
            "            value = Aligned(CodecRepeatUnconditionalCountLinksList(value.first), \
             CodecRepeatUnconditionalCountWeightsList(value.second))\n",
            ".links_weights.first if isinstance(",
            ".links_weights.second if isinstance(",
            "    _d_links_len = len(_e_links)\n",
            "links_weights=Aligned(f_links, f_weights)",
        ],
    );
    holds_none(name, &text, &["    weights: ", "weights=f_weights"]);
}

/// A variant is held as the arm the value holds: a value of a named arm's
/// codec, or the default arm's class of the tag it keeps, held to the tags no
/// arm names (docs/adr/0065 §6 R3), and its body. The encode writes the arm
/// by its codec's own encode, and the decode makes the arm it read.
///
/// Turns red where `python_source::positional_forms` names a variant
/// unlowered again, `variant_part` stops holding the default arm's tag in its
/// class or starts the value in another arm than the document marks, or the
/// template's decode stops making the default arm's class.
#[test]
fn a_variant_is_the_arm_the_value_holds_and_the_default_arm_keeps_its_tag() {
    let name = "codec_variant_default_marker";
    assert!(unlowered(name).is_empty(), "{:?}", unlowered(name));
    let text = resource_module(name);
    let arm_a = "sce_codec_default_marker_arm_a.CodecDefaultMarkerArmA";
    let arm_b = "sce_codec_default_marker_arm_b.CodecDefaultMarkerArmB";
    holds(
        name,
        &text,
        &[
            "from sce_forge_runtime.bounded import UnknownTag\n",
            "class CodecVariantDefaultMarkerUnknownTag(UnknownTag):\n",
            "    MOST = 3\n    KNOWN = frozenset({1, 2})\n",
            "    tag: CodecVariantDefaultMarkerUnknownTag = CodecVariantDefaultMarkerUnknownTag(0)\n",
            &format!("    body: {arm_b} = field(default_factory={arm_b})\n"),
            &format!(
                "CodecVariantDefaultMarkerVariant = Union[{arm_a}, {arm_b}, \
                 CodecVariantDefaultMarkerDefault]\n"
            ),
            &format!(
                "    body: CodecVariantDefaultMarkerVariant = field(default_factory={arm_b})\n"
            ),
            "    def body(self) -> CodecVariantDefaultMarkerVariant: ...\n",
            &format!(
                "    if isinstance(_v_body, {arm_a}):\n        \
                 sce_codec_default_marker_arm_a.encode_source(_v_body, w)\n"
            ),
            "    else:\n        sce_codec_default_marker_arm_b.encode_source(_v_body.body, w)\n",
            "            body = CodecVariantDefaultMarkerDefault(tag=",
            "            body = _arm\n",
        ],
    );
    // The arm's own header holds its tag (a peek byte), so the value has no
    // tag a caller hands, and no `tag`.
    holds_none(
        name,
        &text,
        &[
            "class CodecVariantDefaultMarkerVariant:",
            "body.kind = ",
            "\ndef tag(",
        ],
    );
}

/// A chain that ends on its entries' flag sets it on every entry but the last,
/// in place of the one the entry presents: the entry's encode takes each
/// one-bit flag a parent may set as a keyword, folded last into its carrier.
///
/// Turns red where `python_source::positional_forms` names a chain that ends
/// on its entries' flag unlowered again, `encode_source` stops folding an
/// override into its carrier, or the walk stops setting the flag from how
/// many entries are left.
#[test]
fn a_chain_sets_the_flag_its_entries_end_on() {
    let entry = "codec_chain_depth_entry";
    assert!(unlowered(entry).is_empty(), "{:?}", unlowered(entry));
    let text = resource_module(entry);
    holds(
        entry,
        &text,
        &[
            "\ndef encode_source(src: CodecChainDepthEntrySource, w: SceSink, *, \
             override_header_z: Optional[int] = None) -> None:\n",
            "    if override_header_z is not None:\n        \
             _c_header = (_c_header & ~0x80) | ((override_header_z & 0x1) << 7)\n",
        ],
    );
    let chain = "codec_chain_depth_plain";
    assert!(unlowered(chain).is_empty(), "{:?}", unlowered(chain));
    let text = resource_module(chain);
    holds(
        chain,
        &text,
        &["    _left_extensions = len(src.extensions)\n    \
           for _entry in src.extensions:\n        \
           _left_extensions -= 1\n        \
           sce_codec_chain_depth_entry.encode_source(_entry, w, \
           override_header_z=1 if _left_extensions != 0 else 0)\n"],
    );
    // A chain that hands each entry whether the one before it was of a kind
    // keeps that as it writes, 0 before the first entry.
    let handed = "codec_chain_handed";
    assert!(unlowered(handed).is_empty(), "{:?}", unlowered(handed));
    let text = resource_module(handed);
    holds(
        handed,
        &text,
        &[
            "    _prev_after_marker = 0\n    _left_entries = len(src.entries)\n    \
           for _entry in src.entries:\n        \
           _left_entries -= 1\n        \
           sce_codec_chain_handed_entry.encode_source(_entry, w, _prev_after_marker, \
           override_header_z=1 if _left_entries != 0 else 0)\n        \
           _prev_after_marker = 1 if ((_entry.header >> 0) & 0xF) == 0x4 else 0\n",
        ],
    );
}

/// A list whose entries say the input the codec hands every one of them is
/// held to that law as a `Lawful` of it (docs/adr/0065 §6 R8): the law is the
/// runtime's automaton reading each entry through the entry codec's own
/// `input_<name>`, the flag the list hands its entries is derived from the
/// list's start, and the decode holds the list it read to the law from the
/// flag the frame gave.
///
/// Turns red where `python_source::value_forms` names a list held to a law
/// unlowered again, `law_part` stops defining the law or its `Lawful`,
/// `plain_measure` stops reading a flag off the list's start, or
/// `decoded_lawful` stops making the decoded list a `Lawful`.
#[test]
fn a_list_held_to_a_law_is_a_lawful_whose_start_derives_its_bound_flag() {
    let name = "codec_repeat_flag_carrier";
    assert!(unlowered(name).is_empty(), "{:?}", unlowered(name));
    let text = resource_module(name);
    holds(
        name,
        &text,
        &[
            "from sce_forge_runtime.law import Lawful, Uniform\n",
            "CodecRepeatFlagCarrierEntriesLaw = \
             (Uniform(sce_codec_chain_flag_entry.input_wide),)\n",
            "class CodecRepeatFlagCarrierEntriesLawful(Lawful):\n",
            "    ENTRIES = CodecRepeatFlagCarrierEntriesList\n    \
             LAW = CodecRepeatFlagCarrierEntriesLaw\n",
            "_c_head = (_c_head & ~0x1) | ((int(src.entries.start)) << 0)\n",
            "entries=CodecRepeatFlagCarrierEntriesLawful(",
        ],
    );
}

/// A chain a chain supplies an input from the entry before, held to a law,
/// reads each entry's input through the entry codec's own `input_<name>` by an
/// `AfterKey` that waits for the kind a class of the codec tells.
///
/// Turns red where `python_source::law_part` stops printing an `AfterKey` of
/// the kind, or `key_class` stops reading the kind off the chain's `entry-id`.
#[test]
fn an_input_the_entry_before_decides_is_an_after_key_law_of_its_kind() {
    let name = "codec_chain_prev_carrier";
    assert!(unlowered(name).is_empty(), "{:?}", unlowered(name));
    let text = resource_module(name);
    holds(
        name,
        &text,
        &[
            "from sce_forge_runtime.law import AfterKey, Lawful, Uniform\n",
            "Uniform(sce_codec_chain_prev_entry.input_tagged)",
            "AfterKey(sce_codec_chain_prev_entry.input_after_marker, ",
            "    def matches(entry: sce_codec_chain_prev_entry.CodecChainPrevEntrySource) -> bool:\n",
            "        return ((entry.header >> 0) & 0xF) == 0x4\n",
        ],
    );
}

/// The gates on a chain's entries make a group whose arms hold the chain by
/// the kinds each says (docs/adr/0065 §6 R6b): a `Chain` of an entry of each
/// kind the arm sets and of none it clears, a field gated on a membership
/// written where the arm the value holds says it, and the decode making the
/// arm its read chain's kinds select. A bare entry's arm holds the others and
/// the entry's place (R6a), and the named entry is made by the kind's class,
/// its arm the one its identifier's tag bits select.
///
/// Turns red where `python_source::positional_forms` names a group of a
/// chain's memberships unlowered again, `chain_member` stops holding the
/// chain as a `Chain` or a `Placed`, `decoded_arm_test` stops reading a
/// membership off the decoded chain, or `named_entry` stops making the named
/// entry's arm through the entry codec's `ARM_CLASSES`.
#[test]
fn a_chains_memberships_are_a_group_whose_arms_hold_the_chain_by_its_kinds() {
    let name = "codec_chain_has_envelope";
    assert!(unlowered(name).is_empty(), "{:?}", unlowered(name));
    let text = resource_module(name);
    holds(
        name,
        &text,
        &[
            "from sce_forge_runtime.membership import Chain\n",
            "    \"\"\"The kind of entry of ``extensions`` of :class:`CodecChainHasEnvelope` a gate or a\n",
            "    def matches(entry: sce_codec_zenoh_ext_entry.CodecZenohExtEntrySource) -> bool:\n",
            "Chain(Chain):\n",
            "    HOLDS = (CodecChainHasEnvelope",
            "    LACKS = (CodecChainHasEnvelope",
            " is not None and any(",
        ],
    );
    holds_none(name, &text, &["(Placed):\n", "def named("]);
    let name = "codec_chain_has_marker";
    assert!(unlowered(name).is_empty(), "{:?}", unlowered(name));
    let text = resource_module(name);
    holds(
        name,
        &text,
        &[
            "from sce_forge_runtime.membership import Chain, Placed\n",
            "Placed(Placed):\n",
            "    def named() -> sce_codec_zenoh_ext_entry.CodecZenohExtEntry:\n",
            "        return sce_codec_zenoh_ext_entry.CodecZenohExtEntry(header=0x12, \
             body=sce_codec_zenoh_ext_entry.ARM_CLASSES[0x0]())\n",
            "Placed.place_first(",
        ],
    );
    let entry = resource_module("codec_zenoh_ext_entry");
    holds("codec_zenoh_ext_entry", &entry, &["\nARM_CLASSES = {0x0: "]);
}

/// An input a chain's entry says, which the chain supplies from the entry
/// before, is held entry by entry to whether the entry before was of the kind
/// where the chain's memberships hold it to no law (docs/adr/0065 §6 R8b): the
/// encode refuses an entry that says otherwise before the first byte.
///
/// Turns red where `python_source::input_check` stops walking the chain with
/// the entry before's kind, or `positional_forms` names such a check unlowered.
#[test]
fn an_input_the_entry_before_decides_is_checked_where_no_law_holds_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let entry = "codec_chain_prev_entry.scxml";
    std::fs::copy(resources().join(entry), dir.path().join(entry)).expect("copy the entry");
    let holder = dir.path().join("codec_prev_checked.scxml");
    std::fs::write(
        &holder,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="codec_prev_checked">
  <sce:import src="codec_chain_prev_entry.scxml" kind="codec" as="codec_chain_prev_entry">
    <sce:flag-bind input="tagged" source="head.T"/>
  </sce:import>
  <datamodel>
    <sce:flags id="head" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="T" bit="0"/>
    </sce:flags>
    <sce:tlv-chain id="entries" type="codec_chain_prev_entry" sce:byte="1"
                   max-depth="4" entry-id="header.ext_id">
      <sce:entry-flag-bind input="after_marker" previous-entry-id="0x4"/>
    </sce:tlv-chain>
    <sce:field id="marker_note" sce:type="uint8" sce:byte="2" sce:bit-size="8"
               sce:present-if="entries.has(0x4)"/>
  </datamodel>
</scxml>
"#,
    )
    .expect("write the holding codec");
    let forms = python_codec_unlowered_file(&holder, &[], &ForgeCompileOptions::default())
        .expect("the holding codec reads")
        .expect("the holding codec is a codec");
    assert!(forms.is_empty(), "{forms:?}");
    let text = module_of(&holder).expect("the holding codec generates on Python");
    holds(
        "codec_prev_checked",
        &text,
        &[
            "    _prev = False\n",
            "sce_codec_chain_prev_entry.input_after_marker(_x)",
            "    _prev = ((_x.header >> 0) & 0xF) == 0x4\n",
            "raise PresentIfMismatch(\"entries\")",
        ],
    );
}

/// A CBOR map is printed from its plan: every entry is presented, each held in
/// the class of its bound or width (an exact byte string a least equal to its
/// most), and the encode over any source writes the map head, counting the
/// optional entries present, then each entry present in ascending key order.
/// No bound is checked in the encode, which the classes hold.
///
/// Turns red where `python_source::cbor::render` writes the entries in
/// declaration order rather than `by_key`, drops an optional entry from the
/// count, holds an exact byte string to its most alone, or checks a bound the
/// class holds.
#[test]
fn a_cbor_map_is_printed_from_its_plan_in_ascending_key_order() {
    let name = "codec_cbor_map";
    assert!(unlowered(name).is_empty());
    let text = resource_module(name);
    holds(
        name,
        &text,
        &[
            "from typing import Optional, Protocol\n",
            "class CodecCborMapIdBytes(BoundedBytes):\n",
            "    LEAST = 16\n    MOST = 16\n",
            "class CodecCborMapSourceText(BoundedText):\n",
            "    LEAST = 0\n    MOST = 64\n",
            "class CodecCborMapPatternInt(BoundedInt):\n",
            "    BITS = 8\n    SIGNED = False\n",
            "class CodecCborMapSource(Protocol):\n",
            "    @property\n    def deadline(self) -> Optional[CodecCborMapDeadlineInt]: ...\n",
            "    @property\n    def urgent(self) -> Optional[bool]: ...\n",
            "    id: CodecCborMapIdBytes = CodecCborMapIdBytes(b\"\\x00\" * 16)\n",
            "        elif name == \"deadline\":\n            \
             value = None if value is None else CodecCborMapDeadlineInt(value)\n",
            "    count = 4\n    if src.deadline is not None:\n        count += 1\n    \
             if src.urgent is not None:\n        count += 1\n    cbor.write_map_head(w, count)\n    \
             v = src.id\n    cbor.write_uint(w, 0)\n    cbor.write_bytes(w, v)\n    \
             v = src.source\n    cbor.write_uint(w, 1)\n    cbor.write_text(w, v)\n",
            "    v = src.urgent\n    if v is not None:\n        cbor.write_uint(w, 7)\n        \
             cbor.write_bool(w, v)\n    v = src.deadline\n    if v is not None:\n        \
             cbor.write_uint(w, 12)\n",
            "        encode_source(self, w)\n",
        ],
    );
    holds_none(
        name,
        &text,
        &[
            "CborWrongLength()",
            "if not 0 <=",
            "> 256:",
            "dataclasses.field",
        ],
    );
}

/// A content-line component is printed from its plan: every property and
/// parameter is presented by the `Protocol`, an integer held alone in the
/// class of its width, and the encode writes each property's lines in
/// declaration order into a buffer first, raising each refusal the plan names
/// before a byte reaches the caller's sink.
///
/// Turns red where `python_source::unlowered_forms` names a content-line
/// component again, where `content_line::render` stops staging the component,
/// drops the judgement of a repeated property's count or of a parameter given
/// without its property, reads a parameter off the value rather than off the
/// line record, or holds an integer alone in a bare `int`.
#[test]
fn a_content_line_component_is_printed_from_its_plan() {
    let name = "codec_content_line_event";
    assert!(unlowered(name).is_empty());
    let text = resource_module(name);
    holds(
        name,
        &text,
        &[
            "from sce_forge_runtime.bounded import BoundedInt\n",
            "from typing import List, Optional, Protocol, Sequence\n",
            "class CodecContentLineEventSequenceInt(BoundedInt):\n",
            "    BITS = 16\n    SIGNED = False\n",
            "class CodecContentLineEventPriorityInt(BoundedInt):\n",
            "    BITS = 32\n    SIGNED = True\n",
            "class CodecContentLineEventSource(Protocol):\n",
            "    @property\n    def exdate(self) -> Sequence[str]: ...\n",
            "    @property\n    def sequence(self) -> Optional[CodecContentLineEventSequenceInt]: ...\n",
            "    sequence: Optional[CodecContentLineEventSequenceInt] = None\n",
            "        if name == \"sequence\":\n            \
             value = None if value is None else CodecContentLineEventSequenceInt(value)\n",
            "    _w = w\n    _frame = bytearray()\n    w = BytearraySink(_frame)\n",
            "    v = src.dtstart\n    out.property(\"DTSTART\")\n    \
             out.param(\"TZID\", src.dtstart_tzid, 24)\n    out.string(v, False, 20)\n",
            "    v = src.summary\n    if v is not None:\n        out.property(\"SUMMARY\")\n        \
             out.string(v, True, 24)\n",
            "    v = src.organizer\n    if v is not None:\n        out.property(\"ORGANIZER\")\n        \
             _pv = src.organizer_cn\n        if _pv is not None:\n            \
             out.param(\"CN\", _pv, 24)\n        out.string(v, False, 40)\n    \
             elif src.organizer_cn is not None:\n        \
             raise content_line.LineRequiredMissing(\"organizer\")\n",
            "    v = src.exdate\n    if len(v) > 3:\n        raise content_line.LineTooMany(\"exdate\")\n    \
             for _v in v:\n        out.property(\"EXDATE\")\n        out.string(_v, False, 20)\n",
            "    v = src.priority\n    if v is not None:\n        out.property(\"PRIORITY\")\n        \
             out.integer(v)\n",
            "    out.finish()\n    _w.write_bytes(_frame)\n",
            "        encode_source(self, w)\n",
        ],
    );
    holds_none(
        name,
        &text,
        &["raise content_line.LineBadValue()", "if not -2147483648 <="],
    );

    let name = "codec_content_line_enums";
    assert!(unlowered(name).is_empty());
    let text = resource_module(name);
    holds(
        name,
        &text,
        &[
            "class CodecContentLineEnumsAttendeeLineSource(Protocol):\n",
            "    @property\n    def attendee(self) -> \
             Sequence[CodecContentLineEnumsAttendeeLineSource]: ...\n",
            "    v = src.attendee\n    if len(v) > 3:\n        \
             raise content_line.LineTooMany(\"attendee\")\n    for _line in v:\n        \
             out.property(\"ATTENDEE\")\n        \
             out.enum_param(\"ROLE\", _TEXTS_role, _line.attendee_role.to_underlying())\n",
        ],
    );
}

/// A codec whose plan uses a form the Python printer does not lower keeps the
/// template's encode whole and names its form: a field of a `float32`.
///
/// Turns red where `python_source::value_forms` stops naming a float field.
#[test]
fn a_codec_of_a_form_the_printer_does_not_lower_keeps_the_template_encode() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("measure.scxml");
    std::fs::write(
        &path,
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="measure">
  <datamodel>
    <sce:field id="tag" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:field id="reading" sce:type="float32" sce:byte="1" sce:bit-size="32"/>
  </datamodel>
</scxml>"#,
    )
    .expect("write the measure codec");
    let forms = python_codec_unlowered_file(&path, &[], &ForgeCompileOptions::default())
        .expect("the document reads")
        .expect("the document is a codec");
    assert_eq!(
        forms,
        vec!["field 'reading': a `float32` field".to_string()],
        "measure: the printer names the float field"
    );
    let text = module_of(&path).expect("measure generates on Python");
    holds_none("measure", &text, &["def encode_source(", "(Protocol):"]);
}

/// A VLE of eight bits never reaches the printer: the parser refuses it, so
/// the printer writes every VLE it is handed by the runtime's writer of its
/// width and names no VLE form.
///
/// Turns red where the parser admits a `uint8` VLE, which the printer's
/// `EVERY_VLE_WRITTEN` invariant then no longer holds.
#[test]
fn a_vle_of_eight_bits_is_refused_before_the_printer() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("narrow.scxml");
    std::fs::write(
        &path,
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="narrow">
  <datamodel>
    <sce:field id="tag" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:field id="n" sce:type="uint8" sce:byte="1" sce:bit-size="vle"/>
  </datamodel>
</scxml>"#,
    )
    .expect("write the narrow codec");
    let refused = python_codec_unlowered_file(&path, &[], &ForgeCompileOptions::default())
        .expect_err("the parser refuses a uint8 VLE");
    assert!(
        format!("{refused:?}").contains("vle requires sce:type"),
        "{refused:?}"
    );
}

/// Every codec of the resources is printed exactly where the printer lowers
/// each of its forms, and each printed one generates a module with its encode
/// over a source. The printed codecs are shown with `--nocapture`.
///
/// Turns red where `python_codec_printed` and `python_codec_unlowered` come
/// apart, where `render_codec` prints a codec the printer does not lower or
/// leaves one it lowers on the template's encode, or where the printer panics
/// or the template fails on a codec it lowers.
#[test]
fn the_python_backend_prints_each_codec_of_the_resources_whose_forms_it_lowers() {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(resources())
        .expect("the resources directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "scxml"))
        .collect();
    paths.sort();
    let mut printed: Vec<String> = Vec::new();
    for path in &paths {
        let stem = path
            .file_stem()
            .expect("a document has a stem")
            .to_string_lossy()
            .into_owned();
        // A document that does not read, or is no codec, has no encode.
        let Ok(Some(forms)) =
            python_codec_unlowered_file(path, &[], &ForgeCompileOptions::default())
        else {
            continue;
        };
        assert_eq!(
            python_codec_printed_file(path, &[], &ForgeCompileOptions::default()).ok(),
            Some(Some(forms.is_empty())),
            "{stem}: printed exactly where no form is unlowered"
        );
        match module_of(path) {
            Ok(text) => assert_eq!(
                text.contains("\ndef encode_source(src: "),
                forms.is_empty(),
                "{stem}: the module prints its encode over a source exactly where the \
                 printer lowers every form ({forms:?})"
            ),
            // A codec of MCU-only features is generated on no host backend,
            // Python among them, whatever its forms; one the printer prints
            // generates otherwise.
            Err(refusal) => assert!(
                !forms.is_empty() || refusal.contains("CodegenMcuClassKindOnNonMcuLanguage"),
                "{stem}: a codec the printer prints generates on Python: {refusal}"
            ),
        }
        if forms.is_empty() {
            printed.push(stem);
        }
    }
    eprintln!("{} codec(s) printed on Python:", printed.len());
    for stem in &printed {
        eprintln!("  {stem}");
    }
    for name in [
        "codec_length_ref",
        "codec_scout_zid_body",
        "codec_simple_frame",
        "codec_present_if_checked",
        "codec_length_shared",
        "codec_encoded_length_checked",
        "codec_length_stated",
        "codec_length_ref_dotted_basic",
        "codec_chain_handed_entry",
        "codec_zenoh_undecl_token",
        "codec_repeat_unconditional_count",
        "codec_variant_default_marker",
        "codec_repeat_flag_carrier",
        "codec_chain_prev_carrier",
        "codec_chain_has_envelope",
        "codec_chain_has_marker",
    ] {
        assert!(
            printed.iter().any(|stem| stem == name),
            "{name} is printed on Python"
        );
    }
}
