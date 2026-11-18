# SCE-MAP: codec_ext_encoding_info:44 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, LengthOutOfRange, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedBytes, BoundedInt

from dataclasses import dataclass, field
from typing import Optional, Protocol, Union


class CodecExtEncodingInfoCombinedIdInt(BoundedInt):
    """The ``combined_id`` element of :class:`CodecExtEncodingInfo`, an unsigned integer of
    32 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 32
    SIGNED = False


class CodecExtEncodingInfoSchemaBytes(BoundedBytes):
    """The ``schema`` element of :class:`CodecExtEncodingInfo`, 0 to 64 bytes,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = 64


class CodecExtEncodingInfoSchemaSizeInt(BoundedInt):
    """The ``schema_size`` element of :class:`CodecExtEncodingInfo`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


@dataclass
class CodecExtEncodingInfoCombinedIdHasSchemaHasSchema:
    """The ``combined_id_has_schema`` group of :class:`CodecExtEncodingInfo` where ``combined_id.has_schema`` holds
    (docs/adr/0065 §6 R4): the members present then."""

    schema: CodecExtEncodingInfoSchemaBytes = CodecExtEncodingInfoSchemaBytes()

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "schema":
            value = CodecExtEncodingInfoSchemaBytes(value)
        object.__setattr__(self, name, value)


@dataclass
class CodecExtEncodingInfoCombinedIdHasSchemaNotHasSchema:
    """The ``combined_id_has_schema`` group of :class:`CodecExtEncodingInfo` where ``combined_id.has_schema`` does not hold
    (docs/adr/0065 §6 R4): the members present then."""

    schema_size: CodecExtEncodingInfoSchemaSizeInt = 0

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "schema_size":
            value = CodecExtEncodingInfoSchemaSizeInt(value)
        object.__setattr__(self, name, value)


CodecExtEncodingInfoCombinedIdHasSchema = Union[CodecExtEncodingInfoCombinedIdHasSchemaHasSchema, CodecExtEncodingInfoCombinedIdHasSchemaNotHasSchema]
"""The ``combined_id_has_schema`` group of :class:`CodecExtEncodingInfo`: the arm the value holds."""


class CodecExtEncodingInfoSource(Protocol):
    """The elements a value of :class:`CodecExtEncodingInfo` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def combined_id(self) -> CodecExtEncodingInfoCombinedIdInt: ...

    @property
    def combined_id_has_schema(self) -> CodecExtEncodingInfoCombinedIdHasSchema: ...


def encode_source(src: CodecExtEncodingInfoSource, w: SceSink) -> None:
    """Write any source of :class:`CodecExtEncodingInfo` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It derives ``schema_size`` where what it measures is there, and reads
    what the source states where none is. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _g_combined_id_has_schema = src.combined_id_has_schema
    _e_schema = _g_combined_id_has_schema.schema if isinstance(_g_combined_id_has_schema, CodecExtEncodingInfoCombinedIdHasSchemaHasSchema) else None
    _e_schema_size = _g_combined_id_has_schema.schema_size if isinstance(_g_combined_id_has_schema, CodecExtEncodingInfoCombinedIdHasSchemaNotHasSchema) else None
    _d_schema_size = (len(_e_schema)) if _e_schema is not None else (0 if _e_schema_size is None else _e_schema_size)
    _c_combined_id = src.combined_id
    _c_combined_id = (_c_combined_id & ~0x1) | (0x1 if _e_schema is not None else 0)
    w.write_vle_u32(_c_combined_id)
    w.write_u8(_d_schema_size & 0xFF)
    if _e_schema is not None:
        w.write_bytes(_e_schema)


@dataclass
class CodecExtEncodingInfo:
    combined_id: CodecExtEncodingInfoCombinedIdInt = 0
    combined_id_has_schema: CodecExtEncodingInfoCombinedIdHasSchema = field(default_factory=CodecExtEncodingInfoCombinedIdHasSchemaNotHasSchema)

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "combined_id":
            value = CodecExtEncodingInfoCombinedIdInt(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecExtEncodingInfo:
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
        f_combined_id = cursor.read_vle_u32()
        raw = cursor.peek_slice(1)
        f_schema_size = raw[0]
        cursor.advance(1)
        if (f_combined_id & 0x00000001) != 0:
            _len = f_schema_size
            if _len > 64:
                raise LengthOutOfRange()
            _n = _len
            raw = cursor.peek_slice(_n)
            _v = bytes(raw)
            cursor.advance(_n)
            f_schema = _v
        else:
            f_schema = None
        return cls(
            combined_id=f_combined_id,
            combined_id_has_schema=CodecExtEncodingInfoCombinedIdHasSchemaHasSchema(schema=f_schema) if (((f_combined_id >> 0) & 1) != 0) else CodecExtEncodingInfoCombinedIdHasSchemaNotHasSchema(schema_size=f_schema_size),
        )

    # RFC §synth-5-B flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def has_schema(self) -> bool:
        return (self.combined_id & 0x00000001) != 0

    def set_has_schema(self, v: bool) -> None:
        if v:
            self.combined_id = (self.combined_id | 0x00000001) & 0xFFFFFFFF
        else:
            self.combined_id = self.combined_id & (0xFFFFFFFF ^ 0x00000001)

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
