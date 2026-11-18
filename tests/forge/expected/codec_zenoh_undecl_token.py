# SCE-MAP: codec_zenoh_undecl_token:16 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedInt
from .codec_zenoh_decl_ext_keyexpr import CodecZenohDeclExtKeyexpr
from . import codec_zenoh_decl_ext_keyexpr as sce_codec_zenoh_decl_ext_keyexpr

from dataclasses import dataclass
from typing import Optional, Protocol


class CodecZenohUndeclTokenIdInt(BoundedInt):
    """The ``id`` element of :class:`CodecZenohUndeclToken`, an unsigned integer of
    32 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 32
    SIGNED = False


class CodecZenohUndeclTokenSource(Protocol):
    """The elements a value of :class:`CodecZenohUndeclToken` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def id(self) -> CodecZenohUndeclTokenIdInt: ...

    @property
    def ext_keyexpr(self) -> Optional[sce_codec_zenoh_decl_ext_keyexpr.CodecZenohDeclExtKeyexprSource]: ...


def encode_source(src: CodecZenohUndeclTokenSource, w: SceSink) -> None:
    """Write any source of :class:`CodecZenohUndeclToken` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    w.write_vle_u32(src.id)
    if src.ext_keyexpr is not None:
        sce_codec_zenoh_decl_ext_keyexpr.encode_source(src.ext_keyexpr, w)


def input_z(src: CodecZenohUndeclTokenSource) -> int:
    """The flag input ``Z`` a source of :class:`CodecZenohUndeclToken` says, which
    its encode is not handed (docs/adr/0065 §6 R4a)."""
    return (1 if src.ext_keyexpr is not None else 0)


@dataclass
class CodecZenohUndeclToken:
    id: CodecZenohUndeclTokenIdInt = 0
    ext_keyexpr: Optional[CodecZenohDeclExtKeyexpr] = None

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "id":
            value = CodecZenohUndeclTokenIdInt(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor, z: int) -> CodecZenohUndeclToken:
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
        if (z & 0x01) != 0:
            f_ext_keyexpr = CodecZenohDeclExtKeyexpr.decode(cursor)
        else:
            f_ext_keyexpr = None
        return cls(
            id=f_id,
            ext_keyexpr=f_ext_keyexpr,
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
