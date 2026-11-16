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
            "from sce_forge_runtime.bounded import BoundedBytes\n",
            "from typing import Optional, Protocol\n",
            "class CodecLengthRefPayloadBytes(BoundedBytes):\n",
            "    LEAST = 0\n    MOST = 32\n",
            "class CodecLengthRefSource(Protocol):\n",
            "    @property\n    def msg_id(self) -> int: ...\n",
            "    @property\n    def payload(self) -> CodecLengthRefPayloadBytes: ...\n",
            "    payload: CodecLengthRefPayloadBytes = CodecLengthRefPayloadBytes()\n",
            "    def __setattr__(self, name: str, value: object) -> None:\n",
            "        if name == \"payload\":\n            value = CodecLengthRefPayloadBytes(value)\n",
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

/// A codec of integers alone is printed with no class of a bound, its value's
/// fields unchanged.
///
/// Turns red where `python_source::positional_forms` refuses an integer
/// element, or where the template imports `BoundedBytes` for a codec that
/// presents no byte string.
#[test]
fn a_codec_of_integers_is_printed_with_no_class_of_a_bound() {
    assert!(unlowered("codec_simple_frame").is_empty());
    let text = resource_module("codec_simple_frame");
    holds(
        "codec_simple_frame",
        &text,
        &[
            "class CodecSimpleFrameSource(Protocol):\n",
            "\ndef encode_source(src: CodecSimpleFrameSource, w: SceSink) -> None:\n",
            "        encode_source(self, w)\n",
        ],
    );
    holds_none(
        "codec_simple_frame",
        &text,
        &["BoundedBytes", "__setattr__", "_d_"],
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
            "    @property\n    def seq(self) -> Optional[int]: ...\n",
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
            "    @property\n    def span_len(self) -> Optional[int]: ...\n",
            "    span_len: Optional[int] = None\n",
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
             after_marker: int) -> None:\n",
            "    if (after_marker != 0 or ((src.header >> 6) & 1) != 0) != \
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
    let parent = resource_module("codec_zenoh_declaration");
    holds(
        "codec_zenoh_declaration",
        &parent,
        &[
            "CodecZenohUndeclToken.decode(cursor, ((f_header >> 7) & 0x1))\n",
            "self.body.codec_zenoh_undecl_token.encode(w)\n",
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
            "    payload_size: int = 0\n",
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

/// A codec whose plan uses a form the Python printer does not lower keeps the
/// template's encode whole: a variant and a CBOR map each name their form, and
/// so does a group of a chain's memberships.
///
/// Turns red where `python_source::positional_forms` stops naming a variant or
/// a group of a chain's memberships, or `unlowered_forms` a CBOR map.
#[test]
fn a_codec_of_a_form_the_printer_does_not_lower_keeps_the_template_encode() {
    let forms = unlowered("codec_chain_has_envelope");
    assert!(
        forms
            .iter()
            .any(|f| f.contains("a group of a chain's memberships")),
        "codec_chain_has_envelope: a group of a chain's memberships is not printed yet: {forms:?}"
    );
    for (name, form) in [
        ("codec_variant_default_marker", "a variant"),
        ("codec_cbor_map", "a CBOR map"),
    ] {
        let forms = unlowered(name);
        assert!(
            forms.iter().any(|f| f.contains(form)),
            "{name}: the printer names `{form}` among {forms:?}"
        );
        let text = resource_module(name);
        holds_none(
            name,
            &text,
            &["def encode_source(", "(Protocol):", "BoundedBytes"],
        );
    }
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
    ] {
        assert!(
            printed.iter().any(|stem| stem == name),
            "{name} is printed on Python"
        );
    }
}
