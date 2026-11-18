# SCE-MAP: codec_zenoh_decl_token:28 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedInt
from .codec_zenoh_wireexpr import CodecZenohWireexpr
from . import codec_zenoh_wireexpr as sce_codec_zenoh_wireexpr

from dataclasses import dataclass, field
from typing import Optional, Protocol


class CodecZenohDeclTokenIdInt(BoundedInt):
    """The ``id`` element of :class:`CodecZenohDeclToken`, an unsigned integer of
    32 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 32
    SIGNED = False


class CodecZenohDeclTokenSource(Protocol):
    """The elements a value of :class:`CodecZenohDeclToken` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def id(self) -> CodecZenohDeclTokenIdInt: ...

    @property
    def wireexpr(self) -> sce_codec_zenoh_wireexpr.CodecZenohWireexprSource: ...


def encode_source(src: CodecZenohDeclTokenSource, w: SceSink) -> None:
    """Write any source of :class:`CodecZenohDeclToken` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    w.write_vle_u32(src.id)
    sce_codec_zenoh_wireexpr.encode_source(src.wireexpr, w)


def input_n(src: CodecZenohDeclTokenSource) -> int:
    """The flag input ``N`` a source of :class:`CodecZenohDeclToken` says, which
    its encode is not handed (docs/adr/0065 §6 R4a)."""
    return sce_codec_zenoh_wireexpr.input_n(src.wireexpr)


@dataclass
class CodecZenohDeclToken:
    id: CodecZenohDeclTokenIdInt = 0
    wireexpr: CodecZenohWireexpr = field(default_factory=CodecZenohWireexpr)

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "id":
            value = CodecZenohDeclTokenIdInt(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor, n: int) -> CodecZenohDeclToken:
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
        f_id = cursor.read_vle_u32()
        f_wireexpr = CodecZenohWireexpr.decode(cursor, n)
        return cls(
            id=f_id,
            wireexpr=f_wireexpr,
        )

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
