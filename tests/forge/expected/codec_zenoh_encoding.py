# -*- coding: utf-8 -*-
# SCE-MAP: codec_zenoh_encoding:69 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, InvalidUtf8, LengthOutOfRange, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedInt, BoundedText

from dataclasses import dataclass
from typing import Optional, Protocol


class CodecZenohEncodingPackedIdInt(BoundedInt):
    """The ``packed_id`` element of :class:`CodecZenohEncoding`, an unsigned integer of
    32 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 32
    SIGNED = False


class CodecZenohEncodingSchemaText(BoundedText):
    """The ``schema`` element of :class:`CodecZenohEncoding`, 0 to 255 UTF-8 bytes,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = 255


class CodecZenohEncodingSource(Protocol):
    """The elements a value of :class:`CodecZenohEncoding` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def packed_id(self) -> CodecZenohEncodingPackedIdInt: ...

    @property
    def schema(self) -> Optional[CodecZenohEncodingSchemaText]: ...


def encode_source(src: CodecZenohEncodingSource, w: SceSink) -> None:
    """Write any source of :class:`CodecZenohEncoding` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It derives ``schema_len`` from what each measures, and never reads it off
    the source. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _d_schema_len = (len(src.schema.encode("utf-8"))) if src.schema is not None else 0
    _c_packed_id = src.packed_id
    _c_packed_id = (_c_packed_id & ~0x1) | (0x1 if src.schema is not None else 0)
    w.write_vle_u32(_c_packed_id)
    if ((_c_packed_id >> 0) & 1) != 0:
        w.write_vle_u64(_d_schema_len)
    if src.schema is not None:
        w.write_bytes(src.schema.encode("utf-8"))


@dataclass
class CodecZenohEncoding:
    packed_id: CodecZenohEncodingPackedIdInt = 0
    schema: Optional[CodecZenohEncodingSchemaText] = None

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "packed_id":
            value = CodecZenohEncodingPackedIdInt(value)
        elif name == "schema":
            value = None if value is None else CodecZenohEncodingSchemaText(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecZenohEncoding:
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
        f_packed_id = cursor.read_vle_u32()
        if (f_packed_id & 0x00000001) != 0:
            _v = cursor.read_vle_u64()
            f_schema_len = _v
        else:
            f_schema_len = None
        if (f_packed_id & 0x00000001) != 0:
            _len = f_schema_len
            if _len > 255:
                raise LengthOutOfRange()
            _n = _len
            raw = cursor.peek_slice(_n)
            try:
                _v = bytes(raw).decode('utf-8')
            except UnicodeDecodeError as exc:
                raise InvalidUtf8() from exc
            cursor.advance(_n)
            f_schema = _v
        else:
            f_schema = None
        return cls(
            packed_id=f_packed_id,
            schema=f_schema,
        )

    # RFC §synth-5-B flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def has_schema(self) -> bool:
        return (self.packed_id & 0x00000001) != 0

    def set_has_schema(self, v: bool) -> None:
        if v:
            self.packed_id = (self.packed_id | 0x00000001) & 0xFFFFFFFF
        else:
            self.packed_id = self.packed_id & (0xFFFFFFFF ^ 0x00000001)

    def encode(self, w: SceSink) -> None:
        """RFC §synth-5-B encode-side primary: write ``self`` into the
        caller-owned ``w`` sink. Returns ``None`` on success; raises
        :class:`BufferOverflow` from a bounded sink when the destination
        has insufficient remaining capacity; growable sinks (e.g.
        :class:`BytearraySink`) are effectively infallible."""
        encode_source(self, w)

    def encode_to_bytes(self) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst))
        return bytes(_dst)
