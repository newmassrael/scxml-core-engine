# SCE-MAP: codec_zenoh_value_zbuf:27 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, LengthOutOfRange, NeedMoreBytes, PresentIfMismatch, SceCursor, SceSink
from .codec_zenoh_value_slice import CodecZenohValueSlice

from dataclasses import dataclass, field
from typing import Optional, List


@dataclass
class CodecZenohValueZbuf:
    value_len: int = 0
    value: Optional[bytes] = None
    encoding: Optional[int] = None
    slice_count: Optional[int] = None
    slices: Optional[List[CodecZenohValueSlice]] = None

    @classmethod
    def decode(cls, cursor: SceCursor, after_shm: int) -> CodecZenohValueZbuf:
        """Decode the next frame from ``cursor``; on success the cursor
        advances past the consumed bytes. A frame the codec refuses raises
        the ``CodecError`` subclass that names the refusal (docs/adr/0099,
        amendment of 2026-11-16): ``NeedMoreBytes`` when the cursor's tail
        is shorter than the frame (RFC §synth-5-B L494-519), which the
        caller resumes after appending bytes, and for a frame no more bytes
        make whole ``VleWidthOverflow``, ``InvalidUtf8``,
        ``UndeclaredEnumValue``, or a length or a count past its bound
        (``LengthOutOfRange``, ``TooManyElements``, docs/adr/0077)."""
        # Streaming cursor decode (SSOT selection: `needs_streaming`).
        # The positional `raw[byte_off]` path is valid only when every
        # field's absolute offset is fixed at codegen time; this branch
        # handles every codec where it is not — present-if-gated fields
        # (runtime presence), VLE / repeat / TLV-chain / embed fields
        # (runtime width), string fields (UTF-8 decode), and a fixed field
        # after a variable-length payload (offset depends on the payload
        # length). Each field reads its own bytes and advances past what it
        # consumed; a refusal raises the `CodecError` that names it.
        f_value_len = cursor.read_vle_u64()
        if (after_shm & 0x01) == 0:
            _len = f_value_len
            if _len > 4294967295:
                raise LengthOutOfRange()
            _n = _len
            raw = cursor.peek_slice(_n)
            _v = bytes(raw)
            cursor.advance(_n)
            f_value = _v
        else:
            f_value = None
        if (after_shm & 0x01) != 0:
            _v = cursor.read_vle_u64()
            f_encoding = _v
        else:
            f_encoding = None
        if (after_shm & 0x01) != 0:
            _v = cursor.read_vle_u64()
            f_slice_count = _v
        else:
            f_slice_count = None
        if (after_shm & 0x01) != 0:
            f_slices = []
            for _ in range(f_slice_count):
                f_slices.append(CodecZenohValueSlice.decode(cursor))
        else:
            f_slices = None
        return cls(
            value_len=f_value_len,
            value=f_value,
            encoding=f_encoding,
            slice_count=f_slice_count,
            slices=f_slices,
        )

    def encode(self, w: SceSink, after_shm: int) -> None:
        """RFC §synth-5-B encode-side primary: write ``self`` into the
        caller-owned ``w`` sink. Returns ``None`` on success; raises
        :class:`BufferOverflow` from a bounded sink when the destination
        has insufficient remaining capacity; growable sinks (e.g.
        :class:`BytearraySink`) are effectively infallible. Raises,
        before the first byte, the error a decode of the frame would give
        for a value it cannot carry: a byte string, text or list outside its
        bound, or a gated field its gate disagrees with."""
        if (after_shm & 0x01) == 0:
            if self.value is None:
                raise PresentIfMismatch("value")
        elif self.value is not None:
            raise PresentIfMismatch("value")
        if (after_shm & 0x01) != 0:
            if self.encoding is None:
                raise PresentIfMismatch("encoding")
        elif self.encoding is not None:
            raise PresentIfMismatch("encoding")
        if (after_shm & 0x01) != 0:
            if self.slices is None:
                raise PresentIfMismatch("slices")
        elif self.slices is not None:
            raise PresentIfMismatch("slices")
        if self.value is not None and (len(self.value) > 4294967295):
            raise LengthOutOfRange()
        # Streaming cursor encode (SSOT selection: `needs_streaming`).
        # Mirrors the streaming decode: every field appends its own bytes
        # in declaration order through the per-field encode blocks, so a
        # gated field skips its append when absent, and a fixed field after
        # a variable-length payload lands after the payload (the positional
        # path appends variable fields last, placing it ahead on the wire).
        # Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
        # dedicated helpers; everything else uses `present_if_encode_block`.
        w.write_vle_u64(self.value_len)
        if self.value is not None:
            w.write_bytes(self.value)
        if self.encoding is not None:
            w.write_vle_u64(int(self.encoding))
        if self.slice_count is not None:
            w.write_vle_u64(int(self.slice_count))
        if self.slices is not None:
            for _e in self.slices:
                _e.encode(w)

    def encode_to_bytes(self, after_shm: int) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst), after_shm)
        return bytes(_dst)
