# SCE-MAP: codec_origin_leaf:7 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, LengthOutOfRange, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedBytes

from dataclasses import dataclass
from typing import Optional, Protocol


class CodecOriginLeafDataBytes(BoundedBytes):
    """The ``data`` element of :class:`CodecOriginLeaf`, 0 to 16 bytes,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = 16


class CodecOriginLeafSource(Protocol):
    """The elements a value of :class:`CodecOriginLeaf` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def data(self) -> CodecOriginLeafDataBytes: ...


def encode_source(src: CodecOriginLeafSource, w: SceSink) -> None:
    """Write any source of :class:`CodecOriginLeaf` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It derives ``len`` from what each measures, and never reads it off
    the source. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _d_len = len(src.data)
    w.write_u8(_d_len & 0xFF)
    w.write_bytes(src.data)


@dataclass
class CodecOriginLeaf:
    data: CodecOriginLeafDataBytes = CodecOriginLeafDataBytes()

    def __setattr__(self, name: str, value: object) -> None:
        # Each byte string and text is held to its bound wherever it is
        # set, where the value is made and after (docs/adr/0081 §2),
        # whatever it was handed.
        if name == "data":
            value = CodecOriginLeafDataBytes(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecOriginLeaf:
        """Decode the next frame from ``cursor``; on success the cursor
        advances past the consumed bytes. A frame the codec refuses raises
        the ``CodecError`` subclass that names the refusal (docs/adr/0099,
        amendment of 2026-11-16): ``NeedMoreBytes`` when the cursor's tail
        is shorter than the frame (RFC §synth-5-B L494-519), which the
        caller resumes after appending bytes, and for a frame no more bytes
        make whole ``VleWidthOverflow``, ``InvalidUtf8``,
        ``UndeclaredEnumValue``, or a length or a count past its bound
        (``LengthOutOfRange``, ``TooManyElements``, docs/adr/0077)."""
        _frame_len = cursor.remaining()
        if _frame_len < 1:
            raise NeedMoreBytes()
        raw = cursor.peek_slice(_frame_len)
        f_len = raw[0]
        if f_len > 16:
            raise LengthOutOfRange()
        if _frame_len < 1 + f_len:
            raise NeedMoreBytes()
        f_data = raw[1:1 + f_len]
        value = cls(
            data=f_data,
        )
        cursor.advance(_frame_len)
        return value

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
