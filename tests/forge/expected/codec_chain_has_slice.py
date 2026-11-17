# SCE-MAP: codec_chain_has_slice:8 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, LengthOutOfRange, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedBytes

from dataclasses import dataclass
from typing import Optional, Protocol


class CodecChainHasSliceBytesBytes(BoundedBytes):
    """The ``bytes`` element of :class:`CodecChainHasSlice`, 0 to 256 bytes,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = 256


class CodecChainHasSliceSource(Protocol):
    """The elements a value of :class:`CodecChainHasSlice` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def kind(self) -> int: ...

    @property
    def bytes(self) -> CodecChainHasSliceBytesBytes: ...


def encode_source(src: CodecChainHasSliceSource, w: SceSink) -> None:
    """Write any source of :class:`CodecChainHasSlice` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It derives ``len`` from what each measures, and never reads it off
    the source. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _d_len = len(src.bytes)
    w.write_vle_u16(src.kind)
    w.write_vle_u64(_d_len)
    w.write_bytes(src.bytes)


@dataclass
class CodecChainHasSlice:
    kind: int = 0
    bytes: CodecChainHasSliceBytesBytes = CodecChainHasSliceBytesBytes()

    def __setattr__(self, name: str, value: object) -> None:
        # Each byte string and text is held to its bound wherever it is
        # set, where the value is made and after (docs/adr/0081 §2),
        # whatever it was handed.
        if name == "bytes":
            value = CodecChainHasSliceBytesBytes(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecChainHasSlice:
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
        f_kind = cursor.read_vle_u16()
        f_len = cursor.read_vle_u64()
        _len = f_len
        if _len > 256:
            raise LengthOutOfRange()
        _n = _len
        raw = cursor.peek_slice(_n)
        f_bytes = bytes(raw)
        cursor.advance(_n)
        return cls(
            kind=f_kind,
            bytes=f_bytes,
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
