# SCE-MAP: codec_zenoh_join:41

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink

from dataclasses import dataclass
from typing import Optional


@dataclass
class CodecZenohJoin:
    version: int = 0
    cbyte: int = 0
    zid: bytes = b""
    sn_res: Optional[int] = None
    batch_size: Optional[int] = None
    lease: int = 0
    next_sn_reliable: int = 0
    next_sn_best_effort: int = 0

    @classmethod
    def decode(cls, cursor: SceCursor, s: int) -> Optional[CodecZenohJoin]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §synth-5-B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        # Streaming cursor decode (SSOT selection: `needs_streaming`).
        # The positional `raw[byte_off]` path is valid only when every
        # field's absolute offset is fixed at codegen time; this branch
        # handles every codec where it is not — present-if-gated fields
        # (runtime presence), VLE / repeat / TLV-chain / embed fields
        # (runtime width), string fields (UTF-8 decode), and a fixed field
        # after a variable-length payload (offset depends on the payload
        # length). Each field reads its own bytes and advances past what it
        # consumed, all inside one outer `try:`. The `except` catches the
        # base `CodecError` so a peek/advance `NeedMoreBytes` and a VLE
        # field's `VleWidthOverflow` both unwind to a single `return None`
        # (non-VLE codecs only ever raise `NeedMoreBytes`, a `CodecError`
        # subclass, so this is behaviour-identical for them).
        try:
            raw = cursor.peek_slice(1)
            version = raw[0]
            cursor.advance(1)
            raw = cursor.peek_slice(1)
            cbyte = raw[0]
            cursor.advance(1)
            _n = (((cbyte >> 4) & 0xF) + 1)
            raw = cursor.peek_slice(_n)
            zid = bytes(raw)
            cursor.advance(_n)
            if (s & 0x01) != 0:
                raw = cursor.peek_slice(1)
                _v = raw[0]
                cursor.advance(1)
                sn_res = _v
            else:
                sn_res = None
            if (s & 0x01) != 0:
                raw = cursor.peek_slice(2)
                _v = raw[0] | (raw[1] << 8)
                cursor.advance(2)
                batch_size = _v
            else:
                batch_size = None
            lease = cursor.read_vle_u64()
            next_sn_reliable = cursor.read_vle_u64()
            next_sn_best_effort = cursor.read_vle_u64()
        except CodecError:
            return None
        return cls(
            version=version,
            cbyte=cbyte,
            zid=zid,
            sn_res=sn_res,
            batch_size=batch_size,
            lease=lease,
            next_sn_reliable=next_sn_reliable,
            next_sn_best_effort=next_sn_best_effort,
        )

    # RFC §synth-5-B flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def whatami(self) -> int:
        return (self.cbyte >> 0) & 0x03

    def set_whatami(self, v: int) -> None:
        _shifted_mask = 0x03 << 0
        _val = (v & 0x03) << 0
        self.cbyte = ((self.cbyte & (0xFF ^ _shifted_mask)) | _val) & 0xFF

    def zid_len_m1(self) -> int:
        return (self.cbyte >> 4) & 0x0F

    def set_zid_len_m1(self, v: int) -> None:
        _shifted_mask = 0x0F << 4
        _val = (v & 0x0F) << 4
        self.cbyte = ((self.cbyte & (0xFF ^ _shifted_mask)) | _val) & 0xFF

    def encode(self, w: SceSink, s: int) -> None:
        """RFC §synth-5-B encode-side primary: write ``self`` into the
        caller-owned ``w`` sink. Returns ``None`` on success; raises
        :class:`BufferOverflow` from a bounded sink when the destination
        has insufficient remaining capacity; growable sinks (e.g.
        :class:`BytearraySink`) are effectively infallible."""
        # Streaming cursor encode (SSOT selection: `needs_streaming`).
        # Mirrors the streaming decode: every field appends its own bytes
        # in declaration order through the per-field encode blocks, so a
        # gated field skips its append when absent, and a fixed field after
        # a variable-length payload lands after the payload (the positional
        # path appends variable fields last, placing it ahead on the wire).
        # Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
        # dedicated helpers; everything else uses `present_if_encode_block`.
        w.write_u8(self.version & 0xFF)
        w.write_u8(self.cbyte & 0xFF)
        w.write_bytes(self.zid)
        if self.sn_res is not None:
            w.write_u8(self.sn_res & 0xFF)
        if self.batch_size is not None:
            w.write_u8(self.batch_size & 0xFF)
            w.write_u8((self.batch_size >> 8) & 0xFF)
        w.write_vle_u64(self.lease)
        w.write_vle_u64(self.next_sn_reliable)
        w.write_vle_u64(self.next_sn_best_effort)

    def encode_to_bytes(self, s: int) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst), s)
        return bytes(_dst)
